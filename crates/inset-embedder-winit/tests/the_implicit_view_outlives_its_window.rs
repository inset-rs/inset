//! The implicit view stays the same view for the app's whole life, as in Flutter's macOS embedder:
//! once its window closes, the app hears no removal, the view is still among the views and still
//! the implicit view, and its window is gone from AppKit's list.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for a moment, and needs a login session's window server. The other window
//! keeps the app from being asked to quit when the implicit view's closes.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "the_implicit_view_outlives_its_window";

/// The implicit view's window's title, by which AppKit finds it.
#[cfg(target_os = "macos")]
const IMPLICIT_WINDOW: &str = "the implicit view's window";

/// Long enough for the windows to open and one to close, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder::WindowConfig;
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default()
        .implicit_view(Some(WindowConfig {
            title: IMPLICIT_WINDOW.into(),
            size: [300.0, 200.0],
            ..WindowConfig::default()
        }))
        .run(client::ClosingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, ViewId, ViewRef,
        WindowConfig, WindowFuture, WindowRef,
    };

    use crate::support::{self, appkit};
    use crate::{IMPLICIT_WINDOW, TEST};

    /// How long the embedder is given to let the closed window go.
    const SETTLE: Duration = Duration::from_millis(300);

    /// Opens a second window, closes the implicit view's with its close button once that one is
    /// made, and looks at the views a moment later.
    pub(crate) struct ClosingClient {
        platform: PlatformRef,
        implicit_view: ViewRef,
        opening: Vec<WindowFuture>,
        /// Held to the end, so the app keeps a window of its own.
        windows: Vec<WindowRef>,
        removed: Vec<ViewId>,
        closed_at: Option<Instant>,
    }

    impl ClosingClient {
        pub(crate) fn new(platform: PlatformRef) -> ClosingClient {
            let implicit_view = platform
                .implicit_view()
                .expect("the implicit view, at launch");
            let owner = platform
                .windowing_owner()
                .expect("a desktop host makes windows");
            let opening = vec![owner.create(WindowConfig {
                title: "another window".into(),
                size: [300.0, 200.0],
                ..WindowConfig::default()
            })];
            ClosingClient {
                platform,
                implicit_view,
                opening,
                windows: Vec::new(),
                removed: Vec::new(),
                closed_at: None,
            }
        }

        /// Closes the implicit view's window once the other is made, and asks to be woken a
        /// moment later.
        fn close_the_implicit_window_once_the_other_is_made(&mut self) {
            support::take_opened_windows(&mut self.opening, &mut self.windows);
            if self.closed_at.is_some() || self.windows.is_empty() {
                return;
            }
            appkit::press_close_button(IMPLICIT_WINDOW);
            let now = Instant::now();
            self.closed_at = Some(now);
            self.platform.dispatcher().wake_at(Some(now + SETTLE));
        }

        /// What is wrong once the implicit view's window has had time to go.
        fn what_is_wrong(&self) -> Vec<&'static str> {
            let id = self.implicit_view.id();
            let mut wrong = Vec::new();
            if self.removed.contains(&id) {
                wrong.push("the app heard the implicit view removed");
            }
            if !self.platform.views().iter().any(|view| view.id() == id) {
                wrong.push("the implicit view left the views");
            }
            let still_implicit = self
                .platform
                .implicit_view()
                .is_some_and(|view| Rc::ptr_eq(&view, &self.implicit_view));
            if !still_implicit {
                wrong.push("the implicit view changed");
            }
            if appkit::lists_window_titled(IMPLICIT_WINDOW) {
                wrong.push("its window is still among the application's windows");
            }
            wrong
        }
    }

    impl EmbedderClient for ClosingClient {
        fn wake(&mut self, _elapsed: Duration) {
            self.close_the_implicit_window_once_the_other_is_made();
            let Some(closed_at) = self.closed_at else {
                return;
            };
            if closed_at.elapsed() < SETTLE {
                return;
            }
            let wrong = self.what_is_wrong();
            if !wrong.is_empty() {
                support::fail(TEST, &format!("{wrong:?}"));
            }
            support::pass(TEST);
        }

        fn view_removed(&mut self, id: ViewId) {
            self.removed.push(id);
        }

        fn frame(&mut self, _frame: Frame) {}
        fn view_added(&mut self, _id: ViewId) {}
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
