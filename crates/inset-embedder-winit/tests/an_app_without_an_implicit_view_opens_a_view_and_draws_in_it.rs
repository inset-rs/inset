//! An app started with no implicit view has no views; it opens one through the windowing owner,
//! hears it added, and draws in it. The view gets its surface as its window opens, since the
//! system gave the windows theirs at launch, and the frames the app asks for are presented
//! into it.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server.

// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "an_app_without_an_implicit_view_opens_a_view_and_draws_in_it";

/// Long enough for the window to open and the frames to be drawn, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the view was not drawn in within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::OpeningClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::sync::Arc;
    use std::time::Duration;

    use inset_embedder::{
        Canvas, EmbedderClient, Frame, KeyData, Offset, Paint, PlatformRef, PointerDataPacket,
        Rect, Scene, SceneBuilder, ViewId, WindowConfig, WindowFuture, WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    const TITLE: &str = "opened after launch";
    /// The frames presented into the view before what it shows is looked at.
    const FRAMES_PRESENTED: u32 = 10;

    /// Checks that it starts with no view, opens one, and presents a square into it from each
    /// frame it asks for; once enough frames are presented, checks that the view was added and
    /// draws on a surface.
    pub(crate) struct OpeningClient {
        platform: PlatformRef,
        opening: Vec<WindowFuture>,
        windows: Vec<WindowRef>,
        added: Vec<ViewId>,
        presented: u32,
    }

    impl OpeningClient {
        pub(crate) fn new(platform: PlatformRef) -> OpeningClient {
            if !platform.views().is_empty() || platform.implicit_view().is_some() {
                support::fail(TEST, "the app started with a view");
            }
            let owner = platform
                .windowing_owner()
                .expect("a desktop host makes windows");
            let opening = vec![owner.create(WindowConfig {
                title: TITLE.into(),
                size: [300.0, 200.0],
                ..WindowConfig::default()
            })];
            OpeningClient {
                platform,
                opening,
                windows: Vec::new(),
                added: Vec::new(),
                presented: 0,
            }
        }

        /// Asks for the first frame once the window is made.
        fn draw_once_the_window_is_made(&mut self) {
            let was_made = !self.windows.is_empty();
            support::take_opened_windows(&mut self.opening, &mut self.windows);
            if !was_made && !self.windows.is_empty() {
                self.platform.request_frame();
            }
        }

        /// What is wrong with the view once the frames are presented.
        fn what_is_wrong(&self, window: &WindowRef) -> Vec<&'static str> {
            let id = window.view().id();
            let mut wrong = Vec::new();
            if self.added != [id] {
                wrong.push("the app did not hear the view added, once");
            }
            if !self.platform.views().iter().any(|view| view.id() == id) {
                wrong.push("the platform does not list the view");
            }
            if self.platform.implicit_view().is_some() {
                wrong.push("the view opened became the implicit view");
            }
            if !appkit::has_a_metal_layer(TITLE) {
                wrong.push("the view has no surface");
            }
            wrong
        }
    }

    /// A scene with a square in it.
    fn a_square() -> Arc<Scene> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(20.0, 20.0, 100.0, 100.0), &Paint::default());
        let mut scene = SceneBuilder::new();
        scene.add_picture(Offset::ZERO, &Arc::new(canvas.build()), false, false);
        Arc::new(scene.build())
    }

    impl EmbedderClient for OpeningClient {
        fn frame(&mut self, _frame: Frame) {
            let Some(window) = self.windows.first() else {
                return;
            };
            window.view().present(a_square());
            self.presented += 1;
            if self.presented < FRAMES_PRESENTED {
                self.platform.request_frame();
                return;
            }
            let wrong = self.what_is_wrong(window);
            if !wrong.is_empty() {
                support::fail(TEST, &format!("{wrong:?}"));
            }
            support::pass(TEST);
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.draw_once_the_window_is_made();
        }

        fn view_added(&mut self, id: ViewId) {
            self.added.push(id);
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
