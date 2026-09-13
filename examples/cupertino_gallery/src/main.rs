//! The desktop binary. The gallery, and its entry point, live in the library.
//!
//! ```text
//! cargo run -p cupertino-gallery
//! cargo inset run -d ios -p cupertino-gallery
//! ```

fn main() {
    cupertino_gallery::main();
}
