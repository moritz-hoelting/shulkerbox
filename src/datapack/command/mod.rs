//! Represents a command that can be included in a function.

mod execute;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::RangeInclusive,
    sync::LazyLock,
};

pub use execute::{Condition, Execute};

use chksum_md5 as md5;

use super::Function;
use crate::{
    prelude::Datapack,
    util::{
        compile::{CompileOptions, CompiledCommand, FunctionCompilerState, MutCompilerState},
        CommandCollection, MacroString, MacroStringPart,
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
    Group(Group),
    /// Comment to be added to the function
    Comment(String),
    /// Return value
    Return(ReturnCommand),
    /// While loop
    While(While),

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
            Self::Group(group) => group.compile(options, global_state, function_state),
            Self::Comment(comment) => {
                vec![CompiledCommand::new("#".to_string() + comment).with_forbid_prefix(true)]
            }
            Self::Return(return_cmd) => return_cmd.compile(options, global_state, function_state),
            Self::While(while_cmd) => while_cmd.compile(options, global_state, function_state),

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
            Self::Group(group) => group.get_count(options),
            Self::Return(_) => 1,
            Self::While(while_cmd) => while_cmd.get_count(options),

            Self::Concat(a, b) => a.get_count(options) + b.get_count(options) - 1,
        }
    }

    /// Check whether the command is valid with the given pack format.
    #[must_use]
    pub fn validate(&self, pack_formats: &RangeInclusive<u8>) -> bool {
        let command_valid = match self {
            Self::Comment(_) | Self::Debug(_) | Self::Group(_) | Self::While(_) => true,
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
            command_valid && !self.contains_macros()
        } else {
            command_valid
        }
    }

    /// Check whether the command contains a macro.
    #[must_use]
    pub fn contains_macros(&self) -> bool {
        match self {
            Self::Raw(_) | Self::Comment(_) => false,
            Self::UsesMacro(s) | Self::Debug(s) => s.contains_macros(),
            Self::Group(group) => group.contains_macros(),
            Self::Execute(ex) => ex.contains_macros(),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(value) => value.contains_macros(),
                ReturnCommand::Command(cmd) => cmd.contains_macros(),
            },
            Self::While(while_cmd) => while_cmd.contains_macros(),

            Self::Concat(a, b) => a.contains_macros() || b.contains_macros(),
        }
    }

    /// Returns the names of the macros used
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        match self {
            Self::Raw(_) | Self::Comment(_) => HashSet::new(),
            Self::UsesMacro(s) | Self::Debug(s) => s.get_macros(),
            Self::Group(group) => group.get_macros(),
            Self::Execute(ex) => ex.get_macros(),
            Self::Return(ret) => match ret {
                ReturnCommand::Value(value) => value.get_macros(),
                ReturnCommand::Command(cmd) => cmd.get_macros(),
            },
            Self::While(while_cmd) => while_cmd.get_macros(),

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
            Self::Raw(_)
            | Self::Debug(_)
            | Self::Execute(_)
            | Self::UsesMacro(_)
            | Self::While(_) => false,
            Self::Group(group) => group.forbid_prefix(),
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
            Self::Execute(exec) => exec.contains_return(),
            Self::Raw(cmd) => cmd.starts_with("return "),
            Self::UsesMacro(m) => m.compile().starts_with("return "),
            Self::Group(g) => g.contains_return(),
            Self::While(w) => w.contains_return(),

            Self::Concat(a, b) => a.contains_return() || b.contains_return(),
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

impl From<ReturnCommand> for Command {
    fn from(value: ReturnCommand) -> Self {
        Self::Return(value)
    }
}

impl From<Group> for Command {
    fn from(value: Group) -> Self {
        Self::Group(value)
    }
}

impl From<While> for Command {
    fn from(value: While) -> Self {
        Self::While(value)
    }
}

/// Represents a group of commands to be executed in sequence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The commands in the group.
    commands: Vec<Command>,
    /// Whether to always create a function for this group, even if it contains only one command.
    always_create_function: bool,
    /// Optional name for the data storage used for arguments.
    data_storage_name: Option<String>,
    /// Optional set of macros that should not be passed to the function, even though they are contained.
    /// This can be used together with `data_storage_name` to dynamically pass arguments to the function.
    block_pass_macros: Option<HashSet<String>>,
}

