//! Function struct and implementation

use getset::Getters;
use serde::{Deserialize, Serialize};

use crate::{
    util::compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    virtual_fs::VFile,
};

use super::command::Command;

/// Function that can be called by a command
#[derive(Debug, Clone, Default, Serialize, Deserialize, Getters)]
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
    pub fn get_commands(&self) -> &Vec<Command> {
        &self.commands
    }

    /// Mutably get the commands of the function.
    pub fn get_commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    /// Compile the function into a virtual file.
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> VFile {
        let content = self
            .commands
            .iter()
            .flat_map(|c| c.compile(options, global_state, function_state))
            .collect::<Vec<String>>()
            .join("\n");
        VFile::Text(content)
    }
}
