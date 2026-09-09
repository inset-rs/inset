//! Flutter counterpart: `rendering/shifted_box.dart` (`BoxConstraintsTransform`,
//! `RenderShiftedBox`, `RenderPadding`, `RenderAligningShiftedBox`, `RenderPositionedBox`,
//! `OverflowBoxFit`, `RenderConstrainedOverflowBox`, `RenderConstraintsTransformBox`,
//! `RenderSizedOverflowBox`, `RenderFractionallySizedOverflowBox`,
//! `RenderCustomSingleChildLayoutBox`, `SingleChildLayoutDelegate`, `RenderBaseline`).

use std::any::Any;
use std::fmt::Debug;
use std::rc::Rc;

use reveal_embedder::{Clip, Offset, Rect, Size, TextBaseline};
use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_painting::{
    Alignment, AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry, TextDirection,
};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestResult, BoxParentData, RenderBox, RenderBoxData,
};
use crate::layer::{ClipRectLayer, LayerHandle};
use crate::layout_helper::{ChildBaselineGetter, ChildLayoutHelper, ChildLayouter};
use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::stack::RelativeRect;

/// Signature for a function that transforms a [`BoxConstraints`] to another [`BoxConstraints`].
///
/// Used by [`RenderConstraintsTransformBox`]. Typically the caller requires the returned
/// [`BoxConstraints`] to be [`Constraints::is_normalized`].
pub type BoxConstraintsTransform = fn(BoxConstraints) -> BoxConstraints;

/// Abstract class for one-child-layout render boxes that provide control over
/// the child's position.
///
/// Flutter's `RenderShiftedBox`: the shared bodies. A leaf implements the marker and calls
/// these where Dart would run the inherited method: `RenderShiftedBox::paint(self, …)`.
pub trait RenderShiftedBox:
    RenderObjectWithChildMixin<ChildType = AnyRenderBox> + RenderBox
{
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

    /// The child's baseline shifted by the child's paint offset, or none without a child.
    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let child = self.child(app)?;
        debug_assert!(!child.as_object().debug_needs_layout(app));
        let result = child.get_distance_to_actual_baseline(app, baseline)?;
        let offset = child
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset;
        Some(result + offset.dy())
    }

    /// The child's dry baseline at this box's constraints. The base applies no transform;
    /// subclasses override to add their offset.
    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.child(app)
            .and_then(|child| child.get_dry_baseline(app, constraints, baseline))
    }

    /// Hit tests the child at its parent-data offset.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let child_offset = child
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset;
        result.add_with_paint_offset(Some(child_offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position - child_offset);
            child.hit_test(app, result, transformed)
        })
    }

    /// Paints the child at its parent-data offset.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<BoxParentData>(app)
                .offset;
            context.paint_child(app, child.as_object(), child_offset + offset);
        }
    }
}

