//! A window says whether it is minimized and whether it is occluded, and the app hears each time
//! either changes or the window moves, through each way a window is minimized, covered or hidden
//! and comes back; a resize alone it hears as a metrics change only.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for about fifteen seconds, hides the application for a while, and needs a
//! login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_window_tells_the_app_it_was_minimized_occluded_or_moved";

/// Long enough for every step, twice over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(40);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::WindowChangesClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, Rect, RequestId, ViewId,
        WindowConfig, WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    /// How long a step is given to settle before the window is asked: the Dock's animations take
    /// about half of it.
    const SETTLE: Duration = Duration::from_millis(1000);

    const WATCHED_WINDOW: &str = "a window watched";

    /// Where the watched window is moved.
    const MOVED_TO: [f64; 2] = [340.0, 320.0];

    /// What a step does to the watched window, and how the app should find it.
    struct Step {
        name: &'static str,
        bring: fn(&WindowChangesClient),
        minimized: bool,
        occluded: bool,
        /// Whether the app hears that the window changed in the step; `None` for the first, which
        /// the window may have been heard shown before.
        heard: Option<bool>,
    }

    const STEPS: [Step; 12] = [
        Step {
            name: "shown",
            bring: |_| {},
            minimized: false,
            occluded: false,
            heard: None,
        },
        Step {
            name: "covered by another window",
            bring: |client| client.windows[1].show(),
            minimized: false,
            occluded: true,
            heard: Some(true),
        },
        Step {
            name: "uncovered",
            bring: |client| client.windows[1].hide(),
            minimized: false,
            occluded: false,
            heard: Some(true),
        },
        Step {
            name: "minimized",
            bring: |_| appkit::minimize(WATCHED_WINDOW),
            minimized: true,
            occluded: true,
            heard: Some(true),
        },
        Step {
            name: "restored from the Dock",
            bring: |_| appkit::restore(WATCHED_WINDOW),
            minimized: false,
            occluded: false,
            heard: Some(true),
        },
        Step {
            name: "hidden by the app",
            bring: |client| client.windows[0].hide(),
            minimized: false,
            occluded: true,
            heard: Some(true),
        },
        Step {
            name: "shown by the app",
            bring: |client| client.windows[0].show(),
            minimized: false,
            occluded: false,
            heard: Some(true),
        },
        Step {
            name: "application hidden",
            bring: |_| appkit::hide_application(),
            minimized: false,
            occluded: true,
            heard: Some(true),
        },
        Step {
            name: "application shown again",
            bring: |_| {
                appkit::unhide_application();
                // Unhiding does not activate the application, so its window can come back
                // behind other applications' windows, where a covered window is occluded.
                appkit::order_in_front_of_everything(WATCHED_WINDOW);
            },
            minimized: false,
            occluded: false,
            heard: Some(true),
        },
        Step {
            name: "widened in place",
            bring: |client| client.widen_watched(),
            minimized: false,
            occluded: false,
            heard: Some(false),
        },
        Step {
            name: "moved",
            bring: |client| client.move_watched(),
            minimized: false,
            occluded: false,
            heard: Some(true),
        },
        Step {
            name: "left alone",
            bring: |_| {},
            minimized: false,
            occluded: false,
            heard: Some(false),
        },
    ];

    /// Opens a window to watch and a larger one to cover it, then brings the first through each
    /// step, asking it how it is once the step has settled and counting the changes the app hears
    /// of it. Wakes, which come whether a window is visible or not, time the steps.
    pub(crate) struct WindowChangesClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        step: Option<usize>,
        step_began: Instant,
        changes_heard: u32,
        failures: Vec<String>,
    }

    impl WindowChangesClient {
        pub(crate) fn new(platform: PlatformRef) -> WindowChangesClient {
            let requested = vec![
                support::request_window(
                    &platform,
                    WindowConfig {
                        title: WATCHED_WINDOW.into(),
                        size: [400.0, 300.0],
                        position: Some([300.0, 300.0]),
                        ..WindowConfig::default()
                    },
                ),
                support::request_window(
                    &platform,
                    WindowConfig {
                        title: "a window covering it".into(),
                        size: [800.0, 700.0],
                        position: Some([100.0, 100.0]),
                        visible: false,
                        ..WindowConfig::default()
                    },
                ),
            ];
            WindowChangesClient {
                platform,
                requested,
                windows: Vec::new(),
                step: None,
                step_began: Instant::now(),
                changes_heard: 0,
                failures: Vec::new(),
            }
        }

        /// Widens the watched window where it is, which is a resize alone.
        fn widen_watched(&self) {
            let frame = self.windows[0].frame();
            let wider =
                Rect::from_ltwh(frame.left, frame.top, frame.width() + 80.0, frame.height());
            self.windows[0].set_frame(wider, None);
        }

        /// Moves the watched window to `MOVED_TO`, at the size it has.
        fn move_watched(&self) {
            let frame = self.windows[0].frame();
            let [left, top] = MOVED_TO;
            let moved = Rect::from_ltwh(left, top, frame.width(), frame.height());
            self.windows[0].set_frame(moved, None);
        }

        /// The first step begins once both windows are made.
        fn begin_once_both_windows_are_made(&mut self) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            if self.step.is_some() || self.windows.len() < 2 {
                return;
            }
            self.step = Some(0);
            self.step_began = Instant::now();
            self.changes_heard = 0;
            support::wake_after_a_tick(&self.platform);
        }

        /// Judges the current step once it has settled, and brings the next. Answers whether every
        /// step is done.
        fn go_on(&mut self) -> bool {
            let Some(step) = self.step else {
                return false;
            };
            if self.step_began.elapsed() < SETTLE {
                return false;
            }
            self.judge(&STEPS[step]);
            let next = step + 1;
            let Some(next_step) = STEPS.get(next) else {
                return true;
            };
            self.step = Some(next);
            self.step_began = Instant::now();
            self.changes_heard = 0;
            (next_step.bring)(self);
            false
        }

        fn judge(&mut self, step: &Step) {
            let window = &self.windows[0];
            let minimized = window.is_minimized();
            let occluded = window.is_occluded();
            let heard = self.changes_heard > 0;
            let as_expected = minimized == step.minimized
                && occluded == step.occluded
                && step.heard.is_none_or(|expected| heard == expected);
            println!(
                "{:<26} minimized {:<5} occluded {:<5} changes heard {}  {}",
                step.name,
                minimized,
                occluded,
                self.changes_heard,
                if as_expected { "ok" } else { "FAILED" }
            );
            if !as_expected {
                self.failures.push(step.name.to_owned());
            }
            if step.name == "moved" {
                let frame = window.frame();
                if [frame.left, frame.top] != MOVED_TO {
                    println!("moved to {frame:?}, not {MOVED_TO:?}  FAILED");
                    self.failures.push("moved to where it was asked".to_owned());
                }
            }
        }
    }

    impl EmbedderClient for WindowChangesClient {
        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}

        fn wake(&mut self, _elapsed: Duration) {
            self.begin_once_both_windows_are_made();
            if self.step.is_none() {
                return;
            }
            if !self.go_on() {
                support::wake_after_a_tick(&self.platform);
                return;
            }
            assert!(
                self.failures.is_empty(),
                "the window not as expected in: {:?}",
                self.failures
            );
            support::pass(TEST);
        }

        fn view_added(&mut self, _id: ViewId) {
            self.begin_once_both_windows_are_made();
        }

        fn window_changed(&mut self, view: ViewId) {
            if self
                .windows
                .first()
                .is_some_and(|window| window.view().id() == view)
            {
                self.changes_heard += 1;
            }
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
