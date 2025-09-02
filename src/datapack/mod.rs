//! Datapack module for creating and managing Minecraft datapacks.

mod command;
mod function;
mod namespace;
pub mod tag;
pub use command::{Command, Condition, Execute, Group, ReturnCommand};
pub use function::Function;
pub use namespace::Namespace;

use std::{borrow::Cow, collections::BTreeMap, ops::RangeInclusive, sync::RwLock};

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
    main_namespace_name: String,
    namespaces: BTreeMap<String, Namespace>,
    /// Scoreboard name -> (criteria, display name)
    scoreboards: BTreeMap<String, (Option<String>, Option<String>)>,
    uninstall_commands: Vec<Command>,
    custom_files: VFolder,
}

impl Datapack {
    pub const LATEST_FORMAT: u8 = 81;

    /// Create a new Minecraft datapack.
    #[must_use]
    pub fn new(main_namespace_name: impl Into<String>, pack_format: u8) -> Self {
        Self {
            description: String::from("A Minecraft datapack created with shulkerbox"),
            pack_format,
            supported_formats: None,
            main_namespace_name: main_namespace_name.into(),
            namespaces: BTreeMap::new(),
            scoreboards: BTreeMap::new(),
            uninstall_commands: Vec::new(),
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
    #[cfg(feature = "fs_access")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fs_access")))]
    pub fn with_template_folder<P>(self, path: P) -> std::io::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let template = VFolder::try_from(path.as_ref())?;
        Ok(self.with_template_vfolder(template))
    }

    /// Set the custom files of the datapack.
    #[must_use]
    pub fn with_template_vfolder(self, mut template: VFolder) -> Self {
        template.merge(self.custom_files);
        Self {
            custom_files: template,
            ..self
        }
    }

    /// Get a namespace by name.
    #[must_use]
    pub fn namespace(&self, name: &str) -> Option<&Namespace> {
        self.namespaces.get(name)
    }

    /// Mutably get a namespace by name or create a new one if it doesn't exist.
    pub fn namespace_mut(&mut self, name: impl Into<String>) -> &mut Namespace {
        let name = name.into();
        self.namespaces
            .entry(name.clone())
            .or_insert_with(|| Namespace::new(name))
    }

    /// Add a function to the tick function list.
    pub fn add_tick(&mut self, function: impl Into<String>) {
        self.namespace_mut("minecraft")
            .tag_mut("tick", tag::TagType::Function)
            .add_value(tag::TagValue::Simple(function.into()));
    }

    /// Add a function to the load function list.
    pub fn add_load(&mut self, function: impl Into<String>) {
        self.namespace_mut("minecraft")
            .tag_mut("load", tag::TagType::Function)
            .add_value(tag::TagValue::Simple(function.into()));
    }

    /// Register a scoreboard.
    pub fn register_scoreboard(
        &mut self,
        name: impl Into<String>,
        criteria: Option<impl Into<String>>,
        display_name: Option<impl Into<String>>,
    ) {
        self.scoreboards.insert(
            name.into(),
            (criteria.map(Into::into), display_name.map(Into::into)),
        );
    }

    /// Scoreboards registered in the datapack.
    #[must_use]
    pub fn scoreboards(&self) -> &BTreeMap<String, (Option<String>, Option<String>)> {
        &self.scoreboards
    }

    /// Add commands to the uninstall function.
    pub fn add_uninstall_commands(&mut self, commands: Vec<Command>) {
        self.uninstall_commands.extend(commands);
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
        let options = CompileOptions {
            pack_format: self.pack_format,
            ..options.clone()
        };

        let compiler_state = RwLock::new(CompilerState::default());

        let mut root_folder = self.custom_files.clone();
        let mcmeta = generate_mcmeta(self, &options, &compiler_state);
        root_folder.add_file("pack.mcmeta", mcmeta);
        let mut data_folder = VFolder::new();

        let mut modified_namespaces = self
            .namespaces
            .iter()
            .map(|(name, namespace)| (name.as_str(), Cow::Borrowed(namespace)))
            .collect::<BTreeMap<_, _>>();

        let mut uninstall_commands = options
            .uninstall_function
            .then_some(Cow::Borrowed(&self.uninstall_commands));

        if !self.scoreboards.is_empty() {
            let main_namespace = modified_namespaces
                .entry(&self.main_namespace_name)
                .or_insert_with(|| Cow::Owned(Namespace::new(&self.main_namespace_name)));
            let register_scoreboard_function = main_namespace
                .to_mut()
                .function_mut("sb/register_scoreboards");
            for (name, (criteria, display_name)) in &self.scoreboards {
                let mut creation_command = format!(
                    "scoreboard objectives add {name} {criteria}",
                    criteria = criteria.as_deref().unwrap_or("dummy")
                );
                if let Some(display_name) = display_name {
                    creation_command.push(' ');
                    creation_command.push_str(display_name);
                }
                register_scoreboard_function.add_command(Command::Raw(creation_command));

                if let Some(uninstall_commands) = uninstall_commands.as_mut() {
                    uninstall_commands
                        .to_mut()
                        .push(Command::Raw(format!("scoreboard objectives remove {name}")));
                }
            }

            let minecraft_namespace = modified_namespaces
                .entry("minecraft")
                .or_insert_with(|| Cow::Owned(Namespace::new("minecraft")));
            minecraft_namespace
                .to_mut()
                .tag_mut("load", tag::TagType::Function)
                .values_mut()
                .insert(
                    0,
                    tag::TagValue::Simple(format!(
                        "{}:sb/register_scoreboards",
                        self.main_namespace_name
                    )),
                );
        }

        if let Some(uninstall_commands) = uninstall_commands {
            if !uninstall_commands.is_empty() {
                let main_namespace = modified_namespaces
                    .entry(&self.main_namespace_name)
                    .or_insert_with(|| Cow::Owned(Namespace::new(&self.main_namespace_name)));
                let uninstall_function = main_namespace.to_mut().function_mut("uninstall");
                uninstall_function
                    .get_commands_mut()
                    .extend(uninstall_commands.into_owned());
            }
        }

        // Compile namespaces
        for (name, namespace) in modified_namespaces {
            let namespace_folder = namespace.compile(&options, &compiler_state);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datapack() {
        let template_dir = tempfile::tempdir().expect("error creating tempdir");

        let mut dp = Datapack::new("main", Datapack::LATEST_FORMAT)
            .with_description("My datapack")
            .with_template_folder(template_dir.path())
            .expect("error reading template folder");

        assert_eq!(dp.namespaces.len(), 0);

        let _ = dp.namespace_mut("foo");
        assert_eq!(dp.namespaces.len(), 1);
    }

    #[test]
    fn test_generate_mcmeta() {
        let dp = &Datapack::new("main", Datapack::LATEST_FORMAT).with_description("foo");
        let state = RwLock::new(CompilerState::default());
        let mcmeta = generate_mcmeta(dp, &CompileOptions::default(), &state);

        let json = if let VFile::Text(text) = mcmeta {
            serde_json::from_str::<serde_json::Value>(&text).unwrap()
        } else {
            panic!("mcmeta should be text not binary")
        };

        let pack = json
            .as_object()
            .expect("mcmeta is not object")
            .get("pack")
            .expect("no pack value")
            .as_object()
            .expect("mcmeta pack is not object");
        assert_eq!(
            pack.get("description")
                .expect("no key pack.description")
                .as_str(),
            Some("foo")
        );
        assert_eq!(
            pack.get("pack_format")
                .expect("no key pack.pack_format")
                .as_u64(),
            Some(u64::from(Datapack::LATEST_FORMAT))
        );
    }
}
