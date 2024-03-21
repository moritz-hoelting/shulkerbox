//! Namespace of a datapack

use serde::{Deserialize, Serialize};

use crate::{
    util::compile::{CompileOptions, MutCompilerState},
    virtual_fs::VFolder,
};

use super::{function::Function, tag::Tag};
use std::collections::HashMap;

/// Namespace of a datapack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    name: String,
    functions: HashMap<String, Function>,
    main_function: Function,
    tags: HashMap<String, Tag>,
}

impl Namespace {
    /// Create a new namespace.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: HashMap::new(),
            main_function: Function::default(),
            tags: HashMap::new(),
        }
    }

    /// Get the name of the namespace.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the main function of the namespace.
    pub fn get_main_function(&self) -> &Function {
        &self.main_function
    }
    /// Get the main function of the namespace mutably.
    pub fn get_main_function_mut(&mut self) -> &mut Function {
        &mut self.main_function
    }

    /// Get the functions of the namespace.
    pub fn get_functions(&self) -> &HashMap<String, Function> {
        &self.functions
    }

    /// Get the tags of the namespace.
    pub fn get_tags(&self) -> &HashMap<String, Tag> {
        &self.tags
    }

    /// Add a function to the namespace.
    pub fn add_function(&mut self, name: &str, function: Function) {
        self.functions.insert(name.to_string(), function);
    }

    /// Add a tag to the namespace.
    pub fn add_tag(&mut self, name: &str, tag: Tag) {
        self.tags.insert(name.to_string(), tag);
    }

    /// Compile the namespace into a virtual folder.
    pub fn compile(&self, options: &CompileOptions, state: &MutCompilerState) -> VFolder {
        let mut root_folder = VFolder::new();

        // Compile functions
        for (path, function) in &self.functions {
            root_folder.add_file(
                &format!("functions/{}.mcfunction", path),
                function.compile(options, state),
            );
        }
        if !self.main_function.get_commands().is_empty() {
            root_folder.add_file(
                "functions/main.mcfunction",
                self.main_function.compile(options, state),
            );
        }

        // Compile tags
        for (path, tag) in &self.tags {
            let (tag_type, vfile) = tag.compile(options, state);
            root_folder.add_file(&format!("tags/{}/{}.json", tag_type, path), vfile);
        }

        root_folder
    }
}
