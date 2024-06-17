use std::ops::{BitAnd, BitOr, Not};

use chksum_md5 as md5;

use super::Command;
use crate::util::{
    compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    ExtendableQueue,
};

#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
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
            | Self::Store(arg, next)
            | Self::Summon(arg, next) => format_execute(
                prefix,
                &format!("{op} {arg} ", op = self.variant_name()),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::AsAt(selector, next) => format_execute(
                prefix,
                &format!("as {selector} at @s "),
                next,
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
}

/// Combine command parts, respecting if the second part is a comment
fn map_run_cmd(cmd: String, prefix: &str) -> (bool, String) {
    if cmd.starts_with('#') {
        (false, cmd)
    } else {
        (true, prefix.to_string() + "run " + &cmd)
    }
}

/// Format the execute command, compiling the next command
fn format_execute(
    prefix: String,
    new: &str,
    next: &Execute,
    require_grouping: bool,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<(bool, String)> {
    next.compile_internal(
        prefix + new,
        require_grouping,
        options,
        global_state,
        function_state,
    )
}

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
        let uid = function_state.request_uid();
        let pre_hash = function_state.path().to_owned() + ":" + &uid.to_string();

        md5::hash(pre_hash).to_hex_lowercase()
    });
    #[allow(clippy::option_if_let_else)]
    let then = if let Some(success_uid) = require_grouping_uid.as_deref() {
        let mut group_cmd = match then.clone() {
            Execute::Run(cmd) => vec![*cmd],
            Execute::Runs(cmds) => cmds,
            ex => vec![Command::Execute(ex)],
        };
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
    let then_commands = combine_conditions_commands(successful_cond, &then);
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

#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Atom(String),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}
impl Condition {
    /// Normalize the condition.
    #[must_use]
    pub fn normalize(&self) -> Self {
        match self {
            Self::Atom(_) => self.clone(),
            Self::Not(c) => match *c.clone() {
                Self::Atom(c) => Self::Not(Box::new(Self::Atom(c))),
                Self::Not(c) => c.normalize(),
                Self::And(c1, c2) => ((!*c1).normalize()) | ((!*c2).normalize()),
                Self::Or(c1, c2) => ((!*c1).normalize()) & ((!*c2).normalize()),
            },
            Self::And(c1, c2) => c1.normalize() & c2.normalize(),
            Self::Or(c1, c2) => c1.normalize() | c2.normalize(),
        }
    }

    /// Compile the condition into a list of strings.
    #[allow(clippy::only_used_in_recursion)]
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<String> {
        match self.normalize() {
            Self::Atom(a) => vec!["if ".to_string() + &a],
            Self::Not(n) => match n.as_ref() {
                Self::Atom(a) => vec!["unless ".to_string() + a],
                _ => unreachable!("Cannot happen because of normalization"),
            },
            Self::And(c1, c2) => {
                let c1 = c1.compile(options, global_state, function_state);
                let c2 = c2.compile(options, global_state, function_state);

                c1.into_iter()
                    .flat_map(|c1| c2.iter().map(move |c2| c1.clone() + " " + c2))
                    .collect()
            }
            Self::Or(c1, c2) => {
                let mut c1 = c1.compile(options, global_state, function_state);
                let c2 = c2.compile(options, global_state, function_state);
                c1.extend(c2);
                c1
            }
        }
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

    #[test]
    fn test_condition() {
        let c1 = Condition::Atom("foo".to_string());
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
            c1.clone() & c2.clone() | c3 & c1.clone()
        );
        assert_eq!((!(c1.clone() | c2.clone())).normalize(), !c1 & !c2);
    }
}
