use std::path::Path;

// import the prelude to get all the necessary structs
use shulkerbox::prelude::*;

fn main() {
    // create a new datapack
    let mut dp = Datapack::new(16).with_supported_formats(16..=20);

    // get the namespace "test"
    let namespace = dp.namespace_mut("test");

    // get the function "foo" of the namespace "test" and add some commands
    let foo_function = namespace.function_mut("foo");
    foo_function.add_command("say Hello, world!");
    foo_function.add_command(Command::Debug("debug message".into()));

    // get a call command to the function "foo"
    let call_func = Command::from(foo_function);
    let bar_function = namespace.function_mut("bar");
    // add the call command to the function "bar"
    bar_function.add_command(call_func);
    // add a complex command to the function "bar"
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

    dp.add_load("test:foo");

    // compile the datapack
    let v_folder = dp.compile(&CompileOptions::default());

    // place the compiled datapack in the "./dist" folder
    v_folder.place(Path::new("./dist")).unwrap();
}
