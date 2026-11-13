//! Flutter counterpart: dart:ui `FlutterView`. The host owns the native
//! window; this stable handle exposes its identity and current configuration.

use crate::{
    Matrix4, Rect, ScreenId, Size, SystemMouseCursorKind, SystemUiOverlayStyle, TextEditingValue,
    TextInputConfiguration,
};

/// Opaque id for one [`View`] (`FlutterView.viewId`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ViewId(pub u64);

/// An event for the engine to communicate view focus changes to the app.
/// Flutter counterpart: `ViewFocusEvent` in `dart:ui/platform_dispatcher.dart`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ViewFocusState {
    /// The view does not have platform focus.
    Unfocused,
    /// The view has platform focus.
    Focused,
}

/// Represents the direction in which focus transitioned across views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ViewMetrics {
    pub physical_size: [f64; 2],
    pub physical_constraints: ViewConstraints,
    pub device_pixel_ratio: f64,
    pub padding: ViewPadding,
    pub view_padding: ViewPadding,
    pub view_insets: ViewPadding,
    /// The physical pixels on each side of the view that it can draw into but where the system
    /// takes input gestures for its navigation, as a swipe in from an edge goes back:
    /// `_ViewConfiguration.systemGestureInsets`, which `FlutterView.systemGestureInsets` reads.
    pub system_gesture_insets: ViewPadding,
    /// The screen the view is on, one of `Platform::screens`: `_ViewConfiguration.displayId`,
    /// which `FlutterView.display` looks up. `None` while it is on none the host can name, as a
    /// view without a window is.
    pub screen: Option<ScreenId>,
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
            system_gesture_insets: ViewPadding::ZERO,
            screen: None,
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
/// the surface; the framework builds a [`crate::Scene`] and hands it over.
pub trait View: 'static {
    fn id(&self) -> ViewId;
    fn metrics(&self) -> ViewMetrics;

    /// Flutter `FlutterView.gestureSettings`.
    fn gesture_settings(&self) -> GestureSettings {
        GestureSettings::default()
    }

    /// Renders and presents one scene, the frame's layer tree, shared so the host may keep it: to
    /// present it again when the window is seen again, and to tell a scene equal to the last from
    /// one worth drawing. Physical pixels; the presenter applies no extra scaling.
    /// [`slices`](crate::slices) makes of it the picture a host draws and the composition layers a
    /// host with a compositor places in the same native transaction; a host without one draws the
    /// picture alone.
    fn present(&self, scene: std::sync::Arc<crate::Scene>);

    /// What the view asks of the system: its input methods, its cursor and its focus.
    fn host(&self) -> &dyn ViewHost;

    /// What whoever gave or asked for this view attached to it, such as which screen to show:
    /// what the app asked for it with ([`crate::Platform::request_view`]), or what an embedder
    /// said, as Flutter web's `initialData` does. `None` for a view with none.
    fn initial_data(&self) -> Option<String> {
        None
    }

    /// The request this view arrived for ([`crate::Platform::request_view`],
    /// [`crate::Platform::add_view`]); `None` for a view the platform gave the app unasked.
    fn request_id(&self) -> Option<crate::RequestId> {
        None
    }

    /// The window this view is in, where the platform lets the app manage it, as `NSView.window`
    /// answers: every window on a desktop, whether the app asked for it or the platform gave it,
    /// such as WaOS's window around a program's view, and every activity's task on Android. `None`
    /// for a view in nothing the app may manage: an iPhone's screen, a web page's element, a host
    /// app's view. Closing a view is its
    /// window's: a view with none is not the app's to close, and its way out is to quit.
    fn window(&self) -> Option<crate::WindowRef> {
        None
    }
}

/// What one view asks of the system, provided by whoever hosts the view: a window's view gets its
/// window's. Chromium's `RenderWidgetHostView` and Android's `SurfaceControlViewHost` are this
/// piece; where Flutter's embedders take the same requests on the platform as a whole, through
/// `SystemChannels.textInput`, `SystemChannels.mouseCursor`, `SystemChannels.platform` and
/// `PlatformDispatcher.requestViewFocusChange`, here each view's own host takes them.
pub trait ViewHost {
    /// The input methods that edit text in this view; none where it takes no text.
    fn text_input(&self) -> Option<&dyn TextInputHost>;

