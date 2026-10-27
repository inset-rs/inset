//! Flutter counterpart: `rendering/wrap.dart`.

use std::any::{Any, TypeId};
use std::fmt;
use std::ops::{Add, Sub};

use inset_embedder::{Clip, Offset, Size, TextBaseline, TextDirection};
use inset_foundation::{App, Handle, PRECISION_ERROR_TOLERANCE};
use inset_painting::{Axis, VerticalDirection};

use crate::box_::{
    AnyRenderBox, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
};
use crate::layer::{ClipRectLayer, LayerHandle};
use crate::layout_helper::{ChildLayoutHelper, ChildLayouter};
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// A 2D vector that uses a [`RenderWrap`]'s main axis and cross axis as its first and second
/// coordinate axes. It represents the same vector as `(main_axis_extent, cross_axis_extent)`.
#[derive(Clone, Copy)]
struct AxisSize(Size);

impl AxisSize {
    const EMPTY: AxisSize = AxisSize(Size::ZERO);

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

    fn flipped(self) -> AxisSize {
        AxisSize(self.0.flipped())
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

impl Sub for AxisSize {
    type Output = AxisSize;

    fn sub(self, other: AxisSize) -> AxisSize {
        AxisSize(Size::new(
            self.0.width() - other.0.width(),
            self.0.height() - other.0.height(),
        ))
    }
}

/// How [`RenderWrap`] should align objects.
///
/// Used both to align children within a run in the main axis as well as to align the runs
/// themselves in the cross axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WrapAlignment {
    /// Place the objects as close to the start of the axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available
    /// to determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the start is the top or the bottom.
    #[default]
    Start,

    /// Place the objects as close to the end of the axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available
    /// to determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the end is the top or the bottom.
    End,

    /// Place the objects as close to the middle of the axis as possible.
    Center,

    /// Place the free space evenly between the objects.
    SpaceBetween,

    /// Place the free space evenly between the objects as well as half of that space before and
    /// after the first and last objects.
    SpaceAround,

    /// Place the free space evenly between the objects as well as before and after the first
    /// and last objects.
    SpaceEvenly,
}

impl WrapAlignment {
    /// The space before the first object and between each two, for `item_count` objects.
    fn distribute_space(
        self,
        free_space: f64,
        item_spacing: f64,
        item_count: usize,
        flipped: bool,
    ) -> (f64, f64) {
        debug_assert!(item_count > 0);
        let count = item_count as f64;
        match self {
            WrapAlignment::Start => (if flipped { free_space } else { 0.0 }, item_spacing),
            WrapAlignment::End => WrapAlignment::Start.distribute_space(
                free_space,
                item_spacing,
                item_count,
                !flipped,
            ),
            WrapAlignment::SpaceBetween if item_count < 2 => {
                WrapAlignment::Start.distribute_space(free_space, item_spacing, item_count, flipped)
            }
            WrapAlignment::Center => (free_space / 2.0, item_spacing),
            WrapAlignment::SpaceBetween => (0.0, free_space / (count - 1.0) + item_spacing),
            WrapAlignment::SpaceAround => {
                (free_space / count / 2.0, free_space / count + item_spacing)
            }
            WrapAlignment::SpaceEvenly => (
                free_space / (count + 1.0),
                free_space / (count + 1.0) + item_spacing,
            ),
        }
    }
}

/// Who [`RenderWrap`] should align children within a run in the cross axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WrapCrossAlignment {
    /// Place the children as close to the start of the run in the cross axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available
    /// to determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the start is the top or the bottom.
    #[default]
    Start,

    /// Place the children as close to the end of the run in the cross axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [`TextDirection`] must be available
    /// to determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [`VerticalDirection`] must be available
    /// to determine if the end is the top or the bottom.
    End,

    /// Place the children as close to the middle of the run in the cross axis as possible.
    Center,
    // TODO(ianh): baseline.
}

impl WrapCrossAlignment {
    fn flipped(self) -> WrapCrossAlignment {
        match self {
            WrapCrossAlignment::Start => WrapCrossAlignment::End,
            WrapCrossAlignment::End => WrapCrossAlignment::Start,
            WrapCrossAlignment::Center => WrapCrossAlignment::Center,
        }
    }

    fn alignment(self) -> f64 {
        match self {
            WrapCrossAlignment::Start => 0.0,
            WrapCrossAlignment::End => 1.0,
            WrapCrossAlignment::Center => 0.5,
        }
    }
}

/// A run's extent, its number of children, and the child it starts placing from.
struct RunMetrics {
    axis_size: AxisSize,
    child_count: usize,
    leading_child: AnyRenderBox,
}

