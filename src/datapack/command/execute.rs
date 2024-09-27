use std::ops::{BitAnd, BitOr, Not, RangeInclusive};

use chksum_md5 as md5;

use super::Command;
use crate::util::{
    compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    ExtendableQueue,
};

/// Execute command with all its variants.
#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Execute {
    Align(String, Box<Execute>),
    Anchored(String, Box<Execute>),
    As(String, Box<Execute>),
    At(String, Box<Execute>),
    AsAt(String, Box<Execute>),
    Facing(String, Box<Execute>),
    In(String, Box<Execute>),
    On(String, Box<Execute>),
    Positioned(String, Box<Execute>),
    Rotated(String, Box<Execute>),
    Store(String, Box<Execute>),
    Summon(String, Box<Execute>),
    If(Condition, Box<Execute>, Option<Box<Execute>>),
    Run(Box<Command>),
    Runs(Vec<Command>),
}

impl Execute {
    /// Compile the execute command into a list of strings.
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<String> {
        // Directly compile the command if it is a run command, skipping the execute part
        // Otherwise, compile the execute command using internal function
        if let Self::Run(cmd) = self {
            cmd.compile(options, global_state, function_state)
        } else {
            self.compile_internal(
                String::from("execute "),
                false,
                options,
                global_state,
                function_state,
            )
            .into_iter()
            .map(|(_, cmd)| cmd)
            .collect()
        }
    }

