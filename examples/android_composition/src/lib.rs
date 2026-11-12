//! Android's compositor under Waft's picture. A producer thread draws a moving stripe into
//! hardware buffers in turn, one frame at each refresh of the screen, which it waits for on the
//! platform's vsync waiter, and presents them to the layer a `CompositionLayerView` shows; the
//! framework moves the layer to another place every second, through a clip that cuts its left
//! edge off, and paints a label above it, which shows that its own content stays on top. In the
//! corner a buffer drawn once shows as an image, imported on the renderer's device
//! (`Platform::import_hardware_buffer`). Once a second the producer writes to the system's log
//! what it counted, under the tag `android_composition`, so the compositor can be checked without
//! looking: `adb logcat -s android_composition`.
//!
//! ```text
//! cargo waft run -d emulator-5554 -p android_composition
//! ```
//!
//! Elsewhere it says so and quits: what it shows is Android's.
#![feature(arbitrary_self_types)]
// Whether the producer's state is `Send` reaches past the default depth through wgpu's types.
#![cfg_attr(target_os = "android", recursion_limit = "256")]

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
mod producer;

#[waft::main]
fn main(app: &mut waft::App) {
    #[cfg(target_os = "android")]
    android::run(app);
    #[cfg(not(target_os = "android"))]
    elsewhere(app);
}

/// Says that the example is Android's, and quits.
#[cfg(not(target_os = "android"))]
fn elsewhere(app: &mut waft::App) {
    eprintln!("android_composition: the compositor this shows is Android's; run it there");
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}
