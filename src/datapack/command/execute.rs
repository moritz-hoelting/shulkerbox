use std::ops::{BitAnd, BitOr, Not};

use crate::util::compile::{CompileOptions, FunctionCompilerState, MutCompilerState};

use super::Command;

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
    #[allow(clippy::too_many_lines)]
    fn compile_internal(
        &self,
        prefix: String,
        require_grouping: bool,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<(bool, String)> {
        match self {
            Self::Align(align, next) => format_execute(
                prefix,
                &format!("align {align} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::Anchored(anchor, next) => format_execute(
                prefix,
                &format!("anchored {anchor} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::As(selector, next) => format_execute(
                prefix,
                &format!("as {selector} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::At(selector, next) => format_execute(
                prefix,
                &format!("at {selector} "),
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
            Self::Facing(facing, next) => format_execute(
                prefix,
                &format!("facing {facing} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::In(dim, next) => format_execute(
                prefix,
                &format!("in {dim} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::On(dim, next) => format_execute(
                prefix,
                &format!("on {dim} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::Positioned(pos, next) => format_execute(
                prefix,
                &format!("positioned {pos} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::Rotated(rot, next) => format_execute(
                prefix,
                &format!("rotated {rot} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::Store(store, next) => format_execute(
                prefix,
                &format!("store {store} "),
                next,
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::Summon(entity, next) => format_execute(
                prefix,
                &format!("summon {entity} "),
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
}

fn map_run_cmd(cmd: String, prefix: &str) -> (bool, String) {
    if cmd.starts_with('#') {
        (false, cmd)
    } else {
        (true, prefix.to_string() + "run " + &cmd)
    }
}

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

fn compile_if_cond(
    cond: &Condition,
    then: &Execute,
    el: Option<&Execute>,
    prefix: &str,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<(bool, String)> {
    // TODO: fix conflicting data storage location when nesting if-else conditions

    let str_then = then.compile_internal(
        prefix.to_string(),
        false,
        options,
        global_state,
        function_state,
    );
    let str_cond = cond.clone().compile(options, global_state, function_state);
    let require_grouping = el.is_some() || str_then.len() > 1;
    let then = if require_grouping {
        let mut group_cmd = match then.clone() {
            Execute::Run(cmd) => vec![*cmd],
            Execute::Runs(cmds) => cmds,
            ex => vec![Command::Execute(ex)],
        };
        if el.is_some() && str_cond.len() <= 1 {
            group_cmd.push("data modify storage shulkerbox:cond if_success set value true".into());
        }
        Command::Group(group_cmd)
            .compile(options, global_state, function_state)
            .iter()
            .map(|s| (true, "run ".to_string() + s))
            .collect()
    } else {
        then.compile_internal(
            String::new(),
            require_grouping,
            options,
            global_state,
            function_state,
        )
    };
    let each_or_cmd = (str_cond.len() > 1).then(|| {
        (
            "data modify storage shulkerbox:cond if_success set value true",
            combine_conditions_commands(
                str_cond.clone(),
                &[(
                    true,
                    "run data modify storage shulkerbox:cond if_success set value true".to_string(),
                )],
            ),
        )
    });
    let successful_cond = if each_or_cmd.is_some() {
        Condition::Atom("data storage shulkerbox:cond {if_success:1b}".to_string()).compile(
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
            let else_cond =
                (!Condition::Atom("data storage shulkerbox:cond {if_success:1b}".to_string()))
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
        Some((
            false,
            "data remove storage shulkerbox:cond if_success".to_string(),
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
