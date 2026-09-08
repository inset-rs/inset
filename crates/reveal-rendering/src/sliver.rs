//! Flutter counterpart: `rendering/sliver.dart`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Display};
use std::hash::Hasher;
use std::ops::{Deref, DerefMut};

use reveal_embedder::{Matrix4, Offset, Rect, Size, clamp_double};
use reveal_foundation::{App, Handle, HandleId, PRECISION_ERROR_TOLERANCE};
use reveal_gestures::{HitTestEntry, HitTestResult, HitTestTarget, PointerEvent};
use reveal_painting::{
    Axis, AxisDirection, axis_direction_is_reversed, axis_direction_to_axis, flip_axis_direction,
};

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult};
use crate::object::{
    AnyRenderObject, Constraints, ContainerParentData, ContainerParentDataMixin, ParentData,
    RenderHandle, RenderObject, RenderObjectVTable, RenderObjectWithChildData,
    RenderObjectWithChildMixin, create, resolve, translate,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::viewport_offset::{ScrollDirection, flip_scroll_direction};

/// Signature for a callback that reports the main-axis extent of the item at `index`.
///
/// Used by `ListView.itemExtentBuilder` and `SliverVariedExtentList.itemExtentBuilder`.
///
/// Dart compares two builders with `==`; an [`Rc`](std::rc::Rc) compares by pointer.
pub type ItemExtentBuilder = std::rc::Rc<dyn Fn(i32, SliverLayoutDimensions) -> Option<f64>>;

/// Relates the dimensions of the [`RenderSliver`] during layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverLayoutDimensions {
    /// Scroll offset in this sliver's coordinate system of the earliest visible part of this
    /// sliver.
    pub scroll_offset: f64,

    /// Scroll distance consumed by all slivers that came before this one.
    pub preceding_scroll_extent: f64,

    /// The number of pixels the viewport can display in the main axis.
    ///
    /// For a vertical list, this is the height of the viewport.
    pub viewport_main_axis_extent: f64,

    /// The number of pixels in the cross-axis.
    ///
    /// For a vertical list, this is the width of the sliver.
    pub cross_axis_extent: f64,
}

impl SliverLayoutDimensions {
    /// Constructs the layout dimensions with the specified parameters.
    pub const fn new(
        scroll_offset: f64,
        preceding_scroll_extent: f64,
        viewport_main_axis_extent: f64,
        cross_axis_extent: f64,
    ) -> SliverLayoutDimensions {
        SliverLayoutDimensions {
            scroll_offset,
            preceding_scroll_extent,
            viewport_main_axis_extent,
            cross_axis_extent,
        }
    }
}

impl Display for SliverLayoutDimensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scrollOffset: {} precedingScrollExtent: {} viewportMainAxisExtent: {} \
             crossAxisExtent: {}",
            self.scroll_offset,
            self.preceding_scroll_extent,
            self.viewport_main_axis_extent,
            self.cross_axis_extent
        )
    }
}

/// The direction in which a sliver's contents are ordered, relative to the
/// scroll offset axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrowthDirection {
    /// Contents are ordered in the same direction as the [`AxisDirection`].
    Forward,

    /// Contents are ordered in the opposite direction of the [`AxisDirection`].
    Reverse,
}

/// Flips the [`AxisDirection`] if the [`GrowthDirection`] is
/// [`GrowthDirection::Reverse`].
pub fn apply_growth_direction_to_axis_direction(
    axis_direction: AxisDirection,
    growth_direction: GrowthDirection,
) -> AxisDirection {
    match growth_direction {
        GrowthDirection::Forward => axis_direction,
        GrowthDirection::Reverse => flip_axis_direction(axis_direction),
    }
}

/// Flips the [`ScrollDirection`] if the [`GrowthDirection`] is
/// [`GrowthDirection::Reverse`].
pub fn apply_growth_direction_to_scroll_direction(
    scroll_direction: ScrollDirection,
    growth_direction: GrowthDirection,
) -> ScrollDirection {
    match growth_direction {
        GrowthDirection::Forward => scroll_direction,
        GrowthDirection::Reverse => flip_scroll_direction(scroll_direction),
    }
}

/// Immutable layout constraints for [`RenderSliver`] layout.
#[derive(Clone, Copy, Debug)]
pub struct SliverConstraints {
    /// The direction in which [`scroll_offset`](Self::scroll_offset) and
    /// [`remaining_paint_extent`](Self::remaining_paint_extent) increase.
    pub axis_direction: AxisDirection,
    /// The direction in which the contents of slivers are ordered.
    pub growth_direction: GrowthDirection,
    /// The direction in which the user is attempting to scroll.
    pub user_scroll_direction: ScrollDirection,
    /// Scroll offset in this sliver's coordinate system of the earliest visible
    /// part of this sliver.
    pub scroll_offset: f64,
    /// Scroll distance consumed by all slivers that came before this one.
    pub preceding_scroll_extent: f64,
    /// Pixels from the painted [`scroll_offset`](Self::scroll_offset) up to the
    /// first pixel not yet painted by an earlier sliver.
    pub overlap: f64,
    /// Pixels of content the sliver should consider providing.
    pub remaining_paint_extent: f64,
    /// Pixels in the cross-axis.
    pub cross_axis_extent: f64,
    /// Direction in which children should be placed in the cross axis.
    pub cross_axis_direction: AxisDirection,
    /// Pixels the viewport can display in the main axis.
    pub viewport_main_axis_extent: f64,
    /// How much content the sliver should provide starting from
    /// [`cache_origin`](Self::cache_origin).
    pub remaining_cache_extent: f64,
    /// Where the cache area starts relative to the
    /// [`scroll_offset`](Self::scroll_offset). Always negative or zero.
    pub cache_origin: f64,
}

impl SliverConstraints {
    /// Creates sliver constraints with the given information.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        axis_direction: AxisDirection,
        growth_direction: GrowthDirection,
        user_scroll_direction: ScrollDirection,
        scroll_offset: f64,
        preceding_scroll_extent: f64,
        overlap: f64,
        remaining_paint_extent: f64,
        cross_axis_extent: f64,
        cross_axis_direction: AxisDirection,
        viewport_main_axis_extent: f64,
        remaining_cache_extent: f64,
        cache_origin: f64,
    ) -> SliverConstraints {
        SliverConstraints {
            axis_direction,
            growth_direction,
            user_scroll_direction,
            scroll_offset,
            preceding_scroll_extent,
            overlap,
            remaining_paint_extent,
            cross_axis_extent,
            cross_axis_direction,
            viewport_main_axis_extent,
            remaining_cache_extent,
            cache_origin,
        }
    }

    /// Creates a copy of this object. Chain field replacements.
    pub const fn copy_with(self) -> SliverConstraints {
        self
    }

    /// Sets [`axis_direction`](Self::axis_direction).
    pub const fn axis_direction(mut self, value: AxisDirection) -> SliverConstraints {
        self.axis_direction = value;
        self
    }

    /// Sets [`growth_direction`](Self::growth_direction).
    pub const fn growth_direction(mut self, value: GrowthDirection) -> SliverConstraints {
        self.growth_direction = value;
        self
    }

    /// Sets [`user_scroll_direction`](Self::user_scroll_direction).
    pub const fn user_scroll_direction(mut self, value: ScrollDirection) -> SliverConstraints {
        self.user_scroll_direction = value;
        self
    }

    /// Sets [`scroll_offset`](Self::scroll_offset).
    pub const fn scroll_offset(mut self, value: f64) -> SliverConstraints {
        self.scroll_offset = value;
        self
    }

    /// Sets [`preceding_scroll_extent`](Self::preceding_scroll_extent).
    pub const fn preceding_scroll_extent(mut self, value: f64) -> SliverConstraints {
        self.preceding_scroll_extent = value;
        self
    }

    /// Sets [`overlap`](Self::overlap).
    pub const fn overlap(mut self, value: f64) -> SliverConstraints {
        self.overlap = value;
        self
    }

    /// Sets [`remaining_paint_extent`](Self::remaining_paint_extent).
    pub const fn remaining_paint_extent(mut self, value: f64) -> SliverConstraints {
        self.remaining_paint_extent = value;
        self
    }

    /// Sets [`cross_axis_extent`](Self::cross_axis_extent).
    pub const fn cross_axis_extent(mut self, value: f64) -> SliverConstraints {
        self.cross_axis_extent = value;
        self
    }

    /// Sets [`cross_axis_direction`](Self::cross_axis_direction).
    pub const fn cross_axis_direction(mut self, value: AxisDirection) -> SliverConstraints {
        self.cross_axis_direction = value;
        self
    }

    /// Sets [`viewport_main_axis_extent`](Self::viewport_main_axis_extent).
    pub const fn viewport_main_axis_extent(mut self, value: f64) -> SliverConstraints {
        self.viewport_main_axis_extent = value;
        self
    }

    /// Sets [`remaining_cache_extent`](Self::remaining_cache_extent).
    pub const fn remaining_cache_extent(mut self, value: f64) -> SliverConstraints {
        self.remaining_cache_extent = value;
        self
    }

    /// Sets [`cache_origin`](Self::cache_origin).
    pub const fn cache_origin(mut self, value: f64) -> SliverConstraints {
        self.cache_origin = value;
        self
    }

    /// The axis along which the scroll offset is measured.
    pub fn axis(self) -> Axis {
        axis_direction_to_axis(self.axis_direction)
    }

    /// [`growth_direction`](Self::growth_direction) as if the axis were down or
    /// right.
    pub fn normalized_growth_direction(self) -> GrowthDirection {
        if axis_direction_is_reversed(self.axis_direction) {
            match self.growth_direction {
                GrowthDirection::Forward => GrowthDirection::Reverse,
                GrowthDirection::Reverse => GrowthDirection::Forward,
            }
        } else {
            self.growth_direction
        }
    }

    /// [`BoxConstraints`] that reflect these sliver constraints.
    ///
    /// Dart named defaults: `min_extent` 0, `max_extent` infinity, `cross_axis_extent`
    /// this sliver's cross extent.
    pub fn as_box_constraints(
        self,
        min_extent: f64,
        max_extent: f64,
        cross_axis_extent: Option<f64>,
    ) -> BoxConstraints {
        let cross = cross_axis_extent.unwrap_or(self.cross_axis_extent);
        match self.axis() {
            Axis::Horizontal => BoxConstraints {
                min_width: min_extent,
                max_width: max_extent,
                min_height: cross,
                max_height: cross,
            },
            Axis::Vertical => BoxConstraints {
                min_width: cross,
                max_width: cross,
                min_height: min_extent,
                max_height: max_extent,
            },
        }
    }
}

