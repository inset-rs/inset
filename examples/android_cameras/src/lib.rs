//! Android's cameras as a Waft app has them (`Platform::cameras`): the cameras listed, the first
//! opened, which asks the user for the camera the first time, and started in its largest format
//! up to 1280 by 720 with its live picture on a composition layer, its frames in a reader, and
//! photos at its largest size. The screen shows the live picture, the newest frame the reader
//! handed out, read back from its memory each half second and turned and mirrored as it says,
//! and the last photo taken, decoded by
//! the host's image codecs, with its size and its EXIF orientation; Switch moves to the next
//! camera and Photo takes another. What it finds is written to the system's log under the tag `android_cameras`:
//! `adb logcat -s android_cameras`.
//!
//! ```text
//! cargo waft run -d emulator-5554 -p android_cameras
//! ```
//!
//! Elsewhere it says so and quits: the cameras it shows are Android's.
#![feature(arbitrary_self_types)]

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod log;

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
    eprintln!("android_cameras: the cameras this shows are Android's; run it there");
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}
