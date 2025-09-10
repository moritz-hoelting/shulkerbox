//! Utility functions for the Shulkerbox project.

mod command_collection;
pub mod compile;
mod extendable_queue;
mod macro_string;
pub(crate) mod pack_format;

#[doc(inline)]
pub use extendable_queue::ExtendableQueue;

#[doc(inline)]
pub use macro_string::{MacroString, MacroStringPart};

#[doc(inline)]
pub use command_collection::CommandCollection;
