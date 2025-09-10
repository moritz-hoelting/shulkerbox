#![allow(clippy::module_name_repetitions)]

use std::{borrow::Cow, collections::HashSet, ops::Add};

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
    pub fn contains_macros(&self) -> bool {
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

    #[must_use]
    pub fn normalize(self) -> Self {
        match self {
            Self::String(_) => self,
            Self::MacroString(parts) => {
                let mut normalized_parts = Vec::new();

                for part in parts {
                    match part {
                        MacroStringPart::String(s) => {
                            if let Some(MacroStringPart::String(last)) = normalized_parts.last_mut()
                            {
                                last.push_str(&s);
                            } else if !s.is_empty() {
                                normalized_parts.push(MacroStringPart::String(s));
                            }
                        }
                        MacroStringPart::MacroUsage(m) => {
                            normalized_parts.push(MacroStringPart::MacroUsage(m));
                        }
                    }
                }

                if normalized_parts.len() == 1 {
                    if let MacroStringPart::String(s) = &normalized_parts[0] {
                        return Self::String(s.clone());
                    }
                }

                Self::MacroString(normalized_parts)
            }
        }
    }

    /// Returns the amount of lines the string has
    #[must_use]
    pub fn line_count(&self) -> usize {
        match self {
            Self::String(s) => s.split('\n').count(),
            Self::MacroString(parts) => {
                parts
                    .iter()
                    .map(|p| match p {
                        MacroStringPart::String(s) => s.split('\n').count() - 1,
                        MacroStringPart::MacroUsage(_) => 0,
                    })
                    .sum::<usize>()
                    + 1
            }
        }
    }

    /// Returns the names of the macros used in the [`MacroString`]
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        match self {
            Self::String(_) => HashSet::new(),
            Self::MacroString(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    MacroStringPart::String(_) => None,
                    MacroStringPart::MacroUsage(m) => Some(m.as_str()),
                })
                .collect(),
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

impl Add for MacroString {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::String(mut s1), Self::String(s2)) => {
                s1.push_str(&s2);
                Self::String(s1)
            }
            (Self::String(s1), Self::MacroString(s2)) => Self::MacroString(
                std::iter::once(MacroStringPart::String(s1))
                    .chain(s2)
                    .collect(),
            ),
            (Self::MacroString(mut s1), Self::String(s2)) => {
                s1.push(MacroStringPart::String(s2));
                Self::MacroString(s1)
            }
            (Self::MacroString(mut s1), Self::MacroString(s2)) => {
                s1.extend(s2);
                Self::MacroString(s1)
            }
        }
    }
}

impl Add<&str> for MacroString {
    type Output = Self;

    fn add(self, rhs: &str) -> Self::Output {
        match self {
            Self::String(mut s1) => {
                s1.push_str(rhs);
                Self::String(s1)
            }
            Self::MacroString(mut s1) => {
                s1.push(MacroStringPart::String(rhs.to_string()));
                Self::MacroString(s1)
            }
        }
    }
}