impl RunMetrics {
    fn new(leading_child: AnyRenderBox, axis_size: AxisSize) -> RunMetrics {
        RunMetrics {
            axis_size,
            child_count: 1,
            leading_child,
        }
    }

    // Look ahead, creates a new run if incorporating the child would exceed the allowed line
    // width.
    fn try_adding_new_child(
        &mut self,
        child: AnyRenderBox,
        child_size: AxisSize,
        flip_main_axis: bool,
        spacing: f64,
        max_main_extent: f64,
    ) -> Option<RunMetrics> {
        let needs_new_run =
            self.axis_size.main_axis_extent() + child_size.main_axis_extent() + spacing
                - max_main_extent
                > PRECISION_ERROR_TOLERANCE;
        if needs_new_run {
            return Some(RunMetrics::new(child, child_size));
        }
        self.axis_size = self.axis_size + child_size + AxisSize::new(spacing, 0.0);
        self.child_count += 1;
        if flip_main_axis {
            self.leading_child = child;
        }
        None
    }
}

/// Parent data for use with [`RenderWrap`].
#[derive(Debug, Default)]
pub struct WrapParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl WrapParentData {
    /// Creates wrap parent data at offset zero, with no siblings.
    pub const fn new() -> WrapParentData {
        WrapParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
        }
    }
}

impl ParentData for WrapParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<WrapParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<WrapParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for WrapParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for WrapParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for WrapParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)
    }
}

/// Displays its children in multiple horizontal or vertical runs.
///
/// A [`RenderWrap`] lays out each child and attempts to place the child adjacent to the previous
/// child in the main axis, given by [`direction`](Self::direction), leaving
/// [`spacing`](Self::spacing) space in between. If there is not enough space to fit the child,
/// [`RenderWrap`] creates a new _run_ adjacent to the existing children in the cross axis.
///
/// After all the children have been allocated to runs, the children within the runs are
/// positioned according to the [`alignment`](Self::alignment) in the main axis and according
/// to the [`cross_axis_alignment`](Self::cross_axis_alignment) in the cross axis.
///
/// The runs themselves are then positioned in the cross axis according to the
/// [`run_spacing`](Self::run_spacing) and [`run_alignment`](Self::run_alignment).
pub struct RenderWrap {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    direction: Axis,
    alignment: WrapAlignment,
    spacing: f64,
    run_alignment: WrapAlignment,
    run_spacing: f64,
    cross_axis_alignment: WrapCrossAlignment,
    text_direction: Option<TextDirection>,
    vertical_direction: VerticalDirection,
    clip_behavior: Clip,
    has_visual_overflow: bool,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
}

impl RenderWrap {
    /// Creates a wrap render object.
    ///
    /// By default, the wrap layout is horizontal and both the children and the runs are aligned
    /// to the start. Add children with [`add`](ContainerRenderObjectMixin::add) or
    /// [`add_all`](ContainerRenderObjectMixin::add_all); Dart's constructor takes them, and its
    /// other named arguments are the setters.
    pub fn new(app: &mut App) -> RenderHandle<RenderWrap> {
        RenderHandle::new_box(
            app,
            RenderWrap {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                direction: Axis::Horizontal,
                alignment: WrapAlignment::Start,
                spacing: 0.0,
                run_alignment: WrapAlignment::Start,
                run_spacing: 0.0,
                cross_axis_alignment: WrapCrossAlignment::Start,
                text_direction: None,
                vertical_direction: VerticalDirection::Down,
                clip_behavior: Clip::None,
                has_visual_overflow: false,
                clip_rect_layer: LayerHandle::new(),
            },
        )
    }

