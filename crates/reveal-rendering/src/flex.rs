//! Flutter counterpart: `rendering/flex.dart` (`FlexFit`, `FlexParentData`, `MainAxisSize`,
//! `MainAxisAlignment`, `CrossAxisAlignment`, `RenderFlex`).
//!
//! Baseline alignment waits; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::ops::Add;

use reveal_embedder::{Clip, Offset, Size, TextBaseline, TextDirection};
use reveal_foundation::{App, Handle, PRECISION_ERROR_TOLERANCE};
use reveal_painting::{Axis, VerticalDirection};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestResult, BoxParentData, ContainerBoxParentData,
    DryLayoutFailure, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
};
use crate::layer::{ClipRectLayer, LayerHandle};
use crate::layout_helper::ChildLayoutHelper;
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// A 2D vector that uses a [`RenderFlex`]'s main axis and cross axis as its first and second
/// coordinate axes. It represents the same vector as `(main_axis_extent, cross_axis_extent)`.
#[derive(Clone, Copy)]
struct AxisSize(Size);

impl AxisSize {
    fn new(main_axis_extent: f64, cross_axis_extent: f64) -> AxisSize {
        AxisSize(Size::new(main_axis_extent, cross_axis_extent))
    }

    fn from_size(size: Size, direction: Axis) -> AxisSize {
        AxisSize(AxisSize::convert(size, direction))
    }

    fn convert(size: Size, direction: Axis) -> Size {
        match direction {
            Axis::Horizontal => size,
            Axis::Vertical => size.flipped(),
        }
    }

    fn main_axis_extent(self) -> f64 {
        self.0.width()
    }

    fn cross_axis_extent(self) -> f64 {
        self.0.height()
    }

    fn to_size(self, direction: Axis) -> Size {
        AxisSize::convert(self.0, direction)
    }

    fn apply_constraints(self, constraints: BoxConstraints, direction: Axis) -> AxisSize {
        let effective_constraints = match direction {
            Axis::Horizontal => constraints,
            Axis::Vertical => constraints.flipped(),
        };
        AxisSize(effective_constraints.constrain(self.0))
    }
}

impl Add for AxisSize {
    type Output = AxisSize;

    fn add(self, other: AxisSize) -> AxisSize {
        AxisSize(Size::new(
            self.0.width() + other.0.width(),
            self.0.height().max(other.0.height()),
        ))
    }
}

/// Flutter's `_ChildSizingFunction`: a method that finds a child's size in the sizing
/// direction, given the extent in the other direction.
type ChildSizingFunction = fn(AnyRenderBox, &mut App, f64) -> f64;

/// What [`RenderFlexMixin::compute_sizes`] worked out about a layout.
pub struct LayoutSizes {
    /// The final constrained [`AxisSize`] of the [`RenderFlex`].
    axis_size: AxisSize,

    /// The free space along the main axis. If the value is positive, the free space will be
    /// distributed according to the [`MainAxisAlignment`] specified. A negative value indicates
    /// the [`RenderFlex`] overflows along the main axis.
    main_axis_free_space: f64,
}

/// How the child is inscribed into the available space.
///
/// See also:
///
///  * [`RenderFlex`], the flex render object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexFit {
    /// The child is forced to fill the available space.
    Tight,

    /// The child can be at most as large as the available space (but is allowed to be smaller).
    Loose,
}

/// Parent data for use with [`RenderFlex`].
#[derive(Debug)]
pub struct FlexParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,

    /// The flex factor to use for this child.
    ///
    /// If `None` or zero, the child is inflexible and determines its own size. If non-zero, the
    /// amount of space the child's can occupy in the main axis is determined by dividing the
    /// free space (after placing the inflexible children) according to the flex factors of the
    /// flexible children.
    pub flex: Option<i32>,

    /// How a flexible child is inscribed into the available space.
    ///
    /// If [`flex`](Self::flex) is non-zero, the fit determines whether the child fills the space
    /// the parent makes available during layout. If the fit is [`FlexFit::Tight`], the child is
    /// required to fill the available space. If the fit is [`FlexFit::Loose`], the child can be
    /// at most as large as the available space (but is allowed to be smaller).
    pub fit: Option<FlexFit>,
}

impl FlexParentData {
    /// Creates flex parent data with no flex factor and no fit.
    pub const fn new() -> FlexParentData {
        FlexParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            flex: None,
            fit: None,
        }
    }
}

impl Default for FlexParentData {
    fn default() -> FlexParentData {
        FlexParentData::new()
    }
}

impl ParentData for FlexParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<FlexParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<FlexParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for FlexParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for FlexParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for FlexParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; flex={:?}; fit={:?}",
            self.box_parent_data, self.flex, self.fit
        )
    }
}

/// How much space should be occupied in the main axis.
///
/// During a flex layout, available space along the main axis is allocated to children. After
/// allocating space, there might be some remaining free space. This value controls whether to
/// maximize or minimize the amount of free space, subject to the incoming layout constraints.
///
/// See also:
///
///  * [`RenderFlex`], the flex render object.
///  * [`MainAxisAlignment`], which controls how the free space is distributed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAxisSize {
    /// Minimize the amount of free space along the main axis, subject to the incoming layout
    /// constraints.
    ///
    /// If the incoming layout constraints have a large enough [`BoxConstraints::min_width`] or
    /// [`BoxConstraints::min_height`], there might still be a non-zero amount of free space.
    ///
    /// If the incoming layout constraints are unbounded, and any children have a non-zero
    /// [`FlexParentData::flex`] and a [`FlexFit::Tight`] fit, the [`RenderFlex`] will assert,
    /// because there would be infinite remaining free space and boxes cannot be given infinite
    /// size.
    Min,

    /// Maximize the amount of free space along the main axis, subject to the incoming layout
    /// constraints.
    ///
    /// If the incoming layout constraints have a small enough [`BoxConstraints::max_width`] or
    /// [`BoxConstraints::max_height`], there might still be no free space.
    ///
    /// If the incoming layout constraints are unbounded, the [`RenderFlex`] will assert, because
    /// there would be infinite remaining free space and boxes cannot be given infinite size.
    Max,
}

/// How the children should be placed along the main axis in a flex layout.
///
/// See also:
///
///  * [`RenderFlex`], the flex render object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAxisAlignment {
    /// Place the children as close to the start of the main axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available to
    /// determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the start is the top or the bottom.
    Start,

    /// Place the children as close to the end of the main axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available to
    /// determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the end is the top or the bottom.
    End,

    /// Place the children as close to the middle of the main axis as possible.
    Center,

    /// Place the free space evenly between the children.
    SpaceBetween,

    /// Place the free space evenly between the children as well as half of that space before and
    /// after the first and last child.
    SpaceAround,

    /// Place the free space evenly between the children as well as before and after the first
    /// and last child.
    SpaceEvenly,
}

