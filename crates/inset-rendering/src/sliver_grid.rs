//! Flutter counterpart: `rendering/sliver_grid.dart`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

use inset_embedder::{Matrix4, Offset};
use inset_foundation::{App, Handle, PRECISION_ERROR_TOLERANCE};
use inset_painting::axis_direction_is_reversed;

use crate::box_::{AnyRenderBox, BoxConstraints};
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::sliver::{
    RenderSliver, RenderSliverData, RenderSliverHelpers, SliverConstraints, SliverGeometry,
    SliverHitTestResult, calculate_cache_offset, calculate_paint_offset,
};
use crate::sliver_multi_box_adaptor::{
    RenderSliverBoxChildManager, RenderSliverMultiBoxAdaptor, RenderSliverMultiBoxAdaptorData,
    RenderSliverWithKeepAliveMixin, SliverMultiBoxAdaptorParentData,
};

/// Describes the placement of a child in a [`RenderSliverGrid`].
///
/// This class is similar to [`Rect`](inset_embedder::Rect), in that it gives a two-dimensional
/// position and a two-dimensional dimension, but is direction-agnostic.
///
/// See also:
///
///  * [`SliverGridLayout`], which represents the geometry of all the tiles in a grid.
///  * [`SliverGridLayout::get_geometry_for_child_index`], which returns this object to describe
///    the child's placement.
///  * [`RenderSliverGrid`], which uses this class during its
///    [`perform_layout`](RenderObject::perform_layout) method.
#[derive(Clone, Copy)]
pub struct SliverGridGeometry {
    /// The scroll offset of the leading edge of the child relative to the leading edge of the
    /// parent.
    pub scroll_offset: f64,
    /// The offset of the child in the non-scrolling axis.
    ///
    /// If the scroll axis is vertical, this offset is from the left-most edge of the parent to
    /// the left-most edge of the child. If the scroll axis is horizontal, this offset is from the
    /// top-most edge of the parent to the top-most edge of the child.
    pub cross_axis_offset: f64,
    /// The extent of the child in the scrolling axis.
    ///
    /// If the scroll axis is vertical, this extent is the child's height. If the scroll axis is
    /// horizontal, this extent is the child's width.
    pub main_axis_extent: f64,
    /// The extent of the child in the non-scrolling axis.
    ///
    /// If the scroll axis is vertical, this extent is the child's width. If the scroll axis is
    /// horizontal, this extent is the child's height.
    pub cross_axis_extent: f64,
}

impl SliverGridGeometry {
    /// Creates an object that describes the placement of a child in a [`RenderSliverGrid`].
    pub const fn new(
        scroll_offset: f64,
        cross_axis_offset: f64,
        main_axis_extent: f64,
        cross_axis_extent: f64,
    ) -> SliverGridGeometry {
        SliverGridGeometry {
            scroll_offset,
            cross_axis_offset,
            main_axis_extent,
            cross_axis_extent,
        }
    }

    /// The scroll offset of the trailing edge of the child relative to the leading edge of the
    /// parent.
    pub fn trailing_scroll_offset(&self) -> f64 {
        self.scroll_offset + self.main_axis_extent
    }

    /// Returns a tight [`BoxConstraints`] that forces the child to have the required size, given
    /// a [`SliverConstraints`].
    pub fn get_box_constraints(&self, constraints: SliverConstraints) -> BoxConstraints {
        constraints.as_box_constraints(
            self.main_axis_extent,
            self.main_axis_extent,
            Some(self.cross_axis_extent),
        )
    }
}

impl Debug for SliverGridGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SliverGridGeometry(scrollOffset: {}, crossAxisOffset: {}, mainAxisExtent: {}, crossAxisExtent: {})",
            self.scroll_offset,
            self.cross_axis_offset,
            self.main_axis_extent,
            self.cross_axis_extent
        )
    }
}

/// The size and position of all the tiles in a [`RenderSliverGrid`].
///
/// Rather that providing a grid with a [`SliverGridLayout`] directly, the grid is provided a
/// [`SliverGridDelegate`], which computes a [`SliverGridLayout`] given a set of
/// [`SliverConstraints`]. This allows the algorithm to dynamically respond to changes in the
/// environment (e.g. the user rotating the device).
///
/// The tiles can be placed arbitrarily, but it is more efficient to place tiles roughly in order
/// by scroll offset because grids reify a contiguous sequence of children.
///
/// See also:
///
///  * [`SliverGridRegularTileLayout`], which represents a layout that uses equally sized and
///    spaced tiles.
///  * [`SliverGridGeometry`], which represents the size and position of a single tile in a grid.
///  * [`SliverGridDelegate::get_layout`], which returns this object to describe the delegate's
///    layout.
///  * [`RenderSliverGrid`], which uses this class during its
///    [`perform_layout`](RenderObject::perform_layout) method.
pub trait SliverGridLayout: Debug {
    /// The minimum child index that intersects with (or is after) this scroll offset.
    fn get_min_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32;

    /// The maximum child index that intersects with (or is before) this scroll offset.
    fn get_max_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32;

    /// The size and position of the child with the given index.
    fn get_geometry_for_child_index(&self, index: i32) -> SliverGridGeometry;

