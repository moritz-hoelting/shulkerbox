//! Function struct and implementation

use std::ops::RangeInclusive;

use getset::Getters;

use crate::{
    util::compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    virtual_fs::VFile,
};

use super::command::Command;

/// Function that can be called by a command
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, Getters, PartialEq, Eq, Hash)]
pub struct Function {
    commands: Vec<Command>,
    /// Name of the function
    #[get = "pub"]
    name: String,
    /// Namespace of the function
    #[get = "pub"]
    namespace: String,
}

impl Function {
    pub(in crate::datapack) fn new(namespace: &str, name: &str) -> Self {
        Self {
            commands: Vec::new(),
            name: name.to_string(),
            namespace: namespace.to_string(),
        }
    }
    /// Add a command to the function.
    pub fn add_command(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
    }

    /// Get the commands of the function.
    #[must_use]
    pub fn get_commands(&self) -> &Vec<Command> {
        &self.commands
    }

    /// Mutably get the commands of the function.
    pub fn get_commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    /// Compile the function into a virtual file.
    #[must_use]
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> VFile {
        tracing::trace!("Compiling function '{}'", self.name);

        let content = self
            .commands
            .iter()
            .flat_map(|c| c.compile(options, global_state, function_state))
            .collect::<Vec<String>>()
            .join("\n");
        VFile::Text(content)
    }

    // Check whether the function is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        self.commands.iter().all(|c| c.validate(pack_formats))
    }
}
