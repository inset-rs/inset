//! The framework hears the application's lifecycle as Flutter's macOS embedder reports it: a
//! visible state while its window shows, hidden once the application is hidden, and a visible
//! state again once it is shown.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a second or two, hides the application for a moment, and needs a login
//! session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "the_lifecycle_follows_the_application";

/// Long enough for every step, several times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(15);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::LifecycleClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        AppLifecycleState, EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, RequestId,
        ViewId, WindowConfig, WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    /// How often the client looks at what it has heard.
    const LOOK_EVERY: Duration = Duration::from_millis(50);
    /// How long a step is given to settle before what the framework heard is judged: AppKit
    /// finishes hiding an application a little after it stops being visible, and does not
    /// unhide one it is still hiding.
    const SETTLE: Duration = Duration::from_millis(1000);

    /// A state the application is brought into, and the states the framework may hear there.
    struct Step {
        name: &'static str,
        bring: fn(),
        hears: &'static [AppLifecycleState],
    }

    /// The title of the test's one window, by which AppKit finds it.
    const WINDOW_TITLE: &str = "a window of the application";

    const VISIBLE: &[AppLifecycleState] =
        &[AppLifecycleState::Resumed, AppLifecycleState::Inactive];

    const STEPS: [Step; 3] = [
        Step {
            name: "its window shown",
            bring: || {},
            hears: VISIBLE,
        },
        Step {
            name: "the application hidden",
            bring: appkit::hide_application,
            hears: &[AppLifecycleState::Hidden],
        },
        Step {
            name: "the application shown again",
            bring: || {
                appkit::unhide_application();
                // Unhiding does not activate the application, so its window can come back
                // behind other applications' windows, where a covered window is hidden.
                appkit::order_in_front_of_everything(WINDOW_TITLE);
            },
            hears: VISIBLE,
        },
    ];

    /// Opens a window, then brings the application through each step once the last one has
    /// settled and the framework has heard the state it puts the application in.
    pub(crate) struct LifecycleClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        heard: Option<AppLifecycleState>,
        step: usize,
        step_began: Instant,
    }

    impl LifecycleClient {
        pub(crate) fn new(platform: PlatformRef) -> LifecycleClient {
            let requested = vec![support::request_window(
                &platform,
                WindowConfig {
                    title: WINDOW_TITLE.into(),
                    size: [300.0, 200.0],
                    ..WindowConfig::default()
                },
            )];
            let client = LifecycleClient {
                platform,
                requested,
                windows: Vec::new(),
                heard: None,
                step: 0,
                step_began: Instant::now(),
            };
            client.look_again_soon();
            client
        }

        fn look_again_soon(&self) {
            self.platform
                .dispatcher()
                .wake_at(Some(Instant::now() + LOOK_EVERY));
        }

        /// Brings the next step once the current one has settled and the framework heard what
        /// it puts the application in.
        fn go_on(&mut self) {
            let step = &STEPS[self.step];
            if self.step_began.elapsed() < SETTLE
                || !self.heard.is_some_and(|heard| step.hears.contains(&heard))
            {
                return;
            }
            println!("{:<28} heard {:?}", step.name, self.heard.expect("heard"));
            self.step += 1;
            self.step_began = Instant::now();
            match STEPS.get(self.step) {
                Some(next) => (next.bring)(),
                None => support::pass(TEST),
            }
        }
    }

    impl EmbedderClient for LifecycleClient {
        fn app_lifecycle_state_changed(&mut self, state: AppLifecycleState) {
            self.heard = Some(state);
        }

        fn wake(&mut self, _elapsed: Duration) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            if !self.windows.is_empty() {
                self.go_on();
            }
            self.look_again_soon();
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}
        fn view_added(&mut self, _id: ViewId) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            if !self.windows.is_empty() {
                self.go_on();
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