    /// The scroll extent needed to fully display all the tiles if there are `child_count`
    /// children in total.
    fn compute_max_scroll_offset(&self, child_count: i32) -> f64;
}

/// A [`SliverGridLayout`] that uses equally sized and spaced tiles.
///
/// Rather that providing a grid with a [`SliverGridLayout`] directly, you instead provide the
/// grid a [`SliverGridDelegate`], which can compute a [`SliverGridLayout`] given the current
/// [`SliverConstraints`].
///
/// This layout is used by [`SliverGridDelegateWithFixedCrossAxisCount`] and
/// [`SliverGridDelegateWithMaxCrossAxisExtent`].
#[derive(Clone, Copy, Debug)]
pub struct SliverGridRegularTileLayout {
    /// The number of children in the cross axis.
    pub cross_axis_count: i32,
    /// The number of pixels from the leading edge of one tile to the leading edge of the next
    /// tile in the main axis.
    pub main_axis_stride: f64,
    /// The number of pixels from the leading edge of one tile to the leading edge of the next
    /// tile in the cross axis.
    pub cross_axis_stride: f64,
    /// The number of pixels from the leading edge of one tile to the trailing edge of the same
    /// tile in the main axis.
    pub child_main_axis_extent: f64,
    /// The number of pixels from the leading edge of one tile to the trailing edge of the same
    /// tile in the cross axis.
    pub child_cross_axis_extent: f64,
    /// Whether the children should be placed in the opposite order of increasing coordinates in
    /// the cross axis.
    ///
    /// For example, if the cross axis is horizontal, the children are placed from left to right
    /// when [`reverse_cross_axis`](Self::reverse_cross_axis) is false and from right to left when
    /// it is true.
    ///
    /// Typically set to the return value of [`axis_direction_is_reversed`] applied to the
    /// [`SliverConstraints::cross_axis_direction`].
    pub reverse_cross_axis: bool,
}

impl SliverGridRegularTileLayout {
    /// Creates a layout that uses equally sized and spaced tiles.
    ///
    /// All of the arguments must not be negative. The `cross_axis_count` argument must be greater
    /// than zero.
    pub fn new(
        cross_axis_count: i32,
        main_axis_stride: f64,
        cross_axis_stride: f64,
        child_main_axis_extent: f64,
        child_cross_axis_extent: f64,
        reverse_cross_axis: bool,
    ) -> SliverGridRegularTileLayout {
        debug_assert!(cross_axis_count > 0);
        debug_assert!(main_axis_stride >= 0.0);
        debug_assert!(cross_axis_stride >= 0.0);
        debug_assert!(child_main_axis_extent >= 0.0);
        debug_assert!(child_cross_axis_extent >= 0.0);
        SliverGridRegularTileLayout {
            cross_axis_count,
            main_axis_stride,
            cross_axis_stride,
            child_main_axis_extent,
            child_cross_axis_extent,
            reverse_cross_axis,
        }
    }

    fn get_offset_from_start_in_cross_axis(&self, cross_axis_start: f64) -> f64 {
        if self.reverse_cross_axis {
            return f64::from(self.cross_axis_count) * self.cross_axis_stride
                - cross_axis_start
                - self.child_cross_axis_extent
                - (self.cross_axis_stride - self.child_cross_axis_extent);
        }
        cross_axis_start
    }
}

impl SliverGridLayout for SliverGridRegularTileLayout {
    fn get_min_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        if self.main_axis_stride > PRECISION_ERROR_TOLERANCE {
            self.cross_axis_count * (scroll_offset / self.main_axis_stride).trunc() as i32
        } else {
            0
        }
    }

    fn get_max_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        if self.main_axis_stride > 0.0 {
            let main_axis_count = (scroll_offset / self.main_axis_stride).ceil() as i32;
            return (self.cross_axis_count * main_axis_count - 1).max(0);
        }
        0
    }

    fn get_geometry_for_child_index(&self, index: i32) -> SliverGridGeometry {
        let cross_axis_start = f64::from(index % self.cross_axis_count) * self.cross_axis_stride;
        SliverGridGeometry::new(
            f64::from(index / self.cross_axis_count) * self.main_axis_stride,
            self.get_offset_from_start_in_cross_axis(cross_axis_start),
            self.child_main_axis_extent,
            self.child_cross_axis_extent,
        )
    }

    fn compute_max_scroll_offset(&self, child_count: i32) -> f64 {
        if child_count == 0 {
            // There are no children in the grid. The max scroll offset should be zero.
            return 0.0;
        }
        let main_axis_count = ((child_count - 1) / self.cross_axis_count) + 1;
        let main_axis_spacing = self.main_axis_stride - self.child_main_axis_extent;
        self.main_axis_stride * f64::from(main_axis_count) - main_axis_spacing
    }
}

