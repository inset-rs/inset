//! A window keeps its view, its content, within the size limits the app sets, which are the
//! limits the system holds the user's resizing to: a minimum above the view's size grows it, a
//! maximum below it shrinks it, and a limit set to none is gone.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for about a second, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_window_keeps_its_view_within_its_size_limits";

/// Long enough for every step, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use waft_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::LimitingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use waft_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, RequestId, Size, ViewId,
        WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    const TITLE: &str = "a window with size limits";

    /// How long a step is given to settle before the window is asked.
    const SETTLE: Duration = Duration::from_millis(300);

    /// Above the 300 by 200 the window opens at.
    const MIN_SIZE: [f64; 2] = [500.0, 380.0];
    /// Below `MIN_SIZE`, which is cleared before it is set.
    const MAX_SIZE: [f64; 2] = [360.0, 260.0];

    /// What the app has done so far, and when.
    enum Step {
        Opening,
        MinimumSet(Instant),
        MaximumSetMinimumCleared(Instant),
        MaximumCleared(Instant),
    }

    /// Opens a window, sets a minimum above its size, then a maximum below that with the minimum
    /// cleared, then clears the maximum, checking after each what the window's view measures and
    /// what AppKit holds its resizing to.
    pub(crate) struct LimitingClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        step: Step,
    }

    impl LimitingClient {
        pub(crate) fn new(platform: PlatformRef) -> LimitingClient {
            let requested = vec![support::request_window(&platform, support::titled(TITLE))];
            LimitingClient {
                platform,
                requested,
                windows: Vec::new(),
                step: Step::Opening,
            }
        }

        fn window(&self) -> &WindowRef {
            &self.windows[0]
        }

        fn set_limits(&mut self, min: Option<[f64; 2]>, max: Option<[f64; 2]>) {
            let limits = self
                .window()
                .size_limits()
                .unwrap_or_else(|| support::fail(TEST, "a desktop's window has no size limits"));
            let size = |[width, height]: [f64; 2]| Size::new(width, height);
            limits.set_min_size(min.map(size));
            limits.set_max_size(max.map(size));
            self.platform
                .dispatcher()
                .wake_at(Some(Instant::now() + SETTLE));
        }

        fn limit_once_open(&mut self) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            if self.windows.is_empty() {
                return;
            }
            self.set_limits(Some(MIN_SIZE), None);
            self.step = Step::MinimumSet(Instant::now());
        }

        /// The view's size in logical points.
        fn view_size(&self) -> [f64; 2] {
            let metrics = self.window().view().metrics();
            let [width, height] = metrics.physical_size;
            let scale = metrics.device_pixel_ratio;
            [width / scale, height / scale]
        }

        /// Fails the test unless the view measures `size` and AppKit holds the window's content
        /// between `min` and `max`.
        fn expect(&self, step: &str, size: [f64; 2], min: [f64; 2], max: [f64; 2]) {
            let measured = self.view_size();
            let (held_min, held_max) = appkit::content_size_limits(TITLE);
            println!("{step:<40} view {measured:?}  content min {held_min:?}  max {held_max:?}");
            if measured != size || held_min != min || held_max != max {
                support::fail(
                    TEST,
                    &format!("{step}: expected view {size:?}, min {min:?}, max {max:?}"),
                );
            }
        }

        fn go_on(&mut self) {
            match self.step {
                Step::Opening => self.limit_once_open(),
                Step::MinimumSet(at) if at.elapsed() >= SETTLE => {
                    self.expect("minimum set", MIN_SIZE, MIN_SIZE, appkit::NO_MAX_SIZE);
                    self.set_limits(None, Some(MAX_SIZE));
                    self.step = Step::MaximumSetMinimumCleared(Instant::now());
                }
                Step::MaximumSetMinimumCleared(at) if at.elapsed() >= SETTLE => {
                    self.expect(
                        "maximum set, minimum cleared",
                        MAX_SIZE,
                        appkit::NO_MIN_SIZE,
                        MAX_SIZE,
                    );
                    self.set_limits(None, None);
                    self.step = Step::MaximumCleared(Instant::now());
                }
                Step::MaximumCleared(at) if at.elapsed() >= SETTLE => {
                    self.expect(
                        "maximum cleared",
                        MAX_SIZE,
                        appkit::NO_MIN_SIZE,
                        appkit::NO_MAX_SIZE,
                    );
                    support::pass(TEST);
                }
                _ => {}
            }
        }
    }

    impl EmbedderClient for LimitingClient {
        fn wake(&mut self, _elapsed: Duration) {
            self.go_on();
        }

        fn view_added(&mut self, _id: ViewId) {
            self.go_on();
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}
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
