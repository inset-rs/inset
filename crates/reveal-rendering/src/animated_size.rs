//! Flutter counterpart: `rendering/animated_size.dart`.

use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatus,
    AnimationStatusListener, Curve, CurvedAnimation, SizeTween,
};
use reveal_embedder::{Clip, Offset, Size, TextBaseline, TextDirection};
use reveal_foundation::{App, Handle, Listener};
use reveal_painting::AlignmentGeometry;

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult, RenderBox, RenderBoxData};
use crate::layer::{ClipRectLayer, LayerHandle};
use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectBase, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::shifted_box::{
    RenderAligningShiftedBox, RenderAligningShiftedBoxData, RenderShiftedBox,
};
use crate::sliver_persistent_header::TickerProviderRef;

/// A [`RenderAnimatedSize`] can be in exactly one of these states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderAnimatedSizeState {
    /// The initial state, when we do not yet know what the starting and target
    /// sizes are to animate.
    ///
    /// The next state is [`Stable`](Self::Stable).
    Start,
    /// At this state the child's size is assumed to be stable and we are either
    /// animating, or waiting for the child's size to change.
    ///
    /// If the child's size changes, the state will become [`Changed`](Self::Changed).
    /// Otherwise, it remains [`Stable`](Self::Stable).
    Stable,
    /// At this state we know that the child has changed once after being assumed
    /// [`Stable`](Self::Stable).
    Changed,
    /// At this state the child's size is assumed to be unstable (changing each
    /// frame).
    Unstable,
}

/// A render object that animates its size to its child's size over a given
/// `duration` and with a given `curve`.
pub struct RenderAnimatedSize {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    vsync: TickerProviderRef,
    clip_behavior: Clip,
    controller: Handle<AnimationController>,
    animation: Handle<CurvedAnimation>,
    size_tween: Handle<SizeTween>,
    has_visual_overflow: bool,
    last_value: Option<f64>,
    state: RenderAnimatedSizeState,
    current_size: Size,
    on_end: Option<Listener>,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
}