    /// Compile the execute command into strings with the given prefix.
    /// Each first tuple element is a boolean indicating if the prefix should be used for that command.
    fn compile_internal(
        &self,
        prefix: String,
        require_grouping: bool,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<(bool, String)> {
        match self {
            Self::Align(arg, next)
            | Self::Anchored(arg, next)
            | Self::As(arg, next)
            | Self::At(arg, next)
            | Self::Facing(arg, next)
            | Self::In(arg, next)
            | Self::On(arg, next)
            | Self::Positioned(arg, next)
            | Self::Rotated(arg, next)
            | Self::Store(arg, next) => next.compile_internal(
                format!("{prefix}{op} {arg} ", op = self.variant_name()),
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::AsAt(selector, next) => next.compile_internal(
                format!("{prefix}as {selector} at @s "),
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::If(cond, then, el) => compile_if_cond(
                cond,
                then.as_ref(),
                el.as_deref(),
                &prefix,
                options,
                global_state,
                function_state,
            ),
            Self::Summon(arg, next) => next.compile_internal(
                format!("{prefix}{op} {arg} ", op = self.variant_name()),
                true,
                options,
                global_state,
                function_state,
            ),
            Self::Run(command) => match &**command {
                Command::Execute(ex) => ex.compile_internal(
                    prefix,
                    require_grouping,
                    options,
                    global_state,
                    function_state,
                ),
                command => command
                    .compile(options, global_state, function_state)
                    .into_iter()
                    .map(|c| map_run_cmd(c, &prefix))
                    .collect(),
            },
            Self::Runs(commands) if !require_grouping => commands
                .iter()
                .flat_map(|c| c.compile(options, global_state, function_state))
                .map(|c| map_run_cmd(c, &prefix))
                .collect(),
            Self::Runs(commands) => Command::Group(commands.clone())
                .compile(options, global_state, function_state)
                .into_iter()
                .map(|c| map_run_cmd(c, &prefix))
                .collect(),
        }
    }

    /// Get the count of the commands the execute command will compile into.
    #[tracing::instrument(skip(options))]
    pub(super) fn get_count(&self, options: &CompileOptions) -> usize {
        let global_state = MutCompilerState::default();
        let function_state =
            FunctionCompilerState::new("[INTERNAL]", "[INTERNAL]", ExtendableQueue::default());

        self.compile_internal(
            String::new(),
            false,
            options,
            &global_state,
            &function_state,
        )
        .len()
    }

    /// Get the variant name of the execute command.
    #[must_use]
    pub fn variant_name(&self) -> &str {
        match self {
            Self::Align(..) => "align",
            Self::Anchored(..) => "anchored",
            Self::As(..) => "as",
            Self::At(..) => "at",
            Self::AsAt(..) => "as_at",
            Self::Facing(..) => "facing",
            Self::In(..) => "in",
            Self::On(..) => "on",
            Self::Positioned(..) => "positioned",
            Self::Rotated(..) => "rotated",
            Self::Store(..) => "store",
            Self::Summon(..) => "summon",
            Self::If(..) => "if",
            Self::Run(..) => "run",
            Self::Runs(..) => "runs",
        }
    }

    /// Check whether the execute command is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        match self {
            Self::Run(cmd) => cmd.validate(pack_formats),
            Self::Runs(cmds) => cmds.iter().all(|cmd| cmd.validate(pack_formats)),
            Self::Facing(_, next)
            | Self::Store(_, next)
            | Self::Positioned(_, next)
            | Self::Rotated(_, next)
            | Self::In(_, next)
            | Self::As(_, next)
            | Self::At(_, next)
            | Self::AsAt(_, next)
            | Self::Align(_, next)
            | Self::Anchored(_, next) => pack_formats.start() >= &4 && next.validate(pack_formats),
            Self::If(_, next, el) => {
                pack_formats.start() >= &4
                    && next.validate(pack_formats)
                    && el.as_deref().map_or(true, |el| el.validate(pack_formats))
            }
            Self::Summon(_, next) | Self::On(_, next) => {
                pack_formats.start() >= &12 && next.validate(pack_formats)
            }
        }
    }
}

/// Combine command parts, respecting if the second part is a comment
/// The first tuple element is a boolean indicating if the prefix should be used
fn map_run_cmd(cmd: String, prefix: &str) -> (bool, String) {
    if cmd.starts_with('#') || cmd.is_empty() || cmd.chars().all(char::is_whitespace) {
        (false, cmd)
    } else {
        (true, prefix.to_string() + "run " + &cmd)
    }
}

/// Compile an if condition command.
/// The first tuple element is a boolean indicating if the prefix should be used for that command.
#[tracing::instrument(skip_all)]
fn compile_if_cond(
    cond: &Condition,
    then: &Execute,
    el: Option<&Execute>,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<(bool, String)> {
    let then_count = then.get_count(options);

    let str_cond = cond.clone().compile(options, global_state, function_state);
    let require_grouping_uid = (el.is_some() || then_count > 1).then(|| {
        // calculate a unique condition id for the else check
        let uid = function_state.request_uid();
        let pre_hash = function_state.path().to_owned() + ":" + &uid.to_string();

        md5::hash(pre_hash).to_hex_lowercase()
    });
    #[allow(clippy::option_if_let_else)]
    let then = if let Some(success_uid) = require_grouping_uid.as_deref() {
        // prepare commands for grouping
        let mut group_cmd = match then.clone() {
            Execute::Run(cmd) => vec![*cmd],
            Execute::Runs(cmds) => cmds,
            ex => vec![Command::Execute(ex)],
        };
        // add success condition to the group
        // this condition will be checked after the group ran to determine if the else part should be executed
        if el.is_some() && str_cond.len() <= 1 {
            group_cmd.push(
                format!("data modify storage shulkerbox:cond {success_uid} set value true")
                    .as_str()
                    .into(),
            );
        }
        Command::Group(group_cmd)
            .compile(options, global_state, function_state)
            .iter()
            .map(|s| (true, "run ".to_string() + s))
            .collect()
    } else {
        then.compile_internal(
            String::new(),
            require_grouping_uid.is_some(),
            options,
            global_state,
            function_state,
        )
    };
    // if the conditions have multiple parts joined by a disjunction, commands need to be grouped
    let each_or_cmd = (str_cond.len() > 1).then(|| {
        let success_uid = require_grouping_uid.as_deref().unwrap_or_else(|| {
            tracing::error!("No success_uid found for each_or_cmd, using default");
            "if_success"
        });
        (
            format!("data modify storage shulkerbox:cond {success_uid} set value true"),
            combine_conditions_commands(
                str_cond.clone(),
                &[(
                    true,
                    format!("run data modify storage shulkerbox:cond {success_uid} set value true"),
                )],
            ),
        )
    });
    // build the condition for each then command
    let successful_cond = if each_or_cmd.is_some() {
        let success_uid = require_grouping_uid.as_deref().unwrap_or_else(|| {
            tracing::error!("No success_uid found for each_or_cmd, using default");
            "if_success"
        });
        Condition::Atom(format!("data storage shulkerbox:cond {{{success_uid}:1b}}")).compile(
            options,
            global_state,
            function_state,
        )
    } else {
        str_cond
    };
    // combine the conditions with the then commands
    let then_commands = combine_conditions_commands(successful_cond, &then);
    // build the else part
    let el_commands = el
        .map(|el| {
            let success_uid = require_grouping_uid.as_deref().unwrap_or_else(|| {
                tracing::error!("No success_uid found for each_or_cmd, using default");
                "if_success"
            });
            let else_cond =
                (!Condition::Atom(format!("data storage shulkerbox:cond {{{success_uid}:1b}}")))
                    .compile(options, global_state, function_state);
            let el = el.compile_internal(
                String::new(),
                else_cond.len() > 1,
                options,
                global_state,
                function_state,
            );
            combine_conditions_commands(else_cond, &el)
        })
        .unwrap_or_default();

    // reset the success storage if needed
    let reset_success_storage = if each_or_cmd.is_some() || el.is_some() {
        let success_uid = require_grouping_uid.as_deref().unwrap_or_else(|| {
            tracing::error!("No success_uid found for each_or_cmd, using default");
            "if_success"
        });
        Some((
            false,
            format!("data remove storage shulkerbox:cond {success_uid}"),
        ))
    } else {
        None
    };

    // combine all parts
    reset_success_storage
        .clone()
        .into_iter()
        .chain(each_or_cmd.map(|(_, cmds)| cmds).unwrap_or_default())
        .chain(then_commands)
        .chain(el_commands)
        .chain(reset_success_storage)
        .map(|(use_prefix, cmd)| {
            let cmd = if use_prefix {
                prefix.to_string() + &cmd
            } else {
                cmd
            };
            (use_prefix, cmd)
        })
        .collect()
}

fn combine_conditions_commands(
    conditions: Vec<String>,
    commands: &[(bool, String)],
) -> Vec<(bool, String)> {
    conditions
        .into_iter()
        .flat_map(|cond| {
            commands.iter().map(move |(use_prefix, cmd)| {
                // combine the condition with the command if it uses a prefix
                let cmd = if *use_prefix {
                    cond.clone() + " " + cmd
                } else {
                    cmd.clone()
                };
                (*use_prefix, cmd)
            })
        })
        .collect()
}

/// Condition for the execute command.
#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Condition {
    Atom(String),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}
impl Condition {
    /// Normalize the condition to eliminate complex negations.
    /// Uses De Morgan's laws to simplify the condition.
    #[must_use]
    pub fn normalize(&self) -> Self {
        match self {
            Self::Atom(_) => self.clone(),
            Self::Not(c) => match *c.clone() {
                Self::Atom(c) => Self::Not(Box::new(Self::Atom(c))),
                Self::Not(c) => c.normalize(),
                Self::And(a, b) => ((!*a).normalize()) | ((!*b).normalize()),
                Self::Or(a, b) => ((!*a).normalize()) & ((!*b).normalize()),
            },
            Self::And(a, b) => a.normalize() & b.normalize(),
            Self::Or(a, b) => a.normalize() | b.normalize(),
        }
    }