impl Constraints for SliverConstraints {
    fn is_tight(&self) -> bool {
        false
    }

    fn is_normalized(&self) -> bool {
        self.scroll_offset >= 0.0
            && self.cross_axis_extent >= 0.0
            && axis_direction_to_axis(self.axis_direction)
                != axis_direction_to_axis(self.cross_axis_direction)
            && self.viewport_main_axis_extent >= 0.0
            && self.remaining_paint_extent >= 0.0
    }
}

impl PartialEq for SliverConstraints {
    fn eq(&self, other: &SliverConstraints) -> bool {
        debug_assert!(self.debug_assert_is_valid(false));
        debug_assert!(other.debug_assert_is_valid(false));
        self.axis_direction == other.axis_direction
            && self.growth_direction == other.growth_direction
            && self.user_scroll_direction == other.user_scroll_direction
            && self.scroll_offset == other.scroll_offset
            && self.preceding_scroll_extent == other.preceding_scroll_extent
            && self.overlap == other.overlap
            && self.remaining_paint_extent == other.remaining_paint_extent
            && self.cross_axis_extent == other.cross_axis_extent
            && self.cross_axis_direction == other.cross_axis_direction
            && self.viewport_main_axis_extent == other.viewport_main_axis_extent
            && self.remaining_cache_extent == other.remaining_cache_extent
            && self.cache_origin == other.cache_origin
    }
}

/// The amount of space a sliver occupies.
#[derive(Clone, Copy, Debug)]
pub struct SliverGeometry {
    /// Estimated total scrollable extent this sliver has content for.
    pub scroll_extent: f64,
    /// Visual location of the first visible part relative to the layout
    /// position.
    pub paint_origin: f64,
    /// Currently visible visual space taken by this sliver.
    pub paint_extent: f64,
    /// Distance from the first visible part of this sliver to the first visible
    /// part of the next.
    pub layout_extent: f64,
    /// Estimated total paint extent if remaining paint extent were infinite.
    pub max_paint_extent: f64,
    /// Maximum extent by which a pinned sliver can reduce the scrollable area.
    pub max_scroll_obstruction_extent: f64,
    /// Distance from where this sliver started painting to the bottom of where
    /// it should accept hits.
    pub hit_test_extent: f64,
    /// Whether this sliver should be painted.
    pub visible: bool,
    /// Whether this sliver has visual overflow.
    pub has_visual_overflow: bool,
    /// Non-zero asks the parent to correct the scroll offset and relayout.
    pub scroll_offset_correction: Option<f64>,
    /// Pixels consumed in the remaining cache extent.
    pub cache_extent: f64,
    /// Cross-axis extent, if the sliver reports one.
    pub cross_axis_extent: Option<f64>,
}

impl SliverGeometry {
    /// A sliver that occupies no space at all.
    pub const ZERO: SliverGeometry = SliverGeometry {
        scroll_extent: 0.0,
        paint_origin: 0.0,
        paint_extent: 0.0,
        layout_extent: 0.0,
        max_paint_extent: 0.0,
        max_scroll_obstruction_extent: 0.0,
        hit_test_extent: 0.0,
        visible: false,
        has_visual_overflow: false,
        scroll_offset_correction: None,
        cache_extent: 0.0,
        cross_axis_extent: None,
    };

    /// Creates geometry with Flutter's constructor defaults.
    pub const fn new() -> SliverGeometry {
        SliverGeometry::ZERO
    }

    /// Sets [`scroll_extent`](Self::scroll_extent).
    pub const fn scroll_extent(mut self, value: f64) -> SliverGeometry {
        self.scroll_extent = value;
        self
    }

    /// Sets [`paint_origin`](Self::paint_origin).
    pub const fn paint_origin(mut self, value: f64) -> SliverGeometry {
        self.paint_origin = value;
        self
    }

    /// Sets [`paint_extent`](Self::paint_extent). Also fills layout, hit-test,
    /// cache, and visible the way Flutter's constructor does when those
    /// arguments are omitted.
    pub const fn paint_extent(mut self, value: f64) -> SliverGeometry {
        self.paint_extent = value;
        self.layout_extent = value;
        self.hit_test_extent = value;
        self.cache_extent = value;
        self.visible = value > 0.0;
        self
    }

    /// Sets [`layout_extent`](Self::layout_extent).
    pub const fn layout_extent(mut self, value: f64) -> SliverGeometry {
        self.layout_extent = value;
        self
    }

    /// Sets [`max_paint_extent`](Self::max_paint_extent).
    pub const fn max_paint_extent(mut self, value: f64) -> SliverGeometry {
        self.max_paint_extent = value;
        self
    }

    /// Sets [`max_scroll_obstruction_extent`](Self::max_scroll_obstruction_extent).
    pub const fn max_scroll_obstruction_extent(mut self, value: f64) -> SliverGeometry {
        self.max_scroll_obstruction_extent = value;
        self
    }

    /// Sets [`hit_test_extent`](Self::hit_test_extent).
    pub const fn hit_test_extent(mut self, value: f64) -> SliverGeometry {
        self.hit_test_extent = value;
        self
    }

    /// Sets [`visible`](Self::visible).
    pub const fn visible(mut self, value: bool) -> SliverGeometry {
        self.visible = value;
        self
    }

    /// Sets [`has_visual_overflow`](Self::has_visual_overflow).
    pub const fn has_visual_overflow(mut self, value: bool) -> SliverGeometry {
        self.has_visual_overflow = value;
        self
    }

    /// Sets [`scroll_offset_correction`](Self::scroll_offset_correction).
    pub fn scroll_offset_correction(mut self, value: f64) -> SliverGeometry {
        debug_assert!(value != 0.0);
        self.scroll_offset_correction = Some(value);
        self
    }

    /// Sets [`cache_extent`](Self::cache_extent).
    pub const fn cache_extent(mut self, value: f64) -> SliverGeometry {
        self.cache_extent = value;
        self
    }

    /// Sets [`cross_axis_extent`](Self::cross_axis_extent).
    pub const fn cross_axis_extent(mut self, value: f64) -> SliverGeometry {
        self.cross_axis_extent = Some(value);
        self
    }

    /// Creates a copy of this object. Chain field replacements.
    pub const fn copy_with(self) -> SliverGeometry {
        self
    }

    /// Dart's `geometry == SliverGeometry.zero`: whether this geometry occupies no space at
    /// all.
    pub fn is_zero(&self) -> bool {
        self.scroll_extent == SliverGeometry::ZERO.scroll_extent
            && self.paint_origin == SliverGeometry::ZERO.paint_origin
            && self.paint_extent == SliverGeometry::ZERO.paint_extent
            && self.layout_extent == SliverGeometry::ZERO.layout_extent
            && self.max_paint_extent == SliverGeometry::ZERO.max_paint_extent
            && self.max_scroll_obstruction_extent
                == SliverGeometry::ZERO.max_scroll_obstruction_extent
            && self.hit_test_extent == SliverGeometry::ZERO.hit_test_extent
            && self.visible == SliverGeometry::ZERO.visible
            && self.has_visual_overflow == SliverGeometry::ZERO.has_visual_overflow
            && self.scroll_offset_correction == SliverGeometry::ZERO.scroll_offset_correction
            && self.cache_extent == SliverGeometry::ZERO.cache_extent
            && self.cross_axis_extent == SliverGeometry::ZERO.cross_axis_extent
    }

