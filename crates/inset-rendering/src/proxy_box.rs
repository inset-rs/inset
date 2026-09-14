//! Flutter counterpart: `rendering/proxy_box.dart` (`RenderProxyBoxMixin`,
//! `HitTestBehavior`, `RenderConstrainedBox`, `RenderLimitedBox`, `RenderOpacity`,
//! `RenderAnimatedOpacityMixin`, `RenderAnimatedOpacity`, `CustomClipper`,
//! `RenderCustomClip`, `RenderClipRect`, `RenderClipRRect`, `RenderPointerListener`,
//! `DecorationPosition`, `RenderDecoratedBox`, `RenderTransform`,
//! `RenderFractionalTranslation`, `RenderRepaintBoundary`, `RenderMouseRegion`,
//! `RenderIgnorePointer`, `RenderOffstage`, `RenderAbsorbPointer`, `RenderAnnotatedRegion`),
//! plus
//! `widgets/basic.dart`'s `_RenderColoredBox`.
//!
//! Intrinsics wait.

use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

use inset_animation::AnyAnimation;
use inset_embedder::{
    BlendMode, Clip, Color, FillRule, ImageFilter, Matrix4, Offset, Paint, Path, PathBuilder,
    RRect, RSuperellipse, Rect, Size, TextBaseline, rsuperellipse_radii_elliptical, valo::Point,
};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_gestures::{
    PointerCancelEventListener, PointerDownEventListener, PointerEnterEventListener, PointerEvent,
    PointerExitEventListener, PointerHoverEventListener, PointerMoveEventListener,
    PointerPanZoomEndEventListener, PointerPanZoomStartEventListener,
    PointerPanZoomUpdateEventListener, PointerSignalEventListener, PointerUpEventListener,
};
use inset_painting::{
    Alignment, AlignmentGeometry, BorderRadiusGeometry, BoxFit, BoxPainter, ClipContext,
    Decoration, ImageConfiguration, ShapeBorder, TextDirection, apply_box_fit, get_as_translation,
};
use inset_services::{MouseCursor, MouseCursorRef, MouseTrackerAnnotation};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestEntry, BoxHitTestResult, RenderBox, RenderBoxBase,
    RenderBoxData,
};
use crate::image_filter_config::{ImageFilterConfig, ImageFilterContext};
use crate::layer::{
    AnnotatedRegionLayer, BackdropFilterLayer, BackdropKey, ClipPathLayer, ClipRRectLayer,
    ClipRectLayer, ContainerLayer, FollowerLayer, LayerHandle, LayerLink, LeaderLayer,
    OffsetLayerMixin, OpacityLayer, TransformLayer,
};
use crate::layout_helper::{ChildLayoutHelper, ChildLayouter};
use crate::object::{
    AnyRenderObject, Constraints, EmptyParentData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// A base class for render boxes that resemble their children.
///
/// Flutter's `RenderProxyBoxMixin`: the shared bodies of a proxy box. A leaf implements the
/// marker and calls these where Dart would run the inherited method:
/// `RenderProxyBoxMixin::perform_layout(self, app)`.
pub trait RenderProxyBoxMixin:
    RenderObjectWithChildMixin<ChildType = AnyRenderBox> + RenderBox
{
    /// A proxy child gets plain parent data, not [`crate::BoxParentData`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if child.parent_data(app).is_none() {
            child.set_parent_data(app, EmptyParentData);
        }
    }

    /// A proxy paints its child at its own offset: no transform to apply.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let _ = (app, child, transform);
    }

    /// The child's minimum intrinsic width, or 0 without a child.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    /// The child's maximum intrinsic width, or 0 without a child.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    /// The child's minimum intrinsic height, or 0 without a child.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    /// The child's maximum intrinsic height, or 0 without a child.
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    /// The child's baseline, or none without a child.
    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.child(app)
            .and_then(|child| child.get_distance_to_actual_baseline(app, baseline))
    }

    /// The child's dry baseline, or none without a child.
    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.child(app)
            .and_then(|child| child.get_dry_baseline(app, constraints, baseline))
    }

    /// The child's dry layout at this box's constraints, or
    /// [`compute_size_for_no_child`](Self::compute_size_for_no_child).
    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        match self.child(app) {
            Some(child) => child.get_dry_layout(app, constraints),
            None => self.compute_size_for_no_child(app, constraints),
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_width() && additional.has_tight_width() {
            return additional.min_width;
        }
        let width = RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height);
        debug_assert!(width.is_finite());
        if additional.has_infinite_width() {
            width
        } else {
            additional.constrain_width(width)
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_width() && additional.has_tight_width() {
            return additional.min_width;
        }
        let width = RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height);
        debug_assert!(width.is_finite());
        if additional.has_infinite_width() {
            width
        } else {
            additional.constrain_width(width)
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_height() && additional.has_tight_height() {
            return additional.min_height;
        }
        let height = RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width);
        debug_assert!(height.is_finite());
        if additional.has_infinite_height() {
            height
        } else {
            additional.constrain_height(height)
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_height() && additional.has_tight_height() {
            return additional.min_height;
        }
        let height = RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width);
        debug_assert!(height.is_finite());
        if additional.has_infinite_height() {
            height
        } else {
            additional.constrain_height(height)
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let additional = self.get(app).additional_constraints;
        self.child(app).and_then(|child| {
            child.get_dry_baseline(app, additional.enforce(constraints), baseline)
        })
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let additional = self.get(app).additional_constraints;
        match self.child(app) {
            Some(child) => child.get_dry_layout(app, additional.enforce(constraints)),
            None => additional.enforce(constraints).constrain(Size::ZERO),
        }
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
        let did_need_compositing = self.always_needs_compositing(app);
        let this = self.get_mut(app);
        this.opacity = value;
        this.alpha = Color::get_alpha_from_opacity(value);
        if did_need_compositing != self.always_needs_compositing(app) {
            self.mark_needs_compositing_bits_update(app);
        }
        self.mark_needs_composited_layer_update(app);
    }
}