/// Insets its child by the given padding.
pub struct RenderPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    resolved_padding_cache: Option<EdgeInsets>,
    padding: EdgeInsetsGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderPadding {
    /// Creates a render object that insets its child.
    ///
    /// `padding` must have non-negative insets.
    pub fn new(
        app: &mut App,
        padding: EdgeInsetsGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderPadding> {
        debug_assert!(padding.is_non_negative());
        let this = RenderHandle::new_box(
            app,
            RenderPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                resolved_padding_cache: None,
                padding,
                text_direction,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The amount to pad the child in each dimension.
    pub fn padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsGeometry {
        self.get(app).padding
    }

    /// Sets [`padding`](Self::padding).
    pub fn set_padding(self: RenderHandle<Self>, app: &mut App, value: EdgeInsetsGeometry) {
        debug_assert!(value.is_non_negative());
        if self.get(app).padding == value {
            return;
        }
        self.get_mut(app).padding = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`padding`](Self::padding).
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
        self.mark_need_resolution(app);
    }

    fn resolved_padding(&mut self) -> EdgeInsets {
        if let Some(cached) = self.resolved_padding_cache {
            return cached;
        }
        let resolved = self.padding.resolve(self.text_direction);
        debug_assert!(resolved.is_non_negative());
        self.resolved_padding_cache = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_padding_cache = None;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderPadding {
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

impl RenderShiftedBox for RenderPadding {}

impl RenderObject for RenderPadding {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let padding = self.get_mut(app).resolved_padding();
        let child = self.child(app);
        if child.is_none() {
            self.set_size(
                app,
                constraints.constrain(Size::new(padding.horizontal(), padding.vertical())),
            );
            return;
        }
        let child = child.expect("checked");
        let inner_constraints = constraints.deflate(EdgeInsetsGeometry::Insets(padding));
        child.layout(app, inner_constraints, true);
        child.parent_data_of_mut::<BoxParentData>(app).offset =
            Offset::new(padding.left, padding.top);
        self.set_size(
            app,
            constraints.constrain(Size::new(
                padding.horizontal() + child.size(app).width(),
                padding.vertical() + child.size(app).height(),
            )),
        );
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderPadding {
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
        let padding = self.get_mut(app).resolved_padding();
        match self.child(app) {
            // Relies on f64::INFINITY absorption.
            Some(child) => {
                child.get_min_intrinsic_width(app, (height - padding.vertical()).max(0.0))
                    + padding.horizontal()
            }
            None => padding.horizontal(),
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let padding = self.get_mut(app).resolved_padding();
        match self.child(app) {
            Some(child) => {
                child.get_max_intrinsic_width(app, (height - padding.vertical()).max(0.0))
                    + padding.horizontal()
            }
            None => padding.horizontal(),
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let padding = self.get_mut(app).resolved_padding();
        match self.child(app) {
            Some(child) => {
                child.get_min_intrinsic_height(app, (width - padding.horizontal()).max(0.0))
                    + padding.vertical()
            }
            None => padding.vertical(),
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let padding = self.get_mut(app).resolved_padding();
        match self.child(app) {
            Some(child) => {
                child.get_max_intrinsic_height(app, (width - padding.horizontal()).max(0.0))
                    + padding.vertical()
            }
            None => padding.vertical(),
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let padding = self.get_mut(app).resolved_padding();
        let inner_constraints = constraints.deflate(EdgeInsetsGeometry::Insets(padding));
        let child_baseline = child.get_dry_baseline(app, inner_constraints, baseline)?;
        Some(child_baseline + padding.top)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let padding = self.get_mut(app).resolved_padding();
        let Some(child) = self.child(app) else {
            return constraints.constrain(Size::new(padding.horizontal(), padding.vertical()));
        };
        let inner_constraints = constraints.deflate(EdgeInsetsGeometry::Insets(padding));
        let child_size = child.get_dry_layout(app, inner_constraints);
        constraints.constrain(Size::new(
            padding.horizontal() + child_size.width(),
            padding.vertical() + child_size.height(),
        ))
    }
}

/// The mixin fields of [`RenderAligningShiftedBox`].
pub struct RenderAligningShiftedBoxData {
    resolved_alignment: Option<Alignment>,
    alignment: AlignmentGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderAligningShiftedBoxData {
    /// Flutter's constructor arguments; `alignment` defaults to center.
    pub const fn new(
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
    ) -> RenderAligningShiftedBoxData {
        RenderAligningShiftedBoxData {
            resolved_alignment: None,
            alignment,
            text_direction,
        }
    }
}

/// Abstract class for one-child-layout render boxes that use a
/// [`AlignmentGeometry`] to align their children.
pub trait RenderAligningShiftedBox: RenderShiftedBox {
    /// Mixin field access.
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData;

    /// See [`aligning_data`](Self::aligning_data).
    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData;

    /// The [`alignment`](Self::alignment) resolved against [`text_direction`](Self::text_direction).
    fn resolved_alignment(self: RenderHandle<Self>, app: &mut App) -> Alignment {
        let data = self.aligning_data(app);
        if let Some(resolved) = data.resolved_alignment {
            return resolved;
        }
        let resolved = data.alignment.resolve(data.text_direction);
        self.aligning_data_mut(app).resolved_alignment = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.aligning_data_mut(app).resolved_alignment = None;
        self.mark_needs_layout(app);
    }

    /// How to align the child.
    ///
    /// The x and y values of the alignment control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    fn alignment(self: RenderHandle<Self>, app: &App) -> AlignmentGeometry {
        self.aligning_data(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: AlignmentGeometry) {
        if self.aligning_data(app).alignment == value {
            return;
        }
        self.aligning_data_mut(app).alignment = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`alignment`](Self::alignment).
    fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.aligning_data(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: Option<TextDirection>) {
        if self.aligning_data(app).text_direction == value {
            return;
        }
        self.aligning_data_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    /// Apply the current [`alignment`](Self::alignment) to the child.
    ///
    /// Subclasses should call this method if they have a child, to have this
    /// class perform the actual alignment. If they don't have a child, they
    /// should not call this method.
    fn align_child(self: RenderHandle<Self>, app: &mut App) {
        let child = self.child(app).expect("align_child requires a child");
        debug_assert!(!child.as_object().debug_needs_layout(app));
        let offset = self
            .resolved_alignment(app)
            .along_offset(self.size(app) - child.size(app));
        child
            .as_object()
            .parent_data_of_mut::<BoxParentData>(app)
            .offset = offset;
    }
}

/// Positions its child using an [`AlignmentGeometry`].
///
/// For example, to align a box at the bottom right, you would pass this box a
/// tight constraint that is bigger than the child's natural size, with an
/// alignment of `Alignment::BOTTOM_RIGHT`.
///
/// By default, sizes to be as big as possible in both axes. If either axis is
/// unconstrained, then in that direction it will be sized to fit the child's
/// dimensions. Using `width_factor` and `height_factor` you can force this
/// latter behavior in all cases.
pub struct RenderPositionedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

impl RenderPositionedBox {
    /// Creates a render object that positions its child.
    ///
    /// `width_factor` and `height_factor` must be non-negative when given.
    pub fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        width_factor: Option<f64>,
        height_factor: Option<f64>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        debug_assert!(width_factor.is_none_or(|factor| factor >= 0.0));
        debug_assert!(height_factor.is_none_or(|factor| factor >= 0.0));
        let this = RenderHandle::new_box(
            app,
            RenderPositionedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                width_factor,
                height_factor,
            },
        );
        this.set_child(app, child);
        this
    }

    /// If non-null, sets its width to the child's width multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be positive.
    pub fn width_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).width_factor
    }

    /// Sets [`width_factor`](Self::width_factor).
    pub fn set_width_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).width_factor == value {
            return;
        }
        self.get_mut(app).width_factor = value;
        self.mark_needs_layout(app);
    }

    /// If non-null, sets its height to the child's height multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be positive.
    pub fn height_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).height_factor
    }

    /// Sets [`height_factor`](Self::height_factor).
    pub fn set_height_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).height_factor == value {
            return;
        }
        self.get_mut(app).height_factor = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderPositionedBox {
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

impl RenderShiftedBox for RenderPositionedBox {}

impl RenderAligningShiftedBox for RenderPositionedBox {
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

impl RenderPositionedBox {
    /// Dart's sizing line of `performLayout` / `computeDryLayout`: shrink-wrap an axis when it
    /// has a factor or is unbounded, and `child_size` is zero without a child.
    fn size_for_child(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
        child_size: Size,
    ) -> Size {
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let shrink_wrap_width = width_factor.is_some() || constraints.max_width == f64::INFINITY;
        let shrink_wrap_height = height_factor.is_some() || constraints.max_height == f64::INFINITY;
        constraints.constrain(Size::new(
            if shrink_wrap_width {
                child_size.width() * width_factor.unwrap_or(1.0)
            } else {
                f64::INFINITY
            },
            if shrink_wrap_height {
                child_size.height() * height_factor.unwrap_or(1.0)
            } else {
                f64::INFINITY
            },
        ))
    }
}

impl RenderObject for RenderPositionedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        match self.child(app) {
            Some(child) => {
                child.layout(app, constraints.loosen(), true);
                let child_size = child.size(app);
                let size = self.size_for_child(app, constraints, child_size);
                self.set_size(app, size);
                self.align_child(app);
            }
            None => {
                let size = self.size_for_child(app, constraints, Size::ZERO);
                self.set_size(app, size);
            }
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderPositionedBox {
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
        let factor = self.get(app).width_factor.unwrap_or(1.0);
        RenderShiftedBox::compute_min_intrinsic_width(self, app, height) * factor
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let factor = self.get(app).width_factor.unwrap_or(1.0);
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height) * factor
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let factor = self.get(app).height_factor.unwrap_or(1.0);
        RenderShiftedBox::compute_min_intrinsic_height(self, app, width) * factor
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let factor = self.get(app).height_factor.unwrap_or(1.0);
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width) * factor
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = constraints.loosen();
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, child_constraints);
        let size = self.size_for_child(app, constraints, child_size);
        let child_offset = self.resolved_alignment(app).along_offset(size - child_size);
        Some(result + child_offset.dy())
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        match self.child(app) {
            Some(child) => {
                let child_size = child.get_dry_layout(app, constraints.loosen());
                self.size_for_child(app, constraints, child_size)
            }
            None => self.size_for_child(app, constraints, Size::ZERO),
        }
    }
}

/// How much space should be occupied by the [`RenderConstrainedOverflowBox`] when it is not
/// overflowing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowBoxFit {
    /// The widget will size itself to be as large as the parent allows.
    Max,

    /// The widget will follow the child's size.
    ///
    /// More specifically, the render object will size itself to match the size of its child
    /// within the constraints of its parent, or as small as the parent allows if no child is
    /// set.
    DeferToChild,
}

/// A render object that imposes different constraints on its child than it gets from its parent,
/// possibly allowing the child to overflow the parent.
///
/// A render overflow box proxies most functions in the render box protocol to its child, except
/// that when laying out its child, it passes constraints based on the `min_width`, `max_width`,
/// `min_height`, and `max_height` fields instead of just passing the parent's constraints in.
/// Specifically, it overrides any of the equivalent fields on the constraints given by the parent
/// with the constraints given by these fields, as long as those values are not null. If a value is
/// null, then the original constraint is used.
///
/// Additionally, this render object allows the child to overflow the bounds of this box. If the
/// child overflows, this box will not clip the overflowing part.
pub struct RenderConstrainedOverflowBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    min_width: Option<f64>,
    max_width: Option<f64>,
    min_height: Option<f64>,
    max_height: Option<f64>,
    fit: OverflowBoxFit,
}

