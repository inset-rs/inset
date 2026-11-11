//! A player of a small movie, opened by a picked file's URL at a path the URL must encode, loads it
//! by itself, says its size, length, what it can seek into and one video track, paused as it was
//! never played, plays it, hands its frames to a reader made on its own, each the host's
//! `CoreVideoFrame` over a BGRA pixel buffer backed by an IOSurface, ends, paused at its end,
//! tells a seek under way until it lands and is read where it landed, mutes, and plays on past
//! its end once it loops: all without a window or a sound. On its way it is put on a composition
//! layer, not in any window, which answers its first frame as what it shows though it was never
//! played, lent as the host's `CoreVideoFrame`, and nothing once the player is sent nowhere again;
//! sent nowhere at the end, it feeds the reader no more.
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
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use objc2_core_video::{
        CVPixelBufferGetIOSurface, CVPixelBufferGetPixelFormatType, kCVPixelFormatType_32BGRA,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};
    use waft_embedder::{
        LayerPicture, MediaChangeFuture, MediaTrackKind, PauseReason, PickedFile, PlayerActivity,
        PlayerErrorKind, PlayerStatus, SeekTolerance, Size, TimeRange, VideoOutput,
    };
    use waft_embedder_winit::CoreVideoFrame;

    /// Far longer than a one-second movie takes to load and play through.
    const PATIENCE: Duration = Duration::from_secs(20);
    /// Past the end of the one-second movie, played from its start.
    const LOOPED: Duration = Duration::from_millis(1600);
    /// A moment inside the movie a paused player seeks to.
    const SOUGHT: Duration = Duration::from_millis(300);

    pub(crate) fn reads_the_frames_of_a_file() {
        let players = waft_embedder_winit::test_support::players()
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
        assert_eq!(
            state.activity,
            PlayerActivity::Paused(Some(PauseReason::Requested)),
            "never played"
        );
        assert_eq!(state.rate, 1.0);
        assert_eq!(
            state.seekable,
            vec![TimeRange {
                start: Duration::ZERO,
                end: Duration::from_secs(1)
            }],
            "a file seeks anywhere in it"
        );
        serve_until("its tracks are listed", || {
            !player.state().tracks.is_empty()
        });
        let tracks = player.state().tracks;
        assert_eq!(tracks.len(), 1, "one video track and no sound: {tracks:?}");
        assert_eq!(tracks[0].kind, MediaTrackKind::Video);
        assert!(tracks[0].selected);
        assert!(player.cues().is_empty(), "no text track, no cues");

        let compositor =
            waft_embedder_winit::test_support::compositor().expect("the system's compositor");
        let layer = compositor.make_layer(false);
        player.set_video_output(Some(VideoOutput::Layer(Arc::clone(&layer))));
        let mut shown = None;
        serve_until("the layer shows the first frame", || {
            shown = layer.current_picture();
            shown.is_some()
        });
        let Some(LayerPicture::Frame(shown)) = shown else {
            panic!("the player's frame, lent");
        };
        assert_eq!([shown.width(), shown.height()], [64, 48]);
        let shown: &dyn std::any::Any = &*shown;
        let frame = shown
            .downcast_ref::<CoreVideoFrame>()
            .expect("the host's own frame")
            .pixel_buffer();
        assert_eq!(
            CVPixelBufferGetPixelFormatType(frame),
            kCVPixelFormatType_32BGRA
        );
        assert!(CVPixelBufferGetIOSurface(Some(frame)).is_some());
        assert!(
            layer.current_picture().is_some(),
            "a paused player's frame stays what the layer shows"
        );
        player.set_video_output(None);
        assert!(
            layer.current_picture().is_none(),
            "a player sent nowhere leaves its layer showing nothing"
        );

        let readers = waft_embedder_winit::test_support::video_frame_readers()
            .expect("the system's frame readers");
        let reader = readers.make_reader();
        player.set_video_output(Some(VideoOutput::Frames(Arc::clone(&reader))));
        player.play();
        let mut frame = None;
        serve_until("a frame arrives", || {
            frame = reader.frame_for(Instant::now());
            frame.is_some()
        });
        let frame = frame.expect("a frame");
        assert_eq!([frame.width(), frame.height()], [64, 48]);
        let frame: &dyn std::any::Any = &*frame;
        let buffer = frame
            .downcast_ref::<CoreVideoFrame>()
            .expect("the host's own frame")
            .pixel_buffer();
        assert_eq!(
            CVPixelBufferGetPixelFormatType(buffer),
            kCVPixelFormatType_32BGRA
        );
        assert!(CVPixelBufferGetIOSurface(Some(buffer)).is_some());
        serve_until("it plays", || {
            player.state().activity == PlayerActivity::Playing
        });
        assert!(
            !player.state().loaded.is_empty(),
            "a file being played is loaded"
        );

        serve_until("the movie ends", || {
            player.state().activity == PlayerActivity::Paused(Some(PauseReason::End))
        });
        assert!(player.position() >= Duration::from_millis(900));

        player.seek(SOUGHT, SeekTolerance::EXACT);
        assert_eq!(player.state().seeking, Some(SOUGHT), "under way");
        assert_eq!(player.position(), SOUGHT, "read at its target meanwhile");
        serve_until("the seek lands", || player.state().seeking.is_none());
        assert_eq!(
            player.position(),
            SOUGHT,
            "a paused player is read where its seek landed"
        );

        player.set_muted(true);
        serve_until("it mutes", || player.state().muted);

        player.set_looping(true);
        player.play();
        let looping_since = Instant::now();
        serve_until("a loop", || looping_since.elapsed() > LOOPED);
        assert_eq!(
            player.state().activity,
            PlayerActivity::Playing,
            "a looping player plays on past its end"
        );

        player.set_video_output(None);
        let unfed_since = Instant::now();
        serve_until("a while", || unfed_since.elapsed() > LOOPED / 4);
        assert!(
            reader.frame_for(Instant::now()).is_none(),
            "a player sent nowhere feeds its reader no more"
        );

        let mut gone = player.changed();
        drop(player);
        assert!(
            is_ready(&mut gone),
            "a player let go of tells the app it is gone"
        );
        std::fs::remove_dir_all(movie.parent().expect("the copy's folder")).expect("cleaned up");
        fails_on_what_is_not_media(&*players);
    }

    /// A player of a file that is no movie fails, paused for good, with the kind of its failure.
    fn fails_on_what_is_not_media(players: &dyn waft_embedder::Players) {
        let folder = std::env::temp_dir().join(format!("waft not media {}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the file");
        let text = folder.join("words.mp4");
        std::fs::write(&text, "these are words, not frames").expect("the file written");
        let player = players.open(&PickedFile::Path(text).url());
        serve_until("it fails", || {
            matches!(player.state().status, PlayerStatus::Failed(_))
        });
        let state = player.state();
        let PlayerStatus::Failed(error) = state.status else {
            unreachable!("served until it failed")
        };
        assert_eq!(
            error.kind,
            PlayerErrorKind::Unsupported,
            "what AVFoundation cannot parse it cannot play: {error:?}"
        );
        assert_eq!(state.activity, PlayerActivity::Paused(None));
        drop(player);
        std::fs::remove_dir_all(folder).expect("cleaned up");
    }

    /// The movie, copied into a folder of its own whose name and the copy's need encoding in a
    /// URL: a space, a letter beyond ASCII and a `#`.
    fn copied_where_a_url_must_encode_its_path() -> PathBuf {
        let folder = std::env::temp_dir().join(format!("waft media {}", std::process::id()));
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