impl MainAxisAlignment {
    fn distribute_space(
        self,
        free_space: f64,
        item_count: usize,
        flipped: bool,
        spacing: f64,
    ) -> (f64, f64) {
        let items = item_count as f64;
        match self {
            MainAxisAlignment::Start => {
                if flipped {
                    (free_space, spacing)
                } else {
                    (0.0, spacing)
                }
            }
            MainAxisAlignment::End => {
                MainAxisAlignment::Start.distribute_space(free_space, item_count, !flipped, spacing)
            }
            MainAxisAlignment::SpaceBetween if item_count < 2 => {
                MainAxisAlignment::Start.distribute_space(free_space, item_count, flipped, spacing)
            }
            MainAxisAlignment::SpaceAround if item_count == 0 => {
                MainAxisAlignment::Start.distribute_space(free_space, item_count, flipped, spacing)
            }
            MainAxisAlignment::Center => (free_space / 2.0, spacing),
            MainAxisAlignment::SpaceBetween => (0.0, free_space / (items - 1.0) + spacing),
            MainAxisAlignment::SpaceAround => {
                (free_space / items / 2.0, free_space / items + spacing)
            }
            MainAxisAlignment::SpaceEvenly => (
                free_space / (items + 1.0),
                free_space / (items + 1.0) + spacing,
            ),
        }
    }
}

/// How the children should be placed along the cross axis in a flex layout.
///
/// See also:
///
///  * [`RenderFlex`], the flex render object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    /// Place the children with their start edge aligned with the start side of the cross axis.
    ///
    /// For example, in a column (a flex with a vertical axis) whose [`TextDirection`] is
    /// [`TextDirection::Ltr`], this aligns the left edge of the children along the left edge of
    /// the column.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available to
    /// determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the start is the top or the bottom.
    Start,

    /// Place the children as close to the end of the cross axis as possible.
    ///
    /// For example, in a column (a flex with a vertical axis) whose [`TextDirection`] is
    /// [`TextDirection::Ltr`], this aligns the right edge of the children along the right edge
    /// of the column.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available to
    /// determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the end is the top or the bottom.
    End,

    /// Place the children so that their centers align with the middle of the cross axis.
    ///
    /// This is the default cross-axis alignment.
    Center,

    /// Require the children to fill the cross axis.
    ///
    /// This causes the constraints passed to the children to be tight in the cross axis.
    Stretch,

    /// Place the children along the cross axis such that their baselines match.
    ///
    /// Consider using this value for any horizontal main axis (as with a row) where the children
    /// primarily contain text.
    ///
    /// Because baselines are always horizontal, this alignment is intended for horizontal main
    /// axes. If the main axis is vertical, then this value is treated like
    /// [`Start`](Self::Start).
    ///
    /// Children who report no baseline will be top-aligned. No [`RenderBox`] reports a baseline
    /// yet, so every child is top-aligned; see `PORTING.md`.
    Baseline,
}

impl CrossAxisAlignment {
    /// This method should not be used to position baseline-aligned children.
    fn get_child_cross_axis_offset(self, free_space: f64, flipped: bool) -> f64 {
        match self {
            CrossAxisAlignment::Stretch | CrossAxisAlignment::Baseline => 0.0,
            CrossAxisAlignment::Start => {
                if flipped {
                    free_space
                } else {
                    0.0
                }
            }
            CrossAxisAlignment::Center => free_space / 2.0,
            CrossAxisAlignment::End => {
                CrossAxisAlignment::Start.get_child_cross_axis_offset(free_space, !flipped)
            }
        }
    }
}

/// The fields of a [`RenderFlex`].
///
/// A render object that inherits the flex layout holds this bag and implements
/// [`RenderFlexMixin`].
pub struct RenderFlexData {
    direction: Axis,
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    cross_axis_alignment: CrossAxisAlignment,
    text_direction: Option<TextDirection>,
    vertical_direction: VerticalDirection,
    text_baseline: Option<TextBaseline>,
    clip_behavior: Clip,
    spacing: f64,
    /// Set during layout if overflow occurred on the main axis.
    overflow: f64,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
}

impl RenderFlexData {
    /// Creates the flex fields with their defaults.
    ///
    /// The layout is horizontal and children are aligned to the start of the main axis and the
    /// center of the cross axis.
    pub fn new() -> RenderFlexData {
        RenderFlexData {
            direction: Axis::Horizontal,
            main_axis_alignment: MainAxisAlignment::Start,
            main_axis_size: MainAxisSize::Max,
            cross_axis_alignment: CrossAxisAlignment::Center,
            text_direction: None,
            vertical_direction: VerticalDirection::Down,
            text_baseline: None,
            clip_behavior: Clip::None,
            spacing: 0.0,
            overflow: 0.0,
            clip_rect_layer: LayerHandle::new(),
        }
    }
}

impl Default for RenderFlexData {
    fn default() -> RenderFlexData {
        RenderFlexData::new()
    }
}

impl RenderFlexData {
    /// Dart `RenderFlex(direction:)`.
    pub fn direction(mut self, direction: Axis) -> RenderFlexData {
        self.direction = direction;
        self
    }

    /// Dart `RenderFlex(mainAxisAlignment:)`.
    pub fn main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> RenderFlexData {
        self.main_axis_alignment = main_axis_alignment;
        self
    }

    /// Dart `RenderFlex(mainAxisSize:)`.
    pub fn main_axis_size(mut self, main_axis_size: MainAxisSize) -> RenderFlexData {
        self.main_axis_size = main_axis_size;
        self
    }

    /// Dart `RenderFlex(crossAxisAlignment:)`.
    pub fn cross_axis_alignment(
        mut self,
        cross_axis_alignment: CrossAxisAlignment,
    ) -> RenderFlexData {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    /// Dart `RenderFlex(textDirection:)`.
    pub fn text_direction(mut self, text_direction: Option<TextDirection>) -> RenderFlexData {
        self.text_direction = text_direction;
        self
    }

    /// Dart `RenderFlex(verticalDirection:)`.
    pub fn vertical_direction(mut self, vertical_direction: VerticalDirection) -> RenderFlexData {
        self.vertical_direction = vertical_direction;
        self
    }

    /// Dart `RenderFlex(textBaseline:)`.
    pub fn text_baseline(mut self, text_baseline: Option<TextBaseline>) -> RenderFlexData {
        self.text_baseline = text_baseline;
        self
    }

    /// Dart `RenderFlex(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> RenderFlexData {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `RenderFlex(spacing:)`.
    pub fn spacing(mut self, spacing: f64) -> RenderFlexData {
        self.spacing = spacing;
        self
    }
}

/// Displays its children in a one-dimensional array.
///
/// Flutter's `RenderFlex`: the shared bodies of the flex layout. A leaf holds a
/// [`RenderFlexData`], implements this trait, and calls these where Dart would run the inherited
/// method: `RenderFlexMixin::perform_layout(self, app)`.
///
/// See [`RenderFlex`] for the layout algorithm.
pub trait RenderFlexMixin:
    RenderBox
    + ContainerRenderObjectMixin<ChildType = AnyRenderBox, ParentDataType = FlexParentData>
    + RenderBoxContainerDefaultsMixin
{
    /// Mixin field access.
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData;

    /// See [`flex_data`](Self::flex_data).
    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData;

    /// The direction to use as the main axis.
    fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        self.flex_data(app).direction
    }

