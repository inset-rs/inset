//! The desktop binary. The app, and its entry point, live in the library.
//!
//! ```text
//! cargo run -p shaders
//! ```
#![windows_subsystem = "windows"]

fn main() {
    shaders_lib::main();
}
