use std::path::Path;

// import the prelude to get all the necessary structs
use shulkerbox::prelude::*;

fn main() {
    let mut dp = Datapack::new("main", 20).with_supported_formats(16..=20);

    // get the namespace "test"
    let namespace = dp.namespace_mut("test");

    // get the function "foo" of the namespace "test" and add some commands
    let foo_function = namespace.function_mut("foo");

    let ex = Execute::If(
        Condition::from("entity A"),
        Box::new(Execute::Run(Box::new("say A".into()))),
        Some(Box::new(Execute::If(
            Condition::from("entity B"),
            Box::new(Execute::Run(Box::new("say B".into()))),
            Some(Box::new(Execute::Run(Box::new("say C".into())))),
        ))),
    );
    foo_function.add_command(ex);

    // compile the datapack
    let v_folder = dp.compile(&CompileOptions::default());

    // place the compiled datapack in the "./dist" folder
    v_folder.place(Path::new("./dist")).unwrap();
}
