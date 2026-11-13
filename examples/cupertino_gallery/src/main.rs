//! The desktop binary. The gallery, and its entry point, live in the library.
//!
//! ```text
//! cargo run -p cupertino-gallery
//! cargo waft run -d ios -p cupertino-gallery
//! ```
#![windows_subsystem = "windows"]

fn main() {
    cupertino_gallery_lib::main();
}
