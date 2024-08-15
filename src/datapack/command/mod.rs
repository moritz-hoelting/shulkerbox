//! Represents a command that can be included in a function.

mod execute;
use std::{collections::HashMap, ops::RangeInclusive, sync::OnceLock};

pub use execute::{Condition, Execute};

use chksum_md5 as md5;

use super::Function;
use crate::{
    prelude::Datapack,
    util::compile::{CompileOptions, FunctionCompilerState, MutCompilerState},
};

/// Represents a command that can be included in a function.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    /// A command that is already formatted as a string.
    Raw(String),
    /// Message to be printed only in debug mode
    Debug(String),
    /// Execute command
    Execute(Execute),
    /// Group of commands to be called instantly after each other
    Group(Vec<Command>),
    /// Comment to be added to the function
    Comment(String),
}

impl Command {
    /// Create a new raw command.
    #[must_use]
    pub fn raw(command: &str) -> Self {
        Self::Raw(command.to_string())
    }

    /// Compile the command into a string.
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<String> {
        match self {
            Self::Raw(command) => vec![command.clone()],
            Self::Debug(message) => compile_debug(message, options),
            Self::Execute(ex) => ex.compile(options, global_state, function_state),
            Self::Group(commands) => compile_group(commands, options, global_state, function_state),
            Self::Comment(comment) => vec!["#".to_string() + comment],
        }
    }

    /// Get the count of the commands this command will compile into.
    #[must_use]
    fn get_count(&self, options: &CompileOptions) -> usize {
        match self {
            // TODO: change comment to compile to `1`, make sure nothing breaks
            Self::Comment(_) => 0,
            Self::Debug(_) => usize::from(options.debug),
            Self::Raw(cmd) => cmd.split('\n').count(),
            Self::Execute(ex) => ex.get_count(options),
            Self::Group(_) => 1,
        }
    }

    /// Check whether the command is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        match self {
            Self::Comment(_) | Self::Debug(_) | Self::Group(_) => true,
            Self::Raw(cmd) => validate_raw_cmd(cmd, pack_formats),
            Self::Execute(ex) => ex.validate(pack_formats),
        }
    }
}

impl From<&str> for Command {
    fn from(command: &str) -> Self {
        Self::raw(command)
    }
}
impl From<&Function> for Command {
    fn from(value: &Function) -> Self {
        Self::Raw(format!("function {}:{}", value.namespace(), value.name()))
    }
}
impl From<&mut Function> for Command {
    fn from(value: &mut Function) -> Self {
        Self::Raw(format!("function {}:{}", value.namespace(), value.name()))
    }
}

fn compile_debug(message: &str, option: &CompileOptions) -> Vec<String> {
    if option.debug {
        vec![format!(
            r#"tellraw @a [{{"text":"[","color":"dark_blue"}},{{"text":"DEBUG","color":"dark_green","hoverEvent":{{"action":"show_text","value":[{{"text":"Debug message generated by Shulkerbox"}},{{"text":"\nSet debug message to 'false' to disable"}}]}}}},{{"text":"]","color":"dark_blue"}},{{"text":" {}","color":"black"}}]"#,
            message
        )]
    } else {
        Vec::new()
    }
}

#[tracing::instrument(skip_all, fields(commands = ?commands))]
fn compile_group(
    commands: &[Command],
    options: &CompileOptions,
    global_state: &MutCompilerState,
    function_state: &FunctionCompilerState,
) -> Vec<String> {
    let command_count = commands
        .iter()
        .map(|cmd| cmd.get_count(options))
        .sum::<usize>();
    // only create a function if there are more than one command
    if command_count > 1 {
        let uid = function_state.request_uid();

        // calculate a hashed path for the function in the `sb` subfolder
        let function_path = {
            let function_path = function_state.path();
            let function_path = function_path.strip_prefix("sb/").unwrap_or(function_path);

            let pre_hash_path = function_path.to_owned() + ":" + &uid.to_string();
            let hash = md5::hash(pre_hash_path).to_hex_lowercase();

            "sb/".to_string() + function_path + "/" + &hash[..16]
        };

        let namespace = function_state.namespace();

        // create a new function with the commands
        let mut function = Function::new(namespace, &function_path);
        function.get_commands_mut().extend(commands.iter().cloned());
        function_state.add_function(&function_path, function);

        vec![format!("function {namespace}:{function_path}")]
    } else {
        commands
            .iter()
            .flat_map(|cmd| cmd.compile(options, global_state, function_state))
            .collect::<Vec<_>>()
    }
}

