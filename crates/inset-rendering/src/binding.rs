//! Flutter counterpart: `rendering/binding.dart` (`RendererBinding`).
//!
//! Semantics and `performReassemble` wait.

use std::rc::Rc;

use inset_embedder::{Offset, ViewId, ViewRef};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::{GestureBinding, GestureBindingOverridesObject, HitTestResult, PointerEvent};
use inset_scheduler::{FrameCallback, FrameTarget, PersistentFrameCallback, SchedulerBinding};

use crate::mouse_tracker::MouseTracker;
use crate::object::RenderHandle;
use crate::pipeline_owner::{PipelineManifold, PipelineOwner};
use crate::view::{RenderView, ViewConfiguration};

/// The object side of Flutter's `WidgetsBinding` overrides of `RendererBinding.drawFrame`:
/// the widget layer builds before the renderer's frame and finalizes its tree after it.
/// Implement it on the binding above and register `Rc::new(handle)` with
/// [`RendererBinding::set_overrides`]; `Handle<T>` is then [`RendererBindingOverrides`].
pub trait RendererBindingOverridesObject: Sized + 'static {
    /// Runs at the start of [`RendererBinding::draw_frame`], before layout.
    fn will_draw_frame(self: Handle<Self>, app: &mut App);

    /// Runs at the end of [`RendererBinding::draw_frame`], after compositing.
    fn did_draw_frame(self: Handle<Self>, app: &mut App);

    /// Runs at the end of [`RendererBinding::handle_metrics_changed`]: Flutter's
    /// `WidgetsBinding.handleMetricsChanged` override, which tells its observers.
    fn did_handle_metrics_changed(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }
}

/// What the binding above this one adds around `drawFrame`; kept erased.
pub trait RendererBindingOverrides {
    /// See [`RendererBindingOverridesObject::will_draw_frame`].
    fn will_draw_frame(&self, app: &mut App);

    /// See [`RendererBindingOverridesObject::did_draw_frame`].
    fn did_draw_frame(&self, app: &mut App);

    /// See [`RendererBindingOverridesObject::did_handle_metrics_changed`].
    fn did_handle_metrics_changed(&self, app: &mut App);
}

impl<T: RendererBindingOverridesObject> RendererBindingOverrides for Handle<T> {
    fn will_draw_frame(&self, app: &mut App) {
        T::will_draw_frame(*self, app);
    }

    fn did_draw_frame(&self, app: &mut App) {
        T::did_draw_frame(*self, app);
    }

    fn did_handle_metrics_changed(&self, app: &mut App) {
        T::did_handle_metrics_changed(*self, app);
    }
}

/// Dart's `_BindingPipelineManifold`: the [`PipelineManifold`] the binding attaches its
/// root [`PipelineOwner`] to. Its `semanticsEnabled` waits with semantics.
struct BindingPipelineManifold;

impl PipelineManifold for BindingPipelineManifold {
    /// A frame of every view, for an owner that renders no view of its own.
    fn request_visual_update(&self, app: &mut App) {
        SchedulerBinding::ensure_visual_update(app, FrameTarget::EveryView);
    }
}