impl Group {
    /// Create a new group of commands.
    #[must_use]
    pub fn new(commands: Vec<Command>) -> Self {
        Self {
            commands,
            always_create_function: false,
            data_storage_name: None,
            block_pass_macros: None,
        }
    }

    #[must_use]
    pub fn with_always_create_function(mut self, always_create_function: bool) -> Self {
        self.always_create_function = always_create_function;
        self
    }

    #[must_use]
    pub fn with_block_pass_macros(mut self, block: HashSet<String>) -> Self {
        self.block_pass_macros = Some(block);
        self
    }

    #[must_use]
    pub fn with_data_storage_name(mut self, name: String) -> Self {
        self.data_storage_name = Some(name);
        self
    }

    /// Compile the execute command into a list of compiled commands.
    #[expect(clippy::too_many_lines)]
    #[tracing::instrument(skip_all, fields(commands = ?self.commands))]
    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<CompiledCommand> {
        let command_count = self
            .commands
            .iter()
            .map(|cmd| cmd.get_count(options))
            .sum::<usize>();
        // only create a function if there are more than one command
        match command_count {
            0 if !self.always_create_function => Vec::new(),
            1 if !self.always_create_function => {
                self.commands[0].compile(options, global_state, function_state)
            }
            _ => {
                let commands = &self.commands;
                let block_pass_macros = self.block_pass_macros.as_ref();
                let macro_data_storage_name = self.data_storage_name.as_deref();

                // calculate a hashed path for the function in the `sb` subfolder
                let function_path = generate_group_function_path(function_state);

                let namespace = function_state.namespace();

                let contained_macros = {
                    let mut macros = HashSet::new();
                    for cmd in commands {
                        macros.extend(cmd.get_macros());
                    }
                    macros
                };
                let (prepare_data_storage, function_invocation) = get_group_invocation_commands(
                    &function_path,
                    namespace,
                    commands,
                    &contained_macros,
                    block_pass_macros,
                    macro_data_storage_name,
                );

                create_group_function(&function_path, commands, function_state);

                let contains_return = commands.contains_return();

                let additional_return_cmds = if contains_return {
                    let full_path = format!(
                        "{namespace}:{function_path}",
                        namespace = function_state.namespace()
                    );
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

                if let Some((mut pre_cmds, post_cmds)) = additional_return_cmds {
                    if let Some(prepare_datastorage_cmd) = prepare_data_storage {
                        pre_cmds.extend(prepare_datastorage_cmd.compile(
                            options,
                            global_state,
                            function_state,
                        ));
                    }
                    pre_cmds.extend(function_invocation.compile(
                        options,
                        global_state,
                        function_state,
                    ));
                    pre_cmds.extend(post_cmds);
                    pre_cmds
                } else {
                    prepare_data_storage
                        .into_iter()
                        .flat_map(|prep| prep.compile(options, global_state, function_state))
                        .chain(function_invocation.compile(options, global_state, function_state))
                        .collect()
                }
            }
        }
    }

    /// Get a reference to the commands in the group.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Get a mutable reference to the commands in the group.
    #[must_use]
    pub fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    /// Get whether to always create a function for this group.
    #[must_use]
    pub fn always_create_function(&self) -> bool {
        self.always_create_function
    }

    /// Get the optional data storage name.
    #[must_use]
    pub fn data_storage_name(&self) -> Option<&String> {
        self.data_storage_name.as_ref()
    }

    /// Get the optional set of blocked macros.
    #[must_use]
    pub fn block_pass_macros(&self) -> Option<&HashSet<String>> {
        self.block_pass_macros.as_ref()
    }

    /// Check whether the group contains a macro.
    #[must_use]
    pub fn contains_macros(&self) -> bool {
        self.commands.contains_macros()
    }

    /// Check whether the group contains a return command.
    #[must_use]
    pub fn contains_return(&self) -> bool {
        self.commands.contains_return()
    }

    /// Returns the names of the macros used
    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        let mut macros = HashSet::new();
        for cmd in &self.commands {
            macros.extend(cmd.get_macros());
        }
        macros
    }

    /// Check whether the group should not have a prefix.
    #[must_use]
    pub fn forbid_prefix(&self) -> bool {
        self.commands.len() == 1 && self.commands[0].forbid_prefix()
    }

