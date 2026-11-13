//! An app adds a view on a composition layer of its own and controls it as Flutter's embedder
//! controls a view: the view arrives carrying its request's id, draws into the layer, takes the
//! metrics the app gives it, holds its frames while the app says it is out of sight, takes a tap the
//! app forwards, asks what it asks of the system of the host the app gave it, hears the focus change
//! the app reports, and goes when the app removes it. The app's asking to resize or remove a
//! window's view is refused.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server.

// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "an_app_adds_a_view_on_a_layer_and_controls_it";

/// Long enough for every step, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(15);

#[cfg(target_os = "macos")]
fn main() {
    use waft_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::AddingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_quartz_core::CALayer;
    use waft_embedder::{
        Canvas, CompositionLayer, Contact, EmbedderClient, Frame, KeyData, Matrix4, Mutator,
        Offset, Paint, Placement, PlatformRef, PointerDeviceKind, PointerReport, Rect,
        ReportedChange, RequestId, Scene, SceneBuilder, SystemMouseCursorKind, TextInputHost,
        ViewConstraints, ViewEmbedder, ViewFocusDirection, ViewFocusEvent, ViewFocusState,
        ViewHost, ViewId, ViewMetrics,
    };
    use waft_embedder_winit::WinitPlatform;

    use crate::TEST;
    use crate::support::{self, appkit};

    /// The added view's size as the app adds it, and as it resizes it, in physical pixels.
    const SIZE: [f64; 2] = [200.0, 100.0];
    const RESIZED: [f64; 2] = [300.0, 150.0];
    /// The pixels of the added view to one of its parent layer's points.
    const SCALE: f64 = 2.0;
    /// The size the app asks a window's view to take, which it is refused.
    const REFUSED: [f64; 2] = [123.0, 45.0];
    /// The frames presented into the added view before its layer is looked at.
    const FRAMES_PRESENTED: u32 = 5;
    /// How long the added view is out of sight, with a frame asked for that must not come.
    const OUT_OF_SIGHT_FOR: Duration = Duration::from_millis(300);
    /// The forwarded mouse's device number: 64 random bits, which none of the host's own devices
    /// takes.
    const FORWARDED_MOUSE: i64 = 0x5d2e_91c4_7a03_b16f;

    /// Where the steps are.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Step {
        /// Waiting for the added view and the window's view.
        Arriving,
        /// Presenting frames into the added view.
        Drawing,
        /// Waiting to hear the added view's new metrics.
        Resizing,
        /// The added view is out of sight, with a frame asked for, since `since`.
        OutOfSight { since: Instant },
        /// The added view is in sight again, and its held frame is owed.
        BackInSight,
        /// A tap was forwarded to the added view.
        Tapped,
        /// A focus change of the added view was reported.
        Focusing,
        /// The added view was removed.
        Removing,
    }

    /// What the added view asks of the system, recorded.
    #[derive(Default)]
    struct RecordingHost {
        cursors: RefCell<Vec<(i64, SystemMouseCursorKind)>>,
        focus_requests: RefCell<Vec<(ViewFocusState, ViewFocusDirection)>>,
    }

    impl ViewHost for RecordingHost {
        fn text_input(&self) -> Option<&dyn TextInputHost> {
            None
        }

        fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
            self.cursors.borrow_mut().push((device, kind));
        }

        fn request_focus_change(&self, state: ViewFocusState, direction: ViewFocusDirection) {
            self.focus_requests.borrow_mut().push((state, direction));
        }
    }

    /// Adds a view on a layer it placed in a layer of its own, asks for a window, and steps the
    /// added view through what the app controls of it.
    pub(crate) struct AddingClient {
        platform: PlatformRef,
        /// The layer the test holds, in which the added view's layer is placed.
        parent: Retained<CALayer>,
        /// The added view's layer, which the app holds as long as the view draws into it.
        _layer: Arc<dyn CompositionLayer>,
        host: Rc<RecordingHost>,
        added_request: RequestId,
        window_request: RequestId,
        added: Option<ViewId>,
        window_view: Option<ViewId>,
        step: Step,
        presented: u32,
    }

    impl AddingClient {
        pub(crate) fn new(platform: PlatformRef) -> AddingClient {
            let parent = layer_of_points(SIZE.map(|side| side / SCALE));
            let layer = place_layer_in(&platform, &parent);
            let host = Rc::new(RecordingHost::default());
            let added_request = platform
                .view_embedder()
                .expect("the macOS host embeds views")
                .add_view(
                    &layer,
                    metrics(&platform, SIZE),
                    Rc::clone(&host) as Rc<dyn ViewHost>,
                    None,
                )
                .expect("the macOS host adds a view on a layer of its compositor's");
            let window_request = support::request_window(&platform, support::titled(TEST));
            AddingClient {
                platform,
                parent,
                _layer: layer,
                host,
                added_request,
                window_request,
                added: None,
                window_view: None,
                step: Step::Arriving,
                presented: 0,
            }
        }

        fn embedder(&self) -> &dyn ViewEmbedder {
            self.platform
                .view_embedder()
                .expect("the macOS host embeds views")
        }

        fn added(&self) -> ViewId {
            self.added.expect("the added view arrived")
        }

        /// Once both views have arrived: the app asks to resize and to remove the window's view,
        /// which is refused, and starts drawing into the added view.
        fn start_once_both_arrived(&mut self) {
            let (Some(added), Some(window_view)) = (self.added, self.window_view) else {
                return;
            };
            self.embedder()
                .set_view_metrics(window_view, metrics(&self.platform, REFUSED));
            self.embedder().remove_view(window_view);
            self.step = Step::Drawing;
            self.platform.request_frame(added);
        }

        /// Presents a square into the added view; once enough are presented, checks that it draws
        /// into its layer and resizes it.
        fn draw(&mut self) {
            self.present_into_added();
            self.presented += 1;
            if self.presented < FRAMES_PRESENTED {
                self.platform.request_frame(self.added());
                return;
            }
            if !appkit::draws_into_a_metal_layer_in(&self.parent) {
                support::fail(TEST, "the added view does not draw into its layer");
            }
            self.step = Step::Resizing;
            self.embedder()
                .set_view_metrics(self.added(), metrics(&self.platform, RESIZED));
        }

        fn present_into_added(&self) {
            let view = self.platform.view(self.added()).expect("the added view");
            view.present(a_square());
        }

        /// The added view took its new metrics: the app says it is out of sight and asks for a
        /// frame of it, which must not come until it is in sight again.
        fn hide_once_resized(&mut self) {
            let view = self.platform.view(self.added()).expect("the added view");
            if view.metrics().physical_size != RESIZED {
                support::fail(
                    TEST,
                    "the added view did not take the metrics the app gave it",
                );
            }
            self.step = Step::OutOfSight {
                since: Instant::now(),
            };
            self.embedder().set_view_in_sight(self.added(), false);
            self.platform.request_frame(self.added());
            support::wake_after_a_tick(&self.platform);
        }

        fn show_once_hidden_long_enough(&mut self, since: Instant) {
            if since.elapsed() < OUT_OF_SIGHT_FOR {
                support::wake_after_a_tick(&self.platform);
                return;
            }
            self.step = Step::BackInSight;
            self.embedder().set_view_in_sight(self.added(), true);
        }

        /// The held frame came as the view came into sight: the app forwards a tap to it.
        fn tap_once_back_in_sight(&mut self) {
            self.present_into_added();
            self.step = Step::Tapped;
            self.embedder().send_pointer_reports(tap(self.added()));
        }

        /// The tap arrived: the view asks what it asks of the system, which reaches the app's
        /// host, and the app reports the view focused.
        fn focus_once_tapped(&mut self, reports: Vec<PointerReport>) {
            if reports != tap(self.added()) {
                support::fail(TEST, &format!("the forwarded tap arrived as {reports:?}"));
            }
            let view = self.platform.view(self.added()).expect("the added view");
            view.host()
                .activate_system_cursor(FORWARDED_MOUSE, SystemMouseCursorKind::Click);
            view.host()
                .request_focus_change(ViewFocusState::Focused, ViewFocusDirection::Forward);
            self.step = Step::Focusing;
            self.embedder().send_view_focus_event(focused(self.added()));
        }

        fn remove_once_focused(&mut self, event: ViewFocusEvent) {
            let ViewFocusEvent {
                state, direction, ..
            } = event;
            if (state, direction) != (ViewFocusState::Focused, ViewFocusDirection::Forward) {
                support::fail(TEST, &format!("the focus change arrived as {event:?}"));
            }
            self.step = Step::Removing;
            self.embedder().remove_view(self.added());
        }

        /// What is wrong once the added view went.
        fn what_is_wrong(&self) -> Vec<&'static str> {
            let mut wrong = Vec::new();
            if self.platform.view(self.added()).is_some() {
                wrong.push("the platform lists the removed view");
            }
            if *self.host.cursors.borrow() != [(FORWARDED_MOUSE, SystemMouseCursorKind::Click)] {
                wrong.push("the added view's cursor did not reach the app's host");
            }
            let focus_requests = self.host.focus_requests.borrow();
            if *focus_requests != [(ViewFocusState::Focused, ViewFocusDirection::Forward)] {
                wrong.push("the added view's focus request did not reach the app's host");
            }
            let window_view = self.window_view.and_then(|id| self.platform.view(id));
            match window_view {
                None => wrong.push("the window's view was removed at the app's word"),
                Some(view) if view.metrics().physical_size == REFUSED => {
                    wrong.push("the window's view took the metrics the app gave it");
                }
                Some(_) => {}
            }
            wrong
        }
    }

    /// A layer of `size` points, which a layer placed in it at the origin fills.
    fn layer_of_points(size: [f64; 2]) -> Retained<CALayer> {
        let layer = CALayer::new();
        layer.setBounds(CGRect::new(
            CGPoint::new(0.0, 0.0),
            CGSize::new(size[0], size[1]),
        ));
        layer
    }

    /// A composition layer of the platform's, which sees through, placed in `parent` filling it:
    /// at its logical size, under a transform by `SCALE` to its pixels, as a view's scene places a
    /// layer.
    fn place_layer_in(platform: &PlatformRef, parent: &CALayer) -> Arc<dyn CompositionLayer> {
        let layer = platform
            .compositor()
            .expect("the macOS host has a compositor")
            .make_layer(false);
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, SIZE[0] / SCALE, SIZE[1] / SCALE),
            mutators: vec![Mutator::Transform(Matrix4::scale(
                SCALE as f32,
                SCALE as f32,
            ))],
        };
        platform
            .downcast_ref::<WinitPlatform>()
            .and_then(WinitPlatform::macos)
            .expect("the macOS host")
            .place_layer_in(parent, &layer, &placement, SCALE);
        layer
    }

    /// A view's metrics at `size` physical pixels, `SCALE` to a logical one, on the main screen,
    /// whose refreshes its frames follow.
    fn metrics(platform: &PlatformRef, size: [f64; 2]) -> ViewMetrics {
        let main_screen = platform
            .screens()
            .and_then(|screens| screens.screens().first().map(|screen| screen.id));
        ViewMetrics {
            physical_size: size,
            physical_constraints: ViewConstraints::tight(size[0], size[1]),
            device_pixel_ratio: SCALE,
            screen: main_screen,
            ..ViewMetrics::default()
        }
    }

    /// A click of the forwarded mouse in the middle of `view`: it comes in, presses, lifts and
    /// leaves.
    fn tap(view: ViewId) -> Vec<PointerReport> {
        let changes = [
            ReportedChange::Add,
            ReportedChange::Down { buttons: 1 },
            ReportedChange::Up { buttons: 0 },
            ReportedChange::Remove,
        ];
        changes
            .into_iter()
            .map(|change| PointerReport {
                view_id: view,
                time_stamp: Duration::ZERO,
                device: FORWARDED_MOUSE,
                kind: PointerDeviceKind::Mouse,
                position: [SIZE[0] / 2.0, SIZE[1] / 2.0],
                change,
                contact: Contact::default(),
            })
            .collect()
    }

    fn focused(view: ViewId) -> ViewFocusEvent {
        ViewFocusEvent {
            view_id: view,
            state: ViewFocusState::Focused,
            direction: ViewFocusDirection::Forward,
        }
    }

    /// A scene with a square in it.
    fn a_square() -> Arc<Scene> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(20.0, 20.0, 60.0, 60.0), &Paint::default());
        let mut scene = SceneBuilder::new();
        scene.add_picture(Offset::ZERO, &Arc::new(canvas.build()), false, false);
        Arc::new(scene.build())
    }

    impl EmbedderClient for AddingClient {
        fn frame(&mut self, views: &[ViewId], _frame: Frame) {
            if self.added.is_none_or(|added| !views.contains(&added)) {
                return;
            }
            match self.step {
                Step::Drawing => self.draw(),
                Step::OutOfSight { .. } => {
                    support::fail(TEST, "a frame of the added view came while out of sight");
                }
                Step::BackInSight => self.tap_once_back_in_sight(),
                _ => {}
            }
        }

        fn wake(&mut self, _elapsed: Duration) {
            if let Step::OutOfSight { since } = self.step {
                self.show_once_hidden_long_enough(since);
            }
        }

        fn view_added(&mut self, id: ViewId) {
            let view = self.platform.view(id).expect("the platform lists the view");
            if view.request_id() == Some(self.added_request) {
                if view.metrics().physical_size != SIZE {
                    support::fail(TEST, "the added view arrived at other metrics");
                }
                self.added = Some(id);
            } else if view.request_id() == Some(self.window_request) {
                self.window_view = Some(id);
            }
            self.start_once_both_arrived();
        }

        fn view_metrics_changed(&mut self, id: ViewId) {
            if self.step == Step::Resizing && Some(id) == self.added {
                self.hide_once_resized();
            }
        }

        fn view_removed(&mut self, id: ViewId) {
            if Some(id) != self.added {
                return;
            }
            if self.step != Step::Removing {
                support::fail(TEST, "the added view went before the app removed it");
            }
            let wrong = self.what_is_wrong();
            if !wrong.is_empty() {
                support::fail(TEST, &format!("{wrong:?}"));
            }
            support::pass(TEST);
        }

        /// The window's own reports, of a mouse that happens to be over it, are not the tap's.
        fn pointer_reports(&mut self, reports: Vec<PointerReport>) {
            let over_added = reports
                .iter()
                .any(|report| Some(report.view_id) == self.added);
            if self.step == Step::Tapped && over_added {
                self.focus_once_tapped(reports);
            }
        }

        fn view_focus_changed(&mut self, event: ViewFocusEvent) {
            if self.step == Step::Focusing && Some(event.view_id) == self.added {
                self.remove_once_focused(event);
            }
        }

        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