    /// Asserts that this geometry is internally consistent.
    ///
    /// Does nothing if asserts are disabled. Always returns true.
    pub fn debug_assert_is_valid(&self) -> bool {
        debug_assert!(self.scroll_extent >= 0.0, "The scroll_extent is negative.");
        debug_assert!(self.paint_extent >= 0.0, "The paint_extent is negative.");
        debug_assert!(self.layout_extent >= 0.0, "The layout_extent is negative.");
        debug_assert!(self.cache_extent >= 0.0, "The cache_extent is negative.");
        debug_assert!(
            self.layout_extent <= self.paint_extent,
            "The layout_extent ({}) exceeds the paint_extent ({}).",
            self.layout_extent,
            self.paint_extent
        );
        // If the paint_extent is slightly more than the max_paint_extent, but the difference is
        // still less than PRECISION_ERROR_TOLERANCE, we will not fail the assert below.
        debug_assert!(
            self.paint_extent - self.max_paint_extent <= PRECISION_ERROR_TOLERANCE,
            "The max_paint_extent ({}) is less than the paint_extent ({}). By definition, a \
             sliver can't paint more than the maximum that it can paint!",
            self.max_paint_extent,
            self.paint_extent
        );
        debug_assert!(
            self.hit_test_extent >= 0.0,
            "The hit_test_extent is negative."
        );
        debug_assert!(
            self.scroll_offset_correction != Some(0.0),
            "The scroll_offset_correction is zero."
        );
        true
    }
}

impl Default for SliverGeometry {
    fn default() -> SliverGeometry {
        SliverGeometry::new()
    }
}

impl Display for SliverGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SliverGeometry({}x{})",
            self.scroll_extent, self.paint_extent
        )
    }
}

/// Method signature for hit testing a [`RenderSliver`].
///
/// Used by [`SliverHitTestResult::add_with_axis_offset`] to hit test [`RenderSliver`] children.
///
/// See also:
///
///  * [`RenderSliver::hit_test`], which documents more details around hit testing slivers.
pub type SliverHitTest<'r> = dyn FnOnce(&mut SliverHitTestResult<'r>, f64, f64) -> bool;

/// The result of performing a hit test on [`RenderSliver`]s.
///
/// A view over a [`HitTestResult`]: Dart's `SliverHitTestResult.wrap`. Dart's bare
/// `SliverHitTestResult()` constructor is `wrap` over a fresh [`HitTestResult`].
pub struct SliverHitTestResult<'a>(&'a mut HitTestResult);

impl<'a> SliverHitTestResult<'a> {
    /// Wraps `result` to create a [`SliverHitTestResult`] that shares its path.
    ///
    /// This is used by render objects that adapt between the sliver world and the non-sliver
    /// world.
    pub fn wrap(result: &'a mut HitTestResult) -> SliverHitTestResult<'a> {
        SliverHitTestResult(result)
    }

    /// Transforms `main_axis_position` and `cross_axis_position` to the local coordinate system
    /// of a child for hit-testing the child.
    ///
    /// The actual hit testing of the child needs to be implemented in the provided `hit_test`
    /// callback, which is invoked with the transformed positions as arguments.
    ///
    /// For the transform `main_axis_offset` is subtracted from `main_axis_position` and
    /// `cross_axis_offset` is subtracted from `cross_axis_position`.
    ///
    /// The `paint_offset` describes how the paint position of a point painted at the provided
    /// `main_axis_position` and `cross_axis_position` would change after `main_axis_offset` and
    /// `cross_axis_offset` have been applied. It is used to properly convert pointer events to
    /// the local coordinate system of the event receiver.
    ///
    /// The `paint_offset` may be `None` if `main_axis_offset` and `cross_axis_offset` are both
    /// zero.
    ///
    /// The function returns the return value of `hit_test`.
    pub fn add_with_axis_offset(
        &mut self,
        paint_offset: Option<Offset>,
        main_axis_offset: f64,
        cross_axis_offset: f64,
        main_axis_position: f64,
        cross_axis_position: f64,
        hit_test: impl FnOnce(&mut SliverHitTestResult<'_>, f64, f64) -> bool,
    ) -> bool {
        if let Some(paint_offset) = paint_offset {
            self.0.push_offset(Offset::ZERO - paint_offset);
        }
        let is_hit = hit_test(
            self,
            main_axis_position - main_axis_offset,
            cross_axis_position - cross_axis_offset,
        );
        if paint_offset.is_some() {
            self.0.pop_transform();
        }
        is_hit
    }
}

impl Deref for SliverHitTestResult<'_> {
    type Target = HitTestResult;

    fn deref(&self) -> &HitTestResult {
        self.0
    }
}

impl DerefMut for SliverHitTestResult<'_> {
    fn deref_mut(&mut self) -> &mut HitTestResult {
        self.0
    }
}

/// A hit test entry used by [`RenderSliver`].
///
/// The coordinate system used by this hit test entry is relative to the [`AxisDirection`] of
/// the target sliver.
///
/// Dart's `SliverHitTestEntry` subclasses `HitTestEntry`; here it is the entry's target,
/// carrying the sliver and the two positions. It becomes a [`HitTestEntry`] with [`From`].
#[derive(Clone, Copy, Debug)]
pub struct SliverHitTestEntry {
    target: AnyRenderSliver,
    main_axis_position: f64,
    cross_axis_position: f64,
}

impl SliverHitTestEntry {
    /// Creates a sliver hit test entry.
    pub fn new(
        target: AnyRenderSliver,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> SliverHitTestEntry {
        SliverHitTestEntry {
            target,
            main_axis_position,
            cross_axis_position,
        }
    }

    /// The [`RenderSliver`] that was hit.
    pub fn target(&self) -> AnyRenderSliver {
        self.target
    }

    /// The distance in the [`AxisDirection`] from the edge of the sliver's painted area (as
    /// given by the [`SliverConstraints::scroll_offset`]) to the hit point.
    ///
    /// This can be an unusual direction, for example in the [`AxisDirection::Up`] case this is
    /// a distance from the _bottom_ of the sliver's painted area.
    pub fn main_axis_position(&self) -> f64 {
        self.main_axis_position
    }

    /// The distance to the hit point in the axis opposite the [`SliverConstraints::axis`].
    ///
    /// If the cross axis is horizontal (i.e. the [`SliverConstraints::axis_direction`] is
    /// either [`AxisDirection::Down`] or [`AxisDirection::Up`]), then this is a distance from
    /// the left edge of the sliver. If the cross axis is vertical, then this is a distance
    /// from the top edge of the sliver.
    ///
    /// This is always a distance from the left or top of the parent, never a distance from the
    /// right or bottom.
    pub fn cross_axis_position(&self) -> f64 {
        self.cross_axis_position
    }
}

impl HitTestTarget for SliverHitTestEntry {
    fn retained_handle(&self) -> Option<HandleId> {
        Some(self.target.id)
    }

    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        self.target.handle_event(app, event, self);
    }
}

impl From<SliverHitTestEntry> for HitTestEntry {
    fn from(entry: SliverHitTestEntry) -> HitTestEntry {
        HitTestEntry::new(entry)
    }
}

/// Parent data structure used by parents of slivers that position their children using layout
/// offsets.
///
/// This data structure is optimized for fast layout. It is best used by parents that expect to
/// have many children whose relative positions don't change even when the scroll offset does.
#[derive(Clone, Copy, Debug, Default)]
pub struct SliverLogicalParentData {
    /// The position of the child relative to the zero scroll offset.
    ///
    /// The number of pixels from the zero scroll offset of the parent sliver (the line at
    /// which its [`SliverConstraints::scroll_offset`] is zero) to the side of the child closest
    /// to that offset. It can be `None` when it cannot be determined. The value will be set
    /// after layout.
    ///
    /// In a typical list, this does not change as the parent is scrolled.
    pub layout_offset: Option<f64>,
}

impl SliverLogicalParentData {
    /// Creates parent data with no layout offset.
    pub const fn new() -> SliverLogicalParentData {
        SliverLogicalParentData {
            layout_offset: None,
        }
    }
}

impl ParentData for SliverLogicalParentData {
    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        (id == TypeId::of::<SliverLogicalParentData>()).then_some(self)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        (id == TypeId::of::<SliverLogicalParentData>()).then_some(self)
    }
}

impl Display for SliverLogicalParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.layout_offset {
            None => f.write_str("layoutOffset=None"),
            Some(offset) => write!(f, "layoutOffset={offset:.1}"),
        }
    }
}

/// Parent data for slivers that have multiple children and that position their children using
/// layout offsets.
#[derive(Debug)]
pub struct SliverLogicalContainerParentData {
    sliver_logical: SliverLogicalParentData,
    container: ContainerParentData<AnyRenderSliver>,
}

impl SliverLogicalContainerParentData {
    /// Creates parent data with no layout offset and no siblings.
    pub const fn new() -> SliverLogicalContainerParentData {
        SliverLogicalContainerParentData {
            sliver_logical: SliverLogicalParentData::new(),
            container: ContainerParentData::new(),
        }
    }

    /// The [`SliverLogicalParentData`] half.
    pub fn sliver_logical_parent_data(&self) -> &SliverLogicalParentData {
        &self.sliver_logical
    }

    /// See [`sliver_logical_parent_data`](Self::sliver_logical_parent_data).
    pub fn sliver_logical_parent_data_mut(&mut self) -> &mut SliverLogicalParentData {
        &mut self.sliver_logical
    }

    /// See [`SliverLogicalParentData::layout_offset`].
    pub fn layout_offset(&self) -> Option<f64> {
        self.sliver_logical.layout_offset
    }

    /// Sets [`layout_offset`](Self::layout_offset).
    pub fn set_layout_offset(&mut self, value: Option<f64>) {
        self.sliver_logical.layout_offset = value;
    }
}