impl RenderConstrainedOverflowBox {
    /// Creates a render object that lets its child overflow itself.
    pub fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderConstrainedOverflowBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                fit: OverflowBoxFit::Max,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The minimum width constraint to give the child. `None` (the default) uses the constraint
    /// from the parent instead.
    pub fn min_width(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).min_width
    }

    /// Sets [`min_width`](Self::min_width).
    pub fn set_min_width(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).min_width == value {
            return;
        }
        self.get_mut(app).min_width = value;
        self.mark_needs_layout(app);
    }

    /// The maximum width constraint to give the child. `None` (the default) uses the constraint
    /// from the parent instead.
    pub fn max_width(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).max_width
    }

    /// Sets [`max_width`](Self::max_width).
    pub fn set_max_width(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).max_width == value {
            return;
        }
        self.get_mut(app).max_width = value;
        self.mark_needs_layout(app);
    }

    /// The minimum height constraint to give the child. `None` (the default) uses the constraint
    /// from the parent instead.
    pub fn min_height(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).min_height
    }

    /// Sets [`min_height`](Self::min_height).
    pub fn set_min_height(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).min_height == value {
            return;
        }
        self.get_mut(app).min_height = value;
        self.mark_needs_layout(app);
    }

    /// The maximum height constraint to give the child. `None` (the default) uses the constraint
    /// from the parent instead.
    pub fn max_height(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).max_height
    }

    /// Sets [`max_height`](Self::max_height).
    pub fn set_max_height(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).max_height == value {
            return;
        }
        self.get_mut(app).max_height = value;
        self.mark_needs_layout(app);
    }

    /// The way to size the render object.
    ///
    /// This only affects the scenario where the child does not indeed overflow. If set to
    /// [`OverflowBoxFit::DeferToChild`], the render object will size itself to match the size of
    /// its child within the constraints of its parent, or as small as the parent allows if no
    /// child is set. If set to [`OverflowBoxFit::Max`] (the default), the render object will size
    /// itself to be as large as the parent allows.
    pub fn fit(self: RenderHandle<Self>, app: &App) -> OverflowBoxFit {
        self.get(app).fit
    }

    /// Sets [`fit`](Self::fit).
    pub fn set_fit(self: RenderHandle<Self>, app: &mut App, value: OverflowBoxFit) {
        if self.get(app).fit == value {
            return;
        }
        self.get_mut(app).fit = value;
        self.as_object()
            .mark_needs_layout_for_sized_by_parent_change(app);
    }

    fn get_inner_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let this = self.get(app);
        BoxConstraints {
            min_width: this.min_width.unwrap_or(constraints.min_width),
            max_width: this.max_width.unwrap_or(constraints.max_width),
            min_height: this.min_height.unwrap_or(constraints.min_height),
            max_height: this.max_height.unwrap_or(constraints.max_height),
        }
    }
}

