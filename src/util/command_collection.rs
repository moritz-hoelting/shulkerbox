use std::collections::HashSet;

use crate::prelude::Command;

/// A trait for collections that can hold `Command` items.
pub trait CommandCollection {
    /// Returns an iterator over the commands in the collection.
    fn commands(&self) -> impl Iterator<Item = &Command>;

    /// Checks if any command in the collection contains macros.
    fn contains_macros(&self) -> bool {
        self.commands().any(Command::contains_macros)
    }

    /// Returns a set of all macro names used in the commands of the collection.
    fn get_macros(&self) -> HashSet<&str> {
        self.commands()
            .flat_map(Command::get_macros)
            .collect::<HashSet<&str>>()
    }

    /// Checks if any command in the collection is a return command.
    fn contains_return(&self) -> bool {
        self.commands().any(Command::contains_return)
    }
}

impl<C> CommandCollection for C
where
    C: AsRef<[Command]>,
{
    fn commands(&self) -> impl Iterator<Item = &Command> {
        self.as_ref().iter()
    }
}
