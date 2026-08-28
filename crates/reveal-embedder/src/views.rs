//! Flutter counterpart: dart:ui `FlutterView`. The host owns the native
//! window; this stable handle exposes its identity and current configuration.

/// Opaque id for one [`View`] (`FlutterView.viewId`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewId(pub u64);

/// Physical padding on each side of a view (`dart:ui` `ViewPadding`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewPadding {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl ViewPadding {
    /// A view padding that has zeros for each edge.
    pub const ZERO: ViewPadding = ViewPadding {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
}

/// Immutable layout constraints for a view (`dart:ui` `ViewConstraints`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewConstraints {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl ViewConstraints {
    pub const fn new(
        min_width: f64,
        max_width: f64,
        min_height: f64,
        max_height: f64,
    ) -> ViewConstraints {
        ViewConstraints {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Constraints respected only by the given size.
    pub const fn tight(width: f64, height: f64) -> ViewConstraints {
        ViewConstraints {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }
}

impl Default for ViewConstraints {
    fn default() -> ViewConstraints {
        ViewConstraints {
            min_width: 0.0,
            max_width: f64::INFINITY,
            min_height: 0.0,
            max_height: f64::INFINITY,
        }
    }
}

/// Per-view geometry the host reports (`_ViewConfiguration` fields we use).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewMetrics {
    pub physical_size: [f64; 2],
    pub physical_constraints: ViewConstraints,
    pub device_pixel_ratio: f64,
    pub padding: ViewPadding,
    pub view_padding: ViewPadding,
    pub view_insets: ViewPadding,
}

impl Default for ViewMetrics {
    fn default() -> ViewMetrics {
        ViewMetrics {
            physical_size: [0.0, 0.0],
            physical_constraints: ViewConstraints::tight(0.0, 0.0),
            device_pixel_ratio: 1.0,
            padding: ViewPadding::ZERO,
            view_padding: ViewPadding::ZERO,
            view_insets: ViewPadding::ZERO,
        }
    }
}

/// A stable handle to one host-provided view.
///
/// Rendering joins this interface when the real display-list type and
/// presenter exist. Until then this type reports view state only.
pub trait View: 'static {
    fn id(&self) -> ViewId;
    fn metrics(&self) -> ViewMetrics;
}
