/// Get the name of the function directory depending on the pack format.
#[must_use]
pub const fn function_directory_name(pack_format: u8) -> &'static str {
    if pack_format < 45 {
        "functions"
    } else {
        "function"
    }
}
