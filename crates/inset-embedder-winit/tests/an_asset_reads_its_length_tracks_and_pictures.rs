//! A media asset of a small movie, opened by a picked file's URL at a path the URL must encode,
//! reads its length, its one video track, selected, and its tags without playing it, and answers
//! its picture at a time exactly, as the host's `CoreVideoFrame` over a BGRA pixel buffer backed
//! by an IOSurface, the picture drawn into it, and near a time scaled down to fit a size; an asset
//! of a file that is no movie fails with the kind of its failure. All without a window or a sound.
//!
//! Its own binary, without the test harness, as the player's test is: it serves the main run loop
//! while it waits, as the app's main thread does. The movie, `tests/media/ten-frames.mp4`, is one
//! second of H.264 at 64 by 48 pixels, ten frames of a colour that changes each frame.

// Serving the main run loop and reading a pixel buffer need `unsafe`, which the crate denies; this
// test alone allows it.
#![allow(unsafe_code)]

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() {
    asset::reads_its_length_tracks_and_pictures();
    println!("an_asset_reads_its_length_tracks_and_pictures: ok");
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn main() {}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod asset {
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use inset_embedder::{
        MediaAssets, MediaTrackKind, PickedFile, PlayerErrorKind, SeekTolerance, Size, VideoFrame,
    };
    use inset_embedder_winit::CoreVideoFrame;
    use objc2_core_video::{
        CVPixelBufferGetBaseAddress, CVPixelBufferGetIOSurface, CVPixelBufferGetPixelFormatType,
        CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
        kCVPixelFormatType_32BGRA,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

    /// Far longer than reading a one-second movie takes.
    const PATIENCE: Duration = Duration::from_secs(20);
    /// A moment inside the movie whose picture is asked for exactly.
    const EXACTLY_AT: Duration = Duration::from_millis(300);

    pub(crate) fn reads_its_length_tracks_and_pictures() {
        let assets = inset_embedder_winit::test_support::media_assets()
            .expect("the system's reading of media");
        let movie = copied_where_a_url_must_encode_its_path();
        let asset = assets.open(&PickedFile::Path(movie.clone()).url());

        let duration = served("its length", asset.duration()).expect("a length");
        assert_eq!(duration, Some(Duration::from_secs(1)));
        let tracks = served("its tracks", asset.tracks()).expect("its tracks");
        assert_eq!(tracks.len(), 1, "one video track and no sound: {tracks:?}");
        assert_eq!(tracks[0].kind, MediaTrackKind::Video);
        assert!(tracks[0].selected, "a player would show it");
        let metadata = served("its tags", asset.metadata()).expect("its tags");
        assert_eq!(metadata.artwork, None, "{metadata:?}");

        let frame = served(
            "its picture at a time",
            asset.frame_at(EXACTLY_AT, SeekTolerance::EXACT, None),
        )
        .expect("a picture");
        assert_eq!([frame.width(), frame.height()], [64, 48]);
        assert_drawn_in_a_frame_readers_buffer(&frame);

        let small = served(
            "a small picture near a time",
            asset.frame_at(
                Duration::from_millis(500),
                SeekTolerance::ANY,
                Some(Size::new(32.0, 32.0)),
            ),
        )
        .expect("a small picture");
        assert_eq!(
            [small.width(), small.height()],
            [32, 24],
            "scaled down to fit, its aspect kept"
        );
        drop(asset);
        std::fs::remove_dir_all(movie.parent().expect("the copy's folder")).expect("cleaned up");
        fails_on_what_is_not_media(&*assets);
    }

    /// `frame` is the host's own, over a BGRA pixel buffer backed by an IOSurface, its picture
    /// drawn into it: every pixel opaque.
    fn assert_drawn_in_a_frame_readers_buffer(frame: &Arc<dyn VideoFrame>) {
        let frame: &dyn std::any::Any = &**frame;
        let buffer = frame
            .downcast_ref::<CoreVideoFrame>()
            .expect("the host's own frame")
            .pixel_buffer();
        assert_eq!(
            CVPixelBufferGetPixelFormatType(buffer),
            kCVPixelFormatType_32BGRA
        );
        assert!(CVPixelBufferGetIOSurface(Some(buffer)).is_some());
        // SAFETY: locked for reading while its first pixel's four bytes are read.
        let first_pixel = unsafe {
            CVPixelBufferLockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly);
            let base = CVPixelBufferGetBaseAddress(buffer).cast::<[u8; 4]>();
            let pixel = *base;
            CVPixelBufferUnlockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly);
            pixel
        };
        assert_eq!(first_pixel[3], 255, "the picture drawn: {first_pixel:?}");
    }

    /// An asset of a file that is no movie fails, with the kind of its failure.
    fn fails_on_what_is_not_media(assets: &dyn MediaAssets) {
        let folder = std::env::temp_dir().join(format!("inset not media {}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the file");
        let text = folder.join("words.mp4");
        std::fs::write(&text, "these are words, not frames").expect("the file written");
        let asset = assets.open(&PickedFile::Path(text).url());
        let error = served("its length", asset.duration()).expect_err("no length");
        assert_eq!(
            error.kind,
            PlayerErrorKind::Unsupported,
            "what AVFoundation cannot parse it cannot read: {error:?}"
        );
        let error = served(
            "its picture",
            asset.frame_at(Duration::ZERO, SeekTolerance::ANY, None),
        )
        .err()
        .expect("no picture");
        assert_ne!(error.message, "", "the system's words: {error:?}");
        std::fs::remove_dir_all(folder).expect("cleaned up");
    }

    /// The movie, copied into a folder of its own whose name and the copy's need encoding in a
    /// URL: a space, a letter beyond ASCII and a `#`.
    fn copied_where_a_url_must_encode_its_path() -> PathBuf {
        let folder = std::env::temp_dir().join(format!("inset asset {}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the copy");
        let copy = folder.join("été #1.mp4");
        let movie = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/media/ten-frames.mp4");
        std::fs::copy(movie, &copy).expect("the movie copied");
        copy
    }

    /// What `future` answers, serving the main run loop until it does, failing once `PATIENCE`
    /// has passed.
    fn served<T>(what: &str, future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let started = Instant::now();
        loop {
            if let Poll::Ready(answer) = future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                return answer;
            }
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
}