impl Default for SliverLogicalContainerParentData {
    fn default() -> SliverLogicalContainerParentData {
        SliverLogicalContainerParentData::new()
    }
}

impl ParentData for SliverLogicalContainerParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SliverLogicalContainerParentData>() {
            return Some(self);
        }
        self.sliver_logical.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SliverLogicalContainerParentData>() {
            return Some(self);
        }
        self.sliver_logical.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SliverLogicalContainerParentData {
    type ChildType = AnyRenderSliver;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderSliver> {
        &self.container
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderSliver> {
        &mut self.container
    }
}

impl Display for SliverLogicalContainerParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.sliver_logical, f)
    }
}

/// Parent data structure used by parents of slivers that position their children using absolute
/// coordinates.
///
/// For example, used by [`crate::RenderViewport`].
///
/// This data structure is optimized for fast painting, at the cost of requiring additional work
/// during layout when the children change their offsets. It is best used by parents that expect
/// to have few children, especially if those children will themselves be very tall relative to
/// the parent.
#[derive(Clone, Copy, Debug)]
pub struct SliverPhysicalParentData {
    /// The position of the child relative to the parent.
    ///
    /// This is the distance from the top left visible corner of the parent to the top left
    /// visible corner of the sliver.
    pub paint_offset: Offset,

    /// The cross axis flex factor to use for this sliver child.
    ///
    /// If used outside of a `SliverCrossAxisGroup` widget, this value has no meaning.
    ///
    /// If `None` or zero, the child is inflexible and determines its own size in the cross
    /// axis. If non-zero, the amount of space the child can occupy in the cross axis is
    /// determined by dividing the free space (after placing the inflexible children) according
    /// to the flex factors of the flexible children.
    pub cross_axis_flex: Option<i32>,
}

impl SliverPhysicalParentData {
    /// Creates parent data with a zero paint offset.
    pub const fn new() -> SliverPhysicalParentData {
        SliverPhysicalParentData {
            paint_offset: Offset::ZERO,
            cross_axis_flex: None,
        }
    }

    /// Apply the [`paint_offset`](Self::paint_offset) to the given `transform`.
    ///
    /// Used to implement [`RenderObject::apply_paint_transform`] by slivers that use
    /// [`SliverPhysicalParentData`].
    pub fn apply_paint_transform(&self, transform: &mut Matrix4) {
        // Hit test logic relies on this always providing an invertible matrix.
        translate(transform, self.paint_offset.dx(), self.paint_offset.dy());
    }
}

impl Default for SliverPhysicalParentData {
    fn default() -> SliverPhysicalParentData {
        SliverPhysicalParentData::new()
    }
}

impl ParentData for SliverPhysicalParentData {
    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        (id == TypeId::of::<SliverPhysicalParentData>()).then_some(self)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        (id == TypeId::of::<SliverPhysicalParentData>()).then_some(self)
    }
}

impl Display for SliverPhysicalParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "paintOffset={:?}", self.paint_offset)
    }
}

/// Parent data for slivers that have multiple children and that position their children using
/// absolute coordinates.
#[derive(Debug)]
pub struct SliverPhysicalContainerParentData {
    sliver_physical: SliverPhysicalParentData,
    container: ContainerParentData<AnyRenderSliver>,
}

impl SliverPhysicalContainerParentData {
    /// Creates parent data with a zero paint offset and no siblings.
    pub const fn new() -> SliverPhysicalContainerParentData {
        SliverPhysicalContainerParentData {
            sliver_physical: SliverPhysicalParentData::new(),
            container: ContainerParentData::new(),
        }
    }

    /// The [`SliverPhysicalParentData`] half.
    pub fn sliver_physical_parent_data(&self) -> &SliverPhysicalParentData {
        &self.sliver_physical
    }

    /// See [`sliver_physical_parent_data`](Self::sliver_physical_parent_data).
    pub fn sliver_physical_parent_data_mut(&mut self) -> &mut SliverPhysicalParentData {
        &mut self.sliver_physical
    }

    /// See [`SliverPhysicalParentData::paint_offset`].
    pub fn paint_offset(&self) -> Offset {
        self.sliver_physical.paint_offset
    }

    /// Sets [`paint_offset`](Self::paint_offset).
    pub fn set_paint_offset(&mut self, value: Offset) {
        self.sliver_physical.paint_offset = value;
    }

    /// See [`SliverPhysicalParentData::apply_paint_transform`].
    pub fn apply_paint_transform(&self, transform: &mut Matrix4) {
        self.sliver_physical.apply_paint_transform(transform);
    }
}

impl Default for SliverPhysicalContainerParentData {
    fn default() -> SliverPhysicalContainerParentData {
        SliverPhysicalContainerParentData::new()
    }
}

impl ParentData for SliverPhysicalContainerParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SliverPhysicalContainerParentData>() {
            return Some(self);
        }
        self.sliver_physical.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SliverPhysicalContainerParentData>() {
            return Some(self);
        }
        self.sliver_physical.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SliverPhysicalContainerParentData {
    type ChildType = AnyRenderSliver;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderSliver> {
        &self.container
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderSliver> {
        &mut self.container
    }
}

impl Display for SliverPhysicalContainerParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.sliver_physical, f)
    }
}

/// The body of [`RenderSliver::calculate_paint_offset`], reachable without a sliver.
pub(crate) fn calculate_paint_offset(constraints: SliverConstraints, from: f64, to: f64) -> f64 {
    debug_assert!(from <= to);
    let a = constraints.scroll_offset;
    let b = constraints.scroll_offset + constraints.remaining_paint_extent;
    // the clamp on the next line is to avoid floating point rounding errors
    clamp_double(
        clamp_double(to, a, b) - clamp_double(from, a, b),
        0.0,
        constraints.remaining_paint_extent,
    )
}

/// The body of [`RenderSliver::calculate_cache_offset`], reachable without a sliver.
pub(crate) fn calculate_cache_offset(constraints: SliverConstraints, from: f64, to: f64) -> f64 {
    debug_assert!(from <= to);
    let a = constraints.scroll_offset + constraints.cache_origin;
    let b = constraints.scroll_offset + constraints.remaining_cache_extent;
    // the clamp on the next line is to avoid floating point rounding errors
    clamp_double(
        clamp_double(to, a, b) - clamp_double(from, a, b),
        0.0,
        constraints.remaining_cache_extent,
    )
}

/// Flutter's `RenderSliver` fields.
pub struct RenderSliverData {
    pub(crate) geometry: Option<SliverGeometry>,
    pub(crate) constraints: Option<SliverConstraints>,
}

impl RenderSliverData {
    /// Unlaid-out sliver state.
    pub fn new() -> RenderSliverData {
        RenderSliverData {
            geometry: None,
            constraints: None,
        }
    }
}

impl Default for RenderSliverData {
    fn default() -> RenderSliverData {
        RenderSliverData::new()
    }
}

/// Implements [`RenderSliver`] field accessors for a `render_sliver` field.
#[macro_export]
macro_rules! render_sliver_accessors {
    () => {
        fn render_sliver_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderSliverData {
            &self.get(app).render_sliver
        }
        fn render_sliver_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderSliverData {
            &mut self.get_mut(app).render_sliver
        }
    };
}

/// A render object using the sliver protocol.
///
/// Flutter's counterpart is `RenderSliver`. [`RenderObject::perform_layout`] is the leaf
/// override; [`AnyRenderSliver::layout`] is the framework wrapper. `RenderObject`'s tree
/// operations are provided here and forward to [`as_object`](Self::as_object).
pub trait RenderSliver: RenderObject {
    /// Mixin field access.
    fn render_sliver_data(self: RenderHandle<Self>, app: &App) -> &RenderSliverData;

    /// See [`render_sliver_data`](Self::render_sliver_data).
    fn render_sliver_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderSliverData;

    /// The sliver constraints most recently supplied by the parent.
    fn constraints(self: RenderHandle<Self>, app: &App) -> SliverConstraints {
        self.render_sliver_data(app).constraints.unwrap_or_else(|| {
            panic!("A RenderObject does not have any constraints before it has been laid out.")
        })
    }

    /// The amount of space this sliver occupies.
    fn geometry(self: RenderHandle<Self>, app: &App) -> SliverGeometry {
        self.as_sliver().geometry(app)
    }

    /// Sets the geometry of this sliver. Call from [`RenderObject::perform_layout`].
    fn set_geometry(self: RenderHandle<Self>, app: &mut App, geometry: SliverGeometry) {
        self.as_sliver().set_geometry(app, geometry)
    }

    /// An estimate of the bounds within which this render object will paint: the paint extent
    /// along the main axis by the cross axis extent.
    fn paint_bounds(self: RenderHandle<Self>, app: &App) -> Rect {
        let constraints = self.constraints(app);
        let geometry = self.geometry(app);
        match constraints.axis() {
            Axis::Horizontal => Rect::from_ltwh(
                0.0,
                0.0,
                geometry.paint_extent,
                constraints.cross_axis_extent,
            ),
            Axis::Vertical => Rect::from_ltwh(
                0.0,
                0.0,
                constraints.cross_axis_extent,
                geometry.paint_extent,
            ),
        }
    }

