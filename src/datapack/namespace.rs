//! Namespace of a datapack

use crate::{
    util::{
        compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
        pack_format, ExtendableQueue,
    },
    virtual_fs::VFolder,
};

use super::{
    function::Function,
    tag::{Tag, TagType},
};
use std::{
    collections::{HashMap, VecDeque},
    ops::RangeInclusive,
};

/// Namespace of a datapack
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
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
            .entry((name.to_string(), tag_type))
            .or_insert_with(|| Tag::new(false))
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
                &format!(
                    "{directory_name}/{path}.mcfunction",
                    directory_name = pack_format::function_directory_name(options.pack_format)
                ),
                function.compile(options, state, &function_state),
            );
        }

        // compile tags
        for ((path, tag_type), tag) in &self.tags {
            let vfile = tag.compile(options, state);
            root_folder.add_file(
                &format!(
                    "tags/{tag_directory}/{path}.json",
                    tag_directory = tag_type.get_directory_name(options.pack_format)
                ),
                vfile,
            );
        }

        root_folder
    }

    /// Check whether the namespace is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        self.functions
            .values()
            .all(|function| function.validate(pack_formats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace() {
        let mut namespace = Namespace::new("foo");

        assert_eq!(namespace.get_name(), "foo");
        assert_eq!(namespace.get_functions().len(), 0);
        assert_eq!(namespace.get_tags().len(), 0);

        let _ = namespace.function_mut("bar");
        assert_eq!(namespace.get_functions().len(), 1);

        assert!(namespace.function("bar").is_some());
        assert!(namespace.function("baz").is_none());
    }
}
