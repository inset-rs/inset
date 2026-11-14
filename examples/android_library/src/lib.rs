//! An Android library merged into the package: the example asks the AAR's class,
//! `com.example.greeter.Greeter`, for its greeting, through JNI, and shows and logs what it
//! answers. The answer is made of what the package carries only if the library was merged: the
//! AAR's string resource, read through the R class the build generates for it; the meta-data its
//! manifest adds to the application; its XML resource, which its manifest also names as the
//! application's network security configuration; its native library, loaded; a class of the plain
//! JAR it depends on; and that JAR's Java resource. Logged under the tag `android_library`:
//! `adb logcat -s android_library`.
//!
//! ```text
//! cargo waft run -d emulator-5554 -p android_library
//! ```
//!
//! Elsewhere it says so and quits: the library is Android's.

#[cfg(target_os = "android")]
mod greeter;
#[cfg(target_os = "android")]
mod log;

#[waft::main]
fn main(app: &mut waft::App) {
    #[cfg(target_os = "android")]
    android(app);
    #[cfg(not(target_os = "android"))]
    elsewhere(app);
}

/// Shows and logs the greeter's answer, or why there is none.
#[cfg(target_os = "android")]
fn android(app: &mut waft::App) {
    let answer = match greeter::greet() {
        Ok(answer) => answer,
        Err(error) => format!("the greeter did not answer: {error}"),
    };
    log::info(&answer);
    waft_widgets::run_app(app, message(&answer));
}

#[cfg(target_os = "android")]
fn message(text: &str) -> waft_widgets::WidgetRef {
    use waft_widgets::{Center, Directionality, IntoWidget, Padding, Text};
    Directionality::new(
        waft_embedder::TextDirection::Ltr,
        Padding::new(waft_painting::EdgeInsetsGeometry::all(24.0))
            .child(Center::new().child(Text::new(text).into_widget())),
    )
    .into_widget()
}

/// Says that the example is Android's, and quits.
#[cfg(not(target_os = "android"))]
fn elsewhere(app: &mut waft::App) {
    eprintln!("android_library: the library this calls is Android's; run it there");
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}