    /// Convert the condition into a truth table.
    /// This will expand the condition into all possible combinations of its atoms.
    /// All vector elements are in disjunction with each other and do not contain disjunctions and complex negations in them.
    #[must_use]
    pub fn to_truth_table(&self) -> Vec<Self> {
        match self.normalize() {
            Self::Atom(_) | Self::Not(_) => vec![self.clone()],
            Self::Or(a, b) => a
                .to_truth_table()
                .into_iter()
                .chain(b.to_truth_table())
                .collect(),
            Self::And(a, b) => {
                let a = a.to_truth_table();
                let b = b.to_truth_table();

                a.into_iter()
                    .flat_map(|el1| {
                        b.iter()
                            .map(move |el2| Self::And(Box::new(el1.clone()), Box::new(el2.clone())))
                    })
                    .collect()
            }
        }
    }

    /// Convert the condition into a string.
    ///
    /// Will fail if the condition contains an `Or` variant. Use `compile` instead.
    fn str_cond(&self) -> Option<String> {
        match self {
            Self::Atom(s) => Some("if ".to_string() + s),
            Self::Not(n) => match *(*n).clone() {
                Self::Atom(s) => Some("unless ".to_string() + &s),
                _ => None,
            },
            Self::And(a, b) => {
                let a = a.str_cond()?;
                let b = b.str_cond()?;

                Some(a + " " + &b)
            }
            Self::Or(..) => None,
        }
    }

    /// Compile the condition into a list of strings that can be used in Minecraft.
    #[allow(clippy::only_used_in_recursion)]
    pub fn compile(
        &self,
        _options: &CompileOptions,
        _global_state: &MutCompilerState,
        _function_state: &FunctionCompilerState,
    ) -> Vec<String> {
        let truth_table = self.to_truth_table();

        truth_table
            .into_iter()
            .map(|c| {
                c.str_cond()
                    .expect("Truth table should not contain Or variants")
            })
            .collect()
    }
}

impl From<&str> for Condition {
    fn from(s: &str) -> Self {
        Self::Atom(s.to_string())
    }
}

impl Not for Condition {
    type Output = Self;

    fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
}
impl BitAnd for Condition {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self::And(Box::new(self), Box::new(rhs))
    }
}
impl BitOr for Condition {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self::Or(Box::new(self), Box::new(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::redundant_clone)]
    #[test]
    fn test_condition() {
        let c1 = Condition::from("foo");
        let c2 = Condition::Atom("bar".to_string());
        let c3 = Condition::Atom("baz".to_string());

        assert_eq!(
            (c1.clone() & c2.clone()).normalize(),
            c1.clone() & c2.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() & c3.clone()).normalize(),
            c1.clone() & c2.clone() & c3.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone()).normalize(),
            c1.clone() | c2.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone() | c3.clone()).normalize(),
            c1.clone() | c2.clone() | c3.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() | c3.clone()).normalize(),
            c1.clone() & c2.clone() | c3.clone()
        );
        assert_eq!(
            (c1.clone() | c2.clone() & c3.clone()).normalize(),
            c1.clone() | c2.clone() & c3.clone()
        );
        assert_eq!(
            (c1.clone() & c2.clone() | c3.clone() & c1.clone()).normalize(),
            c1.clone() & c2.clone() | c3.clone() & c1.clone()
        );
        assert_eq!(
            (!(c1.clone() | c2.clone())).normalize(),
            !c1.clone() & !c2.clone()
        );
        assert_eq!(
            (!(c1.clone() & c2.clone())).normalize(),
            !c1.clone() | !c2.clone()
        );
    }

    #[allow(clippy::redundant_clone)]
    #[test]
    fn test_truth_table() {
        let c1 = Condition::Atom("foo".to_string());
        let c2 = Condition::Atom("bar".to_string());
        let c3 = Condition::Atom("baz".to_string());
        let c4 = Condition::Atom("foobar".to_string());

        assert_eq!(
            (c1.clone() & c2.clone()).to_truth_table(),
            vec![c1.clone() & c2.clone()]
        );

        assert_eq!(
            (c1.clone() & c2.clone() & c3.clone()).to_truth_table(),
            vec![c1.clone() & c2.clone() & c3.clone()]
        );

        assert_eq!(
            (c1.clone() | c2.clone()).to_truth_table(),
            vec![c1.clone(), c2.clone()]
        );

        assert_eq!(
            ((c1.clone() | c2.clone()) & c3.clone()).to_truth_table(),
            vec![c1.clone() & c3.clone(), c2.clone() & c3.clone()]
        );

        assert_eq!(
            ((c1.clone() & c2.clone()) | c3.clone()).to_truth_table(),
            vec![c1.clone() & c2.clone(), c3.clone()]
        );

        assert_eq!(
            (c1.clone() & !(c2.clone() | (c3.clone() & c4.clone()))).to_truth_table(),
            vec![
                c1.clone() & (!c2.clone() & !c3.clone()),
                c1.clone() & (!c2.clone() & !c4.clone())
            ]
        );
    }

    #[test]
    fn test_combine_conditions_commands() {
        let conditions = vec!["a", "b", "c"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let commands = &[(true, "1".to_string()), (false, "2".to_string())];

        let combined = combine_conditions_commands(conditions, commands);
        assert_eq!(
            combined,
            vec![
                (true, "a 1".to_string()),
                (false, "2".to_string()),
                (true, "b 1".to_string()),
                (false, "2".to_string()),
                (true, "c 1".to_string()),
                (false, "2".to_string())
            ]
        );
    }

    #[test]
    fn test_compile() {
        let compiled = Execute::As(
            "@ְa".to_string(),
            Box::new(Execute::If(
                "block ~ ~-1 ~ minecraft:stone".into(),
                Box::new(Execute::Run(Box::new("say hi".into()))),
                None,
            )),
        )
        .compile(
            &CompileOptions::default(),
            &MutCompilerState::default(),
            &FunctionCompilerState::default(),
        );

        assert_eq!(
            compiled,
            vec!["execute as @ְa if block ~ ~-1 ~ minecraft:stone run say hi".to_string()]
        );

        let direct = Execute::Run(Box::new("say direct".into())).compile(
            &CompileOptions::default(),
            &MutCompilerState::default(),
            &FunctionCompilerState::default(),
        );

        assert_eq!(direct, vec!["say direct".to_string()]);
    }
}
