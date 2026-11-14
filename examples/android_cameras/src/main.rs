//! The desktop binary, which says the example is Android's. The example, and its entry point,
//! live in the library.
//!
//! ```text
//! cargo waft run -d emulator-5554 -p android_cameras
//! ```

fn main() {
    // On Android the activity loads the library, whose host calls its `android_main`; there is no `main`.
    #[cfg(not(target_os = "android"))]
    android_cameras_lib::main();
}
