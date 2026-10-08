//! The app starts before its implicit view's window opens, as Flutter's engine runs Dart's `main`
//! with the implicit view at empty metrics: `start` gets view 0 at no size, while AppKit lists no
//! window of it, and the app then hears the view's new metrics, not a view added, once the
//! window is open.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "the_implicit_view_is_there_before_its_window";

/// The implicit view's window's title, by which AppKit finds it.
#[cfg(target_os = "macos")]
const IMPLICIT_WINDOW: &str = "the implicit view's window, opened after the app started";

/// Long enough for the window to open, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder::WindowConfig;
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        "the implicit view's window did not open".to_owned()
    });
    WinitEmbedder::default()
        .implicit_view(Some(WindowConfig {
            title: IMPLICIT_WINDOW.into(),
            size: [300.0, 200.0],
            ..WindowConfig::default()
        }))
        .run(client::StartingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, ViewId};

    use crate::support::{self, appkit};
    use crate::{IMPLICIT_WINDOW, TEST};

    /// Checks the implicit view as the app starts, and again once it hears the view's metrics.
    pub(crate) struct StartingClient {
        platform: PlatformRef,
        added: Vec<ViewId>,
    }

    impl StartingClient {
        pub(crate) fn new(platform: PlatformRef) -> StartingClient {
            let view = platform
                .implicit_view()
                .expect("the implicit view, as the app starts");
            let mut wrong = Vec::new();
            if view.metrics().physical_size != [0.0, 0.0] {
                wrong.push("the implicit view had a size before its window");
            }
            if !platform.views().iter().any(|other| other.id() == view.id()) {
                wrong.push("the implicit view was not among the views");
            }
            if appkit::lists_window_titled(IMPLICIT_WINDOW) {
                wrong.push("its window was open before the app started");
            }
            if !wrong.is_empty() {
                support::fail(TEST, &format!("as the app started: {wrong:?}"));
            }
            StartingClient {
                platform,
                added: Vec::new(),
            }
        }
    }

    impl EmbedderClient for StartingClient {
        fn view_added(&mut self, id: ViewId) {
            self.added.push(id);
        }

        fn view_metrics_changed(&mut self, id: ViewId) {
            let view = self.platform.implicit_view().expect("the implicit view");
            if id != view.id() || view.metrics().physical_size == [0.0, 0.0] {
                return;
            }
            let mut wrong = Vec::new();
            if self.added.contains(&id) {
                wrong.push("the app heard the implicit view added");
            }
            if !appkit::lists_window_titled(IMPLICIT_WINDOW) {
                wrong.push("its window is not among the application's windows");
            }
            if !wrong.is_empty() {
                support::fail(TEST, &format!("once sized: {wrong:?}"));
            }
            support::pass(TEST);
        }

        fn wake(&mut self, _elapsed: Duration) {}
        fn frame(&mut self, _frame: Frame) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
