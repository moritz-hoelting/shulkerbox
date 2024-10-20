//! Shulkerbox is a library for creating Minecraft data packs.

#![deny(
    unsafe_code,
    missing_debug_implementations,
    missing_copy_implementations,
    clippy::nursery,
    rustdoc::broken_intra_doc_links,
    clippy::missing_errors_doc
)]
#![warn(clippy::all, clippy::pedantic, clippy::perf)]
#![allow(clippy::missing_panics_doc, clippy::missing_const_for_fn)]

pub mod datapack;
pub mod util;
pub mod virtual_fs;

/// Prelude for shulkerbox.
///
/// This module contains the most common types and traits that you may want to import.
pub mod prelude {
    pub use crate::{
        datapack::{Command, Condition, Datapack, Execute},
        util::compile::CompileOptions,
    };
}
