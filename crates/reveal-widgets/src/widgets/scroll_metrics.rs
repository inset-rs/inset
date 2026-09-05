//! Flutter counterpart: `widgets/scroll_metrics.dart`.

use std::fmt::{self, Debug};

use reveal_embedder::clamp_double;
use reveal_painting::{Axis, AxisDirection, axis_direction_to_axis};

/// A description of a `Scrollable`'s contents, useful for modeling the state
/// of its viewport.
///
/// This trait defines a current position, [`pixels`](Self::pixels), and a range of values
/// considered "in bounds" for that position. The range has a minimum value at
/// [`min_scroll_extent`](Self::min_scroll_extent) and a maximum value at
/// [`max_scroll_extent`](Self::max_scroll_extent) (inclusive). The viewport scrolls in the
/// direction and axis described by [`axis_direction`](Self::axis_direction) and
/// [`axis`](Self::axis).
///
/// The [`out_of_range`](Self::out_of_range) getter will return true if
/// [`pixels`](Self::pixels) is outside this defined range. The [`at_edge`](Self::at_edge)
/// getter will return true if the [`pixels`](Self::pixels) position equals either the
/// [`min_scroll_extent`](Self::min_scroll_extent) or the
/// [`max_scroll_extent`](Self::max_scroll_extent).
///
/// The dimensions of the viewport in the given [`axis`](Self::axis) are described by
/// [`viewport_dimension`](Self::viewport_dimension).
///
/// The above values are also exposed in terms of [`extent_before`](Self::extent_before),
/// [`extent_inside`](Self::extent_inside), and [`extent_after`](Self::extent_after), which
/// may be more useful for use cases such as scroll bars.
///
/// See also:
///
///  * [`FixedScrollMetrics`], which is an immutable object that implements this
///    interface.
pub trait ScrollMetrics: Debug {
    /// Creates a [`ScrollMetrics`] that has the same properties as this object.
    ///
    /// This is useful if this object is mutable, but you want to get a snapshot
    /// of the current state.
    ///
    /// The `with_*` setters on the result allow the values to be adjusted in the process.
    /// This is useful to examine hypothetical situations, for example "would applying this
    /// delta unmodified take the position [`out_of_range`](Self::out_of_range)?".
    fn copy_with(&self) -> FixedScrollMetrics {
        FixedScrollMetrics::new(
            self.has_content_dimensions()
                .then(|| self.min_scroll_extent()),
            self.has_content_dimensions()
                .then(|| self.max_scroll_extent()),
            self.has_pixels().then(|| self.pixels()),
            self.has_viewport_dimension()
                .then(|| self.viewport_dimension()),
            self.axis_direction(),
            self.device_pixel_ratio(),
        )
    }

    /// The minimum in-range value for [`pixels`](Self::pixels).
    ///
    /// The actual [`pixels`](Self::pixels) value might be
    /// [`out_of_range`](Self::out_of_range).
    ///
    /// This value is typically less than or equal to
    /// [`max_scroll_extent`](Self::max_scroll_extent). It can be negative infinity, if the
    /// scroll is unbounded.
    fn min_scroll_extent(&self) -> f64;

    /// The maximum in-range value for [`pixels`](Self::pixels).
    ///
    /// The actual [`pixels`](Self::pixels) value might be
    /// [`out_of_range`](Self::out_of_range).
    ///
    /// This value is typically greater than or equal to
    /// [`min_scroll_extent`](Self::min_scroll_extent). It can be infinity, if the scroll is
    /// unbounded.
    ///
    /// For scrollables that lazily construct their contents, such as
    /// `ListView.builder`, this value can be an estimate that changes as more
    /// children are laid out.
    fn max_scroll_extent(&self) -> f64;

    /// Whether the [`min_scroll_extent`](Self::min_scroll_extent) and the
    /// [`max_scroll_extent`](Self::max_scroll_extent) properties are available.
    fn has_content_dimensions(&self) -> bool;

    /// The current scroll position, in logical pixels along the
    /// [`axis_direction`](Self::axis_direction).
    fn pixels(&self) -> f64;

    /// Whether the [`pixels`](Self::pixels) property is available.
    fn has_pixels(&self) -> bool;

    /// The extent of the viewport along the [`axis_direction`](Self::axis_direction).
    fn viewport_dimension(&self) -> f64;

    /// Whether the [`viewport_dimension`](Self::viewport_dimension) property is available.
    fn has_viewport_dimension(&self) -> bool;

    /// The direction in which the scroll view scrolls.
    fn axis_direction(&self) -> AxisDirection;

    /// The axis in which the scroll view scrolls.
    fn axis(&self) -> Axis {
        axis_direction_to_axis(self.axis_direction())
    }

