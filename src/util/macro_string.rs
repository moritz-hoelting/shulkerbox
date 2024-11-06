#![allow(clippy::module_name_repetitions)]

use std::borrow::Cow;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroString {
    String(String),
    MacroString(Vec<MacroStringPart>),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroStringPart {
    String(String),
    MacroUsage(String),
}

impl MacroString {
    /// Returns whether the [`MacroString`] contains any macro usages
    #[must_use]
    pub fn contains_macro(&self) -> bool {
        match self {
            Self::String(_) => false,
            Self::MacroString(parts) => !parts
                .iter()
                .all(|p| matches!(p, MacroStringPart::String(_))),
        }
    }

    /// Compiles to a string that Minecraft can interpret
    #[must_use]
    pub fn compile(&self) -> String {
        match self {
            Self::String(s) => s.to_owned(),
            Self::MacroString(parts) => parts
                .iter()
                .map(|p| match p {
                    MacroStringPart::String(s) => Cow::Borrowed(s),
                    MacroStringPart::MacroUsage(m) => Cow::Owned(format!("$({m})")),
                })
                .collect::<Vec<_>>()
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// Returns the amount of lines the string has
    #[must_use]
    pub fn line_count(&self) -> usize {
        match self {
            Self::String(s) => s.lines().count(),
            Self::MacroString(parts) => {
                parts
                    .iter()
                    .map(|p| match p {
                        MacroStringPart::String(s) => s.lines().count() - 1,
                        MacroStringPart::MacroUsage(_) => 0,
                    })
                    .sum::<usize>()
                    + 1
            }
        }
    }
}

impl From<String> for MacroString {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for MacroString {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for MacroStringPart {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for MacroStringPart {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