#[derive(Default)]
pub struct RendererBinding {
    overrides: Option<Rc<dyn RendererBindingOverrides>>,
    root_pipeline_owner: Option<Handle<PipelineOwner>>,
    /// The render view [`init_render_view`](Self::init_render_view) made, which it answers again.
    test_render_view: Option<RenderHandle<RenderView>>,
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
            root_pipeline_owner.attach(app, Rc::new(BindingPipelineManifold));
            SchedulerBinding::add_persistent_frame_callback(
                app,
                PersistentFrameCallback::new(|app, views, _time_stamp| {
                    RendererBinding::instance(app).handle_persistent_frame_callback(app, views)
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
    /// owners the widget layer adds as children). A test that renders without widgets attaches
    /// its view's here, through [`init_render_view`](Self::init_render_view).
    pub fn root_pipeline_owner(self: Handle<Self>, app: &App) -> Handle<PipelineOwner> {
        app.get(self)
            .root_pipeline_owner
            .expect("RendererBinding::instance must run first")
    }

    /// Creates the [`PipelineOwner`] that serves as the root of the pipeline owner tree.
    ///
    /// When a test's render view is its root node ([`init_render_view`](Self::init_render_view)),
    /// that view is marked to be composited after a paint that had work.
    fn create_root_pipeline_owner(self: Handle<Self>, app: &mut App) -> Handle<PipelineOwner> {
        PipelineOwner::new(
            app,
            Some(Listener::new(|app| {
                SchedulerBinding::ensure_visual_update(app, FrameTarget::EveryView);
            })),
            Some(Rc::new(move |app: &mut App, is_dirty: bool| {
                if is_dirty {
                    let root_node = self.root_pipeline_owner(app).root_node(app);
                    if let Some(render_view) =
                        root_node.and_then(|root_node| root_node.downcast::<RenderView>(app))
                    {
                        render_view.mark_needs_composite_frame(app);
                    }
                }
            })),
        )
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
        // Ensure the view composites at least once in a non-warm-up frame after
        // being added, so its contents appear even if no descendants are dirty.
        view.mark_needs_composite_frame(app);
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

    /// Marks all views to require compositing, which will force them to re-render
    /// on their next frame.
    pub fn handle_mark_all_views_need_render(self: Handle<Self>, app: &mut App) {
        for view in self.render_views(app) {
            view.mark_needs_composite_frame(app);
        }
    }

    /// Called when the system metrics change.
    ///
    /// See `View::metrics`.
    pub fn handle_metrics_changed(self: Handle<Self>, app: &mut App) {
        for view in self.render_views(app) {
            let force_frame = view.child(app).is_some();
            let configuration = self.create_view_configuration_for(app, view);
            view.set_configuration(app, configuration);
            if force_frame {
                let view_id = view.flutter_view(app).id();
                SchedulerBinding::schedule_forced_frame(app, FrameTarget::View(view_id));
            }
        }
        if let Some(overrides) = app.get(self).overrides.clone() {
            overrides.did_handle_metrics_changed(app);
        }
    }

    fn handle_persistent_frame_callback(self: Handle<Self>, app: &mut App, views: &[ViewId]) {
        self.draw_frame(app, views);
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
            None,
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
            SchedulerBinding::schedule_forced_frame(app, FrameTarget::EveryView);
        }
    }

    /// Call this to pretend that no frames have been sent to the host yet.
    pub fn reset_first_frame_sent(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).first_frame_sent = false;
    }

    /// Registers the binding layered above this one, as Flutter's `WidgetsBinding` does by
    /// overriding `drawFrame`.
    pub fn set_overrides(
        self: Handle<Self>,
        app: &mut App,
        overrides: Rc<dyn RendererBindingOverrides>,
    ) {
        app.get_mut(self).overrides = Some(overrides);
    }

    /// Pump the rendering pipeline to generate a frame of `views`.
    ///
    /// This method is called by `handle_draw_frame`, which itself is called automatically by
    /// the host when it is time to lay out and paint a frame.
    ///
    /// Each frame consists of the following phases: layout and compositing bits for every
    /// render tree, then paint and compositing for the [`RenderView`] of each of `views`. A view
    /// paints its own render tree alone, not the views anchored in it, which paint at frames of
    /// their own; it is composited only when it [needs it](RenderView::needs_composite_frame),
    /// and otherwise keeps showing its last frame.
    pub fn draw_frame(self: Handle<Self>, app: &mut App, views: &[ViewId]) {
        let overrides = app.get(self).overrides.clone();
        if let Some(overrides) = &overrides {
            overrides.will_draw_frame(app);
        }
        let root_pipeline_owner = self.root_pipeline_owner(app);
        root_pipeline_owner.flush_layout(app);
        root_pipeline_owner.flush_compositing_bits(app);
        let render_views: Vec<RenderHandle<RenderView>> = views
            .iter()
            .filter_map(|&view| self.render_view_of(app, view))
            .collect();
        for render_view in &render_views {
            if let Some(owner) = render_view.as_object().owner(app) {
                owner.flush_paint_without_children(app);
            }
        }
        if self.send_frames_to_engine(app) {
            for render_view in render_views {
                if render_view.needs_composite_frame(app) {
                    render_view.composite_frame(app); // this sends the bits to the GPU
                }
                if SchedulerBinding::is_warm_up_frame(app) {
                    // Warm-up frame does not guarantee that the content is actually
                    // painted (see [SchedulerBinding::schedule_warm_up_frame]). This is to
                    // ensure that on the subsequent real frame the view is recomposited.
                    render_view.mark_needs_composite_frame(app);
                }
            }
            app.get_mut(self).first_frame_sent = true;
        }
        if let Some(overrides) = &overrides {
            overrides.did_draw_frame(app);
        }
    }

    /// The [`RenderView`] of `view`, if this binding has one.
    fn render_view_of(
        self: Handle<Self>,
        app: &App,
        view: ViewId,
    ) -> Option<RenderHandle<RenderView>> {
        app.get(self)
            .render_views
            .iter()
            .find(|(id, _)| *id == view)
            .map(|(_, render_view)| *render_view)
    }

    /// Flutter's `TestRenderingFlutterBinding.initRenderView`: makes a [`RenderView`] for `view`
    /// the root of [`root_pipeline_owner`](Self::root_pipeline_owner), registers it, and
    /// prepares its first frame. Idempotent, as `_ReusableRenderView` is: a later call answers
    /// the first call's render view.
    pub fn init_render_view(
        self: Handle<Self>,
        app: &mut App,
        view: ViewRef,
    ) -> RenderHandle<RenderView> {
        if let Some(render_view) = app.get(self).test_render_view {
            return render_view;
        }
        let render_view = RenderView::new(app, None, None, view);
        app.get_mut(self).test_render_view = Some(render_view);
        self.root_pipeline_owner(app)
            .set_root_node(app, Some(render_view.as_object()));
        self.add_render_view(app, render_view);
        render_view.prepare_initial_frame(app);
        render_view
    }
}

/// Flutter's `RendererBinding.hitTestInView` override: the render view for `view_id` hit-tests
/// first; `GestureBinding` then adds itself.
/// Flutter's `RendererBinding` overrides of `GestureBinding`: `hitTestInView` walks the render
/// trees, `dispatchEvent` updates the mouse tracker before routing.
impl GestureBindingOverridesObject for RendererBinding {
    fn hit_test_in_view(
        self: Handle<Self>,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    ) {
        if let Some(render_view) = self.render_view_of(app, view_id) {
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
    use std::time::Duration;

    use inset_embedder::{Size, TargetPlatform};
    use inset_foundation::AppCell;
    use inset_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};

    use super::*;
    use crate::box_::{BoxConstraints, RenderBox};
    use crate::proxy_box::RenderConstrainedBox;

    /// An [`App`] over one 800x600 view at 2x, with the view and the platform to read back.
    fn app_with_view() -> (Rc<AppCell>, Rc<TestView>, Rc<TestPlatform>) {
        let view = Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 2.0));
        let platform = Rc::new(
            TestPlatform::new()
                .on(TargetPlatform::MacOS)
                .with_view(view.clone()),
        );
        (AppCell::with_platform(platform.clone()), view, platform)
    }

    /// A frame of `views` at `at`, with the microtasks drained after each half.
    fn pump(app: &mut App, views: &[ViewId], at: Duration) {
        SchedulerBinding::handle_begin_frame(app, views, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app, views);
        app.drain_microtasks();
    }

    #[test]
    fn a_frame_lays_out_paints_and_presents_the_view() {
        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app, view.clone());
        assert_eq!(
            render_view.configuration(&app).logical_constraints,
            BoxConstraints::tight(Size::new(400.0, 300.0))
        );
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        render_view.set_child(&mut app, Some(child.as_box()));

        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);

