//! Flutter counterpart: `rendering/stack.dart` (`RelativeRect`, `StackParentData`, `StackFit`,
//! `RenderStack`, `RenderIndexedStack`).
//!
//! Baselines wait; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};

use inset_embedder::{Clip, Offset, Rect, Size, TextDirection, lerp_double};
use inset_foundation::{App, Handle};
use inset_painting::{Alignment, AlignmentGeometry};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestResult, BoxParentData, ContainerBoxParentData,
    RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
};
use crate::layer::{ClipRectLayer, LayerHandle};
use crate::layout_helper::ChildLayoutHelper;
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// An immutable 2D, axis-aligned, floating-point rectangle whose coordinates are given relative
/// to another rectangle's edges, known as the container.
///
/// Since the dimensions of the rectangle are relative to those of the container, this type has no
/// width and height members. To determine the width or height of the rectangle, convert it to a
/// [`Rect`] using [`to_rect`](Self::to_rect) (passing the container's own [`Rect`]), and then
/// examine that object.
#[derive(Clone, Copy, PartialEq)]
pub struct RelativeRect {
    /// Distance from the left side of the container to the left side of this rectangle.
    ///
    /// May be negative if the left side of the rectangle is outside of the container.
    pub left: f64,

    /// Distance from the top side of the container to the top side of this rectangle.
    ///
    /// May be negative if the top side of the rectangle is outside of the container.
    pub top: f64,

    /// Distance from the right side of the container to the right side of this rectangle.
    ///
    /// May be positive if the right side of the rectangle is outside of the container.
    pub right: f64,

    /// Distance from the bottom side of the container to the bottom side of this rectangle.
    ///
    /// May be positive if the bottom side of the rectangle is outside of the container.
    pub bottom: f64,
}

impl RelativeRect {
    /// Creates a [`RelativeRect`] with the given values.
    pub const fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> RelativeRect {
        RelativeRect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Creates a [`RelativeRect`] from a [`Rect`] and a [`Size`].
    ///
    /// The rect and the [`RelativeRect`] (the output) are in the coordinate space of the
    /// rectangle described by the size, with 0,0 being at the top left.
    pub fn from_size(rect: Rect, container: Size) -> RelativeRect {
        RelativeRect {
            left: rect.left,
            top: rect.top,
            right: container.width() - rect.right,
            bottom: container.height() - rect.bottom,
        }
    }

    /// Creates a [`RelativeRect`] from two [`Rect`]s.
    ///
    /// `container` provides the container; `rect` provides the rectangle, in the same coordinate
    /// space, that is to be converted to a [`RelativeRect`]. The output will be in the
    /// container's coordinate space.
    ///
    /// For example, if the top left of the rect is at 0,0, and the top left of the container is
    /// at 100,100, then the top left of the output will be at -100,-100.
    ///
    /// If `rect` is actually in the container's coordinate space, then use
    /// [`from_size`](Self::from_size) and pass the container's size as the second argument
    /// instead.
    pub fn from_rect(rect: Rect, container: Rect) -> RelativeRect {
        RelativeRect {
            left: rect.left - container.left,
            top: rect.top - container.top,
            right: container.right - rect.right,
            bottom: container.bottom - rect.bottom,
        }
    }

    /// Creates a [`RelativeRect`] from horizontal position using `start` and `end` rather than
    /// `left` and `right`.
    ///
    /// If `text_direction` is [`TextDirection::Rtl`], then `start` is used for the
    /// [`right`](Self::right) property and `end` for the [`left`](Self::left) property.
    /// Otherwise, `start` is used for [`left`](Self::left) and `end` for
    /// [`right`](Self::right).
    pub fn from_directional(
        text_direction: TextDirection,
        start: f64,
        top: f64,
        end: f64,
        bottom: f64,
    ) -> RelativeRect {
        let (left, right) = match text_direction {
            TextDirection::Rtl => (end, start),
            TextDirection::Ltr => (start, end),
        };
        RelativeRect::from_ltrb(left, top, right, bottom)
    }

    /// A rect that covers the entire container.
    pub const FILL: RelativeRect = RelativeRect::from_ltrb(0.0, 0.0, 0.0, 0.0);

    /// Returns whether any of the values are greater than zero.
    ///
    /// This corresponds to one of the sides ([`left`](Self::left), [`top`](Self::top),
    /// [`right`](Self::right), or [`bottom`](Self::bottom)) having some positive inset towards
    /// the center.
    pub fn has_insets(&self) -> bool {
        self.left > 0.0 || self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0
    }

    /// Returns a new rectangle object translated by the given offset.
    pub fn shift(&self, offset: Offset) -> RelativeRect {
        RelativeRect::from_ltrb(
            self.left + offset.dx(),
            self.top + offset.dy(),
            self.right - offset.dx(),
            self.bottom - offset.dy(),
        )
    }

    /// Returns a new rectangle with edges moved outwards by the given delta.
    pub fn inflate(&self, delta: f64) -> RelativeRect {
        RelativeRect::from_ltrb(
            self.left - delta,
            self.top - delta,
            self.right - delta,
            self.bottom - delta,
        )
    }

    /// Returns a new rectangle with edges moved inwards by the given delta.
    pub fn deflate(&self, delta: f64) -> RelativeRect {
        self.inflate(-delta)
    }

    /// Returns a new rectangle that is the intersection of the given rectangle and this one.
    pub fn intersect(&self, other: RelativeRect) -> RelativeRect {
        RelativeRect::from_ltrb(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Convert this [`RelativeRect`] to a [`Rect`], in the coordinate space of the container.
    ///
    /// See also:
    ///
    ///  * [`to_size`](Self::to_size), which returns the size part of the rect, based on the size
    ///    of the container.
    pub fn to_rect(&self, container: Rect) -> Rect {
        Rect::from_ltrb(
            self.left,
            self.top,
            container.width() - self.right,
            container.height() - self.bottom,
        )
    }

    /// Convert this [`RelativeRect`] to a [`Size`], assuming a container with the given size.
    ///
    /// See also:
    ///
    ///  * [`to_rect`](Self::to_rect), which also computes the position relative to the container.
    pub fn to_size(&self, container: Size) -> Size {
        Size::new(
            container.width() - self.left - self.right,
            container.height() - self.top - self.bottom,
        )
    }

    /// Linearly interpolate between two [`RelativeRect`]s.
    ///
    /// If either rect is `None`, this function interpolates from [`FILL`](Self::FILL).
    pub fn lerp(a: Option<RelativeRect>, b: Option<RelativeRect>, t: f64) -> Option<RelativeRect> {
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(RelativeRect::from_ltrb(
                b.left * t,
                b.top * t,
                b.right * t,
                b.bottom * t,
            )),
            (Some(a), None) => {
                let k = 1.0 - t;
                Some(RelativeRect::from_ltrb(
                    a.left * k,
                    a.top * k,
                    a.right * k,
                    a.bottom * k,
                ))
            }
            (Some(a), Some(b)) => Some(RelativeRect::from_ltrb(
                lerp_double(Some(a.left), Some(b.left), t).unwrap(),
                lerp_double(Some(a.top), Some(b.top), t).unwrap(),
                lerp_double(Some(a.right), Some(b.right), t).unwrap(),
                lerp_double(Some(a.bottom), Some(b.bottom), t).unwrap(),
            )),
        }
    }
}

impl Debug for RelativeRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RelativeRect::from_ltrb({:.1}, {:.1}, {:.1}, {:.1})",
            self.left, self.top, self.right, self.bottom
        )
    }
}

