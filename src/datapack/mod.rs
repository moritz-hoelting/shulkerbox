//! Datapack module for creating and managing Minecraft datapacks.

mod command;
mod function;
mod namespace;
pub mod tag;
pub use command::{Command, Condition, Execute};
pub use function::Function;
pub use namespace::Namespace;

use std::{collections::HashMap, ops::RangeInclusive, path::Path, sync::Mutex};

use crate::{
    util::compile::{CompileOptions, CompilerState, MutCompilerState},
    virtual_fs::{VFile, VFolder},
};

/// A Minecraft datapack.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datapack {
    // TODO: Support filter and overlays
    description: String,
    pack_format: u8,
    supported_formats: Option<RangeInclusive<u8>>,
    namespaces: HashMap<String, Namespace>,
    custom_files: VFolder,
}

impl Datapack {
    pub(crate) const LATEST_FORMAT: u8 = 48;

    /// Create a new Minecraft datapack.
    #[must_use]
    pub fn new(pack_format: u8) -> Self {
        Self {
            description: String::from("A Minecraft datapack created with shulkerbox"),
            pack_format,
            supported_formats: None,
            namespaces: HashMap::new(),
            custom_files: VFolder::new(),
        }
    }

    /// Set the description of the datapack.
    #[must_use]
    pub fn with_description(self, description: &str) -> Self {
        Self {
            description: description.to_string(),
            ..self
        }
    }

    /// Set the supported pack formats of the datapack.
    #[must_use]
    pub fn with_supported_formats(self, supported_formats: RangeInclusive<u8>) -> Self {
        Self {
            supported_formats: Some(supported_formats),
            ..self
        }
    }

    /// Set the custom files of the datapack.
    ///
    /// # Errors
    /// - If loading the directory fails
    pub fn with_template_folder(self, path: &Path) -> std::io::Result<Self> {
        let mut template = VFolder::try_from(path)?;
        template.merge(self.custom_files);

        Ok(Self {
            custom_files: template,
            ..self
        })
    }

    /// Get a namespace by name.
    #[must_use]
    pub fn namespace(&self, name: &str) -> Option<&Namespace> {
        self.namespaces.get(name)
    }

    /// Mutably get a namespace by name or create a new one if it doesn't exist.
    pub fn namespace_mut(&mut self, name: &str) -> &mut Namespace {
        self.namespaces
            .entry(name.to_string())
            .or_insert_with(|| Namespace::new(name))
    }

    /// Add a function to the tick function list.
    pub fn add_tick(&mut self, function: &str) {
        self.namespace_mut("minecraft")
            .tag_mut("tick", tag::TagType::Functions)
            .add_value(tag::TagValue::Simple(function.to_string()));
    }

    /// Add a function to the load function list.
    pub fn add_load(&mut self, function: &str) {
        self.namespace_mut("minecraft")
            .tag_mut("load", tag::TagType::Functions)
            .add_value(tag::TagValue::Simple(function.to_string()));
    }

    /// Add a custom file to the datapack.
    pub fn add_custom_file(&mut self, path: &str, file: VFile) {
        self.custom_files.add_file(path, file);
    }

    /// Compile the pack into a virtual folder.
    #[must_use]
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn compile(&self, options: &CompileOptions) -> VFolder {
        tracing::debug!("Compiling datapack: {:?}", self);

        let compiler_state = Mutex::new(CompilerState::default());

        let mut root_folder = self.custom_files.clone();
        let mcmeta = generate_mcmeta(self, options, &compiler_state);
        root_folder.add_file("pack.mcmeta", mcmeta);
        let mut data_folder = VFolder::new();

        // Compile namespaces
        for (name, namespace) in &self.namespaces {
            let namespace_folder = namespace.compile(options, &compiler_state);
            data_folder.add_existing_folder(name, namespace_folder);
        }

        root_folder.add_existing_folder("data", data_folder);
        root_folder
    }

    /// Check whether the datapack is valid with the given pack format.
    #[must_use]
    pub fn validate(&self) -> bool {
        let pack_formats = self
            .supported_formats
            .clone()
            .unwrap_or(self.pack_format..=self.pack_format);
        self.namespaces
            .values()
            .all(|namespace| namespace.validate(&pack_formats))
    }
}

fn generate_mcmeta(dp: &Datapack, _options: &CompileOptions, _state: &MutCompilerState) -> VFile {
    let mut content = serde_json::json!({
        "pack": {
            "description": dp.description,
            "pack_format": dp.pack_format
        }
    });
    if let Some(supported_formats) = &dp.supported_formats {
        content["pack"]["supported_formats"] = serde_json::json!({
            "min_inclusive": *supported_formats.start(),
            "max_inclusive": *supported_formats.end()
        });
    }

    VFile::Text(content.to_string())
}
