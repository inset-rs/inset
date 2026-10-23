//! A native surface beneath the framework's content. A producer thread draws a moving stripe
//! into IOSurfaces in turn, three or a few more, one frame at each refresh of the window's screen,
//! which it waits for on the platform's vsync waiter, and presents them to a `SurfaceView`; a label the framework paints above the surface shows that
//! its own content stays on top. Once a second the producer prints what it counted, so the
//! compositor can be checked without looking. `--scale` draws the surface and its label through
//! a transform that scales them about their middle: at 0.5 the whole buffer shows, at half its
//! size, inside a frame half as big.
//!
//! ```text
//! cargo run -p native_surface
//! cargo run -p native_surface -- --exit-after 5
//! cargo run -p native_surface -- --scale 0.5
//! ```

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod iosurface;
#[cfg(target_os = "macos")]
mod producer;

#[cfg(target_os = "macos")]
fn main() {
    app::run(exit_after(), scale());
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native_surface: the compositor this shows runs on macOS only");
}

/// `--exit-after SECONDS`, for a smoke run nobody closes.
#[cfg(target_os = "macos")]
fn exit_after() -> Option<std::time::Duration> {
    let seconds = argument_after("--exit-after")?
        .parse()
        .expect("--exit-after takes a number of seconds");
    Some(std::time::Duration::from_secs(seconds))
}

/// `--scale FACTOR`, what the surface and its label are scaled by; 1 without it.
#[cfg(target_os = "macos")]
fn scale() -> f64 {
    argument_after("--scale").map_or(1.0, |factor| {
        factor.parse().expect("--scale takes a number, such as 0.5")
    })
}

/// The command line's argument after `flag`, where it has one.
#[cfg(target_os = "macos")]
fn argument_after(flag: &str) -> Option<String> {
    std::env::args().skip_while(|arg| arg != flag).nth(1)
}