impl RenderObjectWithChildMixin for RenderConstrainedOverflowBox {
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

impl RenderShiftedBox for RenderConstrainedOverflowBox {}

impl RenderAligningShiftedBox for RenderConstrainedOverflowBox {
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

impl RenderObject for RenderConstrainedOverflowBox {
    crate::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, app: &App) -> bool {
        match self.fit(app) {
            OverflowBoxFit::Max => true,
            // If defer_to_child, the size will be as small as its child when non-overflowing,
            // thus it cannot be sized_by_parent.
            OverflowBoxFit::DeferToChild => false,
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        match self.child(app) {
            Some(child) => {
                let inner = self.get_inner_constraints(app, constraints);
                child.layout(app, inner, true);
                match self.fit(app) {
                    OverflowBoxFit::Max => debug_assert!(self.sized_by_parent(app)),
                    OverflowBoxFit::DeferToChild => {
                        let size = constraints.constrain(child.size(app));
                        self.set_size(app, size);
                    }
                }
                self.align_child(app);
            }
            None => match self.fit(app) {
                OverflowBoxFit::Max => debug_assert!(self.sized_by_parent(app)),
                OverflowBoxFit::DeferToChild => self.set_size(app, constraints.smallest()),
            },
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderConstrainedOverflowBox {
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
        match self.fit(app) {
            OverflowBoxFit::Max => constraints.biggest(),
            OverflowBoxFit::DeferToChild => match self.child(app) {
                Some(child) => child.get_dry_layout(app, constraints),
                None => constraints.smallest(),
            },
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self.get_inner_constraints(app, constraints);
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, child_constraints);
        let size = self.get_dry_layout(app, constraints);
        Some(
            result
                + self
                    .resolved_alignment(app)
                    .along_offset(size - child_size)
                    .dy(),
        )
    }
}

/// A [`RenderBox`] that applies an arbitrary transform to its constraints, and sizes its child
/// using the resulting [`BoxConstraints`], optionally clipping, or treating the overflow as an
/// error.
///
/// This [`RenderBox`] sizes its child using a [`BoxConstraints`] created by applying
/// [`constraints_transform`](Self::constraints_transform) to this [`RenderBox`]'s own constraints.
/// This box will then attempt to adopt the same size, within the limits of its own constraints. If
/// it ends up with a different size, it will align the child based on
/// [`RenderAligningShiftedBox::alignment`]. If the box cannot expand enough to accommodate the
/// entire child, the child will be clipped if
/// [`clip_behavior`](Self::clip_behavior) is not [`Clip::None`].
///
/// When the child is `None`, this [`RenderBox`] takes the smallest possible size and never
/// overflows.
pub struct RenderConstraintsTransformBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    constraints_transform: BoxConstraintsTransform,
    clip_behavior: Clip,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
    overflow_container_rect: Rect,
    overflow_child_rect: Rect,
    is_overflowing: bool,
    child_constraints: Option<BoxConstraints>,
}

impl RenderConstraintsTransformBox {
    /// Creates a [`RenderBox`] that sizes itself to the child and modifies the constraints before
    /// passing them down to that child.
    pub fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        constraints_transform: BoxConstraintsTransform,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderConstraintsTransformBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                constraints_transform,
                clip_behavior: Clip::None,
                clip_rect_layer: LayerHandle::new(),
                overflow_container_rect: Rect::ZERO,
                overflow_child_rect: Rect::ZERO,
                is_overflowing: false,
                child_constraints: None,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The transform this box applies to its own constraints before laying its child out.
    pub fn constraints_transform(self: RenderHandle<Self>, app: &App) -> BoxConstraintsTransform {
        self.get(app).constraints_transform
    }

    /// Sets [`constraints_transform`](Self::constraints_transform).
    pub fn set_constraints_transform(
        self: RenderHandle<Self>,
        app: &mut App,
        value: BoxConstraintsTransform,
    ) {
        if std::ptr::fn_addr_eq(self.get(app).constraints_transform, value) {
            return;
        }
        self.get_mut(app).constraints_transform = value;
        // The render object only needs layout if the new transform maps the current constraints
        // to a different value, or the render object has never been laid out before.
        let needs_layout = match self.get(app).child_constraints {
            Some(child_constraints) => child_constraints != value(self.constraints(app)),
            None => true,
        };
        if needs_layout {
            self.mark_needs_layout(app);
        }
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
}

impl RenderObjectWithChildMixin for RenderConstraintsTransformBox {
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

impl RenderShiftedBox for RenderConstraintsTransformBox {}

impl RenderAligningShiftedBox for RenderConstraintsTransformBox {
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

impl RenderObject for RenderConstraintsTransformBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        match self.child(app) {
            Some(child) => {
                let child_constraints = (self.constraints_transform(app))(constraints);
                debug_assert!(
                    child_constraints.is_normalized(),
                    "{child_constraints} is not normalized"
                );
                self.get_mut(app).child_constraints = Some(child_constraints);
                child.layout(app, child_constraints, true);
                let size = constraints.constrain(child.size(app));
                self.set_size(app, size);
                self.align_child(app);
                let child_offset = child
                    .as_object()
                    .parent_data_of::<BoxParentData>(app)
                    .offset;
                let child_rect = child_offset & child.size(app);
                let this = self.get_mut(app);
                this.overflow_container_rect = Offset::ZERO & size;
                this.overflow_child_rect = child_rect;
            }
            None => {
                self.set_size(app, constraints.smallest());
                let this = self.get_mut(app);
                this.overflow_container_rect = Rect::ZERO;
                this.overflow_child_rect = Rect::ZERO;
            }
        }
        let this = self.get_mut(app);
        this.is_overflowing =
            RelativeRect::from_rect(this.overflow_container_rect, this.overflow_child_rect)
                .has_insets();
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
        if !self.get(app).is_overflowing {
            RenderShiftedBox::paint(self, app, context, offset);
            return;
        }
        // We have overflow and the clipBehavior isn't none. Clip it.
        let clip_rect = Offset::ZERO & self.size(app);
        let clip_behavior = self.clip_behavior(app);
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
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
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

impl RenderBox for RenderConstraintsTransformBox {
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
        let transformed =
            (self.constraints_transform(app))(BoxConstraints::new().max_height(height));
        RenderShiftedBox::compute_min_intrinsic_width(self, app, transformed.max_height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let transformed =
            (self.constraints_transform(app))(BoxConstraints::new().max_height(height));
        RenderShiftedBox::compute_max_intrinsic_width(self, app, transformed.max_height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let transformed = (self.constraints_transform(app))(BoxConstraints::new().max_width(width));
        RenderShiftedBox::compute_min_intrinsic_height(self, app, transformed.max_width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let transformed = (self.constraints_transform(app))(BoxConstraints::new().max_width(width));
        RenderShiftedBox::compute_max_intrinsic_height(self, app, transformed.max_width)
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
        match self.child(app) {
            Some(child) => {
                let child_constraints = (self.constraints_transform(app))(constraints);
                let child_size = child.get_dry_layout(app, child_constraints);
                constraints.constrain(child_size)
            }
            None => constraints.smallest(),
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = (self.constraints_transform(app))(constraints);
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, child_constraints);
        let size = constraints.constrain(child_size);
        Some(
            result
                + self
                    .resolved_alignment(app)
                    .along_offset(size - child_size)
                    .dy(),
        )
    }
}

/// A render object that is a specific size but passes its original constraints through to its
/// child, which it allows to overflow.
///
/// If the child's resulting size differs from this render object's size, then the child is aligned
/// according to [`RenderAligningShiftedBox::alignment`].
pub struct RenderSizedOverflowBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    requested_size: Size,
}

impl RenderSizedOverflowBox {
    /// Creates a render box of a given size that lets its child overflow.
    pub fn new(
        app: &mut App,
        requested_size: Size,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderSizedOverflowBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                requested_size,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The size this render box should attempt to be.
    pub fn requested_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).requested_size
    }

    /// Sets [`requested_size`](Self::requested_size).
    pub fn set_requested_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.get(app).requested_size == value {
            return;
        }
        self.get_mut(app).requested_size = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderSizedOverflowBox {
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

impl RenderShiftedBox for RenderSizedOverflowBox {}

impl RenderAligningShiftedBox for RenderSizedOverflowBox {
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

impl RenderObject for RenderSizedOverflowBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = constraints.constrain(self.requested_size(app));
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            child.layout(app, constraints, true);
            self.align_child(app);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderSizedOverflowBox {
    crate::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        self.requested_size(app).width()
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        self.requested_size(app).width()
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, _width: f64) -> f64 {
        self.requested_size(app).height()
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, _width: f64) -> f64 {
        self.requested_size(app).height()
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
        constraints.constrain(self.requested_size(app))
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
        let size = self.get_dry_layout(app, constraints);
        Some(
            result
                + self
                    .resolved_alignment(app)
                    .along_offset(size - child_size)
                    .dy(),
        )
    }
}

/// Sizes its child to a fraction of the total available space.
///
/// For both its width and height, this render object imposes a tight constraint on its child that
/// is a multiple (typically less than 1.0) of the maximum constraint it received from its parent
/// on that axis. If the factor for a given axis is `None`, then the constraints from the parent
/// are just passed through instead.
///
/// It then tries to size itself to the size of its child. Where this is not possible (e.g. if the
/// constraints from the parent are themselves tight), the child is aligned according to
/// [`RenderAligningShiftedBox::alignment`].
pub struct RenderFractionallySizedOverflowBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

impl RenderFractionallySizedOverflowBox {
    /// Creates a render box that sizes its child to a fraction of the total available space.
    pub fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderFractionallySizedOverflowBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                width_factor: None,
                height_factor: None,
            },
        );
        this.set_child(app, child);
        this
    }

    /// If non-null, the factor of the incoming width to use.
    ///
    /// If non-null, the child is given a tight width constraint that is the max incoming width
    /// constraint multiplied by this factor. If null, the child is given the incoming width
    /// constraints.
    pub fn width_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).width_factor
    }

    /// Sets [`width_factor`](Self::width_factor).
    pub fn set_width_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).width_factor == value {
            return;
        }
        self.get_mut(app).width_factor = value;
        self.mark_needs_layout(app);
    }

