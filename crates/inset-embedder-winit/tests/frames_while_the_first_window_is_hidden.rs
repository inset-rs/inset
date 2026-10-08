//! The framework's frames keep coming while the first window, whose display's vsyncs they are
//! drawn at, is hidden and another is shown, as a launcher's panel is hidden once a program opens
//! from it. Drawn at the vsyncs of the hidden window's own display link, as they once were, they
//! stopped: that link gives none while its window is hidden.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for about a second, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "the_frames_keep_coming_while_the_first_window_is_hidden";

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, frames::GIVE_UP_AFTER, frames::counted);
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::CountingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, ViewId, WindowConfig,
        WindowFuture, WindowRef,
    };

    use crate::{TEST, frames, support};

    /// How long the frames are counted once the first window is hidden.
    const COUNTED_FOR: Duration = Duration::from_millis(500);
    /// The fewest frames that half second may bring: a 60 Hz display brings about 30.
    const FEWEST_FRAMES: u32 = 10;

    /// Opens two windows, the first of which the frames take their vsyncs from, hides that one
    /// once both are made, asks for a frame from each frame, and counts the frames of the half
    /// second after the hide.
    pub(crate) struct CountingClient {
        platform: PlatformRef,
        opening: Vec<WindowFuture>,
        windows: Vec<WindowRef>,
        hidden_at: Option<Instant>,
    }

    impl CountingClient {
        pub(crate) fn new(platform: PlatformRef) -> CountingClient {
            let owner = platform
                .windowing_owner()
                .expect("a desktop host makes windows");
            let opening = ["first, then hidden", "shown"]
                .into_iter()
                .map(|title| {
                    owner.create(WindowConfig {
                        title: title.into(),
                        ..WindowConfig::default()
                    })
                })
                .collect();
            CountingClient {
                platform,
                opening,
                windows: Vec::new(),
                hidden_at: None,
            }
        }

        /// Hides the first window once both windows are made, and starts asking for frames.
        fn hide_the_first_window_once_both_are_made(&mut self) {
            if self.hidden_at.is_some() || self.windows.len() < 2 {
                return;
            }
            self.windows[0].hide();
            self.hidden_at = Some(Instant::now());
            self.platform.request_frame();
        }
    }

    impl EmbedderClient for CountingClient {
        fn frame(&mut self, _frame: Frame) {
            let Some(hidden_at) = self.hidden_at else {
                return;
            };
            let frames = frames::count_one();
            if hidden_at.elapsed() < COUNTED_FOR {
                self.platform.request_frame();
                return;
            }
            assert!(
                frames >= FEWEST_FRAMES,
                "{frames} frames in {COUNTED_FOR:?} with the first window hidden"
            );
            support::pass(TEST);
        }

        fn wake(&mut self, _elapsed: Duration) {
            support::take_opened_windows(&mut self.opening, &mut self.windows);
            self.hide_the_first_window_once_both_are_made();
        }

        fn view_added(&mut self, _id: ViewId) {}
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

/// The frames counted since the first window was hidden, and how long the test waits for them.
#[cfg(target_os = "macos")]
mod frames {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    /// Long enough for the windows to open and the frames to be counted, many times over.
    pub(crate) const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

    static FRAMES: AtomicU32 = AtomicU32::new(0);

    /// One more frame since the first window was hidden; answers how many there have been.
    pub(crate) fn count_one() -> u32 {
        FRAMES.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The frames counted, as a failure reports them.
    pub(crate) fn counted() -> String {
        format!(
            "{} frames in {GIVE_UP_AFTER:?}",
            FRAMES.load(Ordering::SeqCst)
        )
    }
}
