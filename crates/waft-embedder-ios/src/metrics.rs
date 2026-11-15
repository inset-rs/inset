//! A view's metrics from what UIKit says of its view, as Flutter's `FlutterViewController` sets its
//! viewport metrics (`setViewportMetricsSize`, `setViewportMetricsPaddings`) and its keyboard inset
//! manager the bottom inset: the view's bounds in physical pixels, its safe area as the padding the
//! system's bars and the home indicator take, and the docked keyboard as an inset.

use waft_embedder::{ScreenId, ViewConstraints, ViewMetrics, ViewPadding};

/// What UIKit says of a view this turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Geometry {
    /// The view's bounds, in points.
    pub(crate) size: [f64; 2],
    /// Physical pixels per point, the screen's scale.
    pub(crate) scale: f64,
    /// The view's safe area insets, in points: what the status bar, the sensor housing and the
    /// home indicator cover.
    pub(crate) safe_area: ViewPadding,
    /// How much of the view's bottom the docked keyboard covers, in points.
    pub(crate) keyboard_inset: f64,
    /// The screen the view is on.
    pub(crate) screen: Option<ScreenId>,
}

impl Geometry {
    /// The view's metrics: its whole bounds, with Flutter's three paddings of it; iOS has no system
    /// gesture insets Flutter reports.
    pub(crate) fn metrics(self) -> ViewMetrics {
        let [width, height] = self.size.map(|side| side * self.scale);
        let view_padding = physical(self.safe_area, self.scale);
        let view_insets = ViewPadding {
            bottom: self.keyboard_inset * self.scale,
            ..ViewPadding::ZERO
        };
        ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio: self.scale,
            padding: clear_of(view_padding, view_insets),
            view_padding,
            view_insets,
            system_gesture_insets: ViewPadding::ZERO,
            screen: self.screen,
        }
    }
}

/// `insets` in points as physical pixels at `scale`.
fn physical(insets: ViewPadding, scale: f64) -> ViewPadding {
    ViewPadding {
        left: insets.left * scale,
        top: insets.top * scale,
        right: insets.right * scale,
        bottom: insets.bottom * scale,
    }
}

/// `padding` as dart:ui makes it of the other two (`_buildViewConfiguration`, `hooks.dart`): what
/// of the system's bars the keyboard does not already cover.
fn clear_of(view_padding: ViewPadding, view_insets: ViewPadding) -> ViewPadding {
    ViewPadding {
        left: (view_padding.left - view_insets.left).max(0.0),
        top: (view_padding.top - view_insets.top).max(0.0),
        right: (view_padding.right - view_insets.right).max(0.0),
        bottom: (view_padding.bottom - view_insets.bottom).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An iPhone 16 in portrait: 393 by 852 points at three pixels a point, with the Dynamic
    /// Island's 59 points above and the home indicator's 34 below.
    fn phone() -> Geometry {
        Geometry {
            size: [393.0, 852.0],
            scale: 3.0,
            safe_area: ViewPadding {
                left: 0.0,
                top: 59.0,
                right: 0.0,
                bottom: 34.0,
            },
            keyboard_inset: 0.0,
            screen: None,
        }
    }

    #[test]
    fn the_bounds_and_safe_area_are_in_physical_pixels() {
        let metrics = phone().metrics();
        assert_eq!(metrics.physical_size, [1179.0, 2556.0]);
        assert_eq!(metrics.device_pixel_ratio, 3.0);
        assert_eq!(metrics.view_padding.top, 177.0);
        assert_eq!(metrics.view_padding.bottom, 102.0);
        assert_eq!(metrics.padding, metrics.view_padding);
        assert_eq!(metrics.view_insets, ViewPadding::ZERO);
    }

    #[test]
    fn the_keyboard_is_an_inset_that_takes_the_bottom_padding_it_covers() {
        let metrics = Geometry {
            keyboard_inset: 336.0,
            ..phone()
        }
        .metrics();
        assert_eq!(metrics.view_insets.bottom, 1008.0);
        assert_eq!(metrics.view_padding.bottom, 102.0);
        assert_eq!(metrics.padding.bottom, 0.0);
        assert_eq!(metrics.padding.top, 177.0);
    }
}