    /// Whether the [`pixels`](Self::pixels) value is outside the
    /// [`min_scroll_extent`](Self::min_scroll_extent) and
    /// [`max_scroll_extent`](Self::max_scroll_extent).
    fn out_of_range(&self) -> bool {
        self.pixels() < self.min_scroll_extent() || self.pixels() > self.max_scroll_extent()
    }

    /// Whether the [`pixels`](Self::pixels) value is exactly at the
    /// [`min_scroll_extent`](Self::min_scroll_extent) or the
    /// [`max_scroll_extent`](Self::max_scroll_extent).
    fn at_edge(&self) -> bool {
        self.pixels() == self.min_scroll_extent() || self.pixels() == self.max_scroll_extent()
    }

    /// The quantity of content conceptually "above" the viewport in the scrollable.
    /// This is the content above the content described by
    /// [`extent_inside`](Self::extent_inside).
    fn extent_before(&self) -> f64 {
        (self.pixels() - self.min_scroll_extent()).max(0.0)
    }

    /// The quantity of content conceptually "inside" the viewport in the
    /// scrollable (including empty space if the total amount of content is less
    /// than the [`viewport_dimension`](Self::viewport_dimension)).
    ///
    /// The value is typically the extent of the viewport
    /// ([`viewport_dimension`](Self::viewport_dimension)) when
    /// [`out_of_range`](Self::out_of_range) is false. It can be less when overscrolling.
    ///
    /// The value is always non-negative, and less than or equal to
    /// [`viewport_dimension`](Self::viewport_dimension).
    fn extent_inside(&self) -> f64 {
        debug_assert!(self.min_scroll_extent() <= self.max_scroll_extent());
        self.viewport_dimension()
            // "above" overscroll value
            - clamp_double(
                self.min_scroll_extent() - self.pixels(),
                0.0,
                self.viewport_dimension(),
            )
            // "below" overscroll value
            - clamp_double(
                self.pixels() - self.max_scroll_extent(),
                0.0,
                self.viewport_dimension(),
            )
    }

    /// The quantity of content conceptually "below" the viewport in the scrollable.
    /// This is the content below the content described by
    /// [`extent_inside`](Self::extent_inside).
    fn extent_after(&self) -> f64 {
        (self.max_scroll_extent() - self.pixels()).max(0.0)
    }

    /// The total quantity of content available.
    ///
    /// This is the sum of [`extent_before`](Self::extent_before),
    /// [`extent_inside`](Self::extent_inside), and [`extent_after`](Self::extent_after),
    /// modulo any rounding errors.
    fn extent_total(&self) -> f64 {
        self.max_scroll_extent() - self.min_scroll_extent() + self.viewport_dimension()
    }

    /// The `FlutterView.devicePixelRatio` of the view that the `Scrollable`
    /// associated with this metrics object is drawn into.
    fn device_pixel_ratio(&self) -> f64;
}

/// An immutable snapshot of values associated with a `Scrollable` viewport.
///
/// For details, see [`ScrollMetrics`], which defines this object's interfaces.
#[derive(Clone, Copy)]
pub struct FixedScrollMetrics {
    min_scroll_extent: Option<f64>,
    max_scroll_extent: Option<f64>,
    pixels: Option<f64>,
    viewport_dimension: Option<f64>,
    axis_direction: AxisDirection,
    device_pixel_ratio: f64,
}

impl FixedScrollMetrics {
    /// Creates an immutable snapshot of values associated with a `Scrollable` viewport.
    pub fn new(
        min_scroll_extent: Option<f64>,
        max_scroll_extent: Option<f64>,
        pixels: Option<f64>,
        viewport_dimension: Option<f64>,
        axis_direction: AxisDirection,
        device_pixel_ratio: f64,
    ) -> FixedScrollMetrics {
        FixedScrollMetrics {
            min_scroll_extent,
            max_scroll_extent,
            pixels,
            viewport_dimension,
            axis_direction,
            device_pixel_ratio,
        }
    }

    /// Dart `copyWith(minScrollExtent:)`.
    pub fn with_min_scroll_extent(mut self, min_scroll_extent: f64) -> FixedScrollMetrics {
        self.min_scroll_extent = Some(min_scroll_extent);
        self
    }

    /// Dart `copyWith(maxScrollExtent:)`.
    pub fn with_max_scroll_extent(mut self, max_scroll_extent: f64) -> FixedScrollMetrics {
        self.max_scroll_extent = Some(max_scroll_extent);
        self
    }

    /// Dart `copyWith(pixels:)`.
    pub fn with_pixels(mut self, pixels: f64) -> FixedScrollMetrics {
        self.pixels = Some(pixels);
        self
    }

