# Shulkerbox-rs

Create datapacks with all the features of rust you like.

## Usage

Add the following to your dependencies in `Cargo.toml`:
```toml
[dependencies]
shulkerbox = "0.1.0"
```

## Example Usage

```rust
use shulkerbox::{
    datapack::{Datapack, Function, Namespace},
    util::compile::CompileOptions,
};

let mut dp = Datapack::new("shulkerpack", 20) // Create a new datapack with the name "shulkerpack" and the pack format 20
    .with_description("I created this datapack with rust") // Add a description to the datapack
    .with_supported_formats(16..=20) // Add the supported formats of the datapack
    .with_template_folder(Path::new("./template")) // Add a template folder to the datapack. This will include all files in the template folder in the root of the datapack and can be used for including the "pack.png" file
    .unwrap();
let namespace = datapack.namespace_mut("shulker"); // Create a new namespace with the name "shulker"

let hello_function = namespace.function_mut("hello"); // Create a new function
hello_function.add_command("say Hello, world!"); // Add a command to the function

let v_folder = dp.compile(&CompileOptions::default()); // Compile the datapack with default options

v_folder.place(Path::new("./dist")).unwrap(); // Place the datapack in the dist folder
v_folder.zip(Path::new("./dist.zip")).unwrap(); // Zip the datapack to the dist.zip file
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.