/// Parent data for use with [`RenderStack`].
#[derive(Debug)]
pub struct StackParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,

    /// The distance by which the child's top edge is inset from the top of the stack.
    pub top: Option<f64>,

    /// The distance by which the child's right edge is inset from the right of the stack.
    pub right: Option<f64>,

    /// The distance by which the child's bottom edge is inset from the bottom of the stack.
    pub bottom: Option<f64>,

    /// The distance by which the child's left edge is inset from the left of the stack.
    pub left: Option<f64>,

    /// The child's width.
    ///
    /// Ignored if both [`left`](Self::left) and [`right`](Self::right) are non-null.
    pub width: Option<f64>,

    /// The child's height.
    ///
    /// Ignored if both [`top`](Self::top) and [`bottom`](Self::bottom) are non-null.
    pub height: Option<f64>,
}

impl StackParentData {
    /// Creates stack parent data for a non-positioned child.
    pub const fn new() -> StackParentData {
        StackParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            top: None,
            right: None,
            bottom: None,
            left: None,
            width: None,
            height: None,
        }
    }

    /// The current values in terms of a [`RelativeRect`].
    ///
    /// # Panics
    ///
    /// If any of the four edges is unset.
    pub fn rect(&self) -> RelativeRect {
        RelativeRect::from_ltrb(
            self.left.expect("left is set"),
            self.top.expect("top is set"),
            self.right.expect("right is set"),
            self.bottom.expect("bottom is set"),
        )
    }

    /// Sets [`rect`](Self::rect).
    pub fn set_rect(&mut self, value: RelativeRect) {
        self.top = Some(value.top);
        self.right = Some(value.right);
        self.bottom = Some(value.bottom);
        self.left = Some(value.left);
    }

    /// Whether this child is considered positioned.
    ///
    /// A child is positioned if any of the top, right, bottom, or left properties are non-null.
    /// Positioned children do not factor into determining the size of the stack but are instead
    /// placed relative to the non-positioned children in the stack.
    pub fn is_positioned(&self) -> bool {
        self.top.is_some()
            || self.right.is_some()
            || self.bottom.is_some()
            || self.left.is_some()
            || self.width.is_some()
            || self.height.is_some()
    }

    /// Computes the [`BoxConstraints`] the stack layout algorithm would give to this child,
    /// given the [`Size`] of the stack.
    ///
    /// This method should only be called when [`is_positioned`](Self::is_positioned) is true for
    /// the child.
    pub fn positioned_child_constraints(&self, stack_size: Size) -> BoxConstraints {
        debug_assert!(self.is_positioned());
        let width = match (self.left, self.right) {
            (Some(left), Some(right)) => Some(stack_size.width() - right - left),
            _ => self.width,
        };
        let height = match (self.top, self.bottom) {
            (Some(top), Some(bottom)) => Some(stack_size.height() - bottom - top),
            _ => self.height,
        };
        debug_assert!(height.is_none_or(|height| !height.is_nan()));
        debug_assert!(width.is_none_or(|width| !width.is_nan()));
        BoxConstraints::tight_for(
            width.map(|width| width.max(0.0)),
            height.map(|height| height.max(0.0)),
        )
    }
}