/// Controls the layout of tiles in a grid.
///
/// Given the current constraints on the grid, a [`SliverGridDelegate`] computes the layout for
/// the tiles in the grid. The tiles can be placed arbitrarily, but it is more efficient to place
/// tiles roughly in order by scroll offset because grids reify a contiguous sequence of children.
///
/// See also:
///
///  * [`SliverGridDelegateWithFixedCrossAxisCount`], which creates a layout with a fixed number
///    of tiles in the cross axis.
///  * [`SliverGridDelegateWithMaxCrossAxisExtent`], which creates a layout with tiles that have a
///    maximum cross-axis extent.
///  * `GridView`, which uses this delegate to control the layout of its tiles.
///  * `SliverGrid`, which uses this delegate to control the layout of its tiles.
///  * [`RenderSliverGrid`], which uses this delegate to control the layout of its tiles.
pub trait SliverGridDelegate: Debug {
    /// Returns information about the size and position of the tiles in the grid.
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout>;

    /// Override this method to return true when the children need to be laid out.
    ///
    /// This should compare the fields of the current delegate and the given `old_delegate` and
    /// return true if the fields are such that the layout would be different. The grid asks only
    /// for an `old_delegate` of this delegate's own type.
    fn should_relayout(&self, old_delegate: &dyn SliverGridDelegate) -> bool;

    /// Downcast support for Dart's `runtimeType` comparison and `covariant` parameters.
    fn as_any(&self) -> &dyn Any;
}

/// Creates grid layouts with a fixed number of tiles in the cross axis.
///
/// For example, if the grid is vertical, this delegate will create a layout with a fixed number
/// of columns. If the grid is horizontal, this delegate will create a layout with a fixed number
/// of rows.
///
/// This delegate creates grids with equally sized and spaced tiles.
///
/// Dart's named constructor arguments are the fluent setters.
///
/// See also:
///
///  * [`SliverGridDelegateWithMaxCrossAxisExtent`], which creates a layout with tiles that have a
///    maximum cross-axis extent.
///  * [`SliverGridDelegate`], which creates arbitrary layouts.
///  * [`RenderSliverGrid`], which can use this delegate to control the layout of its tiles.
#[derive(Clone, Debug)]
pub struct SliverGridDelegateWithFixedCrossAxisCount {
    /// The number of children in the cross axis.
    pub cross_axis_count: i32,
    /// The number of logical pixels between each child along the main axis.
    pub main_axis_spacing: f64,
    /// The number of logical pixels between each child along the cross axis.
    pub cross_axis_spacing: f64,
    /// The ratio of the cross-axis to the main-axis extent of each child.
    pub child_aspect_ratio: f64,
    /// The extent of each tile in the main axis. If provided it would define the logical pixels
    /// taken by each tile in the main-axis.
    ///
    /// If `None`, [`child_aspect_ratio`](Self::child_aspect_ratio) is used instead.
    pub main_axis_extent: Option<f64>,
}

impl SliverGridDelegateWithFixedCrossAxisCount {
    /// Creates a delegate that makes grid layouts with a fixed number of tiles in the cross axis.
    ///
    /// The `cross_axis_count` must be greater than zero.
    pub fn new(cross_axis_count: i32) -> SliverGridDelegateWithFixedCrossAxisCount {
        debug_assert!(cross_axis_count > 0);
        SliverGridDelegateWithFixedCrossAxisCount {
            cross_axis_count,
            main_axis_spacing: 0.0,
            cross_axis_spacing: 0.0,
            child_aspect_ratio: 1.0,
            main_axis_extent: None,
        }
    }

    /// Dart `SliverGridDelegateWithFixedCrossAxisCount(mainAxisSpacing:)`. Must not be negative.
    pub fn main_axis_spacing(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.main_axis_spacing = value;
        self
    }

    /// Dart `SliverGridDelegateWithFixedCrossAxisCount(crossAxisSpacing:)`. Must not be negative.
    pub fn cross_axis_spacing(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.cross_axis_spacing = value;
        self
    }

    /// Dart `SliverGridDelegateWithFixedCrossAxisCount(childAspectRatio:)`. Must be greater than
    /// zero.
    pub fn child_aspect_ratio(mut self, value: f64) -> Self {
        debug_assert!(value > 0.0);
        self.child_aspect_ratio = value;
        self
    }

    /// Dart `SliverGridDelegateWithFixedCrossAxisCount(mainAxisExtent:)`. Must not be negative.
    pub fn main_axis_extent(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.main_axis_extent = Some(value);
        self
    }

    fn debug_assert_is_valid(&self) -> bool {
        debug_assert!(self.cross_axis_count > 0);
        debug_assert!(self.main_axis_spacing >= 0.0);
        debug_assert!(self.cross_axis_spacing >= 0.0);
        debug_assert!(self.child_aspect_ratio > 0.0);
        true
    }
}

impl SliverGridDelegate for SliverGridDelegateWithFixedCrossAxisCount {
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout> {
        debug_assert!(self.debug_assert_is_valid());
        let usable_cross_axis_extent = (constraints.cross_axis_extent
            - self.cross_axis_spacing * f64::from(self.cross_axis_count - 1))
        .max(0.0);
        let child_cross_axis_extent = usable_cross_axis_extent / f64::from(self.cross_axis_count);
        let child_main_axis_extent = self
            .main_axis_extent
            .unwrap_or(child_cross_axis_extent / self.child_aspect_ratio);
        Box::new(SliverGridRegularTileLayout::new(
            self.cross_axis_count,
            child_main_axis_extent + self.main_axis_spacing,
            child_cross_axis_extent + self.cross_axis_spacing,
            child_main_axis_extent,
            child_cross_axis_extent,
            axis_direction_is_reversed(constraints.cross_axis_direction),
        ))
    }

