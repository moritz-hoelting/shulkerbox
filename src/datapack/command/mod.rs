//! Represents a command that can be included in a function.

mod execute;
use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
    sync::OnceLock,
};

pub use execute::{Condition, Execute};

use chksum_md5 as md5;

use super::Function;
use crate::{
    prelude::Datapack,
    util::{
        compile::{CompileOptions, CompiledCommand, FunctionCompilerState, MutCompilerState},
        MacroString,
    },
};

/// Represents a command that can be included in a function.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    /// A command that is already formatted as a string.
    Raw(String),
    /// A command that contains macro usages
    UsesMacro(MacroString),
    /// Message to be printed only in debug mode
    Debug(MacroString),
    /// Execute command
    Execute(Execute),
    /// Group of commands to be called instantly after each other
    Group(Vec<Command>),
    /// Comment to be added to the function
    Comment(String),
    /// Return value
    Return(ReturnCommand),

    /// Command that is a concatenation of two commands
    Concat(Box<Command>, Box<Command>),
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
    ) -> Vec<CompiledCommand> {
        match self {
            Self::Raw(command) => vec![CompiledCommand::new(command.clone())],
            Self::UsesMacro(command) => {
                vec![CompiledCommand::new(command.compile()).with_contains_macros(true)]
            }
            Self::Debug(message) => compile_debug(message, options),
            Self::Execute(ex) => ex.compile(options, global_state, function_state),
            Self::Group(commands) => compile_group(commands, options, global_state, function_state),
            Self::Comment(comment) => {
                vec![CompiledCommand::new("#".to_string() + comment).with_forbid_prefix(true)]
            }
            Self::Return(return_cmd) => return_cmd.compile(options, global_state, function_state),
            Self::Concat(a, b) => {
                let a = a.compile(options, global_state, function_state);
                let b = b.compile(options, global_state, function_state);
                a.into_iter()
                    .flat_map(|a| {
                        b.iter().map(move |b| {
                            if a.is_empty() {
                                b.clone()
                            } else if b.is_empty() {
                                a.clone()
                            } else {
                                b.clone()
                                    .apply_prefix(a.as_str())
                                    .or_forbid_prefix(a.forbids_prefix())
                            }
                        })
                    })
                    .collect()
            }
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
            Self::UsesMacro(cmd) => cmd.line_count(),
            Self::Execute(ex) => ex.get_count(options),
            Self::Group(_) | Self::Return(_) => 1,
            Self::Concat(a, b) => a.get_count(options) + b.get_count(options) - 1,
        }
    }

    /// Check whether the command is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        let command_valid = match self {
            Self::Comment(_) | Self::Debug(_) | Self::Group(_) => true,
            Self::Raw(cmd) => validate_raw_cmd(cmd, pack_formats),
            Self::UsesMacro(cmd) => validate_raw_cmd(&cmd.compile(), pack_formats),
            Self::Execute(ex) => ex.validate(pack_formats),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(_) => pack_formats.start() >= &14,
                ReturnCommand::Command(cmd) => {
                    pack_formats.start() >= &16 && cmd.validate(pack_formats)
                }
            },
            Self::Concat(a, b) => a.validate(pack_formats) && b.validate(pack_formats),
        };
        if pack_formats.start() < &16 {
            command_valid && !self.contains_macro()
        } else {
            command_valid
        }
    }

    /// Check whether the command contains a macro.
    #[must_use]
    pub fn contains_macro(&self) -> bool {
        match self {
            Self::Raw(_) | Self::Comment(_) => false,
            Self::UsesMacro(s) | Self::Debug(s) => s.contains_macro(),
            Self::Group(commands) => group_contains_macro(commands),
            Self::Execute(ex) => ex.contains_macro(),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(value) => value.contains_macro(),
                ReturnCommand::Command(cmd) => cmd.contains_macro(),
            },
            Self::Concat(a, b) => a.contains_macro() || b.contains_macro(),
        }
    }

    /// Returns the names of the macros used
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        match self {
            Self::Raw(_) | Self::Comment(_) => HashSet::new(),
            Self::UsesMacro(s) | Self::Debug(s) => s.get_macros(),
            Self::Group(commands) => group_get_macros(commands),
            Self::Execute(ex) => ex.get_macros(),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(value) => value.get_macros(),
                ReturnCommand::Command(cmd) => cmd.get_macros(),
            },
            Self::Concat(a, b) => {
                let mut macros = a.get_macros();
                macros.extend(b.get_macros());
                macros
            }
        }
    }

    /// Check whether the command should not have a prefix.
    #[must_use]
    pub fn forbid_prefix(&self) -> bool {
        match self {
            Self::Comment(_) => true,
            Self::Raw(_) | Self::Debug(_) | Self::Execute(_) | Self::UsesMacro(_) => false,
            Self::Group(commands) => commands.len() == 1 && commands[0].forbid_prefix(),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(_) => false,
                ReturnCommand::Command(cmd) => cmd.forbid_prefix(),
            },
            Self::Concat(a, _) => a.forbid_prefix(),
        }
    }

    // Check whether the command contains a return command.
    #[must_use]
    pub fn contains_return(&self) -> bool {
        match self {
            Self::Comment(_) | Self::Debug(_) => false,
            Self::Return(_) => true,
            Self::Concat(a, b) => a.contains_return() || b.contains_return(),
            Self::Execute(exec) => exec.contains_return(),
            Self::Raw(cmd) => cmd.starts_with("return "),
            Self::UsesMacro(m) => m.compile().starts_with("return "),
            Self::Group(g) => g.iter().any(Self::contains_return),
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

/// Represents a command that returns a value.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReturnCommand {
    /// Returns the value
    Value(MacroString),
    /// Returns the result of the command
    Command(Box<Command>),
}

fn compile_debug(message: &MacroString, option: &CompileOptions) -> Vec<CompiledCommand> {
    if option.debug {
        vec![CompiledCommand::new(format!(
            r#"tellraw @a [{{"text":"[","color":"dark_blue"}},{{"text":"DEBUG","color":"dark_green","hoverEvent":{{"action":"show_text","value":[{{"text":"Debug message generated by Shulkerbox"}},{{"text":"\nSet debug to 'false' to disable"}}]}}}},{{"text":"] ","color":"dark_blue"}},{{"text":"{}","color":"black"}}]"#,
            message.compile()
        ))]
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
) -> Vec<CompiledCommand> {
    let command_count = commands
        .iter()
        .map(|cmd| cmd.get_count(options))
        .sum::<usize>();
    // only create a function if there are more than one command
    match command_count {
        0 => Vec::new(),
        1 => commands[0].compile(options, global_state, function_state),
        _ => {
            let pass_macros = group_contains_macro(commands);
            let contains_return = commands.iter().any(Command::contains_return);

            // calculate a hashed path for the function in the `sb` subfolder
            let function_path = generate_group_function_path(function_state);

            let namespace = function_state.namespace();

            // create a new function with the commands
            let mut function = Function::new(namespace, &function_path);
            function.get_commands_mut().extend(commands.iter().cloned());
            function_state.add_function(&function_path, function);

            let mut function_invocation = format!("function {namespace}:{function_path}");

            let additional_return_cmds = if contains_return {
                let full_path = format!("{namespace}:{function_path}");
                let return_data_path = md5::hash(&full_path).to_hex_lowercase();

                let pre_cmds = Command::Raw(format!(
                    "data remove storage shulkerbox:return {return_data_path}"
                ))
                .compile(options, global_state, function_state)
                .into_iter()
                .map(|c| c.with_forbid_prefix(true))
                .collect::<Vec<_>>();
                let post_condition = Condition::Atom(
                    format!("data storage shulkerbox:return {return_data_path}").into(),
                );

                let post_cmd_store = global_state
                    .read()
                    .unwrap()
                    .functions_with_special_return
                    .get(&format!(
                        "{}:{}",
                        function_state.namespace(),
                        function_state.path()
                    ))
                    .cloned().map(|parent_return_data_path| {
                        Command::Execute(Execute::If(
                        post_condition.clone(),
                        Box::new(Execute::Run(Box::new(Command::Raw(format!(
                            "data modify storage shulkerbox:return {parent_return_data_path} set from storage shulkerbox:return {return_data_path}"
                        ))))),
                        None,
                    ))
                    });

                let post_cmd_return = Command::Execute(Execute::If(
                    post_condition,
                    Box::new(Execute::Run(Box::new(Command::Raw(format!(
                        "return run data get storage shulkerbox:return {return_data_path}"
                    ))))),
                    None,
                ));

                let post_cmds = post_cmd_store
                    .into_iter()
                    .chain(std::iter::once(post_cmd_return))
                    .flat_map(|cmd| cmd.compile(options, global_state, function_state))
                    .map(|c| c.with_forbid_prefix(true))
                    .collect::<Vec<_>>();

                global_state
                    .write()
                    .unwrap()
                    .functions_with_special_return
                    .insert(full_path, return_data_path);

                Some((pre_cmds, post_cmds))
            } else {
                None
            };

            if pass_macros {
                // WARNING: this seems to be the only way to pass macros to the function called.
                // Because everything is passed as a string, it looses one "level" of escaping per pass.
                let macros_block = group_get_macros(commands)
                    .into_iter()
                    .map(|m| format!(r#"{m}:"$({m})""#))
                    .collect::<Vec<_>>()
                    .join(",");
                function_invocation.push_str(&format!(" {{{macros_block}}}"));
            }

            if let Some((mut pre_cmds, post_cmds)) = additional_return_cmds {
                pre_cmds.push(
                    CompiledCommand::new(function_invocation).with_contains_macros(pass_macros),
                );
                pre_cmds.extend(post_cmds);
                pre_cmds
            } else {
                vec![CompiledCommand::new(function_invocation).with_contains_macros(pass_macros)]
            }
        }
    }
}

fn generate_group_function_path(function_state: &FunctionCompilerState) -> String {
    let uid = function_state.request_uid();
    let function_path = function_state.path();
    let function_path = function_path.strip_prefix("sb/").unwrap_or(function_path);

    let pre_hash_path = function_path.to_owned() + ":" + &uid.to_string();
    let hash = md5::hash(pre_hash_path).to_hex_lowercase();

    "sb/".to_string() + function_path + "/" + &hash[..16]
}

fn group_contains_macro(commands: &[Command]) -> bool {
    commands.iter().any(Command::contains_macro)
}

fn group_get_macros(commands: &[Command]) -> HashSet<&str> {
    let mut macros = HashSet::new();
    for cmd in commands {
        macros.extend(cmd.get_macros());
    }
    macros
}

impl ReturnCommand {
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<CompiledCommand> {
        let return_data_path = global_state
            .read()
            .unwrap()
            .functions_with_special_return
            .get(&format!(
                "{}:{}",
                function_state.namespace(),
                function_state.path()
            ))
            .cloned();
        match (self, return_data_path) {
            (Self::Value(value), None) => {
                vec![CompiledCommand::new(format!("return {}", value.compile()))]
            }
            (Self::Value(value), Some(data_path)) => {
                let value = value.compile();
                let store_cmd = CompiledCommand::new(format!(
                    "data modify storage shulkerbox:return {data_path} set value {value}",
                ));
                let return_cmd = CompiledCommand::new(format!("return {value}"));
                vec![store_cmd, return_cmd]
            }
            (Self::Command(cmd), None) => {
                let compiled_cmd = Command::Group(vec![*cmd.clone()]).compile(
                    options,
                    global_state,
                    function_state,
                );
                let compiled_cmd = compiled_cmd
                    .into_iter()
                    .next()
                    .expect("group will always return exactly one command");
                vec![compiled_cmd.apply_prefix("return run ")]
            }
            (Self::Command(cmd), Some(data_path)) => {
                let compiled_cmd = Command::Execute(Execute::Store(
                    format!("result storage shulkerbox:return {data_path} int 1.0").into(),
                    Box::new(Execute::Run(Box::new(Command::Group(vec![*cmd.clone()])))),
                ))
                .compile(options, global_state, function_state);
                let compiled_cmd = compiled_cmd
                    .into_iter()
                    .next()
                    .expect("group will always return exactly one command");
                let return_cmd = CompiledCommand::new(format!(
                    "return run data get storage shulkerbox:return {data_path} value"
                ));
                vec![compiled_cmd, return_cmd]
            }
        }
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

    cmd.split_ascii_whitespace().next().is_none_or(|cmd| {
        cmd_formats.get(cmd).is_none_or(|range| {
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
    use std::sync::RwLock;

    use crate::util::compile::CompilerState;

    use super::*;

    #[test]
    fn test_raw() {
        let command_a = Command::Raw("say Hello, world!".to_string());
        let command_b = Command::raw("say foo bar");

        let options = &CompileOptions::default();
        let global_state = &RwLock::new(CompilerState::default());
        let function_state = &FunctionCompilerState::default();

        assert_eq!(
            command_a.compile(options, global_state, function_state),
            vec![CompiledCommand::new("say Hello, world!")]
        );
        assert_eq!(command_a.get_count(options), 1);
        assert_eq!(
            command_b.compile(options, global_state, function_state),
            vec![CompiledCommand::new("say foo bar")]
        );
        assert_eq!(command_b.get_count(options), 1);
    }

    #[test]
    fn test_comment() {
        let comment = Command::Comment("this is a comment".to_string());

        let options = &CompileOptions::default();
        let global_state = &RwLock::new(CompilerState::default());
        let function_state = &FunctionCompilerState::default();

        assert_eq!(
            comment.compile(options, global_state, function_state),
            vec![CompiledCommand::new("#this is a comment").with_forbid_prefix(true)]
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
