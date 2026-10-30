//! A native surface beneath the framework's content. A producer thread draws a moving stripe
//! into IOSurfaces in turn, three or a few more, one frame at each refresh of the window's screen,
//! which it waits for on the platform's vsync waiter, and presents them to a `SurfaceView`; a label the framework paints above the surface shows that
//! its own content stays on top. Once a second the producer prints what it counted, so the
//! compositor can be checked without looking. `--scale` draws the surface and its label through
//! a transform that scales them about their middle: at 0.5 the whole buffer shows, at half its
//! size, inside a frame half as big. `--opacity` fades them together: the room the framework makes
//! for the surface keeps that share of the background beneath it. `--turn` turns them by a number
//! of degrees and `--rounded` cuts their corners round by a radius: Core Animation turns and masks
//! the surface's layers as the framework turns and clips its own drawing.
//!
//! ```text
//! cargo run -p native_surface
//! cargo run -p native_surface -- --exit-after 5
//! cargo run -p native_surface -- --scale 0.5
//! cargo run -p native_surface -- --opacity 0.5
//! cargo run -p native_surface -- --turn 15 --rounded 24
//! ```

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod iosurface;
#[cfg(target_os = "macos")]
mod producer;

#[cfg(target_os = "macos")]
fn main() {
    let presentation = app::Presentation {
        scale: number_after("--scale", 1.0),
        opacity: number_after("--opacity", 1.0),
        turn_degrees: number_after("--turn", 0.0),
        corner_radius: number_after("--rounded", 0.0),
    };
    app::run(exit_after(), presentation);
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

/// The number after `flag` on the command line, such as `--scale 0.5`; `otherwise` without it.
#[cfg(target_os = "macos")]
fn number_after(flag: &str, otherwise: f64) -> f64 {
    argument_after(flag).map_or(otherwise, |number| {
        number
            .parse()
            .unwrap_or_else(|_| panic!("{flag} takes a number, such as 0.5"))
    })
}

/// The command line's argument after `flag`, where it has one.
#[cfg(target_os = "macos")]
fn argument_after(flag: &str) -> Option<String> {
    std::env::args().skip_while(|arg| arg != flag).nth(1)
}