    /// If non-null, the factor of the incoming height to use.
    ///
    /// If non-null, the child is given a tight height constraint that is the max incoming height
    /// constraint multiplied by this factor. If null, the child is given the incoming height
    /// constraints.
    pub fn height_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).height_factor
    }

    /// Sets [`height_factor`](Self::height_factor).
    pub fn set_height_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).height_factor == value {
            return;
        }
        self.get_mut(app).height_factor = value;
        self.mark_needs_layout(app);
    }

    fn get_inner_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let mut min_width = constraints.min_width;
        let mut max_width = constraints.max_width;
        if let Some(factor) = width_factor {
            let width = max_width * factor;
            min_width = width;
            max_width = width;
        }
        let mut min_height = constraints.min_height;
        let mut max_height = constraints.max_height;
        if let Some(factor) = height_factor {
            let height = max_height * factor;
            min_height = height;
            max_height = height;
        }
        BoxConstraints {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }
}

impl RenderObjectWithChildMixin for RenderFractionallySizedOverflowBox {
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

impl RenderShiftedBox for RenderFractionallySizedOverflowBox {}

impl RenderAligningShiftedBox for RenderFractionallySizedOverflowBox {
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

impl RenderObject for RenderFractionallySizedOverflowBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let inner = self.get_inner_constraints(app, constraints);
        match self.child(app) {
            Some(child) => {
                child.layout(app, inner, true);
                let size = constraints.constrain(child.size(app));
                self.set_size(app, size);
                self.align_child(app);
            }
            None => {
                let size = constraints.constrain(inner.constrain(Size::ZERO));
                self.set_size(app, size);
            }
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderFractionallySizedOverflowBox {
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
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let result = match self.child(app) {
            // Relies on f64::INFINITY absorption.
            Some(child) => {
                child.get_min_intrinsic_width(app, height * height_factor.unwrap_or(1.0))
            }
            None => RenderShiftedBox::compute_min_intrinsic_width(self, app, height),
        };
        debug_assert!(result.is_finite());
        result / width_factor.unwrap_or(1.0)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let result = match self.child(app) {
            Some(child) => {
                child.get_max_intrinsic_width(app, height * height_factor.unwrap_or(1.0))
            }
            None => RenderShiftedBox::compute_max_intrinsic_width(self, app, height),
        };
        debug_assert!(result.is_finite());
        result / width_factor.unwrap_or(1.0)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let result = match self.child(app) {
            Some(child) => child.get_min_intrinsic_height(app, width * width_factor.unwrap_or(1.0)),
            None => RenderShiftedBox::compute_min_intrinsic_height(self, app, width),
        };
        debug_assert!(result.is_finite());
        result / height_factor.unwrap_or(1.0)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let result = match self.child(app) {
            Some(child) => child.get_max_intrinsic_height(app, width * width_factor.unwrap_or(1.0)),
            None => RenderShiftedBox::compute_max_intrinsic_height(self, app, width),
        };
        debug_assert!(result.is_finite());
        result / height_factor.unwrap_or(1.0)
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
        let inner = self.get_inner_constraints(app, constraints);
        match self.child(app) {
            Some(child) => {
                let child_size = child.get_dry_layout(app, inner);
                constraints.constrain(child_size)
            }
            None => constraints.constrain(inner.constrain(Size::ZERO)),
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self.get_inner_constraints(app, constraints);
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, child_constraints);
        let size = self.get_dry_layout(app, constraints);
        Some(
            result
                + self
                    .resolved_alignment(app)
                    .along_offset(size - child_size)
                    .dy(),
        )
    }
}

/// Shifts the child down such that the child's baseline (or the bottom of the child, if the child
/// has no baseline) is [`baseline`](Self::baseline) logical pixels below the top of this box, then
/// sizes this box to contain the child.
///
/// If [`baseline`](Self::baseline) is less than the distance from the top of the child to the
/// baseline of the child, then the child will overflow the top of the box. This is typically not
/// desirable, in particular, that part of the child will not be found when doing hit tests, so
/// the user cannot interact with that part of the child.
///
/// This box will be sized so that its bottom is coincident with the bottom of the child. This
/// means if this box shifts the child down, there will be space between the top of this box and
/// the top of the child, but there is never space between the bottom of the child and the bottom
/// of the box.
pub struct RenderBaseline {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    baseline: f64,
    baseline_type: TextBaseline,
}

/// What [`RenderBaseline::compute_sizes`] worked out: Dart's `({Size size, double top})`.
struct BaselineSizes {
    size: Size,
    top: f64,
}

impl RenderBaseline {
    /// Creates a [`RenderBaseline`] object.
    pub fn new(
        app: &mut App,
        baseline: f64,
        baseline_type: TextBaseline,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        let this = RenderHandle::new_box(
            app,
            RenderBaseline {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                baseline,
                baseline_type,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The number of logical pixels from the top of this box at which to position the child's
    /// baseline.
    pub fn baseline(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).baseline
    }

    /// Sets [`baseline`](Self::baseline).
    pub fn set_baseline(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).baseline == value {
            return;
        }
        self.get_mut(app).baseline = value;
        self.mark_needs_layout(app);
    }

    /// The type of baseline to use for positioning the child.
    pub fn baseline_type(self: RenderHandle<Self>, app: &App) -> TextBaseline {
        self.get(app).baseline_type
    }

    /// Sets [`baseline_type`](Self::baseline_type).
    pub fn set_baseline_type(self: RenderHandle<Self>, app: &mut App, value: TextBaseline) {
        if self.get(app).baseline_type == value {
            return;
        }
        self.get_mut(app).baseline_type = value;
        self.mark_needs_layout(app);
    }

    /// Flutter's `_computeSizes`: this box's size and the offset the child is shifted down by.
    fn compute_sizes(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
        get_baseline: ChildBaselineGetter,
    ) -> BaselineSizes {
        let Some(child) = self.child(app) else {
            return BaselineSizes {
                size: constraints.smallest(),
                top: 0.0,
            };
        };
        let child_constraints = constraints.loosen();
        let child_size = layout_child(app, child, child_constraints);
        let baseline_type = self.baseline_type(app);
        let child_baseline = get_baseline(app, child, child_constraints, baseline_type)
            .unwrap_or(child_size.height());
        let top = self.baseline(app) - child_baseline;
        BaselineSizes {
            size: constraints.constrain(Size::new(child_size.width(), top + child_size.height())),
            top,
        }
    }
}

impl RenderObjectWithChildMixin for RenderBaseline {
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

impl RenderShiftedBox for RenderBaseline {}

impl RenderObject for RenderBaseline {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let sizes = self.compute_sizes(
            app,
            constraints,
            ChildLayoutHelper::layout_child,
            ChildLayoutHelper::get_baseline,
        );
        self.set_size(app, sizes.size);
        if let Some(child) = self.child(app) {
            child.parent_data_of_mut::<BoxParentData>(app).offset = Offset::new(0.0, sizes.top);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

impl RenderBox for RenderBaseline {
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
        self.compute_sizes(
            app,
            constraints,
            ChildLayoutHelper::dry_layout_child,
            ChildLayoutHelper::get_dry_baseline,
        )
        .size
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = constraints.loosen();
        let baseline_type = self.baseline_type(app);
        let result1 = child.get_dry_baseline(app, child_constraints, baseline)?;
        let result2 = child.get_dry_baseline(app, child_constraints, baseline_type)?;
        Some(self.baseline(app) + result1 - result2)
    }
}

/// A delegate for computing the layout of a render object with a single child.
///
/// Used by [`RenderCustomSingleChildLayoutBox`] and `CustomSingleChildLayout` (in the widgets
/// library).
///
/// When asked to layout, [`RenderCustomSingleChildLayoutBox`] first calls
/// [`get_size`](Self::get_size) with its incoming constraints to determine its size. It then
/// uses [`get_constraints_for_child`](Self::get_constraints_for_child) to determine the
/// constraints to apply to the child. After the child completes its layout,
/// [`RenderCustomSingleChildLayoutBox`] calls [`get_position_for_child`](Self::get_position_for_child)
/// to determine the child's position.
///
/// The [`relayout`](Self::relayout) listenable causes the layout to update whenever it
/// notifies its listeners.
pub trait SingleChildLayoutDelegate: Debug + 'static {
    /// The [`Listenable`] the layout will update on, Dart's `relayout` constructor argument.
    ///
    /// Defaults to `None`.
    fn relayout(&self) -> Option<&Rc<dyn Listenable>> {
        None
    }