impl Default for StackParentData {
    fn default() -> StackParentData {
        StackParentData::new()
    }
}

impl ParentData for StackParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<StackParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<StackParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for StackParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for StackParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for StackParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut values = Vec::new();
        for (name, value) in [
            ("top", self.top),
            ("right", self.right),
            ("bottom", self.bottom),
            ("left", self.left),
            ("width", self.width),
            ("height", self.height),
        ] {
            if let Some(value) = value {
                values.push(format!("{name}={value}"));
            }
        }
        if values.is_empty() {
            values.push("not positioned".to_string());
        }
        values.push(self.box_parent_data.to_string());
        f.write_str(&values.join("; "))
    }
}

/// How to size the non-positioned children of a [`RenderStack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackFit {
    /// The constraints passed to the stack from its parent are loosened.
    ///
    /// For example, if the stack has constraints that force it to 350x600, then this would allow
    /// the non-positioned children of the stack to have any width from zero to 350 and any
    /// height from zero to 600.
    Loose,

    /// The constraints passed to the stack from its parent are tightened to the biggest size
    /// allowed.
    ///
    /// For example, if the stack has loose constraints with a width in the range 10 to 100 and a
    /// height in the range 0 to 600, then the non-positioned children of the stack would all be
    /// sized as 100 pixels wide and 600 high.
    Expand,

    /// The constraints passed to the stack from its parent are passed unmodified to the
    /// non-positioned children.
    Passthrough,
}

/// Flutter's `RenderStack` fields.
pub struct RenderStackData {
    has_visual_overflow: bool,
    alignment: AlignmentGeometry,
    text_direction: Option<TextDirection>,
    fit: StackFit,
    clip_behavior: Clip,
    resolved_alignment_cache: Option<Alignment>,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
}

impl RenderStackData {
    /// The stack defaults: children aligned by their top start corners, loosely fit, clipped.
    pub fn new() -> RenderStackData {
        RenderStackData {
            has_visual_overflow: false,
            alignment: AlignmentGeometry::TOP_START,
            text_direction: None,
            fit: StackFit::Loose,
            clip_behavior: Clip::HardEdge,
            resolved_alignment_cache: None,
            clip_rect_layer: LayerHandle::new(),
        }
    }
}

impl Default for RenderStackData {
    fn default() -> RenderStackData {
        RenderStackData::new()
    }
}

/// Helper function for calculating the intrinsic metrics of a stack.
///
/// Flutter's `RenderStack.getIntrinsicDimension`.
pub fn get_intrinsic_dimension(
    app: &mut App,
    first_child: Option<AnyRenderBox>,
    mut main_child_size_getter: impl FnMut(&mut App, AnyRenderBox) -> f64,
) -> f64 {
    let mut extent: f64 = 0.0;
    let mut child = first_child;
    while let Some(current) = child {
        let child_parent_data = current.as_object().parent_data_of::<StackParentData>(app);
        let (is_positioned, next_sibling) = (
            child_parent_data.is_positioned(),
            child_parent_data.next_sibling(),
        );
        if !is_positioned {
            extent = extent.max(main_child_size_getter(app, current));
        }
        child = next_sibling;
    }
    extent
}

