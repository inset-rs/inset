//! A player of a small movie, opened by a picked file's URL at a path the URL must encode, loads it
//! by itself, says its size and length, plays it, hands its frames to a reader as BGRA pixel
//! buffers backed by IOSurfaces, ends, and plays on past its end once it loops: all without a
//! window, a surface or a sound.
//!
//! Its own binary, without the test harness: AVFoundation changes a player's state on the main
//! queue, which only the main thread serves, so this one serves it while it waits. On iOS it runs
//! in a booted simulator, `xcrun simctl spawn booted <the test's binary>`. The movie,
//! `tests/media/ten-frames.mp4`, is one second of H.264 at 64 by 48 pixels, ten frames of a colour
//! that changes each frame, which `AVAssetWriter` wrote; it is 1342 bytes.

// Serving the main run loop needs `unsafe`, which the crate denies; this test alone allows it.
#![allow(unsafe_code)]

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() {
    player::reads_the_frames_of_a_file();
    println!("a_player_reads_the_frames_of_a_file: ok");
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn main() {}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod player {
    use std::path::PathBuf;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use inset_embedder::{MediaChangeFuture, PickedFile, PlayerStatus, Size};
    use objc2_core_video::{
        CVPixelBufferGetHeight, CVPixelBufferGetIOSurface, CVPixelBufferGetPixelFormatType,
        CVPixelBufferGetWidth, kCVPixelFormatType_32BGRA,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

    /// Far longer than a one-second movie takes to load and play through.
    const PATIENCE: Duration = Duration::from_secs(20);
    /// Past the end of the one-second movie, played from its start.
    const LOOPED: Duration = Duration::from_millis(1600);

    pub(crate) fn reads_the_frames_of_a_file() {
        let players = inset_embedder_winit::test_support::players()
            .expect("the system's players, on the main thread");
        let movie = copied_where_a_url_must_encode_its_path();
        let player = players.open(&PickedFile::Path(movie.clone()).url());
        player.set_volume(0.0);

        let mut changed = player.changed();
        serve_until("the movie loads", || {
            player.state().status != PlayerStatus::Loading
        });
        assert!(is_ready(&mut changed), "the player said its state changed");
        let state = player.state();
        assert_eq!(state.status, PlayerStatus::Ready);
        assert_eq!(state.size, Some(Size::new(64.0, 48.0)));
        assert_eq!(state.duration, Some(Duration::from_secs(1)));
        assert!(!state.playing && !state.ended);

        let frames = player.read_frames();
        player.play();
        let mut frame = None;
        serve_until("a frame arrives", || {
            frame = frames.newest();
            frame.is_some()
        });
        let frame = frame.expect("a frame");
        assert_eq!(CVPixelBufferGetWidth(&frame), 64);
        assert_eq!(CVPixelBufferGetHeight(&frame), 48);
        assert_eq!(
            CVPixelBufferGetPixelFormatType(&frame),
            kCVPixelFormatType_32BGRA
        );
        assert!(CVPixelBufferGetIOSurface(Some(&frame)).is_some());
        assert!(player.state().playing);

        serve_until("the movie ends", || player.state().ended);
        assert!(!player.state().playing);
        assert!(player.state().position >= Duration::from_millis(900));

        player.set_looping(true);
        player.play();
        let looping_since = Instant::now();
        serve_until("a loop", || looping_since.elapsed() > LOOPED);
        let state = player.state();
        assert!(
            state.playing && !state.ended,
            "a looping player plays on past its end"
        );

        let mut gone = player.changed();
        drop(player);
        assert!(
            is_ready(&mut gone),
            "a player let go of tells the app it is gone"
        );
        std::fs::remove_dir_all(movie.parent().expect("the copy's folder")).expect("cleaned up");
    }

    /// The movie, copied into a folder of its own whose name and the copy's need encoding in a
    /// URL: a space, a letter beyond ASCII and a `#`.
    fn copied_where_a_url_must_encode_its_path() -> PathBuf {
        let folder = std::env::temp_dir().join(format!("inset media {}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the copy");
        let copy = folder.join("été #1.mp4");
        let movie = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/media/ten-frames.mp4");
        std::fs::copy(movie, &copy).expect("the movie copied");
        copy
    }

    /// Serves the main run loop until `done`, failing once `PATIENCE` has passed.
    fn serve_until(what: &str, mut done: impl FnMut() -> bool) {
        let started = Instant::now();
        while !done() {
            assert!(
                started.elapsed() < PATIENCE,
                "{what} took over {PATIENCE:?}"
            );
            // SAFETY: serving the main thread's run loop a moment, from the main thread.
            unsafe {
                NSRunLoop::mainRunLoop().runMode_beforeDate(
                    NSDefaultRunLoopMode,
                    &NSDate::dateWithTimeIntervalSinceNow(0.01),
                );
            }
        }
    }

    fn is_ready(future: &mut MediaChangeFuture) -> bool {
        matches!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(())
        )
    }
}
