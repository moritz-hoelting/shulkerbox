//! Compile options for the compiler.

use std::{
    collections::HashMap,
    fmt::Display,
    ops::Deref,
    sync::{Mutex, RwLock},
};

use getset::Getters;

use crate::datapack::{Datapack, Function};

use super::extendable_queue::ExtendableQueue;

/// Compile options for the compiler.
#[allow(missing_copy_implementations, clippy::module_name_repetitions)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// The pack format of the target datapack.
    pub(crate) pack_format: u8,
    /// Whether to compile in debug mode.
    pub(crate) debug: bool,
    /// Whether to generate an uninstall function.
    pub(crate) uninstall_function: bool,
}

impl CompileOptions {
    /// Set whether to compile in debug mode.
    #[must_use]
    pub fn with_debug(self, debug: bool) -> Self {
        Self { debug, ..self }
    }

    /// Set whether to generate an uninstall function.
    #[must_use]
    pub fn with_uninstall_function(self, uninstall_function: bool) -> Self {
        Self {
            uninstall_function,
            ..self
        }
    }
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            pack_format: Datapack::LATEST_FORMAT,
            debug: true,
            uninstall_function: true,
        }
    }
}

/// State of the compiler that can change during compilation.
#[allow(missing_copy_implementations)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct CompilerState {
    /// Functions and their return value data path.
    pub(crate) functions_with_special_return: HashMap<String, String>,
}
/// Mutex for the compiler state.
pub type MutCompilerState = RwLock<CompilerState>;

/// State of the compiler for each function that can change during compilation.
#[derive(Debug, Getters, Default)]
pub struct FunctionCompilerState {
    /// Next unique identifier.
    uid_counter: Mutex<usize>,
    /// Path of the current function.
    #[get = "pub"]
    path: String,
    /// Namespace of the current function.
    #[get = "pub"]
    namespace: String,
    /// Queue of functions to be generated.
    functions: FunctionQueue,
}

type FunctionQueue = ExtendableQueue<(String, Function)>;

impl FunctionCompilerState {
    /// Create a new function compiler state.
    #[must_use]
    pub fn new(path: &str, namespace: &str, functions: FunctionQueue) -> Self {
        Self {
            uid_counter: Mutex::new(0),
            namespace: namespace.to_string(),
            path: path.to_string(),
            functions,
        }
    }

    /// Add a function to the queue.
    pub fn add_function(&self, name: &str, function: Function) {
        self.functions.push((name.to_string(), function));
    }

    #[must_use]
    pub fn request_uid(&self) -> usize {
        let mut guard = self.uid_counter.lock().unwrap();
        let uid = *guard;
        *guard += 1;
        uid
    }
}

/// Compiled command, ready to be written to a function.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CompiledCommand {
    /// The command string.
    command: String,
    /// Whether the command is not allowed to be prefixed.
    forbid_prefix: bool,
    /// Whether the command contains a macro.
    contains_macros: bool,
}

impl CompiledCommand {
    /// Create a new compiled command.
    #[must_use]
    pub fn new<S>(command: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            command: command.into(),
            forbid_prefix: false,
            contains_macros: false,
        }
    }

    /// Get the command string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.command
    }

    /// Set the command string.
    #[must_use]
    pub fn with_command(mut self, command: String) -> Self {
        self.command = command;
        self
    }

    /// Set whether the command is forbidden to be prefixed.
    #[must_use]
    pub fn with_forbid_prefix(mut self, forbid_prefix: bool) -> Self {
        self.forbid_prefix = forbid_prefix;
        self
    }

    /// Set whether the command contains a macro.
    #[must_use]
    pub fn with_contains_macros(mut self, contains_macros: bool) -> Self {
        self.contains_macros = contains_macros;
        self
    }

    /// Get whether the command is forbidden to be prefixed.
    #[must_use]
    pub fn forbids_prefix(&self) -> bool {
        self.forbid_prefix
    }

    /// Get whether the command contains a macro.
    #[must_use]
    pub fn contains_macros(&self) -> bool {
        self.contains_macros
    }

    /// Apply a prefix to the command (if allowed).
    #[must_use]
    pub fn apply_prefix<S>(mut self, prefix: S) -> Self
    where
        S: Into<String>,
    {
        if !self.forbid_prefix {
            self.command = prefix.into() + &self.command;
        }
        self
    }

    /// Combine current forbid prefix status with the input.
    #[must_use]
    pub fn or_forbid_prefix(mut self, forbid_prefix: bool) -> Self {
        self.forbid_prefix = self.forbid_prefix || forbid_prefix;
        self
    }

    /// Combine current contains macro status with the input.
    #[must_use]
    pub fn or_contains_macros(mut self, contains_macros: bool) -> Self {
        self.contains_macros = self.contains_macros || contains_macros;
        self
    }
}

impl Deref for CompiledCommand {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl Display for CompiledCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command)
    }
}

impl From<CompiledCommand> for String {
    fn from(compiled_command: CompiledCommand) -> Self {
        compiled_command.command
    }
}

impl From<String> for CompiledCommand {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CompiledCommand {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}