    /// The size of this object given the incoming constraints.
    ///
    /// Defaults to the biggest size that satisfies the given constraints.
    fn get_size(&self, constraints: BoxConstraints) -> Size {
        constraints.biggest()
    }

    /// The constraints for the child given the incoming constraints.
    ///
    /// During layout, the child is given the layout constraints returned by this function. The
    /// child is required to pick a size for itself that satisfies these constraints.
    ///
    /// Defaults to the given constraints.
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints
    }

    /// The position where the child should be placed.
    ///
    /// The `size` argument is the size of the parent, which might be different from the value
    /// returned by [`get_size`](Self::get_size) if that size doesn't satisfy the constraints
    /// passed to [`get_size`](Self::get_size). The `child_size` argument is the size of the
    /// child, which will satisfy the constraints returned by
    /// [`get_constraints_for_child`](Self::get_constraints_for_child).
    ///
    /// Defaults to positioning the child in the upper left corner of the parent.
    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let _ = (size, child_size);
        Offset::ZERO
    }

    /// Called whenever a new instance of the custom layout delegate class is provided to the
    /// [`RenderCustomSingleChildLayoutBox`] object, or any time that a new `CustomSingleChildLayout`
    /// object is created with a new instance of the custom layout delegate class (which amounts
    /// to the same thing, because the latter is implemented in terms of the former).
    ///
    /// If the new instance represents different information than the old instance, then the
    /// method should return true, otherwise it should return false.
    ///
    /// If the method returns false, then the [`get_size`](Self::get_size),
    /// [`get_constraints_for_child`](Self::get_constraints_for_child), and
    /// [`get_position_for_child`](Self::get_position_for_child) calls might be optimized away.
    ///
    /// It's possible that the layout methods will get called even if [`should_relayout`](Self::should_relayout)
    /// returns false (e.g. if an ancestor changed its layout). It's also possible that the
    /// layout method will get called without [`should_relayout`](Self::should_relayout) being
    /// called at all (e.g. if the parent changes size).
    ///
    /// `old_delegate` is of the same concrete type, Dart's `covariant`: narrow it with
    /// [`as_any`](Self::as_any).
    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool;

    /// The concrete delegate, for Dart's `runtimeType` comparison and its `covariant`
    /// narrowing of [`should_relayout`](Self::should_relayout)'s argument.
    fn as_any(&self) -> &dyn Any;
}

/// Defers the layout of its single child to a delegate.
///
/// The delegate can determine the layout constraints for the child and can decide where to
/// position the child. The delegate can also determine the size of the parent, but the size of
/// the parent cannot depend on the size of the child.
pub struct RenderCustomSingleChildLayoutBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    delegate: Rc<dyn SingleChildLayoutDelegate>,
}

impl RenderCustomSingleChildLayoutBox {
    /// Creates a render box that defers its layout to a delegate.
    pub fn new(
        app: &mut App,
        delegate: Rc<dyn SingleChildLayoutDelegate>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderCustomSingleChildLayoutBox> {
        let this = RenderHandle::new_box(
            app,
            RenderCustomSingleChildLayoutBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                delegate,
            },
        );
        this.set_child(app, child);
        this
    }

