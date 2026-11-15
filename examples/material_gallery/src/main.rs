//! The desktop binary. The gallery, and its entry point, live in the library.
//!
//! ```text
//! cargo run -p material-gallery
//! cargo waft run -d android -p material-gallery
//! ```
#![windows_subsystem = "windows"]

fn main() {
    material_gallery_lib::main();
}
