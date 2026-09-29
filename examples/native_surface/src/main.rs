//! A native surface beneath the framework's content. A producer thread draws a moving stripe
//! into three IOSurfaces in turn, one frame at each vsync of the window's display, and
//! presents them to a `SurfaceView`; a label the framework paints above the surface shows that
//! its own content stays on top. Once a second the producer prints what it counted, so the
//! compositor can be checked without looking.
//!
//! ```text
//! cargo run -p native_surface
//! cargo run -p native_surface -- --exit-after 5
//! ```

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod iosurface;
#[cfg(target_os = "macos")]
mod producer;

#[cfg(target_os = "macos")]
fn main() {
    app::run(exit_after());
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native_surface: the compositor this shows runs on macOS only");
}

/// `--exit-after SECONDS`, for a smoke run nobody closes.
#[cfg(target_os = "macos")]
fn exit_after() -> Option<std::time::Duration> {
    let mut args = std::env::args()
        .skip_while(|arg| arg != "--exit-after")
        .skip(1);
    let seconds = args
        .next()?
        .parse()
        .expect("--exit-after takes a number of seconds");
    Some(std::time::Duration::from_secs(seconds))
}