impl RenderAnimatedSize {
    /// Creates a render object that animates its size to match its child.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        vsync: TickerProviderRef,
        duration: Duration,
        reverse_duration: Option<Duration>,
        curve: Rc<dyn Curve>,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        clip_behavior: Clip,
        on_end: Option<Listener>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let controller = AnimationController::create(
            app,
            None,
            Some(duration),
            reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync.clone(),
        );
        let animation = CurvedAnimation::create(app, controller.view(), curve, None);
        let size_tween = SizeTween::new(app, None, None);
        let this = RenderHandle::new_box(
            app,
            RenderAnimatedSize {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                vsync,
                clip_behavior,
                controller,
                animation,
                size_tween,
                has_visual_overflow: false,
                last_value: None,
                state: RenderAnimatedSizeState::Start,
                current_size: Size::ZERO,
                on_end,
                clip_rect_layer: LayerHandle::new(),
            },
        );
        controller.add_listener(
            app,
            Listener::handle_method(this.handle(), on_controller_tick),
        );
        this.set_child(app, child);
        this
    }

    /// The duration of the animation.
    pub fn duration(self: RenderHandle<Self>, app: &App) -> Duration {
        app.get(self.get(app).controller)
            .duration
            .expect("duration")
    }

    /// Sets [`duration`](Self::duration).
    pub fn set_duration(self: RenderHandle<Self>, app: &mut App, value: Duration) {
        let controller = self.get(app).controller;
        if app.get(controller).duration == Some(value) {
            return;
        }
        app.get_mut(controller).duration = Some(value);
    }

    /// The duration of the animation when running in reverse.
    pub fn reverse_duration(self: RenderHandle<Self>, app: &App) -> Option<Duration> {
        app.get(self.get(app).controller).reverse_duration
    }

    /// Sets [`reverse_duration`](Self::reverse_duration).
    pub fn set_reverse_duration(self: RenderHandle<Self>, app: &mut App, value: Option<Duration>) {
        let controller = self.get(app).controller;
        if app.get(controller).reverse_duration == value {
            return;
        }
        app.get_mut(controller).reverse_duration = value;
    }

    /// The curve of the animation.
    pub fn curve(self: RenderHandle<Self>, app: &App) -> Rc<dyn Curve> {
        app.get(self.get(app).animation).curve.clone()
    }

    /// Sets [`curve`](Self::curve).
    pub fn set_curve(self: RenderHandle<Self>, app: &mut App, value: Rc<dyn Curve>) {
        let animation = self.get(app).animation;
        if Rc::ptr_eq(&app.get(animation).curve, &value) {
            return;
        }
        app.get_mut(animation).curve = value;
    }

    /// {@macro flutter.material.Material.clipBehavior}
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if self.get(app).clip_behavior == value {
            return;
        }
        self.get_mut(app).clip_behavior = value;
        RenderBox::mark_needs_paint(self, app);
    }

    /// Whether the size is being currently animated towards the child's size.
    pub fn is_animating(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).controller.as_animation().is_animating(app)
    }

    /// The [`TickerProvider`] for the [`AnimationController`] that runs the animation.
    pub fn vsync(self: RenderHandle<Self>, app: &App) -> TickerProviderRef {
        self.get(app).vsync.clone()
    }

    /// Sets [`vsync`](Self::vsync).
    pub fn set_vsync(self: RenderHandle<Self>, app: &mut App, value: TickerProviderRef) {
        if self.get(app).vsync == value {
            return;
        }
        self.get_mut(app).vsync = value.clone();
        let controller = self.get(app).controller;
        controller.resync(app, value);
    }

    /// Called every time an animation completes.
    pub fn on_end(self: RenderHandle<Self>, app: &App) -> Option<Listener> {
        self.get(app).on_end.clone()
    }

    /// Sets [`on_end`](Self::on_end).
    pub fn set_on_end(self: RenderHandle<Self>, app: &mut App, value: Option<Listener>) {
        self.get_mut(app).on_end = value;
    }

    /// The state this size animation is in.
    pub fn state(self: RenderHandle<Self>, app: &App) -> RenderAnimatedSizeState {
        self.get(app).state
    }

    fn animated_size(self: RenderHandle<Self>, app: &App) -> Option<Size> {
        let size_tween = self.get(app).size_tween;
        let animation = self.get(app).animation;
        Animatable::evaluate(&size_tween, app, animation.as_animation())
    }

    fn restart_animation(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).last_value = Some(0.0);
        let controller = self.get(app).controller;
        controller.forward(app, Some(0.0));
    }

    fn layout_start(self: RenderHandle<Self>, app: &mut App) {
        let child_size = self.child(app).expect("child").size(app);
        let size_tween = self.get(app).size_tween;
        app.get_mut(size_tween).begin = Some(child_size);
        app.get_mut(size_tween).end = Some(child_size);
        self.get_mut(app).state = RenderAnimatedSizeState::Stable;
    }

    fn layout_stable(self: RenderHandle<Self>, app: &mut App) {
        let child_size = self.child(app).expect("child").size(app);
        let size_tween = self.get(app).size_tween;
        let end = app.get(size_tween).end;
        if end != Some(child_size) {
            let size = self.size(app);
            app.get_mut(size_tween).begin = Some(size);
            app.get_mut(size_tween).end = Some(child_size);
            self.restart_animation(app);
            self.get_mut(app).state = RenderAnimatedSizeState::Changed;
        } else {
            let controller = self.get(app).controller;
            let value = controller.value(app);
            let upper = app.get(controller).upper_bound;
            if value == upper {
                app.get_mut(size_tween).begin = Some(child_size);
                app.get_mut(size_tween).end = Some(child_size);
            } else if !controller.is_animating(app) {
                controller.forward(app, None);
            }
        }
    }

    fn layout_changed(self: RenderHandle<Self>, app: &mut App) {
        let child_size = self.child(app).expect("child").size(app);
        let size_tween = self.get(app).size_tween;
        if app.get(size_tween).end != Some(child_size) {
            app.get_mut(size_tween).begin = Some(child_size);
            app.get_mut(size_tween).end = Some(child_size);
            self.restart_animation(app);
            self.get_mut(app).state = RenderAnimatedSizeState::Unstable;
        } else {
            self.get_mut(app).state = RenderAnimatedSizeState::Stable;
            let controller = self.get(app).controller;
            if !controller.is_animating(app) {
                controller.forward(app, None);
            }
        }
    }

    fn layout_unstable(self: RenderHandle<Self>, app: &mut App) {
        let child_size = self.child(app).expect("child").size(app);
        let size_tween = self.get(app).size_tween;
        if app.get(size_tween).end != Some(child_size) {
            app.get_mut(size_tween).begin = Some(child_size);
            app.get_mut(size_tween).end = Some(child_size);
            self.restart_animation(app);
        } else {
            let controller = self.get(app).controller;
            controller.stop(app, true);
            self.get_mut(app).state = RenderAnimatedSizeState::Stable;
        }
    }
}