    /// Sets [`direction`](Self::direction).
    fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        if self.flex_data(app).direction != value {
            self.flex_data_mut(app).direction = value;
            self.mark_needs_layout(app);
        }
    }

    /// How the children should be placed along the main axis.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], and the alignment is either
    /// [`MainAxisAlignment::Start`] or [`MainAxisAlignment::End`], then the
    /// [`text_direction`](Self::text_direction) must not be `None`.
    fn main_axis_alignment(self: RenderHandle<Self>, app: &App) -> MainAxisAlignment {
        self.flex_data(app).main_axis_alignment
    }

    /// Sets [`main_axis_alignment`](Self::main_axis_alignment).
    fn set_main_axis_alignment(self: RenderHandle<Self>, app: &mut App, value: MainAxisAlignment) {
        if self.flex_data(app).main_axis_alignment != value {
            self.flex_data_mut(app).main_axis_alignment = value;
            self.mark_needs_layout(app);
        }
    }

    /// How much space should be occupied in the main axis.
    ///
    /// After allocating space to children, there might be some remaining free space. This value
    /// controls whether to maximize or minimize the amount of free space, subject to the incoming
    /// layout constraints.
    fn main_axis_size(self: RenderHandle<Self>, app: &App) -> MainAxisSize {
        self.flex_data(app).main_axis_size
    }

    /// Sets [`main_axis_size`](Self::main_axis_size).
    fn set_main_axis_size(self: RenderHandle<Self>, app: &mut App, value: MainAxisSize) {
        if self.flex_data(app).main_axis_size != value {
            self.flex_data_mut(app).main_axis_size = value;
            self.mark_needs_layout(app);
        }
    }

    /// How the children should be placed along the cross axis.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], and the alignment is either
    /// [`CrossAxisAlignment::Start`] or [`CrossAxisAlignment::End`], then the
    /// [`text_direction`](Self::text_direction) must not be `None`.
    fn cross_axis_alignment(self: RenderHandle<Self>, app: &App) -> CrossAxisAlignment {
        self.flex_data(app).cross_axis_alignment
    }

    /// Sets [`cross_axis_alignment`](Self::cross_axis_alignment).
    fn set_cross_axis_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: CrossAxisAlignment,
    ) {
        if self.flex_data(app).cross_axis_alignment != value {
            self.flex_data_mut(app).cross_axis_alignment = value;
            self.mark_needs_layout(app);
        }
    }

    /// Determines the order to lay children out horizontally and how to interpret `start` and
    /// `end` in the horizontal direction.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], this controls the order in
    /// which children are positioned (left-to-right or right-to-left), and the meaning of the
    /// [`main_axis_alignment`](Self::main_axis_alignment) property's
    /// [`MainAxisAlignment::Start`] and [`MainAxisAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], this controls the meaning of
    /// the [`cross_axis_alignment`](Self::cross_axis_alignment) property's
    /// [`CrossAxisAlignment::Start`] and [`CrossAxisAlignment::End`] values.
    fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.flex_data(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: Option<TextDirection>) {
        if self.flex_data(app).text_direction != value {
            self.flex_data_mut(app).text_direction = value;
            self.mark_needs_layout(app);
        }
    }

    /// Determines the order to lay children out vertically and how to interpret `start` and `end`
    /// in the vertical direction.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], this controls which order
    /// children are painted in (down or up), and the meaning of the
    /// [`main_axis_alignment`](Self::main_axis_alignment) property's
    /// [`MainAxisAlignment::Start`] and [`MainAxisAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], this controls the meaning
    /// of the [`cross_axis_alignment`](Self::cross_axis_alignment) property's
    /// [`CrossAxisAlignment::Start`] and [`CrossAxisAlignment::End`] values.
    fn vertical_direction(self: RenderHandle<Self>, app: &App) -> VerticalDirection {
        self.flex_data(app).vertical_direction
    }

    /// Sets [`vertical_direction`](Self::vertical_direction).
    fn set_vertical_direction(self: RenderHandle<Self>, app: &mut App, value: VerticalDirection) {
        if self.flex_data(app).vertical_direction != value {
            self.flex_data_mut(app).vertical_direction = value;
            self.mark_needs_layout(app);
        }
    }

    /// If aligning items according to their baseline, which baseline to use.
    ///
    /// Must not be `None` if [`cross_axis_alignment`](Self::cross_axis_alignment) is
    /// [`CrossAxisAlignment::Baseline`].
    fn text_baseline(self: RenderHandle<Self>, app: &App) -> Option<TextBaseline> {
        self.flex_data(app).text_baseline
    }

    /// Sets [`text_baseline`](Self::text_baseline).
    fn set_text_baseline(self: RenderHandle<Self>, app: &mut App, value: Option<TextBaseline>) {
        debug_assert!(
            self.flex_data(app).cross_axis_alignment != CrossAxisAlignment::Baseline
                || value.is_some()
        );
        if self.flex_data(app).text_baseline != value {
            self.flex_data_mut(app).text_baseline = value;
            self.mark_needs_layout(app);
        }
    }

    /// Whether the flex overflowed along the main axis.
    ///
    /// Values below an epsilon are treated as not overflowing. Internal to the flex layout; a
    /// leaf does not republish it.
    fn has_overflow(self: RenderHandle<Self>, app: &App) -> bool {
        self.flex_data(app).overflow > PRECISION_ERROR_TOLERANCE
    }

    /// How to clip the children that overflow along the main axis.
    ///
    /// Defaults to [`Clip::None`].
    fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.flex_data(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value != self.flex_data(app).clip_behavior {
            self.flex_data_mut(app).clip_behavior = value;
            self.mark_needs_paint(app);
        }
    }

    /// How much space to place between children in the main axis.
    ///
    /// The spacing is only applied between children in the main axis: not before the first child
    /// or after the last one. When it is non-zero, the layout size will be larger than the sum of
    /// the children's layout sizes in the main axis, and the children overflow when their sizes
    /// plus the total spacing exceed the maximum main axis constraint.
    ///
    /// Defaults to 0.0.
    fn spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        self.flex_data(app).spacing
    }

    /// Sets [`spacing`](Self::spacing).
    fn set_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!(value >= 0.0);
        if self.flex_data(app).spacing == value {
            return;
        }
        self.flex_data_mut(app).spacing = value;
        self.mark_needs_layout(app);
    }

    /// Flutter's `_debugHasNecessaryDirections`: asserts the directions the current alignment
    /// needs are available. Always returns true. Internal to the flex layout.
    fn debug_has_necessary_directions(self: RenderHandle<Self>, app: &App) -> bool {
        let direction = self.direction(app);
        if self.first_child(app).is_some() && self.last_child(app) != self.first_child(app) {
            // i.e. there's more than one child
            if direction == Axis::Horizontal {
                debug_assert!(
                    self.text_direction(app).is_some(),
                    "Horizontal RenderFlex with multiple children has a null textDirection, so \
                     the layout order is undefined."
                );
            }
        }
        let main_axis_alignment = self.main_axis_alignment(app);
        if (main_axis_alignment == MainAxisAlignment::Start
            || main_axis_alignment == MainAxisAlignment::End)
            && direction == Axis::Horizontal
        {
            debug_assert!(
                self.text_direction(app).is_some(),
                "Horizontal RenderFlex with {main_axis_alignment:?} has a null textDirection, so \
                 the alignment cannot be resolved."
            );
        }
        let cross_axis_alignment = self.cross_axis_alignment(app);
        if (cross_axis_alignment == CrossAxisAlignment::Start
            || cross_axis_alignment == CrossAxisAlignment::End)
            && direction == Axis::Vertical
        {
            debug_assert!(
                self.text_direction(app).is_some(),
                "Vertical RenderFlex with {cross_axis_alignment:?} has a null textDirection, so \
                 the alignment cannot be resolved."
            );
        }
        true
    }

    /// The extent of `size` along the cross axis. Internal to the flex layout.
    fn get_cross_size(self: RenderHandle<Self>, app: &App, size: Size) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => size.height(),
            Axis::Vertical => size.width(),
        }
    }

    /// The extent of `size` along the main axis. Internal to the flex layout.
    fn get_main_size(self: RenderHandle<Self>, app: &App, size: Size) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => size.width(),
            Axis::Vertical => size.height(),
        }
    }

    /// Whether to lay out left-to-right / top-to-bottom (`false`), or right-to-left /
    /// bottom-to-top (`true`). `false` when the layout direction does not matter (for instance,
    /// there is no child). Internal to the flex layout.
    fn flip_main_axis(self: RenderHandle<Self>, app: &App) -> bool {
        self.first_child(app).is_some()
            && match self.direction(app) {
                Axis::Horizontal => self.text_direction(app) == Some(TextDirection::Rtl),
                Axis::Vertical => self.vertical_direction(app) == VerticalDirection::Up,
            }
    }

    /// The cross axis counterpart of [`flip_main_axis`](Self::flip_main_axis). Internal to the
    /// flex layout.
    fn flip_cross_axis(self: RenderHandle<Self>, app: &App) -> bool {
        self.first_child(app).is_some()
            && match self.direction(app) {
                Axis::Vertical => self.text_direction(app) == Some(TextDirection::Rtl),
                Axis::Horizontal => self.vertical_direction(app) == VerticalDirection::Up,
            }
    }

    /// The constraints an inflexible child is laid out with. Internal to the flex layout.
    fn constraints_for_non_flex_child(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let fill_cross_axis = self.cross_axis_alignment(app) == CrossAxisAlignment::Stretch;
        match self.direction(app) {
            Axis::Horizontal => {
                if fill_cross_axis {
                    BoxConstraints::tight_for(None, Some(constraints.max_height))
                } else {
                    BoxConstraints::new().max_height(constraints.max_height)
                }
            }
            Axis::Vertical => {
                if fill_cross_axis {
                    BoxConstraints::tight_for(Some(constraints.max_width), None)
                } else {
                    BoxConstraints::new().max_width(constraints.max_width)
                }
            }
        }
    }

    /// The constraints a flexible child is laid out with, given the main axis extent it was
    /// allocated. Internal to the flex layout.
    fn constraints_for_flex_child(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
        max_child_extent: f64,
    ) -> BoxConstraints {
        debug_assert!(get_flex(app, child) > 0);
        debug_assert!(max_child_extent >= 0.0);
        let min_child_extent = match get_fit(app, child) {
            FlexFit::Tight => max_child_extent,
            FlexFit::Loose => 0.0,
        };
        let fill_cross_axis = self.cross_axis_alignment(app) == CrossAxisAlignment::Stretch;
        match self.direction(app) {
            Axis::Horizontal => BoxConstraints {
                min_width: min_child_extent,
                max_width: max_child_extent,
                min_height: if fill_cross_axis {
                    constraints.max_height
                } else {
                    0.0
                },
                max_height: constraints.max_height,
            },
            Axis::Vertical => BoxConstraints {
                min_width: if fill_cross_axis {
                    constraints.max_width
                } else {
                    0.0
                },
                max_width: constraints.max_width,
                min_height: min_child_extent,
                max_height: max_child_extent,
            },
        }
    }

    /// Flutter's `_getIntrinsicSize`: the intrinsic extent along `sizing_direction`, given the
    /// `extent` in the other direction and a `child_size` that measures one child. Internal to
    /// the flex layout.
    fn get_intrinsic_size(
        self: RenderHandle<Self>,
        app: &mut App,
        sizing_direction: Axis,
        extent: f64,
        child_size: ChildSizingFunction,
    ) -> f64 {
        let direction = self.direction(app);
        if direction == sizing_direction {
            // INTRINSIC MAIN SIZE
            // Intrinsic main size is the smallest size the flex container can take while
            // maintaining the min/max-content contributions of its flex items.
            let mut total_flex = 0.0;
            let mut inflexible_space = self.spacing(app) * (self.child_count(app) as f64 - 1.0);
            let mut max_flex_fraction_so_far: f64 = 0.0;
            let mut child = self.first_child(app);
            while let Some(current) = child {
                let flex = get_flex(app, current);
                total_flex += f64::from(flex);
                if flex > 0 {
                    let flex_fraction = child_size(current, app, extent) / f64::from(flex);
                    max_flex_fraction_so_far = max_flex_fraction_so_far.max(flex_fraction);
                } else {
                    inflexible_space += child_size(current, app, extent);
                }
                child = self.child_after(app, current);
            }
            return max_flex_fraction_so_far * total_flex + inflexible_space;
        }

        // INTRINSIC CROSS SIZE
        // Intrinsic cross size is the max of the intrinsic cross sizes of the children, after
        // the flexible children are fit into the available space, with the children sized using
        // their max intrinsic dimensions.
        let is_horizontal = direction == Axis::Horizontal;
        let layout_child = |app: &mut App, child: AnyRenderBox, constraints: BoxConstraints| {
            let main_axis_size_from_constraints = if is_horizontal {
                constraints.max_width
            } else {
                constraints.max_height
            };
            // An infinite main_axis_size_from_constraints means this child is flexible (or
            // extent is infinite).
            debug_assert_eq!(
                get_flex(app, child) != 0 && extent.is_finite(),
                main_axis_size_from_constraints.is_finite()
            );
            let max_main_axis_size = if main_axis_size_from_constraints.is_finite() {
                main_axis_size_from_constraints
            } else if is_horizontal {
                child.get_max_intrinsic_width(app, f64::INFINITY)
            } else {
                child.get_max_intrinsic_height(app, f64::INFINITY)
            };
            let cross_size = child_size(child, app, max_main_axis_size);
            if is_horizontal {
                Size::new(max_main_axis_size, cross_size)
            } else {
                Size::new(cross_size, max_main_axis_size)
            }
        };
        let constraints = if is_horizontal {
            BoxConstraints::new().max_width(extent)
        } else {
            BoxConstraints::new().max_height(extent)
        };
        self.compute_sizes(app, constraints, layout_child)
            .axis_size
            .cross_axis_extent()
    }

    /// Flutter's `_debugCheckConstraints`: the error to report when a child has a non-zero flex
    /// factor while the incoming main axis constraint is unbounded and that child would have to
    /// fill it, or `None` when the constraints are usable. Internal to the flex layout.
    fn debug_check_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> Option<String> {
        let direction = self.direction(app);
        let max_main_size = match direction {
            Axis::Horizontal => constraints.max_width,
            Axis::Vertical => constraints.max_height,
        };
        let can_flex = max_main_size < f64::INFINITY;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            if get_flex(app, current) > 0
                && !can_flex
                && (self.main_axis_size(app) == MainAxisSize::Max
                    || get_fit(app, current) == FlexFit::Tight)
            {
                let (identity, axis, dimension) = match direction {
                    Axis::Horizontal => ("row", "horizontal", "width"),
                    Axis::Vertical => ("column", "vertical", "height"),
                };
                return Some(format!(
                    "RenderFlex children have non-zero flex but incoming {dimension} constraints \
                     are unbounded.\n\
                     When a {identity} is in a parent that does not provide a finite {dimension} \
                     constraint, for example if it is in a {axis} scrollable, it will try to \
                     shrink-wrap its children along the {axis} axis. Setting a flex on a child \
                     (e.g. using Expanded) indicates that the child is to expand to fill the \
                     remaining space in the {axis} direction.\n\
                     These two directives are mutually exclusive. If a parent is to shrink-wrap \
                     its child, the child cannot simultaneously expand to fit its parent.\n\
                     Consider setting mainAxisSize to MainAxisSize.min and using FlexFit.loose \
                     fits for the flexible children (using Flexible rather than Expanded). This \
                     will allow the flexible children to size themselves to less than the \
                     infinite remaining space they would otherwise be forced to take, and then \
                     will cause the RenderFlex to shrink-wrap the children rather than expanding \
                     to fit the maximum constraints provided by the parent."
                ));
            }
            child = self.child_after(app, current);
        }
        None
    }

    /// Flutter's `_computeSizes`: measures the children with `layout_child` and works out this
    /// flex's own size. Internal to the flex layout.
    fn compute_sizes(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        mut layout_child: impl FnMut(&mut App, AnyRenderBox, BoxConstraints) -> Size,
    ) -> LayoutSizes {
        debug_assert!(self.debug_has_necessary_directions(app));

        // Determine used flex factor, size inflexible items, calculate free space.
        let direction = self.direction(app);
        let max_main_size = self.get_main_size(app, constraints.biggest());
        let can_flex = max_main_size.is_finite();
        let non_flex_child_constraints = self.constraints_for_non_flex_child(app, constraints);

        // The first pass lays out non-flex children and computes total flex.
        let mut total_flex = 0;
        let mut first_flex_child: Option<AnyRenderBox> = None;
        // Initially, accumulated_size is the sum of the spaces between children in the main axis.
        let mut accumulated_size = AxisSize::new(
            self.spacing(app) * (self.child_count(app) as f64 - 1.0),
            0.0,
        );
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let flex = get_flex(app, current);
            if can_flex && flex > 0 {
                total_flex += flex;
                first_flex_child.get_or_insert(current);
            } else {
                let child_size = layout_child(app, current, non_flex_child_constraints);
                accumulated_size = accumulated_size + AxisSize::from_size(child_size, direction);
            }
            child = self.child_after(app, current);
        }

        debug_assert_eq!(total_flex == 0, first_flex_child.is_none());
        // If we are given infinite space there's no need for this extra step.
        debug_assert!(first_flex_child.is_none() || can_flex);

        // The second pass distributes free space to flexible children.
        let flex_space = (max_main_size - accumulated_size.main_axis_extent()).max(0.0);
        let space_per_flex = flex_space / total_flex as f64;
        let mut child = first_flex_child;
        while let Some(current) = child {
            if total_flex == 0 {
                break;
            }
            let flex = get_flex(app, current);
            if flex != 0 {
                total_flex -= flex;
                debug_assert!(space_per_flex.is_finite());
                let max_child_extent = space_per_flex * f64::from(flex);
                debug_assert!(
                    get_fit(app, current) == FlexFit::Loose || max_child_extent < f64::INFINITY
                );
                let child_constraints =
                    self.constraints_for_flex_child(app, current, constraints, max_child_extent);
                let child_size = layout_child(app, current, child_constraints);
                accumulated_size = accumulated_size + AxisSize::from_size(child_size, direction);
            }
            child = self.child_after(app, current);
        }
        debug_assert_eq!(total_flex, 0);

        let ideal_main_size = match self.main_axis_size(app) {
            MainAxisSize::Max if max_main_size.is_finite() => max_main_size,
            MainAxisSize::Max | MainAxisSize::Min => accumulated_size.main_axis_extent(),
        };

        let constrained_size = AxisSize::new(ideal_main_size, accumulated_size.cross_axis_extent())
            .apply_constraints(constraints, direction);
        LayoutSizes {
            axis_size: constrained_size,
            main_axis_free_space: constrained_size.main_axis_extent()
                - accumulated_size.main_axis_extent(),
        }
    }

    /// Sizes this flex and positions its children; see [`RenderFlex`] for the algorithm.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        if cfg!(debug_assertions)
            && let Some(error) = self.debug_check_constraints(app, constraints)
        {
            panic!("{error}");
        }

        let sizes = self.compute_sizes(app, constraints, ChildLayoutHelper::layout_child);

        let direction = self.direction(app);
        let cross_axis_extent = sizes.axis_size.cross_axis_extent();
        self.set_size(app, sizes.axis_size.to_size(direction));
        self.flex_data_mut(app).overflow = (-sizes.main_axis_free_space).max(0.0);

        let remaining_space = sizes.main_axis_free_space.max(0.0);
        let flip_main_axis = self.flip_main_axis(app);
        let flip_cross_axis = self.flip_cross_axis(app);
        let cross_axis_alignment = self.cross_axis_alignment(app);
        let (leading_space, between_space) = self.main_axis_alignment(app).distribute_space(
            remaining_space,
            self.child_count(app),
            flip_main_axis,
            self.spacing(app),
        );
        let top_left_child = if flip_main_axis {
            self.last_child(app)
        } else {
            self.first_child(app)
        };

        // Position all children in visual order: starting from the top-left child and work
        // towards the child that's farthest away from the origin.
        let mut child_main_position = leading_space;
        let mut child = top_left_child;
        while let Some(current) = child {
            let child_size = current.size(app);
            let free_cross_space = cross_axis_extent - self.get_cross_size(app, child_size);
            let child_cross_position = if cross_axis_alignment == CrossAxisAlignment::Baseline
                && direction == Axis::Horizontal
            {
                // In a baseline-aligned horizontal flex, a child without a baseline is aligned to
                // the top of the cross axis (dy = 0), regardless of verticalDirection or
                // crossAxisAlignment. That is, we intentionally ignore flip_cross_axis here.
                CrossAxisAlignment::Start.get_child_cross_axis_offset(free_cross_space, false)
            } else {
                // Non-baseline alignment respects the configured cross_axis_alignment.
                cross_axis_alignment.get_child_cross_axis_offset(free_cross_space, flip_cross_axis)
            };
            let offset = match direction {
                Axis::Horizontal => Offset::new(child_main_position, child_cross_position),
                Axis::Vertical => Offset::new(child_cross_position, child_main_position),
            };
            current
                .parent_data_of_mut::<FlexParentData>(app)
                .set_offset(offset);
            child_main_position += self.get_main_size(app, child_size) + between_space;
            child = if flip_main_axis {
                self.child_before(app, current)
            } else {
                self.child_after(app, current)
            };
        }
    }

    /// Paints the children, clipping them to this flex when they overflow the main axis.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if !self.has_overflow(app) {
            self.default_paint(app, context, offset);
            return;
        }

        // There's no point in drawing the children if we're empty.
        let size = self.size(app);
        if size.is_empty() {
            return;
        }

        let clip_behavior = self.clip_behavior(app);
        let old = self.flex_data(app).clip_rect_layer.layer();
        let layer = context.push_clip_rect(
            app,
            self.as_object().needs_compositing(app),
            offset,
            Offset::ZERO & size,
            |app, context, offset| self.default_paint(app, context, offset),
            clip_behavior,
            old,
        );
        LayerHandle::set_layer(
            app,
            |app| &mut self.flex_data_mut(app).clip_rect_layer,
            layer,
        );
    }

    /// Flutter's `dispose`: drop the clip layer handle, then `super.dispose()`.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(
            app,
            |app| &mut self.flex_data_mut(app).clip_rect_layer,
            None,
        );
        crate::object::RenderObjectBase::dispose(self, app);
    }

    /// A flex child gets a [`FlexParentData`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<FlexParentData>(app) {
            child.set_parent_data(app, FlexParentData::new());
        }
    }

    /// The minimum intrinsic width of the children laid out in a flex.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.get_intrinsic_size(
            app,
            Axis::Horizontal,
            height,
            AnyRenderBox::get_min_intrinsic_width,
        )
    }

    /// The maximum intrinsic width of the children laid out in a flex.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.get_intrinsic_size(
            app,
            Axis::Horizontal,
            height,
            AnyRenderBox::get_max_intrinsic_width,
        )
    }

    /// The minimum intrinsic height of the children laid out in a flex.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.get_intrinsic_size(
            app,
            Axis::Vertical,
            width,
            AnyRenderBox::get_min_intrinsic_height,
        )
    }

    /// The maximum intrinsic height of the children laid out in a flex.
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.get_intrinsic_size(
            app,
            Axis::Vertical,
            width,
            AnyRenderBox::get_max_intrinsic_height,
        )
    }

    /// The size this flex would take, measuring the children dry.
    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if cfg!(debug_assertions)
            && let Some(error) = self.debug_check_constraints(app, constraints)
        {
            self.debug_cannot_compute_dry_layout(DryLayoutFailure::Error(&error));
            return Size::ZERO;
        }
        let direction = self.direction(app);
        self.compute_sizes(app, constraints, ChildLayoutHelper::dry_layout_child)
            .axis_size
            .to_size(direction)
    }

    /// Hit tests the children in paint order.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.default_hit_test_children(app, result, position)
    }
}

