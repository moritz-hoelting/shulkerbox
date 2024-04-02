//! Compile options for the compiler.

use std::{cell::Cell, sync::Mutex};

use getset::Getters;
use serde::{Deserialize, Serialize};

use crate::datapack::Function;

use super::extendable_queue::ExtendableQueue;

/// Compile options for the compiler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    /// Whether to compile in debug mode.
    pub debug: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { debug: true }
    }
}

/// State of the compiler that can change during compilation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilerState {}
/// Mutex for the compiler state.
pub type MutCompilerState = Mutex<CompilerState>;

/// State of the compiler for each function that can change during compilation.
#[derive(Debug, Clone, Getters)]
pub struct FunctionCompilerState {
    /// Number of generated functions in the current function.
    #[get = "pub"]
    generated_functions: Cell<usize>,
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
    pub fn new(path: &str, namespace: &str, functions: FunctionQueue) -> Self {
        Self {
            generated_functions: Cell::new(0),
            namespace: namespace.to_string(),
            path: path.to_string(),
            functions,
        }
    }

    /// Add a function to the queue.
    pub fn add_function(&self, name: &str, function: Function) {
        self.functions.push((name.to_string(), function));
    }
}