        assert_eq!(render_view.size(&app), Size::new(400.0, 300.0));
        assert_eq!(child.size(&app), Size::new(400.0, 300.0));
        assert_eq!(view.presented(), 1);
        assert!(!child.debug_needs_layout(&app));
        assert!(!child.as_object().debug_needs_paint(&app));
    }

    #[test]
    fn a_dirty_node_requests_a_visual_update() {
        let (cell, view, platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app, view.clone());
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        render_view.set_child(&mut app, Some(child.as_box()));
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);
        platform.take_frame_requests();

        child.mark_needs_layout(&mut app);
        assert_eq!(
            platform.take_frame_requests(),
            [IMPLICIT_VIEW_ID],
            "the root owner's visual update reaches the host"
        );
    }

    #[test]
    fn a_pointer_down_reaches_a_pointer_listener_through_the_gesture_binding() {
        use inset_gestures::{PointerDownEvent, PointerEvent};

        use crate::proxy_box::{HitTestBehavior, RenderPointerListener};

        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app, view.clone());
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
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);

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
        use inset_embedder::PointerDeviceKind;
        use inset_gestures::{
            PointerAddedEvent, PointerEnterEvent, PointerEvent, PointerExitEvent,
        };
        use inset_painting::EdgeInsetsGeometry;

        use crate::proxy_box::RenderMouseRegion;
        use crate::shifted_box::RenderPadding;

        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        let render_view = binding.init_render_view(&mut app, view.clone());
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
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);

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
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::from_millis(16));
        assert_eq!(
            *log.borrow(),
            ["enter Offset(50.0, 50.0)", "exit Offset(50.0, 50.0)"],
            "the exit is placed with the transform recorded when the region was entered"
        );
    }

    #[test]
    fn deferred_first_frame_is_not_presented() {
        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        binding.init_render_view(&mut app, view.clone());
        binding.defer_first_frame(&mut app);
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);
        assert_eq!(view.presented(), 0);

        binding.allow_first_frame(&mut app);
        pump(&mut app, &[IMPLICIT_VIEW_ID], Duration::ZERO);
        assert_eq!(view.presented(), 1);
    }

    const FIRST: ViewId = ViewId(1);
    const SECOND: ViewId = ViewId(2);

    /// An app whose platform has views [`FIRST`] and [`SECOND`], each with a render view under a
    /// pipeline owner the root owner adopted, which marks its view to be composited after a
    /// paint that had work, as `RawView`'s does.
    fn app_with_render_views() -> (Rc<AppCell>, Rc<TestPlatform>, [RenderHandle<RenderView>; 2]) {
        let platform = Rc::new(
            TestPlatform::new()
                .on(TargetPlatform::MacOS)
                .with_views(&[FIRST, SECOND]),
        );
        let cell = AppCell::with_platform(platform.clone());
        let render_views = {
            let mut app = cell.borrow_mut();
            [FIRST, SECOND].map(|view| add_render_view(&mut app, platform.test_view(view)))
        };
        (cell, platform, render_views)
    }

    /// A render view of `view` under a pipeline owner of its own, registered with the binding.
    fn add_render_view(app: &mut App, view: Rc<TestView>) -> RenderHandle<RenderView> {
        let binding = RendererBinding::instance(app);
        let render_view = RenderView::new(app, None, None, view);
        let owner = PipelineOwner::new(
            app,
            None,
            Some(Rc::new(move |app: &mut App, is_dirty: bool| {
                if is_dirty {
                    render_view.mark_needs_composite_frame(app);
                }
            })),
        );
        owner.set_root_node(app, Some(render_view.as_object()));
        binding.root_pipeline_owner(app).adopt_child(app, owner);
        binding.add_render_view(app, render_view);
        render_view.prepare_initial_frame(app);
        render_view
    }

    /// How many frames each of [`FIRST`] and [`SECOND`] was presented.
    fn presented(platform: &TestPlatform) -> [u32; 2] {
        [FIRST, SECOND].map(|view| platform.test_view(view).presented())
    }

    // multi_view_binding_test.dart 'all registered renderviews are asked to composite frame',
    // with a frame of each view's own where Flutter's one frame is of every view.
    #[test]
    fn every_registered_render_view_is_composited_at_its_frame() {
        let (cell, platform, [first, second]) = app_with_render_views();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        assert_eq!(presented(&platform), [0, 0]);

        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 1]);

        binding.remove_render_view(&mut app, first);
        second.as_object().mark_needs_paint(&mut app);
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 2]);

        binding.remove_render_view(&mut app, second);
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 2]);
    }

    // multi_view_binding_test.dart 'render views that are not dirty are not composited'.
    #[test]
    fn a_render_view_that_is_not_dirty_is_not_composited() {
        let (cell, platform, [first, second]) = app_with_render_views();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);

        // First frame: both views are composited (they are newly added).
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 1]);

        // Second frame: neither view is dirty, so neither is composited.
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 1]);

        // Only the second is marked dirty: only the second is composited.
        second.as_object().mark_needs_paint(&mut app);
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 2]);

        binding.remove_render_view(&mut app, first);
        binding.remove_render_view(&mut app, second);
    }

    #[test]
    fn a_frame_paints_and_composites_only_its_own_views() {
        let (cell, platform, [first, second]) = app_with_render_views();
        let mut app = cell.borrow_mut();
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);

        first.as_object().mark_needs_paint(&mut app);
        second.as_object().mark_needs_paint(&mut app);
        pump(&mut app, &[SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [1, 2]);
        assert!(
            first.as_object().debug_needs_paint(&app),
            "left for its own frame"
        );
    }

    #[test]
    fn a_view_drawn_in_the_warm_up_frame_is_composited_again_at_its_next_frame() {
        let (cell, platform, _render_views) = app_with_render_views();
        SchedulerBinding::schedule_warm_up_frame(&mut cell.borrow_mut());
        cell.elapse(Duration::ZERO);
        assert_eq!(presented(&platform), [1, 1]);

        let mut app = cell.borrow_mut();
        pump(&mut app, &[FIRST], Duration::ZERO);
        assert_eq!(
            presented(&platform),
            [2, 1],
            "a warm-up frame may not reach the screen: the next real frame composites again"
        );
        pump(&mut app, &[FIRST], Duration::ZERO);
        assert_eq!(presented(&platform), [2, 1]);
    }

    #[test]
    fn marking_every_view_to_be_rendered_composites_each_at_its_next_frame() {
        let (cell, platform, _render_views) = app_with_render_views();
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);

        binding.handle_mark_all_views_need_render(&mut app);
        pump(&mut app, &[FIRST, SECOND], Duration::ZERO);
        assert_eq!(presented(&platform), [2, 2]);
    }
}
