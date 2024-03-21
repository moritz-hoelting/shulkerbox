//! Compile options for the compiler.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FunctionCompilerState {}
/// Mutex for the function compiler state.
pub type MutFunctionCompilerState = Mutex<FunctionCompilerState>;