/// Flutter's `RenderFlex._getFlex`.
fn get_flex(app: &App, child: AnyRenderBox) -> i32 {
    child
        .as_object()
        .parent_data_of::<FlexParentData>(app)
        .flex
        .unwrap_or(0)
}

/// Flutter's `RenderFlex._getFit`.
fn get_fit(app: &App, child: AnyRenderBox) -> FlexFit {
    child
        .as_object()
        .parent_data_of::<FlexParentData>(app)
        .fit
        .unwrap_or(FlexFit::Tight)
}

/// Displays its children in a one-dimensional array.
///
/// ## Layout algorithm
///
/// _This section describes how the framework causes [`RenderFlex`] to position its children._
/// _See [`BoxConstraints`] for an introduction to box layout models._
///
/// Layout for a [`RenderFlex`] proceeds in six steps:
///
/// 1. Layout each child with a null or zero flex factor with unbounded main axis constraints and
///    the incoming cross axis constraints. If the [`cross_axis_alignment`](Self::cross_axis_alignment)
///    is [`CrossAxisAlignment::Stretch`], instead use tight cross axis constraints that match the
///    incoming max extent in the cross axis.
/// 2. Divide the remaining main axis space among the children with non-zero flex factors
///    according to their flex factor. For example, a child with a flex factor of 2.0 will receive
///    twice the amount of main axis space as a child with a flex factor of 1.0.
/// 3. Layout each of the remaining children with the same cross axis constraints as in step 1,
///    but instead of using unbounded main axis constraints, use max axis constraints based on the
///    amount of space allocated in step 2. Children with [`FlexFit::Tight`] are given tight
///    constraints (i.e., forced to fill the allocated space), and children with [`FlexFit::Loose`]
///    are given loose constraints (i.e., not forced to fill the allocated space).
/// 4. The cross axis extent of the [`RenderFlex`] is the maximum cross axis extent of the
///    children (which will always satisfy the incoming constraints).
/// 5. The main axis extent of the [`RenderFlex`] is determined by the
///    [`main_axis_size`](Self::main_axis_size) property. If it is [`MainAxisSize::Max`], then the
///    main axis extent of the [`RenderFlex`] is the max extent of the incoming main axis
///    constraints. If it is [`MainAxisSize::Min`], then the main axis extent is the sum of the
///    main axis extents of the children (subject to the incoming constraints).
/// 6. Determine the position for each child according to the
///    [`main_axis_alignment`](Self::main_axis_alignment) and the
///    [`cross_axis_alignment`](Self::cross_axis_alignment). For example, if the
///    [`main_axis_alignment`](Self::main_axis_alignment) is [`MainAxisAlignment::SpaceBetween`],
///    any main axis space that has not been allocated to children is divided evenly and placed
///    between the children.
pub struct RenderFlex {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    flex: RenderFlexData,
}

