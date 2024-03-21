//! A tag for various types.

use serde::{Deserialize, Serialize};

use crate::{
    util::compile::{CompileOptions, MutCompilerState},
    virtual_fs::VFile,
};

/// A tag for various types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    r#type: TagType,
    replace: bool,
    values: Vec<TagValue>,
}
impl Tag {
    /// Create a new tag.
    pub fn new(r#type: TagType, replace: bool) -> Self {
        Self {
            r#type,
            replace,
            values: Vec::new(),
        }
    }

    /// Add a value to the tag.
    pub fn add_value(&mut self, value: TagValue) {
        self.values.push(value);
    }

    /// Compile the tag into a virtual file without state
    pub fn compile_no_state(&self, _options: &CompileOptions) -> (String, VFile) {
        let json = serde_json::json!({
            "replace": self.replace,
            "values": self.values.iter().map(TagValue::compile).collect::<Vec<_>>()
        });
        let type_str = self.r#type.to_string();
        let vfile = VFile::Text(serde_json::to_string(&json).expect("Failed to serialize tag"));

        (type_str, vfile)
    }

    /// Compile the tag into a virtual file.
    pub fn compile(&self, options: &CompileOptions, _state: &MutCompilerState) -> (String, VFile) {
        self.compile_no_state(options)
    }
}

/// The type of a tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            Self::Blocks => "blocks".to_string(),
            Self::Fluids => "fluids".to_string(),
            Self::Items => "items".to_string(),
            Self::Entities => "entity_types".to_string(),
            Self::GameEvents => "game_events".to_string(),
            Self::Functions => "functions".to_string(),
            Self::Others(path) => path.to_string(),
        }
    }
}

/// The value of a tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