    /// Get the count of the commands this command will compile into.
    #[must_use]
    fn get_count(&self, options: &CompileOptions) -> usize {
        let command_count = self
            .commands
            .iter()
            .map(|cmd| cmd.get_count(options))
            .sum::<usize>();
        // only create a function if there are more than one command
        match command_count {
            0 if !self.always_create_function => 0,
            1 if !self.always_create_function => 1,
            _ => {
                let pass_macros = self.contains_macros();
                let contains_return = self.contains_return();

                let additional_return_cmds = if contains_return {
                    let post_cmd_store = Command::Execute(Execute::If(
                        Condition::Atom("".into()),
                        Box::new(Execute::Run(Box::new(Command::Raw(String::new())))),
                        None,
                    ))
                    .get_count(options);

                    let post_cmd_return = Command::Execute(Execute::If(
                        Condition::Atom("".into()),
                        Box::new(Execute::Run(Box::new(Command::Raw(String::new())))),
                        None,
                    ))
                    .get_count(options);

                    let post_cmds = post_cmd_store + post_cmd_return;

                    Some(post_cmds + 1)
                } else {
                    None
                };

                let prepare_data_storage = if pass_macros {
                    let contained_macros = self.get_macros();
                    let not_all_macros_blocked = self
                        .block_pass_macros
                        .as_ref()
                        .is_none_or(|b| contained_macros.iter().any(|&m| !b.contains(m)));

                    !contained_macros.is_empty()
                        && (self.data_storage_name.is_some() || not_all_macros_blocked)
                            & self.data_storage_name.is_some()
                } else {
                    false
                };

                additional_return_cmds.map_or_else(
                    || 1 + usize::from(prepare_data_storage),
                    |additional_return_cmds| {
                        additional_return_cmds + 1 + usize::from(prepare_data_storage)
                    },
                )
            }
        }
    }
}

impl Hash for Group {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.commands.hash(state);
        self.always_create_function.hash(state);
        if let Some(block) = &self.block_pass_macros {
            #[expect(clippy::collection_is_never_read)]
            let mut block_vec = block.iter().collect::<Vec<_>>();
            block_vec.sort();
            block_vec.hash(state);
        }
    }
}

