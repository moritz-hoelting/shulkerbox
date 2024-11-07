use chksum_md5 as md5;
use std::{
    collections::HashSet,
    ops::{BitAnd, BitOr, Not},
};

use crate::{
    prelude::Command,
    util::{
        compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
        MacroString,
    },
};

use super::Execute;

/// Compile an if condition command.
/// The first tuple element is a boolean indicating if the prefix should be used for that command.
#[tracing::instrument(skip_all)]
pub fn compile_if_cond(
    cond: &Condition,
    then: &Execute,
    el: Option<&Execute>,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<(bool, String)> {
    if options.pack_format < 20 {
        compile_pre_20_format(
            cond,
            then,
            el,
            prefix,
            options,
            global_state,
            function_state,
        )
    } else {
        compile_since_20_format(
            cond,
            then,
            el,
            prefix,
            options,
            global_state,
            function_state,
        )
    }
}

#[allow(clippy::too_many_lines)]
fn compile_pre_20_format(
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
        let group = Command::Group(group_cmd);
        let allows_prefix = !group.forbid_prefix();
        group
            .compile(options, global_state, function_state)
            .iter()
            .map(|s| {
                if allows_prefix {
                    (true, "run ".to_string() + s)
                } else {
                    (false, s.clone())
                }
            })
            .collect()
    } else {
        then.compile_internal(String::new(), false, options, global_state, function_state)
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
        Condition::Atom(MacroString::from(format!(
            "data storage shulkerbox:cond {{{success_uid}:1b}}"
        )))
        .compile(options, global_state, function_state)
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
            let else_cond = (!Condition::Atom(MacroString::from(format!(
                "data storage shulkerbox:cond {{{success_uid}:1b}}"
            ))))
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

fn compile_since_20_format(
    cond: &Condition,
    then: &Execute,
    el: Option<&Execute>,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<(bool, String)> {
    let then_count = then.get_count(options);

    let str_cond = cond
        .clone()
        .compile_keep_macros(options, global_state, function_state);

    // if the conditions have multiple parts joined by a disjunction or an else part, commands need to be grouped
    if el.is_some() || str_cond.len() > 1 {
        let group_cmds = handle_return_group_case_since_20(
            str_cond,
            then,
            el,
            prefix,
            options,
            global_state,
            function_state,
        );
        let group = Command::Group(group_cmds);
        let allows_prefix = !group.forbid_prefix();
        group
            .compile(options, global_state, function_state)
            .into_iter()
            .map(|s| (allows_prefix, s))
            .collect()
    } else if then_count > 1 {
        let then_cmd = match then.clone() {
            Execute::Run(cmd) => vec![*cmd],
            Execute::Runs(cmds) => cmds,
            ex => vec![Command::Execute(ex)],
        };
        let group_cmd = Command::Group(then_cmd);
        let then_cmd = if group_cmd.forbid_prefix() {
            group_cmd
        } else {
            Command::Concat(
                Box::new(Command::Raw("run ".to_string())),
                Box::new(group_cmd),
            )
        };
        combine_conditions_commands_concat(str_cond, &then_cmd)
            .into_iter()
            .map(|cmd| {
                (
                    cmd.forbid_prefix(),
                    cmd.compile(options, global_state, function_state),
                )
            })
            .flat_map(|(forbid_prefix, cmds)| {
                cmds.into_iter()
                    .map(move |cmd| (!forbid_prefix, prefix.to_string() + &cmd))
            })
            .collect()
    } else {
        combine_conditions_commands_concat(
            str_cond,
            &Command::Concat(
                Box::new(Command::Raw("run ".to_string())),
                Box::new(Command::Execute(then.clone())),
            ),
        )
        .into_iter()
        .map(|cmd| {
            (
                cmd.forbid_prefix(),
                cmd.compile(options, global_state, function_state),
            )
        })
        .flat_map(|(forbid_prefix, cmds)| {
            cmds.into_iter()
                .map(move |cmd| (!forbid_prefix, prefix.to_string() + &cmd))
        })
        .collect()
    }
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

fn combine_conditions_commands_concat(
    conditions: Vec<MacroString>,
    command: &Command,
) -> Vec<Command> {
    if command.forbid_prefix() {
        vec![command.clone()]
    } else {
        conditions
            .into_iter()
            .map(|cond| {
                let prefix = if cond.contains_macro() {
                    Command::UsesMacro(cond + " ")
                } else {
                    Command::Raw(cond.compile() + " ")
                };
                Command::Concat(Box::new(prefix), Box::new(command.clone()))
            })
            .collect()
    }
}

fn handle_return_group_case_since_20(
    str_cond: Vec<MacroString>,
    then: &Execute,
    el: Option<&Execute>,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<Command> {
    // prepare commands for grouping
    let then_cmd = match then.clone() {
        Execute::Run(cmd) => vec![*cmd],
        Execute::Runs(cmds) => cmds,
        ex => vec![Command::Execute(ex)],
    };
    let group = Command::Group(then_cmd);
    let then_cmd_concat = if group.forbid_prefix() {
        group
    } else {
        Command::Concat(
            Box::new(Command::Raw("run return run ".to_string())),
            Box::new(group),
        )
    };
    let then_cond_concat = combine_conditions_commands_concat(str_cond, &then_cmd_concat);
    let mut group_cmds = then_cond_concat
        .into_iter()
        .map(|cmd| {
            if cmd.forbid_prefix() {
                cmd
            } else {
                Command::Concat(
                    Box::new(Command::Raw("execute ".to_string())),
                    Box::new(cmd),
                )
            }
        })
        .collect::<Vec<_>>();
    if let Some(el) = el {
        handle_else_since_20(
            &mut group_cmds,
            el.clone(),
            prefix,
            options,
            global_state,
            function_state,
        );
    }
    group_cmds
}

fn handle_else_since_20(
    group_cmds: &mut Vec<Command>,
    el: Execute,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) {
    let el_cmd = match el {
        Execute::If(cond, then, el) => handle_return_group_case_since_20(
            cond.compile_keep_macros(options, global_state, function_state),
            &then,
            el.as_deref(),
            prefix,
            options,
            global_state,
            function_state,
        ),
        Execute::Run(cmd) => match *cmd {
            Command::Execute(Execute::If(cond, then, el)) => handle_return_group_case_since_20(
                cond.compile_keep_macros(options, global_state, function_state),
                &then,
                el.as_deref(),
                prefix,
                options,
                global_state,
                function_state,
            ),

            _ => vec![*cmd],
        },
        Execute::Runs(cmds) => cmds,
        ex => vec![Command::Execute(ex)],
    };
    group_cmds.extend(el_cmd);
}

/// Condition for the execute command.
#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Condition {
    Atom(MacroString),
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

    /// Convert the condition into a [`MacroString`].
    ///
    /// Will fail if the condition contains an `Or` variant. Use `compile` instead.
    fn str_cond(&self) -> Option<MacroString> {
        match self {
            Self::Atom(s) => Some(MacroString::from("if ") + s.clone()),
            Self::Not(n) => match *(*n).clone() {
                Self::Atom(s) => Some(MacroString::from("unless ") + s),
                _ => None,
            },
            Self::And(a, b) => {
                let a = a.str_cond()?;
                let b = b.str_cond()?;

                Some(a + MacroString::from(" ") + b)
            }
            Self::Or(..) => None,
        }
    }

    /// Compile the condition into a list of strings that can be used in Minecraft.
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<String> {
        self.compile_keep_macros(options, global_state, function_state)
            .into_iter()
            .map(|s| s.compile())
            .collect()
    }

    /// Compile the condition into a list of macro strings.
    pub fn compile_keep_macros(
        &self,
        _options: &CompileOptions,
        _global_state: &MutCompilerState,
        _function_state: &FunctionCompilerState,
    ) -> Vec<MacroString> {
        let truth_table = self.to_truth_table();

        truth_table
            .into_iter()
            .map(|c| {
                c.str_cond()
                    .expect("Truth table should not contain Or variants")
            })
            .collect::<Vec<_>>()
    }

    /// Check whether the condition contains a macro.
    #[must_use]
    pub fn contains_macro(&self) -> bool {
        match self {
            Self::Atom(s) => s.contains_macro(),
            Self::Not(n) => n.contains_macro(),
            Self::And(a, b) | Self::Or(a, b) => a.contains_macro() || b.contains_macro(),
        }
    }

    /// Returns the names of the macros used
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        match self {
            Self::Atom(s) => s.get_macros(),
            Self::Not(n) => n.get_macros(),
            Self::And(a, b) | Self::Or(a, b) => {
                let mut set = a.get_macros();
                set.extend(b.get_macros());
                set
            }
        }
    }
}

impl From<&str> for Condition {
    fn from(s: &str) -> Self {
        Self::Atom(s.into())
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
        let c2 = Condition::Atom("bar".into());
        let c3 = Condition::Atom("baz".into());

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
        let c1 = Condition::Atom("foo".into());
        let c2 = Condition::Atom("bar".into());
        let c3 = Condition::Atom("baz".into());
        let c4 = Condition::Atom("foobar".into());

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
}