    /// A delegate that controls this object's layout.
    pub fn delegate(self: RenderHandle<Self>, app: &App) -> Rc<dyn SingleChildLayoutDelegate> {
        Rc::clone(&self.get(app).delegate)
    }

    /// Sets [`delegate`](Self::delegate).
    pub fn set_delegate(
        self: RenderHandle<Self>,
        app: &mut App,
        new_delegate: Rc<dyn SingleChildLayoutDelegate>,
    ) {
        let old_delegate = Rc::clone(&self.get(app).delegate);
        if Rc::ptr_eq(&old_delegate, &new_delegate) {
            return;
        }
        if new_delegate.as_any().type_id() != old_delegate.as_any().type_id()
            || new_delegate.should_relayout(&*old_delegate)
        {
            self.mark_needs_layout(app);
        }
        self.get_mut(app).delegate = Rc::clone(&new_delegate);
        if self.attached(app) {
            if let Some(relayout) = old_delegate.relayout() {
                relayout.remove_listener(app, &self.relayout_listener());
            }
            if let Some(relayout) = new_delegate.relayout() {
                relayout.add_listener(app, self.relayout_listener());
            }
        }
    }

    /// The `markNeedsLayout` tear-off, equal to itself across registrations.
    fn relayout_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), mark_needs_layout_custom_single_child)
    }

    /// Dart's `_getSize`.
    fn get_size(self: RenderHandle<Self>, app: &App, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.get(app).delegate.get_size(constraints))
    }
}

fn mark_needs_layout_custom_single_child(
    this: Handle<RenderCustomSingleChildLayoutBox>,
    app: &mut App,
) {
    RenderHandle::from_handle(this).mark_needs_layout(app);
}

impl RenderObjectWithChildMixin for RenderCustomSingleChildLayoutBox {
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

impl RenderShiftedBox for RenderCustomSingleChildLayoutBox {}

impl RenderObject for RenderCustomSingleChildLayoutBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.get_size(app, constraints);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            let child_constraints = self
                .get(app)
                .delegate
                .get_constraints_for_child(constraints);
            debug_assert!(child_constraints.debug_assert_is_valid(true));
            child.layout(app, child_constraints, !child_constraints.is_tight());
            let child_size = if child_constraints.is_tight() {
                child_constraints.smallest()
            } else {
                child.size(app)
            };
            child.parent_data_of_mut::<BoxParentData>(app).offset = self
                .get(app)
                .delegate
                .get_position_for_child(size, child_size);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        if let Some(relayout) = self.delegate(app).relayout() {
            relayout.add_listener(app, self.relayout_listener());
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(relayout) = self.delegate(app).relayout() {
            relayout.remove_listener(app, &self.relayout_listener());
        }
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }
}

impl RenderBox for RenderCustomSingleChildLayoutBox {
    crate::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    // TODO(ianh): It's a bit dubious to be using the getSize function from the delegate to
    // figure out the intrinsic dimensions. We really should either not support intrinsics,
    // or we should expose intrinsic delegate callbacks and throw if they're not implemented.

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let width = self
            .get_size(app, BoxConstraints::tight_for_finite(f64::INFINITY, height))
            .width();
        if width.is_finite() { width } else { 0.0 }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let width = self
            .get_size(app, BoxConstraints::tight_for_finite(f64::INFINITY, height))
            .width();
        if width.is_finite() { width } else { 0.0 }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let height = self
            .get_size(app, BoxConstraints::tight_for_finite(width, f64::INFINITY))
            .height();
        if height.is_finite() { height } else { 0.0 }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let height = self
            .get_size(app, BoxConstraints::tight_for_finite(width, f64::INFINITY))
            .height();
        if height.is_finite() { height } else { 0.0 }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.get_size(app, constraints)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self
            .get(app)
            .delegate
            .get_constraints_for_child(constraints);
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = if child_constraints.is_tight() {
            child_constraints.smallest()
        } else {
            child.get_dry_layout(app, child_constraints)
        };
        Some(
            result
                + self
                    .get(app)
                    .delegate
                    .get_position_for_child(self.get_size(app, constraints), child_size)
                    .dy(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::Size;
    use reveal_foundation::AppCell;

    use crate::box_::BoxConstraints;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::{RenderConstrainedBox, RenderRepaintBoundary};
    use reveal_embedder::valo::Op;

    /// `padding_test.dart`: the padding is added to the child's intrinsics, and taken off the
    /// extent handed to the child.
    #[test]
    fn padding_intrinsics_add_the_insets() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(8.0),
            None,
            Some(child.as_box()),
        );
        let padded = padding.as_box();
        assert_eq!(padded.get_min_intrinsic_width(&mut app, 100.0), 96.0);
        assert_eq!(padded.get_max_intrinsic_width(&mut app, 100.0), 96.0);
        assert_eq!(padded.get_min_intrinsic_height(&mut app, 100.0), 56.0);
        assert_eq!(padded.get_max_intrinsic_height(&mut app, 100.0), 56.0);
    }

