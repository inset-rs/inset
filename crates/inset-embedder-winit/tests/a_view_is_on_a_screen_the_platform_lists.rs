//! The platform lists the screens from the start, before the app has a window, and the user's
//! languages; the view of a window the app opens is on one of the screens listed, at that screen's
//! scale.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server.

// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_view_is_on_a_screen_the_platform_lists";

/// Long enough for the window to open, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the window did not open within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::ScreenClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, Offset, PlatformRef, PointerDataPacket, Rect, Screen,
        ViewId,
    };

    use crate::TEST;
    use crate::support;

    /// Checks the screens and the languages as it starts, asks for a window, and checks the
    /// screen its view is on once it arrives.
    pub(crate) struct ScreenClient {
        platform: PlatformRef,
    }

    impl ScreenClient {
        pub(crate) fn new(platform: PlatformRef) -> ScreenClient {
            let wrong = what_is_wrong_at_start(&platform);
            if !wrong.is_empty() {
                support::fail(TEST, &format!("{wrong:?}"));
            }
            support::request_window(&platform, support::titled("on a screen"));
            ScreenClient { platform }
        }
    }

    fn screens(platform: &PlatformRef) -> Vec<Screen> {
        platform
            .screens()
            .map(|screens| screens.screens())
            .unwrap_or_default()
    }

    /// What is wrong with the screens and the languages before the app has a window.
    fn what_is_wrong_at_start(platform: &PlatformRef) -> Vec<&'static str> {
        let screens = screens(platform);
        let mut wrong = Vec::new();
        let Some(main) = screens.first() else {
            return vec!["the platform lists no screen"];
        };
        if main.frame.top_left() != Offset::ZERO {
            wrong.push("the main screen is not where positions start from");
        }
        if screens
            .iter()
            .any(|screen| screen.frame.is_empty() || screen.scale <= 0.0)
        {
            wrong.push("a screen has no size or no scale");
        }
        if screens
            .iter()
            .any(|screen| !inside(screen.visible_frame, screen.frame))
        {
            wrong.push("a screen's visible frame is not inside it");
        }
        if platform.locales().is_empty() {
            wrong.push("the platform lists none of the user's languages");
        }
        wrong
    }

    /// Whether `inner` is all within `outer`.
    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.left >= outer.left
            && inner.top >= outer.top
            && inner.right <= outer.right
            && inner.bottom <= outer.bottom
    }

    impl EmbedderClient for ScreenClient {
        fn frame(&mut self, _frame: Frame) {}

        fn wake(&mut self, _elapsed: Duration) {}

        fn view_added(&mut self, id: ViewId) {
            let view = self.platform.view(id).expect("the platform lists the view");
            let metrics = view.metrics();
            let Some(on) = metrics.screen else {
                support::fail(TEST, "the view is on no screen");
            };
            let Some(screen) = screens(&self.platform)
                .into_iter()
                .find(|screen| screen.id == on)
            else {
                support::fail(TEST, "the view's screen is not listed");
            };
            if screen.scale != metrics.device_pixel_ratio {
                support::fail(TEST, "the view's pixel ratio is not its screen's scale");
            }
            support::pass(TEST);
        }

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