impl RenderFlex {
    /// Creates a flex render object.
    ///
    /// By default, the flex layout is horizontal and children are aligned to the start of the
    /// main axis and the center of the cross axis. Add children with
    /// [`add`](ContainerRenderObjectMixin::add) or
    /// [`add_all`](ContainerRenderObjectMixin::add_all); Dart's constructor takes them.
    pub fn new(app: &mut App) -> RenderHandle<RenderFlex> {
        RenderHandle::new_box(
            app,
            RenderFlex {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                flex: RenderFlexData::new(),
            },
        )
    }

    /// See [`RenderFlexMixin::direction`].
    pub fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        RenderFlexMixin::direction(self, app)
    }

    /// See [`RenderFlexMixin::set_direction`].
    pub fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        RenderFlexMixin::set_direction(self, app, value)
    }

    /// See [`RenderFlexMixin::main_axis_alignment`].
    pub fn main_axis_alignment(self: RenderHandle<Self>, app: &App) -> MainAxisAlignment {
        RenderFlexMixin::main_axis_alignment(self, app)
    }

    /// See [`RenderFlexMixin::set_main_axis_alignment`].
    pub fn set_main_axis_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: MainAxisAlignment,
    ) {
        RenderFlexMixin::set_main_axis_alignment(self, app, value)
    }

    /// See [`RenderFlexMixin::main_axis_size`].
    pub fn main_axis_size(self: RenderHandle<Self>, app: &App) -> MainAxisSize {
        RenderFlexMixin::main_axis_size(self, app)
    }

    /// See [`RenderFlexMixin::set_main_axis_size`].
    pub fn set_main_axis_size(self: RenderHandle<Self>, app: &mut App, value: MainAxisSize) {
        RenderFlexMixin::set_main_axis_size(self, app, value)
    }

    /// See [`RenderFlexMixin::cross_axis_alignment`].
    pub fn cross_axis_alignment(self: RenderHandle<Self>, app: &App) -> CrossAxisAlignment {
        RenderFlexMixin::cross_axis_alignment(self, app)
    }

    /// See [`RenderFlexMixin::set_cross_axis_alignment`].
    pub fn set_cross_axis_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: CrossAxisAlignment,
    ) {
        RenderFlexMixin::set_cross_axis_alignment(self, app, value)
    }

    /// See [`RenderFlexMixin::text_direction`].
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        RenderFlexMixin::text_direction(self, app)
    }

    /// See [`RenderFlexMixin::set_text_direction`].
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        RenderFlexMixin::set_text_direction(self, app, value)
    }

    /// See [`RenderFlexMixin::vertical_direction`].
    pub fn vertical_direction(self: RenderHandle<Self>, app: &App) -> VerticalDirection {
        RenderFlexMixin::vertical_direction(self, app)
    }

    /// See [`RenderFlexMixin::set_vertical_direction`].
    pub fn set_vertical_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: VerticalDirection,
    ) {
        RenderFlexMixin::set_vertical_direction(self, app, value)
    }

    /// See [`RenderFlexMixin::text_baseline`].
    pub fn text_baseline(self: RenderHandle<Self>, app: &App) -> Option<TextBaseline> {
        RenderFlexMixin::text_baseline(self, app)
    }

    /// See [`RenderFlexMixin::set_text_baseline`].
    pub fn set_text_baseline(self: RenderHandle<Self>, app: &mut App, value: Option<TextBaseline>) {
        RenderFlexMixin::set_text_baseline(self, app, value)
    }

    /// See [`RenderFlexMixin::clip_behavior`].
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        RenderFlexMixin::clip_behavior(self, app)
    }

    /// See [`RenderFlexMixin::set_clip_behavior`].
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        RenderFlexMixin::set_clip_behavior(self, app, value)
    }

    /// See [`RenderFlexMixin::spacing`].
    pub fn spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        RenderFlexMixin::spacing(self, app)
    }

    /// See [`RenderFlexMixin::set_spacing`].
    pub fn set_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        RenderFlexMixin::set_spacing(self, app, value)
    }
}

