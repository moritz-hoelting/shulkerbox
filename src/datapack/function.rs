//! Function struct and implementation

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::{
    util::compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
    virtual_fs::VFile,
};

use super::command::Command;

/// Function that can be called by a command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Function {
    commands: Vec<Command>,
}

impl Function {
    /// Create a new function.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a command to the function.
    pub fn add_command(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Get the commands of the function.
    pub fn get_commands(&self) -> &Vec<Command> {
        &self.commands
    }

    /// Compile the function into a virtual file.
    pub fn compile(&self, options: &CompileOptions, state: &MutCompilerState) -> VFile {
        let function_state = Mutex::new(FunctionCompilerState::default());

        let content = self
            .commands
            .iter()
            .map(|c| c.compile(options, state, &function_state))
            .collect::<Vec<String>>()
            .join("\n");
        VFile::Text(content)
    }
}