    /// Dart `copyWith(viewportDimension:)`.
    pub fn with_viewport_dimension(mut self, viewport_dimension: f64) -> FixedScrollMetrics {
        self.viewport_dimension = Some(viewport_dimension);
        self
    }

    /// Dart `copyWith(axisDirection:)`.
    pub fn with_axis_direction(mut self, axis_direction: AxisDirection) -> FixedScrollMetrics {
        self.axis_direction = axis_direction;
        self
    }

    /// Dart `copyWith(devicePixelRatio:)`.
    pub fn with_device_pixel_ratio(mut self, device_pixel_ratio: f64) -> FixedScrollMetrics {
        self.device_pixel_ratio = device_pixel_ratio;
        self
    }
}

impl ScrollMetrics for FixedScrollMetrics {
    fn min_scroll_extent(&self) -> f64 {
        self.min_scroll_extent
            .expect("FixedScrollMetrics.minScrollExtent was read without content dimensions")
    }

    fn max_scroll_extent(&self) -> f64 {
        self.max_scroll_extent
            .expect("FixedScrollMetrics.maxScrollExtent was read without content dimensions")
    }

    fn has_content_dimensions(&self) -> bool {
        self.min_scroll_extent.is_some() && self.max_scroll_extent.is_some()
    }

    fn pixels(&self) -> f64 {
        self.pixels
            .expect("FixedScrollMetrics.pixels was read before it was available")
    }

    fn has_pixels(&self) -> bool {
        self.pixels.is_some()
    }

    fn viewport_dimension(&self) -> f64 {
        self.viewport_dimension
            .expect("FixedScrollMetrics.viewportDimension was read before it was available")
    }

    fn has_viewport_dimension(&self) -> bool {
        self.viewport_dimension.is_some()
    }

    fn axis_direction(&self) -> AxisDirection {
        self.axis_direction
    }

    fn device_pixel_ratio(&self) -> f64 {
        self.device_pixel_ratio
    }
}

impl Debug for FixedScrollMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FixedScrollMetrics({:.1}..[{:.1}]..{:.1})",
            self.extent_before(),
            self.extent_inside(),
            self.extent_after()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overscrolled() -> FixedScrollMetrics {
        FixedScrollMetrics::new(
            Some(0.0),
            Some(400.0),
            Some(-20.0),
            Some(600.0),
            AxisDirection::Down,
            3.0,
        )
    }

    #[test]
    fn out_of_range_and_at_edge_read_the_pixels_against_the_extents() {
        let metrics = overscrolled();
        assert!(metrics.out_of_range());
        assert!(!metrics.at_edge());

        let at_min = metrics.copy_with().with_pixels(0.0);
        assert!(!at_min.out_of_range());
        assert!(at_min.at_edge());

        let at_max = metrics.copy_with().with_pixels(400.0);
        assert!(!at_max.out_of_range());
        assert!(at_max.at_edge());

        let inside = metrics.copy_with().with_pixels(100.0);
        assert!(!inside.out_of_range());
        assert!(!inside.at_edge());
    }

    #[test]
    fn the_extents_split_the_content_around_the_viewport() {
        let metrics = overscrolled();
        assert_eq!(metrics.extent_before(), 0.0);
        assert_eq!(metrics.extent_inside(), 580.0);
        assert_eq!(metrics.extent_after(), 420.0);
        assert_eq!(metrics.extent_total(), 1000.0);
        assert_eq!(metrics.axis(), Axis::Vertical);
        assert_eq!(
            format!("{metrics:?}"),
            "FixedScrollMetrics(0.0..[580.0]..420.0)"
        );

        let inside = metrics.copy_with().with_pixels(100.0);
        assert_eq!(inside.extent_before(), 100.0);
        assert_eq!(inside.extent_inside(), 600.0);
        assert_eq!(inside.extent_after(), 300.0);
    }

    #[test]
    fn copy_with_carries_the_absent_values_across() {
        let empty = FixedScrollMetrics::new(None, None, None, None, AxisDirection::Right, 2.0);
        assert!(!empty.has_content_dimensions());
        assert!(!empty.has_pixels());
        assert!(!empty.has_viewport_dimension());
        assert_eq!(empty.axis(), Axis::Horizontal);

        let copy = empty.copy_with().with_pixels(7.0);
        assert!(!copy.has_content_dimensions());
        assert_eq!(copy.pixels(), 7.0);
        assert_eq!(copy.device_pixel_ratio(), 2.0);

        let full = overscrolled().copy_with();
        assert!(full.has_content_dimensions());
        assert!(full.has_pixels());
        assert!(full.has_viewport_dimension());
    }
}
