//! Namespace of a datapack

use crate::{
    util::{
        compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
        ExtendableQueue,
    },
    virtual_fs::VFolder,
};

use super::{function::Function, tag::Tag};
use std::collections::{HashMap, VecDeque};

/// Namespace of a datapack
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Namespace {
    name: String,
    functions: HashMap<String, Function>,
    main_function: Function,
    tags: HashMap<String, Tag>,
}

impl Namespace {
    /// Create a new namespace.
    pub(in crate::datapack) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: HashMap::new(),
            main_function: Function::default(),
            tags: HashMap::new(),
        }
    }

    /// Get the name of the namespace.
    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the main function of the namespace.
    #[must_use]
    pub fn get_main_function(&self) -> &Function {
        &self.main_function
    }
    /// Get the main function of the namespace mutably.
    pub fn get_main_function_mut(&mut self) -> &mut Function {
        &mut self.main_function
    }

    /// Get the functions of the namespace.
    #[must_use]
    pub fn get_functions(&self) -> &HashMap<String, Function> {
        &self.functions
    }

    /// Get the tags of the namespace.
    #[must_use]
    pub fn get_tags(&self) -> &HashMap<String, Tag> {
        &self.tags
    }

    /// Get a function by name.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    /// Mutably get a function by name or create a new one if it doesn't exist.
    pub fn function_mut(&mut self, name: &str) -> &mut Function {
        self.functions
            .entry(name.to_string())
            .or_insert_with(|| Function::new(&self.name, name))
    }

    /// Add a tag to the namespace.
    pub fn add_tag(&mut self, name: &str, tag: Tag) {
        self.tags.insert(name.to_string(), tag);
    }

    /// Compile the namespace into a virtual folder.
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn compile(&self, options: &CompileOptions, state: &MutCompilerState) -> VFolder {
        tracing::debug!("Compiling namespace");

        let mut root_folder = VFolder::new();

        // collect functions
        let mut functions = self
            .functions
            .iter()
            .map(|(name, content)| (name.clone(), content.clone()))
            .collect::<VecDeque<_>>();

        if !self.main_function.get_commands().is_empty() {
            functions.push_front(("main".to_string(), self.main_function.clone()));
        }

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
        for (path, tag) in &self.tags {
            let (tag_type, vfile) = tag.compile(options, state);
            root_folder.add_file(&format!("tags/{tag_type}/{path}.json"), vfile);
        }

        root_folder
    }
}
