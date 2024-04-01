use std::ops::{BitAnd, BitOr, Deref, Not};

use serde::{Deserialize, Serialize};

use crate::util::compile::{CompileOptions, MutCompilerState, MutFunctionCompilerState};

use super::Command;

#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        function_state: &MutFunctionCompilerState,
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
        }
    }
    fn compile_internal(
        &self,
        prefix: String,
        require_grouping: bool,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &MutFunctionCompilerState,
    ) -> Vec<String> {
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
                prefix,
                options,
                global_state,
                function_state,
            ),
            Self::Run(command) if !require_grouping => command
                .compile(options, global_state, function_state)
                .into_iter()
                .map(|c| prefix.clone() + "run " + &c)
                .collect(),
            Self::Run(command) => Command::Group(vec![command.deref().clone()])
                .compile(options, global_state, function_state)
                .into_iter()
                .map(|c| prefix.clone() + "run " + &c)
                .collect(),
            Self::Runs(commands) if !require_grouping => commands
                .iter()
                .flat_map(|c| c.compile(options, global_state, function_state))
                .map(|c| prefix.clone() + "run " + &c)
                .collect(),
            Self::Runs(commands) => Command::Group(commands.clone())
                .compile(options, global_state, function_state)
                .into_iter()
                .map(|c| prefix.clone() + "run " + &c)
                .collect(),
        }
    }
}

fn format_execute(
    prefix: String,
    new: &str,
    next: &Execute,
    require_grouping: bool,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &MutFunctionCompilerState,
) -> Vec<String> {
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
    prefix: String,
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &MutFunctionCompilerState,
) -> Vec<String> {
    let str_cond = cond.clone().compile(options, global_state, function_state);
    let require_grouping = el.is_some() || str_cond.len() > 1;
    let then = if require_grouping {
        let mut group_cmd = vec![Command::Execute(then.clone())];
        if el.is_some() {
            group_cmd.push("data modify storage shulkerbox:cond if_success set value true".into());
        }
        Command::Group(group_cmd)
            .compile(options, global_state, function_state)
            .iter()
            .map(|s| "run ".to_string() + s)
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
    let then_commands = combine_conditions_commands(str_cond, then);
    let el = el
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
            combine_conditions_commands(else_cond, el)
                .into_iter()
                .map(|cmd| (true, cmd))
                .chain(std::iter::once((
                    false,
                    "data remove storage shulkerbox:cond if_success".to_string(),
                )))
                .map(|(use_prefix, cmd)| {
                    if use_prefix {
                        prefix.clone() + &cmd
                    } else {
                        cmd
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    then_commands
        .into_iter()
        .map(|cmd| prefix.clone() + &cmd)
        .chain(el)
        .collect()
}

fn combine_conditions_commands(conditions: Vec<String>, commands: Vec<String>) -> Vec<String> {
    conditions
        .into_iter()
        .flat_map(|cond| commands.iter().map(move |cmd| cond.clone() + " " + cmd))
        .collect()
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    Atom(String),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}
impl Condition {
    /// Normalize the condition.
    pub fn normalize(&self) -> Self {
        match self {
            Self::Atom(_) => self.clone(),
            Self::Not(c) => match *c.clone() {
                Self::Atom(c) => Self::Not(Box::new(Self::Atom(c.clone()))),
                Self::Not(c) => c.normalize(),
                Self::And(c1, c2) => ((!*c1).normalize()) | ((!*c2).normalize()),
                Self::Or(c1, c2) => ((!*c1).normalize()) & ((!*c2).normalize()),
            },
            Self::And(c1, c2) => c1.normalize() & c2.normalize(),
            Self::Or(c1, c2) => c1.normalize() | c2.normalize(),
        }
    }

    /// Compile the condition into a list of strings.
    pub fn compile(
        &self,
        _options: &CompileOptions,
        _global_state: &MutCompilerState,
        _function_state: &MutFunctionCompilerState,
    ) -> Vec<String> {
        match self.normalize() {
            Self::Atom(a) => vec!["if ".to_string() + &a],
            Self::Not(n) => match n.as_ref() {
                Self::Atom(a) => vec!["unless ".to_string() + a],
                _ => unreachable!("Cannot happen because of normalization"),
            },
            Self::And(c1, c2) => {
                let c1 = c1.compile(_options, _global_state, _function_state);
                let c2 = c2.compile(_options, _global_state, _function_state);

                c1.into_iter()
                    .flat_map(|c1| c2.iter().map(move |c2| c1.clone() + " " + c2))
                    .collect()
            }
            Self::Or(c1, c2) => {
                let mut c1 = c1.compile(_options, _global_state, _function_state);
                let c2 = c2.compile(_options, _global_state, _function_state);
                c1.extend(c2);
                c1
            }
        }
    }
}

impl From<&str> for Condition {
    fn from(s: &str) -> Self {
        Condition::Atom(s.to_string())
    }
}

impl Not for Condition {
    type Output = Self;

    fn not(self) -> Self {
        Condition::Not(Box::new(self))
    }
}
impl BitAnd for Condition {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Condition::And(Box::new(self), Box::new(rhs))
    }
}
impl BitOr for Condition {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Condition::Or(Box::new(self), Box::new(rhs))
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
            c1.clone() & c2.clone() | c3.clone() & c1.clone()
        );
        assert_eq!(
            (!(c1.clone() | c2.clone())).normalize(),
            !c1.clone() & !c2.clone()
        );
    }
}