impl RenderObjectWithChildMixin for RenderOpacity {
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

impl RenderProxyBoxMixin for RenderOpacity {}

impl RenderObject for RenderOpacity {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some() && self.get(app).alpha > 0
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
        self.always_needs_compositing(app)
    }

    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<crate::AnyOffsetLayer>,
    ) -> crate::AnyOffsetLayer {
        let layer = match old_layer {
            Some(old) => app
                .handle::<OpacityLayer>(old.id())
                .expect("RenderOpacity reuses an OpacityLayer"),
            None => OpacityLayer::new(app),
        };
        layer.set_alpha(app, self.get(app).alpha);
        layer.as_offset_layer()
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Provides an anchor for a [`RenderFollowerLayer`].
///
/// Flutter's `RenderLeaderLayer`: paints its child through a leader item that records where the
/// child lands for the link's followers.
pub struct RenderLeaderLayer {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    link: Handle<LayerLink>,
    /// The size of the last layout, which `link.leader_size` reports.
    previous_layout_size: Option<Size>,
}

impl RenderLeaderLayer {
    pub fn new(
        app: &mut App,
        link: Handle<LayerLink>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderLeaderLayer {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                link,
                previous_layout_size: None,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The link object that connects this [`RenderLeaderLayer`] with one or more
    /// [`RenderFollowerLayer`]s.
    pub fn link(self: RenderHandle<Self>, app: &App) -> Handle<LayerLink> {
        self.get(app).link
    }

    /// Sets [`link`](Self::link).
    pub fn set_link(self: RenderHandle<Self>, app: &mut App, value: Handle<LayerLink>) {
        let link = self.get(app).link;
        if link == value {
            return;
        }
        let old = app.get_mut(link);
        old.leader_size = None;
        self.get_mut(app).link = value;
        if let Some(size) = self.get(app).previous_layout_size {
            app.get_mut(value).leader_size = Some(size);
        }
        self.mark_needs_paint(app);
    }
}

impl RenderObjectWithChildMixin for RenderLeaderLayer {
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

impl RenderProxyBoxMixin for RenderLeaderLayer {}

impl RenderObject for RenderLeaderLayer {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        let size = self.size(app);
        self.get_mut(app).previous_layout_size = Some(size);
        let link = self.get(app).link;
        app.get_mut(link).leader_size = Some(size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let link = self.get(app).link;
        let layer = match self.as_object().layer_as::<LeaderLayer>(app) {
            None => {
                let layer = LeaderLayer::new(app, link);
                layer.set_offset(app, offset);
                self.as_object()
                    .set_layer(app, Some(layer.as_container_layer()));
                layer
            }
            Some(layer) => {
                layer.set_link(app, link);
                layer.set_offset(app, offset);
                layer
            }
        };
        context.push_layer(
            app,
            layer.as_container_layer(),
            |app, context, offset| {
                RenderProxyBoxMixin::paint(self, app, context, offset);
            },
            Offset::ZERO,
            None,
        );
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

impl RenderBox for RenderLeaderLayer {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Transforms its child using a transform matrix computed at composite time from the link's
/// [`RenderLeaderLayer`]: the child paints where the leader is, whatever lies between them.
///
/// Flutter's `RenderFollowerLayer`.
pub struct RenderFollowerLayer {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    link: Handle<LayerLink>,
    show_when_unlinked: bool,
    offset: Offset,
    leader_anchor: Alignment,
    follower_anchor: Alignment,
}

impl RenderFollowerLayer {
    pub fn new(
        app: &mut App,
        link: Handle<LayerLink>,
        show_when_unlinked: bool,
        offset: Offset,
        leader_anchor: Alignment,
        follower_anchor: Alignment,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderFollowerLayer {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                link,
                show_when_unlinked,
                offset,
                leader_anchor,
                follower_anchor,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The link object that connects this [`RenderFollowerLayer`] with a [`RenderLeaderLayer`].
    pub fn link(self: RenderHandle<Self>, app: &App) -> Handle<LayerLink> {
        self.get(app).link
    }

    /// Sets [`link`](Self::link).
    pub fn set_link(self: RenderHandle<Self>, app: &mut App, value: Handle<LayerLink>) {
        if self.get(app).link == value {
            return;
        }
        self.get_mut(app).link = value;
        self.mark_needs_paint(app);
    }

    /// Whether to show the render object's contents when there is no corresponding
    /// [`RenderLeaderLayer`].
    pub fn show_when_unlinked(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).show_when_unlinked
    }

    /// Sets [`show_when_unlinked`](Self::show_when_unlinked).
    pub fn set_show_when_unlinked(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).show_when_unlinked == value {
            return;
        }
        self.get_mut(app).show_when_unlinked = value;
        self.mark_needs_paint(app);
    }

    /// The offset to apply to the origin of the linked [`RenderLeaderLayer`] to obtain this
    /// render object's origin.
    pub fn offset(self: RenderHandle<Self>, app: &App) -> Offset {
        self.get(app).offset
    }

    /// Sets [`offset`](Self::offset).
    pub fn set_offset(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if self.get(app).offset == value {
            return;
        }
        self.get_mut(app).offset = value;
        self.mark_needs_paint(app);
    }

    /// The anchor point on the linked [`RenderLeaderLayer`] that [`follower_anchor`] lines up
    /// with.
    ///
    /// [`follower_anchor`]: Self::follower_anchor
    pub fn leader_anchor(self: RenderHandle<Self>, app: &App) -> Alignment {
        self.get(app).leader_anchor
    }

    /// Sets [`leader_anchor`](Self::leader_anchor).
    pub fn set_leader_anchor(self: RenderHandle<Self>, app: &mut App, value: Alignment) {
        if self.get(app).leader_anchor == value {
            return;
        }
        self.get_mut(app).leader_anchor = value;
        self.mark_needs_paint(app);
    }

    /// The anchor point on this render object that lines up with [`leader_anchor`].
    ///
    /// [`leader_anchor`]: Self::leader_anchor
    pub fn follower_anchor(self: RenderHandle<Self>, app: &App) -> Alignment {
        self.get(app).follower_anchor
    }

    /// Sets [`follower_anchor`](Self::follower_anchor).
    pub fn set_follower_anchor(self: RenderHandle<Self>, app: &mut App, value: Alignment) {
        if self.get(app).follower_anchor == value {
            return;
        }
        self.get_mut(app).follower_anchor = value;
        self.mark_needs_paint(app);
    }

    /// The transform that was used during the last composite, from this render object's
    /// children to its own coordinates, or the identity when there was none.
    ///
    /// Flutter's `getCurrentTransform`.
    pub fn current_transform(self: RenderHandle<Self>, app: &App) -> Matrix4 {
        self.as_object()
            .layer_as::<FollowerLayer>(app)
            .and_then(|layer| layer.get_last_transform(app))
            .unwrap_or(Matrix4::IDENTITY)
    }
}

impl RenderObjectWithChildMixin for RenderFollowerLayer {
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

impl RenderProxyBoxMixin for RenderFollowerLayer {}

impl RenderObject for RenderFollowerLayer {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    /// Flutter's `detach`: `layer = null`.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().set_layer(app, None);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let link = self.get(app).link;
        let leader_size = app.get(link).leader_size;
        debug_assert!(
            leader_size.is_some()
                || link.leader(app).is_none()
                || self.get(app).leader_anchor == Alignment::TOP_LEFT,
            "a leader size is required when leader_anchor is not Alignment::TOP_LEFT"
        );
        let this = self.get(app);
        let effective_linked_offset = match leader_size {
            None => this.offset,
            Some(leader_size) => {
                this.leader_anchor.along_size(leader_size)
                    - this.follower_anchor.along_size(self.size(app))
                    + this.offset
            }
        };
        let show_when_unlinked = this.show_when_unlinked;
        let layer = match self.as_object().layer_as::<FollowerLayer>(app) {
            None => {
                let layer = FollowerLayer::new(app, link);
                layer.set_show_when_unlinked(app, show_when_unlinked);
                layer.set_linked_offset(app, effective_linked_offset);
                layer.set_unlinked_offset(app, offset);
                self.as_object()
                    .set_layer(app, Some(layer.as_container_layer()));
                layer
            }
            Some(layer) => {
                layer.set_link(app, link);
                layer.set_show_when_unlinked(app, show_when_unlinked);
                layer.set_linked_offset(app, effective_linked_offset);
                layer.set_unlinked_offset(app, offset);
                layer
            }
        };
        context.push_layer(
            app,
            layer.as_container_layer(),
            |app, context, offset| {
                RenderProxyBoxMixin::paint(self, app, context, offset);
            },
            Offset::ZERO,
            Some(Rect::from_ltrb(
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            )),
        );
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

impl RenderBox for RenderFollowerLayer {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        *transform = transform.then(&self.current_transform(app));
    }

    /// Disables the hit testing if this render object is hidden. A follower does not check
    /// whether it is itself hit: the untransformed size and the child's transformed position
    /// do not relate.
    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let link = self.get(app).link;
        if link.leader(app).is_none() && !self.get(app).show_when_unlinked {
            return false;
        }
        RenderBox::hit_test_children(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let transform = self.current_transform(app);
        result.add_with_paint_transform(Some(transform), position, |result, position| {
            RenderProxyBoxMixin::hit_test_children(self, app, result, position)
        })
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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
pub trait RenderAnimatedOpacityMixin:
    RenderObjectWithChildMixin<ChildType = AnyRenderBox> + RenderBox
{
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
        old_layer: Option<crate::AnyOffsetLayer>,
    ) -> crate::AnyOffsetLayer {
        let alpha = self
            .animated_opacity_data(app)
            .alpha
            .expect("set by the opacity setter");
        let layer = match old_layer {
            Some(old) => app
                .handle::<OpacityLayer>(old.id())
                .expect("RenderAnimatedOpacity reuses an OpacityLayer"),
            None => OpacityLayer::new(app),
        };
        layer.set_alpha(app, alpha);
        layer.as_offset_layer()
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
                self.mark_needs_compositing_bits_update(app);
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
        old_layer: Option<crate::AnyOffsetLayer>,
    ) -> crate::AnyOffsetLayer {
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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
                .copy_with(None, None, None, None, Some(size), None);
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Dart's `a == b` for two objects that do not override `==`: the same instance.
pub(crate) fn same_instance<T: ?Sized>(a: Option<&Rc<T>>, b: Option<&Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Constrains the child's [`BoxConstraints::max_width`] and [`BoxConstraints::max_height`] if
/// they're otherwise unconstrained.
///
/// This has the effect of giving the child a natural dimension in unbounded
/// environments. For example, by providing a [`max_height`](Self::max_height) to a widget that
/// normally tries to be as big as possible, the widget will normally size
/// itself to fit its parent, but when placed in a vertical list, it will take
/// on the given height.
///
/// This is useful when composing widgets that normally try to match their
/// parents' size, so that they behave reasonably in lists (which are
/// unbounded).
pub struct RenderLimitedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    max_width: f64,
    max_height: f64,
}

impl RenderLimitedBox {
    /// Creates a render box that imposes a maximum width or maximum height on its
    /// child if the child is otherwise unconstrained.
    ///
    /// The `max_width` and `max_height` arguments must be non-negative.
    pub fn new(
        app: &mut App,
        max_width: f64,
        max_height: f64,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderLimitedBox> {
        debug_assert!(max_width >= 0.0);
        debug_assert!(max_height >= 0.0);
        let this = RenderHandle::new_box(
            app,
            RenderLimitedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                max_width,
                max_height,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The value to use for `max_width` if the incoming `max_width` constraint is infinite.
    pub fn max_width(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).max_width
    }

    /// Sets [`max_width`](Self::max_width).
    pub fn set_max_width(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!(value >= 0.0);
        if self.get(app).max_width == value {
            return;
        }
        self.get_mut(app).max_width = value;
        self.mark_needs_layout(app);
    }

    /// The value to use for `max_height` if the incoming `max_height` constraint is infinite.
    pub fn max_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).max_height
    }

    /// Sets [`max_height`](Self::max_height).
    pub fn set_max_height(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!(value >= 0.0);
        if self.get(app).max_height == value {
            return;
        }
        self.get_mut(app).max_height = value;
        self.mark_needs_layout(app);
    }

    /// Flutter's `_computeSize`.
    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        let limited = self.limit_constraints(app, constraints);
        match self.child(app) {
            Some(child) => constraints.constrain(layout_child(app, child, limited)),
            None => limited.constrain(Size::ZERO),
        }
    }

    fn limit_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let this = self.get(app);
        BoxConstraints::new()
            .min_width(constraints.min_width)
            .max_width(if constraints.has_bounded_width() {
                constraints.max_width
            } else {
                constraints.constrain_width(this.max_width)
            })
            .min_height(constraints.min_height)
            .max_height(if constraints.has_bounded_height() {
                constraints.max_height
            } else {
                constraints.constrain_height(this.max_height)
            })
    }
}

impl RenderObjectWithChildMixin for RenderLimitedBox {
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

impl RenderProxyBoxMixin for RenderLimitedBox {}

impl RenderObject for RenderLimitedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
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

impl RenderBox for RenderLimitedBox {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }
}

/// Attempts to size the child to a specific aspect ratio.
///
/// The render object first tries the largest width permitted by the layout constraints. The
/// height of the render object is determined by applying the given aspect ratio to the width,
/// expressed as a ratio of width to height.
///
/// For example, a 16:9 width:height aspect ratio would have a value of 16.0/9.0. If the maximum
/// width is infinite, the initial width is determined by applying the aspect ratio to the maximum
/// height.
///
/// Now consider a second example, this time with an aspect ratio of 2.0 and layout constraints
/// that require the width to be between 0.0 and 100.0 and the height to be between 0.0 and 100.0.
/// The largest width permitted is 100.0, and applying the ratio gives a height of 50.0.
pub struct RenderAspectRatio {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aspect_ratio: f64,
}

impl RenderAspectRatio {
    /// Creates a render object with a specific aspect ratio.
    ///
    /// The `aspect_ratio` argument must be a finite, positive value.
    pub fn new(
        app: &mut App,
        aspect_ratio: f64,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderAspectRatio> {
        debug_assert!(aspect_ratio > 0.0);
        debug_assert!(aspect_ratio.is_finite());
        let this = RenderHandle::new_box(
            app,
            RenderAspectRatio {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aspect_ratio,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The aspect ratio to attempt to use.
    ///
    /// The aspect ratio is expressed as a ratio of width to height. For example, a 16:9
    /// width:height aspect ratio would have a value of 16.0/9.0.
    pub fn aspect_ratio(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).aspect_ratio
    }

    /// Sets [`aspect_ratio`](Self::aspect_ratio).
    pub fn set_aspect_ratio(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!(value > 0.0);
        debug_assert!(value.is_finite());
        if self.get(app).aspect_ratio == value {
            return;
        }
        self.get_mut(app).aspect_ratio = value;
        self.mark_needs_layout(app);
    }

    fn apply_aspect_ratio(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        debug_assert!(constraints.debug_assert_is_valid(false));
        let aspect_ratio = self.get(app).aspect_ratio;
        debug_assert!(
            constraints.has_bounded_width() || constraints.has_bounded_height(),
            "{} has unbounded constraints.\n\
             This render object was given an aspect ratio of {aspect_ratio} but was given both \
             unbounded width and unbounded height constraints. Because both constraints were \
             unbounded, this render object doesn't know how much size to consume.",
            std::any::type_name::<Self>()
        );

        if constraints.is_tight() {
            return constraints.smallest();
        }

        let mut width = constraints.max_width;
        let mut height;

        // We default to picking the height based on the width, but if the width would be
        // infinite, that's not sensible so we try to infer the height from the width.
        if width.is_finite() {
            height = width / aspect_ratio;
        } else {
            height = constraints.max_height;
            width = height * aspect_ratio;
        }

        // We iteratively attempt to fit within the given constraints while maintaining the given
        // aspect ratio. The order of applying the constraints is also biased towards inferring
        // the height from the width.

        if width > constraints.max_width {
            width = constraints.max_width;
            height = width / aspect_ratio;
        }

        if height > constraints.max_height {
            height = constraints.max_height;
            width = height * aspect_ratio;
        }

        if width < constraints.min_width {
            width = constraints.min_width;
            height = width / aspect_ratio;
        }

        if height < constraints.min_height {
            height = constraints.min_height;
            width = height * aspect_ratio;
        }

        constraints.constrain(Size::new(width, height))
    }
}

impl RenderObjectWithChildMixin for RenderAspectRatio {
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

impl RenderProxyBoxMixin for RenderAspectRatio {}

impl RenderObject for RenderAspectRatio {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.get_dry_layout(app, constraints);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            child.layout(app, BoxConstraints::tight(size), false);
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

impl RenderBox for RenderAspectRatio {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if height.is_finite() {
            return height * self.get(app).aspect_ratio;
        }
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if height.is_finite() {
            return height * self.get(app).aspect_ratio;
        }
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if width.is_finite() {
            return width / self.get(app).aspect_ratio;
        }
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if width.is_finite() {
            return width / self.get(app).aspect_ratio;
        }
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.apply_aspect_ratio(app, constraints)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let size = self.get_dry_layout(app, constraints);
        RenderProxyBoxMixin::compute_dry_baseline(self, app, BoxConstraints::tight(size), baseline)
    }
}

/// Sizes its child to the child's maximum intrinsic width.
///
/// This class is useful, for example, when unlimited width is available and you would like a
/// child that would otherwise attempt to expand infinitely to instead size itself to a more
/// reasonable width.
///
/// The constraints that this object passes to its child will adhere to the parent's constraints,
/// so if the constraints are not large enough to satisfy the child's maximum intrinsic width,
/// then the child will get less width than it otherwise would. Likewise, if the minimum width
/// constraint is larger than the child's maximum intrinsic width, the child will be given more
/// width than it otherwise would.
///
/// If [`step_width`](Self::step_width) is non-null, the child's width will be snapped to a
/// multiple of it. Similarly, if [`step_height`](Self::step_height) is non-null, the child's
/// height will be snapped to a multiple of it.
///
/// This class is relatively expensive, because it adds a speculative layout pass before the final
/// layout phase. Avoid using it where possible. In the worst case, this render object can result
/// in a layout that is O(N²) in the depth of the tree.
pub struct RenderIntrinsicWidth {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    step_width: Option<f64>,
    step_height: Option<f64>,
}

/// Flutter's `RenderIntrinsicWidth._applyStep`.
fn apply_step(input: f64, step: Option<f64>) -> f64 {
    debug_assert!(input.is_finite());
    match step {
        Some(step) => (input / step).ceil() * step,
        None => input,
    }
}

impl RenderIntrinsicWidth {
    /// Creates a render object that sizes itself to its child's intrinsic width.
    ///
    /// If `step_width` is given it must be > 0.0. Similarly if `step_height` is given it must be
    /// > 0.0.
    pub fn new(
        app: &mut App,
        step_width: Option<f64>,
        step_height: Option<f64>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderIntrinsicWidth> {
        debug_assert!(step_width.is_none_or(|step| step > 0.0));
        debug_assert!(step_height.is_none_or(|step| step > 0.0));
        let this = RenderHandle::new_box(
            app,
            RenderIntrinsicWidth {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                step_width,
                step_height,
            },
        );
        this.set_child(app, child);
        this
    }

    /// If non-null, force the child's width to be a multiple of this value.
    pub fn step_width(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).step_width
    }

    /// Sets [`step_width`](Self::step_width).
    pub fn set_step_width(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|step| step > 0.0));
        if self.get(app).step_width == value {
            return;
        }
        self.get_mut(app).step_width = value;
        self.mark_needs_layout(app);
    }

    /// If non-null, force the child's height to be a multiple of this value.
    pub fn step_height(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).step_height
    }

    /// Sets [`step_height`](Self::step_height).
    pub fn set_step_height(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|step| step > 0.0));
        if self.get(app).step_height == value {
            return;
        }
        self.get_mut(app).step_height = value;
        self.mark_needs_layout(app);
    }

    fn child_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let (step_width, step_height) = {
            let this = self.get(app);
            (this.step_width, this.step_height)
        };
        let width = if constraints.has_tight_width() {
            None
        } else {
            Some(apply_step(
                child.get_max_intrinsic_width(app, constraints.max_height),
                step_width,
            ))
        };
        let height = step_height.map(|_| {
            apply_step(
                child.get_max_intrinsic_height(app, constraints.max_width),
                step_height,
            )
        });
        constraints.tighten(width, height)
    }

    /// Flutter's `_computeSize`.
    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        match self.child(app) {
            Some(child) => {
                let child_constraints = self.child_constraints(app, child, constraints);
                layout_child(app, child, child_constraints)
            }
            None => constraints.smallest(),
        }
    }
}

impl RenderObjectWithChildMixin for RenderIntrinsicWidth {
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

impl RenderProxyBoxMixin for RenderIntrinsicWidth {}

impl RenderObject for RenderIntrinsicWidth {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
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

impl RenderBox for RenderIntrinsicWidth {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.as_box().get_max_intrinsic_width(app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let width = child.get_max_intrinsic_width(app, height);
        apply_step(width, self.get(app).step_width)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let width = if width.is_finite() {
            width
        } else {
            self.as_box().get_max_intrinsic_width(app, f64::INFINITY)
        };
        debug_assert!(width.is_finite());
        let height = child.get_min_intrinsic_height(app, width);
        apply_step(height, self.get(app).step_height)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let width = if width.is_finite() {
            width
        } else {
            self.as_box().get_max_intrinsic_width(app, f64::INFINITY)
        };
        debug_assert!(width.is_finite());
        let height = child.get_max_intrinsic_height(app, width);
        apply_step(height, self.get(app).step_height)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self.child_constraints(app, child, constraints);
        child.get_dry_baseline(app, child_constraints, baseline)
    }
}

/// Sizes its child to the child's intrinsic height.
///
/// This class is useful, for example, when unlimited height is available and you would like a
/// child that would otherwise attempt to expand infinitely to instead size itself to a more
/// reasonable height.
///
/// The constraints that this object passes to its child will adhere to the parent's constraints,
/// so if the constraints are not large enough to satisfy the child's maximum intrinsic height,
/// then the child will get less height than it otherwise would. Likewise, if the minimum height
/// constraint is larger than the child's maximum intrinsic height, the child will be given more
/// height than it otherwise would.
///
/// This class is relatively expensive, because it adds a speculative layout pass before the final
/// layout phase. Avoid using it where possible. In the worst case, this render object can result
/// in a layout that is O(N²) in the depth of the tree.
pub struct RenderIntrinsicHeight {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
}

impl RenderIntrinsicHeight {
    /// Creates a render object that sizes itself to its child's intrinsic height.
    pub fn new(app: &mut App, child: Option<AnyRenderBox>) -> RenderHandle<RenderIntrinsicHeight> {
        let this = RenderHandle::new_box(
            app,
            RenderIntrinsicHeight {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
            },
        );
        this.set_child(app, child);
        this
    }

    fn child_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        if constraints.has_tight_height() {
            return constraints;
        }
        let height = child.get_max_intrinsic_height(app, constraints.max_width);
        constraints.tighten(None, Some(height))
    }

    /// Flutter's `_computeSize`.
    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        match self.child(app) {
            Some(child) => {
                let child_constraints = self.child_constraints(app, child, constraints);
                layout_child(app, child, child_constraints)
            }
            None => constraints.smallest(),
        }
    }
}

impl RenderObjectWithChildMixin for RenderIntrinsicHeight {
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

impl RenderProxyBoxMixin for RenderIntrinsicHeight {}

impl RenderObject for RenderIntrinsicHeight {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
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

impl RenderBox for RenderIntrinsicHeight {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let height = if height.is_finite() {
            height
        } else {
            child.get_max_intrinsic_height(app, f64::INFINITY)
        };
        debug_assert!(height.is_finite());
        child.get_min_intrinsic_width(app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let height = if height.is_finite() {
            height
        } else {
            child.get_max_intrinsic_height(app, f64::INFINITY)
        };
        debug_assert!(height.is_finite());
        child.get_max_intrinsic_width(app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_max_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self.child_constraints(app, child, constraints);
        child.get_dry_baseline(app, child_constraints, baseline)
    }
}

/// An interface for providing custom clips.
///
/// This trait is used by a number of clip widgets (e.g., `ClipRect` and `ClipPath`).
///
/// The [`get_clip`](Self::get_clip) method is called whenever the custom clip needs to be
/// updated.
///
/// The [`should_reclip`](Self::should_reclip) method is called when a new instance of the class
/// is provided, to check if the new instance actually represents different information.
///
/// The most efficient way to update the clip provided by this class is to
/// supply a [`reclip`](Self::reclip) listenable, which notifies its listeners when it is time to
/// reclip, avoiding both the build and layout phases of the pipeline.
pub trait CustomClipper<T>: 'static {
    /// The [`Listenable`] that notifies when it is time to reclip.
    ///
    /// Dart's `CustomClipper` takes it as a constructor argument and forwards `addListener` /
    /// `removeListener` to it; here the clipper answers with it and the same forwarding is the
    /// [`Listenable`] implementation of `dyn CustomClipper<T>`.
    fn reclip(&self) -> Option<&Rc<dyn Listenable>> {
        None
    }

    /// Returns a description of the clip given that the render object being
    /// clipped is of the given size.
    fn get_clip(&self, size: Size) -> T;

    /// Called whenever a new instance of the custom clipper delegate class is
    /// provided to the clip object, or any time that a new clip object is created
    /// with a new instance of the custom clipper delegate class (which amounts to
    /// the same thing, because the latter is implemented in terms of the former).
    ///
    /// If the new instance represents different information than the old
    /// instance, then the method should return true, otherwise it should return
    /// false.
    ///
    /// If the method returns false, then the [`get_clip`](Self::get_clip) call might be
    /// optimized away.
    ///
    /// It's possible that the [`get_clip`](Self::get_clip) method will get called even if
    /// [`should_reclip`](Self::should_reclip) returns false or if the
    /// [`should_reclip`](Self::should_reclip) method is never called at all (e.g. if the box
    /// changes size). It is only called with a clipper of this clipper's own type, as Dart's
    /// `covariant` parameter is.
    fn should_reclip(&self, old_clipper: &dyn CustomClipper<T>) -> bool;

    /// Downcast support for Dart's `runtimeType` comparison and `covariant` parameter.
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> Listenable for dyn CustomClipper<T> {
    /// Register a closure to be notified when it is time to reclip.
    ///
    /// Forwards to the [`reclip`](CustomClipper::reclip) listenable, if it is not `None`.
    fn add_listener(&self, app: &mut App, listener: Listener) {
        if let Some(reclip) = self.reclip() {
            reclip.add_listener(app, listener);
        }
    }

    /// Remove a previously registered closure from the list of closures that the
    /// object notifies when it is time to reclip.
    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        if let Some(reclip) = self.reclip() {
            reclip.remove_listener(app, listener);
        }
    }
}

/// The fields of [`RenderCustomClip`].
pub struct RenderCustomClipData<T> {
    clipper: Option<Rc<dyn CustomClipper<T>>>,
    clip: Option<T>,
    clip_behavior: Clip,
}

impl<T> RenderCustomClipData<T> {
    /// Creates the fields of a clip with no clip computed yet.
    pub fn new(
        clipper: Option<Rc<dyn CustomClipper<T>>>,
        clip_behavior: Clip,
    ) -> RenderCustomClipData<T> {
        RenderCustomClipData {
            clipper,
            clip: None,
            clip_behavior,
        }
    }
}

/// A [`CustomClipper`] that clips to the outer path of a `ShapeBorder`.
pub struct ShapeBorderClipper {
    /// The shape border whose outer path defines the clip.
    pub shape: Box<dyn ShapeBorder>,
    /// The text direction to use for getting the outer path for `shape`.
    ///
    /// `ShapeBorder`s can depend on the text direction (e.g having a "dent" on the left hand
    /// side in LTR and on the right hand side in RTL).
    pub text_direction: Option<TextDirection>,
}

impl ShapeBorderClipper {
    /// Creates a [`ShapeBorder`] clipper.
    ///
    /// The `text_direction` argument must be provided non-`None` if `shape` has a text
    /// direction dependency (for example if it is expressed in terms of "start" and "end"
    /// instead of "left" and "right"). It may be `None` if the border will not need the text
    /// direction to paint itself.
    pub fn new(shape: Box<dyn ShapeBorder>, text_direction: Option<TextDirection>) -> Self {
        ShapeBorderClipper {
            shape,
            text_direction,
        }
    }
}

impl CustomClipper<Arc<Path>> for ShapeBorderClipper {
    fn get_clip(&self, size: Size) -> Arc<Path> {
        self.shape
            .get_outer_path(Offset::ZERO & size, self.text_direction)
    }

    fn should_reclip(&self, old_clipper: &dyn CustomClipper<Arc<Path>>) -> bool {
        let Some(old_clipper) = old_clipper.as_any().downcast_ref::<ShapeBorderClipper>() else {
            return true;
        };
        !old_clipper.shape.eq_shape(&*self.shape)
            || old_clipper.text_direction != self.text_direction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A base class for render boxes that clip their child.
///
/// Flutter's private `_RenderCustomClip<T>`: the shared bodies of a clip. A leaf implements
/// [`default_clip`](Self::default_clip), holds a [`RenderCustomClipData<T>`], and calls these
/// where Dart would run the inherited method: `RenderCustomClip::perform_layout(self, app)`.
pub trait RenderCustomClip<T: Clone + 'static>: RenderProxyBoxMixin {
    /// Mixin field access.
    fn custom_clip_data(self: RenderHandle<Self>, app: &App) -> &RenderCustomClipData<T>;

    /// See [`custom_clip_data`](Self::custom_clip_data).
    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<T>;

    /// The clip to use when [`clipper`](Self::clipper) is `None`.
    fn default_clip(self: RenderHandle<Self>, app: &App) -> T;

    /// If non-`None`, determines which clip to use on the child.
    fn clipper(self: RenderHandle<Self>, app: &App) -> Option<Rc<dyn CustomClipper<T>>> {
        self.custom_clip_data(app).clipper.clone()
    }

    /// Sets [`clipper`](Self::clipper).
    fn set_clipper(
        self: RenderHandle<Self>,
        app: &mut App,
        new_clipper: Option<Rc<dyn CustomClipper<T>>>,
    ) {
        let old_clipper = self.clipper(app);
        if same_instance(old_clipper.as_ref(), new_clipper.as_ref()) {
            return;
        }
        self.custom_clip_data_mut(app).clipper = new_clipper.clone();
        debug_assert!(new_clipper.is_some() || old_clipper.is_some());
        let reclip = match (&new_clipper, &old_clipper) {
            (Some(new_clipper), Some(old_clipper)) => {
                new_clipper.as_any().type_id() != old_clipper.as_any().type_id()
                    || new_clipper.should_reclip(&**old_clipper)
            }
            _ => true,
        };
        if reclip {
            self.mark_needs_clip(app);
        }
        if self.attached(app) {
            if let Some(old_clipper) = &old_clipper {
                old_clipper.remove_listener(app, &self.clip_listener());
            }
            if let Some(new_clipper) = &new_clipper {
                new_clipper.add_listener(app, self.clip_listener());
            }
        }
    }

    /// Flutter's `attach` after `super.attach(owner)`.
    fn did_attach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(clipper) = self.clipper(app) {
            clipper.add_listener(app, self.clip_listener());
        }
    }

    /// Flutter's `detach` before `super.detach()`.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(clipper) = self.clipper(app) {
            clipper.remove_listener(app, &self.clip_listener());
        }
    }

    /// Flutter's `_markNeedsClip`.
    fn mark_needs_clip(self: RenderHandle<Self>, app: &mut App) {
        self.custom_clip_data_mut(app).clip = None;
        self.mark_needs_paint(app);
    }

    /// The `_markNeedsClip` tear-off, equal to itself across registrations.
    fn clip_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), mark_needs_clip::<Self, T>)
    }

    /// Whether and how to clip the child.
    fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.custom_clip_data(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value != self.clip_behavior(app) {
            self.custom_clip_data_mut(app).clip_behavior = value;
            self.mark_needs_paint(app);
        }
    }

    /// Flutter's `performLayout`: a resize drops the cached clip.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let old_size = self.has_size(app).then(|| self.size(app));
        RenderProxyBoxMixin::perform_layout(self, app);
        if old_size != Some(self.size(app)) {
            self.custom_clip_data_mut(app).clip = None;
        }
    }

    /// Flutter's `_updateClip`: computes the clip if it is not cached.
    fn update_clip(self: RenderHandle<Self>, app: &mut App) {
        if self.custom_clip_data(app).clip.is_some() {
            return;
        }
        let clip = match self.clipper(app) {
            Some(clipper) => clipper.get_clip(self.size(app)),
            None => self.default_clip(app),
        };
        self.custom_clip_data_mut(app).clip = Some(clip);
    }

    /// The cached clip. Call [`update_clip`](Self::update_clip) first; Dart's `_clip!`.
    fn clip(self: RenderHandle<Self>, app: &App) -> T {
        self.custom_clip_data(app)
            .clip
            .clone()
            .expect("update_clip computes the clip")
    }
}

fn mark_needs_clip<C: RenderCustomClip<T>, T: Clone + 'static>(this: Handle<C>, app: &mut App) {
    RenderCustomClip::mark_needs_clip(RenderHandle::from_handle(this), app);
}

/// Clips its child using a rectangle.
///
/// By default, [`RenderClipRect`] prevents its child from painting outside its
/// bounds, but the size and location of the clip rect can be customized using a
/// custom [`clipper`](RenderCustomClip::clipper).
pub struct RenderClipRect {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    custom_clip: RenderCustomClipData<Rect>,
}

impl RenderClipRect {
    /// Creates a rectangular clip.
    ///
    /// If `clipper` is `None`, the clip will match the layout size and position of
    /// the child.
    ///
    /// If `clip_behavior` is [`Clip::None`], no clipping will be applied.
    pub fn new(
        app: &mut App,
        clipper: Option<Rc<dyn CustomClipper<Rect>>>,
        clip_behavior: Clip,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderClipRect> {
        let this = RenderHandle::new_box(
            app,
            RenderClipRect {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                custom_clip: RenderCustomClipData::new(clipper, clip_behavior),
            },
        );
        this.set_child(app, child);
        this
    }
}

impl RenderObjectWithChildMixin for RenderClipRect {
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

impl RenderProxyBoxMixin for RenderClipRect {}

impl RenderCustomClip<Rect> for RenderClipRect {
    fn custom_clip_data(self: RenderHandle<Self>, app: &App) -> &RenderCustomClipData<Rect> {
        &self.get(app).custom_clip
    }

    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<Rect> {
        &mut self.get_mut(app).custom_clip
    }

    fn default_clip(self: RenderHandle<Self>, app: &App) -> Rect {
        Offset::ZERO & self.size(app)
    }
}

impl RenderObject for RenderClipRect {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderCustomClip::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let clip_behavior = self.clip_behavior(app);
            if clip_behavior != Clip::None {
                self.update_clip(app);
                let clip = self.clip(app);
                let old = self.as_object().layer_as::<ClipRectLayer>(app);
                let layer = context.push_clip_rect(
                    app,
                    self.as_object().needs_compositing(app),
                    offset,
                    clip,
                    |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                    clip_behavior,
                    old,
                );
                self.as_object()
                    .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            } else {
                context.paint_child(app, child.as_object(), offset);
                self.as_object().set_layer(app, None);
            }
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderClipRect {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if self.clipper(app).is_some() {
            self.update_clip(app);
            if !self.clip(app).contains(position) {
                return false;
            }
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Clips its child using a rounded rectangle.
///
/// By default, [`RenderClipRRect`] uses its own bounds as the base rectangle for
/// the clip, but the size and location of the clip can be customized using a
/// custom [`clipper`](RenderCustomClip::clipper).
pub struct RenderClipRRect {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    custom_clip: RenderCustomClipData<RRect>,
    border_radius: BorderRadiusGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderClipRRect {
    /// Creates a rounded-rectangular clip.
    ///
    /// The `border_radius` defaults to [`BorderRadiusGeometry::ZERO`], i.e. a rectangle with
    /// right-angled corners.
    ///
    /// If `clipper` is non-`None`, then `border_radius` is ignored.
    ///
    /// If `clip_behavior` is [`Clip::None`], no clipping will be applied.
    pub fn new(
        app: &mut App,
        border_radius: BorderRadiusGeometry,
        clipper: Option<Rc<dyn CustomClipper<RRect>>>,
        clip_behavior: Clip,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderClipRRect> {
        let this = RenderHandle::new_box(
            app,
            RenderClipRRect {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                custom_clip: RenderCustomClipData::new(clipper, clip_behavior),
                border_radius,
                text_direction,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The border radius of the rounded corners.
    ///
    /// Values are clamped so that horizontal and vertical radii sums do not
    /// exceed width/height.
    ///
    /// This value is ignored if [`clipper`](RenderCustomClip::clipper) is non-`None`.
    pub fn border_radius(self: RenderHandle<Self>, app: &App) -> BorderRadiusGeometry {
        self.get(app).border_radius
    }

    /// Sets [`border_radius`](Self::border_radius).
    pub fn set_border_radius(self: RenderHandle<Self>, app: &mut App, value: BorderRadiusGeometry) {
        if self.get(app).border_radius == value {
            return;
        }
        self.get_mut(app).border_radius = value;
        self.mark_needs_clip(app);
    }

    /// The text direction with which to resolve [`border_radius`](Self::border_radius).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_clip(app);
    }
}

impl RenderObjectWithChildMixin for RenderClipRRect {
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

impl RenderProxyBoxMixin for RenderClipRRect {}

impl RenderCustomClip<RRect> for RenderClipRRect {
    fn custom_clip_data(self: RenderHandle<Self>, app: &App) -> &RenderCustomClipData<RRect> {
        &self.get(app).custom_clip
    }

    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<RRect> {
        &mut self.get_mut(app).custom_clip
    }

    fn default_clip(self: RenderHandle<Self>, app: &App) -> RRect {
        let this = self.get(app);
        this.border_radius
            .resolve(this.text_direction)
            .to_rrect(Offset::ZERO & self.size(app))
    }
}

impl RenderObject for RenderClipRRect {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderCustomClip::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let clip_behavior = self.clip_behavior(app);
            if clip_behavior != Clip::None {
                self.update_clip(app);
                let clip = self.clip(app);
                let old = self.as_object().layer_as::<ClipRRectLayer>(app);
                let layer = context.push_clip_rrect(
                    app,
                    self.as_object().needs_compositing(app),
                    offset,
                    clip.outer_rect(),
                    clip,
                    |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                    clip_behavior,
                    old,
                );
                self.as_object()
                    .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            } else {
                context.paint_child(app, child.as_object(), offset);
                self.as_object().set_layer(app, None);
            }
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderClipRRect {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if self.clipper(app).is_some() {
            self.update_clip(app);
            if !self.clip(app).contains(position) {
                return false;
            }
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Clips its child using an oval.
///
/// Inscribes an oval into its layout dimensions and prevents its child from
/// painting outside that oval.
pub struct RenderClipOval {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    custom_clip: RenderCustomClipData<Rect>,
    cached_rect: Option<Rect>,
    cached_path: Option<Arc<Path>>,
}

impl RenderClipOval {
    /// Creates an oval-shaped clip.
    ///
    /// If `clipper` is `None`, the oval will be inscribed into the layout size and
    /// position of the child.
    ///
    /// If `clip_behavior` is [`Clip::None`], no clipping will be applied.
    pub fn new(
        app: &mut App,
        clipper: Option<Rc<dyn CustomClipper<Rect>>>,
        clip_behavior: Clip,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderClipOval> {
        let this = RenderHandle::new_box(
            app,
            RenderClipOval {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                custom_clip: RenderCustomClipData::new(clipper, clip_behavior),
                cached_rect: None,
                cached_path: None,
            },
        );
        this.set_child(app, child);
        this
    }

    fn get_clip_path(self: RenderHandle<Self>, app: &mut App, rect: Rect) -> Arc<Path> {
        if self.get(app).cached_rect != Some(rect) {
            let radius = [(rect.width() / 2.0) as f32, (rect.height() / 2.0) as f32];
            let mut path = PathBuilder::new();
            path.rrect_radii_elliptical(rect, [radius; 4]);
            let this = self.get_mut(app);
            this.cached_rect = Some(rect);
            this.cached_path = Some(path.build());
        }
        self.get(app).cached_path.clone().expect("cached above")
    }
}

impl RenderObjectWithChildMixin for RenderClipOval {
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

impl RenderProxyBoxMixin for RenderClipOval {}

impl RenderCustomClip<Rect> for RenderClipOval {
    fn custom_clip_data(self: RenderHandle<Self>, app: &App) -> &RenderCustomClipData<Rect> {
        &self.get(app).custom_clip
    }

    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<Rect> {
        &mut self.get_mut(app).custom_clip
    }

    fn default_clip(self: RenderHandle<Self>, app: &App) -> Rect {
        Offset::ZERO & self.size(app)
    }
}

impl RenderObject for RenderClipOval {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderCustomClip::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let clip_behavior = self.clip_behavior(app);
            if clip_behavior != Clip::None {
                self.update_clip(app);
                let clip = self.clip(app);
                let clip_path = self.get_clip_path(app, clip);
                let old = self.as_object().layer_as::<ClipPathLayer>(app);
                let layer = context.push_clip_path(
                    app,
                    self.as_object().needs_compositing(app),
                    offset,
                    clip,
                    clip_path,
                    |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                    clip_behavior,
                    old,
                );
                self.as_object()
                    .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            } else {
                context.paint_child(app, child.as_object(), offset);
                self.as_object().set_layer(app, None);
            }
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderClipOval {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.update_clip(app);
        let clip = self.clip(app);
        let center = clip.center();
        // convert the position to an offset from the center of the unit circle
        let offset = Offset::new(
            (position.dx() - center.dx()) / clip.width(),
            (position.dy() - center.dy()) / clip.height(),
        );
        // check if the point is outside the unit circle
        if offset.distance_squared() > 0.25 {
            // x^2 + y^2 > r^2
            return false;
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Clips its child using a path.
///
/// Takes a delegate whose primary method returns a path that should
/// be used to prevent the child from painting outside the path.
///
/// Clipping to a path is expensive. Certain shapes have more
/// optimized render objects:
///
///  * To clip to a rectangle, consider [`RenderClipRect`].
///  * To clip to an oval or circle, consider [`RenderClipOval`].
///  * To clip to a rounded rectangle, consider [`RenderClipRRect`].
pub struct RenderClipPath {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    custom_clip: RenderCustomClipData<Arc<Path>>,
}

impl RenderClipPath {
    /// Creates a path clip.
    ///
    /// If `clipper` is `None`, the clip will be a rectangle that matches the layout
    /// size and location of the child. However, rather than use this default,
    /// consider using a [`RenderClipRect`], which can achieve the same effect more
    /// efficiently.
    ///
    /// If `clip_behavior` is [`Clip::None`], no clipping will be applied.
    pub fn new(
        app: &mut App,
        clipper: Option<Rc<dyn CustomClipper<Arc<Path>>>>,
        clip_behavior: Clip,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderClipPath> {
        let this = RenderHandle::new_box(
            app,
            RenderClipPath {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                custom_clip: RenderCustomClipData::new(clipper, clip_behavior),
            },
        );
        this.set_child(app, child);
        this
    }
}

impl RenderObjectWithChildMixin for RenderClipPath {
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

impl RenderProxyBoxMixin for RenderClipPath {}

impl RenderCustomClip<Arc<Path>> for RenderClipPath {
    fn custom_clip_data(self: RenderHandle<Self>, app: &App) -> &RenderCustomClipData<Arc<Path>> {
        &self.get(app).custom_clip
    }

    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<Arc<Path>> {
        &mut self.get_mut(app).custom_clip
    }

    fn default_clip(self: RenderHandle<Self>, app: &App) -> Arc<Path> {
        let mut path = PathBuilder::new();
        path.rect((Offset::ZERO & self.size(app)).into());
        path.build()
    }
}

impl RenderObject for RenderClipPath {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderCustomClip::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let clip_behavior = self.clip_behavior(app);
            if clip_behavior != Clip::None {
                self.update_clip(app);
                let clip = self.clip(app);
                let bounds = Offset::ZERO & self.size(app);
                let old = self.as_object().layer_as::<ClipPathLayer>(app);
                let layer = context.push_clip_path(
                    app,
                    self.as_object().needs_compositing(app),
                    offset,
                    bounds,
                    clip,
                    |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                    clip_behavior,
                    old,
                );
                self.as_object()
                    .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            } else {
                context.paint_child(app, child.as_object(), offset);
                self.as_object().set_layer(app, None);
            }
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderClipPath {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if self.clipper(app).is_some() {
            self.update_clip(app);
            let point = Point::new(position.dx() as f32, position.dy() as f32);
            if !self.clip(app).contains(point, FillRule::NonZero) {
                return false;
            }
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Clips its child using a rounded superellipse.
///
/// By default, [`RenderClipRSuperellipse`] uses its own
/// [`border_radius`](Self::border_radius) to create the shape. A custom `clipper` can
/// define any rounded superellipse.
pub struct RenderClipRSuperellipse {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    custom_clip: RenderCustomClipData<RSuperellipse>,
    border_radius: BorderRadiusGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderClipRSuperellipse {
    /// Creates a rounded-superellipse clip.
    ///
    /// The `border_radius` defaults to [`BorderRadiusGeometry::ZERO`], i.e. a rectangle with
    /// right-angled corners.
    ///
    /// If `clipper` is non-`None`, then `border_radius` is ignored.
    ///
    /// If `clip_behavior` is [`Clip::None`], no clipping will be applied.
    pub fn new(
        app: &mut App,
        border_radius: BorderRadiusGeometry,
        clipper: Option<Rc<dyn CustomClipper<RSuperellipse>>>,
        clip_behavior: Clip,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderClipRSuperellipse> {
        let this = RenderHandle::new_box(
            app,
            RenderClipRSuperellipse {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                custom_clip: RenderCustomClipData::new(clipper, clip_behavior),
                border_radius,
                text_direction,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The border radius of the rounded corners.
    ///
    /// Values are clamped so that horizontal and vertical radii sums do not exceed
    /// width/height.
    ///
    /// This value is ignored if `clipper` is non-`None`.
    pub fn border_radius(self: RenderHandle<Self>, app: &App) -> BorderRadiusGeometry {
        self.get(app).border_radius
    }

    /// Sets [`border_radius`](Self::border_radius).
    pub fn set_border_radius(self: RenderHandle<Self>, app: &mut App, value: BorderRadiusGeometry) {
        if self.get(app).border_radius == value {
            return;
        }
        self.get_mut(app).border_radius = value;
        self.mark_needs_clip(app);
    }

    /// The text direction with which to resolve [`border_radius`](Self::border_radius).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_clip(app);
    }
}

impl RenderObjectWithChildMixin for RenderClipRSuperellipse {
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

impl RenderProxyBoxMixin for RenderClipRSuperellipse {}

impl RenderCustomClip<RSuperellipse> for RenderClipRSuperellipse {
    fn custom_clip_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderCustomClipData<RSuperellipse> {
        &self.get(app).custom_clip
    }

    fn custom_clip_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderCustomClipData<RSuperellipse> {
        &mut self.get_mut(app).custom_clip
    }

    fn default_clip(self: RenderHandle<Self>, app: &App) -> RSuperellipse {
        let this = self.get(app);
        this.border_radius
            .resolve(this.text_direction)
            .to_r_superellipse(Offset::ZERO & self.size(app))
    }
}

impl RenderObject for RenderClipRSuperellipse {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        RenderCustomClip::did_attach(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderCustomClip::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let clip_behavior = self.clip_behavior(app);
            if clip_behavior != Clip::None {
                self.update_clip(app);
                let clip = self.clip(app);
                let mut clip_path = PathBuilder::new();
                clip_path.rsuperellipse_radii(
                    clip.outer_rect().into(),
                    rsuperellipse_radii_elliptical(clip),
                );
                let old = self.as_object().layer_as::<ClipPathLayer>(app);
                let layer = context.push_clip_path(
                    app,
                    self.as_object().needs_compositing(app),
                    offset,
                    clip.outer_rect(),
                    clip_path.build(),
                    |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                    clip_behavior,
                    old,
                );
                self.as_object()
                    .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            } else {
                context.paint_child(app, child.as_object(), offset);
                self.as_object().set_layer(app, None);
            }
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderClipRSuperellipse {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if self.clipper(app).is_some() {
            self.update_clip(app);
            if !self.clip(app).outer_rect().contains(position) {
                return false;
            }
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Applies a transformation before painting its child.
pub struct RenderTransform {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    origin: Option<Offset>,
    alignment: Option<AlignmentGeometry>,
    text_direction: Option<TextDirection>,
    transform_hit_tests: bool,
    transform: Matrix4,
}

impl RenderTransform {
    /// Creates a render object that transforms its child.
    pub fn new(
        app: &mut App,
        transform: Matrix4,
        origin: Option<Offset>,
        alignment: Option<AlignmentGeometry>,
        text_direction: Option<TextDirection>,
        transform_hit_tests: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderTransform> {
        let this = RenderHandle::new_box(
            app,
            RenderTransform {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                origin,
                alignment,
                text_direction,
                transform_hit_tests,
                transform,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The origin of the coordinate system (relative to the upper left corner of
    /// this render object) in which to apply the matrix.
    ///
    /// Setting an origin is equivalent to conjugating the transform matrix by a
    /// translation. This property is provided just for convenience.
    pub fn origin(self: RenderHandle<Self>, app: &App) -> Option<Offset> {
        self.get(app).origin
    }

    /// Sets [`origin`](Self::origin).
    pub fn set_origin(self: RenderHandle<Self>, app: &mut App, value: Option<Offset>) {
        if self.get(app).origin == value {
            return;
        }
        self.get_mut(app).origin = value;
        self.mark_needs_paint(app);
    }

    /// The alignment of the origin, relative to the size of the box.
    ///
    /// This is equivalent to setting an origin based on the size of the box.
    /// If it is specified at the same time as an offset, both are applied.
    ///
    /// An `AlignmentDirectional::CENTER_START` value is the same as an [`Alignment`]
    /// whose `x` value is `-1.0` if [`text_direction`](Self::text_direction) is
    /// [`TextDirection::Ltr`], and `1.0` if it is [`TextDirection::Rtl`].
    /// Similarly `AlignmentDirectional::CENTER_END` is the same as an [`Alignment`]
    /// whose `x` value is `1.0` if [`text_direction`](Self::text_direction) is
    /// [`TextDirection::Ltr`], and `-1.0` if it is [`TextDirection::Rtl`].
    ///
    /// [`Alignment`]: inset_painting::Alignment
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> Option<AlignmentGeometry> {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<AlignmentGeometry>,
    ) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.mark_needs_paint(app);
    }

    /// The text direction with which to resolve [`alignment`](Self::alignment).
    ///
    /// This may be changed to `None`, but only after [`alignment`](Self::alignment) has been
    /// changed to a value that does not depend on the direction.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_paint(app);
    }

    /// When true, hit tests are performed based on the position of the
    /// child as it is painted. When false, hit tests are performed
    /// ignoring the transformation.
    ///
    /// [`RenderBox::apply_paint_transform`], and therefore
    /// [`AnyRenderBox::local_to_global`] and [`AnyRenderBox::global_to_local`], always honor
    /// the transformation, regardless of the value of this property.
    pub fn transform_hit_tests(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).transform_hit_tests
    }

    /// Sets [`transform_hit_tests`](Self::transform_hit_tests). Dart's plain field.
    pub fn set_transform_hit_tests(self: RenderHandle<Self>, app: &mut App, value: bool) {
        self.get_mut(app).transform_hit_tests = value;
    }

    /// Sets the matrix to transform the child by during painting.
    ///
    /// There is no getter for the transform, as in Dart, where the matrix is mutable and
    /// mutations outside of the control of the render object could not reliably be reflected
    /// in the rendering.
    pub fn set_transform(self: RenderHandle<Self>, app: &mut App, value: Matrix4) {
        if self.get(app).transform == value {
            return;
        }
        self.get_mut(app).transform = value;
        self.mark_needs_paint(app);
    }

    /// Sets the transform to the identity matrix.
    pub fn set_identity(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).transform = Matrix4::IDENTITY;
        self.mark_needs_paint(app);
    }

    /// Concatenates a rotation about the x axis into the transform.
    pub fn rotate_x(self: RenderHandle<Self>, app: &mut App, radians: f64) {
        let (sin, cos) = (radians.sin() as f32, radians.cos() as f32);
        #[rustfmt::skip]
        let rotation = Matrix4::from_flutter_array(&[
            1.0, 0.0, 0.0, 0.0,
            0.0, cos, sin, 0.0,
            0.0, -sin, cos, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        self.concat(app, &rotation);
    }

    /// Concatenates a rotation about the y axis into the transform.
    pub fn rotate_y(self: RenderHandle<Self>, app: &mut App, radians: f64) {
        let (sin, cos) = (radians.sin() as f32, radians.cos() as f32);
        #[rustfmt::skip]
        let rotation = Matrix4::from_flutter_array(&[
            cos, 0.0, -sin, 0.0,
            0.0, 1.0, 0.0, 0.0,
            sin, 0.0, cos, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        self.concat(app, &rotation);
    }

    /// Concatenates a rotation about the z axis into the transform.
    pub fn rotate_z(self: RenderHandle<Self>, app: &mut App, radians: f64) {
        let (sin, cos) = (radians.sin() as f32, radians.cos() as f32);
        #[rustfmt::skip]
        let rotation = Matrix4::from_flutter_array(&[
            cos, sin, 0.0, 0.0,
            -sin, cos, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        self.concat(app, &rotation);
    }

    /// Concatenates a translation by (x, y, z) into the transform.
    pub fn translate(self: RenderHandle<Self>, app: &mut App, x: f64, y: f64, z: f64) {
        #[rustfmt::skip]
        let translation = Matrix4::from_flutter_array(&[
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            x as f32, y as f32, z as f32, 1.0,
        ]);
        self.concat(app, &translation);
    }

    /// Concatenates a scale into the transform. Dart's optional `y` and `z` default to `x`.
    pub fn scale(self: RenderHandle<Self>, app: &mut App, x: f64, y: Option<f64>, z: Option<f64>) {
        let (y, z) = (y.unwrap_or(x) as f32, z.unwrap_or(x) as f32);
        #[rustfmt::skip]
        let scale = Matrix4::from_flutter_array(&[
            x as f32, 0.0, 0.0, 0.0,
            0.0, y, 0.0, 0.0,
            0.0, 0.0, z, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        self.concat(app, &scale);
    }

    /// vector_math's in-place `rotateX` / `translate` / `scale`: a right multiplication of the
    /// transform, then a repaint.
    fn concat(self: RenderHandle<Self>, app: &mut App, other: &Matrix4) {
        crate::object::multiply(&mut self.get_mut(app).transform, other);
        self.mark_needs_paint(app);
    }

    /// Flutter's `_effectiveTransform`: the transform around [`origin`](Self::origin) and the
    /// resolved [`alignment`](Self::alignment).
    fn effective_transform(self: RenderHandle<Self>, app: &App) -> Matrix4 {
        let this = self.get(app);
        let resolved_alignment = this
            .alignment
            .map(|alignment| alignment.resolve(this.text_direction));
        let origin = this.origin;
        if origin.is_none() && resolved_alignment.is_none() {
            return this.transform;
        }
        let mut result = Matrix4::IDENTITY;
        if let Some(origin) = origin {
            crate::object::translate(&mut result, origin.dx(), origin.dy());
        }
        let translation = resolved_alignment.map(|alignment| alignment.along_size(self.size(app)));
        if let Some(translation) = translation {
            crate::object::translate(&mut result, translation.dx(), translation.dy());
        }
        crate::object::multiply(&mut result, &self.get(app).transform);
        if let Some(translation) = translation {
            crate::object::translate(&mut result, -translation.dx(), -translation.dy());
        }
        if let Some(origin) = origin {
            crate::object::translate(&mut result, -origin.dx(), -origin.dy());
        }
        result
    }
}

impl RenderObjectWithChildMixin for RenderTransform {
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

impl RenderProxyBoxMixin for RenderTransform {}

impl RenderObject for RenderTransform {
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
        if self.child(app).is_none() {
            return;
        }
        let transform = self.effective_transform(app);
        let Some(child_offset) = get_as_translation(transform) else {
            // If the matrix is singular the children would be compressed to a line or
            // single point, instead short-circuit and paint nothing.
            let determinant = transform.determinant();
            if determinant == 0.0 || !determinant.is_finite() {
                self.as_object().set_layer(app, None);
                return;
            }
            let old = self.as_object().layer_as::<TransformLayer>(app);
            let layer = context.push_transform(
                app,
                self.as_object().needs_compositing(app),
                offset,
                transform,
                |app, context, offset| {
                    RenderProxyBoxMixin::paint(self, app, context, offset);
                },
                old,
            );
            self.as_object()
                .set_layer(app, layer.map(|layer| layer.as_container_layer()));
            return;
        };
        RenderProxyBoxMixin::paint(self, app, context, offset + child_offset);
        self.as_object().set_layer(app, None);
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

impl RenderBox for RenderTransform {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        crate::object::multiply(transform, &self.effective_transform(app));
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        // RenderTransform objects don't check if they are
        // themselves hit, because it's confusing to think about
        // how the untransformed size and the child's transformed
        // position interact.
        RenderBox::hit_test_children(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let transform = self
            .transform_hit_tests(app)
            .then(|| self.effective_transform(app));
        result.add_with_paint_transform(transform, position, |result, position| {
            RenderProxyBoxMixin::hit_test_children(self, app, result, position)
        })
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Scales and positions its child within itself according to [`fit`](Self::fit).
pub struct RenderFittedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    fit: BoxFit,
    alignment: AlignmentGeometry,
    text_direction: Option<TextDirection>,
    resolved_alignment: Option<Alignment>,
    clip_behavior: Clip,
    has_visual_overflow: Option<bool>,
    transform: Option<Matrix4>,
}

/// Whether a change of [`BoxFit`] changes the size this box takes.
fn fit_affects_layout(fit: BoxFit) -> bool {
    match fit {
        BoxFit::ScaleDown => true,
        BoxFit::Contain
        | BoxFit::Cover
        | BoxFit::Fill
        | BoxFit::FitHeight
        | BoxFit::FitWidth
        | BoxFit::None => false,
    }
}

impl RenderFittedBox {
    /// Creates a render object that scales and positions its child within itself.
    pub fn new(
        app: &mut App,
        fit: BoxFit,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderFittedBox> {
        let this = RenderHandle::new_box(
            app,
            RenderFittedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                fit,
                alignment,
                text_direction,
                resolved_alignment: None,
                clip_behavior: Clip::None,
                has_visual_overflow: None,
                transform: None,
            },
        );
        this.set_child(app, child);
        this
    }

    fn resolve(self: RenderHandle<Self>, app: &mut App) -> Alignment {
        let this = self.get(app);
        if let Some(resolved) = this.resolved_alignment {
            return resolved;
        }
        let resolved = this.alignment.resolve(this.text_direction);
        self.get_mut(app).resolved_alignment = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_alignment = None;
        self.mark_needs_paint(app);
    }

    /// How to inscribe the child into the space allocated during layout.
    pub fn fit(self: RenderHandle<Self>, app: &App) -> BoxFit {
        self.get(app).fit
    }

    /// Sets [`fit`](Self::fit).
    pub fn set_fit(self: RenderHandle<Self>, app: &mut App, value: BoxFit) {
        let last_fit = self.get(app).fit;
        if last_fit == value {
            return;
        }
        self.get_mut(app).fit = value;
        if fit_affects_layout(last_fit) || fit_affects_layout(value) {
            self.mark_needs_layout(app);
        } else {
            self.clear_paint_data(app);
            self.mark_needs_paint(app);
        }
    }

    /// How to align the child within its parent's bounds.
    ///
    /// An alignment of (0.0, 0.0) aligns the child to the top-left corner of its parent's bounds.
    /// An alignment of (1.0, 0.5) aligns the child to the middle of the right edge of its
    /// parent's bounds.
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> AlignmentGeometry {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: AlignmentGeometry) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.clear_paint_data(app);
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`alignment`](Self::alignment).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.clear_paint_data(app);
        self.mark_need_resolution(app);
    }

    /// How to clip a child that overflows this box. Defaults to [`Clip::None`].
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if self.get(app).clip_behavior == value {
            return;
        }
        self.get_mut(app).clip_behavior = value;
        self.mark_needs_paint(app);
    }

    /// Whether the given child would be painted if [`RenderObject::paint`] were called.
    ///
    /// Flutter's `RenderObject.paintsChild` is a virtual whose default is true; here it is an
    /// inherent method (`PORTING.md`).
    pub fn paints_child(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> bool {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        !self.size(app).is_empty() && !child.size(app).is_empty()
    }

    fn clear_paint_data(self: RenderHandle<Self>, app: &mut App) {
        let this = self.get_mut(app);
        this.has_visual_overflow = None;
        this.transform = None;
    }

    /// Flutter's `_updatePaintData`, without the cache: the transform that fits the child into
    /// this box, and whether the child overflows it.
    fn paint_data(self: RenderHandle<Self>, app: &App) -> (bool, Matrix4) {
        let Some(child) = self.child(app) else {
            return (false, Matrix4::IDENTITY);
        };
        let this = self.get(app);
        let resolved_alignment = this
            .resolved_alignment
            .unwrap_or_else(|| this.alignment.resolve(this.text_direction));
        let child_size = child.size(app);
        let sizes = apply_box_fit(this.fit, child_size, self.size(app));
        let scale_x = sizes.destination.width() / sizes.source.width();
        let scale_y = sizes.destination.height() / sizes.source.height();
        let source_rect = resolved_alignment.inscribe(sizes.source, Offset::ZERO & child_size);
        let destination_rect =
            resolved_alignment.inscribe(sizes.destination, Offset::ZERO & self.size(app));
        debug_assert!(scale_x.is_finite() && scale_y.is_finite());
        let mut transform =
            Matrix4::translation(destination_rect.left as f32, destination_rect.top as f32);
        crate::object::multiply(
            &mut transform,
            &Matrix4::scale(scale_x as f32, scale_y as f32),
        );
        crate::object::translate(&mut transform, -source_rect.left, -source_rect.top);
        let has_visual_overflow =
            source_rect.width() < child_size.width() || source_rect.height() < child_size.height();
        (has_visual_overflow, transform)
    }

    fn update_paint_data(self: RenderHandle<Self>, app: &mut App) {
        if self.get(app).transform.is_some() {
            return;
        }
        // Dart's `_resolve()` memoizes the resolved alignment here.
        self.resolve(app);
        let (has_visual_overflow, transform) = self.paint_data(app);
        let this = self.get_mut(app);
        this.has_visual_overflow = Some(has_visual_overflow);
        this.transform = Some(transform);
    }

    fn paint_child_with_transform(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) -> Option<Handle<TransformLayer>> {
        let transform = self.get(app).transform.expect("update_paint_data ran");
        match get_as_translation(transform) {
            Some(child_offset) => {
                RenderProxyBoxMixin::paint(self, app, context, offset + child_offset);
                None
            }
            None => context.push_transform(
                app,
                self.as_object().needs_compositing(app),
                offset,
                transform,
                |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                self.as_object().layer_as::<TransformLayer>(app),
            ),
        }
    }
}

impl RenderObjectWithChildMixin for RenderFittedBox {
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

impl RenderProxyBoxMixin for RenderFittedBox {}

impl RenderObject for RenderFittedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let Some(child) = self.child(app) else {
            self.set_size(app, constraints.smallest());
            return;
        };
        child.layout(app, BoxConstraints::new(), true);
        let child_size = child.size(app);
        let size = match self.fit(app) {
            BoxFit::ScaleDown => {
                let size_constraints = constraints.loosen();
                let unconstrained_size = size_constraints
                    .constrain_size_and_attempt_to_preserve_aspect_ratio(child_size);
                constraints.constrain(unconstrained_size)
            }
            BoxFit::Contain
            | BoxFit::Cover
            | BoxFit::Fill
            | BoxFit::FitHeight
            | BoxFit::FitWidth
            | BoxFit::None => {
                constraints.constrain_size_and_attempt_to_preserve_aspect_ratio(child_size)
            }
        };
        self.set_size(app, size);
        self.clear_paint_data(app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        if self.size(app).is_empty() || child.size(app).is_empty() {
            return;
        }
        self.update_paint_data(app);
        let clip_behavior = self.clip_behavior(app);
        if self.get(app).has_visual_overflow == Some(true) && clip_behavior != Clip::None {
            let clip_rect = Offset::ZERO & self.size(app);
            let old = self.as_object().layer_as::<ClipRectLayer>(app);
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                clip_rect,
                |app, context, offset| {
                    let _ = self.paint_child_with_transform(app, context, offset);
                },
                clip_behavior,
                old,
            );
            self.as_object()
                .set_layer(app, layer.map(|layer| layer.as_container_layer()));
        } else {
            let layer = self.paint_child_with_transform(app, context, offset);
            self.as_object()
                .set_layer(app, layer.map(|layer| layer.as_container_layer()));
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

impl RenderBox for RenderFittedBox {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let child = child.as_box().expect("a fitted box has a box child");
        if !self.paints_child(app, child) {
            *transform = crate::object::zero_matrix();
            return;
        }
        let (_, fitted) = self.paint_data(app);
        crate::object::multiply(transform, &fitted);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let child_is_empty = self
            .child(app)
            .is_none_or(|child| child.size(app).is_empty());
        if self.size(app).is_empty() || child_is_empty {
            return false;
        }
        self.update_paint_data(app);
        let transform = self.get(app).transform;
        result.add_with_paint_transform(transform, position, |result, position| {
            RenderProxyBoxMixin::hit_test_children(self, app, result, position)
        })
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let Some(child) = self.child(app) else {
            return constraints.smallest();
        };
        let child_size = child.get_dry_layout(app, BoxConstraints::new());
        match self.fit(app) {
            BoxFit::ScaleDown => {
                let size_constraints = constraints.loosen();
                let unconstrained_size = size_constraints
                    .constrain_size_and_attempt_to_preserve_aspect_ratio(child_size);
                constraints.constrain(unconstrained_size)
            }
            BoxFit::Contain
            | BoxFit::Cover
            | BoxFit::Fill
            | BoxFit::FitHeight
            | BoxFit::FitWidth
            | BoxFit::None => {
                constraints.constrain_size_and_attempt_to_preserve_aspect_ratio(child_size)
            }
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        _constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        // The baseline of the child with unconstrained layout, without applying any transforms
        // or scaling that would be applied during paint.
        self.child(app)?
            .get_dry_baseline(app, BoxConstraints::new(), baseline)
    }
}

/// Applies a translation transformation before painting its child.
///
/// The translation is expressed as an [`Offset`] scaled to the child's size. For
/// example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
/// translation of one quarter the width of the child.
///
/// Hit tests will only be detected inside the bounds of the
/// [`RenderFractionalTranslation`], even if the contents are offset such that
/// they overflow.
pub struct RenderFractionalTranslation {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    translation: Offset,
    transform_hit_tests: bool,
}

impl RenderFractionalTranslation {
    /// Creates a render object that translates its child's painting.
    pub fn new(
        app: &mut App,
        translation: Offset,
        transform_hit_tests: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderFractionalTranslation> {
        let this = RenderHandle::new_box(
            app,
            RenderFractionalTranslation {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                translation,
                transform_hit_tests,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The translation to apply to the child, scaled to the child's size.
    ///
    /// For example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
    /// translation of one quarter the width of the child.
    pub fn translation(self: RenderHandle<Self>, app: &App) -> Offset {
        self.get(app).translation
    }

    /// Sets [`translation`](Self::translation).
    pub fn set_translation(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if self.get(app).translation == value {
            return;
        }
        self.get_mut(app).translation = value;
        self.mark_needs_paint(app);
    }

    /// When true, hit tests are performed based on the position of the
    /// child as it is painted. When false, hit tests are performed
    /// ignoring the transformation.
    ///
    /// [`RenderBox::apply_paint_transform`], and therefore
    /// [`AnyRenderBox::local_to_global`] and [`AnyRenderBox::global_to_local`], always honor
    /// the transformation, regardless of the value of this property.
    pub fn transform_hit_tests(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).transform_hit_tests
    }

    /// Sets [`transform_hit_tests`](Self::transform_hit_tests). Dart's plain field.
    pub fn set_transform_hit_tests(self: RenderHandle<Self>, app: &mut App, value: bool) {
        self.get_mut(app).transform_hit_tests = value;
    }

    /// The translation in layout pixels: [`translation`](Self::translation) along the size.
    fn absolute_translation(self: RenderHandle<Self>, app: &App) -> Offset {
        let translation = self.get(app).translation;
        let size = self.size(app);
        Offset::new(
            translation.dx() * size.width(),
            translation.dy() * size.height(),
        )
    }
}

impl RenderObjectWithChildMixin for RenderFractionalTranslation {
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

impl RenderProxyBoxMixin for RenderFractionalTranslation {}

impl RenderObject for RenderFractionalTranslation {
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
        debug_assert!(!self.debug_needs_layout(app));
        if self.child(app).is_none() {
            return;
        }
        let translation = self.absolute_translation(app);
        RenderProxyBoxMixin::paint(self, app, context, offset + translation);
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

impl RenderBox for RenderFractionalTranslation {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let translation = self.absolute_translation(app);
        crate::object::translate(transform, translation.dx(), translation.dy());
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        // RenderFractionalTranslation objects don't check if they are
        // themselves hit, because it's confusing to think about
        // how the untransformed size and the child's transformed
        // position interact.
        RenderBox::hit_test_children(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        debug_assert!(!self.debug_needs_layout(app));
        let offset = self
            .transform_hit_tests(app)
            .then(|| self.absolute_translation(app));
        result.add_with_paint_offset(offset, position, |result, position| {
            RenderProxyBoxMixin::hit_test_children(self, app, result, position)
        })
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// A render object that is invisible during hit testing.
///
/// When [`ignoring`](Self::ignoring) is true, this render object (and its subtree) is invisible
/// to hit testing. It still consumes space during layout and paints its child
/// as usual. It just cannot be the target of located events, because its render
/// object returns false from [`RenderBox::hit_test`].
///
/// See also:
///
///  * [`RenderAbsorbPointer`], which takes the pointer events but prevents any
///    nodes in the subtree from seeing them.
pub struct RenderIgnorePointer {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    ignoring: bool,
}

impl RenderIgnorePointer {
    /// Creates a render object that is invisible to hit testing.
    pub fn new(
        app: &mut App,
        ignoring: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderIgnorePointer> {
        let this = RenderHandle::new_box(
            app,
            RenderIgnorePointer {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                ignoring,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether this render object is ignored during hit testing.
    ///
    /// Regardless of whether this render object is ignored during hit testing, it
    /// will still consume space during layout and be visible during painting.
    pub fn ignoring(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).ignoring
    }

    /// Sets [`ignoring`](Self::ignoring).
    pub fn set_ignoring(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).ignoring {
            return;
        }
        self.get_mut(app).ignoring = value;
    }
}

impl RenderObjectWithChildMixin for RenderIgnorePointer {
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

impl RenderProxyBoxMixin for RenderIgnorePointer {}

impl RenderObject for RenderIgnorePointer {
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

impl RenderBox for RenderIgnorePointer {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        !self.ignoring(app) && RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Lays the child out as if it was in the tree, but without painting anything,
/// without making the child available for hit testing, and without taking any
/// room in the parent.
pub struct RenderOffstage {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    offstage: bool,
}

impl RenderOffstage {
    /// Creates an offstage render object.
    pub fn new(
        app: &mut App,
        offstage: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderOffstage> {
        let this = RenderHandle::new_box(
            app,
            RenderOffstage {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                offstage,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether the child is hidden from the rest of the tree.
    ///
    /// If true, the child is laid out as if it was in the tree, but without
    /// painting anything, without making the child available for hit testing, and
    /// without taking any room in the parent.
    ///
    /// If false, the child is included in the tree as normal.
    pub fn offstage(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).offstage
    }

    /// Sets [`offstage`](Self::offstage).
    pub fn set_offstage(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).offstage {
            return;
        }
        self.get_mut(app).offstage = value;
        self.as_object()
            .mark_needs_layout_for_sized_by_parent_change(app);
    }

    /// Whether the given child would be painted if [`RenderObject::paint`] were called.
    ///
    /// Flutter's `RenderObject.paintsChild` is a virtual whose default is true; here only this
    /// override exists, as an inherent method (`PORTING.md`).
    pub fn paints_child(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> bool {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        !self.get(app).offstage
    }
}

impl RenderObjectWithChildMixin for RenderOffstage {
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

impl RenderProxyBoxMixin for RenderOffstage {}

impl RenderObject for RenderOffstage {
    crate::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).offstage
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        if !self.get(app).offstage {
            RenderProxyBoxMixin::perform_layout(self, app);
            return;
        }
        if let Some(child) = self.child(app) {
            let constraints = self.constraints(app);
            child.layout(app, constraints, false);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.get(app).offstage {
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

impl RenderBox for RenderOffstage {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        !self.get(app).offstage && RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if self.get(app).offstage {
            return 0.0;
        }
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if self.get(app).offstage {
            return 0.0;
        }
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if self.get(app).offstage {
            return 0.0;
        }
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if self.get(app).offstage {
            return 0.0;
        }
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        if self.get(app).offstage {
            return None;
        }
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        if self.get(app).offstage {
            return None;
        }
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if self.get(app).offstage {
            return constraints.smallest();
        }
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }

    /// Flutter's `performResize` asserts and calls `super.performResize`.
    fn perform_resize(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(self.get(app).offstage);
        RenderBoxBase::perform_resize(self, app);
    }
}

/// A render object that absorbs pointers during hit testing.
///
/// When [`absorbing`](Self::absorbing) is true, this render object prevents its subtree from
/// receiving pointer events by terminating hit testing at itself. It still
/// consumes space during layout and paints its child as usual. It just prevents
/// its children from being the target of located events, because its render
/// object returns true from [`RenderBox::hit_test`].
///
/// See also:
///
///  * [`RenderIgnorePointer`], which has the opposite effect: removing the
///    subtree from considering entirely for the purposes of hit testing.
pub struct RenderAbsorbPointer {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    absorbing: bool,
}

impl RenderAbsorbPointer {
    /// Creates a render object that absorbs pointers during hit testing.
    pub fn new(
        app: &mut App,
        absorbing: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderAbsorbPointer> {
        let this = RenderHandle::new_box(
            app,
            RenderAbsorbPointer {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                absorbing,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether this render object absorbs pointers during hit testing.
    ///
    /// Regardless of whether this render object absorbs pointers during hit
    /// testing, it will still consume space during layout and be visible during
    /// painting.
    pub fn absorbing(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).absorbing
    }

    /// Sets [`absorbing`](Self::absorbing).
    pub fn set_absorbing(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).absorbing == value {
            return;
        }
        self.get_mut(app).absorbing = value;
    }
}

impl RenderObjectWithChildMixin for RenderAbsorbPointer {
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

impl RenderProxyBoxMixin for RenderAbsorbPointer {}

impl RenderObject for RenderAbsorbPointer {
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

impl RenderBox for RenderAbsorbPointer {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if self.absorbing(app) {
            return self.size(app).contains(position);
        }
        RenderBoxBase::hit_test(self, app, result, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Paints its area with a specified [`Color`] and then draws its child on top of that color.
///
/// Flutter's `_RenderColoredBox`, the render object of the `ColoredBox` widget, in
/// `widgets/basic.dart`.
pub struct RenderColoredBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    color: Color,
    is_anti_alias: bool,
    behavior: HitTestBehavior,
}

impl RenderColoredBox {
    /// Creates a render object that paints its area with `color`.
    pub fn new(
        app: &mut App,
        color: Color,
        is_anti_alias: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderColoredBox> {
        let this = RenderHandle::new_box(
            app,
            RenderColoredBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                color,
                is_anti_alias,
                behavior: HitTestBehavior::Opaque,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The fill color for this render object.
    pub fn color(self: RenderHandle<Self>, app: &App) -> Color {
        self.get(app).color
    }

    /// Sets [`color`](Self::color).
    pub fn set_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if value == self.get(app).color {
            return;
        }
        self.get_mut(app).color = value;
        self.mark_needs_paint(app);
    }

    /// Whether to apply anti-aliasing when painting the box.
    pub fn is_anti_alias(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_anti_alias
    }

    /// Sets [`is_anti_alias`](Self::is_anti_alias).
    pub fn set_is_anti_alias(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).is_anti_alias {
            return;
        }
        self.get_mut(app).is_anti_alias = value;
        self.mark_needs_paint(app);
    }
}

impl RenderObjectWithChildMixin for RenderColoredBox {
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

impl RenderProxyBoxMixin for RenderColoredBox {}

impl RenderProxyBoxWithHitTestBehavior for RenderColoredBox {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.get(app).behavior
    }
}

impl RenderObject for RenderColoredBox {
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
        // It's tempting to want to optimize out this `draw_rect()` call if the
        // color is transparent (alpha==0), but doing so would be incorrect. See
        // https://github.com/flutter/flutter/pull/72526#issuecomment-749185938 for
        // a good description of why.
        let size = self.size(app);
        if size.gt(&Size::ZERO) {
            let paint = Paint {
                color: self.get(app).color.into(),
                ..Paint::default()
            };
            context.canvas().draw_rect(offset & size, &paint);
        }
        if let Some(child) = self.child(app) {
            context.paint_child(app, child.as_object(), offset);
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

impl RenderBox for RenderColoredBox {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Holds opaque meta data in the render tree.
///
/// Useful for decorating the render tree with information that will be consumed
/// later. For example, you could store information in the render tree that will
/// be used when the user interacts with the render tree but has no visual impact
/// prior to the interaction.
pub struct RenderMetaData {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    meta_data: Option<Rc<dyn Any>>,
    behavior: HitTestBehavior,
}

impl RenderMetaData {
    /// Creates a render object that hold opaque meta data.
    ///
    /// The `behavior` argument defaults to [`HitTestBehavior::DeferToChild`].
    pub fn new(
        app: &mut App,
        meta_data: Option<Rc<dyn Any>>,
        behavior: HitTestBehavior,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderMetaData {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                meta_data,
                behavior,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Opaque meta data ignored by the render tree.
    pub fn meta_data(self: RenderHandle<Self>, app: &App) -> Option<Rc<dyn Any>> {
        self.get(app).meta_data.clone()
    }

    /// Sets [`meta_data`](Self::meta_data).
    pub fn set_meta_data(self: RenderHandle<Self>, app: &mut App, value: Option<Rc<dyn Any>>) {
        self.get_mut(app).meta_data = value;
    }

    /// Sets [`behavior`](RenderProxyBoxWithHitTestBehavior::behavior).
    pub fn set_behavior(self: RenderHandle<Self>, app: &mut App, value: HitTestBehavior) {
        self.get_mut(app).behavior = value;
    }
}

impl RenderObjectWithChildMixin for RenderMetaData {
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

impl RenderProxyBoxMixin for RenderMetaData {}

impl RenderProxyBoxWithHitTestBehavior for RenderMetaData {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.get(app).behavior
    }
}

impl RenderObject for RenderMetaData {
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

impl RenderBox for RenderMetaData {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Applies a filter to the existing painted content and then paints `child`.
///
/// This effect is relatively expensive, especially if the filter is non-local,
/// such as a blur.
pub struct RenderBackdropFilter {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    filter_config: ImageFilterConfig,
    blend_mode: BlendMode,
    enabled: bool,
    backdrop_key: Option<BackdropKey>,
}

impl RenderBackdropFilter {
    /// Creates a backdrop filter.
    ///
    /// The `blend_mode` argument defaults to [`BlendMode::SrcOver`].
    pub fn new(
        app: &mut App,
        filter_config: ImageFilterConfig,
        blend_mode: BlendMode,
        enabled: bool,
        backdrop_key: Option<BackdropKey>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderBackdropFilter {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                filter_config,
                blend_mode,
                enabled,
                backdrop_key,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether or not the backdrop filter operation will be applied to the child.
    pub fn enabled(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).enabled
    }

    /// Sets [`enabled`](Self::enabled).
    pub fn set_enabled(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).enabled == value {
            return;
        }
        self.get_mut(app).enabled = value;
        self.mark_needs_paint(app);
    }

    /// The image filter to apply to the existing painted content before painting the child.
    ///
    /// Dart's deprecated `filter` getter: only valid when the configuration was set through
    /// [`set_filter`](Self::set_filter).
    pub fn filter(self: RenderHandle<Self>, app: &App) -> ImageFilter {
        self.get(app)
            .filter_config
            .filter()
            .expect(
                "This getter should only be called when the filter is assigned via the \
                 `filter` setter.",
            )
            .clone()
    }

    /// Sets the filter as a direct configuration (Dart's `filter` setter).
    pub fn set_filter(self: RenderHandle<Self>, app: &mut App, value: ImageFilter) {
        self.set_filter_config(app, ImageFilterConfig::new(value));
    }

    /// The configuration that resolves the filter against the paint bounds.
    pub fn filter_config(self: RenderHandle<Self>, app: &App) -> &ImageFilterConfig {
        &self.get(app).filter_config
    }

    /// Sets [`filter_config`](Self::filter_config).
    pub fn set_filter_config(self: RenderHandle<Self>, app: &mut App, value: ImageFilterConfig) {
        if self.get(app).filter_config == value {
            return;
        }
        self.get_mut(app).filter_config = value;
        self.mark_needs_paint(app);
    }

    /// The blend mode to use to apply the filtered background content onto the background
    /// surface.
    pub fn blend_mode(self: RenderHandle<Self>, app: &App) -> BlendMode {
        self.get(app).blend_mode
    }

    /// Sets [`blend_mode`](Self::blend_mode).
    pub fn set_blend_mode(self: RenderHandle<Self>, app: &mut App, value: BlendMode) {
        if self.get(app).blend_mode == value {
            return;
        }
        self.get_mut(app).blend_mode = value;
        self.mark_needs_paint(app);
    }

    /// The backdrop key that identifies the shared backdrop this filter belongs to.
    pub fn backdrop_key(self: RenderHandle<Self>, app: &App) -> Option<BackdropKey> {
        self.get(app).backdrop_key
    }

    /// Sets [`backdrop_key`](Self::backdrop_key).
    pub fn set_backdrop_key(self: RenderHandle<Self>, app: &mut App, value: Option<BackdropKey>) {
        if self.get(app).backdrop_key == value {
            return;
        }
        self.get_mut(app).backdrop_key = value;
        self.mark_needs_paint(app);
    }
}

impl RenderObjectWithChildMixin for RenderBackdropFilter {
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

impl RenderProxyBoxMixin for RenderBackdropFilter {}

impl RenderObject for RenderBackdropFilter {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some()
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
        if !self.get(app).enabled {
            RenderProxyBoxMixin::paint(self, app, context, offset);
            return;
        }
        let bounds = offset & self.size(app);
        let effective_filter = self
            .get(app)
            .filter_config
            .resolve(ImageFilterContext { bounds });
        if self.child(app).is_some() {
            debug_assert!(self.as_object().needs_compositing(app));
            let (blend_mode, backdrop_key) = {
                let this = self.get(app);
                (this.blend_mode, this.backdrop_key)
            };
            let layer = match self.as_object().layer_as::<BackdropFilterLayer>(app) {
                Some(layer) => layer,
                None => {
                    let layer = BackdropFilterLayer::new(app);
                    self.as_object()
                        .set_layer(app, Some(layer.as_container_layer()));
                    layer
                }
            };
            layer.set_filter(app, Some(effective_filter));
            layer.set_blend_mode(app, blend_mode);
            layer.set_backdrop_key(app, backdrop_key);
            context.push_layer(
                app,
                layer.as_container_layer(),
                |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
                offset,
                None,
            );
        } else {
            self.as_object().set_layer(app, None);
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

impl RenderBox for RenderBackdropFilter {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Annotates a region of the layer tree with a value.
///
/// The value can be retrieved with [`AnyLayer::find`](crate::AnyLayer::find) at a
/// position; `AnnotatedRegion` is the widget that inserts one.
pub struct RenderAnnotatedRegion<T> {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    value: Rc<T>,
    sized: bool,
    layer_handle: LayerHandle<Handle<AnnotatedRegionLayer>>,
}

impl<T: PartialEq + 'static> RenderAnnotatedRegion<T> {
    /// Creates a new [`RenderAnnotatedRegion`] to insert `value` into the layer tree.
    ///
    /// If `sized` is true, the layer is provided with the size of this render object to clip
    /// the results of [`AnyLayer::find`](crate::AnyLayer::find).
    pub fn new(
        app: &mut App,
        value: Rc<T>,
        sized: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderAnnotatedRegion<T>> {
        let this = RenderHandle::new_box(
            app,
            RenderAnnotatedRegion {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                value,
                sized,
                layer_handle: LayerHandle::new(),
            },
        );
        this.set_child(app, child);
        this
    }

    /// A value which can be retrieved using
    /// [`AnyLayer::find`](crate::AnyLayer::find).
    pub fn value(self: RenderHandle<Self>, app: &App) -> Rc<T> {
        Rc::clone(&self.get(app).value)
    }

    /// Sets [`value`](Self::value).
    pub fn set_value(self: RenderHandle<Self>, app: &mut App, new_value: Rc<T>) {
        if *self.get(app).value == *new_value {
            return;
        }
        self.get_mut(app).value = new_value;
        self.mark_needs_paint(app);
    }

    /// Whether the render object will pass its size to the annotated region.
    pub fn sized(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).sized
    }

    /// Sets [`sized`](Self::sized).
    pub fn set_sized(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).sized == value {
            return;
        }
        self.get_mut(app).sized = value;
        self.mark_needs_paint(app);
    }
}

impl<T: PartialEq + 'static> RenderObjectWithChildMixin for RenderAnnotatedRegion<T> {
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

impl<T: PartialEq + 'static> RenderProxyBoxMixin for RenderAnnotatedRegion<T> {}

impl<T: PartialEq + 'static> RenderObject for RenderAnnotatedRegion<T> {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
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
        let sized = self.sized(app);
        let value: Rc<dyn Any> = self.value(app);
        let layer = AnnotatedRegionLayer::new(app, value);
        if sized {
            layer.set_size(app, Some(self.size(app)));
            layer.set_offset(app, offset);
        }
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).layer_handle, Some(layer));
        context.push_layer(
            app,
            layer.as_container_layer(),
            |app, context, offset| RenderProxyBoxMixin::paint(self, app, context, offset),
            offset,
            None,
        );
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).layer_handle, None);
        crate::object::RenderObjectBase::dispose(self, app);
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

impl<T: PartialEq + 'static> RenderBox for RenderAnnotatedRegion<T> {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

#[cfg(test)]
mod tests {
    use inset_animation::{k_always_complete_animation, k_always_dismissed_animation};
    use inset_embedder::ColorFilter;
    use inset_embedder::valo::{self, Op};
    use inset_foundation::AppCell;
    use inset_foundation::ValueNotifier;
    use inset_gestures::HitTestResult;
    use inset_painting::BoxDecoration;

    use super::*;
    use crate::layer::{ErasedLayer, OffsetLayer, PictureLayer};
    use crate::pipeline_owner::PipelineOwner;
    use inset_embedder::SceneBuilder;

    fn sized_box(app: &mut App, size: Size) -> RenderHandle<RenderConstrainedBox> {
        RenderConstrainedBox::new(app, BoxConstraints::tight(size), None)
    }

    /// A leaf that reports fixed intrinsic dimensions and takes the size its constraints allow
    /// around them.
    struct IntrinsicTestBox {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        intrinsic: Size,
    }

    impl RenderObject for IntrinsicTestBox {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let intrinsic = self.get(app).intrinsic;
            let size = self.constraints(app).constrain(intrinsic);
            self.set_size(app, size);
        }
    }

    impl RenderBox for IntrinsicTestBox {
        crate::render_box_accessors!();

        fn compute_min_intrinsic_width(
            self: RenderHandle<Self>,
            app: &mut App,
            _height: f64,
        ) -> f64 {
            self.get(app).intrinsic.width()
        }

        fn compute_max_intrinsic_width(
            self: RenderHandle<Self>,
            app: &mut App,
            _height: f64,
        ) -> f64 {
            self.get(app).intrinsic.width()
        }

        fn compute_min_intrinsic_height(
            self: RenderHandle<Self>,
            app: &mut App,
            _width: f64,
        ) -> f64 {
            self.get(app).intrinsic.height()
        }

        fn compute_max_intrinsic_height(
            self: RenderHandle<Self>,
            app: &mut App,
            _width: f64,
        ) -> f64 {
            self.get(app).intrinsic.height()
        }

        fn compute_dry_layout(
            self: RenderHandle<Self>,
            app: &mut App,
            constraints: BoxConstraints,
        ) -> Size {
            constraints.constrain(self.get(app).intrinsic)
        }
    }

    fn intrinsic_box(app: &mut App, intrinsic: Size) -> RenderHandle<IntrinsicTestBox> {
        RenderHandle::new_box(
            app,
            IntrinsicTestBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                intrinsic,
            },
        )
    }

    fn schedule_root_paint(app: &mut App, node: AnyRenderObject) {
        let root = OffsetLayer::new(app, Offset::ZERO);
        root.as_layer().attach(app, node.id());
        node.schedule_initial_paint(app, root.as_container_layer());
    }

    /// The test binding's first frame: attach, lay the root out, schedule and flush paint.
    fn first_frame(app: &mut App, root: AnyRenderBox) -> Handle<PipelineOwner> {
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.layout(app, BoxConstraints::tight(Size::new(100.0, 100.0)), false);
        schedule_root_paint(app, root.as_object());
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
        owner
    }

    fn pump_frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
    }

    /// `box_test.dart`: a `RenderConstrainedBox` reports its child's intrinsics enforced by its
    /// additional constraints, and a tight axis reports that tight extent.
    #[test]
    fn constrained_box_intrinsics_enforce_the_additional_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = intrinsic_box(&mut app, Size::new(20.0, 30.0));
        let loose = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::new().min_width(50.0).max_width(60.0),
            Some(child.as_box()),
        );
        assert_eq!(loose.as_box().get_min_intrinsic_width(&mut app, 0.0), 50.0);
        assert_eq!(loose.as_box().get_max_intrinsic_width(&mut app, 0.0), 50.0);
        assert_eq!(loose.as_box().get_min_intrinsic_height(&mut app, 0.0), 30.0);

        let child = intrinsic_box(&mut app, Size::new(20.0, 30.0));
        let tight = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(80.0, 90.0)),
            Some(child.as_box()),
        );
        assert_eq!(tight.as_box().get_max_intrinsic_width(&mut app, 0.0), 80.0);
        assert_eq!(tight.as_box().get_max_intrinsic_height(&mut app, 0.0), 90.0);
    }

    /// `intrinsic_width_test.dart`: the child is sized to its maximum intrinsic width, snapped to
    /// `step_width` when there is one.
    #[test]
    fn intrinsic_width_sizes_the_child_to_its_max_intrinsic_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = intrinsic_box(&mut app, Size::new(100.0, 40.0));
        let intrinsic = RenderIntrinsicWidth::new(&mut app, None, None, Some(child.as_box()));
        intrinsic.layout(&mut app, BoxConstraints::new().max_width(500.0), false);
        assert_eq!(intrinsic.size(&app), Size::new(100.0, 40.0));
        assert_eq!(child.size(&app), Size::new(100.0, 40.0));

        intrinsic.set_step_width(&mut app, Some(30.0));
        intrinsic.layout(&mut app, BoxConstraints::new().max_width(500.0), false);
        assert_eq!(intrinsic.size(&app), Size::new(120.0, 40.0));
    }

    /// The dry layout of a `RenderIntrinsicWidth` is the size it lays out to.
    #[test]
    fn intrinsic_width_dry_layout_matches_its_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = intrinsic_box(&mut app, Size::new(100.0, 40.0));
        let intrinsic = RenderIntrinsicWidth::new(&mut app, None, None, Some(child.as_box()));
        let constraints = BoxConstraints::new().max_width(500.0);
        let dry = intrinsic.as_box().get_dry_layout(&mut app, constraints);
        intrinsic.layout(&mut app, constraints, false);
        assert_eq!(dry, intrinsic.size(&app));
    }

    /// `intrinsic_height_test.dart`: the child is sized to its maximum intrinsic height.
    #[test]
    fn intrinsic_height_sizes_the_child_to_its_max_intrinsic_height() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = intrinsic_box(&mut app, Size::new(100.0, 40.0));
        let intrinsic = RenderIntrinsicHeight::new(&mut app, Some(child.as_box()));
        intrinsic.layout(&mut app, BoxConstraints::new().max_height(500.0), false);
        assert_eq!(intrinsic.size(&app), Size::new(100.0, 40.0));
        assert_eq!(
            intrinsic
                .as_box()
                .get_max_intrinsic_width(&mut app, f64::INFINITY),
            100.0
        );
    }

    /// `fitted_box_test.dart`: a `BoxFit::Contain` child is scaled into the box, and the scale
    /// shows up in the paint transform.
    #[test]
    fn fitted_box_scales_its_child_into_itself() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = sized_box(&mut app, Size::new(100.0, 100.0));
        let fitted = RenderFittedBox::new(
            &mut app,
            BoxFit::Contain,
            AlignmentGeometry::CENTER,
            None,
            Some(child.as_box()),
        );
        fitted.layout(
            &mut app,
            BoxConstraints::tight(Size::new(50.0, 50.0)),
            false,
        );
        assert_eq!(fitted.size(&app), Size::new(50.0, 50.0));
        assert_eq!(child.size(&app), Size::new(100.0, 100.0));

        let mut transform = Matrix4::IDENTITY;
        RenderBox::apply_paint_transform(fitted, &app, child.as_object(), &mut transform);
        assert_eq!(
            inset_painting::transform_point(&transform, Offset::new(100.0, 100.0)),
            Offset::new(50.0, 50.0)
        );
    }

    /// `aspect_ratio_test.dart`: the widest width the constraints allow, with the height from the
    /// ratio.
    #[test]
    fn aspect_ratio_sizes_itself_from_the_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = sized_box(&mut app, Size::new(10.0, 10.0));
        let aspect = RenderAspectRatio::new(&mut app, 2.0, Some(child.as_box()));
        aspect.layout(
            &mut app,
            BoxConstraints::new().max_width(200.0).max_height(200.0),
            false,
        );
        assert_eq!(aspect.size(&app), Size::new(200.0, 100.0));
        assert_eq!(child.size(&app), Size::new(200.0, 100.0));
        assert_eq!(
            aspect.as_box().get_min_intrinsic_width(&mut app, 50.0),
            100.0
        );
        assert_eq!(
            aspect.as_box().get_min_intrinsic_height(&mut app, 50.0),
            25.0
        );
    }

    /// An aspect ratio box infers its width from the height when the width is unbounded.
    #[test]
    fn aspect_ratio_infers_the_width_from_an_unbounded_axis() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let aspect = RenderAspectRatio::new(&mut app, 0.5, None);
        aspect.layout(&mut app, BoxConstraints::new().max_height(100.0), false);
        assert_eq!(aspect.size(&app), Size::new(50.0, 100.0));
    }

    #[test]
    fn constrained_box_without_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let box_ =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(box_.size(&app), Size::new(80.0, 40.0));
    }

    #[test]
    fn additional_constraints_change_marks_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        for (opacity, composites) in [(0.0, false), (1.0, true), (0.1, true)] {
            let child = sized_box(&mut app, Size::new(1.0, 1.0));
            let opacity = RenderOpacity::new(&mut app, opacity, Some(child.as_box()));
            assert_eq!(opacity.as_object().is_repaint_boundary(&app), composites);
        }
    }

    /// `layers_test.dart`: `non-painted layers are detached`.
    #[test]
    fn non_painted_layers_are_detached() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        assert!(layer.as_layer().attached(&app), "this time it painted");

        opacity.set_opacity(&mut app, 0.0);
        pump_frame(&mut app, owner);
        assert!(inner.as_object().debug_layer(&app).is_none());
        let layer = boundary.as_object().debug_layer(&app).expect("kept");
        assert!(!layer.as_layer().attached(&app), "this time it did not");

        opacity.set_opacity(&mut app, 0.5);
        pump_frame(&mut app, owner);
        let layer = boundary.as_object().debug_layer(&app).expect("kept");
        assert!(layer.as_layer().attached(&app), "this time it did again");
    }

    #[test]
    fn animated_opacity_follows_its_animation() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let opacity = app
            .handle::<OpacityLayer>(layer.id())
            .expect("an OpacityLayer");
        assert_eq!(opacity.alpha(&app), Some(255));
    }

    #[test]
    fn decorated_box_paints_its_decoration() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let decorated = RenderDecoratedBox::new(
            &mut app,
            Box::new(BoxDecoration::new().color(Color::from_argb(255, 255, 0, 0))),
            DecorationPosition::Background,
            ImageConfiguration::EMPTY,
            None,
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(decorated.as_box()));
        first_frame(&mut app, root.as_box());
        let ops = scene_ops(&mut app, root.as_object());
        assert!(
            ops.iter()
                .any(|op| matches!(op, inset_embedder::valo::Op::DrawDisplayList { .. })),
            "{ops:?}"
        );
    }

    /// A leaf that fills the constraints it is given and is hit anywhere within them.
    fn opaque_leaf(app: &mut App) -> RenderHandle<RenderPointerListener> {
        RenderPointerListener::new(app, HitTestBehavior::Opaque, None)
    }

    const RED: Color = Color::from_argb(255, 255, 0, 0);
    const BLUE: Color = Color::from_argb(255, 0, 0, 255);

    fn red_box(app: &mut App) -> RenderHandle<RenderColoredBox> {
        RenderColoredBox::new(app, RED, true, None)
    }

    /// Hit tests `target` at `position`: whether the hit was absorbed, and the path's length.
    fn hit_test(app: &mut App, target: AnyRenderBox, position: Offset) -> (bool, usize) {
        let mut result = HitTestResult::new();
        let is_hit = target.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
        (is_hit, result.path().len())
    }

    /// Every canvas operation of the pictures in a repaint boundary's layer tree.
    fn picture_ops(app: &App, boundary: AnyRenderObject) -> Vec<Op> {
        let layer = boundary.debug_layer(app).expect("painted");
        layer
            .depth_first_iterate_children(app)
            .into_iter()
            .filter_map(|child| {
                app.handle::<PictureLayer>(child.id())
                    .and_then(|picture| picture.picture(app).map(|p| p.ops().to_vec()))
            })
            .flatten()
            .collect()
    }

    fn picture_has_clip(app: &App, boundary: AnyRenderObject) -> bool {
        picture_ops(app, boundary)
            .iter()
            .any(|op| matches!(op, Op::ClipPath { .. }))
    }

    /// The rect and color of every fill in a repaint boundary's recording.
    fn drawn_rects(app: &App, boundary: AnyRenderObject) -> Vec<(valo::Rect, valo::Color)> {
        picture_ops(app, boundary)
            .iter()
            .filter_map(|op| match op {
                Op::DrawRect { rect, paint, .. } => Some((*rect, paint.color)),
                _ => None,
            })
            .collect()
    }

    fn fill(rect: Rect, color: Color) -> (valo::Rect, valo::Color) {
        (rect.into(), color.into())
    }

    #[test]
    fn limited_box_limits_only_unbounded_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let limited = RenderLimitedBox::new(&mut app, 100.0, 50.0, Some(child.as_box()));

        limited.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(limited.size(&app), Size::new(100.0, 50.0));

        limited.layout(
            &mut app,
            BoxConstraints::loose(Size::new(30.0, 30.0)),
            false,
        );
        assert_eq!(
            limited.size(&app),
            Size::new(30.0, 30.0),
            "a bounded constraint is left alone"
        );
    }

    #[test]
    fn limited_box_without_child_is_empty() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let limited = RenderLimitedBox::new(&mut app, 100.0, 50.0, None);
        limited.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(limited.size(&app), Size::ZERO);
    }

    /// Clips to the left half of the box, so a test can tell the clip from the bounds.
    struct LeftHalfClipper;

    impl CustomClipper<Rect> for LeftHalfClipper {
        fn get_clip(&self, size: Size) -> Rect {
            Rect::from_ltwh(0.0, 0.0, size.width() / 2.0, size.height())
        }

        fn should_reclip(&self, _old_clipper: &dyn CustomClipper<Rect>) -> bool {
            false
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl CustomClipper<RRect> for LeftHalfClipper {
        fn get_clip(&self, size: Size) -> RRect {
            RRect::from_rect_xy(
                Rect::from_ltwh(0.0, 0.0, size.width() / 2.0, size.height()),
                0.0,
                0.0,
            )
        }

        fn should_reclip(&self, _old_clipper: &dyn CustomClipper<RRect>) -> bool {
            false
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// A clipper of the whole box whose `reclip` listenable a test can notify.
    struct NotifyingClipper {
        reclip: Rc<dyn Listenable>,
    }

    impl CustomClipper<Rect> for NotifyingClipper {
        fn reclip(&self) -> Option<&Rc<dyn Listenable>> {
            Some(&self.reclip)
        }

        fn get_clip(&self, size: Size) -> Rect {
            Offset::ZERO & size
        }

        fn should_reclip(&self, _old_clipper: &dyn CustomClipper<Rect>) -> bool {
            false
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn clip_rect_pushes_its_clipper_and_clips_hit_tests() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let clipper: Rc<dyn CustomClipper<Rect>> = Rc::new(LeftHalfClipper);
        let clip = RenderClipRect::new(
            &mut app,
            Some(clipper),
            Clip::HardEdge,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        first_frame(&mut app, root.as_box());

        let ops = picture_ops(&app, root.as_object());
        let clip_path = ops
            .iter()
            .find_map(|op| match op {
                Op::ClipPath { path, .. } => Some(path),
                _ => None,
            })
            .expect("the recording clips");
        assert_eq!(
            clip_path.bounds(),
            Rect::from_ltwh(0.0, 0.0, 50.0, 100.0).into()
        );

        assert!(hit_test(&mut app, clip.as_box(), Offset::new(25.0, 50.0)).0);
        assert!(
            !hit_test(&mut app, clip.as_box(), Offset::new(75.0, 50.0)).0,
            "outside the clipper's clip"
        );
    }

    #[test]
    fn clip_rect_without_clipping_paints_its_child_directly() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let clip = RenderClipRect::new(&mut app, None, Clip::None, Some(child.as_box()));
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        first_frame(&mut app, root.as_box());

        assert!(
            !picture_has_clip(&app, root.as_object()),
            "Clip::None paints the child directly"
        );
        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), RED)]
        );
    }

    #[test]
    fn clip_rrect_pushes_its_border_radius() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let clip = RenderClipRRect::new(
            &mut app,
            BorderRadiusGeometry::circular(20.0),
            None,
            Clip::AntiAlias,
            None,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        first_frame(&mut app, root.as_box());

        let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
        let ops = picture_ops(&app, root.as_object());
        let clip_path = ops
            .iter()
            .find_map(|op| match op {
                Op::ClipPath { path, .. } => Some(path),
                _ => None,
            })
            .expect("the recording starts with the clip");
        assert_eq!(clip_path.bounds(), bounds.into());

        assert!(
            hit_test(&mut app, clip.as_box(), Offset::new(99.0, 99.0)).0,
            "without a clipper the hit test is not clipped, as in Dart"
        );
    }

    #[test]
    fn clip_rrect_clips_hit_tests_to_its_clipper() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let clipper: Rc<dyn CustomClipper<RRect>> = Rc::new(LeftHalfClipper);
        let clip = RenderClipRRect::new(
            &mut app,
            BorderRadiusGeometry::ZERO,
            Some(clipper),
            Clip::AntiAlias,
            None,
            Some(child.as_box()),
        );
        clip.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        assert!(hit_test(&mut app, clip.as_box(), Offset::new(25.0, 50.0)).0);
        assert!(!hit_test(&mut app, clip.as_box(), Offset::new(75.0, 50.0)).0);
    }

    /// The cached clip is dropped when the clipper notifies and when the box is resized.
    #[test]
    fn clip_rect_reclips_on_notification_and_on_resize() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let reclip = app.create(ValueNotifier::new(0));
        let clipper: Rc<dyn CustomClipper<Rect>> = Rc::new(NotifyingClipper {
            reclip: Rc::new(reclip),
        });
        let child = red_box(&mut app);
        let clip = RenderClipRect::new(
            &mut app,
            Some(clipper),
            Clip::HardEdge,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        let owner = first_frame(&mut app, root.as_box());
        assert!(clip.custom_clip_data(&app).clip.is_some());

        reclip.set_value(&mut app, 1);
        assert!(
            clip.custom_clip_data(&app).clip.is_none(),
            "the clipper's notification dropped the cached clip"
        );
        assert!(clip.as_object().debug_needs_paint(&app));
        pump_frame(&mut app, owner);
        assert!(clip.custom_clip_data(&app).clip.is_some());

        clip.layout(
            &mut app,
            BoxConstraints::tight(Size::new(50.0, 50.0)),
            false,
        );
        assert!(
            clip.custom_clip_data(&app).clip.is_none(),
            "a new size dropped the cached clip"
        );
    }

    #[test]
    fn transform_paints_under_its_matrix() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let transform = RenderTransform::new(
            &mut app,
            Matrix4::scale(2.0, 2.0),
            None,
            None,
            None,
            true,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(transform.as_box()));
        first_frame(&mut app, root.as_box());

        let ops = picture_ops(&app, root.as_object());
        let pushed = ops.iter().find_map(|op| match op {
            Op::Transform(transform) => Some(*transform),
            _ => None,
        });
        assert_eq!(pushed, Some(Matrix4::scale(2.0, 2.0)));
    }

    /// A translation is folded into the child's paint offset instead of a pushed transform.
    #[test]
    fn transform_by_a_translation_paints_at_an_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let transform = RenderTransform::new(
            &mut app,
            Matrix4::translation(10.0, 20.0),
            None,
            None,
            None,
            true,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(transform.as_box()));
        first_frame(&mut app, root.as_box());

        assert!(
            !picture_ops(&app, root.as_object())
                .iter()
                .any(|op| matches!(op, Op::Transform(_))),
            "a translation is folded into the child's paint offset"
        );
        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(Rect::from_ltwh(10.0, 20.0, 100.0, 100.0), RED)]
        );
    }

    #[test]
    fn transform_hit_tests_through_its_matrix() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let transform = RenderTransform::new(
            &mut app,
            Matrix4::scale(2.0, 2.0),
            None,
            None,
            None,
            true,
            Some(child.as_box()),
        );
        transform.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        assert_eq!(
            hit_test(&mut app, transform.as_box(), Offset::new(150.0, 50.0)),
            (true, 1),
            "the child covers twice the box, and the transform does not add itself"
        );

        transform.set_transform_hit_tests(&mut app, false);
        assert!(!hit_test(&mut app, transform.as_box(), Offset::new(150.0, 50.0)).0);
        assert!(hit_test(&mut app, transform.as_box(), Offset::new(50.0, 50.0)).0);
    }

    /// `Transform.scale(alignment: Alignment.center)`: the child grows around the box's centre.
    #[test]
    fn transform_scales_around_its_alignment_and_origin() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let transform = RenderTransform::new(
            &mut app,
            Matrix4::scale(2.0, 2.0),
            None,
            Some(AlignmentGeometry::CENTER),
            None,
            true,
            Some(child.as_box()),
        );
        transform.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert!(hit_test(&mut app, transform.as_box(), Offset::new(140.0, 140.0)).0);
        assert!(!hit_test(&mut app, transform.as_box(), Offset::new(160.0, 160.0)).0);

        transform.set_alignment(&mut app, None);
        transform.set_origin(&mut app, Some(Offset::new(50.0, 50.0)));
        assert!(
            hit_test(&mut app, transform.as_box(), Offset::new(140.0, 140.0)).0,
            "an origin at the centre is the same conjugation"
        );
        assert!(!hit_test(&mut app, transform.as_box(), Offset::new(160.0, 160.0)).0);
    }

    #[test]
    fn transform_applies_its_paint_transform_to_the_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let transform = RenderTransform::new(
            &mut app,
            Matrix4::translation(10.0, 20.0),
            None,
            None,
            None,
            true,
            Some(child.as_box()),
        );
        let mut matrix = Matrix4::IDENTITY;
        RenderBox::apply_paint_transform(transform, &app, child.as_object(), &mut matrix);
        assert_eq!(matrix, Matrix4::translation(10.0, 20.0));
    }

    #[test]
    fn fractional_translation_offsets_paint_and_hits() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let translated = RenderFractionalTranslation::new(
            &mut app,
            Offset::new(0.5, 0.0),
            true,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(translated.as_box()));
        first_frame(&mut app, root.as_box());

        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(Rect::from_ltwh(50.0, 0.0, 100.0, 100.0), RED)],
            "half the width to the right"
        );

        assert!(hit_test(&mut app, translated.as_box(), Offset::new(75.0, 50.0)).0);
        assert!(!hit_test(&mut app, translated.as_box(), Offset::new(25.0, 50.0)).0);

        translated.set_transform_hit_tests(&mut app, false);
        assert!(
            hit_test(&mut app, translated.as_box(), Offset::new(25.0, 50.0)).0,
            "an untransformed hit test reaches the child where it was laid out"
        );
    }

    #[test]
    fn ignore_pointer_is_invisible_to_hit_testing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let ignoring = RenderIgnorePointer::new(&mut app, true, Some(child.as_box()));
        ignoring.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        assert_eq!(
            hit_test(&mut app, ignoring.as_box(), Offset::new(50.0, 50.0)),
            (false, 0)
        );

        ignoring.set_ignoring(&mut app, false);
        assert_eq!(
            hit_test(&mut app, ignoring.as_box(), Offset::new(50.0, 50.0)),
            (true, 2),
            "the child and the ignore pointer itself"
        );
    }

    #[test]
    fn absorb_pointer_takes_the_hit_from_its_subtree() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let absorbing = RenderAbsorbPointer::new(&mut app, true, Some(child.as_box()));
        absorbing.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        assert_eq!(
            hit_test(&mut app, absorbing.as_box(), Offset::new(50.0, 50.0)),
            (true, 0),
            "hit testing stops here, and nothing is added to the path"
        );
        assert_eq!(
            hit_test(&mut app, absorbing.as_box(), Offset::new(150.0, 50.0)),
            (false, 0)
        );

        absorbing.set_absorbing(&mut app, false);
        assert_eq!(
            hit_test(&mut app, absorbing.as_box(), Offset::new(50.0, 50.0)),
            (true, 2)
        );
    }

    #[test]
    fn offstage_lays_out_without_painting_or_hit_testing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let offstage = RenderOffstage::new(&mut app, true, Some(child.as_box()));
        let root = RenderRepaintBoundary::new(&mut app, Some(offstage.as_box()));
        let owner = first_frame(&mut app, root.as_box());

        assert!(RenderObject::sized_by_parent(offstage, &app));
        assert_eq!(child.size(&app), Size::new(100.0, 100.0), "still laid out");
        assert!(
            !picture_has_clip(&app, root.as_object()),
            "offstage paints nothing"
        );
        assert_eq!(
            hit_test(&mut app, offstage.as_box(), Offset::new(50.0, 50.0)),
            (false, 0)
        );
        assert!(!offstage.paints_child(&app, child.as_box()));

        offstage.set_offstage(&mut app, false);
        pump_frame(&mut app, owner);
        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), RED)]
        );
        assert!(hit_test(&mut app, offstage.as_box(), Offset::new(50.0, 50.0)).0);
        assert!(offstage.paints_child(&app, child.as_box()));
    }

    #[test]
    fn colored_box_fills_its_bounds() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let colored = red_box(&mut app);
        let root = RenderRepaintBoundary::new(&mut app, Some(colored.as_box()));
        first_frame(&mut app, root.as_box());

        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), RED)]
        );
        assert_eq!(
            hit_test(&mut app, colored.as_box(), Offset::new(50.0, 50.0)),
            (true, 1),
            "an opaque box is hit by itself"
        );
    }

    #[test]
    fn colored_box_paints_its_child_over_the_color() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = red_box(&mut app);
        let colored = RenderColoredBox::new(&mut app, BLUE, true, Some(child.as_box()));
        let root = RenderRepaintBoundary::new(&mut app, Some(colored.as_box()));
        first_frame(&mut app, root.as_box());

        let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
        assert_eq!(
            drawn_rects(&app, root.as_object()),
            [fill(bounds, BLUE), fill(bounds, RED)]
        );
    }

    #[test]
    fn meta_data_carries_its_payload_and_hit_tests_by_behavior() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let payload: Rc<dyn Any> = Rc::new(42u32);
        let opaque = RenderMetaData::new(
            &mut app,
            Some(Rc::clone(&payload)),
            HitTestBehavior::Opaque,
            None,
        );
        opaque.layout(
            &mut app,
            BoxConstraints::tight(Size::new(10.0, 10.0)),
            false,
        );
        assert_eq!(
            opaque
                .meta_data(&app)
                .and_then(|data| data.downcast::<u32>().ok())
                .as_deref(),
            Some(&42)
        );
        let mut result = HitTestResult::new();
        assert!(opaque.as_box().hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0)
        ));
        assert_eq!(result.path().len(), 1);

        let deferring = RenderMetaData::new(&mut app, None, HitTestBehavior::DeferToChild, None);
        deferring.layout(
            &mut app,
            BoxConstraints::tight(Size::new(10.0, 10.0)),
            false,
        );
        let mut result = HitTestResult::new();
        assert!(!deferring.as_box().hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0)
        ));
        assert!(result.path().is_empty());

        // Deferring to a child that is hit puts the meta data on the path above it.
        let child = opaque_leaf(&mut app);
        deferring.set_child(&mut app, Some(child.as_box()));
        deferring.layout(
            &mut app,
            BoxConstraints::tight(Size::new(10.0, 10.0)),
            false,
        );
        let mut result = HitTestResult::new();
        assert!(deferring.as_box().hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0)
        ));
        assert_eq!(result.path().len(), 2, "the child, then the meta data");
    }

    /// The scene of a repaint-boundary root after a frame, as display-list ops.
    fn scene_ops(app: &mut App, root: AnyRenderObject) -> Vec<Op> {
        let scene = root
            .debug_layer(app)
            .expect("painted")
            .build_scene(app, SceneBuilder::new());
        inset_embedder::flattened_ops(&scene)
    }

    #[test]
    fn a_backdrop_filter_blurs_under_its_bounds_before_its_child_paints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderColoredBox::new(&mut app, Color::new(0xFF00FF00), true, None);
        let filter = RenderBackdropFilter::new(
            &mut app,
            ImageFilterConfig::new(ImageFilter::blur(6.0, 6.0)),
            BlendMode::SrcOver,
            true,
            None,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(filter.as_box()));
        let owner = first_frame(&mut app, root.as_box());
        let ops = scene_ops(&mut app, root.as_object());
        let blur = ops
            .iter()
            .position(|op| {
                matches!(
                    op,
                    Op::SaveLayer {
                        backdrop_filter: Some(_),
                        ..
                    }
                )
            })
            .expect("a backdrop layer");
        let draw = ops
            .iter()
            .position(|op| matches!(op, Op::DrawRect { .. }))
            .expect("the child's paint");
        assert!(blur < draw, "the blur precedes the child: {ops:?}");
        if let Op::SaveLayer {
            backdrop_filter,
            backdrop_key,
            ..
        } = &ops[blur]
        {
            assert_eq!(
                *backdrop_filter,
                Some(inset_embedder::valo::ImageFilter::blur(6.0, 6.0))
            );
            assert_eq!(*backdrop_key, None);
        }

        let key = BackdropKey::new();
        filter.set_backdrop_key(&mut app, Some(key));
        pump_frame(&mut app, owner);
        let ops = scene_ops(&mut app, root.as_object());
        assert!(
            ops.iter().any(|op| matches!(
                op,
                Op::SaveLayer {
                    backdrop_key: Some(_),
                    ..
                }
            )),
            "a keyed filter shares its blur: {ops:?}"
        );

        filter.set_enabled(&mut app, false);
        pump_frame(&mut app, owner);
        let ops = scene_ops(&mut app, root.as_object());
        assert!(
            !ops.iter().any(|op| matches!(
                op,
                Op::SaveLayer {
                    backdrop_filter: Some(_),
                    ..
                }
            )),
            "disabled paints the child only: {ops:?}"
        );
    }

    #[test]
    fn a_composed_backdrop_filter_replays_the_blur_it_composes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderColoredBox::new(&mut app, Color::new(0xFF00FF00), true, None);
        let saturation = ImageFilterConfig::new(ImageFilter::Color(ColorFilter::saturation(1.8)));
        let filter = RenderBackdropFilter::new(
            &mut app,
            ImageFilterConfig::compose(saturation, ImageFilterConfig::blur().sigma_x(4.0)),
            BlendMode::SrcOver,
            true,
            None,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(filter.as_box()));
        first_frame(&mut app, root.as_box());
        let ops = scene_ops(&mut app, root.as_object());
        // The current Inset adapter extracts only blur (see the host's PORTING.md).
        assert!(
            ops.iter().any(|op| matches!(
                op,
                Op::SaveLayer {
                    backdrop_filter: Some(inset_embedder::valo::ImageFilter::Blur { sigma_x, sigma_y }),
                    ..
                } if *sigma_x == 4.0 && *sigma_y == 4.0
            )),
            "the composed blur reaches the backdrop: {ops:?}"
        );
    }

    #[test]
    fn clip_oval_hit_tests_inside_the_inscribed_oval_and_clips_with_its_path() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let oval = RenderClipOval::new(&mut app, None, Clip::AntiAlias, Some(child.as_box()));
        let root = RenderRepaintBoundary::new(&mut app, Some(oval.as_box()));
        first_frame(&mut app, root.as_box());
        let hit = |app: &mut App, x: f64, y: f64| {
            let mut result = HitTestResult::new();
            oval.as_box().hit_test(
                app,
                &mut BoxHitTestResult::wrap(&mut result),
                Offset::new(x, y),
            )
        };
        assert!(hit(&mut app, 50.0, 50.0), "the center is inside");
        assert!(!hit(&mut app, 2.0, 2.0), "a corner is outside the oval");
        assert!(
            picture_has_clip(&app, root.as_object()),
            "an oval clips with a path"
        );
    }

    #[test]
    fn clip_path_uses_its_clipper_for_hit_tests_and_the_rect_by_default() {
        struct Triangle;
        impl CustomClipper<Arc<Path>> for Triangle {
            fn get_clip(&self, size: Size) -> Arc<Path> {
                let mut path = PathBuilder::new();
                path.move_to(Point::new(0.0, 0.0));
                path.line_to(Point::new(size.width() as f32, 0.0));
                path.line_to(Point::new(0.0, size.height() as f32));
                path.close();
                path.build()
            }

            fn should_reclip(&self, _old: &dyn CustomClipper<Arc<Path>>) -> bool {
                false
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = opaque_leaf(&mut app);
        let clip = RenderClipPath::new(
            &mut app,
            Some(Rc::new(Triangle)),
            Clip::AntiAlias,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        first_frame(&mut app, root.as_box());
        let hit = |app: &mut App, x: f64, y: f64| {
            let mut result = HitTestResult::new();
            clip.as_box().hit_test(
                app,
                &mut BoxHitTestResult::wrap(&mut result),
                Offset::new(x, y),
            )
        };
        assert!(hit(&mut app, 10.0, 10.0), "inside the triangle");
        assert!(!hit(&mut app, 90.0, 90.0), "outside the triangle");

        let plain = RenderClipPath::new(&mut app, None, Clip::HardEdge, None);
        plain.layout(
            &mut app,
            BoxConstraints::tight(Size::new(30.0, 20.0)),
            false,
        );
        assert!(
            plain
                .default_clip(&app)
                .contains(Point::new(29.0, 19.0), FillRule::NonZero),
            "the default clip is the box's rect"
        );
    }

    #[test]
    fn clip_rsuperellipse_resolves_its_border_radius_and_clips_with_a_path() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderConstrainedBox::new(&mut app, BoxConstraints::expand(None, None), None);
        let clip = RenderClipRSuperellipse::new(
            &mut app,
            BorderRadiusGeometry::circular(12.0),
            None,
            Clip::AntiAlias,
            Some(TextDirection::Ltr),
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(clip.as_box()));
        first_frame(&mut app, root.as_box());
        clip.update_clip(&mut app);
        assert_eq!(clip.clip(&app).tl_radius_x, 12.0);
        assert_eq!(
            clip.clip(&app).outer_rect(),
            Rect::from_ltwh(0.0, 0.0, 100.0, 100.0)
        );
        assert!(
            picture_has_clip(&app, root.as_object()),
            "a superellipse clips with its path"
        );
        clip.set_border_radius(&mut app, BorderRadiusGeometry::circular(4.0));
        assert!(
            clip.custom_clip_data(&app).clip.is_none(),
            "a new radius marks the clip"
        );
    }

    #[test]
    fn a_shape_border_clipper_clips_to_the_shape_and_reclips_when_it_changes() {
        let square: Box<dyn ShapeBorder> = Box::new(inset_painting::RoundedRectangleBorder::new(
            inset_painting::BorderSide::NONE,
            inset_painting::BorderRadiusGeometry::circular(10.0),
        ));
        let clipper = ShapeBorderClipper::new(square.clone_box(), Some(TextDirection::Ltr));
        let clip = clipper.get_clip(Size::new(40.0, 40.0));
        assert!(clip.contains(Point::new(20.0, 20.0), FillRule::NonZero));
        assert!(
            !clip.contains(Point::new(0.5, 0.5), FillRule::NonZero),
            "a rounded corner"
        );
        let same = ShapeBorderClipper::new(square.clone_box(), Some(TextDirection::Ltr));
        assert!(!clipper.should_reclip(&same));
        let other = ShapeBorderClipper::new(square, Some(TextDirection::Rtl));
        assert!(clipper.should_reclip(&other));
    }
}