    /// For a center sliver, the distance before the absolute zero scroll offset that this
    /// sliver can cover.
    ///
    /// For example, if an [`AxisDirection::Down`] viewport with a
    /// [`crate::RenderViewport::anchor`] of 0.5 has a single sliver with a height of 100.0 and
    /// its `center_offset_adjustment` returns 50.0, then the sliver will be centered in the
    /// viewport when the scroll offset is 0.0.
    ///
    /// The distance here is in the opposite direction of the viewport's axis direction, so
    /// values will typically be positive.
    fn center_offset_adjustment(self: RenderHandle<Self>, _app: &App) -> f64 {
        let _ = self;
        0.0
    }

    /// Determines the set of render objects located at the given position.
    ///
    /// Returns true if the given point is contained in this render object or one of its
    /// descendants. Adds any render objects that contain the point to the given hit test
    /// result.
    ///
    /// The caller is responsible for providing the position in the local coordinate space of
    /// the callee. The callee is responsible for checking whether the given position is within
    /// its bounds.
    ///
    /// # Coordinates for `RenderSliver` objects
    ///
    /// The `main_axis_position` is the distance in the [`AxisDirection`] (after applying the
    /// [`GrowthDirection`]) from the edge of the sliver's painted area. This can be an unusual
    /// direction, for example in the [`AxisDirection::Up`] case this is a distance from the
    /// _bottom_ of the sliver's painted area.
    ///
    /// The `cross_axis_position` is the distance in the other axis. If the cross axis is
    /// horizontal (i.e. the [`SliverConstraints::axis_direction`] is either
    /// [`AxisDirection::Down`] or [`AxisDirection::Up`]), then the `cross_axis_position` is a
    /// distance from the left edge of the sliver. Otherwise it is a distance from the top edge.
    ///
    /// # Implementing hit testing for slivers
    ///
    /// The most straight-forward way to implement hit testing for a new sliver render object is
    /// to override its [`hit_test_self`](Self::hit_test_self) and
    /// [`hit_test_children`](Self::hit_test_children) methods.
    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        if main_axis_position >= 0.0
            && main_axis_position < self.geometry(app).hit_test_extent
            && cross_axis_position >= 0.0
            && cross_axis_position < self.constraints(app).cross_axis_extent
            && (self.hit_test_children(app, result, main_axis_position, cross_axis_position)
                || self.hit_test_self(app, main_axis_position, cross_axis_position))
        {
            result.add(
                SliverHitTestEntry::new(self.as_sliver(), main_axis_position, cross_axis_position)
                    .into(),
            );
            return true;
        }
        false
    }

    /// Override this method if this render object can be hit even if its children were not hit.
    ///
    /// Used by [`hit_test`](Self::hit_test). If you override `hit_test` and do not call this
    /// function, then you don't need to implement this function.
    fn hit_test_self(
        self: RenderHandle<Self>,
        _app: &App,
        _main_axis_position: f64,
        _cross_axis_position: f64,
    ) -> bool {
        let _ = self;
        false
    }

    /// Override this method to check whether any children are located at the given position.
    ///
    /// Typically children should be hit-tested in reverse paint order so that hit tests at
    /// locations where children overlap hit the child that is visually "on top" (i.e., paints
    /// later).
    ///
    /// Used by [`hit_test`](Self::hit_test). If you override `hit_test` and do not call this
    /// function, then you don't need to implement this function.
    fn hit_test_children(
        self: RenderHandle<Self>,
        _app: &mut App,
        _result: &mut SliverHitTestResult<'_>,
        _main_axis_position: f64,
        _cross_axis_position: f64,
    ) -> bool {
        let _ = self;
        false
    }

    /// Override to handle pointer events that hit this render object.
    fn handle_event(
        self: RenderHandle<Self>,
        _app: &mut App,
        _event: &PointerEvent,
        _entry: &SliverHitTestEntry,
    ) {
        let _ = self;
    }

    /// Computes the portion of the region from `from` to `to` that is visible, assuming that
    /// only the region from the [`SliverConstraints::scroll_offset`] that is
    /// [`SliverConstraints::remaining_paint_extent`] high is visible, and that the relationship
    /// between scroll offsets and paint offsets is linear.
    ///
    /// For example, if the constraints have a scroll offset of 100 and a remaining paint extent
    /// of 100, and the arguments to this method describe the region 50..150, then the returned
    /// value would be 50 (from scroll offset 100 to scroll offset 150).
    ///
    /// This method is not useful if there is not a 1:1 relationship between consumed scroll
    /// offset and consumed paint extent.
    fn calculate_paint_offset(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: SliverConstraints,
        from: f64,
        to: f64,
    ) -> f64 {
        let _ = self;
        calculate_paint_offset(constraints, from, to)
    }

    /// Computes the portion of the region from `from` to `to` that is within the cache extent
    /// of the viewport, assuming that only the region from the
    /// [`SliverConstraints::cache_origin`] that is
    /// [`SliverConstraints::remaining_cache_extent`] high is visible, and that the relationship
    /// between scroll offsets and paint offsets is linear.
    ///
    /// This method is not useful if there is not a 1:1 relationship between consumed scroll
    /// offset and consumed cache extent.
    fn calculate_cache_offset(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: SliverConstraints,
        from: f64,
        to: f64,
    ) -> f64 {
        let _ = self;
        calculate_cache_offset(constraints, from, to)
    }

    /// Returns the distance from the leading _visible_ edge of the sliver to the side of the
    /// given child closest to that edge.
    ///
    /// For example, if the [`constraints`](Self::constraints) describe this sliver as having an
    /// axis direction of [`AxisDirection::Down`], then this is the distance from the top of the
    /// visible portion of the sliver to the top of the child. On the other hand, if the axis
    /// direction is [`AxisDirection::Up`], then this is the distance from the bottom of the
    /// visible portion of the sliver to the bottom of the child. In both cases, this is the
    /// direction of increasing [`SliverConstraints::scroll_offset`] and
    /// [`SliverLogicalParentData::layout_offset`].
    ///
    /// For children that are slivers, the leading edge of the _child_ will be the leading
    /// _visible_ edge of the child, not the part of the child that would locally be at scroll
    /// offset 0.0. For box children it is the actual distance to the edge of the box, since
    /// those boxes do not know how to handle being scrolled.
    ///
    /// This method differs from [`child_scroll_offset`](Self::child_scroll_offset) in that it
    /// gives the distance from the leading _visible_ edge of the sliver whereas
    /// `child_scroll_offset` gives the distance from the sliver's zero scroll offset.
    ///
    /// Calling this for a child that is not visible is not valid.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        _app: &App,
        _child: AnyRenderObject,
    ) -> f64 {
        let _ = self;
        panic!("this RenderSliver does not implement child_main_axis_position")
    }

    /// Returns the distance along the cross axis from the zero of the cross axis in this
    /// sliver's paint coordinate space to the nearest side of the given child.
    ///
    /// For example, if the [`constraints`](Self::constraints) describe this sliver as having an
    /// axis direction of [`AxisDirection::Down`] or [`AxisDirection::Up`], then this is the
    /// distance from the left of the sliver to the left of the child. If the axis direction is
    /// [`AxisDirection::Left`] or [`AxisDirection::Right`], then it is the distance from the
    /// top of the sliver to the top of the child.
    ///
    /// Calling this for a child that is not visible is not valid.
    fn child_cross_axis_position(
        self: RenderHandle<Self>,
        _app: &App,
        _child: AnyRenderObject,
    ) -> f64 {
        let _ = self;
        0.0
    }

    /// Returns the scroll offset for the leading edge of the given child.
    ///
    /// The `child` must be a child of this sliver.
    ///
    /// If there are pinned slivers before `child`, the offset should be reduced by the extent
    /// of the pinned children.
    ///
    /// This method differs from [`child_main_axis_position`](Self::child_main_axis_position) in
    /// that `child_main_axis_position` gives the distance from the leading _visible_ edge of
    /// the sliver whereas this gives the distance from the sliver's zero scroll offset.
    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        debug_assert_eq!(child.parent(app), Some(self.as_object()));
        Some(0.0)
    }

    /// This returns a [`Size`] with dimensions relative to the leading edge of the sliver,
    /// specifically the same offset that is given to the [`RenderObject::paint`] method. This
    /// means that the dimensions may be negative.
    ///
    /// This is only valid after layout has completed.
    ///
    /// See also:
    ///
    ///  * [`get_absolute_size`](Self::get_absolute_size), which returns absolute size.
    fn get_absolute_size_relative_to_origin(self: RenderHandle<Self>, app: &App) -> Size {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let constraints = self.constraints(app);
        let paint_extent = self.geometry(app).paint_extent;
        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => Size::new(constraints.cross_axis_extent, -paint_extent),
            AxisDirection::Down => Size::new(constraints.cross_axis_extent, paint_extent),
            AxisDirection::Left => Size::new(-paint_extent, constraints.cross_axis_extent),
            AxisDirection::Right => Size::new(paint_extent, constraints.cross_axis_extent),
        }
    }

    /// This returns the absolute [`Size`] of the sliver.
    ///
    /// The dimensions are always positive and calling this is only valid after layout has
    /// completed.
    ///
    /// See also:
    ///
    ///  * [`get_absolute_size_relative_to_origin`](Self::get_absolute_size_relative_to_origin),
    ///    which returns the size relative to the leading edge of the sliver.
    fn get_absolute_size(self: RenderHandle<Self>, app: &App) -> Size {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let constraints = self.constraints(app);
        let paint_extent = self.geometry(app).paint_extent;
        match constraints.axis_direction {
            AxisDirection::Up | AxisDirection::Down => {
                Size::new(constraints.cross_axis_extent, paint_extent)
            }
            AxisDirection::Right | AxisDirection::Left => {
                Size::new(paint_extent, constraints.cross_axis_extent)
            }
        }
    }

    /// Returns the [`Rect`] that covers the total paint extent of the sliver.
    ///
    /// The rect is expressed in the sliver's local coordinate system, which is axis-aligned
    /// with the painting context's canvas. The coordinate system's origin (0,0) corresponds to
    /// the `offset` argument passed to [`RenderObject::paint`].
    fn get_max_paint_rect(self: RenderHandle<Self>, app: &App) -> Rect {
        let Some(geometry) = self.render_sliver_data(app).geometry else {
            return Rect::ZERO;
        };
        if geometry.is_zero() {
            return Rect::ZERO;
        }
        let constraints = self.constraints(app);
        let mut max_paint_extent = geometry.max_paint_extent;
        if max_paint_extent.is_infinite() {
            max_paint_extent =
                constraints.scroll_offset + geometry.cache_extent + constraints.cache_origin;
        }
        let paint_extent = geometry.paint_extent;
        // To ensure the computed rect remains visible when pinned, the leading offset is capped
        // at the sliver's `scroll_extent - max_scroll_obstruction_extent`.
        let leading_offset = clamp_double(
            constraints.scroll_offset,
            0.0,
            geometry.scroll_extent - geometry.max_scroll_obstruction_extent,
        );
        let cross_axis_extent = geometry
            .cross_axis_extent
            .unwrap_or(constraints.cross_axis_extent);

        let rect = match constraints.axis() {
            Axis::Horizontal => {
                Rect::from_ltwh(-leading_offset, 0.0, max_paint_extent, cross_axis_extent)
            }
            Axis::Vertical => {
                Rect::from_ltwh(0.0, -leading_offset, cross_axis_extent, max_paint_extent)
            }
        };

        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Right | AxisDirection::Down => rect,
            AxisDirection::Left => Rect::from_ltrb(
                paint_extent - rect.right,
                rect.top,
                paint_extent - rect.left,
                rect.bottom,
            ),
            AxisDirection::Up => Rect::from_ltrb(
                rect.left,
                paint_extent - rect.bottom,
                rect.right,
                paint_extent - rect.top,
            ),
        }
    }

    /// See [`AnyRenderSliver::layout`].
    fn layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: SliverConstraints,
        parent_uses_size: bool,
    ) {
        self.as_sliver().layout(app, constraints, parent_uses_size)
    }

    /// The type-erased `RenderSliver` handle. Free: the vtable is a `const`, and the id is copied.
    fn as_sliver(self: RenderHandle<Self>) -> AnyRenderSliver {
        AnyRenderSliver {
            id: self.id(),
            vtable: const { &RenderSliverVTable::of::<Self>() },
        }
    }

    /// The type-erased `RenderObject` handle, through [`as_sliver`](Self::as_sliver).
    fn as_object(self: RenderHandle<Self>) -> AnyRenderObject {
        self.as_sliver().as_object()
    }

    /// See [`AnyRenderObject::adopt_child`].
    fn adopt_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        self.as_object().adopt_child(app, child)
    }

    /// See [`AnyRenderObject::drop_child`].
    fn drop_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        self.as_object().drop_child(app, child)
    }

    /// See [`AnyRenderObject::mark_needs_layout`].
    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_layout(app)
    }

    /// See [`AnyRenderObject::mark_needs_paint`].
    fn mark_needs_paint(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_paint(app)
    }

    /// See [`AnyRenderObject::mark_needs_composited_layer_update`].
    fn mark_needs_composited_layer_update(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_composited_layer_update(app)
    }

    /// See [`AnyRenderObject::schedule_initial_layout`].
    fn schedule_initial_layout(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().schedule_initial_layout(app)
    }

    /// See [`AnyRenderObject::parent`].
    fn parent(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.as_object().parent(app)
    }

    /// See [`AnyRenderObject::owner`].
    fn owner(self: RenderHandle<Self>, app: &App) -> Option<Handle<PipelineOwner>> {
        self.as_object().owner(app)
    }

    /// See [`AnyRenderObject::attached`].
    fn attached(self: RenderHandle<Self>, app: &App) -> bool {
        self.as_object().attached(app)
    }

    /// See [`AnyRenderObject::debug_needs_layout`].
    fn debug_needs_layout(self: RenderHandle<Self>, app: &App) -> bool {
        self.as_object().debug_needs_layout(app)
    }
}

