//! A tag for various types.

use std::fmt::Display;

use crate::{
    util::compile::{CompileOptions, MutCompilerState},
    virtual_fs::VFile,
};

/// A tag for various types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    Block,
    /// A tag for fluids.
    Fluid,
    /// A tag for items.
    Item,
    /// A tag for entities.
    Entity,
    /// A tag for game events.
    GameEvent,
    /// A tag for functions.
    Function,
    /// A custom tag type.
    /// `Other(<registry path>)` => `data/<namespace>/tags/<registry path>`
    Other(String),
}

impl TagType {
    #[must_use]
    pub fn get_directory_name(&self, pack_format: u8) -> &str {
        if pack_format < 43 {
            match self {
                Self::Block => "blocks",
                Self::Fluid => "fluids",
                Self::Item => "items",
                Self::Entity => "entity_types",
                Self::GameEvent => "game_events",
                Self::Function => "functions",
                Self::Other(path) => path,
            }
        } else {
            match self {
                Self::Block => "block",
                Self::Fluid => "fluid",
                Self::Item => "item",
                Self::Entity => "entity_type",
                Self::GameEvent => "game_event",
                Self::Function => {
                    if pack_format < 45 {
                        "functions"
                    } else {
                        "function"
                    }
                }
                Self::Other(path) => path,
            }
        }
    }
}

impl Display for TagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Block => "block".to_string(),
            Self::Fluid => "fluid".to_string(),
            Self::Item => "item".to_string(),
            Self::Entity => "entity_type".to_string(),
            Self::GameEvent => "game_event".to_string(),
            Self::Function => "function".to_string(),
            Self::Other(path) => path.to_string(),
        };
        f.write_str(&str)
    }
}

/// The value of a tag.
#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
                serde_json::json!({
                    "id": id.clone(),
                    "required": *required
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag() {
        let mut tag = Tag::new(false);
        assert!(!tag.get_replace());

        tag.set_replace(true);
        assert!(tag.get_replace());

        tag.add_value(TagValue::from("foo:bar"));
        tag.add_value(TagValue::Advanced {
            id: "bar:baz".to_string(),
            required: true,
        });

        assert_eq!(tag.get_values().len(), 2);

        let compiled = tag.compile(&CompileOptions::default(), &MutCompilerState::default());

        if let VFile::Text(text) = compiled {
            let deserialized = serde_json::from_str::<serde_json::Value>(&text)
                .expect("Failed to deserialize tag");
            assert_eq!(
                deserialized,
                serde_json::json!({
                "replace": true,
                "values": [
                        "foo:bar",
                        {
                            "id": "bar:baz",
                            "required": true
                        }
                    ]
                    })
            );
        }
    }
}
