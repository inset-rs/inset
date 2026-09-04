//! Flutter counterpart: `rendering/proxy_box.dart` (`RenderProxyBoxMixin`,
//! `HitTestBehavior`, `RenderConstrainedBox`, `RenderOpacity`,
//! `RenderAnimatedOpacityMixin`, `RenderAnimatedOpacity`, `RenderPointerListener`,
//! `DecorationPosition`, `RenderDecoratedBox`, `RenderRepaintBoundary`, `RenderMouseRegion`).
//!
//! Intrinsics wait.

use std::rc::Rc;

use reveal_animation::AnyAnimation;
use reveal_embedder::{Color, Offset, Size};
use reveal_foundation::{App, Handle, Listener};
use reveal_gestures::{
    PointerCancelEventListener, PointerDownEventListener, PointerEnterEventListener, PointerEvent,
    PointerExitEventListener, PointerHoverEventListener, PointerMoveEventListener,
    PointerPanZoomEndEventListener, PointerPanZoomStartEventListener,
    PointerPanZoomUpdateEventListener, PointerSignalEventListener, PointerUpEventListener,
};
use reveal_painting::{BoxPainter, ClipContext, Decoration, ImageConfiguration};
use reveal_services::{MouseCursor, MouseCursorRef, MouseTrackerAnnotation};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestEntry, BoxHitTestResult, RenderBox, RenderBoxData,
    RenderObjectWithChildMixin,
};
use crate::layer::CompositedLayer;
use crate::object::{
    AnyRenderObject, Constraints, EmptyParentData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// A base class for render boxes that resemble their children.
///
/// Flutter's `RenderProxyBoxMixin`: the shared bodies of a proxy box. A leaf implements the
/// marker and calls these where Dart would run the inherited method:
/// `RenderProxyBoxMixin::perform_layout(self, app)`.
pub trait RenderProxyBoxMixin: RenderObjectWithChildMixin {
    /// A proxy child gets plain parent data, not [`crate::BoxParentData`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if child.parent_data(app).is_none() {
            child.set_parent_data(app, EmptyParentData);
        }
    }

    /// Lays the child out with this box's constraints and takes its size.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = match self.child(app) {
            Some(child) => {
                child.layout(app, constraints, true);
                child.size(app)
            }
            None => self.compute_size_for_no_child(app, constraints),
        };
        self.set_size(app, size);
    }

    /// Calculate the size the proxy box would have when it has no child.
    fn compute_size_for_no_child(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.smallest()
    }

    /// Hit tests the child at this box's position.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        match self.child(app) {
            Some(child) => child.hit_test(app, result, position),
            None => false,
        }
    }

    /// Paints the child at this box's offset.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        context.paint_child(app, child.as_object(), offset);
    }
}

/// A [`RenderProxyBoxMixin`] that allows customizing its hit-test behavior.
///
/// Flutter's `RenderProxyBoxWithHitTestBehavior`. A leaf stores the [`HitTestBehavior`] and
/// wires `hit_test` / `hit_test_self` to these.
pub trait RenderProxyBoxWithHitTestBehavior: RenderProxyBoxMixin {
    /// How to behave during hit testing when deciding how the hit test propagates to children
    /// and whether to consider targets behind this one.
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior;

    /// Flutter's `hitTest`: a translucent box is added even when nothing was hit.
    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut hit_target = false;
        if self.size(app).contains(position) {
            hit_target = RenderBox::hit_test_children(self, app, result, position)
                || RenderBox::hit_test_self(self, app, position);
            if hit_target || self.behavior(app) == HitTestBehavior::Translucent {
                result.add(BoxHitTestEntry::new(self.as_box(), position).into());
            }
        }
        hit_target
    }

    /// Flutter's `hitTestSelf`: an opaque box is hit by itself.
    fn hit_test_self(self: RenderHandle<Self>, app: &App, _position: Offset) -> bool {
        self.behavior(app) == HitTestBehavior::Opaque
    }
}

/// How to behave during hit tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestBehavior {
    /// Targets that defer to their children receive events within their bounds
    /// only if one of their children is hit by the hit test.
    DeferToChild,

    /// Opaque targets can be hit by hit tests, causing them to both receive
    /// events within their bounds and prevent targets visually behind them from
    /// also receiving events.
    Opaque,

    /// Translucent targets both receive events within their bounds and permit
    /// targets visually behind them to also receive events.
    Translucent,
}

/// Imposes additional constraints on its child.
pub struct RenderConstrainedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    additional_constraints: BoxConstraints,
}

