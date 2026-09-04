//! Flutter counterpart: `rendering/binding.dart` (`RendererBinding`).
//!
//! Semantics, `PipelineManifold`, and `performReassemble` wait.

use std::rc::Rc;

use reveal_embedder::{Offset, ViewId};
use reveal_foundation::{App, Handle, Listener};
use reveal_gestures::{GestureBinding, GestureBindingOverridesObject, HitTestResult, PointerEvent};
use reveal_scheduler::{FrameCallback, SchedulerBinding};

use crate::mouse_tracker::MouseTracker;
use crate::object::RenderHandle;
use crate::pipeline_owner::PipelineOwner;
use crate::view::{RenderView, ViewConfiguration};

/// The glue between the render trees and the Flutter engine.
///
/// Flutter's `RendererBinding` mixin. Like Flutter's binding it is the App's singleton;
/// members take `&mut App`.

#[derive(Default)]
pub struct RendererBinding {
    root_pipeline_owner: Option<Handle<PipelineOwner>>,
    /// Flutter's deprecated `pipelineOwner`, created on first use.
    pipeline_owner: Option<Handle<PipelineOwner>>,
    /// Flutter's deprecated `renderView` for the implicit view, created on first use.
    render_view: Option<RenderHandle<RenderView>>,
    render_view_initial_frame_prepared: bool,
    render_views: Vec<(ViewId, RenderHandle<RenderView>)>,
    mouse_tracker: Option<Handle<MouseTracker>>,
    debug_mouse_tracker_update_scheduled: bool,
    first_frame_deferred_count: u32,
    first_frame_sent: bool,
}

impl RendererBinding {
    /// The current binding, initialized on first access (Flutter's `initInstances`).
    pub fn instance(app: &mut App) -> Handle<RendererBinding> {
        let this: Handle<RendererBinding> = app.singleton();
        if app.get(this).root_pipeline_owner.is_none() {
            let root_pipeline_owner = this.create_root_pipeline_owner(app);
            app.get_mut(this).root_pipeline_owner = Some(root_pipeline_owner);
            SchedulerBinding::add_persistent_frame_callback(
                app,
                FrameCallback::new(|app, _time_stamp| {
                    RendererBinding::instance(app).handle_persistent_frame_callback(app)
                }),
            );
            this.init_mouse_tracker(app, None);
            // Flutter's `RendererBinding` overrides `GestureBinding.hitTestInView` and
            // `dispatchEvent`.
            GestureBinding::instance(app).set_overrides(app, Rc::new(this));
        }
        this
    }

    /// The object that manages state about currently connected mice, for hover
    /// notification.
    pub fn mouse_tracker(self: Handle<Self>, app: &App) -> Handle<MouseTracker> {
        app.get(self)
            .mouse_tracker
            .expect("RendererBinding::instance must run first")
    }

    /// Creates a [`MouseTracker`] which manages state about currently connected
    /// mice, for hover notification.
    ///
    /// Used by testing framework to reinitialize the mouse tracker between tests.
    pub fn init_mouse_tracker(
        self: Handle<Self>,
        app: &mut App,
        tracker: Option<Handle<MouseTracker>>,
    ) {
        if let Some(previous) = app.get(self).mouse_tracker {
            previous.dispose(app);
        }
        let tracker = tracker.unwrap_or_else(|| {
            MouseTracker::new(
                app,
                Rc::new(move |app: &mut App, position: Offset, view_id: ViewId| {
                    let mut result = HitTestResult::new();
                    self.hit_test_in_view(app, &mut result, position, view_id);
                    result
                }),
            )
        });
        app.get_mut(self).mouse_tracker = Some(tracker);
    }

    /// The [`PipelineOwner`] that is the root of the pipeline owner tree.
    ///
    /// Typically, this owner has no root render object (a `RenderView` is attached to the
    /// owners the widget layer adds as children). Until then, [`init_render_view`](Self::init_render_view)
    /// attaches the implicit view here.
    pub fn root_pipeline_owner(self: Handle<Self>, app: &App) -> Handle<PipelineOwner> {
        app.get(self)
            .root_pipeline_owner
            .expect("RendererBinding::instance must run first")
    }

    /// Creates the [`PipelineOwner`] that serves as the root of the pipeline owner tree.
    fn create_root_pipeline_owner(self: Handle<Self>, app: &mut App) -> Handle<PipelineOwner> {
        PipelineOwner::new(
            app,
            Some(Listener::new(|app| {
                SchedulerBinding::ensure_visual_update(app);
            })),
        )
    }

    /// The [`PipelineOwner`] Flutter's binding keeps for the implicit view.
    pub fn pipeline_owner(self: Handle<Self>, app: &mut App) -> Handle<PipelineOwner> {
        if let Some(owner) = app.get(self).pipeline_owner {
            return owner;
        }
        let owner = PipelineOwner::new(app, None);
        app.get_mut(self).pipeline_owner = Some(owner);
        owner
    }