/// The vtable of an [`AnyRenderSliver`]: the object vtable plus the sliver accessors.
pub(crate) struct RenderSliverVTable {
    pub object: RenderObjectVTable,
    pub sliver_data: fn(&App, HandleId) -> &RenderSliverData,
    pub sliver_data_mut: fn(&mut App, HandleId) -> &mut RenderSliverData,
    pub hit_test: fn(&mut App, HandleId, &mut SliverHitTestResult<'_>, f64, f64) -> bool,
    pub handle_event: fn(&mut App, HandleId, &PointerEvent, &SliverHitTestEntry),
    pub center_offset_adjustment: fn(&App, HandleId) -> f64,
    pub child_scroll_offset: fn(&App, HandleId, AnyRenderObject) -> Option<f64>,
    pub child_main_axis_position: fn(&App, HandleId, AnyRenderObject) -> f64,
}

impl RenderSliverVTable {
    const fn of<T: RenderSliver>() -> RenderSliverVTable {
        RenderSliverVTable {
            object: RenderObjectVTable::of::<T>(
                |app, id, child| <T as RenderObject>::setup_parent_data(resolve(id), app, child),
                |app, id| T::paint_bounds(resolve(id), app),
                |app, id, child, transform| {
                    <T as RenderObject>::apply_paint_transform(resolve(id), app, child, transform)
                },
                |app, id| <T as RenderObject>::perform_resize(resolve(id), app),
                |app, id| crate::object::RenderObjectBase::mark_needs_layout(resolve::<T>(id), app),
                None,
                Some(|| const { &RenderSliverVTable::of::<T>() }),
            ),
            sliver_data: |app, id| T::render_sliver_data(resolve(id), app),
            sliver_data_mut: |app, id| T::render_sliver_data_mut(resolve(id), app),
            hit_test: |app, id, result, main, cross| {
                T::hit_test(resolve(id), app, result, main, cross)
            },
            handle_event: |app, id, event, entry| T::handle_event(resolve(id), app, event, entry),
            center_offset_adjustment: |app, id| T::center_offset_adjustment(resolve(id), app),
            child_scroll_offset: |app, id, child| T::child_scroll_offset(resolve(id), app, child),
            child_main_axis_position: |app, id, child| {
                T::child_main_axis_position(resolve(id), app, child)
            },
        }
    }
}

impl<T: RenderSliver> RenderHandle<T> {
    /// Creates a sliver-protocol render object in `app`.
    pub fn new_sliver(app: &mut App, object: T) -> RenderHandle<T> {
        let this = create(app, object);
        let data = this.render_object_data_mut(app);
        data.object_vtable = Some(&const { RenderSliverVTable::of::<T>() }.object);
        // Flutter's `RenderObject()` constructor:
        // `_needsCompositing = isRepaintBoundary || alwaysNeedsCompositing` and
        // `_wasRepaintBoundary = isRepaintBoundary`.
        let is_repaint_boundary = this.is_repaint_boundary(app);
        let always_needs_compositing = this.always_needs_compositing(app);
        let data = this.render_object_data_mut(app);
        data.was_repaint_boundary = is_repaint_boundary;
        data.needs_compositing = is_repaint_boundary || always_needs_compositing;
        this
    }
}

/// Erased `RenderSliver`.
#[derive(Clone, Copy)]
pub struct AnyRenderSliver {
    id: HandleId,
    vtable: &'static RenderSliverVTable,
}

impl PartialEq for AnyRenderSliver {
    fn eq(&self, other: &AnyRenderSliver) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRenderSliver {}

impl std::hash::Hash for AnyRenderSliver {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyRenderSliver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyRenderSliver({:?})", self.id)
    }
}

impl AnyRenderSliver {
    pub(crate) fn from_vtable(
        id: HandleId,
        vtable: &'static RenderSliverVTable,
    ) -> AnyRenderSliver {
        AnyRenderSliver { id, vtable }
    }

    /// The `RenderObject` view of this sliver. Free: points into the nested table.
    pub fn as_object(self) -> AnyRenderObject {
        AnyRenderObject::from_vtable(self.id, &self.vtable.object)
    }

    fn sliver_data(self, app: &App) -> &RenderSliverData {
        (self.vtable.sliver_data)(app, self.id)
    }

    fn sliver_data_mut(self, app: &mut App) -> &mut RenderSliverData {
        (self.vtable.sliver_data_mut)(app, self.id)
    }