    /// Without a child, the padding is all there is.
    #[test]
    fn padding_intrinsics_without_a_child_are_the_insets() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let padding = RenderPadding::new(&mut app, EdgeInsetsGeometry::all(10.0), None, None);
        assert_eq!(
            padding.as_box().get_min_intrinsic_width(&mut app, 0.0),
            20.0
        );
        assert_eq!(
            padding.as_box().get_max_intrinsic_height(&mut app, 0.0),
            20.0
        );
    }

    /// The dry layout of a padding is the size it lays out to.
    #[test]
    fn padding_dry_layout_matches_its_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(8.0),
            None,
            Some(child.as_box()),
        );
        let constraints = BoxConstraints::new().max_width(500.0).max_height(500.0);
        let dry = padding.as_box().get_dry_layout(&mut app, constraints);
        padding.layout(&mut app, constraints, false);
        assert_eq!(dry, padding.size(&app));
    }

    /// `unconstrained_box_test.dart`: an `UnconstrainedBox` is a
    /// `RenderConstraintsTransformBox` whose transform drops every constraint, so the child takes
    /// its natural size and is aligned within what the parent allows.
    #[test]
    fn constraints_transform_box_leaves_the_child_unconstrained_and_aligns_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(50.0, 20.0)), None);
        let unconstrained = RenderConstraintsTransformBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            |_| BoxConstraints::new(),
            Some(child.as_box()),
        );
        unconstrained.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(child.size(&app), Size::new(50.0, 20.0));
        assert_eq!(unconstrained.size(&app), Size::new(100.0, 100.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(25.0, 40.0)
        );
    }

    /// A child that does not fit is clipped when the clip behavior asks for it.
    #[test]
    fn constraints_transform_box_clips_an_overflowing_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(200.0, 200.0)),
            None,
        );
        let unconstrained = RenderConstraintsTransformBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            |_| BoxConstraints::new(),
            Some(child.as_box()),
        );
        unconstrained.set_clip_behavior(&mut app, Clip::HardEdge);
        let root = RenderRepaintBoundary::new(&mut app, Some(unconstrained.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        let paint_root = OffsetLayer::new(&mut app, Offset::ZERO);
        paint_root
            .as_layer()
            .attach(&mut app, root.as_object().id());
        root.as_object()
            .schedule_initial_paint(&mut app, paint_root.as_container_layer());
        owner.flush_compositing_bits(&mut app);
        owner.flush_paint(&mut app);

        assert_eq!(unconstrained.size(&app), Size::new(100.0, 100.0));
        let layer = root.as_object().debug_layer(&app).expect("painted");
        let clipped = layer
            .depth_first_iterate_children(&app)
            .into_iter()
            .filter_map(|child| {
                app.handle::<PictureLayer>(child.id())
                    .and_then(|picture| picture.picture(&app).map(|p| p.ops().to_vec()))
            })
            .flatten()
            .any(|op| matches!(op, Op::ClipPath { .. }));
        assert!(clipped, "the overflowing child is clipped");
    }

    /// A `RenderConstrainedOverflowBox` lets its child overflow and keeps the parent's size.
    #[test]
    fn constrained_overflow_box_sizes_itself_to_the_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderConstrainedBox::new(&mut app, BoxConstraints::new(), None);
        let overflow = RenderConstrainedOverflowBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            Some(child.as_box()),
        );
        overflow.set_min_width(&mut app, Some(200.0));
        overflow.set_max_width(&mut app, Some(200.0));
        overflow.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(overflow.size(&app), Size::new(100.0, 100.0));
        assert_eq!(child.size(&app).width(), 200.0);
        assert_eq!(
            child.as_box().box_parent_data(&app).offset.dx(),
            -50.0,
            "the child is centered, so it overflows both sides"
        );
    }

    /// A `RenderSizedOverflowBox` takes its requested size and passes the original constraints
    /// through, and reports the requested size as its intrinsics.
    #[test]
    fn sized_overflow_box_takes_its_requested_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(30.0, 30.0)), None);
        let sized = RenderSizedOverflowBox::new(
            &mut app,
            Size::new(50.0, 50.0),
            AlignmentGeometry::CENTER,
            None,
            Some(child.as_box()),
        );
        sized.layout(&mut app, BoxConstraints::new().max_width(100.0), false);
        assert_eq!(sized.size(&app), Size::new(50.0, 50.0));
        assert_eq!(sized.as_box().get_min_intrinsic_width(&mut app, 0.0), 50.0);
        assert_eq!(sized.as_box().get_max_intrinsic_height(&mut app, 0.0), 50.0);
    }

    /// A `RenderFractionallySizedOverflowBox` tightens the child to a fraction of the incoming
    /// maximum, and divides its intrinsics by the factor.
    #[test]
    fn fractionally_sized_box_tightens_the_child_to_a_fraction() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderConstrainedBox::new(&mut app, BoxConstraints::new(), None);
        let fraction = RenderFractionallySizedOverflowBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            Some(child.as_box()),
        );
        fraction.set_width_factor(&mut app, Some(0.5));
        fraction.layout(
            &mut app,
            BoxConstraints::new().max_width(200.0).max_height(80.0),
            false,
        );
        assert_eq!(child.size(&app).width(), 100.0);
        assert_eq!(fraction.size(&app).width(), 100.0);
    }

    /// `baseline_test.dart`: the child is shifted down so that its baseline sits at the given
    /// distance from the top.
    #[test]
    fn baseline_shifts_the_child_down_to_its_baseline() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)), None);
        let baseline = RenderBaseline::new(
            &mut app,
            30.0,
            TextBaseline::Alphabetic,
            Some(child.as_box()),
        );
        baseline.layout(&mut app, BoxConstraints::new(), false);
        // The child has no baseline, so its bottom is used: the shift is 30 - 20.
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(0.0, 10.0)
        );
        assert_eq!(baseline.size(&app), Size::new(20.0, 30.0));
    }

    #[test]
    fn padding_around_tight_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(8.0),
            None,
            Some(child.as_box()),
        );
        padding.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(child.size(&app), Size::new(80.0, 40.0));
        assert_eq!(padding.size(&app), Size::new(96.0, 56.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(8.0, 8.0)
        );
    }

    #[test]
    fn positioned_box_centers_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)), None);
        let positioned = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            None,
            None,
            Some(child.as_box()),
        );
        positioned.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(positioned.size(&app), Size::new(100.0, 100.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(40.0, 40.0)
        );
    }

    #[test]
    fn positioned_box_shrink_wraps_with_a_factor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)), None);
        let positioned = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            Some(2.0),
            None,
            Some(child.as_box()),
        );
        positioned.layout(
            &mut app,
            BoxConstraints::new().max_width(100.0).max_height(100.0),
            false,
        );
        assert_eq!(positioned.size(&app), Size::new(40.0, 100.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(10.0, 40.0)
        );
    }

    #[test]
    fn padding_without_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let padding = RenderPadding::new(&mut app, EdgeInsetsGeometry::all(10.0), None, None);
        padding.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(padding.size(&app), Size::new(100.0, 100.0));
    }

    #[derive(Debug)]
    struct OffsetDelegate {
        offset: Offset,
    }

    impl SingleChildLayoutDelegate for OffsetDelegate {
        fn get_size(&self, constraints: BoxConstraints) -> Size {
            constraints.biggest()
        }

        fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
            constraints.loosen()
        }

        fn get_position_for_child(&self, _size: Size, _child_size: Size) -> Offset {
            self.offset
        }

        fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
            old_delegate
                .as_any()
                .downcast_ref::<OffsetDelegate>()
                .is_none_or(|old| old.offset != self.offset)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn custom_single_child_layout_positions_the_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(20.0, 10.0)),
            None,
        );
        let parent = RenderCustomSingleChildLayoutBox::new(
            &mut app,
            Rc::new(OffsetDelegate {
                offset: Offset::new(8.0, 4.0),
            }),
            Some(child.as_box()),
        );
        parent.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 50.0)),
            false,
        );
        assert_eq!(parent.size(&app), Size::new(100.0, 50.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(8.0, 4.0)
        );
    }
}