    /// The [`RenderView`] for the platform's implicit view, created on first access.
    ///
    /// # Panics
    ///
    /// If the platform has no implicit view.
    pub fn render_view(self: Handle<Self>, app: &mut App) -> RenderHandle<RenderView> {
        if let Some(view) = app.get(self).render_view {
            return view;
        }
        let implicit_view = app
            .platform()
            .implicit_view()
            .expect("the platform has no implicit view");
        let view = RenderView::new(app, None, None, implicit_view);
        app.get_mut(self).render_view = Some(view);
        view
    }

    /// The [`RenderView`]s managed by this binding.
    pub fn render_views(self: Handle<Self>, app: &App) -> Vec<RenderHandle<RenderView>> {
        app.get(self)
            .render_views
            .iter()
            .map(|(_, view)| *view)
            .collect()
    }

    /// Adds a [`RenderView`] to this binding.
    ///
    /// The binding will interact with the [`RenderView`] in the following ways:
    ///
    /// - setting and updating [`RenderView::configuration`],
    /// - calling [`RenderView::composite_frame`] when it is time to produce a new frame.
    pub fn add_render_view(self: Handle<Self>, app: &mut App, view: RenderHandle<RenderView>) {
        let view_id = view.flutter_view(app).id();
        debug_assert!(
            !app.get(self)
                .render_views
                .iter()
                .any(|(id, existing)| *id == view_id || *existing == view)
        );
        app.get_mut(self).render_views.push((view_id, view));
        let configuration = self.create_view_configuration_for(app, view);
        view.set_configuration(app, configuration);
    }

    /// Removes a [`RenderView`] previously added with [`add_render_view`](Self::add_render_view).
    pub fn remove_render_view(self: Handle<Self>, app: &mut App, view: RenderHandle<RenderView>) {
        let view_id = view.flutter_view(app).id();
        let views = &mut app.get_mut(self).render_views;
        debug_assert!(
            views
                .iter()
                .any(|(id, existing)| *id == view_id && *existing == view)
        );
        views.retain(|(id, _)| *id != view_id);
    }

    /// Returns a [`ViewConfiguration`] configured for the provided [`RenderView`] based on
    /// the current environment.
    pub fn create_view_configuration_for(
        self: Handle<Self>,
        app: &App,
        render_view: RenderHandle<RenderView>,
    ) -> ViewConfiguration {
        ViewConfiguration::from_view(&*render_view.flutter_view(app))
    }

    /// Called when the system metrics change.
    ///
    /// See `View::metrics`.
    pub fn handle_metrics_changed(self: Handle<Self>, app: &mut App) {
        let mut force_frame = false;
        for view in self.render_views(app) {
            force_frame = force_frame || view.child(app).is_some();
            let configuration = self.create_view_configuration_for(app, view);
            view.set_configuration(app, configuration);
        }
        if force_frame {
            SchedulerBinding::schedule_forced_frame(app);
        }
    }

    fn handle_persistent_frame_callback(self: Handle<Self>, app: &mut App) {
        self.draw_frame(app);
        self.schedule_mouse_tracker_update(app);
    }

