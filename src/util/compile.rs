//! Compile options for the compiler.

use std::sync::Mutex;

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
}

impl CompileOptions {
    /// Set whether to compile in debug mode.
    #[must_use]
    pub fn with_debug(self, debug: bool) -> Self {
        Self { debug, ..self }
    }
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            pack_format: Datapack::LATEST_FORMAT,
            debug: true,
        }
    }
}

/// State of the compiler that can change during compilation.
#[allow(missing_copy_implementations)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct CompilerState {}
/// Mutex for the compiler state.
pub type MutCompilerState = Mutex<CompilerState>;

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
