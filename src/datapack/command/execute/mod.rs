use std::{collections::HashSet, ops::RangeInclusive};

use super::Command;
use crate::util::{
    compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    ExtendableQueue, MacroString,
};

mod conditional;
use conditional::compile_if_cond;
pub use conditional::Condition;

/// Execute command with all its variants.
#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Execute {
    Align(MacroString, Box<Execute>),
    Anchored(MacroString, Box<Execute>),
    As(MacroString, Box<Execute>),
    At(MacroString, Box<Execute>),
    AsAt(MacroString, Box<Execute>),
    Facing(MacroString, Box<Execute>),
    In(MacroString, Box<Execute>),
    On(MacroString, Box<Execute>),
    Positioned(MacroString, Box<Execute>),
    Rotated(MacroString, Box<Execute>),
    Store(MacroString, Box<Execute>),
    Summon(MacroString, Box<Execute>),
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
            | Self::Store(arg, next)
            | Self::Summon(arg, next) => next.compile_internal(
                format!(
                    "{prefix}{op} {arg} ",
                    op = self.variant_name(),
                    arg = arg.compile()
                ),
                require_grouping,
                options,
                global_state,
                function_state,
            ),
            Self::AsAt(selector, next) => next.compile_internal(
                format!(
                    "{prefix}as {selector} at @s ",
                    selector = selector.compile()
                ),
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
                    .map(|c| map_run_cmd(command.forbid_prefix(), c, &prefix))
                    .collect(),
            },
            Self::Runs(commands) if !require_grouping => commands
                .iter()
                .flat_map(|c| {
                    let forbid_prefix = c.forbid_prefix();
                    c.compile(options, global_state, function_state)
                        .into_iter()
                        .map(move |c| (forbid_prefix, c))
                })
                .map(|(forbid_prefix, c)| map_run_cmd(forbid_prefix, c, &prefix))
                .collect(),
            Self::Runs(commands) => {
                let group = Command::Group(commands.clone());
                group
                    .compile(options, global_state, function_state)
                    .into_iter()
                    .map(|c| map_run_cmd(group.forbid_prefix(), c, &prefix))
                    .collect()
            }
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

    /// Check whether the execute command contains a macro.
    #[must_use]
    pub fn contains_macro(&self) -> bool {
        match self {
            Self::Facing(s, next)
            | Self::Store(s, next)
            | Self::Positioned(s, next)
            | Self::Rotated(s, next)
            | Self::In(s, next)
            | Self::As(s, next)
            | Self::At(s, next)
            | Self::AsAt(s, next)
            | Self::Align(s, next)
            | Self::Anchored(s, next)
            | Self::Summon(s, next)
            | Self::On(s, next) => s.contains_macro() || next.contains_macro(),
            Self::If(cond, then, el) => {
                cond.contains_macro()
                    || then.contains_macro()
                    || el.as_deref().map_or(false, Self::contains_macro)
            }
            Self::Run(cmd) => cmd.contains_macro(),
            Self::Runs(cmds) => cmds.iter().any(super::Command::contains_macro),
        }
    }

    /// Returns the names of the macros used
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        match self {
            Self::Facing(s, _)
            | Self::Store(s, _)
            | Self::Positioned(s, _)
            | Self::Rotated(s, _)
            | Self::In(s, _)
            | Self::As(s, _)
            | Self::At(s, _)
            | Self::AsAt(s, _)
            | Self::Align(s, _)
            | Self::Anchored(s, _)
            | Self::Summon(s, _)
            | Self::On(s, _) => s.get_macros(),
            Self::If(cond, then, el) => {
                let mut macros = cond.get_macros();
                macros.extend(then.get_macros());
                if let Some(el) = el {
                    macros.extend(el.get_macros());
                }
                macros
            }
            Self::Run(cmd) => cmd.get_macros(),
            Self::Runs(cmds) => cmds.iter().flat_map(|cmd| cmd.get_macros()).collect(),
        }
    }
}

impl From<Execute> for Command {
    fn from(execute: Execute) -> Self {
        Self::Execute(execute)
    }
}

impl From<Command> for Execute {
    fn from(command: Command) -> Self {
        Self::Run(Box::new(command))
    }
}
impl<V> From<V> for Execute
where
    V: Into<Vec<Command>>,
{
    fn from(value: V) -> Self {
        Self::Runs(value.into())
    }
}

/// Combine command parts, respecting if the second part is a comment
/// The first tuple element is a boolean indicating if the prefix should be used
fn map_run_cmd(forbid_prefix: bool, cmd: String, prefix: &str) -> (bool, String) {
    if forbid_prefix || cmd.is_empty() || cmd.chars().all(char::is_whitespace) {
        (false, cmd)
    } else {
        (true, prefix.to_string() + "run " + &cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile() {
        let compiled = Execute::As(
            "@a".into(),
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
            vec!["execute as @a if block ~ ~-1 ~ minecraft:stone run say hi".to_string()]
        );

        let direct = Execute::Run(Box::new("say direct".into())).compile(
            &CompileOptions::default(),
            &MutCompilerState::default(),
            &FunctionCompilerState::default(),
        );

        assert_eq!(direct, vec!["say direct".to_string()]);
    }
}
