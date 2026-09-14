//! Flutter counterpart: `gestures/gesture_settings.dart`.

use std::fmt::{self, Debug, Display};

use inset_embedder::View;

/// The device specific gesture settings scaled into logical pixels.
///
/// This configuration can be retrieved from the window, or more commonly from a
/// `MediaQuery` widget.
///
/// See also:
///
///  * [`inset_embedder::GestureSettings`], the configuration that this is derived from.
#[derive(Clone, Copy, PartialEq)]
pub struct DeviceGestureSettings {
    /// The touch slop value in logical pixels, or `None` if it was not set.
    pub touch_slop: Option<f64>,
}

impl DeviceGestureSettings {
    /// Create a new [`DeviceGestureSettings`] with configured settings in logical
    /// pixels.
    pub const fn new(touch_slop: Option<f64>) -> DeviceGestureSettings {
        DeviceGestureSettings { touch_slop }
    }

    /// Create a new [`DeviceGestureSettings`] from the provided view.
    pub fn from_view(view: &dyn View) -> DeviceGestureSettings {
        let physical_touch_slop = view.gesture_settings().physical_touch_slop;
        DeviceGestureSettings {
            touch_slop: physical_touch_slop
                .map(|physical_touch_slop| physical_touch_slop / view.metrics().device_pixel_ratio),
        }
    }

    /// The touch slop value for pan gestures, in logical pixels, or `None` if it
    /// was not set.
    pub fn pan_slop(self) -> Option<f64> {
        self.touch_slop.map(|touch_slop| touch_slop * 2.0)
    }
}

impl Debug for DeviceGestureSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for DeviceGestureSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DeviceGestureSettings(touchSlop: {})",
            match self.touch_slop {
                Some(value) => value.to_string(),
                None => "null".to_owned(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::{GestureSettings, View, ViewId, ViewMetrics};

    use super::DeviceGestureSettings;

    struct SettingsView {
        metrics: ViewMetrics,
        gesture_settings: GestureSettings,
    }

    impl View for SettingsView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            self.metrics
        }

        fn gesture_settings(&self) -> GestureSettings {
            self.gesture_settings
        }

        fn present(&self, _picture: std::sync::Arc<inset_embedder::Picture>) {}
    }

    #[test]
    fn from_view_divides_physical_slop_by_device_pixel_ratio() {
        let view = SettingsView {
            metrics: ViewMetrics {
                device_pixel_ratio: 2.0,
                ..ViewMetrics::default()
            },
            gesture_settings: GestureSettings {
                physical_touch_slop: Some(36.0),
                physical_double_tap_slop: None,
            },
        };
        let settings = DeviceGestureSettings::from_view(&view);
        assert_eq!(settings.touch_slop, Some(18.0));
        assert_eq!(settings.pan_slop(), Some(36.0));
    }

    #[test]
    fn from_view_leaves_unset_slop_unset() {
        let view = SettingsView {
            metrics: ViewMetrics::default(),
            gesture_settings: GestureSettings::default(),
        };
        assert!(DeviceGestureSettings::from_view(&view).touch_slop.is_none());
    }
}
