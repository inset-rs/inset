//! Flutter counterpart: dart:ui `FlutterView`. The host owns the native
//! window; this stable handle exposes its identity and current configuration.

use crate::{Matrix4, Rect, Size, TextEditingValue, TextInputConfiguration};

/// Opaque id for one [`View`] (`FlutterView.viewId`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewId(pub u64);

/// An event for the engine to communicate view focus changes to the app.
/// Flutter counterpart: `ViewFocusEvent` in `dart:ui/platform_dispatcher.dart`.
#[derive(Clone, Copy, Debug)]
pub struct ViewFocusEvent {
    /// The ID of the view that experienced a focus change.
    pub view_id: ViewId,
    /// The state focus changed to.
    pub state: ViewFocusState,
    /// The direction focus changed to.
    pub direction: ViewFocusDirection,
}

/// Represents the focus state of a given view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFocusState {
    /// The view does not have platform focus.
    Unfocused,
    /// The view has platform focus.
    Focused,
}

/// Represents the direction in which focus transitioned across views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFocusDirection {
    /// The transition did not have a direction, such as a programmatic request or focus loss.
    Undefined,
    /// The transition was forward, typically from pressing Tab.
    Forward,
    /// The transition was backward, typically from pressing Shift+Tab.
    Backward,
}

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

/// A `null` field indicates that the platform or view does not have a preference
/// and the fallback constants should be used instead.
///
/// Flutter counterpart: `GestureSettings` (`dart:ui` `window.dart`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GestureSettings {
    /// The number of physical pixels a pointer is allowed to drift before it is
    /// considered an intentional movement.
    ///
    /// If `None`, the framework's default touch slop configuration should be used
    /// instead.
    pub physical_touch_slop: Option<f64>,

    /// The number of physical pixels that the first and second tap of a double tap
    /// can drift apart to still be recognized as a double tap.
    ///
    /// If `None`, the framework's default double tap slop configuration should be used
    /// instead.
    pub physical_double_tap_slop: Option<f64>,
}

impl GestureSettings {
    /// Create a new [`GestureSettings`] object from an existing value, overwriting
    /// all of the provided fields.
    pub fn copy_with(
        self,
        physical_touch_slop: Option<f64>,
        physical_double_tap_slop: Option<f64>,
    ) -> GestureSettings {
        GestureSettings {
            physical_touch_slop: physical_touch_slop.or(self.physical_touch_slop),
            physical_double_tap_slop: physical_double_tap_slop.or(self.physical_double_tap_slop),
        }
    }
}

/// A stable handle to one host-provided view.
///
/// [`present`](View::present) is Flutter `FlutterView.render`. The host owns
/// the surface; the framework records a [`crate::Picture`] and hands it over.
pub trait View: 'static {
    fn id(&self) -> ViewId;
    fn metrics(&self) -> ViewMetrics;

    /// Flutter `FlutterView.gestureSettings`.
    fn gesture_settings(&self) -> GestureSettings {
        GestureSettings::default()
    }

    /// Renders and presents one picture, shared so the host may keep it: to present it
    /// again when the window is seen again, and to tell a picture equal to the last from
    /// one worth drawing. Physical pixels; the presenter applies no extra scaling.
    fn present(&self, picture: std::sync::Arc<crate::Picture>);

    /// The view's text input, where the host has one.
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        None
    }
}

/// The host side of Flutter's text input connection for one view.
pub trait TextInputHost {
    /// Flutter `TextInput.attach` / `TextInputConnection.show`: start an IME
    /// session for this view.
    fn start(&self, configuration: &TextInputConfiguration);

    /// Flutter `TextInputConnection.close`: end the IME session.
    fn stop(&self);

    /// Flutter `TextInputConnection.setEditingState`.
    fn set_editing_state(&self, value: &TextEditingValue);

    /// Flutter `TextInput.setComposingRect`.
    fn set_composing_rect(&self, rect: Rect);

    /// Flutter `TextInput.setCaretRect`.
    fn set_caret_rect(&self, rect: Rect);

    /// Flutter `TextInput.setEditableSizeAndTransform`.
    fn set_client_geometry(&self, size: Size, transform: &Matrix4);

    /// Whether this text input turns editing keys into edits before the framework sees
    /// them: backspace and delete, caret movement, the line and document ends.
    ///
    /// Flutter has no such question because each of its embedders is written for one host:
    /// its macOS and iOS embedders do interpret those keys, and its text field bindings for
    /// those platforms step aside so a key is not acted on twice.
    ///
    /// Defaults to false, which is what a host built on a windowing library that reports
    /// plain key presses should answer, whichever platform it runs on. A host that hands the
    /// framework the operating system's own editing commands answers true, and the field
    /// then leaves those keys to it.
    fn handles_editing_keys(&self) -> bool {
        false
    }
}