impl RenderConstrainedBox {
    /// Creates a render box that constrains its child.
    ///
    /// `additional_constraints` must be valid.
    pub fn new(
        app: &mut App,
        additional_constraints: BoxConstraints,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderConstrainedBox> {
        debug_assert!(additional_constraints.debug_assert_is_valid(false));
        let this = RenderHandle::new_box(
            app,
            RenderConstrainedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                additional_constraints,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Additional constraints to apply to the child during layout.
    pub fn additional_constraints(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        self.get(app).additional_constraints
    }

    /// Sets [`additional_constraints`](Self::additional_constraints).
    pub fn set_additional_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        value: BoxConstraints,
    ) {
        debug_assert!(value.debug_assert_is_valid(false));
        if self.get(app).additional_constraints == value {
            return;
        }
        self.get_mut(app).additional_constraints = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderConstrainedBox {
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

impl RenderProxyBoxMixin for RenderConstrainedBox {}

impl RenderObject for RenderConstrainedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let additional = self.get(app).additional_constraints;
        if let Some(child) = self.child(app) {
            child.layout(app, additional.enforce(constraints), true);
            self.set_size(app, child.size(app));
        } else {
            self.set_size(app, additional.enforce(constraints).constrain(Size::ZERO));
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderConstrainedBox {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// Makes its child partially transparent.
///
/// This class paints its child into an intermediate buffer and then blends the
/// child back into the scene partially transparent.
///
/// For values of opacity other than 0.0 and 1.0, this class is relatively
/// expensive because it requires painting the child into an intermediate
/// buffer. For the value 0.0, the child is not painted at all. For the
/// value 1.0, the child is painted immediately without an intermediate buffer.
pub struct RenderOpacity {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    alpha: i32,
    opacity: f64,
}

impl RenderOpacity {
    /// Creates a partially transparent render object.
    ///
    /// The `opacity` argument must be between 0.0 and 1.0, inclusive.
    pub fn new(app: &mut App, opacity: f64, child: Option<AnyRenderBox>) -> RenderHandle<Self> {
        debug_assert!((0.0..=1.0).contains(&opacity));
        let this = RenderHandle::new_box(
            app,
            RenderOpacity {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                alpha: Color::get_alpha_from_opacity(opacity),
                opacity,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The fraction to scale the child's alpha value.
    ///
    /// An opacity of 1.0 is fully opaque. An opacity of 0.0 is fully transparent
    /// (i.e., invisible).
    ///
    /// Values 1.0 and 0.0 are painted with a fast path. Other values
    /// require painting the child into an intermediate buffer, which is
    /// expensive.
    pub fn opacity(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).opacity
    }

    /// Sets [`opacity`](Self::opacity).
    pub fn set_opacity(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!((0.0..=1.0).contains(&value));
        if self.get(app).opacity == value {
            return;
        }
        let did_need_compositing = self.is_repaint_boundary(app);
        let this = self.get_mut(app);
        this.opacity = value;
        this.alpha = Color::get_alpha_from_opacity(value);
        // Flutter: `markNeedsCompositingBitsUpdate`, whose effect on a boundary change is a repaint.
        if did_need_compositing != self.is_repaint_boundary(app) {
            self.mark_needs_paint(app);
        }
        self.mark_needs_composited_layer_update(app);
    }
}

impl RenderObjectWithChildMixin for RenderOpacity {
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

impl RenderProxyBoxMixin for RenderOpacity {}

impl RenderObject for RenderOpacity {
    crate::render_object_accessors!();

    /// Flutter's `alwaysNeedsCompositing`: a visible child composites through its own layer.
    fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some() && self.get(app).alpha > 0
    }

    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<CompositedLayer>,
    ) -> CompositedLayer {
        let offset = old_layer.map_or(Offset::ZERO, |layer| layer.offset);
        CompositedLayer::opacity_layer(self.get(app).alpha, offset)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.child(app).is_none() || self.get(app).alpha == 0 {
            return;
        }
        RenderProxyBoxMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderOpacity {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// The mixin fields of [`RenderAnimatedOpacityMixin`].
pub struct RenderAnimatedOpacityData {
    alpha: Option<i32>,
    currently_is_repaint_boundary: Option<bool>,
    opacity: Option<AnyAnimation<f64>>,
}

impl RenderAnimatedOpacityData {
    /// No animation yet; the host sets one in its constructor.
    pub const fn new() -> RenderAnimatedOpacityData {
        RenderAnimatedOpacityData {
            alpha: None,
            currently_is_repaint_boundary: None,
            opacity: None,
        }
    }
}

impl Default for RenderAnimatedOpacityData {
    fn default() -> RenderAnimatedOpacityData {
        RenderAnimatedOpacityData::new()
    }
}

/// Implementation of [`RenderAnimatedOpacity`] and `RenderSliverAnimatedOpacity`.
///
/// This mixin allows the logic of animating opacity to be used with different
/// layout models, e.g. the way that [`RenderAnimatedOpacity`] uses it for
/// [`RenderBox`] and `RenderSliverAnimatedOpacity` uses it for `RenderSliver`.
///
/// The host wires the virtuals: `is_repaint_boundary`, `update_composited_layer`, `did_attach`,
/// `did_detach`, and `paint` call the same-named functions here.
pub trait RenderAnimatedOpacityMixin: RenderObjectWithChildMixin {
    /// Mixin field access.
    fn animated_opacity_data(self: RenderHandle<Self>, app: &App) -> &RenderAnimatedOpacityData;

    /// See [`animated_opacity_data`](Self::animated_opacity_data).
    fn animated_opacity_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAnimatedOpacityData;

    /// Flutter's `isRepaintBoundary`.
    fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some()
            && self
                .animated_opacity_data(app)
                .currently_is_repaint_boundary
                .expect("set by the opacity setter")
    }

    /// Flutter's `updateCompositedLayer`.
    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<CompositedLayer>,
    ) -> CompositedLayer {
        let alpha = self
            .animated_opacity_data(app)
            .alpha
            .expect("set by the opacity setter");
        let offset = old_layer.map_or(Offset::ZERO, |layer| layer.offset);
        CompositedLayer::opacity_layer(alpha, offset)
    }

    /// The animation that drives this render object's opacity.
    ///
    /// An opacity of 1.0 is fully opaque. An opacity of 0.0 is fully transparent
    /// (i.e., invisible).
    fn opacity(self: RenderHandle<Self>, app: &App) -> AnyAnimation<f64> {
        self.animated_opacity_data(app)
            .opacity
            .expect("set by the constructor")
    }

    /// Sets [`opacity`](Self::opacity).
    fn set_opacity(self: RenderHandle<Self>, app: &mut App, value: AnyAnimation<f64>) {
        if self.animated_opacity_data(app).opacity == Some(value) {
            return;
        }
        if self.attached(app)
            && let Some(opacity) = self.animated_opacity_data(app).opacity
        {
            opacity.remove_listener(app, &self.opacity_listener());
        }
        self.animated_opacity_data_mut(app).opacity = Some(value);
        if self.attached(app) {
            value.add_listener(app, self.opacity_listener());
        }
        self.update_opacity(app);
    }

    /// Flutter's `attach` after `super.attach(owner)`.
    fn did_attach(self: RenderHandle<Self>, app: &mut App) {
        self.opacity(app).add_listener(app, self.opacity_listener());
        self.update_opacity(app); // in case it changed while we weren't listening
    }

    /// Flutter's `detach` before `super.detach()`.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.opacity(app)
            .remove_listener(app, &self.opacity_listener());
    }

    /// The `_updateOpacity` tear-off, equal to itself across registrations.
    fn opacity_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), update_opacity::<Self>)
    }

    /// Flutter's `_updateOpacity`.
    fn update_opacity(self: RenderHandle<Self>, app: &mut App) {
        let old_alpha = self.animated_opacity_data(app).alpha;
        let alpha = Color::get_alpha_from_opacity(self.opacity(app).value(app));
        self.animated_opacity_data_mut(app).alpha = Some(alpha);
        if old_alpha != Some(alpha) {
            let was_repaint_boundary = self
                .animated_opacity_data(app)
                .currently_is_repaint_boundary;
            self.animated_opacity_data_mut(app)
                .currently_is_repaint_boundary = Some(alpha > 0);
            // Flutter: `markNeedsCompositingBitsUpdate`, whose effect on a boundary change is a repaint.
            if self.child(app).is_some() && was_repaint_boundary != Some(alpha > 0) {
                self.mark_needs_paint(app);
            }
            self.mark_needs_composited_layer_update(app);
        }
    }

    /// Flutter's `paint`: nothing when fully transparent, else the proxy paint.
    fn paint(self: RenderHandle<Self>, app: &mut App, context: &mut PaintingContext, offset: Offset)
    where
        Self: RenderProxyBoxMixin,
    {
        if self.animated_opacity_data(app).alpha == Some(0) {
            return;
        }
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }
}

fn update_opacity<T: RenderAnimatedOpacityMixin>(this: Handle<T>, app: &mut App) {
    RenderAnimatedOpacityMixin::update_opacity(RenderHandle::from_handle(this), app);
}

/// Makes its child partially transparent, driven from an [`AnyAnimation`].
///
/// This is a variant of [`RenderOpacity`] that uses an [`AnyAnimation<f64>`]
/// rather than a `f64` to control the opacity.
pub struct RenderAnimatedOpacity {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    animated_opacity: RenderAnimatedOpacityData,
}

impl RenderAnimatedOpacity {
    /// Creates a partially transparent render object.
    pub fn new(
        app: &mut App,
        opacity: AnyAnimation<f64>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderAnimatedOpacity {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                animated_opacity: RenderAnimatedOpacityData::new(),
            },
        );
        this.set_child(app, child);
        this.set_opacity(app, opacity);
        this
    }
}

impl RenderObjectWithChildMixin for RenderAnimatedOpacity {
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

impl RenderProxyBoxMixin for RenderAnimatedOpacity {}

impl RenderAnimatedOpacityMixin for RenderAnimatedOpacity {
    fn animated_opacity_data(self: RenderHandle<Self>, app: &App) -> &RenderAnimatedOpacityData {
        &self.get(app).animated_opacity
    }

    fn animated_opacity_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAnimatedOpacityData {
        &mut self.get_mut(app).animated_opacity
    }
}

impl RenderObject for RenderAnimatedOpacity {
    crate::render_object_accessors!();

    fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
        RenderAnimatedOpacityMixin::is_repaint_boundary(self, app)
    }

    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<CompositedLayer>,
    ) -> CompositedLayer {
        RenderAnimatedOpacityMixin::update_composited_layer(self, app, old_layer)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<crate::PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderAnimatedOpacityMixin::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderAnimatedOpacityMixin::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderAnimatedOpacityMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderAnimatedOpacity {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// Calls callbacks in response to common pointer events.
///
/// It responds to events that can indicate the location of a pointer, such as when the pointer
/// presses down, moves, releases, or is cancelled.
///
/// It does not respond to events that are exclusive to mouse, such as when the mouse enters and
/// exits a region without pressing any buttons. For these events, use `RenderMouseRegion`.
///
/// If it has a child, defers to the child for sizing behavior.
///
/// If it does not have a child, grows to fit the parent-provided constraints.
pub struct RenderPointerListener {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    on_pointer_down: Option<PointerDownEventListener>,
    on_pointer_move: Option<PointerMoveEventListener>,
    on_pointer_up: Option<PointerUpEventListener>,
    on_pointer_hover: Option<PointerHoverEventListener>,
    on_pointer_cancel: Option<PointerCancelEventListener>,
    on_pointer_pan_zoom_start: Option<PointerPanZoomStartEventListener>,
    on_pointer_pan_zoom_update: Option<PointerPanZoomUpdateEventListener>,
    on_pointer_pan_zoom_end: Option<PointerPanZoomEndEventListener>,
    on_pointer_signal: Option<PointerSignalEventListener>,
    behavior: HitTestBehavior,
}

macro_rules! pointer_listener_field {
    ($get:ident, $set:ident, $ty:ty, $doc:expr) => {
        #[doc = $doc]
        pub fn $get(self: RenderHandle<Self>, app: &App) -> Option<$ty> {
            self.get(app).$get.clone()
        }

        #[doc = concat!("Sets [`", stringify!($get), "`](Self::", stringify!($get), ").")]
        pub fn $set(self: RenderHandle<Self>, app: &mut App, value: Option<$ty>) {
            self.get_mut(app).$get = value;
        }
    };
}

impl RenderPointerListener {
    /// Creates a render object that forwards pointer events to callbacks.
    ///
    /// The callbacks are set afterwards; Flutter's constructor takes them as optional named
    /// arguments, all `null` by default.
    pub fn new(
        app: &mut App,
        behavior: HitTestBehavior,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderPointerListener {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                on_pointer_down: None,
                on_pointer_move: None,
                on_pointer_up: None,
                on_pointer_hover: None,
                on_pointer_cancel: None,
                on_pointer_pan_zoom_start: None,
                on_pointer_pan_zoom_update: None,
                on_pointer_pan_zoom_end: None,
                on_pointer_signal: None,
                behavior,
            },
        );
        this.set_child(app, child);
        this
    }

    pointer_listener_field!(
        on_pointer_down,
        set_on_pointer_down,
        PointerDownEventListener,
        "Called when a pointer comes into contact with the screen (for touch pointers), or has its button pressed (for mouse pointers) at this widget's location."
    );
    pointer_listener_field!(
        on_pointer_move,
        set_on_pointer_move,
        PointerMoveEventListener,
        "Called when a pointer that triggered an `on_pointer_down` changes position."
    );
    pointer_listener_field!(
        on_pointer_up,
        set_on_pointer_up,
        PointerUpEventListener,
        "Called when a pointer that triggered an `on_pointer_down` is no longer in contact with the screen."
    );
    pointer_listener_field!(
        on_pointer_hover,
        set_on_pointer_hover,
        PointerHoverEventListener,
        "Called when a pointer that has not an `on_pointer_down` changes position."
    );
    pointer_listener_field!(
        on_pointer_cancel,
        set_on_pointer_cancel,
        PointerCancelEventListener,
        "Called when the input from a pointer that triggered an `on_pointer_down` is no longer directed towards this receiver."
    );
    pointer_listener_field!(
        on_pointer_pan_zoom_start,
        set_on_pointer_pan_zoom_start,
        PointerPanZoomStartEventListener,
        "Called when a pan/zoom begins such as from a trackpad gesture."
    );
    pointer_listener_field!(
        on_pointer_pan_zoom_update,
        set_on_pointer_pan_zoom_update,
        PointerPanZoomUpdateEventListener,
        "Called when a pan/zoom is updated."
    );
    pointer_listener_field!(
        on_pointer_pan_zoom_end,
        set_on_pointer_pan_zoom_end,
        PointerPanZoomEndEventListener,
        "Called when a pan/zoom finishes."
    );
    pointer_listener_field!(
        on_pointer_signal,
        set_on_pointer_signal,
        PointerSignalEventListener,
        "Called when a pointer signal occurs over this object."
    );

    /// Sets [`behavior`](RenderProxyBoxWithHitTestBehavior::behavior).
    pub fn set_behavior(self: RenderHandle<Self>, app: &mut App, value: HitTestBehavior) {
        self.get_mut(app).behavior = value;
    }
}

impl RenderObjectWithChildMixin for RenderPointerListener {
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

impl RenderProxyBoxMixin for RenderPointerListener {
    fn compute_size_for_no_child(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }
}

impl RenderProxyBoxWithHitTestBehavior for RenderPointerListener {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.get(app).behavior
    }
}

impl RenderObject for RenderPointerListener {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderPointerListener {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        _entry: &BoxHitTestEntry,
    ) {
        match event {
            PointerEvent::Down(event) => {
                if let Some(callback) = self.get(app).on_pointer_down.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Move(event) => {
                if let Some(callback) = self.get(app).on_pointer_move.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Up(event) => {
                if let Some(callback) = self.get(app).on_pointer_up.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Hover(event) => {
                if let Some(callback) = self.get(app).on_pointer_hover.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Cancel(event) => {
                if let Some(callback) = self.get(app).on_pointer_cancel.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::PanZoomStart(event) => {
                if let Some(callback) = self.get(app).on_pointer_pan_zoom_start.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::PanZoomUpdate(event) => {
                if let Some(callback) = self.get(app).on_pointer_pan_zoom_update.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::PanZoomEnd(event) => {
                if let Some(callback) = self.get(app).on_pointer_pan_zoom_end.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Scroll(_)
            | PointerEvent::ScrollInertiaCancel(_)
            | PointerEvent::Scale(_) => {
                if let Some(callback) = self.get(app).on_pointer_signal.clone() {
                    callback(app, event.clone());
                }
            }
            PointerEvent::Added(_)
            | PointerEvent::Removed(_)
            | PointerEvent::Enter(_)
            | PointerEvent::Exit(_) => {}
        }
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test(self, app, result, position)
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// Where to paint a box decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationPosition {
    /// Paint the box decoration behind the children.
    Background,

    /// Paint the box decoration in front of the children.
    Foreground,
}

/// Paints a [`Decoration`] either before or after its child paints.
pub struct RenderDecoratedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    painter: Option<Box<dyn BoxPainter>>,
    decoration: Box<dyn Decoration>,
    position: DecorationPosition,
    configuration: ImageConfiguration,
}

impl RenderDecoratedBox {
    /// Creates a decorated box.
    ///
    /// The `configuration` is normally obtained from `createLocalImageConfiguration`.
    pub fn new(
        app: &mut App,
        decoration: Box<dyn Decoration>,
        position: DecorationPosition,
        configuration: ImageConfiguration,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderDecoratedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                painter: None,
                decoration,
                position,
                configuration,
            },
        );
        this.set_child(app, child);
        this
    }

    /// What decoration to paint.
    pub fn decoration(self: RenderHandle<Self>, app: &App) -> &dyn Decoration {
        &*self.get(app).decoration
    }

    /// Sets [`decoration`](Self::decoration).
    pub fn set_decoration(self: RenderHandle<Self>, app: &mut App, value: Box<dyn Decoration>) {
        if value.eq_decoration(&*self.get(app).decoration) {
            return;
        }
        let this = self.get_mut(app);
        if let Some(mut painter) = this.painter.take() {
            painter.dispose();
        }
        this.decoration = value;
        self.mark_needs_paint(app);
    }

    /// Whether to paint the box decoration behind or in front of the child.
    pub fn position(self: RenderHandle<Self>, app: &App) -> DecorationPosition {
        self.get(app).position
    }

    /// Sets [`position`](Self::position).
    pub fn set_position(self: RenderHandle<Self>, app: &mut App, value: DecorationPosition) {
        if self.get(app).position == value {
            return;
        }
        self.get_mut(app).position = value;
        self.mark_needs_paint(app);
    }

    /// The settings to pass to the decoration when painting, so that it can
    /// resolve images appropriately. See [`ImageConfiguration`].
    pub fn configuration(self: RenderHandle<Self>, app: &App) -> &ImageConfiguration {
        &self.get(app).configuration
    }

    /// Sets [`configuration`](Self::configuration).
    pub fn set_configuration(self: RenderHandle<Self>, app: &mut App, value: ImageConfiguration) {
        if self.get(app).configuration == value {
            return;
        }
        self.get_mut(app).configuration = value;
        self.mark_needs_paint(app);
    }

    fn paint_decoration(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        configuration: &ImageConfiguration,
    ) {
        let debug_save_count = context.canvas().save_count();
        let is_complex = {
            let this = self.get_mut(app);
            let painter = this
                .painter
                .get_or_insert_with(|| this.decoration.create_box_painter(None));
            painter.paint(context.canvas(), offset, configuration);
            this.decoration.is_complex()
        };
        debug_assert_eq!(
            debug_save_count,
            context.canvas().save_count(),
            "the decoration's painter had mismatching save and restore calls"
        );
        if is_complex {
            context.set_is_complex_hint();
        }
    }
}

impl RenderObjectWithChildMixin for RenderDecoratedBox {
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

impl RenderProxyBoxMixin for RenderDecoratedBox {}

impl RenderObject for RenderDecoratedBox {
    crate::render_object_accessors!();

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(mut painter) = self.get_mut(app).painter.take() {
            painter.dispose();
        }
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
        self.mark_needs_paint(app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let size = self.size(app);
        let filled_configuration =
            self.get(app)
                .configuration
                .copy_with(None, None, None, Some(size), None);
        if self.get(app).position == DecorationPosition::Background {
            self.paint_decoration(app, context, offset, &filled_configuration);
        }
        RenderProxyBoxMixin::paint(self, app, context, offset);
        if self.get(app).position == DecorationPosition::Foreground {
            self.paint_decoration(app, context, offset, &filled_configuration);
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
}

impl RenderBox for RenderDecoratedBox {
    crate::render_box_accessors!();

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        let this = self.get(app);
        this.decoration
            .hit_test(self.size(app), position, this.configuration.text_direction)
    }

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// Creates a separate display list for its child.
///
/// This render object creates a separate display list for its child, which
/// can improve performance if the subtree repaints at different times than
/// the surrounding parts of the tree.
pub struct RenderRepaintBoundary {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
}

impl RenderRepaintBoundary {
    /// Creates a repaint boundary around `child`.
    pub fn new(app: &mut App, child: Option<AnyRenderBox>) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderRepaintBoundary {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
            },
        );
        this.set_child(app, child);
        this
    }
}

impl RenderObjectWithChildMixin for RenderRepaintBoundary {
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

impl RenderProxyBoxMixin for RenderRepaintBoundary {}

impl RenderObject for RenderRepaintBoundary {
    crate::render_object_accessors!();

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderRepaintBoundary {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

/// Calls callbacks in response to pointer events that are exclusive to mice.
///
/// It responds to events that are related to hovering, i.e. when the mouse
/// enters, exits (with or without pressing buttons), or moves over a region
/// without pressing buttons.
///
/// It does not respond to common events that construct gestures, such as when
/// the pointer is pressed, moved, then released or canceled. For these events,
/// use [`RenderPointerListener`].
///
/// If it has a child, it defers to the child for sizing behavior.
///
/// If it does not have a child, it grows to fit the parent-provided constraints.
///
/// Flutter's `RenderMouseRegion implements MouseTrackerAnnotation`: here it answers
/// [`RenderObject::mouse_tracker_annotation`] with its current callbacks, cursor, and validity.
///
/// See also:
///
///  * `MouseRegion`, a widget that listens to hover events using
///    [`RenderMouseRegion`].
pub struct RenderMouseRegion {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    on_enter: Option<PointerEnterEventListener>,
    on_hover: Option<PointerHoverEventListener>,
    on_exit: Option<PointerExitEventListener>,
    cursor: MouseCursorRef,
    valid_for_mouse_tracker: bool,
    opaque: bool,
    behavior: HitTestBehavior,
}

impl RenderMouseRegion {
    /// Creates a render object that forwards pointer events to callbacks.
    ///
    /// Dart's remaining constructor arguments have setters: the callbacks, `cursor`
    /// (`MouseCursor.defer`), `opaque` (true), and `hit_test_behavior` (opaque).
    /// `valid_for_mouse_tracker` has none, so it is a parameter (Dart's default is true).
    pub fn new(
        app: &mut App,
        valid_for_mouse_tracker: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderMouseRegion {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                on_enter: None,
                on_hover: None,
                on_exit: None,
                cursor: <dyn MouseCursor>::defer(),
                valid_for_mouse_tracker,
                opaque: true,
                behavior: HitTestBehavior::Opaque,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether this object should prevent [`RenderMouseRegion`]s visually behind it
    /// from detecting the pointer, thus affecting how their `on_hover`, `on_enter`,
    /// and `on_exit` behave.
    ///
    /// If `opaque` is true, this object will absorb the mouse pointer and
    /// prevent this object's siblings (or any other objects that are not
    /// ancestors or descendants of this object) from detecting the mouse
    /// pointer even when the pointer is within their areas.
    ///
    /// If `opaque` is false, this object will not affect how [`RenderMouseRegion`]s
    /// behind it behave, which will detect the mouse pointer as long as the
    /// pointer is within their areas.
    ///
    /// This defaults to true.
    pub fn opaque(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).opaque
    }

    /// Sets [`opaque`](Self::opaque).
    pub fn set_opaque(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).opaque != value {
            self.get_mut(app).opaque = value;
            // Trigger [MouseTracker]'s device update to recalculate mouse states.
            self.mark_needs_paint(app);
        }
    }

    /// How to behave during hit testing.
    ///
    /// This defaults to [`HitTestBehavior::Opaque`] if `None`.
    pub fn hit_test_behavior(self: RenderHandle<Self>, app: &App) -> Option<HitTestBehavior> {
        Some(self.get(app).behavior)
    }

    /// Sets [`hit_test_behavior`](Self::hit_test_behavior).
    pub fn set_hit_test_behavior(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<HitTestBehavior>,
    ) {
        let new_value = value.unwrap_or(HitTestBehavior::Opaque);
        if self.get(app).behavior != new_value {
            self.get_mut(app).behavior = new_value;
            // Trigger [MouseTracker]'s device update to recalculate mouse states.
            self.mark_needs_paint(app);
        }
    }

    pointer_listener_field!(
        on_enter,
        set_on_enter,
        PointerEnterEventListener,
        "Triggered when a mouse pointer, with or without buttons pressed, has entered the region and `valid_for_mouse_tracker` is true."
    );
    pointer_listener_field!(
        on_hover,
        set_on_hover,
        PointerHoverEventListener,
        "Triggered when a pointer has moved onto or within the region without buttons pressed."
    );
    pointer_listener_field!(
        on_exit,
        set_on_exit,
        PointerExitEventListener,
        "Triggered when a mouse pointer, with or without buttons pressed, has exited the region and `valid_for_mouse_tracker` is true."
    );

    /// The mouse cursor for mouse pointers that are hovering over the region.
    ///
    /// When a mouse enters the region, its cursor will be changed to the `cursor`.
    /// When the mouse leaves the region, the cursor will be set by the region
    /// found at the new location.
    ///
    /// Defaults to `MouseCursor.defer`, deferring the choice of cursor to the next
    /// region behind it in hit-test order.
    pub fn cursor(self: RenderHandle<Self>, app: &App) -> MouseCursorRef {
        Rc::clone(&self.get(app).cursor)
    }

    /// Sets [`cursor`](Self::cursor).
    pub fn set_cursor(self: RenderHandle<Self>, app: &mut App, value: MouseCursorRef) {
        if *self.get(app).cursor != *value {
            self.get_mut(app).cursor = value;
            // A repaint is needed in order to trigger a device update of
            // [MouseTracker] so that this new value can be found.
            self.mark_needs_paint(app);
        }
    }

    /// Whether this is included when a `MouseTracker` collects the list of
    /// annotations: true while attached to a pipeline owner.
    pub fn valid_for_mouse_tracker(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).valid_for_mouse_tracker
    }
}

impl RenderObjectWithChildMixin for RenderMouseRegion {
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

impl RenderProxyBoxMixin for RenderMouseRegion {
    fn compute_size_for_no_child(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }
}

impl RenderProxyBoxWithHitTestBehavior for RenderMouseRegion {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.get(app).behavior
    }
}

impl RenderObject for RenderMouseRegion {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        for child in [self.child(app)].into_iter().flatten() {
            child.as_object().attach(app, owner);
        }
        self.get_mut(app).valid_for_mouse_tracker = true;
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        // Dart clears the flag before `super.detach()`; nothing in the base body reads it.
        self.get_mut(app).valid_for_mouse_tracker = false;
        for child in [self.child(app)].into_iter().flatten() {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn mouse_tracker_annotation(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<MouseTrackerAnnotation> {
        let this = self.get(app);
        Some(MouseTrackerAnnotation {
            on_enter: this.on_enter.clone(),
            on_exit: this.on_exit.clone(),
            cursor: Rc::clone(&this.cursor),
            valid_for_mouse_tracker: this.valid_for_mouse_tracker,
        })
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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
}

impl RenderBox for RenderMouseRegion {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test(self, app, result, position)
            && self.get(app).opaque
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        _entry: &BoxHitTestEntry,
    ) {
        if let PointerEvent::Hover(event) = event
            && let Some(on_hover) = self.get(app).on_hover.clone()
        {
            on_hover(app, event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use reveal_animation::{k_always_complete_animation, k_always_dismissed_animation};
    use reveal_painting::BoxDecoration;

    use super::*;
    use crate::pipeline_owner::PipelineOwner;

    fn sized_box(app: &mut App, size: Size) -> RenderHandle<RenderConstrainedBox> {
        RenderConstrainedBox::new(app, BoxConstraints::tight(size), None)
    }

    /// The test binding's first frame: attach, lay the root out, schedule and flush paint.
    fn first_frame(app: &mut App, root: AnyRenderBox) -> Handle<PipelineOwner> {
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.layout(app, BoxConstraints::tight(Size::new(100.0, 100.0)), false);
        root.as_object()
            .schedule_initial_paint(app, CompositedLayer::default());
        owner.flush_paint(app);
        owner
    }

    fn pump_frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_paint(app);
    }

    #[test]
    fn constrained_box_without_child() {
        let mut app = App::new();
        let box_ =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(box_.size(&app), Size::new(80.0, 40.0));
    }

    #[test]
    fn additional_constraints_change_marks_layout() {
        let mut app = App::new();
        let box_ =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert!(!box_.debug_needs_layout(&app));
        box_.set_additional_constraints(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)));
        assert!(box_.debug_needs_layout(&app));
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(box_.size(&app), Size::new(20.0, 20.0));
    }

    /// Flutter's `RenderProxyBoxMixin.setupParentData` gives the child plain `ParentData`.
    #[test]
    fn proxy_box_child_gets_empty_parent_data() {
        let mut app = App::new();
        let child = sized_box(&mut app, Size::new(10.0, 10.0));
        let parent = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(10.0, 10.0)),
            Some(child.as_box()),
        );
        parent.layout(&mut app, BoxConstraints::new(), false);
        assert!(child.as_box().parent_data_is::<EmptyParentData>(&app));
    }

    /// `proxy_box_test.dart`: `RenderOpacity does not composite if it is transparent` and the
    /// opaque / partially opaque cases. Ours: whether it is a repaint boundary.
    #[test]
    fn render_opacity_composites_only_when_visible() {
        let mut app = App::new();
        for (opacity, composites) in [(0.0, false), (1.0, true), (0.1, true)] {
            let child = sized_box(&mut app, Size::new(1.0, 1.0));
            let opacity = RenderOpacity::new(&mut app, opacity, Some(child.as_box()));
            assert_eq!(opacity.as_object().is_repaint_boundary(&app), composites);
        }
    }

    /// `layers_test.dart`: `non-painted layers are detached`.
    #[test]
    fn non_painted_layers_are_detached() {
        let mut app = App::new();
        let inner = RenderDecoratedBox::new(
            &mut app,
            Box::new(BoxDecoration::new()),
            DecorationPosition::Background,
            ImageConfiguration::EMPTY,
            None,
        );
        let boundary = RenderRepaintBoundary::new(&mut app, Some(inner.as_box()));
        let opacity = RenderOpacity::new(&mut app, 1.0, Some(boundary.as_box()));
        // Flutter's test binding puts a `RenderView` above; a root must stay a repaint boundary.
        let root = RenderRepaintBoundary::new(&mut app, Some(opacity.as_box()));
        let owner = first_frame(&mut app, root.as_box());
        assert!(!inner.as_object().is_repaint_boundary(&app));
        assert!(inner.as_object().debug_layer(&app).is_none());
        assert!(boundary.as_object().is_repaint_boundary(&app));
        let layer = boundary.as_object().debug_layer(&app).expect("painted");
        assert!(layer.attached(), "this time it painted");

        opacity.set_opacity(&mut app, 0.0);
        pump_frame(&mut app, owner);
        assert!(inner.as_object().debug_layer(&app).is_none());
        let layer = boundary.as_object().debug_layer(&app).expect("kept");
        assert!(!layer.attached(), "this time it did not");

        opacity.set_opacity(&mut app, 0.5);
        pump_frame(&mut app, owner);
        let layer = boundary.as_object().debug_layer(&app).expect("kept");
        assert!(layer.attached(), "this time it did again");
    }

    #[test]
    fn animated_opacity_follows_its_animation() {
        let mut app = App::new();
        let child = sized_box(&mut app, Size::new(1.0, 1.0));
        let dismissed = k_always_dismissed_animation(&mut app);
        let complete = k_always_complete_animation(&mut app);
        let fade = RenderAnimatedOpacity::new(&mut app, dismissed, Some(child.as_box()));
        assert!(!fade.as_object().is_repaint_boundary(&app));
        let root = RenderRepaintBoundary::new(&mut app, Some(fade.as_box()));
        let owner = first_frame(&mut app, root.as_box());

        fade.set_opacity(&mut app, complete);
        assert!(fade.as_object().is_repaint_boundary(&app));
        assert!(fade.as_object().debug_needs_paint(&app));
        pump_frame(&mut app, owner);
        let layer = fade.as_object().debug_layer(&app).expect("a boundary now");
        assert_eq!(
            layer.composited().kind,
            crate::layer::CompositedLayerKind::Opacity { alpha: 255 }
        );
    }

    #[test]
    fn decorated_box_paints_its_decoration() {
        let mut app = App::new();
        let decorated = RenderDecoratedBox::new(
            &mut app,
            Box::new(BoxDecoration::new().color(Color::from_argb(255, 255, 0, 0))),
            DecorationPosition::Background,
            ImageConfiguration::EMPTY,
            None,
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(decorated.as_box()));
        first_frame(&mut app, root.as_box());
        let mut canvas = reveal_embedder::Canvas::new();
        root.as_object()
            .debug_layer(&app)
            .expect("root layer")
            .add_to_scene(&app, &mut canvas);
        let ops = canvas.build().ops().to_vec();
        assert!(
            ops.iter()
                .any(|op| matches!(op, reveal_embedder::valo::Op::DrawDisplayList { .. })),
            "{ops:?}"
        );
    }
}