    /// Compute the layout for this sliver.
    pub fn layout(self, app: &mut App, constraints: SliverConstraints, parent_uses_size: bool) {
        debug_assert!(constraints.debug_assert_is_valid(true));
        let same = self.sliver_data(app).constraints == Some(constraints);
        self.as_object()
            .run_layout(app, parent_uses_size, constraints.is_tight(), same, |app| {
                self.sliver_data_mut(app).constraints = Some(constraints)
            });
    }

    /// The sliver constraints most recently supplied by the parent.
    ///
    /// # Panics
    ///
    /// If this sliver has not been laid out.
    pub fn constraints(self, app: &App) -> SliverConstraints {
        self.sliver_data(app).constraints.unwrap_or_else(|| {
            panic!("A RenderObject does not have any constraints before it has been laid out.")
        })
    }

    /// See [`RenderSliver::hit_test`].
    pub fn hit_test(
        self,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        (self.vtable.hit_test)(
            app,
            self.id,
            result,
            main_axis_position,
            cross_axis_position,
        )
    }

    /// See [`RenderSliver::handle_event`].
    pub fn handle_event(self, app: &mut App, event: &PointerEvent, entry: &SliverHitTestEntry) {
        (self.vtable.handle_event)(app, self.id, event, entry)
    }

    /// See [`RenderSliver::center_offset_adjustment`].
    pub fn center_offset_adjustment(self, app: &App) -> f64 {
        (self.vtable.center_offset_adjustment)(app, self.id)
    }

    /// See [`RenderSliver::child_scroll_offset`].
    pub fn child_scroll_offset(self, app: &App, child: AnyRenderObject) -> Option<f64> {
        (self.vtable.child_scroll_offset)(app, self.id, child)
    }

    /// See [`RenderSliver::child_main_axis_position`].
    pub fn child_main_axis_position(self, app: &App, child: AnyRenderObject) -> f64 {
        (self.vtable.child_main_axis_position)(app, self.id, child)
    }

    /// The amount of space this sliver occupies.
    ///
    /// # Panics
    ///
    /// If this sliver has not been laid out.
    pub fn geometry(self, app: &App) -> SliverGeometry {
        self.sliver_data(app)
            .geometry
            .unwrap_or_else(|| panic!("RenderSliver was not laid out: {self:?}"))
    }

    /// Sets the geometry of this sliver.
    pub fn set_geometry(self, app: &mut App, geometry: SliverGeometry) {
        self.sliver_data_mut(app).geometry = Some(geometry);
    }
}

impl crate::object::ErasedRenderObject for AnyRenderSliver {
    fn as_object(self) -> AnyRenderObject {
        AnyRenderSliver::as_object(self)
    }

    fn from_object(object: AnyRenderObject) -> AnyRenderSliver {
        object
            .as_sliver()
            .expect("the render object is not a RenderSliver")
    }
}

impl From<AnyRenderSliver> for AnyRenderObject {
    fn from(sliver: AnyRenderSliver) -> AnyRenderObject {
        sliver.as_object()
    }
}

/// Whether the sliver's growth direction and axis direction agree, so that a box child is
/// positioned from the sliver's leading edge.
///
/// Flutter's private `RenderSliverHelpers._getRightWayUp`.
fn get_right_way_up(constraints: SliverConstraints) -> bool {
    let reversed = axis_direction_is_reversed(constraints.axis_direction);
    match constraints.growth_direction {
        GrowthDirection::Forward => !reversed,
        GrowthDirection::Reverse => reversed,
    }
}

/// Mixin for [`RenderSliver`] subclasses that provides some utility functions.
pub trait RenderSliverHelpers: RenderSliver {
    /// Utility function for [`RenderSliver::hit_test_children`] for use when the children are
    /// [`crate::RenderBox`] widgets.
    ///
    /// This function takes care of converting the position from the sliver coordinate system to
    /// the Cartesian coordinate system used by the box protocol.
    ///
    /// This function relies on [`RenderSliver::child_main_axis_position`] to determine the
    /// position of the child in question.
    ///
    /// Calling this for a child that is not visible is not valid.
    fn hit_test_box_child(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        child: AnyRenderBox,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        let constraints = self.constraints(app);
        let right_way_up = get_right_way_up(constraints);
        let mut delta = self.child_main_axis_position(app, child.as_object());
        let cross_axis_delta = self.child_cross_axis_position(app, child.as_object());
        let mut absolute_position = main_axis_position - delta;
        let absolute_cross_axis_position = cross_axis_position - cross_axis_delta;
        let size = child.size(app);
        let paint_extent = self.geometry(app).paint_extent;
        let (paint_offset, transformed_position) = match constraints.axis() {
            Axis::Horizontal => {
                if !right_way_up {
                    absolute_position = size.width() - absolute_position;
                    delta = paint_extent - size.width() - delta;
                }
                (
                    Offset::new(delta, cross_axis_delta),
                    Offset::new(absolute_position, absolute_cross_axis_position),
                )
            }
            Axis::Vertical => {
                if !right_way_up {
                    absolute_position = size.height() - absolute_position;
                    delta = paint_extent - size.height() - delta;
                }
                (
                    Offset::new(cross_axis_delta, delta),
                    Offset::new(absolute_cross_axis_position, absolute_position),
                )
            }
        };
        result.add_with_out_of_band_position(Some(paint_offset), None, None, |result| {
            child.hit_test(app, result, transformed_position)
        })
    }

    /// Utility function for [`RenderObject::apply_paint_transform`] for use when the children
    /// are [`crate::RenderBox`] widgets.
    ///
    /// This function turns the value returned by [`RenderSliver::child_main_axis_position`] and
    /// [`RenderSliver::child_cross_axis_position`] for the child in question into a translation
    /// that it then applies to the given matrix.
    ///
    /// Calling this for a child that is not visible is not valid.
    fn apply_paint_transform_for_box_child(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderBox,
        transform: &mut Matrix4,
    ) {
        let constraints = self.constraints(app);
        let right_way_up = get_right_way_up(constraints);
        let mut delta = self.child_main_axis_position(app, child.as_object());
        let cross_axis_delta = self.child_cross_axis_position(app, child.as_object());
        let size = child.size(app);
        let paint_extent = self.geometry(app).paint_extent;
        match constraints.axis() {
            Axis::Horizontal => {
                if !right_way_up {
                    delta = paint_extent - size.width() - delta;
                }
                translate(transform, delta, cross_axis_delta);
            }
            Axis::Vertical => {
                if !right_way_up {
                    delta = paint_extent - size.height() - delta;
                }
                translate(transform, cross_axis_delta, delta);
            }
        }
    }
}

// ADAPTER FOR RENDER BOXES INSIDE SLIVERS
// Transitions from the RenderSliver world to the RenderBox world.

/// A [`RenderSliver`] that contains a single [`crate::RenderBox`].
///
/// See also:
///
///  * [`RenderSliverToBoxAdapter`], which implements this trait to size the child according to
///    its preferred size.
pub trait RenderSliverSingleBoxAdapter:
    RenderSliver + RenderObjectWithChildMixin<ChildType = AnyRenderBox> + RenderSliverHelpers
{
    /// The body of Flutter's `setupParentData` override; call it from
    /// [`RenderObject::setup_parent_data`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SliverPhysicalParentData>(app) {
            child.set_parent_data(app, SliverPhysicalParentData::new());
        }
    }

    /// Sets the [`SliverPhysicalParentData::paint_offset`] for the given child according to the
    /// [`SliverConstraints::axis_direction`] and [`SliverConstraints::growth_direction`] and
    /// the given geometry.
    fn set_child_parent_data(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        constraints: SliverConstraints,
        geometry: SliverGeometry,
    ) {
        let paint_offset = match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => Offset::new(
                0.0,
                geometry.paint_extent + constraints.scroll_offset - geometry.scroll_extent,
            ),
            AxisDirection::Left => Offset::new(
                geometry.paint_extent + constraints.scroll_offset - geometry.scroll_extent,
                0.0,
            ),
            AxisDirection::Right => Offset::new(-constraints.scroll_offset, 0.0),
            AxisDirection::Down => Offset::new(0.0, -constraints.scroll_offset),
        };
        child
            .parent_data_of_mut::<SliverPhysicalParentData>(app)
            .paint_offset = paint_offset;
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        debug_assert!(self.geometry(app).hit_test_extent > 0.0);
        let Some(child) = self.child(app) else {
            return false;
        };
        self.hit_test_box_child(
            app,
            &mut BoxHitTestResult::wrap(result),
            child,
            main_axis_position,
            cross_axis_position,
        )
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));
        -self.constraints(app).scroll_offset
    }

    /// The body of Flutter's `applyPaintTransform` override.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));
        child
            .parent_data_of::<SliverPhysicalParentData>(app)
            .apply_paint_transform(transform);
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        if !self.geometry(app).visible {
            return;
        }
        let paint_offset = child
            .as_object()
            .parent_data_of::<SliverPhysicalParentData>(app)
            .paint_offset;
        context.paint_child(app, child.as_object(), offset + paint_offset);
    }
}

/// A [`RenderSliver`] that contains a single [`crate::RenderBox`].
///
/// The child will not be laid out if it is not visible. It is sized according to the child's
/// preferences in the main axis, and with a tight constraint forcing it to the dimensions of
/// the viewport in the cross axis.
///
/// See also:
///
///  * [`crate::RenderViewport`], which allows [`RenderSliver`] objects to be placed inside a
///    [`crate::RenderBox`] (the opposite of this class).
pub struct RenderSliverToBoxAdapter {
    render_object: crate::object::RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
}

