# Shulkerbox-rs

Create datapacks with all the features of rust you like.

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
let mut namespace = Namespace::new("shulker"); // Create a new namespace with the name "shulker"

let mut hello_function = Function::new(); // Create a new function
hello_function.add_command("say Hello, world!".into()); // Add a command to the function
namespace.add_function("hello", hello_function); // Add the function to the namespace

dp.add_namespace(namespace); // Add the namespace to the datapack

let v_folder = dp.compile(&CompileOptions::default()); // Compile the datapack with default options

v_folder.place(Path::new("./dist")).unwrap(); // Place the datapack in the dist folder
v_folder.zip(Path::new("./dist.zip")).unwrap(); // Zip the datapack to the dist.zip file
```