    fn schedule_mouse_tracker_update(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).debug_mouse_tracker_update_scheduled);
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_mouse_tracker_update_scheduled = true;
        }
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                debug_assert!(app.get(self).debug_mouse_tracker_update_scheduled);
                if cfg!(debug_assertions) {
                    app.get_mut(self).debug_mouse_tracker_update_scheduled = false;
                }
                self.mouse_tracker(app).update_all_devices(app);
            }),
        );
    }

    /// Whether frames produced by [`draw_frame`](Self::draw_frame) are sent to the host.
    ///
    /// If false the framework will do all the work to produce a frame, but the frame is never
    /// sent to the host to actually appear on screen.
    pub fn send_frames_to_engine(self: Handle<Self>, app: &App) -> bool {
        let data = app.get(self);
        data.first_frame_sent || data.first_frame_deferred_count == 0
    }

    /// Tell the framework to not send the first frames to the host until there is a
    /// corresponding call to [`allow_first_frame`](Self::allow_first_frame).
    pub fn defer_first_frame(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).first_frame_deferred_count += 1;
    }

    /// Called after [`defer_first_frame`](Self::defer_first_frame) to tell the framework that it
    /// is ok to send the first frame to the host now.
    pub fn allow_first_frame(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).first_frame_deferred_count > 0);
        app.get_mut(self).first_frame_deferred_count -= 1;
        if !app.get(self).first_frame_sent {
            SchedulerBinding::schedule_forced_frame(app);
        }
    }

    /// Call this to pretend that no frames have been sent to the host yet.
    pub fn reset_first_frame_sent(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).first_frame_sent = false;
    }

    /// Pump the rendering pipeline to generate a frame.
    ///
    /// This method is called by `handle_draw_frame`, which itself is called automatically by
    /// the host when it is time to lay out and paint a frame.
    ///
    /// Each frame consists of the following phases: layout, paint, then compositing (for each
    /// registered [`RenderView`]).
    pub fn draw_frame(self: Handle<Self>, app: &mut App) {
        let root_pipeline_owner = self.root_pipeline_owner(app);
        root_pipeline_owner.flush_layout(app);
        root_pipeline_owner.flush_paint(app);
        if self.send_frames_to_engine(app) {
            for render_view in self.render_views(app) {
                render_view.composite_frame(app); // this sends the bits to the GPU
            }
            app.get_mut(self).first_frame_sent = true;
        }
    }

    /// Flutter's `TestRenderingFlutterBinding.initRenderView`: makes the implicit view's
    /// [`RenderView`] the root of [`root_pipeline_owner`](Self::root_pipeline_owner), registers
    /// it, and prepares its first frame. Idempotent, as `_ReusableRenderView` is.
    pub fn init_render_view(self: Handle<Self>, app: &mut App) -> RenderHandle<RenderView> {
        let render_view = self.render_view(app);
        if app.get(self).render_view_initial_frame_prepared {
            return render_view;
        }
        self.root_pipeline_owner(app)
            .set_root_node(app, Some(render_view.as_object()));
        self.add_render_view(app, render_view);
        render_view.prepare_initial_frame(app);
        app.get_mut(self).render_view_initial_frame_prepared = true;
        render_view
    }
}

/// Flutter's `RendererBinding.hitTestInView` override: the render view for `view_id` hit-tests
/// first; `GestureBinding` then adds itself.
/// Flutter's `RendererBinding` overrides of `GestureBinding`: `hitTestInView` walks the render
/// trees, `dispatchEvent` updates the mouse tracker before routing.
impl GestureBindingOverridesObject for RendererBinding {
    fn hit_test(self: Handle<Self>, app: &mut App, result: &mut HitTestResult, position: Offset) {
        let Some(view) = app.platform().implicit_view() else {
            return;
        };
        self.hit_test_in_view(app, result, position, view.id());
    }

    fn hit_test_in_view(
        self: Handle<Self>,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    ) {
        let render_view = app
            .get(self)
            .render_views
            .iter()
            .find(|(id, _)| *id == view_id)
            .map(|(_, view)| *view);
        if let Some(render_view) = render_view {
            render_view.hit_test(app, result, position);
        }
    }

    fn will_dispatch_event(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    ) {
        // When the button is pressed, normal hit test uses a cached
        // result, but MouseTracker requires that the hit test is re-executed to
        // update the hovering events.
        let result = match event {
            PointerEvent::Move(_) => None,
            _ => hit_test_result,
        };
        self.mouse_tracker(app)
            .update_with_event(app, event, result);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use reveal_embedder::{
        Picture, Platform, PlatformRef, Size, TargetPlatform, View, ViewMetrics, ViewRef,
    };

    use super::*;
    use crate::box_::{BoxConstraints, RenderBox};
    use crate::proxy_box::RenderConstrainedBox;

    struct TestView {
        presented: Rc<Cell<u32>>,
    }

    impl View for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics {
                physical_size: [800.0, 600.0],
                physical_constraints: reveal_embedder::ViewConstraints::tight(800.0, 600.0),
                device_pixel_ratio: 2.0,
                ..ViewMetrics::default()
            }
        }

