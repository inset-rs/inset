//! The desktop binary. The example, and its entry point, live in the library.
//!
//! ```text
//! cargo run -p views
//! cargo waft run -d emulator-5554 -p views
//! ```

fn main() {
    // On Android the activity loads the library, whose host calls its `android_main`; there is no `main`.
    #[cfg(not(target_os = "android"))]
    views_lib::main();
}