/// Lays out the positioned `child` according to `alignment` within a stack of `size`.
///
/// Returns true when the child has visual overflow. Flutter's `RenderStack.layoutPositionedChild`;
/// it also takes the child's [`StackParentData`], which is read from the child here.
pub fn layout_positioned_child(
    app: &mut App,
    child: AnyRenderBox,
    size: Size,
    alignment: Alignment,
) -> bool {
    let child_constraints = {
        let child_parent_data = child.as_object().parent_data_of::<StackParentData>(app);
        debug_assert!(child_parent_data.is_positioned());
        child_parent_data.positioned_child_constraints(size)
    };
    child.layout(app, child_constraints, true);

    let child_size = child.size(app);
    let child_parent_data = child.as_object().parent_data_of::<StackParentData>(app);
    let x = match (child_parent_data.left, child_parent_data.right) {
        (Some(left), _) => left,
        (None, Some(right)) => size.width() - right - child_size.width(),
        (None, None) => alignment.along_offset(size - child_size).dx(),
    };
    let y = match (child_parent_data.top, child_parent_data.bottom) {
        (Some(top), _) => top,
        (None, Some(bottom)) => size.height() - bottom - child_size.height(),
        (None, None) => alignment.along_offset(size - child_size).dy(),
    };

    child
        .parent_data_of_mut::<StackParentData>(app)
        .set_offset(Offset::new(x, y));
    x < 0.0
        || x + child_size.width() > size.width()
        || y < 0.0
        || y + child_size.height() > size.height()
}