    /// The direction to use as the main axis.
    ///
    /// For example, if [`direction`](Self::direction) is [`Axis::Horizontal`], the default, the
    /// children are placed adjacent to one another in a horizontal run until the available
    /// horizontal space is consumed, at which point a subsequent children are placed in a new
    /// run vertically adjacent to the previous run.
    pub fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        self.get(app).direction
    }

    /// Sets [`direction`](Self::direction).
    pub fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        if self.get(app).direction == value {
            return;
        }
        self.get_mut(app).direction = value;
        self.mark_needs_layout(app);
    }

    /// How the children within a run should be placed in the main axis.
    ///
    /// For example, if [`alignment`](Self::alignment) is [`WrapAlignment::Center`], the
    /// children in each run are grouped together in the center of their run in the main axis.
    ///
    /// Defaults to [`WrapAlignment::Start`].
    ///
    /// See also:
    ///
    ///  * [`run_alignment`](Self::run_alignment), which controls how the runs are placed
    ///    relative to each other in the cross axis.
    ///  * [`cross_axis_alignment`](Self::cross_axis_alignment), which controls how the children
    ///    within each run are placed relative to each other in the cross axis.
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> WrapAlignment {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: WrapAlignment) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.mark_needs_layout(app);
    }

    /// How much space to place between children in a run in the main axis.
    ///
    /// For example, if [`spacing`](Self::spacing) is 10.0, the children will be spaced at least
    /// 10.0 logical pixels apart in the main axis.
    ///
    /// If there is additional free space in a run (e.g., because the wrap has a minimum size
    /// that is not filled or because some runs are longer than others), the additional free
    /// space will be allocated according to the [`alignment`](Self::alignment).
    ///
    /// Defaults to 0.0.
    pub fn spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).spacing
    }

    /// Sets [`spacing`](Self::spacing).
    pub fn set_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).spacing == value {
            return;
        }
        self.get_mut(app).spacing = value;
        self.mark_needs_layout(app);
    }

    /// How the runs themselves should be placed in the cross axis.
    ///
    /// For example, if [`run_alignment`](Self::run_alignment) is [`WrapAlignment::Center`], the
    /// runs are grouped together in the center of the overall [`RenderWrap`] in the cross axis.
    ///
    /// Defaults to [`WrapAlignment::Start`].
    ///
    /// See also:
    ///
    ///  * [`alignment`](Self::alignment), which controls how the children within each run are
    ///    placed relative to each other in the main axis.
    ///  * [`cross_axis_alignment`](Self::cross_axis_alignment), which controls how the children
    ///    within each run are placed relative to each other in the cross axis.
    pub fn run_alignment(self: RenderHandle<Self>, app: &App) -> WrapAlignment {
        self.get(app).run_alignment
    }

    /// Sets [`run_alignment`](Self::run_alignment).
    pub fn set_run_alignment(self: RenderHandle<Self>, app: &mut App, value: WrapAlignment) {
        if self.get(app).run_alignment == value {
            return;
        }
        self.get_mut(app).run_alignment = value;
        self.mark_needs_layout(app);
    }

    /// How much space to place between the runs themselves in the cross axis.
    ///
    /// For example, if [`run_spacing`](Self::run_spacing) is 10.0, the runs will be spaced at
    /// least 10.0 logical pixels apart in the cross axis.
    ///
    /// If there is additional free space in the overall [`RenderWrap`] (e.g., because the wrap
    /// has a minimum size that is not filled), the additional free space will be allocated
    /// according to the [`run_alignment`](Self::run_alignment).
    ///
    /// Defaults to 0.0.
    pub fn run_spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).run_spacing
    }

    /// Sets [`run_spacing`](Self::run_spacing).
    pub fn set_run_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).run_spacing == value {
            return;
        }
        self.get_mut(app).run_spacing = value;
        self.mark_needs_layout(app);
    }

    /// How the children within a run should be aligned relative to each other in the cross
    /// axis.
    ///
    /// For example, if this is set to [`WrapCrossAlignment::End`], and the
    /// [`direction`](Self::direction) is [`Axis::Horizontal`], then the children within each
    /// run will have their bottom edges aligned to the bottom edge of the run.
    ///
    /// Defaults to [`WrapCrossAlignment::Start`].
    ///
    /// See also:
    ///
    ///  * [`alignment`](Self::alignment), which controls how the children within each run are
    ///    placed relative to each other in the main axis.
    ///  * [`run_alignment`](Self::run_alignment), which controls how the runs are placed
    ///    relative to each other in the cross axis.
    pub fn cross_axis_alignment(self: RenderHandle<Self>, app: &App) -> WrapCrossAlignment {
        self.get(app).cross_axis_alignment
    }

    /// Sets [`cross_axis_alignment`](Self::cross_axis_alignment).
    pub fn set_cross_axis_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: WrapCrossAlignment,
    ) {
        if self.get(app).cross_axis_alignment == value {
            return;
        }
        self.get_mut(app).cross_axis_alignment = value;
        self.mark_needs_layout(app);
    }

    /// Determines the order to lay children out horizontally and how to interpret `start` and
    /// `end` in the horizontal direction.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], this controls the order in
    /// which children are positioned (left-to-right or right-to-left), and the meaning of the
    /// [`alignment`](Self::alignment) property's [`WrapAlignment::Start`] and
    /// [`WrapAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], and either the
    /// [`alignment`](Self::alignment) is either [`WrapAlignment::Start`] or
    /// [`WrapAlignment::End`], or there's more than one child, then the
    /// [`text_direction`](Self::text_direction) must not be `None`.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], this controls the order in
    /// which runs are positioned, the meaning of the [`run_alignment`](Self::run_alignment)
    /// property's [`WrapAlignment::Start`] and [`WrapAlignment::End`] values, as well as the
    /// [`cross_axis_alignment`](Self::cross_axis_alignment) property's
    /// [`WrapCrossAlignment::Start`] and [`WrapCrossAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], and either the
    /// [`run_alignment`](Self::run_alignment) is either [`WrapAlignment::Start`] or
    /// [`WrapAlignment::End`], the [`cross_axis_alignment`](Self::cross_axis_alignment) is
    /// either [`WrapCrossAlignment::Start`] or [`WrapCrossAlignment::End`], or there's more
    /// than one child, then the [`text_direction`](Self::text_direction) must not be `None`.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction != value {
            self.get_mut(app).text_direction = value;
            self.mark_needs_layout(app);
        }
    }

    /// Determines the order to lay children out vertically and how to interpret `start` and
    /// `end` in the vertical direction.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], this controls which order
    /// children are painted in (down or up), the meaning of the [`alignment`](Self::alignment)
    /// property's [`WrapAlignment::Start`] and [`WrapAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], this controls the order
    /// in which runs are positioned, the meaning of the [`run_alignment`](Self::run_alignment)
    /// property's [`WrapAlignment::Start`] and [`WrapAlignment::End`] values, as well as the
    /// [`cross_axis_alignment`](Self::cross_axis_alignment) property's
    /// [`WrapCrossAlignment::Start`] and [`WrapCrossAlignment::End`] values.
    pub fn vertical_direction(self: RenderHandle<Self>, app: &App) -> VerticalDirection {
        self.get(app).vertical_direction
    }

    /// Sets [`vertical_direction`](Self::vertical_direction).
    pub fn set_vertical_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: VerticalDirection,
    ) {
        if self.get(app).vertical_direction != value {
            self.get_mut(app).vertical_direction = value;
            self.mark_needs_layout(app);
        }
    }

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value != self.get(app).clip_behavior {
            self.get_mut(app).clip_behavior = value;
            self.mark_needs_paint(app);
        }
    }

    /// Flutter's `_debugHasNecessaryDirections`: asserts the directions the current alignments
    /// need are available. Always returns true.
    fn debug_has_necessary_directions(self: RenderHandle<Self>, app: &App) -> bool {
        let this = self.get(app);
        let has_text_direction = this.text_direction.is_some();
        if self.first_child(app).is_some() && self.last_child(app) != self.first_child(app) {
            // i.e. there's more than one child
            if this.direction == Axis::Horizontal {
                debug_assert!(
                    has_text_direction,
                    "Horizontal RenderWrap with multiple children has a null textDirection, so \
                     the layout order is undefined."
                );
            }
        }
        let alignment = this.alignment;
        if matches!(alignment, WrapAlignment::Start | WrapAlignment::End)
            && this.direction == Axis::Horizontal
        {
            debug_assert!(
                has_text_direction,
                "Horizontal RenderWrap with alignment {alignment:?} has a null textDirection, so \
                 the alignment cannot be resolved."
            );
        }
        let run_alignment = this.run_alignment;
        if matches!(run_alignment, WrapAlignment::Start | WrapAlignment::End)
            && this.direction == Axis::Vertical
        {
            debug_assert!(
                has_text_direction,
                "Vertical RenderWrap with runAlignment {run_alignment:?} has a null \
                 textDirection, so the alignment cannot be resolved."
            );
        }
        let cross_axis_alignment = this.cross_axis_alignment;
        if matches!(
            cross_axis_alignment,
            WrapCrossAlignment::Start | WrapCrossAlignment::End
        ) && this.direction == Axis::Vertical
        {
            debug_assert!(
                has_text_direction,
                "Vertical RenderWrap with crossAxisAlignment {cross_axis_alignment:?} has a \
                 null textDirection, so the alignment cannot be resolved."
            );
        }
        true
    }

    fn get_main_axis_extent(self: RenderHandle<Self>, app: &App, child_size: Size) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => child_size.width(),
            Axis::Vertical => child_size.height(),
        }
    }

    fn get_cross_axis_extent(self: RenderHandle<Self>, app: &App, child_size: Size) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => child_size.height(),
            Axis::Vertical => child_size.width(),
        }
    }

    fn get_offset(
        self: RenderHandle<Self>,
        app: &App,
        main_axis_offset: f64,
        cross_axis_offset: f64,
    ) -> Offset {
        match self.direction(app) {
            Axis::Horizontal => Offset::new(main_axis_offset, cross_axis_offset),
            Axis::Vertical => Offset::new(cross_axis_offset, main_axis_offset),
        }
    }

    /// Whether the main axis and the cross axis run against their coordinates.
    fn are_axes_flipped(self: RenderHandle<Self>, app: &App) -> (bool, bool) {
        let this = self.get(app);
        let flip_horizontal = match this.text_direction.unwrap_or(TextDirection::Ltr) {
            TextDirection::Ltr => false,
            TextDirection::Rtl => true,
        };
        let flip_vertical = match this.vertical_direction {
            VerticalDirection::Down => false,
            VerticalDirection::Up => true,
        };
        match this.direction {
            Axis::Horizontal => (flip_horizontal, flip_vertical),
            Axis::Vertical => (flip_vertical, flip_horizontal),
        }
    }

    /// The constraints a child is laid out with, and the main axis extent a run fills.
    fn child_constraints_and_main_axis_limit(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> (BoxConstraints, f64) {
        match self.direction(app) {
            Axis::Horizontal => (
                BoxConstraints::new().max_width(constraints.max_width),
                constraints.max_width,
            ),
            Axis::Vertical => (
                BoxConstraints::new().max_height(constraints.max_height),
                constraints.max_height,
            ),
        }
    }

    /// The size this wrap would take, measuring the children dry.
    fn compute_dry_layout_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let (child_constraints, main_axis_limit) =
            self.child_constraints_and_main_axis_limit(app, constraints);
        let (spacing, run_spacing) = (self.spacing(app), self.run_spacing(app));

        let mut main_axis_extent: f64 = 0.0;
        let mut cross_axis_extent = 0.0;
        let mut run_main_axis_extent = 0.0;
        let mut run_cross_axis_extent: f64 = 0.0;
        let mut child_count = 0;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_size = ChildLayoutHelper::dry_layout_child(app, current, child_constraints);
            let child_main_axis_extent = self.get_main_axis_extent(app, child_size);
            let child_cross_axis_extent = self.get_cross_axis_extent(app, child_size);
            // There must be at least one child before we move on to the next run.
            if child_count > 0
                && run_main_axis_extent + child_main_axis_extent + spacing > main_axis_limit
            {
                main_axis_extent = main_axis_extent.max(run_main_axis_extent);
                cross_axis_extent += run_cross_axis_extent + run_spacing;
                run_main_axis_extent = 0.0;
                run_cross_axis_extent = 0.0;
                child_count = 0;
            }
            run_main_axis_extent += child_main_axis_extent;
            run_cross_axis_extent = run_cross_axis_extent.max(child_cross_axis_extent);
            if child_count > 0 {
                run_main_axis_extent += spacing;
            }
            child_count += 1;
            child = self.child_after(app, current);
        }
        cross_axis_extent += run_cross_axis_extent;
        main_axis_extent = main_axis_extent.max(run_main_axis_extent);

        constraints.constrain(match self.direction(app) {
            Axis::Horizontal => Size::new(main_axis_extent, cross_axis_extent),
            Axis::Vertical => Size::new(cross_axis_extent, main_axis_extent),
        })
    }

    /// Lays out (or measures) the children with `layout_child` and splits them into runs:
    /// the extent the runs take together, and each run.
    fn compute_runs(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> (AxisSize, Vec<RunMetrics>) {
        debug_assert!(self.first_child(app).is_some());
        let (child_constraints, main_axis_limit) =
            self.child_constraints_and_main_axis_limit(app, constraints);

        let (flip_main_axis, _) = self.are_axes_flipped(app);
        let direction = self.direction(app);
        let spacing = self.spacing(app);
        let mut run_metrics: Vec<RunMetrics> = Vec::new();

        let mut children_axis_size = AxisSize::EMPTY;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_size =
                AxisSize::from_size(layout_child(app, current, child_constraints), direction);
            let new_run = match run_metrics.last_mut() {
                None => Some(RunMetrics::new(current, child_size)),
                Some(current_run) => current_run.try_adding_new_child(
                    current,
                    child_size,
                    flip_main_axis,
                    spacing,
                    main_axis_limit,
                ),
            };
            if let Some(new_run) = new_run {
                if let Some(current_run) = run_metrics.last() {
                    children_axis_size = children_axis_size + current_run.axis_size.flipped();
                }
                run_metrics.push(new_run);
            }
            child = self.child_after(app, current);
        }
        debug_assert!(!run_metrics.is_empty());
        let total_run_spacing = self.run_spacing(app) * (run_metrics.len() - 1) as f64;
        let current_run = run_metrics.last().expect("at least one run");
        children_axis_size = children_axis_size
            + (AxisSize::new(total_run_spacing, 0.0) + current_run.axis_size.flipped());
        (children_axis_size.flipped(), run_metrics)
    }

    /// Places each child of `run_metrics` within the container: `position_child` is told each
    /// child's offset, and `get_child_size` gives each child's size.
    fn position_children(
        self: RenderHandle<Self>,
        app: &mut App,
        run_metrics: &[RunMetrics],
        free_axis_size: AxisSize,
        container_axis_size: AxisSize,
        position_child: &mut dyn FnMut(&mut App, Offset, AnyRenderBox),
        get_child_size: &mut dyn FnMut(&mut App, AnyRenderBox) -> Size,
    ) {
        debug_assert!(!run_metrics.is_empty());

        let direction = self.direction(app);
        let spacing = self.spacing(app);

        let cross_axis_free_space = free_axis_size.cross_axis_extent().max(0.0);

        let (flip_main_axis, flip_cross_axis) = self.are_axes_flipped(app);
        let cross_axis_alignment = self.cross_axis_alignment(app);
        let effective_cross_alignment = if flip_cross_axis {
            cross_axis_alignment.flipped()
        } else {
            cross_axis_alignment
        };
        let (run_leading_space, run_between_space) = self.run_alignment(app).distribute_space(
            cross_axis_free_space,
            self.run_spacing(app),
            run_metrics.len(),
            flip_cross_axis,
        );
        let alignment = self.alignment(app);

        let mut run_cross_axis_offset = run_leading_space;
        let runs: Box<dyn Iterator<Item = &RunMetrics>> = if flip_cross_axis {
            Box::new(run_metrics.iter().rev())
        } else {
            Box::new(run_metrics.iter())
        };
        for run in runs {
            let run_cross_axis_extent = run.axis_size.cross_axis_extent();
            let child_count = run.child_count;

            let main_axis_free_space = (container_axis_size.main_axis_extent()
                - run.axis_size.main_axis_extent())
            .max(0.0);
            let (child_leading_space, child_between_space) = alignment.distribute_space(
                main_axis_free_space,
                spacing,
                child_count,
                flip_main_axis,
            );

            let mut child_main_axis_offset = child_leading_space;

            let mut remaining_child_count = run.child_count;
            let mut child = Some(run.leading_child);
            while let Some(current) = child
                && remaining_child_count > 0
            {
                let child_axis_size = AxisSize::from_size(get_child_size(app, current), direction);
                let child_cross_axis_offset = effective_cross_alignment.alignment()
                    * (run_cross_axis_extent - child_axis_size.cross_axis_extent());
                let offset = self.get_offset(
                    app,
                    child_main_axis_offset,
                    run_cross_axis_offset + child_cross_axis_offset,
                );
                position_child(app, offset, current);
                child_main_axis_offset += child_axis_size.main_axis_extent() + child_between_space;
                child = if flip_main_axis {
                    self.child_before(app, current)
                } else {
                    self.child_after(app, current)
                };
                remaining_child_count -= 1;
            }
            run_cross_axis_offset += run_cross_axis_extent + run_between_space;
        }
    }
}