impl ContainerRenderObjectMixin for RenderFlex {
    type ChildType = AnyRenderBox;
    type ParentDataType = FlexParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderFlex {}

impl RenderFlexMixin for RenderFlex {
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData {
        &self.get(app).flex
    }

    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData {
        &mut self.get_mut(app).flex
    }
}

impl RenderObject for RenderFlex {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderFlexMixin::perform_layout(self, app)
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
        RenderFlexMixin::paint(self, app, context, offset)
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderFlexMixin::dispose(self, app)
    }
}

impl RenderBox for RenderFlex {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderFlexMixin::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderFlexMixin::compute_dry_layout(self, app, constraints)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderFlexMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::any::Any;

    use reveal_foundation::Handle;
    use reveal_gestures::{HitTestEntry, HitTestResult};

    use super::*;
    use crate::box_::BoxHitTestEntry;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::RenderRepaintBoundary;
    use reveal_embedder::valo::Op;

    /// A leaf that takes its preferred size where the constraints allow, and answers hit tests
    /// within its bounds.
    struct SizedTestBox {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        preferred_size: Size,
    }

    impl RenderObject for SizedTestBox {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let preferred_size = self.get(app).preferred_size;
            let size = self.constraints(app).constrain(preferred_size);
            self.set_size(app, size);
        }
    }

    impl RenderBox for SizedTestBox {
        crate::render_box_accessors!();

        fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
            true
        }
    }

    fn sized_box(app: &mut App, size: Size) -> AnyRenderBox {
        RenderHandle::new_box(
            app,
            SizedTestBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                preferred_size: size,
            },
        )
        .as_box()
    }

    /// A left-to-right [`RenderFlex`] holding `children`.
    fn row(app: &mut App, children: Vec<AnyRenderBox>) -> RenderHandle<RenderFlex> {
        let flex = RenderFlex::new(app);
        flex.set_text_direction(app, Some(TextDirection::Ltr));
        flex.add_all(app, Some(children));
        flex
    }

    fn offset_of(app: &App, child: AnyRenderBox) -> Offset {
        child.box_parent_data(app).offset
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

    fn hit_boxes(result: &HitTestResult) -> Vec<AnyRenderBox> {
        result
            .path()
            .iter()
            .filter_map(|entry: &HitTestEntry| {
                let target: &dyn Any = entry.target();
                target
                    .downcast_ref::<BoxHitTestEntry>()
                    .map(BoxHitTestEntry::target)
            })
            .collect()
    }

    /// `object.dart`'s `ContainerRenderObjectMixin`: the child list is a doubly-linked list that
    /// `insert` / `move` / `remove` keep consistent from both ends.
    #[test]
    fn child_list_insert_move_and_remove() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = sized_box(&mut app, Size::new(10.0, 10.0));
        let middle = sized_box(&mut app, Size::new(10.0, 10.0));
        let last = sized_box(&mut app, Size::new(10.0, 10.0));
        let flex = RenderFlex::new(&mut app);

        flex.add(&mut app, first);
        flex.insert(&mut app, last, Some(first));
        flex.insert(&mut app, middle, Some(first));
        assert_eq!(flex.child_count(&app), 3);
        assert_eq!(flex.children_as_list(&app), vec![first, middle, last]);
        assert_eq!(flex.first_child(&app), Some(first));
        assert_eq!(flex.last_child(&app), Some(last));
        assert_eq!(flex.child_after(&app, first), Some(middle));
        assert_eq!(flex.child_before(&app, last), Some(middle));

        let mut visited = Vec::new();
        flex.as_object()
            .visit_children(&app, &mut |child| visited.push(child));
        assert_eq!(
            visited,
            vec![first.as_object(), middle.as_object(), last.as_object()]
        );

        flex.move_child(&mut app, first, Some(last));
        assert_eq!(flex.children_as_list(&app), vec![middle, last, first]);
        flex.move_child(&mut app, first, None);
        assert_eq!(flex.children_as_list(&app), vec![first, middle, last]);

        flex.remove(&mut app, middle);
        assert_eq!(flex.children_as_list(&app), vec![first, last]);
        assert_eq!(flex.child_count(&app), 2);
        assert!(middle.as_object().parent(&app).is_none());

        flex.remove_all(&mut app);
        assert_eq!(flex.child_count(&app), 0);
        assert!(flex.first_child(&app).is_none());
        assert!(flex.last_child(&app).is_none());
        assert!(first.as_object().parent(&app).is_none());
        assert!(last.as_object().parent(&app).is_none());
    }

    /// `flex_test.dart`: a row gives the flexible child the space the fixed one leaves, and the
    /// spacing goes between them.
    /// A box with fixed intrinsic dimensions that takes the size its constraints allow around
    /// them.
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

    fn intrinsic_box(app: &mut App, intrinsic: Size) -> AnyRenderBox {
        RenderHandle::new_box(
            app,
            IntrinsicTestBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                intrinsic,
            },
        )
        .as_box()
    }

    /// A row of a fixed child and a flexible one: `flex_test.dart`'s intrinsics, where the main
    /// axis intrinsic is the inflexible space plus the largest flex fraction, and the cross axis
    /// intrinsic is the tallest child.
    #[test]
    fn row_intrinsics_add_the_inflexible_space_to_the_largest_flex_fraction() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let fixed = intrinsic_box(&mut app, Size::new(50.0, 20.0));
        let flexible = intrinsic_box(&mut app, Size::new(30.0, 10.0));
        let flex = row(&mut app, vec![fixed, flexible]);
        flexible.parent_data_of_mut::<FlexParentData>(&mut app).flex = Some(1);

        let row = flex.as_box();
        assert_eq!(row.get_max_intrinsic_width(&mut app, f64::INFINITY), 80.0);
        assert_eq!(row.get_min_intrinsic_width(&mut app, f64::INFINITY), 80.0);
        assert_eq!(row.get_max_intrinsic_height(&mut app, 100.0), 20.0);
    }

    /// The spacing between the children counts towards the main axis intrinsic.
    #[test]
    fn row_intrinsics_include_the_spacing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = intrinsic_box(&mut app, Size::new(50.0, 20.0));
        let second = intrinsic_box(&mut app, Size::new(30.0, 10.0));
        let flex = row(&mut app, vec![first, second]);
        flex.set_spacing(&mut app, 10.0);
        assert_eq!(
            flex.as_box()
                .get_max_intrinsic_width(&mut app, f64::INFINITY),
            90.0
        );
    }

    /// The dry layout takes the main axis size into account: a `Max` row fills the constraints,
    /// a `Min` row shrink-wraps its children, and both match what layout produces.
    #[test]
    fn row_dry_layout_follows_the_main_axis_size() {
        for (main_axis_size, expected) in [
            (MainAxisSize::Max, Size::new(200.0, 20.0)),
            (MainAxisSize::Min, Size::new(80.0, 20.0)),
        ] {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let fixed = intrinsic_box(&mut app, Size::new(50.0, 20.0));
            let flexible = intrinsic_box(&mut app, Size::new(30.0, 10.0));
            let flex = row(&mut app, vec![fixed, flexible]);
            flex.set_main_axis_size(&mut app, main_axis_size);
            {
                let parent_data = flexible.parent_data_of_mut::<FlexParentData>(&mut app);
                parent_data.flex = Some(1);
                parent_data.fit = Some(FlexFit::Loose);
            }

            let constraints = BoxConstraints::new().max_width(200.0).max_height(100.0);
            let dry = flex.as_box().get_dry_layout(&mut app, constraints);
            flex.layout(&mut app, constraints, false);
            assert_eq!(dry, expected);
            assert_eq!(dry, flex.size(&app), "the dry layout is the real layout");
        }
    }

    #[test]
    fn row_lays_out_fixed_and_flexible_children_with_spacing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let fixed = sized_box(&mut app, Size::new(50.0, 30.0));
        let flexible = sized_box(&mut app, Size::new(20.0, 10.0));
        let flex = row(&mut app, vec![fixed, flexible]);
        flexible.parent_data_of_mut::<FlexParentData>(&mut app).flex = Some(1);
        flex.set_spacing(&mut app, 10.0);

        flex.layout(
            &mut app,
            BoxConstraints::tight(Size::new(200.0, 100.0)),
            false,
        );

        assert_eq!(flex.size(&app), Size::new(200.0, 100.0));
        assert_eq!(fixed.size(&app), Size::new(50.0, 30.0));
        // 200 - 50 fixed - 10 spacing goes to the one flexible child, tightly.
        assert_eq!(flexible.size(&app), Size::new(140.0, 10.0));
        assert_eq!(offset_of(&app, fixed), Offset::new(0.0, 35.0));
        assert_eq!(offset_of(&app, flexible), Offset::new(60.0, 45.0));
    }

    /// `flex_test.dart`: `Test Flex Widget MainAxisAlignment`.
    #[test]
    fn main_axis_alignment_positions_children() {
        let cases = [
            (MainAxisAlignment::Start, 0.0, 20.0),
            (MainAxisAlignment::End, 60.0, 80.0),
            (MainAxisAlignment::Center, 30.0, 50.0),
            (MainAxisAlignment::SpaceBetween, 0.0, 80.0),
            (MainAxisAlignment::SpaceAround, 15.0, 65.0),
            (MainAxisAlignment::SpaceEvenly, 20.0, 60.0),
        ];
        for (alignment, first_x, second_x) in cases {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let first = sized_box(&mut app, Size::new(20.0, 20.0));
            let second = sized_box(&mut app, Size::new(20.0, 20.0));
            let flex = row(&mut app, vec![first, second]);
            flex.set_main_axis_alignment(&mut app, alignment);

            flex.layout(
                &mut app,
                BoxConstraints::tight(Size::new(100.0, 20.0)),
                false,
            );

            assert_eq!(
                (offset_of(&app, first).dx(), offset_of(&app, second).dx()),
                (first_x, second_x),
                "{alignment:?}"
            );
        }
    }

    /// `flex_test.dart`: `Test Flex Widget CrossAxisAlignment`. Baseline alignment top-aligns
    /// because no `RenderBox` reports a baseline yet; see `PORTING.md`.
    #[test]
    fn cross_axis_alignment_positions_children() {
        let cases = [
            (CrossAxisAlignment::Start, 0.0, 20.0),
            (CrossAxisAlignment::Center, 40.0, 20.0),
            (CrossAxisAlignment::End, 80.0, 20.0),
            (CrossAxisAlignment::Stretch, 0.0, 100.0),
            (CrossAxisAlignment::Baseline, 0.0, 20.0),
        ];
        for (alignment, y, height) in cases {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let child = sized_box(&mut app, Size::new(20.0, 20.0));
            let flex = row(&mut app, vec![child]);
            flex.set_cross_axis_alignment(&mut app, alignment);
            if alignment == CrossAxisAlignment::Baseline {
                flex.set_text_baseline(&mut app, Some(TextBaseline::Alphabetic));
            }

            flex.layout(
                &mut app,
                BoxConstraints::tight(Size::new(100.0, 100.0)),
                false,
            );

            assert_eq!(offset_of(&app, child).dy(), y, "{alignment:?}");
            assert_eq!(child.size(&app).height(), height, "{alignment:?}");
        }
    }

    /// `flex_test.dart`: `Clip.none` versus a clip when the children overflow the main axis.
    #[test]
    fn overflow_is_clipped() {
        for (clip_behavior, clips) in [(Clip::None, false), (Clip::HardEdge, true)] {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let first = sized_box(&mut app, Size::new(80.0, 20.0));
            let second = sized_box(&mut app, Size::new(80.0, 20.0));
            let flex = row(&mut app, vec![first, second]);
            flex.set_clip_behavior(&mut app, clip_behavior);
            let root = RenderRepaintBoundary::new(&mut app, Some(flex.as_box()));

            let owner = first_frame(
                &mut app,
                root.as_box(),
                BoxConstraints::tight(Size::new(100.0, 20.0)),
            );

            assert_eq!(flex.size(&app), Size::new(100.0, 20.0));
            let clipped = picture_has_clip(&app, root.as_object());
            assert_eq!(clipped, clips, "{clip_behavior:?}");
            let _ = owner;
        }
    }

    /// `flex_test.dart`: a hit lands on the child that covers the position.
    #[test]
    fn hit_test_reaches_the_child_under_the_position() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = sized_box(&mut app, Size::new(50.0, 50.0));
        let second = sized_box(&mut app, Size::new(50.0, 50.0));
        let flex = row(&mut app, vec![first, second]);

        flex.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 50.0)),
            false,
        );

        let mut result = HitTestResult::new();
        let hit = flex.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(75.0, 25.0),
        );
        assert!(hit);
        assert_eq!(hit_boxes(&result), vec![second, flex.as_box()]);
    }
}