/// Implements the stack layout algorithm.
///
/// In a stack layout, the children are positioned on top of each other in the order in which they
/// appear in the child list. First, the non-positioned children (those with null values for top,
/// right, bottom, and left) are laid out and initially placed in the upper-left corner of the
/// stack. The stack is then sized to enclose all of the non-positioned children. If there are no
/// non-positioned children, the stack becomes as large as possible.
///
/// The final location of non-positioned children is determined by the alignment parameter. The
/// left of each non-positioned child becomes the difference between the stack's width and the
/// child's width multiplied by (alignment.x + 1.0) / 2.0. The top of each non-positioned child is
/// computed similarly from the stack's height, the child's height, and `alignment.y`.
///
/// Next, the positioned children are laid out. If a child has top and bottom values that are both
/// non-null, the child is given a fixed height determined by subtracting the sum of the top and
/// bottom values from the height of the stack. Similarly, if the child has right and left values
/// that are both non-null, the child is given a fixed width derived from the stack's width.
/// Otherwise, the child is given unbounded constraints in the non-fixed dimensions.
///
/// Once the child is laid out, the stack positions the child according to the top, right, bottom,
/// and left properties of their [`StackParentData`]. If the child extends beyond the bounds of the
/// stack, the stack will clip the child's painting to the bounds of the stack.
///
/// Flutter's `RenderStack` bodies. [`RenderStack`] is the plain leaf; [`RenderIndexedStack`]
/// implements this trait too and overrides [`paint_stack`](Self::paint_stack).
pub trait RenderStackBase:
    RenderBoxContainerDefaultsMixin<ParentDataType = StackParentData>
{
    /// Mixin field access.
    fn stack_data(self: RenderHandle<Self>, app: &App) -> &RenderStackData;

    /// See [`stack_data`](Self::stack_data).
    fn stack_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderStackData;

    /// Installs [`StackParentData`] unless the child already has it.
    ///
    /// The body of Flutter's `setupParentData` override; call it from
    /// [`RenderBox::setup_parent_data`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<StackParentData>(app) {
            child.set_parent_data(app, StackParentData::new());
        }
    }

    /// The [`alignment`](Self::alignment) resolved against
    /// [`text_direction`](Self::text_direction), cached until either changes.
    fn resolved_alignment(self: RenderHandle<Self>, app: &mut App) -> Alignment {
        if let Some(cached) = self.stack_data(app).resolved_alignment_cache {
            return cached;
        }
        let resolved = self.alignment(app).resolve(self.text_direction(app));
        self.stack_data_mut(app).resolved_alignment_cache = Some(resolved);
        resolved
    }

    /// Drops the resolved alignment and marks layout dirty.
    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.stack_data_mut(app).resolved_alignment_cache = None;
        self.mark_needs_layout(app);
    }

    /// How to align the non-positioned or partially-positioned children in the stack.
    ///
    /// The non-positioned children are placed relative to each other such that the points
    /// determined by the alignment are co-located. For example, if the alignment is
    /// `AlignmentGeometry::TOP_LEFT`, then the top left corner of each non-positioned child will
    /// be located at the same global coordinate.
    ///
    /// Partially-positioned children, those that do not specify an alignment in a particular axis
    /// (e.g. that have neither `top` nor `bottom` set), use the alignment to determine how they
    /// should be positioned in that under-specified axis.
    ///
    /// If this is set to a directional alignment, then [`text_direction`](Self::text_direction)
    /// must not be `None`.
    fn alignment(self: RenderHandle<Self>, app: &App) -> AlignmentGeometry {
        self.stack_data(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: AlignmentGeometry) {
        if self.stack_data(app).alignment == value {
            return;
        }
        self.stack_data_mut(app).alignment = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`alignment`](Self::alignment).
    ///
    /// This may be changed to `None`, but only after the alignment has been changed to a value
    /// that does not depend on the direction.
    fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.stack_data(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: Option<TextDirection>) {
        if self.stack_data(app).text_direction == value {
            return;
        }
        self.stack_data_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    /// How to size the non-positioned children in the stack.
    ///
    /// The constraints passed into the stack from its parent are either loosened
    /// ([`StackFit::Loose`]) or tightened to their biggest size ([`StackFit::Expand`]).
    fn fit(self: RenderHandle<Self>, app: &App) -> StackFit {
        self.stack_data(app).fit
    }

    /// Sets [`fit`](Self::fit).
    fn set_fit(self: RenderHandle<Self>, app: &mut App, value: StackFit) {
        if self.stack_data(app).fit != value {
            self.stack_data_mut(app).fit = value;
            self.mark_needs_layout(app);
        }
    }

    /// How to clip the children that overflow the stack.
    ///
    /// Stacks only clip children whose geometry overflows the stack. A child that paints outside
    /// its bounds (e.g. a box with a shadow) will not be clipped, regardless of the value of this
    /// property. Similarly, a child that itself has a descendant that overflows the stack will
    /// not be clipped, as only the geometry of the stack's direct children are considered.
    ///
    /// Defaults to [`Clip::HardEdge`].
    fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.stack_data(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value != self.stack_data(app).clip_behavior {
            self.stack_data_mut(app).clip_behavior = value;
            self.mark_needs_paint(app);
        }
    }

    /// The stack's minimum intrinsic width: the widest non-positioned child's.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let first_child = self.first_child(app);
        get_intrinsic_dimension(app, first_child, |app, child| {
            child.get_min_intrinsic_width(app, height)
        })
    }

    /// The stack's maximum intrinsic width: the widest non-positioned child's.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let first_child = self.first_child(app);
        get_intrinsic_dimension(app, first_child, |app, child| {
            child.get_max_intrinsic_width(app, height)
        })
    }

    /// The stack's minimum intrinsic height: the tallest non-positioned child's.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let first_child = self.first_child(app);
        get_intrinsic_dimension(app, first_child, |app, child| {
            child.get_min_intrinsic_height(app, width)
        })
    }

    /// The stack's maximum intrinsic height: the tallest non-positioned child's.
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let first_child = self.first_child(app);
        get_intrinsic_dimension(app, first_child, |app, child| {
            child.get_max_intrinsic_height(app, width)
        })
    }

    /// The size the stack would take, measuring its non-positioned children dry.
    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    /// Flutter's `_computeSize`: measures the non-positioned children with `layout_child` and
    /// sizes the stack.
    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        mut layout_child: impl FnMut(&mut App, AnyRenderBox, BoxConstraints) -> Size,
    ) -> Size {
        let mut has_non_positioned_children = false;
        if self.child_count(app) == 0 {
            return if constraints.biggest().is_finite() {
                constraints.biggest()
            } else {
                constraints.smallest()
            };
        }

        let mut width = constraints.min_width;
        let mut height = constraints.min_height;

        let non_positioned_constraints = match self.fit(app) {
            StackFit::Loose => constraints.loosen(),
            StackFit::Expand => BoxConstraints::tight(constraints.biggest()),
            StackFit::Passthrough => constraints,
        };

        let mut child = self.first_child(app);
        while let Some(current) = child {
            let is_positioned = current
                .as_object()
                .parent_data_of::<StackParentData>(app)
                .is_positioned();

            if !is_positioned {
                has_non_positioned_children = true;

                let child_size = layout_child(app, current, non_positioned_constraints);

                width = width.max(child_size.width());
                height = height.max(child_size.height());
            }

            child = self.child_after(app, current);
        }

        let size = if has_non_positioned_children {
            let size = Size::new(width, height);
            debug_assert_eq!(size.width(), constraints.constrain_width(width));
            debug_assert_eq!(size.height(), constraints.constrain_height(height));
            size
        } else {
            constraints.biggest()
        };

        debug_assert!(
            size.is_finite(),
            "A Stack requires bounded constraints from its parent. This error commonly occurs \
             when a Stack is placed inside a widget like Column, ListView, or other widgets that \
             do not constrain their children. To fix this, wrap the Stack in a widget that \
             provides finite height and width constraints, such as a SizedBox or ConstrainedBox. \
             Use Expanded only if the parent is a Flex widget like Row or Column."
        );
        size
    }

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.stack_data_mut(app).has_visual_overflow = false;

        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);

        let resolved_alignment = self.resolved_alignment(app);
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let is_positioned = current
                .as_object()
                .parent_data_of::<StackParentData>(app)
                .is_positioned();

            if !is_positioned {
                let offset = resolved_alignment.along_offset(size - current.size(app));
                current
                    .parent_data_of_mut::<StackParentData>(app)
                    .set_offset(offset);
            } else {
                let overflowed = layout_positioned_child(app, current, size, resolved_alignment);
                self.stack_data_mut(app).has_visual_overflow |= overflowed;
            }

            child = self.child_after(app, current);
        }
    }

    /// Override in subclasses to customize how the stack paints.
    ///
    /// By default, the stack uses [`default_paint`](RenderBoxContainerDefaultsMixin::default_paint).
    /// This function is called by [`paint`](Self::paint) after potentially applying a clip to
    /// contain visual overflow.
    fn paint_stack(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.default_paint(app, context, offset);
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let clip_behavior = self.clip_behavior(app);
        if clip_behavior != Clip::None && self.stack_data(app).has_visual_overflow {
            let size = self.size(app);
            let old = self.stack_data(app).clip_rect_layer.layer();
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                Offset::ZERO & size,
                |app, context, offset| self.paint_stack(app, context, offset),
                clip_behavior,
                old,
            );
            LayerHandle::set_layer(
                app,
                |app| &mut self.stack_data_mut(app).clip_rect_layer,
                layer,
            );
        } else {
            LayerHandle::set_layer(
                app,
                |app| &mut self.stack_data_mut(app).clip_rect_layer,
                None,
            );
            self.paint_stack(app, context, offset);
        }
    }

    /// Flutter's `dispose`: drop the clip layer handle, then `super.dispose()`.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(
            app,
            |app| &mut self.stack_data_mut(app).clip_rect_layer,
            None,
        );
        crate::object::RenderObjectBase::dispose(self, app);
    }
}