    /// Shows a system cursor for a pointing device over this view: Flutter's
    /// `activateSystemCursor` message on `SystemChannels.mouseCursor`. A host with one cursor for
    /// every view shows it wherever the pointer is.
    fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind);

    /// Flutter's `PlatformDispatcher.requestViewFocusChange` for this view: asks the system to move
    /// the focus to it, or away from it. A host whose system has no focus to move ignores it.
    fn request_focus_change(&self, state: ViewFocusState, direction: ViewFocusDirection);

    /// Informs the host of whether or not the framework will handle back events in this view
    /// (Flutter `SystemNavigator.setFrameworkHandlesBack`): whether a navigator in the view has a
    /// route to pop. A host that hears Back only while the view said it handles it follows it, as
    /// Android's predictive back does, or one whose view cannot answer a press on the spot.
    ///
    /// The default drops it: a host that asks the app on the spot, through
    /// [`EmbedderClient::pop_route`](crate::EmbedderClient::pop_route), has the answer then and
    /// needs no notice in advance.
    fn set_framework_handles_back(&self, handles: bool) {
        let _ = handles;
    }

    /// Styles the system overlays the host draws over this view: the status bar, and on Android
    /// the system navigation bar (Flutter `SystemChrome.setSystemUIOverlayStyle`). Each window
    /// carries its own bars, so the style is the view's, as Flutter's Android embedding applies
    /// it to its one activity's window.
    ///
    /// The default drops it, for a host that draws no overlays over its views.
    fn set_overlay_style(&self, style: &SystemUiOverlayStyle) {
        let _ = style;
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

    /// Whether the host sends the view Edit's commands as events through a command of its own,
    /// which the user's ⌘C, ⌘X and ⌘V reach once the framework leaves them unhandled: macOS's Edit
    /// menu, or the host's of a program that runs inside another app. A text field then leaves
    /// those keys unhandled and acts on the `clipboard_event` the command sends, as Flutter's text
    /// fields on the web leave them to the browser.
    ///
    /// Defaults to false: a host whose system has no such command, or whose app may have set a
    /// menu bar without Edit's commands, keeps them in the field.
    fn provides_edit_commands(&self) -> bool {
        false
    }
}

/// What the app uses to embed views of its own, as Flutter's embedder does: adding one on a layer
/// it makes and places, giving its metrics and its being in sight, handing it input, and removing
/// it ([`Platform::view_embedder`](crate::Platform::view_embedder)).
pub trait ViewEmbedder {
    /// Flutter's `AddView`, for the app: asks for a view that draws into `layer`, which the app
    /// made with the platform's [`compositor`](crate::Platform::compositor), opaque or not, and
    /// places itself, at `metrics`, asking what it asks of the system of `host`. The view arrives
    /// as a window's does, through `EmbedderClient::view_added`, carrying the id answered here
    /// ([`View::request_id`]); it is in sight until the app says otherwise
    /// ([`set_view_in_sight`](Self::set_view_in_sight)), and its frames follow the screen its
    /// metrics name. Refused at once, with
    /// [`ViewError::Unsupported`](crate::ViewError::Unsupported), where the host cannot draw a
    /// view into `layer`.
    fn add_view(
        &self,
        layer: &std::sync::Arc<dyn crate::CompositionLayer>,
        metrics: ViewMetrics,
        host: std::rc::Rc<dyn ViewHost>,
    ) -> Result<crate::RequestId, crate::ViewError>;

    /// Flutter's `SendWindowMetricsEvent`, for a view the app added with
    /// [`add_view`](Self::add_view), which the framework hears of through
    /// `EmbedderClient::view_metrics_changed`; refused for any other view, whose holder gives it
    /// its metrics.
    fn set_view_metrics(&self, view: ViewId, metrics: ViewMetrics);

    /// Whether a view the app added is in sight, as only the app knows where it placed the view's
    /// layer: its frames are held while it is not, as a window's are while the window is
    /// minimized or occluded. Refused for any other view, as its metrics are.
    fn set_view_in_sight(&self, view: ViewId, in_sight: bool);

    /// Flutter's `RemoveView`, for a view the app added, refused for any other as its metrics
    /// are: the view lets go of its layer, and the framework hears of it through
    /// `EmbedderClient::view_removed`.
    fn remove_view(&self, view: ViewId);

    /// Flutter's `SendPointerEvent`, for the app: what pointing devices did over any view, a
    /// window's included, which the framework hears as it hears the host's own reports
    /// (`EmbedderClient::pointer_reports`), through the one converter that numbers every pointer.
    /// Each device of a forwarded stream (its mouse, trackpad and pen, and each touch from landing
    /// to lifting) takes a device number no other stream uses, a random 64-bit one for now, since
    /// the converter keeps one state per device number, a forwarding app's own host numbers its
    /// devices as this one does, 0, 1, 2 upward, and two streams with no order between them would
    /// mix their devices' states in one (native-surfaces decision 36).
    fn send_pointer_reports(&self, reports: Vec<crate::PointerReport>);

    /// Flutter's `SendViewFocusEvent`, for the app: a view's focus change, which the framework
    /// hears as it hears the host's own (`EmbedderClient::view_focus_changed`), so that the keys
    /// the host reports go where the framework's focus is.
    fn send_view_focus_event(&self, event: ViewFocusEvent);
}