impl RenderSliverToBoxAdapter {
    /// Creates a [`RenderSliver`] that wraps a [`crate::RenderBox`].
    pub fn new(
        app: &mut App,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderSliverToBoxAdapter> {
        let this = RenderHandle::new_sliver(
            app,
            RenderSliverToBoxAdapter {
                render_object: crate::object::RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
            },
        );
        this.set_child(app, child);
        this
    }
}

impl RenderObjectWithChildMixin for RenderSliverToBoxAdapter {
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

impl RenderSliverHelpers for RenderSliverToBoxAdapter {}

impl RenderSliverSingleBoxAdapter for RenderSliverToBoxAdapter {}

impl RenderObject for RenderSliverToBoxAdapter {
    crate::render_object_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderSliverSingleBoxAdapter::setup_parent_data(self, app, child)
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderSliverSingleBoxAdapter::apply_paint_transform(self, app, child, transform)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let Some(child) = self.child(app) else {
            self.set_geometry(app, SliverGeometry::ZERO);
            return;
        };
        let constraints = self.constraints(app);
        child.layout(
            app,
            constraints.as_box_constraints(0.0, f64::INFINITY, None),
            true,
        );
        let child_extent = match constraints.axis() {
            Axis::Horizontal => child.size(app).width(),
            Axis::Vertical => child.size(app).height(),
        };
        let painted_child_size = calculate_paint_offset(constraints, 0.0, child_extent);
        let cache_extent = calculate_cache_offset(constraints, 0.0, child_extent);

        debug_assert!(painted_child_size.is_finite());
        debug_assert!(painted_child_size >= 0.0);
        let geometry = SliverGeometry::new()
            .scroll_extent(child_extent)
            .paint_extent(painted_child_size)
            .cache_extent(cache_extent)
            .max_paint_extent(child_extent)
            .hit_test_extent(painted_child_size)
            .has_visual_overflow(
                child_extent > constraints.remaining_paint_extent
                    || constraints.scroll_offset > 0.0,
            );
        self.set_geometry(app, geometry);
        self.set_child_parent_data(app, child.as_object(), constraints, geometry);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderSliverSingleBoxAdapter::paint(self, app, context, offset)
    }
}

impl RenderSliver for RenderSliverToBoxAdapter {
    render_sliver_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        RenderSliverSingleBoxAdapter::hit_test_children(
            self,
            app,
            result,
            main_axis_position,
            cross_axis_position,
        )
    }

    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        RenderSliverSingleBoxAdapter::child_main_axis_position(self, app, child)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::box_::{RenderBox, RenderBoxData};
    use crate::proxy_box::RenderConstrainedBox;

    struct TestSliver {
        render_object: crate::object::RenderObjectData,
        render_sliver: RenderSliverData,
    }

    impl RenderObject for TestSliver {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let constraints = self.constraints(app);
            let extent = constraints.remaining_paint_extent.min(20.0);
            self.set_geometry(
                app,
                SliverGeometry::new()
                    .scroll_extent(20.0)
                    .paint_extent(extent)
                    .max_paint_extent(20.0),
            );
        }
    }

    impl RenderSliver for TestSliver {
        render_sliver_accessors!();
    }

    fn vertical_constraints(remaining_paint: f64) -> SliverConstraints {
        SliverConstraints::new(
            AxisDirection::Down,
            GrowthDirection::Forward,
            ScrollDirection::Idle,
            0.0,
            0.0,
            0.0,
            remaining_paint,
            100.0,
            AxisDirection::Right,
            400.0,
            remaining_paint,
            0.0,
        )
    }

    #[test]
    fn sliver_layout_sets_geometry() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let sliver = RenderHandle::new_sliver(
            &mut app,
            TestSliver {
                render_object: crate::object::RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
            },
        );
        sliver.layout(&mut app, vertical_constraints(50.0), false);
        let geometry = sliver.geometry(&app);
        assert_eq!(geometry.scroll_extent, 20.0);
        assert_eq!(geometry.paint_extent, 20.0);
        assert!(geometry.visible);
    }

    /// A box that fills the constraints it is given and accepts hits, so that a hit test walks
    /// all the way through the sliver adapter.
    struct HitTestableBox {
        render_object: crate::object::RenderObjectData,
        render_box: RenderBoxData,
    }

    impl RenderObject for HitTestableBox {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let size = self.constraints(app).constrain(Size::new(60.0, 60.0));
            self.set_size(app, size);
        }
    }

    impl RenderBox for HitTestableBox {
        crate::render_box_accessors!();

        fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
            let _ = self;
            true
        }
    }

    fn hit_testable_box(app: &mut App) -> AnyRenderBox {
        RenderHandle::new_box(
            app,
            HitTestableBox {
                render_object: crate::object::RenderObjectData::new(),
                render_box: RenderBoxData::new(),
            },
        )
        .as_box()
    }

    fn adapter_constraints(scroll_offset: f64, remaining_paint: f64) -> SliverConstraints {
        SliverConstraints::new(
            AxisDirection::Down,
            GrowthDirection::Forward,
            ScrollDirection::Idle,
            scroll_offset,
            0.0,
            0.0,
            remaining_paint,
            100.0,
            AxisDirection::Right,
            remaining_paint,
            remaining_paint,
            0.0,
        )
    }

    /// `sliver_test.dart`: a `RenderSliverToBoxAdapter` sizes itself to its box child in the main
    /// axis and reports the visible portion as its paint extent.
    #[test]
    fn a_box_adapter_reports_its_child_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(60.0)), None);
        let adapter = RenderSliverToBoxAdapter::new(&mut app, Some(child.as_box()));

        adapter.layout(&mut app, adapter_constraints(0.0, 400.0), true);

        let geometry = adapter.geometry(&app);
        assert_eq!(geometry.scroll_extent, 60.0);
        assert_eq!(geometry.paint_extent, 60.0);
        assert_eq!(geometry.max_paint_extent, 60.0);
        assert!(!geometry.has_visual_overflow);
        // The cross axis is tight to the sliver's cross axis extent.
        assert_eq!(child.size(&app), Size::new(100.0, 60.0));
        assert_eq!(
            child
                .as_box()
                .as_object()
                .parent_data_of::<SliverPhysicalParentData>(&app)
                .paint_offset,
            Offset::ZERO
        );
    }

    /// A scrolled adapter moves its child up by the scroll offset and shrinks its paint extent.
    #[test]
    fn a_scrolled_box_adapter_offsets_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(60.0)), None);
        let adapter = RenderSliverToBoxAdapter::new(&mut app, Some(child.as_box()));

        adapter.layout(&mut app, adapter_constraints(20.0, 400.0), true);

        let geometry = adapter.geometry(&app);
        assert_eq!(geometry.scroll_extent, 60.0);
        assert_eq!(geometry.paint_extent, 40.0);
        assert!(geometry.has_visual_overflow);
        assert_eq!(
            child
                .as_box()
                .as_object()
                .parent_data_of::<SliverPhysicalParentData>(&app)
                .paint_offset,
            Offset::new(0.0, -20.0)
        );
        assert_eq!(
            RenderSliver::child_main_axis_position(adapter, &app, child.as_object()),
            -20.0
        );
    }

    /// An empty adapter reports zero geometry.
    #[test]
    fn a_childless_box_adapter_is_zero() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let adapter = RenderSliverToBoxAdapter::new(&mut app, None);
        adapter.layout(&mut app, adapter_constraints(0.0, 400.0), true);
        assert!(adapter.geometry(&app).is_zero());
    }

    /// A hit inside the visible part of the child lands on both the child and the sliver.
    #[test]
    fn a_box_adapter_hit_tests_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = hit_testable_box(&mut app);
        let adapter = RenderSliverToBoxAdapter::new(&mut app, Some(child));
        adapter.layout(&mut app, adapter_constraints(0.0, 400.0), true);

        let mut result = HitTestResult::new();
        let hit = adapter.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut result),
            30.0,
            50.0,
        );

        assert!(hit);
        assert_eq!(result.path().len(), 2);
    }

    /// `calculate_paint_offset` clamps the requested region to the visible window.
    #[test]
    fn paint_and_cache_offsets_clamp_to_the_window() {
        let constraints = adapter_constraints(20.0, 50.0);
        assert_eq!(calculate_paint_offset(constraints, 0.0, 100.0), 50.0);
        assert_eq!(calculate_paint_offset(constraints, 0.0, 30.0), 10.0);
        assert_eq!(calculate_paint_offset(constraints, 0.0, 10.0), 0.0);
        assert_eq!(calculate_cache_offset(constraints, 0.0, 100.0), 50.0);
    }

    /// `SliverGeometry.debugAssertIsValid` accepts a consistent geometry.
    #[test]
    fn a_consistent_geometry_is_valid() {
        let geometry = SliverGeometry::new()
            .scroll_extent(100.0)
            .paint_extent(40.0)
            .max_paint_extent(100.0);
        assert!(geometry.debug_assert_is_valid());
        assert!(!geometry.is_zero());
        assert!(SliverGeometry::ZERO.is_zero());
    }
}