    fn should_relayout(&self, old_delegate: &dyn SliverGridDelegate) -> bool {
        let Some(old) = old_delegate.as_any().downcast_ref::<Self>() else {
            return true;
        };
        old.cross_axis_count != self.cross_axis_count
            || old.main_axis_spacing != self.main_axis_spacing
            || old.cross_axis_spacing != self.cross_axis_spacing
            || old.child_aspect_ratio != self.child_aspect_ratio
            || old.main_axis_extent != self.main_axis_extent
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Creates grid layouts with tiles that each have a maximum cross-axis extent.
///
/// This delegate will select a cross-axis extent for the tiles that is as large as possible
/// subject to the following conditions:
///
///  - The extent evenly divides the cross-axis extent of the grid.
///  - The extent is at most [`max_cross_axis_extent`](Self::max_cross_axis_extent).
///
/// For example, if the grid is vertical, the grid is 500.0 pixels wide, and
/// `max_cross_axis_extent` is 150.0, this delegate will create a grid with 4 columns that are
/// 125.0 pixels wide.
///
/// This delegate creates grids with equally sized and spaced tiles.
///
/// Dart's named constructor arguments are the fluent setters.
///
/// See also:
///
///  * [`SliverGridDelegateWithFixedCrossAxisCount`], which creates a layout with a fixed number
///    of tiles in the cross axis.
///  * [`SliverGridDelegate`], which creates arbitrary layouts.
///  * [`RenderSliverGrid`], which can use this delegate to control the layout of its tiles.
#[derive(Clone, Debug)]
pub struct SliverGridDelegateWithMaxCrossAxisExtent {
    /// The maximum extent of tiles in the cross axis.
    pub max_cross_axis_extent: f64,
    /// The number of logical pixels between each child along the main axis.
    pub main_axis_spacing: f64,
    /// The number of logical pixels between each child along the cross axis.
    pub cross_axis_spacing: f64,
    /// The ratio of the cross-axis to the main-axis extent of each child.
    pub child_aspect_ratio: f64,
    /// The extent of each tile in the main axis. If provided it would define the logical pixels
    /// taken by each tile in the main-axis.
    ///
    /// If `None`, [`child_aspect_ratio`](Self::child_aspect_ratio) is used instead.
    pub main_axis_extent: Option<f64>,
}

impl SliverGridDelegateWithMaxCrossAxisExtent {
    /// Creates a delegate that makes grid layouts with tiles that have a maximum cross-axis
    /// extent.
    ///
    /// The `max_cross_axis_extent` must be greater than zero.
    pub fn new(max_cross_axis_extent: f64) -> SliverGridDelegateWithMaxCrossAxisExtent {
        debug_assert!(max_cross_axis_extent > 0.0);
        SliverGridDelegateWithMaxCrossAxisExtent {
            max_cross_axis_extent,
            main_axis_spacing: 0.0,
            cross_axis_spacing: 0.0,
            child_aspect_ratio: 1.0,
            main_axis_extent: None,
        }
    }

    /// Dart `SliverGridDelegateWithMaxCrossAxisExtent(mainAxisSpacing:)`. Must not be negative.
    pub fn main_axis_spacing(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.main_axis_spacing = value;
        self
    }

    /// Dart `SliverGridDelegateWithMaxCrossAxisExtent(crossAxisSpacing:)`. Must not be negative.
    pub fn cross_axis_spacing(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.cross_axis_spacing = value;
        self
    }

    /// Dart `SliverGridDelegateWithMaxCrossAxisExtent(childAspectRatio:)`. Must be greater than
    /// zero.
    pub fn child_aspect_ratio(mut self, value: f64) -> Self {
        debug_assert!(value > 0.0);
        self.child_aspect_ratio = value;
        self
    }

    /// Dart `SliverGridDelegateWithMaxCrossAxisExtent(mainAxisExtent:)`. Must not be negative.
    pub fn main_axis_extent(mut self, value: f64) -> Self {
        debug_assert!(value >= 0.0);
        self.main_axis_extent = Some(value);
        self
    }

    fn debug_assert_is_valid(&self, cross_axis_extent: f64) -> bool {
        debug_assert!(cross_axis_extent > 0.0);
        debug_assert!(self.max_cross_axis_extent > 0.0);
        debug_assert!(self.main_axis_spacing >= 0.0);
        debug_assert!(self.cross_axis_spacing >= 0.0);
        debug_assert!(self.child_aspect_ratio > 0.0);
        true
    }
}

impl SliverGridDelegate for SliverGridDelegateWithMaxCrossAxisExtent {
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout> {
        debug_assert!(self.debug_assert_is_valid(constraints.cross_axis_extent));
        let cross_axis_count = (constraints.cross_axis_extent
            / (self.max_cross_axis_extent + self.cross_axis_spacing))
            .ceil() as i32;
        // Ensure a minimum count of 1, can be zero and result in an infinite extent
        // below when the window size is 0.
        let cross_axis_count = cross_axis_count.max(1);
        let usable_cross_axis_extent = (constraints.cross_axis_extent
            - self.cross_axis_spacing * f64::from(cross_axis_count - 1))
        .max(0.0);
        let child_cross_axis_extent = usable_cross_axis_extent / f64::from(cross_axis_count);
        let child_main_axis_extent = self
            .main_axis_extent
            .unwrap_or(child_cross_axis_extent / self.child_aspect_ratio);
        Box::new(SliverGridRegularTileLayout::new(
            cross_axis_count,
            child_main_axis_extent + self.main_axis_spacing,
            child_cross_axis_extent + self.cross_axis_spacing,
            child_main_axis_extent,
            child_cross_axis_extent,
            axis_direction_is_reversed(constraints.cross_axis_direction),
        ))
    }

    fn should_relayout(&self, old_delegate: &dyn SliverGridDelegate) -> bool {
        let Some(old) = old_delegate.as_any().downcast_ref::<Self>() else {
            return true;
        };
        old.max_cross_axis_extent != self.max_cross_axis_extent
            || old.main_axis_spacing != self.main_axis_spacing
            || old.cross_axis_spacing != self.cross_axis_spacing
            || old.child_aspect_ratio != self.child_aspect_ratio
            || old.main_axis_extent != self.main_axis_extent
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Parent data structure used by [`RenderSliverGrid`].
#[derive(Debug, Default)]
pub struct SliverGridParentData {
    adaptor: SliverMultiBoxAdaptorParentData,
    /// The offset of the child in the non-scrolling axis.
    ///
    /// If the scroll axis is vertical, this offset is from the left-most edge of the parent to
    /// the left-most edge of the child. If the scroll axis is horizontal, this offset is from the
    /// top-most edge of the parent to the top-most edge of the child.
    pub cross_axis_offset: Option<f64>,
}

impl SliverGridParentData {
    /// Creates parent data with no index, layout offset or cross-axis offset.
    pub const fn new() -> SliverGridParentData {
        SliverGridParentData {
            adaptor: SliverMultiBoxAdaptorParentData::new(),
            cross_axis_offset: None,
        }
    }
}

impl ParentData for SliverGridParentData {
    fn detach(&mut self) {
        ParentData::detach(&mut self.adaptor);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SliverGridParentData>() {
            return Some(self);
        }
        self.adaptor.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SliverGridParentData>() {
            return Some(self);
        }
        self.adaptor.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SliverGridParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        self.adaptor.container_parent_data()
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        self.adaptor.container_parent_data_mut()
    }
}

impl Display for SliverGridParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "crossAxisOffset={:?}; {}",
            self.cross_axis_offset, self.adaptor
        )
    }
}

/// A sliver that places multiple box children in a two dimensional arrangement.
///
/// [`RenderSliverGrid`] places its children in arbitrary positions determined by
/// [`grid_delegate`](Self::grid_delegate). Each child is forced to have the size specified by
/// the `grid_delegate`.
///
/// See also:
///
///  * [`RenderSliverList`](crate::RenderSliverList), which places its children in a linear array.
///  * [`RenderSliverFixedExtentList`](crate::RenderSliverFixedExtentList), which places its
///    children in a linear array with a fixed extent in the main axis.
pub struct RenderSliverGrid {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    grid_delegate: Rc<dyn SliverGridDelegate>,
}

impl RenderSliverGrid {
    /// Creates a sliver that contains multiple box children whose size and position are
    /// determined by a delegate.
    pub fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        grid_delegate: Rc<dyn SliverGridDelegate>,
    ) -> RenderHandle<RenderSliverGrid> {
        RenderHandle::new_sliver(
            app,
            RenderSliverGrid {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                grid_delegate,
            },
        )
    }