        fn present(&self, _picture: &Picture) {
            self.presented.set(self.presented.get() + 1);
        }
    }

    struct TestPlatform {
        view: ViewRef,
        frames: Rc<Cell<u32>>,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn request_frame(&self) {
            self.frames.set(self.frames.get() + 1);
        }

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }
    }

    fn app_with_view() -> (App, Rc<Cell<u32>>, Rc<Cell<u32>>) {
        let presented = Rc::new(Cell::new(0));
        let frames = Rc::new(Cell::new(0));
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::new(TestView {
                presented: Rc::clone(&presented),
            }),
            frames: Rc::clone(&frames),
        });
        (App::with_platform(platform), presented, frames)
    }

    #[test]
    fn a_frame_lays_out_paints_and_presents_the_implicit_view() {
        let (mut app, presented, _frames) = app_with_view();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app);
        assert_eq!(
            render_view.configuration(&app).logical_constraints,
            BoxConstraints::tight(Size::new(400.0, 300.0))
        );
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        render_view.set_child(&mut app, Some(child.as_box()));

        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);

        assert_eq!(render_view.size(&app), Size::new(400.0, 300.0));
        assert_eq!(child.size(&app), Size::new(400.0, 300.0));
        assert_eq!(presented.get(), 1);
        assert!(!child.debug_needs_layout(&app));
        assert!(!child.as_object().debug_needs_paint(&app));
    }

    #[test]
    fn a_dirty_node_requests_a_visual_update() {
        let (mut app, _presented, frames) = app_with_view();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app);
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        render_view.set_child(&mut app, Some(child.as_box()));
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);
        let before = frames.get();

        child.mark_needs_layout(&mut app);
        assert!(
            frames.get() > before,
            "the root owner's visual update reaches the host"
        );
    }

    #[test]
    fn a_pointer_down_reaches_a_pointer_listener_through_the_gesture_binding() {
        use reveal_gestures::{PointerDownEvent, PointerEvent};

        use crate::proxy_box::{HitTestBehavior, RenderPointerListener};

        let (mut app, _presented, _frames) = app_with_view();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app);
        let listener = RenderPointerListener::new(&mut app, HitTestBehavior::Opaque, None);
        let positions = Rc::new(std::cell::RefCell::new(Vec::new()));
        let recorded = Rc::clone(&positions);
        listener.set_on_pointer_down(
            &mut app,
            Some(Rc::new(move |_app: &mut App, event: PointerDownEvent| {
                recorded.borrow_mut().push(event.local_position());
            })),
        );
        render_view.set_child(&mut app, Some(listener.as_box()));
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);

        GestureBinding::instance(&mut app).handle_pointer_event(
            &mut app,
            PointerEvent::Down(PointerDownEvent {
                position: Offset::new(30.0, 40.0),
                ..PointerDownEvent::default()
            }),
        );
        assert_eq!(*positions.borrow(), [Offset::new(30.0, 40.0)]);
    }

    #[test]
    fn a_mouse_hover_enters_a_mouse_region_and_a_frame_notices_it_moving_away() {
        use reveal_embedder::PointerDeviceKind;
        use reveal_gestures::{
            PointerAddedEvent, PointerEnterEvent, PointerEvent, PointerExitEvent,
        };
        use reveal_painting::EdgeInsetsGeometry;

        use crate::proxy_box::RenderMouseRegion;
        use crate::shifted_box::RenderPadding;

        let (mut app, _presented, _frames) = app_with_view();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app);
        let region = RenderMouseRegion::new(&mut app, true, None);
        let log = Rc::new(std::cell::RefCell::new(Vec::new()));
        let entered = Rc::clone(&log);
        region.set_on_enter(
            &mut app,
            Some(Rc::new(move |_app: &mut App, event: PointerEnterEvent| {
                entered
                    .borrow_mut()
                    .push(format!("enter {:?}", event.local_position()));
            })),
        );
        let exited = Rc::clone(&log);
        region.set_on_exit(
            &mut app,
            Some(Rc::new(move |_app: &mut App, event: PointerExitEvent| {
                exited
                    .borrow_mut()
                    .push(format!("exit {:?}", event.local_position()));
            })),
        );
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(100.0),
            None,
            Some(region.as_box()),
        );
        render_view.set_child(&mut app, Some(padding.as_box()));
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);

        GestureBinding::instance(&mut app).handle_pointer_event(
            &mut app,
            PointerEvent::Added(PointerAddedEvent {
                kind: PointerDeviceKind::Mouse,
                position: Offset::new(150.0, 150.0),
                ..PointerAddedEvent::default()
            }),
        );
        assert_eq!(*log.borrow(), ["enter Offset(50.0, 50.0)"]);
        assert!(binding.mouse_tracker(&app).mouse_is_connected(&app));

        // The tree moves under the still mouse; the post-frame update sends the exit.
        padding.set_padding(&mut app, EdgeInsetsGeometry::only(300.0, 0.0, 0.0, 0.0));
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::from_millis(16)));
        SchedulerBinding::handle_draw_frame(&mut app);
        assert_eq!(
            *log.borrow(),
            ["enter Offset(50.0, 50.0)", "exit Offset(50.0, 50.0)"],
            "the exit is placed with the transform recorded when the region was entered"
        );
    }

    #[test]
    fn deferred_first_frame_is_not_presented() {
        let (mut app, presented, _frames) = app_with_view();
        let binding = RendererBinding::instance(&mut app);
        binding.init_render_view(&mut app);
        binding.defer_first_frame(&mut app);
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);
        assert_eq!(presented.get(), 0);

        binding.allow_first_frame(&mut app);
        SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::ZERO));
        SchedulerBinding::handle_draw_frame(&mut app);
        assert_eq!(presented.get(), 1);
    }
}
