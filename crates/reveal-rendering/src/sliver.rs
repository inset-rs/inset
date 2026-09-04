//! Flutter counterpart: `rendering/sliver.dart` (`GrowthDirection`,
//! `SliverConstraints`, `SliverGeometry`, `RenderSliver`).
//!
//! Viewport, sliver-to-box adapters, and paint wait.

use std::fmt::{self, Debug, Display};
use std::hash::Hasher;

use reveal_embedder::Rect;
use reveal_foundation::{App, HandleId};
use reveal_painting::{
    Axis, AxisDirection, axis_direction_is_reversed, axis_direction_to_axis, flip_axis_direction,
};

use crate::box_::BoxConstraints;
use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectVTable, create, resolve,
};
use crate::pipeline_owner::PipelineOwner;
use crate::viewport_offset::{ScrollDirection, flip_scroll_direction};

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

    /// See [`AnyRenderSliver::layout`].
    fn layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: SliverConstraints,
        parent_uses_size: bool,
    ) {
        self.as_sliver().layout(app, constraints, parent_uses_size)
    }

    /// The erased `RenderSliver` edge. Free: the vtable is a `const`, and the id is copied.
    fn as_sliver(self: RenderHandle<Self>) -> AnyRenderSliver {
        AnyRenderSliver {
            id: self.id(),
            vtable: const { &RenderSliverVTable::of::<Self>() },
        }
    }

    /// The erased `RenderObject` edge, through [`as_sliver`](Self::as_sliver).
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
    fn owner(self: RenderHandle<Self>, app: &App) -> Option<PipelineOwner> {
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
}

impl RenderSliverVTable {
    const fn of<T: RenderSliver>() -> RenderSliverVTable {
        RenderSliverVTable {
            object: RenderObjectVTable::of::<T>(
                |app, id, child| <T as RenderObject>::setup_parent_data(resolve(id), app, child),
                |app, id| T::paint_bounds(resolve(id), app),
                None,
                Some(|| const { &RenderSliverVTable::of::<T>() }),
            ),
            sliver_data: |app, id| T::render_sliver_data(resolve(id), app),
            sliver_data_mut: |app, id| T::render_sliver_data_mut(resolve(id), app),
        }
    }
}

impl<T: RenderSliver> RenderHandle<T> {
    /// Creates a sliver-protocol render object in `app`.
    pub fn new_sliver(app: &mut App, object: T) -> RenderHandle<T> {
        let this = create(app, object);
        // Flutter's `RenderObject()` constructor: `_wasRepaintBoundary = isRepaintBoundary`.
        let is_repaint_boundary = this.is_repaint_boundary(app);
        this.render_object_data_mut(app).was_repaint_boundary = is_repaint_boundary;
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

impl From<AnyRenderSliver> for AnyRenderObject {
    fn from(sliver: AnyRenderSliver) -> AnyRenderObject {
        sliver.as_object()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut app = App::new();
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
}