fn get_group_invocation_commands(
    function_path: &str,
    namespace: &str,
    commands: &[Command],
    contained_macros: &HashSet<&str>,
    block_pass_macros: Option<&HashSet<String>>,
    macro_data_storage_name: Option<&str>,
) -> (Option<Command>, Command) {
    use std::fmt::Write as _;

    let mut function_invocation = format!("function {namespace}:{function_path}");

    if commands.contains_macros() {
        // WARNING: this seems to be the only way to pass macros to the function called.
        // Because everything is passed as a string, it looses one "level" of escaping per pass.
        let mut macros_parts = contained_macros
            .iter()
            .filter(|&&m| block_pass_macros.is_none_or(|b| !b.contains(m)))
            .flat_map(|&m| {
                vec![
                    MacroStringPart::String(format!(r#"{m}:""#)),
                    MacroStringPart::MacroUsage(m.to_string()),
                    MacroStringPart::String(r#"""#.to_string()),
                ]
            })
            .fold(Vec::new(), |mut acc, part| {
                if !acc.is_empty() {
                    acc.push(MacroStringPart::String(",".to_string()));
                }

                acc.push(part);

                acc
            });

        if let Some(data_storage_name) = macro_data_storage_name {
            let _ = write!(function_invocation, " with storage {data_storage_name}");

            let not_all_macros_blocked =
                block_pass_macros.is_none_or(|b| contained_macros.iter().any(|&m| !b.contains(m)));

            if not_all_macros_blocked {
                macros_parts.insert(
                    0,
                    MacroStringPart::String(format!("data merge storage {data_storage_name} {{")),
                );
                macros_parts.push(MacroStringPart::String("}".to_string()));
                let macro_string = MacroString::MacroString(macros_parts).normalize();

                (
                    Some(Command::UsesMacro(macro_string)),
                    Command::Raw(function_invocation),
                )
            } else {
                (None, Command::Raw(function_invocation))
            }
        } else {
            let _ = write!(function_invocation, " {{");
            macros_parts.insert(0, MacroStringPart::String(function_invocation));
            macros_parts.push(MacroStringPart::String("}".to_string()));

            let macro_string = MacroString::MacroString(macros_parts).normalize();

            (None, Command::UsesMacro(macro_string))
        }
    } else {
        (None, Command::Raw(function_invocation))
    }
}

fn create_group_function(
    function_path: &str,
    commands: &[Command],
    function_state: &FunctionCompilerState,
) {
    let namespace = function_state.namespace();

    // create a new function with the commands
    let mut function = Function::new(namespace, function_path);
    function.get_commands_mut().extend(commands.iter().cloned());
    function_state.add_function(function_path, function);
}

#[must_use]
fn generate_group_function_path(function_state: &FunctionCompilerState) -> String {
    let uid = function_state.request_uid();
    let function_path = function_state.path();
    let function_path = function_path.strip_prefix("sb/").unwrap_or(function_path);

    let pre_hash_path = function_path.to_owned() + ":" + &uid.to_string();
    let hash = md5::hash(pre_hash_path).to_hex_lowercase();

    "sb/".to_string() + function_path + "/" + &hash[..16]
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
                let compiled_cmd = Command::Group(Group::new(vec![*cmd.clone()])).compile(
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
                    Box::new(Execute::Run(Box::new(Command::Group(Group::new(vec![
                        *cmd.clone(),
                    ]))))),
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

/// Loops the commands while a condition is true.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct While {
    condition: Condition,
    commands: Vec<Command>,
}

impl While {
    pub fn new(condition: impl Into<Condition>, commands: Vec<Command>) -> Self {
        Self {
            condition: condition.into(),
            commands,
        }
    }

    pub fn compile(
        &self,
        options: &CompileOptions,
        global_state: &MutCompilerState,
        function_state: &FunctionCompilerState,
    ) -> Vec<CompiledCommand> {
        // calculate a hashed path for the function in the `sb` subfolder
        let function_path = generate_group_function_path(function_state);

        let namespace = function_state.namespace();

        let contained_macros = {
            let mut macros = HashSet::new();
            for cmd in &self.commands {
                macros.extend(cmd.get_macros());
            }
            macros
        };
        let (prepare_data_storage, function_invocation) = get_group_invocation_commands(
            &function_path,
            namespace,
            &self.commands,
            &contained_macros,
            None,
            None,
        );

        let execute_tail = if let Some(prepare_datastorage_cmd) = prepare_data_storage {
            Execute::Runs(vec![prepare_datastorage_cmd, function_invocation])
        } else {
            Execute::Run(Box::new(function_invocation))
        };
        let conditional_run_cmd = Command::Execute(Execute::If(
            self.condition.clone(),
            Box::new(execute_tail),
            None,
        ));

        let mut commands = self.commands.clone();
        commands.push(conditional_run_cmd.clone());

        create_group_function(&function_path, &commands, function_state);

        let contains_return = commands.contains_return();

        if contains_return {
            todo!("While loops with return commands are not yet supported");
        }

        conditional_run_cmd.compile(options, global_state, function_state)
    }

    #[must_use]
    pub fn get_count(&self, options: &CompileOptions) -> usize {
        Execute::If(
            self.condition.clone(),
            Box::new(Execute::Run(Box::new(Command::Raw(String::new())))),
            None,
        )
        .get_count(options)
    }

    #[must_use]
    pub fn contains_macros(&self) -> bool {
        self.condition.contains_macros() || self.commands.contains_macros()
    }

    #[must_use]
    pub fn get_macros(&self) -> HashSet<&str> {
        let mut macros = self.condition.get_macros();
        for cmd in &self.commands {
            macros.extend(cmd.get_macros());
        }
        macros
    }

    #[must_use]
    pub fn contains_return(&self) -> bool {
        self.commands.contains_return()
    }
}

#[allow(clippy::too_many_lines)]
fn validate_raw_cmd(cmd: &str, pack_formats: &RangeInclusive<u8>) -> bool {
    static CMD_FORMATS: LazyLock<HashMap<&str, RangeInclusive<u8>>> = LazyLock::new(|| {
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
        CMD_FORMATS.get(cmd).is_none_or(|range| {
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
