use std::path::Path;

use shulkerbox::{
    datapack::{Command, Condition, Datapack, Execute},
    util::compile::CompileOptions,
};

fn main() {
    let mut dp = Datapack::new(16).with_supported_formats(16..=20);
    let namespace = dp.namespace_mut("test");

    let foo_function = namespace.function_mut("foo");
    foo_function.add_command("say Hello, world!");
    foo_function.add_command(Command::Debug("debug message".into()));

    let call_func = Command::from(foo_function);
    let bar_function = namespace.function_mut("bar");
    bar_function.add_command(call_func);
    bar_function.add_command(Command::Execute(Execute::As(
        "@a".to_string(),
        Box::new(Execute::If(
            Condition::from("block ~ ~ ~ minecraft:stone")
                | !(!Condition::from("block ~ ~1 ~ minecraft:stone")
                    | "block ~ ~-1 ~ minecraft:stone".into()),
            Box::new(Execute::Run(Box::new("say bar".into()))),
            None,
        )),
    )));

    namespace.get_main_function_mut().add_command("say tick");

    let v_folder = dp.compile(&CompileOptions::default());

    v_folder.place(Path::new("./dist")).unwrap();
}