/// The plain [`RenderStackBase`]: paints every child in order.
pub struct RenderStack {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    stack: RenderStackData,
}

impl RenderStack {
    /// Creates a stack render object.
    ///
    /// By default, the non-positioned children of the stack are aligned by their top start
    /// corners. Add children with [`add`](ContainerRenderObjectMixin::add) or
    /// [`add_all`](ContainerRenderObjectMixin::add_all); Dart's constructor takes them.
    pub fn new(app: &mut App) -> RenderHandle<RenderStack> {
        RenderHandle::new_box(
            app,
            RenderStack {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                stack: RenderStackData::new(),
            },
        )
    }
}

impl ContainerRenderObjectMixin for RenderStack {
    type ChildType = AnyRenderBox;
    type ParentDataType = StackParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderStack {}

impl RenderStackBase for RenderStack {
    fn stack_data(self: RenderHandle<Self>, app: &App) -> &RenderStackData {
        &self.get(app).stack
    }

    fn stack_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderStackData {
        &mut self.get_mut(app).stack
    }
}

impl RenderObject for RenderStack {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderStackBase::perform_layout(self, app)
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderStackBase::paint(self, app, context, offset)
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderStackBase::dispose(self, app)
    }
}

impl RenderBox for RenderStack {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderStackBase::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderStackBase::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderStackBase::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderStackBase::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderStackBase::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderStackBase::compute_dry_layout(self, app, constraints)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.default_hit_test_children(app, result, position)
    }
}

/// Implements the same layout algorithm as [`RenderStack`] but only paints the child specified by
/// [`index`](Self::index).
///
/// Although only one child is displayed, the cost of the layout algorithm is still O(N), like an
/// ordinary stack.
pub struct RenderIndexedStack {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    stack: RenderStackData,
    index: Option<i32>,
}

impl RenderIndexedStack {
    /// Creates a stack render object that paints a single child.
    ///
    /// If [`index`](Self::index) is set to `None`, nothing is displayed.
    pub fn new(app: &mut App) -> RenderHandle<RenderIndexedStack> {
        RenderHandle::new_box(
            app,
            RenderIndexedStack {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                stack: RenderStackData::new(),
                index: Some(0),
            },
        )
    }

    /// The index of the child to show, `None` if nothing is to be displayed.
    pub fn index(self: RenderHandle<Self>, app: &App) -> Option<i32> {
        self.get(app).index
    }

    /// Sets [`index`](Self::index).
    pub fn set_index(self: RenderHandle<Self>, app: &mut App, value: Option<i32>) {
        if self.get(app).index != value {
            self.get_mut(app).index = value;
            self.mark_needs_layout(app);
        }
    }

    fn child_at_index(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        let index = self.index(app)?;
        let mut child = self.first_child(app);
        let mut i = 0;
        while i < index
            && let Some(current) = child
        {
            child = self.child_after(app, current);
            i += 1;
        }
        debug_assert!(self.first_child(app).is_none() || child.is_some());
        child
    }
}

impl ContainerRenderObjectMixin for RenderIndexedStack {
    type ChildType = AnyRenderBox;
    type ParentDataType = StackParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderIndexedStack {}

impl RenderStackBase for RenderIndexedStack {
    fn stack_data(self: RenderHandle<Self>, app: &App) -> &RenderStackData {
        &self.get(app).stack
    }

    fn stack_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderStackData {
        &mut self.get_mut(app).stack
    }

    fn paint_stack(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(displayed_child) = self.child_at_index(app) else {
            return;
        };
        let child_offset = displayed_child
            .as_object()
            .parent_data_of::<StackParentData>(app)
            .offset();
        context.paint_child(app, displayed_child.as_object(), child_offset + offset);
    }
}

impl RenderObject for RenderIndexedStack {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderStackBase::perform_layout(self, app)
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderStackBase::paint(self, app, context, offset)
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderStackBase::dispose(self, app)
    }
}

