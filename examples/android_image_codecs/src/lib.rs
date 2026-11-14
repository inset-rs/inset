//! Android's image codecs as a Waft app has them (`Platform::image_codecs`): the formats they
//! decode and encode, and a check of each thing they do, shown with the pictures decoded and
//! written to the system's log under the tag `android_image_codecs`, so they can be checked
//! without looking: `adb logcat -s android_image_codecs`.
//!
//! ```text
//! cargo waft run -d emulator-5554 -p android_image_codecs
//! ```
//!
//! The pictures, under `media/`: `photo.jpg`, `waft-embedder-winit`'s test photo, which Apple's
//! host's test reads too, 48 by 32 as stored, its left half red and its right half blue, with
//! EXIF saying to turn it a quarter clockwise to show it, so upright it is 32 by 48, red above and
//! blue below, and saying when, with what camera and where it was taken; `turned.heic`, the same
//! photo written as HEIF by macOS's `sips`; made with Pillow, `turned.png`, `turned.webp` and
//! `turned.avif`, the same picture as stored with EXIF orientation 6 and nothing else, and
//! `mirrored.jpg`, the same picture saying to mirror it (EXIF 2), so upright it is blue on the
//! left and red on the right; `alpha.png`, 32 by 32, its left half opaque
//! green and its right half red at half alpha; `two-frames.gif`, 8 by 8, red for 200 ms then blue
//! for 500 ms; and `two-frames.webp`, 8 by 8, green for 300 ms then yellow for 700 ms.
//!
//! The entry is written out rather than by `#[waft::main]` on Android, to keep the folder Android
//! gives the app's files, which the photo is opened from as a file.
//!
//! Elsewhere it says so and quits: the codecs it checks are Android's.
#![feature(arbitrary_self_types)]

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod checks;
#[cfg(target_os = "android")]
mod log;

#[cfg_attr(not(target_os = "android"), waft::main)]
fn main(app: &mut waft::App) {
    #[cfg(target_os = "android")]
    android::run(app);
    #[cfg(not(target_os = "android"))]
    elsewhere(app);
}

/// Keeps the folder of the app's files, and runs the app.
#[cfg(target_os = "android")]
// Waft's Android host finds the entry by its name, as `#[waft::main]`'s.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
fn android_main(android_app: waft::AndroidApp) {
    if let Some(files) = android_app.internal_data_path() {
        let _ = checks::FILES.set(files);
    }
    waft::DefaultEmbedder::default()
        .android_app(android_app)
        .run(|platform| waft::Shell::new(platform, main));
}

/// Says that the example is Android's, and quits.
#[cfg(not(target_os = "android"))]
fn elsewhere(app: &mut waft::App) {
    eprintln!("android_image_codecs: the codecs this checks are Android's; run it there");
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}