impl RenderObjectWithChildMixin for RenderAnimatedSize {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderAnimatedSize {}

impl RenderAligningShiftedBox for RenderAnimatedSize {
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData {
        &self.get(app).aligning
    }

    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData {
        &mut self.get_mut(app).aligning
    }
}

impl RenderObject for RenderAnimatedSize {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        match self.get(app).state {
            RenderAnimatedSizeState::Start | RenderAnimatedSizeState::Stable => {}
            RenderAnimatedSizeState::Changed | RenderAnimatedSizeState::Unstable => {
                RenderBox::mark_needs_layout(self, app);
            }
        }
        let controller = self.get(app).controller;
        controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self.handle(), animation_status_listener),
        );
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let controller = self.get(app).controller;
        controller.stop(app, true);
        controller.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self.handle(), animation_status_listener),
        );
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let controller = self.get(app).controller;
        let value = controller.value(app);
        self.get_mut(app).last_value = Some(value);
        self.get_mut(app).has_visual_overflow = false;
        let constraints = self.constraints(app);
        if self.child(app).is_none() || constraints.is_tight() {
            controller.stop(app, true);
            let smallest = constraints.smallest();
            let size_tween = self.get(app).size_tween;
            app.get_mut(size_tween).begin = Some(smallest);
            app.get_mut(size_tween).end = Some(smallest);
            self.set_size(app, smallest);
            self.get_mut(app).current_size = smallest;
            self.get_mut(app).state = RenderAnimatedSizeState::Start;
            if let Some(child) = self.child(app) {
                child.layout(app, constraints, false);
            }
            return;
        }

        let child = self.child(app).expect("child");
        child.layout(app, constraints, true);

        match self.get(app).state {
            RenderAnimatedSizeState::Start => self.layout_start(app),
            RenderAnimatedSizeState::Stable => self.layout_stable(app),
            RenderAnimatedSizeState::Changed => self.layout_changed(app),
            RenderAnimatedSizeState::Unstable => self.layout_unstable(app),
        }

        let animated = self.animated_size(app).expect("animated size");
        let size = constraints.constrain(animated);
        self.set_size(app, size);
        self.get_mut(app).current_size = size;
        self.align_child(app);

        let end = app.get(self.get(app).size_tween).end.expect("end");
        if size.width() < end.width() || size.height() < end.height() {
            self.get_mut(app).has_visual_overflow = true;
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.child(app).is_some()
            && self.get(app).has_visual_overflow
            && self.get(app).clip_behavior != Clip::None
        {
            let clip_rect = Offset::ZERO & self.size(app);
            let clip_behavior = self.get(app).clip_behavior;
            let old = self.get(app).clip_rect_layer.layer();
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                clip_rect,
                |app, context, offset| RenderShiftedBox::paint(self, app, context, offset),
                clip_behavior,
                old,
            );
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, layer);
        } else {
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
            RenderShiftedBox::paint(self, app, context, offset);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
        let controller = self.get(app).controller;
        let animation = self.get(app).animation;
        controller.dispose(app);
        animation.dispose(app);
        RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderAnimatedSize {
    crate::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let Some(child) = self.child(app) else {
            return constraints.smallest();
        };
        if constraints.is_tight() {
            return constraints.smallest();
        }
        let child_size = child.get_dry_layout(app, constraints);
        match self.get(app).state {
            RenderAnimatedSizeState::Start => return constraints.constrain(child_size),
            RenderAnimatedSizeState::Stable => {
                if app.get(self.get(app).size_tween).end != Some(child_size) {
                    return constraints.constrain(self.get(app).current_size);
                }
                let controller = self.get(app).controller;
                if controller.value(app) == app.get(controller).upper_bound {
                    return constraints.constrain(child_size);
                }
            }
            RenderAnimatedSizeState::Unstable | RenderAnimatedSizeState::Changed => {
                if app.get(self.get(app).size_tween).end != Some(child_size) {
                    return constraints.constrain(child_size);
                }
            }
        }
        constraints.constrain(self.animated_size(app).expect("animated size"))
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, constraints, baseline)?;
        let child_size = child.get_dry_layout(app, constraints);
        let my_size = self.get_dry_layout(app, constraints);
        let offset = self
            .resolved_alignment(app)
            .along_offset(my_size - child_size);
        Some(result + offset.dy())
    }
}

impl std::fmt::Debug for RenderAnimatedSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderAnimatedSize").finish_non_exhaustive()
    }
}

fn on_controller_tick(this: Handle<RenderAnimatedSize>, app: &mut App) {
    let this = RenderHandle::from_handle(this);
    let value = this.get(app).controller.value(app);
    if this.get(app).last_value != Some(value) {
        RenderBox::mark_needs_layout(this, app);
    }
}

fn animation_status_listener(
    this: Handle<RenderAnimatedSize>,
    app: &mut App,
    status: AnimationStatus,
) {
    let this = RenderHandle::from_handle(this);
    if status.is_completed()
        && let Some(on_end) = this.get(app).on_end.clone()
    {
        on_end.call(app);
    }
}