impl RenderBox for RenderIndexedStack {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderStackBase::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderStackBase::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderStackBase::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderStackBase::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderStackBase::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderStackBase::compute_dry_layout(self, app, constraints)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(displayed_child) = self.child_at_index(app) else {
            return false;
        };
        let child_offset = displayed_child
            .as_object()
            .parent_data_of::<StackParentData>(app)
            .offset();
        result.add_with_paint_offset(Some(child_offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position - child_offset);
            displayed_child.hit_test(app, result, transformed)
        })
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::flex::{MainAxisSize, RenderFlex};
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::{RenderConstrainedBox, RenderRepaintBoundary};
    use crate::shifted_box::RenderPadding;
    use inset_embedder::valo::Op;
    use inset_painting::EdgeInsetsGeometry;

    /// A leaf that takes its preferred size where the constraints allow, and counts its paints.
    struct SizedTestBox {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        preferred_size: Size,
        paints: Rc<Cell<u32>>,
    }

    impl RenderObject for SizedTestBox {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let preferred_size = self.get(app).preferred_size;
            let size = self.constraints(app).constrain(preferred_size);
            self.set_size(app, size);
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
        ) {
            let paints = self.get(app).paints.clone();
            paints.set(paints.get() + 1);
        }
    }

    impl RenderBox for SizedTestBox {
        crate::render_box_accessors!();
    }

    fn sized_box(app: &mut App, size: Size) -> (AnyRenderBox, Rc<Cell<u32>>) {
        let paints = Rc::new(Cell::new(0));
        let box_ = RenderHandle::new_box(
            app,
            SizedTestBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                preferred_size: size,
                paints: paints.clone(),
            },
        )
        .as_box();
        (box_, paints)
    }

    fn schedule_root_paint(app: &mut App, node: AnyRenderObject) {
        let root = OffsetLayer::new(app, Offset::ZERO);
        root.as_layer().attach(app, node.id());
        node.schedule_initial_paint(app, root.as_container_layer());
    }

    /// The test binding's first frame: attach, lay the root out, schedule and flush paint.
    fn first_frame(
        app: &mut App,
        root: AnyRenderBox,
        constraints: BoxConstraints,
    ) -> Handle<PipelineOwner> {
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.layout(app, constraints, false);
        schedule_root_paint(app, root.as_object());
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
        owner
    }

    fn picture_has_clip(app: &App, boundary: AnyRenderObject) -> bool {
        let layer = boundary.debug_layer(app).expect("painted");
        layer
            .depth_first_iterate_children(app)
            .into_iter()
            .filter_map(|child| {
                app.handle::<PictureLayer>(child.id())
                    .and_then(|picture| picture.picture(app).map(|p| p.ops().to_vec()))
            })
            .flatten()
            .any(|op| matches!(op, Op::ClipPath { .. }))
    }

    /// `stack_test.dart`: a positioned child is sized and placed by its [`RelativeRect`], and does
    /// not take part in sizing the stack.
    /// `stack_test.dart`: a stack's intrinsics are the largest of its non-positioned children's,
    /// and a positioned child does not count.
    #[test]
    fn stack_intrinsics_are_the_largest_non_positioned_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(30.0, 40.0)), None);
        let second =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(50.0, 20.0)), None);
        let positioned = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(200.0, 200.0)),
            None,
        );
        let stack = RenderStack::new(&mut app);
        stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
        stack.add_all(
            &mut app,
            Some(vec![first.as_box(), second.as_box(), positioned.as_box()]),
        );
        positioned
            .as_box()
            .parent_data_of_mut::<StackParentData>(&mut app)
            .left = Some(0.0);

        let box_ = stack.as_box();
        assert_eq!(box_.get_min_intrinsic_width(&mut app, 0.0), 50.0);
        assert_eq!(box_.get_max_intrinsic_width(&mut app, 0.0), 50.0);
        assert_eq!(box_.get_min_intrinsic_height(&mut app, 0.0), 40.0);
        assert_eq!(box_.get_max_intrinsic_height(&mut app, 0.0), 40.0);
    }

    /// The dry layout of a tree of a padding inside a row inside a stack is the size that tree
    /// lays out to.
    #[test]
    fn dry_layout_matches_layout_for_a_padding_in_a_flex_in_a_stack() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let leaf =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(40.0, 20.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(5.0),
            None,
            Some(leaf.as_box()),
        );
        let row = RenderFlex::new(&mut app);
        row.set_text_direction(&mut app, Some(TextDirection::Ltr));
        row.set_main_axis_size(&mut app, MainAxisSize::Min);
        row.add_all(&mut app, Some(vec![padding.as_box()]));
        let stack = RenderStack::new(&mut app);
        stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
        stack.add_all(&mut app, Some(vec![row.as_box()]));

        let constraints = BoxConstraints::new().max_width(300.0).max_height(300.0);
        let dry = stack.as_box().get_dry_layout(&mut app, constraints);
        stack.layout(&mut app, constraints, false);
        assert_eq!(dry, Size::new(50.0, 30.0));
        assert_eq!(dry, stack.size(&app));
        assert_eq!(row.size(&app), Size::new(50.0, 30.0));
    }

    #[test]
    fn positioned_child_is_laid_out_by_its_rect() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (child, _) = sized_box(&mut app, Size::new(10.0, 10.0));
        let stack = RenderStack::new(&mut app);
        stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
        stack.add(&mut app, child);
        child
            .parent_data_of_mut::<StackParentData>(&mut app)
            .set_rect(RelativeRect::from_ltrb(10.0, 20.0, 30.0, 40.0));

        stack.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        // No non-positioned child, so the stack is as large as possible.
        assert_eq!(stack.size(&app), Size::new(100.0, 100.0));
        assert_eq!(child.size(&app), Size::new(60.0, 40.0));
        assert_eq!(child.box_parent_data(&app).offset, Offset::new(10.0, 20.0));
    }

    /// `stack_test.dart`: `Stack fit`. The fit decides the constraints the non-positioned
    /// children get, and the stack sizes itself to them.
    #[test]
    fn non_positioned_children_are_sized_by_the_fit() {
        let cases = [
            (
                StackFit::Loose,
                Size::new(20.0, 20.0),
                Size::new(50.0, 50.0),
            ),
            (
                StackFit::Expand,
                Size::new(100.0, 100.0),
                Size::new(100.0, 100.0),
            ),
            (
                StackFit::Passthrough,
                Size::new(50.0, 50.0),
                Size::new(50.0, 50.0),
            ),
        ];
        for (fit, child_size, stack_size) in cases {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let (child, _) = sized_box(&mut app, Size::new(20.0, 20.0));
            let stack = RenderStack::new(&mut app);
            stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
            stack.set_fit(&mut app, fit);
            stack.add(&mut app, child);

            stack.layout(
                &mut app,
                BoxConstraints {
                    min_width: 50.0,
                    max_width: 100.0,
                    min_height: 50.0,
                    max_height: 100.0,
                },
                false,
            );

            assert_eq!(child.size(&app), child_size, "{fit:?}");
            assert_eq!(stack.size(&app), stack_size, "{fit:?}");
        }
    }

    /// `stack_test.dart`: `RenderIndexedStack` lays every child out but paints only the one at
    /// [`RenderIndexedStack::index`].
    #[test]
    fn indexed_stack_paints_one_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (first, first_paints) = sized_box(&mut app, Size::new(20.0, 20.0));
        let (second, second_paints) = sized_box(&mut app, Size::new(30.0, 30.0));
        let stack = RenderIndexedStack::new(&mut app);
        stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
        stack.add_all(&mut app, Some(vec![first, second]));
        stack.set_index(&mut app, Some(1));
        let root = RenderRepaintBoundary::new(&mut app, Some(stack.as_box()));

        let owner = first_frame(
            &mut app,
            root.as_box(),
            BoxConstraints::tight(Size::new(100.0, 100.0)),
        );

        // Both children are laid out; only the one at the index is painted.
        assert_eq!(first.size(&app), Size::new(20.0, 20.0));
        assert_eq!(second.size(&app), Size::new(30.0, 30.0));
        assert_eq!(first_paints.get(), 0);
        assert_eq!(second_paints.get(), 1);

        stack.set_index(&mut app, Some(0));
        owner.flush_layout(&mut app);
        owner.flush_compositing_bits(&mut app);
        owner.flush_paint(&mut app);
        assert_eq!(first_paints.get(), 1);
        assert_eq!(second_paints.get(), 1);
    }

    /// `stack_test.dart`: a child that overflows the stack is clipped.
    #[test]
    fn overflowing_positioned_child_is_clipped() {
        for (clip_behavior, clips) in [(Clip::None, false), (Clip::HardEdge, true)] {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let (child, _) = sized_box(&mut app, Size::new(10.0, 10.0));
            let stack = RenderStack::new(&mut app);
            stack.set_text_direction(&mut app, Some(TextDirection::Ltr));
            stack.set_clip_behavior(&mut app, clip_behavior);
            stack.add(&mut app, child);
            child.parent_data_of_mut::<StackParentData>(&mut app).left = Some(-10.0);
            let root = RenderRepaintBoundary::new(&mut app, Some(stack.as_box()));

            let owner = first_frame(
                &mut app,
                root.as_box(),
                BoxConstraints::tight(Size::new(100.0, 100.0)),
            );

            let clipped = picture_has_clip(&app, root.as_object());
            assert_eq!(clipped, clips, "{clip_behavior:?}");
            let _ = owner;
        }
    }
}