    /// The delegate that controls the size and position of the children.
    pub fn grid_delegate(self: RenderHandle<Self>, app: &App) -> Rc<dyn SliverGridDelegate> {
        Rc::clone(&self.get(app).grid_delegate)
    }

    /// Sets [`grid_delegate`](Self::grid_delegate).
    pub fn set_grid_delegate(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Rc<dyn SliverGridDelegate>,
    ) {
        let old = self.grid_delegate(app);
        if Rc::ptr_eq(&old, &value) {
            return;
        }
        if value.as_any().type_id() != old.as_any().type_id() || value.should_relayout(&*old) {
            self.mark_needs_layout(app);
        }
        self.get_mut(app).grid_delegate = value;
    }

    /// Stores where `child` was laid out, from `geometry`.
    fn place(app: &mut App, child: AnyRenderBox, geometry: SliverGridGeometry) {
        let child_parent_data = child
            .as_object()
            .parent_data_of_mut::<SliverGridParentData>(app);
        child_parent_data
            .adaptor
            .set_layout_offset(Some(geometry.scroll_offset));
        child_parent_data.cross_axis_offset = Some(geometry.cross_axis_offset);
    }
}

impl ContainerRenderObjectMixin for RenderSliverGrid {
    type ChildType = AnyRenderBox;
    type ParentDataType = SliverMultiBoxAdaptorParentData;

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