impl ContainerRenderObjectMixin for RenderWrap {
    type ChildType = AnyRenderBox;
    type ParentDataType = WrapParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderWrap {}

impl RenderObject for RenderWrap {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        debug_assert!(self.debug_has_necessary_directions(app));
        if self.first_child(app).is_none() {
            self.set_size(app, constraints.smallest());
            self.get_mut(app).has_visual_overflow = false;
            return;
        }

        let direction = self.direction(app);
        let (children_axis_size, run_metrics) =
            self.compute_runs(app, constraints, ChildLayoutHelper::layout_child);
        let container_axis_size = children_axis_size.apply_constraints(constraints, direction);
        self.set_size(app, container_axis_size.to_size(direction));
        let free_axis_size = container_axis_size - children_axis_size;
        self.get_mut(app).has_visual_overflow =
            free_axis_size.main_axis_extent() < 0.0 || free_axis_size.cross_axis_extent() < 0.0;
        self.position_children(
            app,
            &run_metrics,
            free_axis_size,
            container_axis_size,
            &mut |app, offset, child| {
                child
                    .parent_data_of_mut::<WrapParentData>(app)
                    .set_offset(offset);
            },
            &mut |app, child| child.size(app),
        );
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

    /// Paints the children, clipping them to this wrap when they overflow it and
    /// [`clip_behavior`](RenderWrap::clip_behavior) asks for it.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        // TODO(ianh): move the debug flex overflow paint logic somewhere common so it can be
        // reused here
        let clip_behavior = self.clip_behavior(app);
        if self.get(app).has_visual_overflow && clip_behavior != Clip::None {
            let size = self.size(app);
            let old = self.get(app).clip_rect_layer.layer();
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                Offset::ZERO & size,
                |app, context, offset| self.default_paint(app, context, offset),
                clip_behavior,
                old,
            );
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, layer);
        } else {
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
            self.default_paint(app, context, offset);
        }
    }

    /// Flutter's `dispose`: drop the clip layer handle, then `super.dispose()`.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
        crate::object::RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderWrap {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<WrapParentData>(app) {
            child.set_parent_data(app, WrapParentData::new());
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => {
                let mut width: f64 = 0.0;
                let mut child = self.first_child(app);
                while let Some(current) = child {
                    width = width.max(current.get_min_intrinsic_width(app, f64::INFINITY));
                    child = self.child_after(app, current);
                }
                width
            }
            Axis::Vertical => self
                .get_dry_layout(app, BoxConstraints::new().max_height(height))
                .width(),
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => {
                let mut width = 0.0;
                let mut child = self.first_child(app);
                while let Some(current) = child {
                    width += current.get_max_intrinsic_width(app, f64::INFINITY);
                    child = self.child_after(app, current);
                }
                width
            }
            Axis::Vertical => self
                .get_dry_layout(app, BoxConstraints::new().max_height(height))
                .width(),
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => self
                .get_dry_layout(app, BoxConstraints::new().max_width(width))
                .height(),
            Axis::Vertical => {
                let mut height: f64 = 0.0;
                let mut child = self.first_child(app);
                while let Some(current) = child {
                    height = height.max(current.get_min_intrinsic_height(app, f64::INFINITY));
                    child = self.child_after(app, current);
                }
                height
            }
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.direction(app) {
            Axis::Horizontal => self
                .get_dry_layout(app, BoxConstraints::new().max_width(width))
                .height(),
            Axis::Vertical => {
                let mut height = 0.0;
                let mut child = self.first_child(app);
                while let Some(current) = child {
                    height += current.get_max_intrinsic_height(app, f64::INFINITY);
                    child = self.child_after(app, current);
                }
                height
            }
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.default_compute_distance_to_highest_actual_baseline(app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.first_child(app)?;
        let child_constraints = match self.direction(app) {
            Axis::Horizontal => BoxConstraints::new().max_width(constraints.max_width),
            Axis::Vertical => BoxConstraints::new().max_height(constraints.max_height),
        };

        let (children_axis_size, run_metrics) =
            self.compute_runs(app, constraints, ChildLayoutHelper::dry_layout_child);
        let container_axis_size =
            children_axis_size.apply_constraints(constraints, self.direction(app));

        let mut baseline_offset = BaselineOffset::NO_BASELINE;
        self.position_children(
            app,
            &run_metrics,
            children_axis_size,
            container_axis_size,
            &mut |app, offset, child| {
                baseline_offset = baseline_offset.min_of(
                    BaselineOffset(child.get_dry_baseline(app, child_constraints, baseline))
                        + offset.dy(),
                );
            },
            &mut |app, child| child.get_dry_layout(app, child_constraints),
        );
        baseline_offset.offset()
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_dry_layout_size(app, constraints)
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

#[cfg(test)]
mod tests {
    use inset_embedder::valo::Op;
    use inset_foundation::AppCell;

    use super::*;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::proxy_box::{RenderConstrainedBox, RenderRepaintBoundary};

    /// A box at least 80 square, as `wrap_test.dart`'s children are.
    fn box_80(app: &mut App) -> AnyRenderBox {
        let at_least = BoxConstraints::new().min_width(80.0).min_height(80.0);
        RenderConstrainedBox::new(app, at_least, None).as_box()
    }

    /// A left-to-right wrap of three 80 square boxes, `spacing` apart in both axes.
    fn wrap_of_three(app: &mut App, direction: Axis, spacing: f64) -> RenderHandle<RenderWrap> {
        let wrap = RenderWrap::new(app);
        let children = vec![box_80(app), box_80(app), box_80(app)];
        wrap.add_all(app, Some(children));
        wrap.set_spacing(app, spacing);
        wrap.set_run_spacing(app, spacing);
        wrap.set_direction(app, direction);
        wrap
    }

    /// `wrap_test.dart`: "Compute intrinsic height test".
    #[test]
    fn a_horizontal_wrap_is_as_tall_as_its_runs() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Horizontal, 5.0);
        for (width, height) in [(245.0, 165.0), (250.0, 80.0), (80.0, 250.0), (79.0, 250.0)] {
            assert_eq!(wrap.compute_max_intrinsic_height(&mut app, width), height);
            assert_eq!(wrap.compute_min_intrinsic_height(&mut app, width), height);
        }
    }

    /// `wrap_test.dart`: "Compute intrinsic width test".
    #[test]
    fn a_vertical_wrap_is_as_wide_as_its_runs() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Vertical, 5.0);
        for (height, width) in [(245.0, 165.0), (250.0, 80.0), (80.0, 250.0), (79.0, 250.0)] {
            assert_eq!(wrap.compute_max_intrinsic_width(&mut app, height), width);
            assert_eq!(wrap.compute_min_intrinsic_width(&mut app, height), width);
        }
    }

    /// `wrap_test.dart`: "Compute intrinsic height for only one run".
    #[test]
    fn one_child_makes_one_run_whatever_the_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = RenderWrap::new(&mut app);
        let child = box_80(&mut app);
        wrap.add(&mut app, child);
        wrap.set_spacing(&mut app, 5.0);
        wrap.set_run_spacing(&mut app, 5.0);
        for width in [100.0, 79.0, 80.0] {
            assert_eq!(wrap.compute_max_intrinsic_height(&mut app, width), 80.0);
            assert_eq!(wrap.compute_min_intrinsic_height(&mut app, width), 80.0);
        }
    }

    fn offsets(app: &App, wrap: RenderHandle<RenderWrap>) -> Vec<Offset> {
        let mut offsets = Vec::new();
        let mut child = wrap.first_child(app);
        while let Some(current) = child {
            offsets.push(current.box_parent_data(app).offset);
            child = wrap.child_after(app, current);
        }
        offsets
    }

    #[test]
    fn children_that_overflow_a_run_start_the_next_one() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Horizontal, 10.0);
        wrap.set_text_direction(&mut app, Some(TextDirection::Ltr));
        wrap.as_box()
            .layout(&mut app, BoxConstraints::new().max_width(200.0), true);
        assert_eq!(wrap.size(&app), Size::new(170.0, 170.0));
        assert_eq!(
            offsets(&app, wrap),
            [
                Offset::new(0.0, 0.0),
                Offset::new(90.0, 0.0),
                Offset::new(0.0, 90.0)
            ]
        );
    }

    #[test]
    fn a_right_to_left_wrap_places_its_runs_from_the_right() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Horizontal, 10.0);
        wrap.set_text_direction(&mut app, Some(TextDirection::Rtl));
        wrap.set_alignment(&mut app, WrapAlignment::Start);
        wrap.as_box().layout(
            &mut app,
            BoxConstraints::tight(Size::new(200.0, 200.0)),
            true,
        );
        // The first run's 170 sit against the right edge, 30 from the left.
        assert_eq!(
            offsets(&app, wrap),
            [
                Offset::new(120.0, 0.0),
                Offset::new(30.0, 0.0),
                Offset::new(120.0, 90.0)
            ]
        );
    }

    #[test]
    fn alignments_spread_the_free_space() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Horizontal, 0.0);
        wrap.set_text_direction(&mut app, Some(TextDirection::Ltr));
        wrap.set_alignment(&mut app, WrapAlignment::SpaceBetween);
        wrap.set_run_alignment(&mut app, WrapAlignment::End);
        wrap.set_cross_axis_alignment(&mut app, WrapCrossAlignment::Center);
        wrap.as_box().layout(
            &mut app,
            BoxConstraints::tight(Size::new(300.0, 100.0)),
            true,
        );
        assert_eq!(
            offsets(&app, wrap),
            [
                Offset::new(0.0, 20.0),
                Offset::new(110.0, 20.0),
                Offset::new(220.0, 20.0)
            ]
        );
        assert_eq!(
            WrapAlignment::SpaceEvenly.distribute_space(40.0, 0.0, 3, false),
            (10.0, 10.0)
        );
        assert_eq!(
            WrapAlignment::SpaceAround.distribute_space(60.0, 5.0, 3, false),
            (10.0, 25.0)
        );
    }

    #[test]
    fn a_wrap_measures_dry_as_it_lays_out() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let wrap = wrap_of_three(&mut app, Axis::Horizontal, 10.0);
        wrap.set_text_direction(&mut app, Some(TextDirection::Ltr));
        let constraints = BoxConstraints::new().max_width(200.0);
        let dry = wrap.get_dry_layout(&mut app, constraints);
        wrap.as_box().layout(&mut app, constraints, true);
        assert_eq!(dry, wrap.size(&app));
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
        let owner = PipelineOwner::new(app, None, None);
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

    /// `wrap_test.dart`: "Wrap respects clipBehavior", which defaults to no clip.
    #[test]
    fn an_overflowing_wrap_clips_only_when_asked() {
        for (clip_behavior, clips) in [(None, false), (Some(Clip::HardEdge), true)] {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let child = RenderConstrainedBox::new(
                &mut app,
                BoxConstraints::tight(Size::square(200.0)),
                None,
            );
            let wrap = RenderWrap::new(&mut app);
            wrap.set_text_direction(&mut app, Some(TextDirection::Ltr));
            if let Some(clip_behavior) = clip_behavior {
                wrap.set_clip_behavior(&mut app, clip_behavior);
            }
            wrap.add(&mut app, child.as_box());
            let root = RenderRepaintBoundary::new(&mut app, Some(wrap.as_box()));

            let owner = first_frame(
                &mut app,
                root.as_box(),
                BoxConstraints::new().max_width(100.0).max_height(100.0),
            );

            assert_eq!(
                picture_has_clip(&app, root.as_object()),
                clips,
                "{clip_behavior:?}"
            );
            let _ = owner;
        }
    }
}
