//! Namespace of a datapack

use crate::{
    util::{
        compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
        ExtendableQueue,
    },
    virtual_fs::VFolder,
};

use super::{
    function::Function,
    tag::{Tag, TagType},
};
use std::collections::{HashMap, VecDeque};

/// Namespace of a datapack
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Namespace {
    name: String,
    functions: HashMap<String, Function>,
    tags: HashMap<(String, TagType), Tag>,
}

impl Namespace {
    /// Create a new namespace.
    pub(in crate::datapack) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    /// Get the name of the namespace.
    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the functions of the namespace.
    #[must_use]
    pub fn get_functions(&self) -> &HashMap<String, Function> {
        &self.functions
    }

    /// Get the tags of the namespace.
    #[must_use]
    pub fn get_tags(&self) -> &HashMap<(String, TagType), Tag> {
        &self.tags
    }

    /// Get a function by name.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    /// Mutably get a function by name or create a new one if it doesn't exist.
    #[must_use]
    pub fn function_mut(&mut self, name: &str) -> &mut Function {
        self.functions
            .entry(name.to_string())
            .or_insert_with(|| Function::new(&self.name, name))
    }

    /// Get a tag by name and type.
    #[must_use]
    pub fn tag(&self, name: &str, tag_type: TagType) -> Option<&Tag> {
        self.tags.get(&(name.to_string(), tag_type))
    }

    /// Mutably get a tag by name and type or create a new one if it doesn't exist.
    #[must_use]
    pub fn tag_mut(&mut self, name: &str, tag_type: TagType) -> &mut Tag {
        self.tags
            .entry((name.to_string(), tag_type.clone()))
            .or_insert_with(|| Tag::new(tag_type, false))
    }

    /// Compile the namespace into a virtual folder.
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn compile(&self, options: &CompileOptions, state: &MutCompilerState) -> VFolder {
        tracing::debug!("Compiling namespace");

        let mut root_folder = VFolder::new();

        // collect functions
        let functions = self
            .functions
            .iter()
            .map(|(name, content)| (name.clone(), content.clone()))
            .collect::<VecDeque<_>>();

        // compile all functions, allow adding new functions while compiling
        let mut functions = ExtendableQueue::from(functions);
        while let Some((path, function)) = functions.next() {
            let function_state = FunctionCompilerState::new(&path, &self.name, functions.clone());
            root_folder.add_file(
                &format!("function/{path}.mcfunction"),
                function.compile(options, state, &function_state),
            );
        }

        // compile tags
        for ((path, tag_type), tag) in &self.tags {
            let vfile = tag.compile(options, state);
            root_folder.add_file(
                &format!(
                    "tags/{tag_type}/{path}.json",
                    tag_type = tag_type.to_string()
                ),
                vfile,
            );
        }

        root_folder
    }
}
