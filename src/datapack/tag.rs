//! A tag for various types.

use crate::{
    util::compile::{CompileOptions, MutCompilerState},
    virtual_fs::VFile,
};

/// A tag for various types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Tag {
    replace: bool,
    values: Vec<TagValue>,
}
impl Tag {
    /// Create a new tag.
    #[must_use]
    pub fn new(replace: bool) -> Self {
        Self {
            replace,
            values: Vec::new(),
        }
    }

    /// Get whether the tag should replace existing values.
    #[must_use]
    pub fn get_replace(&self) -> bool {
        self.replace
    }

    /// Set whether the tag should replace existing values.
    pub fn set_replace(&mut self, replace: bool) {
        self.replace = replace;
    }

    /// Get the values of the tag.
    #[must_use]
    pub fn get_values(&self) -> &Vec<TagValue> {
        &self.values
    }

    /// Add a value to the tag.
    pub fn add_value(&mut self, value: TagValue) {
        self.values.push(value);
    }

    /// Compile the tag into a virtual file without state
    pub fn compile_no_state(&self, _options: &CompileOptions) -> VFile {
        let json = serde_json::json!({
            "replace": self.replace,
            "values": self.values.iter().map(TagValue::compile).collect::<Vec<_>>()
        });

        VFile::Text(serde_json::to_string(&json).expect("Failed to serialize tag"))
    }

    /// Compile the tag into a virtual file.
    pub fn compile(&self, options: &CompileOptions, _state: &MutCompilerState) -> VFile {
        self.compile_no_state(options)
    }
}

/// The type of a tag.
#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagType {
    /// A tag for blocks.
    Blocks,
    /// A tag for fluids.
    Fluids,
    /// A tag for items.
    Items,
    /// A tag for entities.
    Entities,
    /// A tag for game events.
    GameEvents,
    /// A tag for functions.
    Functions,
    /// A custom tag
    /// `Others(<registry path>)` => `data/<namespace>/tags/<registry path>`
    Others(String),
}
impl ToString for TagType {
    fn to_string(&self) -> String {
        match self {
            Self::Blocks => "block".to_string(),
            Self::Fluids => "fluid".to_string(),
            Self::Items => "item".to_string(),
            Self::Entities => "entity_type".to_string(),
            Self::GameEvents => "game_event".to_string(),
            Self::Functions => "function".to_string(),
            Self::Others(path) => path.to_string(),
        }
    }
}

/// The value of a tag.
#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum TagValue {
    /// A simple value, either a resource location or an id of another tag.
    Simple(String),
    /// An advanced value, with an id (same as above) and whether the loading of the tag should fail when entry is not found.
    Advanced {
        /// The id of the tag.
        id: String,
        /// Whether the loading of the tag should fail when the entry is not found.
        required: bool,
    },
}
impl From<&str> for TagValue {
    fn from(value: &str) -> Self {
        Self::Simple(value.to_string())
    }
}
impl TagValue {
    /// Compile the tag value into a JSON value.
    #[must_use]
    pub fn compile(&self) -> serde_json::Value {
        match self {
            Self::Simple(value) => serde_json::Value::String(value.clone()),
            Self::Advanced { id, required } => {
                let mut map = serde_json::Map::new();
                map.insert("id".to_string(), serde_json::Value::String(id.clone()));
                map.insert("required".to_string(), serde_json::Value::Bool(*required));
                serde_json::Value::Object(map)
            }
        }
    }
}