#[allow(clippy::too_many_lines)]
fn validate_raw_cmd(cmd: &str, pack_formats: &RangeInclusive<u8>) -> bool {
    static CMD_FORMATS: OnceLock<HashMap<&str, RangeInclusive<u8>>> = OnceLock::new();
    let cmd_formats = CMD_FORMATS.get_or_init(|| {
        const LATEST: u8 = Datapack::LATEST_FORMAT;
        const ANY: RangeInclusive<u8> = 0..=LATEST;
        const fn to(to: u8) -> RangeInclusive<u8> {
            0..=to
        }
        const fn from(from: u8) -> RangeInclusive<u8> {
            from..=LATEST
        }

        const ANY_CMD: &[&str] = &[
            "advancement",
            "ban",
            "ban-ip",
            "banlist",
            "clear",
            "clone",
            "debug",
            "defaultgamemode",
            "deop",
            "difficulty",
            "effect",
            "enchant",
            "execute",
            "experience",
            "fill",
            "gamemode",
            "gamerule",
            "give",
            "help",
            "kick",
            "kill",
            "list",
            "locate",
            "me",
            "msg",
            "op",
            "pardon",
            "pardon-ip",
            "particle",
            "playsound",
            "publish",
            "recipe",
            "reload",
            "save-all",
            "save-off",
            "save-on",
            "say",
            "scoreboard",
            "seed",
            "setblock",
            "setidletimeout",
            "setworldspawn",
            "spawnpoint",
            "spreadplayers",
            "stop",
            "stopsound",
            "summon",
            "teleport",
            "tell",
            "tellraw",
            "time",
            "title",
            "tp",
            "trigger",
            "w",
            "weather",
            "whitelist",
            "worldborder",
            "xp",
        ];

        let mut map = HashMap::new();

        for cmd in ANY_CMD {
            map.insert(*cmd, ANY);
        }
        map.insert("attribute", from(6));
        map.insert("bossbar", from(4));
        map.insert("damage", from(12));
        map.insert("data", from(4));
        map.insert("datapack", from(4));
        map.insert("fillbiome", from(12));
        map.insert("forceload", from(4));
        map.insert("function", from(4));
        map.insert("replaceitem", to(6));
        map.insert("item", from(7));
        map.insert("jfr", from(8));
        map.insert("loot", from(4));
        map.insert("perf", from(7));
        map.insert("place", from(10));
        map.insert("placefeature", 9..=9);
        map.insert("random", from(18));
        map.insert("return", from(15));
        map.insert("ride", from(12));
        map.insert("schedule", from(4));
        map.insert("spectate", from(5));
        map.insert("tag", from(4));
        map.insert("team", from(4));
        map.insert("teammsg", from(4));
        map.insert("tick", from(22));
        map.insert("tm", from(4));
        map.insert("transfer", from(41));

        map
    });

    cmd.split_ascii_whitespace().next().map_or(true, |cmd| {
        cmd_formats.get(cmd).map_or(true, |range| {
            let start_cmd = range.start();
            let end_cmd = range.end();

            let start_pack = pack_formats.start();
            let end_pack = pack_formats.end();

            start_cmd <= start_pack && end_cmd >= end_pack
        })
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::util::compile::CompilerState;

    use super::*;

    #[test]
    fn test_raw() {
        let command_a = Command::Raw("say Hello, world!".to_string());
        let command_b = Command::raw("say foo bar");

        let options = &CompileOptions::default();
        let global_state = &Mutex::new(CompilerState::default());
        let function_state = &FunctionCompilerState::default();

        assert_eq!(
            command_a.compile(options, global_state, function_state),
            vec!["say Hello, world!".to_string()]
        );
        assert_eq!(command_a.get_count(options), 1);
        assert_eq!(
            command_b.compile(options, global_state, function_state),
            vec!["say foo bar".to_string()]
        );
        assert_eq!(command_b.get_count(options), 1);
    }

    #[test]
    fn test_comment() {
        let comment = Command::Comment("this is a comment".to_string());

        let options = &CompileOptions::default();
        let global_state = &Mutex::new(CompilerState::default());
        let function_state = &FunctionCompilerState::default();

        assert_eq!(
            comment.compile(options, global_state, function_state),
            vec!["#this is a comment".to_string()]
        );
        assert_eq!(comment.get_count(options), 0);
    }

    #[test]
    fn test_validate() {
        let tag = Command::raw("tag @s add foo");

        assert!(tag.validate(&(6..=9)));
        assert!(!tag.validate(&(2..=5)));

        let kill = Command::raw("kill @p");

        assert!(kill.validate(&(2..=40)));
    }
}