    fn insert(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        RenderSliverMultiBoxAdaptor::insert(self, app, child, after)
    }

    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        RenderSliverMultiBoxAdaptor::move_child(self, app, child, after)
    }

    fn remove(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
        RenderSliverMultiBoxAdaptor::remove(self, app, child)
    }

    fn remove_all(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverMultiBoxAdaptor::remove_all(self, app)
    }
}

impl RenderSliverHelpers for RenderSliverGrid {}

impl RenderSliverWithKeepAliveMixin for RenderSliverGrid {}

impl RenderSliverMultiBoxAdaptor for RenderSliverGrid {
    crate::render_sliver_multi_box_adaptor_accessors!();
}

impl RenderObject for RenderSliverGrid {
    crate::render_object_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SliverGridParentData>(app) {
            child.set_parent_data(app, SliverGridParentData::new());
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        RenderSliverMultiBoxAdaptor::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        RenderSliverMultiBoxAdaptor::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverMultiBoxAdaptor::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverMultiBoxAdaptor::redepth_children(self, app)
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderSliverMultiBoxAdaptor::apply_paint_transform(self, app, child, transform)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderSliverMultiBoxAdaptor::paint(self, app, context, offset)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let child_manager = self.child_manager(app);
        child_manager.did_start_layout(app);
        child_manager.set_did_underflow(app, false);

        let scroll_offset = constraints.scroll_offset + constraints.cache_origin;
        debug_assert!(scroll_offset >= 0.0);
        let remaining_extent = constraints.remaining_cache_extent;
        debug_assert!(remaining_extent >= 0.0);
        let target_end_scroll_offset = scroll_offset + remaining_extent;

        let layout = self.grid_delegate(app).get_layout(constraints);

        let first_index = layout.get_min_child_index_for_scroll_offset(scroll_offset);
        let target_last_index = target_end_scroll_offset
            .is_finite()
            .then(|| layout.get_max_child_index_for_scroll_offset(target_end_scroll_offset));

        if self.first_child(app).is_some() {
            let leading_garbage = self.calculate_leading_garbage(app, first_index);
            let trailing_garbage = match target_last_index {
                Some(last_index) => self.calculate_trailing_garbage(app, last_index),
                None => 0,
            };
            self.collect_garbage(app, leading_garbage, trailing_garbage);
        } else {
            self.collect_garbage(app, 0, 0);
        }

        let first_child_grid_geometry = layout.get_geometry_for_child_index(first_index);

        if self.first_child(app).is_none()
            && !self.add_initial_child(app, first_index, first_child_grid_geometry.scroll_offset)
        {
            // There are either no children, or we are past the end of all our children.
            let max = layout.compute_max_scroll_offset(child_manager.child_count(app));
            self.set_geometry(
                app,
                SliverGeometry::builder()
                    .scroll_extent(max)
                    .max_paint_extent(max)
                    .build(),
            );
            child_manager.did_finish_layout(app);
            return;
        }

        let leading_scroll_offset = first_child_grid_geometry.scroll_offset;
        let mut trailing_scroll_offset = first_child_grid_geometry.trailing_scroll_offset();
        let mut trailing_child_with_layout: Option<AnyRenderBox> = None;
        let mut reached_end = false;

        let first_child = self.first_child(app).expect("added above");
        let mut index = self.index_of(app, first_child) - 1;
        while index >= first_index {
            let grid_geometry = layout.get_geometry_for_child_index(index);
            let child = self
                .insert_and_layout_leading_child(
                    app,
                    grid_geometry.get_box_constraints(constraints),
                    false,
                )
                .expect("the children before the first one are there");
            Self::place(app, child, grid_geometry);
            debug_assert_eq!(
                child
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .index,
                Some(index)
            );
            trailing_child_with_layout.get_or_insert(child);
            trailing_scroll_offset =
                trailing_scroll_offset.max(grid_geometry.trailing_scroll_offset());
            index -= 1;
        }

        if trailing_child_with_layout.is_none() {
            let first_child = self.first_child(app).expect("the child list is not empty");
            first_child.layout(
                app,
                first_child_grid_geometry.get_box_constraints(constraints),
                false,
            );
            Self::place(app, first_child, first_child_grid_geometry);
            trailing_child_with_layout = Some(first_child);
        }

        let mut index = self.index_of(app, trailing_child_with_layout.expect("set above")) + 1;
        while target_last_index.is_none_or(|last_index| index <= last_index) {
            let grid_geometry = layout.get_geometry_for_child_index(index);
            let child_constraints = grid_geometry.get_box_constraints(constraints);
            let after = trailing_child_with_layout.expect("set above");
            let mut child = self.child_after(app, after);
            if child.is_none_or(|current| self.index_of(app, current) != index) {
                child = self.insert_and_layout_child(app, child_constraints, Some(after), false);
                if child.is_none() {
                    reached_end = true;
                    // We have run out of children.
                    break;
                }
            } else {
                child
                    .expect("checked")
                    .layout(app, child_constraints, false);
            }
            let child = child.expect("set above");
            trailing_child_with_layout = Some(child);
            Self::place(app, child, grid_geometry);
            debug_assert_eq!(
                child
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .index,
                Some(index)
            );
            trailing_scroll_offset =
                trailing_scroll_offset.max(grid_geometry.trailing_scroll_offset());
            index += 1;
        }

        let last_index = self.index_of(app, self.last_child(app).expect("not empty"));

        debug_assert!(self.debug_assert_child_list_is_non_empty_and_contiguous(app));
        debug_assert_eq!(
            self.index_of(app, self.first_child(app).expect("not empty")),
            first_index
        );
        debug_assert!(target_last_index.is_none_or(|target| last_index <= target));

        let estimated_total_extent = if reached_end {
            trailing_scroll_offset
        } else {
            child_manager.estimate_max_scroll_offset(
                app,
                constraints,
                Some(first_index),
                Some(last_index),
                Some(leading_scroll_offset),
                Some(trailing_scroll_offset),
            )
        };
        let paint_extent = calculate_paint_offset(
            constraints,
            constraints.scroll_offset.min(leading_scroll_offset),
            trailing_scroll_offset,
        );
        let cache_extent =
            calculate_cache_offset(constraints, leading_scroll_offset, trailing_scroll_offset);

        self.set_geometry(
            app,
            SliverGeometry::builder()
                .scroll_extent(estimated_total_extent)
                .paint_extent(paint_extent)
                .max_paint_extent(estimated_total_extent)
                .cache_extent(cache_extent)
                .has_visual_overflow(
                    estimated_total_extent > paint_extent
                        || constraints.scroll_offset > 0.0
                        || constraints.overlap != 0.0,
                )
                .build(),
        );

        // We may have started the layout while scrolled to the end, which
        // would not expose a new child.
        if estimated_total_extent == trailing_scroll_offset {
            child_manager.set_did_underflow(app, true);
        }
        child_manager.did_finish_layout(app);
    }
}

