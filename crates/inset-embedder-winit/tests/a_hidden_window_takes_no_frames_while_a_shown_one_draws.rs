//! A hidden window takes no frames while a shown one keeps taking its own, as a launcher's panel
//! is hidden once a program opens from it: a refresh of a screen brings the frames of the windows
//! on it that are in sight, and a frame asked of a hidden window waits until it shows.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for about a second, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_hidden_window_takes_no_frames_while_a_shown_one_draws";

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, frames::GIVE_UP_AFTER, frames::counted);
    WinitEmbedder.run(client::CountingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, RequestId, ViewId,
        WindowConfig, WindowRef,
    };

    use crate::{TEST, frames, support};

    /// How long the frames are counted once the first window is hidden.
    const COUNTED_FOR: Duration = Duration::from_millis(500);
    /// The fewest frames of the shown window that half second may bring: a 60 Hz display brings
    /// about 30.
    const FEWEST_FRAMES: u32 = 10;
    /// How long AppKit may take to tell the hidden window out of sight, during which a frame
    /// asked of it may still come.
    const HIDING: Duration = Duration::from_millis(250);

    /// Opens two windows, hides the first once both are made, asks for a frame of each from
    /// every frame of the shown one, and counts the frames of each in the half second after the
    /// hide.
    pub(crate) struct CountingClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        hidden_at: Option<Instant>,
        /// The frames of the hidden window once AppKit has had time to tell it out of sight.
        hidden_frames: u32,
    }

    impl CountingClient {
        pub(crate) fn new(platform: PlatformRef) -> CountingClient {
            let requested = ["first, then hidden", "shown"]
                .into_iter()
                .map(|title| {
                    support::request_window(
                        &platform,
                        WindowConfig {
                            title: title.into(),
                            ..WindowConfig::default()
                        },
                    )
                })
                .collect();
            CountingClient {
                platform,
                requested,
                windows: Vec::new(),
                hidden_at: None,
                hidden_frames: 0,
            }
        }

        /// Hides the first window once both windows are made, and starts asking for frames.
        fn hide_the_first_window_once_both_are_made(&mut self) {
            if self.hidden_at.is_some() || self.windows.len() < 2 {
                return;
            }
            self.windows[0].hide();
            self.hidden_at = Some(Instant::now());
            self.request_a_frame_of_each_window();
        }

        fn request_a_frame_of_each_window(&self) {
            for window in &self.windows {
                self.platform.request_frame(window.view().id());
            }
        }

        fn hidden_view(&self) -> ViewId {
            self.windows[0].view().id()
        }

        fn shown_view(&self) -> ViewId {
            self.windows[1].view().id()
        }
    }

    impl EmbedderClient for CountingClient {
        fn frame(&mut self, views: &[ViewId], _frame: Frame) {
            let Some(hidden_at) = self.hidden_at else {
                return;
            };
            if views.contains(&self.hidden_view()) && hidden_at.elapsed() >= HIDING {
                self.hidden_frames += 1;
            }
            if !views.contains(&self.shown_view()) {
                return;
            }
            let frames = frames::count_one();
            if hidden_at.elapsed() < COUNTED_FOR {
                self.request_a_frame_of_each_window();
                return;
            }
            assert!(
                frames >= FEWEST_FRAMES,
                "{frames} frames of the shown window in {COUNTED_FOR:?} with the first hidden"
            );
            assert_eq!(
                self.hidden_frames, 0,
                "frames of the hidden window once it was out of sight"
            );
            support::pass(TEST);
        }

        fn wake(&mut self, _elapsed: Duration) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.hide_the_first_window_once_both_are_made();
        }

        fn view_added(&mut self, _id: ViewId) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.hide_the_first_window_once_both_are_made();
        }
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_reports(&mut self, _reports: Vec<PointerReport>) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

/// The frames of the shown window counted since the first was hidden, and how long the test waits
/// for them.
#[cfg(target_os = "macos")]
mod frames {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    /// Long enough for the windows to open and the frames to be counted, many times over.
    pub(crate) const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

    static FRAMES: AtomicU32 = AtomicU32::new(0);

    /// One more frame of the shown window since the first was hidden; answers how many there have
    /// been.
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