impl RenderSliver for RenderSliverGrid {
    crate::render_sliver_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        RenderSliverMultiBoxAdaptor::hit_test_children(
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
        RenderSliverMultiBoxAdaptor::child_main_axis_position(self, app, child)
    }

    fn child_cross_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        child
            .parent_data_of::<SliverGridParentData>(app)
            .cross_axis_offset
            .expect("laid out")
    }

    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        RenderSliverMultiBoxAdaptor::child_scroll_offset(self, app, child)
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::Size;
    use inset_foundation::AppCell;
    use inset_gestures::HitTestResult;
    use inset_painting::AxisDirection;

    use super::*;
    use crate::sliver::GrowthDirection;
    use crate::sliver_multi_box_adaptor::test_support::TestChildManager;
    use crate::sliver_multi_box_adaptor::viewport_test_support::ScrollHarness;
    use crate::viewport_offset::ScrollDirection;

    const VIEWPORT: Size = Size::new(500.0, 200.0);

    fn sliver_grid(
        app: &mut App,
        count: usize,
        delegate: Rc<dyn SliverGridDelegate>,
    ) -> (
        RenderHandle<RenderSliverGrid>,
        Rc<TestChildManager<RenderSliverGrid>>,
        ScrollHarness,
    ) {
        let manager = TestChildManager::new(vec![1000.0; count]);
        let grid = RenderSliverGrid::new(app, manager.clone(), delegate);
        manager.attach(grid);
        let harness = ScrollHarness::new(app, grid.as_sliver(), VIEWPORT);
        (grid, manager, harness)
    }

    /// Vertical constraints `cross_axis_extent` wide, at the top of a 600 high viewport.
    fn constraints(cross_axis_extent: f64) -> SliverConstraints {
        SliverConstraints::new(
            AxisDirection::Down,
            GrowthDirection::Forward,
            ScrollDirection::Idle,
            0.0,
            0.0,
            0.0,
            600.0,
            cross_axis_extent,
            AxisDirection::Right,
            600.0,
            1100.0,
            0.0,
        )
    }

    /// Where each child is laid out: its index, scroll offset, cross-axis offset and size.
    fn placed(app: &App, grid: RenderHandle<RenderSliverGrid>) -> Vec<(i32, f64, f64, Size)> {
        let mut placed = Vec::new();
        let mut child = grid.first_child(app);
        while let Some(current) = child {
            let data = current
                .as_object()
                .parent_data_of::<SliverGridParentData>(app);
            placed.push((
                data.adaptor.index.expect("indexed"),
                data.adaptor.layout_offset().expect("laid out"),
                data.cross_axis_offset.expect("laid out"),
                current.size(app),
            ));
            child = grid.child_after(app, current);
        }
        placed
    }

    /// `sliver_grid_test.dart`: the max-extent delegate fits as many columns as it can, each as
    /// wide as an even split allows.
    #[test]
    fn a_max_extent_grid_fits_as_many_columns_as_it_can() {
        let delegate = SliverGridDelegateWithMaxCrossAxisExtent::new(150.0);
        let layout = delegate.get_layout(constraints(500.0));
        let geometry = layout.get_geometry_for_child_index(5);
        assert_eq!(geometry.cross_axis_extent, 125.0, "4 columns of 125 in 500");
        assert_eq!(geometry.cross_axis_offset, 125.0);
        assert_eq!(geometry.scroll_offset, 125.0, "the second row");
        assert_eq!(layout.compute_max_scroll_offset(9), 375.0, "3 rows");
        assert_eq!(layout.compute_max_scroll_offset(0), 0.0);
    }

    /// A grid whose cross axis runs right to left places its first column on the right.
    #[test]
    fn a_reversed_cross_axis_places_the_first_column_last() {
        let mut leftward = constraints(400.0);
        leftward.cross_axis_direction = AxisDirection::Left;
        let layout = SliverGridDelegateWithFixedCrossAxisCount::new(4)
            .cross_axis_spacing(20.0)
            .get_layout(leftward);
        // Columns are 85 wide with 20 between them: the first sits at 400 - 85.
        assert_eq!(
            layout.get_geometry_for_child_index(0).cross_axis_offset,
            315.0
        );
        assert_eq!(
            layout.get_geometry_for_child_index(3).cross_axis_offset,
            0.0
        );
    }

    #[test]
    fn a_grid_lays_out_only_the_rows_it_shows_with_their_spacing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = SliverGridDelegateWithFixedCrossAxisCount::new(2)
            .main_axis_spacing(10.0)
            .cross_axis_spacing(20.0)
            .child_aspect_ratio(2.0);
        let (grid, manager, _harness) = sliver_grid(&mut app, 100, Rc::new(delegate));
        // Each tile is (500 - 20) / 2 = 240 wide and 120 tall, so rows are 130 apart: the 200
        // high viewport and its 250 of cache reach into the fourth row.
        let placed = placed(&app, grid);
        assert_eq!(
            placed[..3],
            [
                (0, 0.0, 0.0, Size::new(240.0, 120.0)),
                (1, 0.0, 260.0, Size::new(240.0, 120.0)),
                (2, 130.0, 0.0, Size::new(240.0, 120.0)),
            ]
        );
        assert_eq!(placed.len(), 8, "four rows of two");
        assert!(manager.created.borrow().iter().all(|index| *index < 8));
    }

    #[test]
    fn scrolling_a_grid_replaces_its_rows() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = SliverGridDelegateWithFixedCrossAxisCount::new(4).main_axis_extent(100.0);
        let (grid, _, harness) = sliver_grid(&mut app, 100, Rc::new(delegate));
        harness.scroll_to(&mut app, 1000.0);
        let placed = placed(&app, grid);
        // The cache reaches 250 before the scroll offset, to 750: row 7, which starts at 700.
        assert_eq!((placed[0].0, placed[0].1), (28, 700.0));
        assert!(placed.iter().all(|(index, ..)| *index >= 28));
        assert!(grid.geometry(&app).has_visual_overflow);
    }

    #[test]
    fn a_short_grid_reports_its_exact_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = SliverGridDelegateWithFixedCrossAxisCount::new(4).main_axis_extent(50.0);
        let (grid, manager, _harness) = sliver_grid(&mut app, 6, Rc::new(delegate));
        let geometry = grid.geometry(&app);
        assert_eq!(geometry.scroll_extent, 100.0, "two rows of 50");
        assert_eq!(geometry.paint_extent, 100.0);
        assert!(!geometry.has_visual_overflow);
        assert!(manager.did_underflow.get());
    }

    /// A hit lands on the tile under it, which only its cross-axis offset tells apart from the
    /// others in its row.
    #[test]
    fn a_grid_hit_tests_the_tile_under_the_point() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = SliverGridDelegateWithFixedCrossAxisCount::new(5).main_axis_extent(100.0);
        let (grid, _, _harness) = sliver_grid(&mut app, 20, Rc::new(delegate));
        let tile = placed(&app, grid)
            .iter()
            .position(|(index, ..)| *index == 8)
            .expect("tile 8 is laid out");
        let mut child = grid.first_child(&app).expect("tiles");
        for _ in 0..tile {
            child = grid.child_after(&app, child).expect("tile");
        }
        assert_eq!(
            RenderSliver::child_cross_axis_position(grid, &app, child.as_object()),
            300.0
        );

        let mut result = HitTestResult::new();
        let hit = grid.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut result),
            150.0,
            350.0,
        );
        assert!(hit);
        assert_eq!(result.path().len(), 2, "tile 8 and the grid");
    }

    #[test]
    fn a_new_delegate_relays_out_only_when_it_changes_the_tiles() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = SliverGridDelegateWithFixedCrossAxisCount::new(2);
        let (grid, _, harness) = sliver_grid(&mut app, 10, Rc::new(delegate.clone()));
        grid.set_grid_delegate(&mut app, Rc::new(delegate.clone()));
        assert!(!grid.as_object().debug_needs_layout(&app), "the same tiles");
        grid.set_grid_delegate(&mut app, Rc::new(delegate.main_axis_spacing(4.0)));
        assert!(grid.as_object().debug_needs_layout(&app));
        harness.owner.flush_layout(&mut app);
        grid.set_grid_delegate(
            &mut app,
            Rc::new(SliverGridDelegateWithMaxCrossAxisExtent::new(250.0)),
        );
        assert!(
            grid.as_object().debug_needs_layout(&app),
            "another kind of delegate"
        );
    }
}
