//! Flutter counterpart: `widgets/text_selection.dart`
//! (`ClipboardStatus` / `ClipboardStatusNotifier`, `LiveTextInputStatus` /
//! `LiveTextInputStatusNotifier`, `TextSelectionHandleType`,
//! `TextSelectionControls`, `TextSelectionHandleControls`,
//! `TextSelectionOverlay`, `SelectionOverlay`, `_SelectionToolbarWrapper`,
//! `TextSelectionGestureDetectorBuilderDelegate`,
//! `TextSelectionGestureDetectorBuilder`, `TextSelectionGestureDetector`).

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{AnimationBehavior, AnimationController};
use reveal_embedder::{
    Offset, PointerDeviceKind, Rect, Size, TargetPlatform, TextAffinity, TextDirection,
    TextEditingValue, TextPosition, TextRange, TextSelection,
};
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, K_IS_WEB, Listenable, Listener,
    ValueChanged, ValueListenable, ValueNotifier,
};
use reveal_gestures::{
    BaseTapAndDragGestureRecognizer, DragEndDetails, DragStartBehavior, DragStartDetails,
    DragUpdateDetails, ForcePressDetails, ForcePressGestureRecognizer, GestureCancelCallback,
    GestureForcePressEndCallback, GestureForcePressStartCallback, GestureLongPressCancelCallback,
    GestureLongPressEndCallback, GestureLongPressMoveUpdateCallback, GestureLongPressStartCallback,
    GestureTapCallback, GestureTapDownCallback, GestureTapDragDownCallback,
    GestureTapDragEndCallback, GestureTapDragStartCallback, GestureTapDragUpCallback,
    GestureTapDragUpdateCallback, LongPressEndDetails, LongPressGestureRecognizer,
    LongPressMoveUpdateDetails, LongPressStartDetails, TapAndHorizontalDragGestureRecognizer,
    TapAndPanGestureRecognizer, TapDownDetails, TapDragDownDetails, TapDragEndDetails,
    TapDragStartDetails, TapDragUpDetails, TapDragUpdateDetails, TapGestureRecognizer,
    TapStatusTracker,
};
use reveal_painting::{Axis, AxisDirection, axis_direction_to_axis, transform_rect};
use reveal_rendering::{
    AnyRenderBox, BoxParentData, ContainerBoxParentData, ContainerParentData, HitTestBehavior,
    LayerLink, ParentData, RenderBox, RenderEditable, RenderHandle, TextSelectionPoint,
};
use reveal_scheduler::{
    FrameCallback, SchedulerBinding, SchedulerPhase, Ticker, TickerCallback, TickerProviderObject,
};
use reveal_services::{
    AnyTextSelectionDelegate, Clipboard, HapticFeedback, HardwareKeyboard, LogicalKeyboardKey,
    SelectionChangedCause, TextLayoutMetrics, TextSelectionDelegate,
};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::{
    CompositedTransformFollower, Directionality, SizedBox, WidgetBuilder,
};
use crate::widgets::editable_text::EditableTextState;
use crate::widgets::feedback::Feedback;
use crate::widgets::gesture_detector::{
    GestureRecognizerFactories, GestureRecognizerFactory, GestureRecognizerFactoryWithHandlers,
    RawGestureDetector,
};
use crate::widgets::magnifier::{MagnifierController, MagnifierInfo, TextMagnifierConfiguration};
use crate::widgets::overlay::{Overlay, OverlayEntry};
use crate::widgets::scrollable::Scrollable;
use crate::widgets::tap_region::TextFieldTapRegion;
use crate::widgets::ticker_provider::{
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
};
use crate::widgets::transitions::FadeTransition;

/// Which kind of selection handle to build.
///
/// Flutter: `rendering/selection.dart`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextSelectionHandleType {
    /// The selection handle is to the left of the selection end point.
    Left,
    /// The selection handle is to the right of the selection end point.
    Right,
    /// The start and end of the selection are co-incident at this point.
    Collapsed,
}

/// Parent data that determines whether or not to paint the corresponding child.
///
/// Used in the layout of the Cupertino and Material text selection menus, which
/// decide whether or not to paint their buttons after laying them out and
/// determining where they overflow.
#[derive(Debug)]
pub struct ToolbarItemsParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
    /// Whether or not this child is painted.
    ///
    /// Children in the selection toolbar may be laid out for measurement purposes
    /// but not painted. This allows these children to be identified.
    pub should_paint: bool,
}

impl ToolbarItemsParentData {
    /// Creates parent data that does not paint the child.
    pub const fn new() -> ToolbarItemsParentData {
        ToolbarItemsParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            should_paint: false,
        }
    }
}

impl Default for ToolbarItemsParentData {
    fn default() -> ToolbarItemsParentData {
        ToolbarItemsParentData::new()
    }
}

impl ParentData for ToolbarItemsParentData {
    fn detach(&mut self) {
        reveal_rendering::ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<ToolbarItemsParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<ToolbarItemsParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl reveal_rendering::ContainerParentDataMixin for ToolbarItemsParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for ToolbarItemsParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for ToolbarItemsParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; shouldPaint={}",
            self.box_parent_data, self.should_paint
        )
    }
}

/// An enumeration of the status of the content on the user's clipboard.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardStatus {
    /// The clipboard content can be pasted, such as a String of nonzero length.
    Pasteable,
    /// The status of the clipboard is unknown. Since getting clipboard data is
    /// asynchronous (see [`Clipboard::get_data`]), this status often exists while
    /// waiting to receive the clipboard contents for the first time.
    Unknown,
    /// The content on the clipboard is not pasteable, such as when it is empty.
    NotPasteable,
}

/// An enumeration that indicates whether the current device is available for Live Text input.
///
/// See also:
///
///  * `LiveText`, where the availability of Live Text input can be obtained.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LiveTextInputStatus {
    /// This device supports Live Text input currently.
    Enabled,
    /// The status of the Live Text input is unknown. Since getting the Live Text input availability
    /// is asynchronous (see `LiveText.isLiveTextInputAvailable`), this status often exists while
    /// waiting to receive the status value for the first time.
    Unknown,
    /// The current device doesn't support Live Text input.
    Disabled,
}

/// A [`ChangeNotifier`] whose value indicates whether the current contents of
/// the clipboard can be pasted.
///
/// Dart's `ValueNotifier<ClipboardStatus>` superclass is the `change_notifier` and `value`
/// fields here. Call [`update`](Self::update) to refresh [`value`](Self::value) from the
/// clipboard; [`Clipboard::has_strings`] is synchronous, so this is too.
pub struct ClipboardStatusNotifier {
    change_notifier: ChangeNotifierData,
    value: ClipboardStatus,
    disposed: bool,
    observer: Option<WidgetsBindingObserverRef>,
    /// Dart's `_WebClipboardStatusNotifier`: [`update`](Self::update) is a no-op and the
    /// value stays [`ClipboardStatus::Pasteable`].
    web: bool,
}

impl Debug for ClipboardStatusNotifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipboardStatusNotifier")
            .field("value", &self.value)
            .finish()
    }
}

impl ClipboardStatusNotifier {
    /// Create a new ClipboardStatusNotifier.
    pub fn new(app: &mut App) -> Handle<ClipboardStatusNotifier> {
        Self::from_value(app, ClipboardStatus::Unknown)
    }

    /// Dart `ClipboardStatusNotifier(value:)`.
    pub fn from_value(app: &mut App, value: ClipboardStatus) -> Handle<ClipboardStatusNotifier> {
        app.create(ClipboardStatusNotifier {
            change_notifier: ChangeNotifierData::new(),
            value,
            disposed: false,
            observer: None,
            web: false,
        })
    }

    /// Dart's `_WebClipboardStatusNotifier()`: value is hardcoded to pasteable so web
    /// does not call [`Clipboard::has_strings`] (which would prompt for permission).
    pub fn web(app: &mut App) -> Handle<ClipboardStatusNotifier> {
        app.create(ClipboardStatusNotifier {
            change_notifier: ChangeNotifierData::new(),
            value: ClipboardStatus::Pasteable,
            disposed: false,
            observer: None,
            web: true,
        })
    }

    fn observer(self: Handle<Self>, app: &mut App) -> WidgetsBindingObserverRef {
        if let Some(observer) = app.get(self).observer.clone() {
            return observer;
        }
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).observer = Some(observer.clone());
        observer
    }

    /// The current clipboard pasteability.
    pub fn value(self: Handle<Self>, app: &App) -> ClipboardStatus {
        app.get(self).value
    }

    /// Replaces the current value, notifying listeners when it differs.
    pub fn set_value(self: Handle<Self>, app: &mut App, new_value: ClipboardStatus) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }

    /// Check the [`Clipboard`] and update [`value`](Self::value) if needed.
    pub fn update(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed || app.get(self).web {
            return;
        }

        let has_strings = Clipboard::has_strings(app);
        let next_status = if has_strings {
            ClipboardStatus::Pasteable
        } else {
            ClipboardStatus::NotPasteable
        };

        if app.get(self).disposed {
            return;
        }
        self.set_value(app, next_status);
    }

    /// Discards any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get(self).observer.clone() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
        app.get_mut(self).disposed = true;
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for ClipboardStatusNotifier {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }

    fn will_add_listener(self: Handle<Self>, app: &mut App) {
        if !app.get(self).change_notifier.has_listeners() {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).add_observer(app, observer);
        }
        if app.get(self).value == ClipboardStatus::Unknown {
            self.update(app);
        }
    }

    fn did_remove_listener(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed || app.get(self).change_notifier.has_listeners() {
            return;
        }
        if let Some(observer) = app.get(self).observer.clone() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
    }
}

impl WidgetsBindingObserverObject for ClipboardStatusNotifier {}

/// A [`ChangeNotifier`] whose value indicates whether the current device supports the Live Text
/// (OCR) function.
///
/// See also:
///
///  * `LiveText`, where the availability of Live Text input can be obtained.
///  * [`LiveTextInputStatus`], an enumeration that indicates whether the current device is available
///    for Live Text input.
///
/// Call [`update`](Self::update) to refresh [`value`](Self::value) if needed.
pub struct LiveTextInputStatusNotifier {
    change_notifier: ChangeNotifierData,
    value: LiveTextInputStatus,
    disposed: bool,
    observer: Option<WidgetsBindingObserverRef>,
}

impl Debug for LiveTextInputStatusNotifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveTextInputStatusNotifier")
            .field("value", &self.value)
            .finish()
    }
}

impl LiveTextInputStatusNotifier {
    /// Create a new LiveTextStatusNotifier.
    pub fn new(app: &mut App) -> Handle<LiveTextInputStatusNotifier> {
        Self::from_value(app, LiveTextInputStatus::Unknown)
    }

    /// Dart `LiveTextInputStatusNotifier(value:)`.
    pub fn from_value(
        app: &mut App,
        value: LiveTextInputStatus,
    ) -> Handle<LiveTextInputStatusNotifier> {
        app.create(LiveTextInputStatusNotifier {
            change_notifier: ChangeNotifierData::new(),
            value,
            disposed: false,
            observer: None,
        })
    }

    fn observer(self: Handle<Self>, app: &mut App) -> WidgetsBindingObserverRef {
        if let Some(observer) = app.get(self).observer.clone() {
            return observer;
        }
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).observer = Some(observer.clone());
        observer
    }

    /// The current Live Text input availability.
    pub fn value(self: Handle<Self>, app: &App) -> LiveTextInputStatus {
        app.get(self).value
    }

    /// Replaces the current value, notifying listeners when it differs.
    pub fn set_value(self: Handle<Self>, app: &mut App, new_value: LiveTextInputStatus) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }

    /// Check the Live Text input status and update [`value`](Self::value) if needed.
    pub fn update(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }

        let next_status = if reveal_services::LiveText::is_live_text_input_available(app) {
            LiveTextInputStatus::Enabled
        } else {
            LiveTextInputStatus::Disabled
        };
        if app.get(self).disposed || next_status == app.get(self).value {
            return;
        }
        self.set_value(app, next_status);
    }

    /// Discards any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get(self).observer.clone() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
        app.get_mut(self).disposed = true;
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for LiveTextInputStatusNotifier {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }

    fn will_add_listener(self: Handle<Self>, app: &mut App) {
        if !app.get(self).change_notifier.has_listeners() {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).add_observer(app, observer);
        }
        if app.get(self).value == LiveTextInputStatus::Unknown {
            self.update(app);
        }
    }

    fn did_remove_listener(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed || app.get(self).change_notifier.has_listeners() {
            return;
        }
        if let Some(observer) = app.get(self).observer.clone() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
    }
}

impl WidgetsBindingObserverObject for LiveTextInputStatusNotifier {}

/// An interface for building the selection UI, to be provided by the
/// implementer of the toolbar widget.
///
/// Parts of this class, including [`build_toolbar`](Self::build_toolbar), have been deprecated in favor
/// of `EditableText.contextMenuBuilder`, which is now the preferred way to
/// customize the context menus.
///
/// ## Use with `EditableText.contextMenuBuilder`
///
/// For backwards compatibility during the deprecation period, when
/// `EditableText.selectionControls` is set to an object that does not mix in
/// [`TextSelectionHandleControls`], `EditableText.contextMenuBuilder` is ignored
/// in favor of the deprecated [`build_toolbar`](Self::build_toolbar).
///
/// To migrate code from [`build_toolbar`](Self::build_toolbar) to the preferred
/// `EditableText.contextMenuBuilder`, while still using [`build_handle`](Self::build_handle), mix in
/// [`TextSelectionHandleControls`] into the [`TextSelectionControls`] subclass when
/// moving any toolbar code to a callback passed to
/// `EditableText.contextMenuBuilder`.
///
/// In due course, [`build_toolbar`](Self::build_toolbar) will be removed, and the mixin will no longer
/// be necessary as a way to flag to the framework that the code has been
/// migrated and does not expect [`build_toolbar`](Self::build_toolbar) to be called.
///
/// For more information, see <https://docs.flutter.dev/release/breaking-changes/context-menus>.
///
/// See also:
///
///  * `SelectionArea`, which selects appropriate text selection controls
///    based on the current platform.
pub trait TextSelectionControls {
    /// Builds a selection handle of the given `handle_type`.
    ///
    /// The top left corner of this widget is positioned at the bottom of the
    /// selection position.
    ///
    /// The supplied `on_tap` should be invoked when the handle is tapped, if such
    /// interaction is allowed. As a counterexample, the default selection handle
    /// on iOS `cupertinoTextSelectionControls` does not call `on_tap` at all,
    /// since its handles are not meant to be tapped.
    fn build_handle(
        &self,
        app: &mut App,
        context: BuildContext,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
        on_tap: Option<Listener>,
    ) -> WidgetRef;

    /// Get the anchor point of the handle relative to itself. The anchor point is
    /// the point that is aligned with a specific point in the text. A handle
    /// often visually "points to" that location.
    fn get_handle_anchor(
        &self,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
    ) -> Offset;

    /// Builds a toolbar near a text selection.
    ///
    /// Typically displays buttons for copying and pasting text.
    ///
    /// The `global_editable_region` parameter is the TextField size of the global
    /// coordinate system in logical pixels.
    ///
    /// The `text_line_height` parameter is the `RenderEditable.preferredLineHeight`
    /// of the `RenderEditable` we are building a toolbar for.
    ///
    /// The `selection_midpoint` parameter is a general calculation midpoint
    /// parameter of the toolbar. More detailed position information
    /// is computable from the `endpoints` parameter.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    #[allow(clippy::too_many_arguments)]
    fn build_toolbar(
        &self,
        app: &mut App,
        context: BuildContext,
        global_editable_region: Rect,
        text_line_height: f64,
        selection_midpoint: Offset,
        endpoints: &[TextSelectionPoint],
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
        last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef;

    /// Returns the size of the selection handle.
    fn get_handle_size(&self, text_line_height: f64) -> Size;

    /// Whether the current selection of the text field managed by the given
    /// `delegate` can be removed from the text field and placed into the
    /// [`Clipboard`].
    ///
    /// By default, false is returned when nothing is selected in the text field.
    ///
    /// Subclasses can use this to decide if they should expose the cut
    /// functionality to the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_cut(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        delegate.cut_enabled(app) && !delegate.text_editing_value(app).selection.is_collapsed()
    }

    /// Whether the current selection of the text field managed by the given
    /// `delegate` can be copied to the [`Clipboard`].
    ///
    /// By default, false is returned when nothing is selected in the text field.
    ///
    /// Subclasses can use this to decide if they should expose the copy
    /// functionality to the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_copy(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        delegate.copy_enabled(app) && !delegate.text_editing_value(app).selection.is_collapsed()
    }

    /// Whether the text field managed by the given `delegate` supports pasting
    /// from the clipboard.
    ///
    /// Subclasses can use this to decide if they should expose the paste
    /// functionality to the user.
    ///
    /// This does not consider the contents of the clipboard. Subclasses may want
    /// to, for example, disallow pasting when the clipboard contains an empty
    /// string.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_paste(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        delegate.paste_enabled(app)
    }

    /// Whether the current selection of the text field managed by the given
    /// `delegate` can be extended to include the entire content of the text
    /// field.
    ///
    /// Subclasses can use this to decide if they should expose the select all
    /// functionality to the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        delegate.select_all_enabled(app)
            && !delegate.text_editing_value(app).text.is_empty()
            && delegate.text_editing_value(app).selection.is_collapsed()
    }

    /// Call [`AnyTextSelectionDelegate::cut_selection`] to cut current selection.
    ///
    /// This is called by subclasses when their cut affordance is activated by
    /// the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_cut(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        delegate.cut_selection(app, SelectionChangedCause::Toolbar);
    }

    /// Call [`AnyTextSelectionDelegate::copy_selection`] to copy current selection.
    ///
    /// This is called by subclasses when their copy affordance is activated by
    /// the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_copy(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        delegate.copy_selection(app, SelectionChangedCause::Toolbar);
    }

    /// Call [`AnyTextSelectionDelegate::paste_text`] to paste text.
    ///
    /// This is called by subclasses when their paste affordance is activated by
    /// the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_paste(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        delegate.paste_text(app, SelectionChangedCause::Toolbar);
    }

    /// Call [`AnyTextSelectionDelegate::select_all`] to set the current selection to
    /// contain the entire text value.
    ///
    /// Does not hide the toolbar.
    ///
    /// This is called by subclasses when their select-all affordance is activated by
    /// the user.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        delegate.select_all(app, SelectionChangedCause::Toolbar);
    }
}

/// [`TextSelectionControls`] that specifically do not manage the toolbar in order
/// to leave that to `EditableText.contextMenuBuilder`.
///
/// A leaf that mixes this in forwards `build_toolbar` / `can_*` / `handle_*` to
/// these defaults from its [`TextSelectionControls`] impl: a subtrait default does
/// not override the supertrait method.
///
/// Marker that `contextMenuBuilder` is used.
pub trait TextSelectionHandleControls: TextSelectionControls {
    /// Dart's mixin `buildToolbar`; unused once `contextMenuBuilder` owns the menu.
    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    #[allow(clippy::too_many_arguments)]
    fn build_toolbar(
        &self,
        app: &mut App,
        context: BuildContext,
        global_editable_region: Rect,
        text_line_height: f64,
        selection_midpoint: Offset,
        endpoints: &[TextSelectionPoint],
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
        last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        let _ = (
            app,
            context,
            global_editable_region,
            text_line_height,
            selection_midpoint,
            endpoints,
            delegate,
            clipboard_status,
            last_secondary_tap_down_position,
        );
        SizedBox::shrink().into_widget()
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_cut(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        let _ = (app, delegate);
        false
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_copy(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        let _ = (app, delegate);
        false
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_paste(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        let _ = (app, delegate);
        false
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        let _ = (app, delegate);
        false
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_cut(
        &self,
        app: &mut App,
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    ) {
        let _ = (app, delegate, clipboard_status);
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_copy(
        &self,
        app: &mut App,
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    ) {
        let _ = (app, delegate, clipboard_status);
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_paste(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        let _ = (app, delegate);
    }

    #[deprecated(note = "Use `contextMenuBuilder` instead. \
                This feature was deprecated after v3.3.0-0.5.pre.")]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        let _ = (app, delegate);
    }
}

/// An object that manages overlay entries for selection handles.
///
/// The [`context`](Self::context) must have an [`Overlay`] as an ancestor.
pub struct TextSelectionOverlay {
    /// The context in which the selection UI should appear.
    pub context: BuildContext,
    /// The editable line in which the selected text is being displayed.
    pub render_object: RenderHandle<RenderEditable>,
    /// Builds text selection handles and toolbar.
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    /// The delegate for manipulating the current selection in the owning text field.
    pub selection_delegate: AnyTextSelectionDelegate,
    selection_overlay: Handle<SelectionOverlay>,
    /// If not provided, no context menu will be built.
    pub context_menu_builder: Option<WidgetBuilder>,
    value: TextEditingValue,
    effective_start_handle_visibility: Handle<ValueNotifier<bool>>,
    effective_end_handle_visibility: Handle<ValueNotifier<bool>>,
    effective_toolbar_visibility: Handle<ValueNotifier<bool>>,
    handles_visible: bool,
    debug_required_for: Option<WidgetRef>,
    drag_start_behavior: DragStartBehavior,
    on_selection_handle_tapped: Option<Listener>,
    clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    end_handle_drag_position: f64,
    end_handle_drag_target: f64,
    start_handle_drag_position: f64,
    start_handle_drag_target: f64,
    drag_start_selection: Option<TextSelection>,
}

impl TextSelectionOverlay {
    /// Creates an object that manages overlay entries for selection handles.
    ///
    /// The [`context`](Self::context) must have an [`Overlay`] as an ancestor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        value: TextEditingValue,
        context: BuildContext,
        toolbar_layer_link: Handle<LayerLink>,
        start_handle_layer_link: Handle<LayerLink>,
        end_handle_layer_link: Handle<LayerLink>,
        render_object: RenderHandle<RenderEditable>,
        selection_delegate: AnyTextSelectionDelegate,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> Handle<Self> {
        let effective_start = app.create(ValueNotifier::new(false));
        let effective_end = app.create(ValueNotifier::new(false));
        let effective_toolbar = app.create(ValueNotifier::new(false));
        let selection_overlay = SelectionOverlay::new(
            app,
            context,
            TextSelectionHandleType::Collapsed,
            0.0,
            TextSelectionHandleType::Collapsed,
            0.0,
            Vec::new(),
            None,
            Some(selection_delegate),
            None,
            start_handle_layer_link,
            end_handle_layer_link,
            toolbar_layer_link,
        )
        .start_handles_visible(app, effective_start)
        .end_handles_visible(app, effective_end)
        .toolbar_visible(app, effective_toolbar)
        .magnifier_configuration(app, magnifier_configuration);
        selection_overlay
            .set_toolbar_location(app, render_object.last_secondary_tap_down_position(app));

        let this = app.create(TextSelectionOverlay {
            context,
            render_object,
            selection_controls: None,
            selection_delegate,
            selection_overlay,
            context_menu_builder: None,
            value,
            effective_start_handle_visibility: effective_start,
            effective_end_handle_visibility: effective_end,
            effective_toolbar_visibility: effective_toolbar,
            handles_visible: false,
            debug_required_for: None,
            drag_start_behavior: DragStartBehavior::Start,
            on_selection_handle_tapped: None,
            clipboard_status: None,
            end_handle_drag_position: 0.0,
            end_handle_drag_target: 0.0,
            start_handle_drag_position: 0.0,
            start_handle_drag_target: 0.0,
            drag_start_selection: None,
        });

        selection_overlay.set_on_start_handle_drag_start(
            app,
            Rc::new(move |app, details| {
                this.handle_selection_start_handle_drag_start(app, details);
            }),
        );
        selection_overlay.set_on_start_handle_drag_update(
            app,
            Rc::new(move |app, details| {
                this.handle_selection_start_handle_drag_update(app, details);
            }),
        );
        selection_overlay.set_on_start_handle_drag_end(
            app,
            Rc::new(move |app, details| {
                this.handle_any_drag_end(app, details);
            }),
        );
        selection_overlay.set_on_end_handle_drag_start(
            app,
            Rc::new(move |app, details| {
                this.handle_selection_end_handle_drag_start(app, details);
            }),
        );
        selection_overlay.set_on_end_handle_drag_update(
            app,
            Rc::new(move |app, details| {
                this.handle_selection_end_handle_drag_update(app, details);
            }),
        );
        selection_overlay.set_on_end_handle_drag_end(
            app,
            Rc::new(move |app, details| {
                this.handle_any_drag_end(app, details);
            }),
        );

        let start_in_viewport = render_object.selection_start_in_viewport(app);
        let end_in_viewport = render_object.selection_end_in_viewport(app);
        start_in_viewport.add_listener(
            app,
            Listener::handle_method(this, Self::update_text_selection_overlay_visibilities),
        );
        end_in_viewport.add_listener(
            app,
            Listener::handle_method(this, Self::update_text_selection_overlay_visibilities),
        );
        this.update_text_selection_overlay_visibilities(app);
        this
    }

    /// Dart `TextSelectionOverlay(debugRequiredFor:)`.
    pub fn debug_required_for(
        self: Handle<Self>,
        app: &mut App,
        widget: WidgetRef,
    ) -> Handle<Self> {
        app.get_mut(self).debug_required_for = Some(widget.clone());
        let inner = app.get(self).selection_overlay;
        app.get_mut(inner).debug_required_for = Some(widget);
        self
    }

    /// Dart `TextSelectionOverlay(selectionControls:)`.
    pub fn selection_controls(
        self: Handle<Self>,
        app: &mut App,
        controls: Rc<dyn TextSelectionControls>,
    ) -> Handle<Self> {
        app.get_mut(self).selection_controls = Some(Rc::clone(&controls));
        let inner = app.get(self).selection_overlay;
        app.get_mut(inner).selection_controls = Some(controls);
        self
    }

    /// Dart `TextSelectionOverlay(handlesVisible:)`.
    pub fn handles_visible(self: Handle<Self>, app: &mut App, visible: bool) -> Handle<Self> {
        self.set_handles_visible(app, visible);
        self
    }

    /// Dart `TextSelectionOverlay(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        self: Handle<Self>,
        app: &mut App,
        drag_start_behavior: DragStartBehavior,
    ) -> Handle<Self> {
        app.get_mut(self).drag_start_behavior = drag_start_behavior;
        let inner = app.get(self).selection_overlay;
        app.get_mut(inner).drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `TextSelectionOverlay(onSelectionHandleTapped:)`.
    pub fn on_selection_handle_tapped(
        self: Handle<Self>,
        app: &mut App,
        on_selection_handle_tapped: Listener,
    ) -> Handle<Self> {
        app.get_mut(self).on_selection_handle_tapped = Some(on_selection_handle_tapped.clone());
        let inner = app.get(self).selection_overlay;
        app.get_mut(inner).on_selection_handle_tapped = Some(on_selection_handle_tapped);
        self
    }

    /// Dart `TextSelectionOverlay(clipboardStatus:)`.
    pub fn clipboard_status(
        self: Handle<Self>,
        app: &mut App,
        clipboard_status: Handle<ClipboardStatusNotifier>,
    ) -> Handle<Self> {
        app.get_mut(self).clipboard_status = Some(clipboard_status);
        let inner = app.get(self).selection_overlay;
        app.get_mut(inner).clipboard_status = Some(clipboard_status);
        self
    }

    /// Dart `TextSelectionOverlay(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        self: Handle<Self>,
        app: &mut App,
        context_menu_builder: WidgetBuilder,
    ) -> Handle<Self> {
        app.get_mut(self).context_menu_builder = Some(context_menu_builder);
        self
    }

    /// Retrieve current value.
    pub fn value(self: Handle<Self>, app: &App) -> TextEditingValue {
        app.get(self).value.clone()
    }

    fn selection(self: Handle<Self>, app: &App) -> TextSelection {
        app.get(self).value.selection
    }

    fn update_text_selection_overlay_visibilities(self: Handle<Self>, app: &mut App) {
        let handles_visible = app.get(self).handles_visible;
        let render_object = app.get(self).render_object;
        let start_in_view = *render_object.selection_start_in_viewport(app).value(app);
        let end_in_view = *render_object.selection_end_in_viewport(app).value(app);
        let start = app.get(self).effective_start_handle_visibility;
        let end = app.get(self).effective_end_handle_visibility;
        let toolbar = app.get(self).effective_toolbar_visibility;
        start.set_value(app, handles_visible && start_in_view);
        end.set_value(app, handles_visible && end_in_view);
        toolbar.set_value(app, start_in_view || end_in_view);
    }

    /// Whether selection handles are visible.
    pub fn handles_visible_value(self: Handle<Self>, app: &App) -> bool {
        app.get(self).handles_visible
    }

    /// Set to false if you want to hide the handles without rebuilding them.
    pub fn set_handles_visible(self: Handle<Self>, app: &mut App, visible: bool) {
        if app.get(self).handles_visible == visible {
            return;
        }
        app.get_mut(self).handles_visible = visible;
        self.update_text_selection_overlay_visibilities(app);
    }

    /// Builds the handles by inserting them into the [`context`](Self::context)'s overlay.
    pub fn show_handles(self: Handle<Self>, app: &mut App) {
        self.update_selection_overlay(app);
        let overlay = app.get(self).selection_overlay;
        overlay.show_handles(app);
    }

    /// Destroys the handles by removing them from overlay.
    pub fn hide_handles(self: Handle<Self>, app: &mut App) {
        let overlay = app.get(self).selection_overlay;
        overlay.hide_handles(app);
    }

    /// Shows the toolbar by inserting it into the overlay.
    ///
    /// Must not be called during the build or layout phase.
    pub fn show_toolbar(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks,
            "showToolbar must not be called during the build or layout phase.",
        );
        self.update_selection_overlay(app);
        let overlay = app.get(self).selection_overlay;
        let context = app.get(self).context;
        let builder = app.get(self).context_menu_builder.clone();
        if let Some(builder) = builder {
            debug_assert!(context.mounted(app));
            overlay.show_toolbar(app, Some(context), Some(builder));
            return;
        }
        if app.get(self).selection_controls.is_some() {
            overlay.show_toolbar(app, None, None);
        }
    }

    /// Shows toolbar with spell check suggestions of misspelled words that are
    /// available for click-and-replace.
    pub fn show_spell_check_suggestions_toolbar(
        self: Handle<Self>,
        app: &mut App,
        spell_check_suggestions_toolbar_builder: WidgetBuilder,
    ) {
        self.update_selection_overlay(app);
        let context = app.get(self).context;
        debug_assert!(context.mounted(app));
        let overlay = app.get(self).selection_overlay;
        overlay.show_spell_check_suggestions_toolbar(
            app,
            Some(context),
            spell_check_suggestions_toolbar_builder,
        );
        self.hide_handles(app);
    }

    /// Shows the magnifier, and hides the toolbar if it was showing.
    pub fn show_magnifier(self: Handle<Self>, app: &mut App, position_to_show: Offset) {
        let render_object = app.get(self).render_object;
        let position = render_object.get_position_for_point(app, position_to_show);
        self.update_selection_overlay(app);
        let info = self.build_magnifier(app, render_object, position_to_show, position);
        let overlay = app.get(self).selection_overlay;
        overlay.show_magnifier(app, info);
    }

    /// Update the current magnifier with new selection data.
    pub fn update_magnifier(self: Handle<Self>, app: &mut App, position_to_show: Offset) {
        let render_object = app.get(self).render_object;
        let position = render_object.get_position_for_point(app, position_to_show);
        self.update_selection_overlay(app);
        let info = self.build_magnifier(app, render_object, position_to_show, position);
        let overlay = app.get(self).selection_overlay;
        overlay.update_magnifier(app, info);
    }

    /// Hide the current magnifier.
    pub fn hide_magnifier(self: Handle<Self>, app: &mut App) {
        let overlay = app.get(self).selection_overlay;
        overlay.hide_magnifier(app);
    }

    /// Updates the overlay after the selection has changed.
    pub fn update(self: Handle<Self>, app: &mut App, new_value: TextEditingValue) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.update_selection_overlay(app);
        let overlay = app.get(self).selection_overlay;
        overlay.mark_needs_build(app);
    }

    fn update_selection_overlay(self: Handle<Self>, app: &mut App) {
        let render_object = app.get(self).render_object;
        let selection = self.selection(app);
        let endpoints = render_object.get_endpoints_for_selection(app, selection);
        debug_assert!(!endpoints.is_empty());

        let (start_handle_type, end_handle_type) = if selection.is_collapsed() {
            (
                TextSelectionHandleType::Collapsed,
                TextSelectionHandleType::Collapsed,
            )
        } else {
            let text_direction = render_object.text_direction(app);
            let prefer_field_direction = app.platform().target_platform() == TargetPlatform::IOS;
            let (start_handle_direction, end_handle_direction) =
                if prefer_field_direction || endpoints.len() < 2 {
                    (text_direction, text_direction)
                } else {
                    (
                        endpoints[0].direction.unwrap_or(text_direction),
                        endpoints[endpoints.len() - 1]
                            .direction
                            .unwrap_or(text_direction),
                    )
                };
            (
                match start_handle_direction {
                    TextDirection::Ltr => TextSelectionHandleType::Left,
                    TextDirection::Rtl => TextSelectionHandleType::Right,
                },
                match end_handle_direction {
                    TextDirection::Ltr => TextSelectionHandleType::Right,
                    TextDirection::Rtl => TextSelectionHandleType::Left,
                },
            )
        };

        let line_height_at_start = self.start_glyph_height(app);
        let line_height_at_end = self.end_glyph_height(app);
        let overlay = app.get(self).selection_overlay;
        overlay.set_start_handle_type(app, start_handle_type);
        overlay.set_line_height_at_start(app, line_height_at_start);
        overlay.set_end_handle_type(app, end_handle_type);
        overlay.set_line_height_at_end(app, line_height_at_end);
        overlay.set_selection_endpoints(app, endpoints);
        overlay.set_toolbar_location(app, render_object.last_secondary_tap_down_position(app));
    }

    /// Causes the overlay to update its rendering after a scroll or metrics change.
    pub fn update_for_scroll(self: Handle<Self>, app: &mut App) {
        self.update_selection_overlay(app);
        let overlay = app.get(self).selection_overlay;
        overlay.mark_needs_build(app);
    }

    /// Whether the handles are currently visible.
    pub fn handles_are_visible(self: Handle<Self>, app: &App) -> bool {
        let overlay = app.get(self).selection_overlay;
        overlay.has_handles(app) && app.get(self).handles_visible
    }

    /// Whether the toolbar is currently visible.
    pub fn toolbar_is_visible(self: Handle<Self>, app: &App) -> bool {
        let overlay = app.get(self).selection_overlay;
        overlay.toolbar_is_visible(app)
    }

    /// Whether the magnifier is currently visible.
    pub fn magnifier_is_visible(self: Handle<Self>, app: &App) -> bool {
        let overlay = app.get(self).selection_overlay;
        overlay.magnifier_is_visible(app)
    }

    /// Whether the magnifier currently exists in the overlay.
    pub fn magnifier_exists(self: Handle<Self>, app: &App) -> bool {
        let overlay = app.get(self).selection_overlay;
        overlay.magnifier_exists(app)
    }

    /// Whether the spell check menu is currently visible.
    pub fn spell_check_toolbar_is_visible(self: Handle<Self>, app: &App) -> bool {
        let overlay = app.get(self).selection_overlay;
        overlay.spell_check_toolbar_is_shown(app)
    }

    /// Hides the entire overlay including the toolbar and the handles.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        let overlay = app.get(self).selection_overlay;
        overlay.hide(app);
    }

    /// Hides the toolbar part of the overlay.
    pub fn hide_toolbar(self: Handle<Self>, app: &mut App) {
        let overlay = app.get(self).selection_overlay;
        overlay.hide_toolbar(app);
    }

    /// Disposes this object and release resources.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let overlay = app.get(self).selection_overlay;
        overlay.dispose(app);
        let render_object = app.get(self).render_object;
        let start_in_viewport = render_object.selection_start_in_viewport(app);
        let end_in_viewport = render_object.selection_end_in_viewport(app);
        start_in_viewport.remove_listener(
            app,
            &Listener::handle_method(self, Self::update_text_selection_overlay_visibilities),
        );
        end_in_viewport.remove_listener(
            app,
            &Listener::handle_method(self, Self::update_text_selection_overlay_visibilities),
        );
        app.get_mut(app.get(self).effective_toolbar_visibility)
            .dispose();
        app.get_mut(app.get(self).effective_start_handle_visibility)
            .dispose();
        app.get_mut(app.get(self).effective_end_handle_visibility)
            .dispose();
        self.hide_toolbar(app);
    }

    fn start_glyph_height(self: Handle<Self>, app: &mut App) -> f64 {
        let delegate = app.get(self).selection_delegate;
        let curr_text = delegate.text_editing_value(app).text;
        let render_object = app.get(self).render_object;
        let selection = self.selection(app);
        let mut start_handle_rect = None;
        if render_object.plain_text(app) == curr_text
            && selection.is_valid()
            && !selection.is_collapsed()
        {
            let selected = selection.range().text_inside(&curr_text);
            if let Some(first) = selected.chars().next() {
                let extent = first.len_utf16() as i32;
                start_handle_rect = render_object.get_rect_for_composing_range(
                    app,
                    TextRange::new(selection.start(), selection.start() + extent),
                );
            }
        }
        start_handle_rect
            .map(|rect| rect.height())
            .unwrap_or_else(|| render_object.preferred_line_height(app))
    }

    fn end_glyph_height(self: Handle<Self>, app: &mut App) -> f64 {
        let delegate = app.get(self).selection_delegate;
        let curr_text = delegate.text_editing_value(app).text;
        let render_object = app.get(self).render_object;
        let selection = self.selection(app);
        let mut end_handle_rect = None;
        if render_object.plain_text(app) == curr_text
            && selection.is_valid()
            && !selection.is_collapsed()
        {
            let selected = selection.range().text_inside(&curr_text);
            if let Some(last) = selected.chars().next_back() {
                let extent = last.len_utf16() as i32;
                end_handle_rect = render_object.get_rect_for_composing_range(
                    app,
                    TextRange::new(selection.end() - extent, selection.end()),
                );
            }
        }
        end_handle_rect
            .map(|rect| rect.height())
            .unwrap_or_else(|| render_object.preferred_line_height(app))
    }

    fn build_magnifier(
        self: Handle<Self>,
        app: &mut App,
        render_editable: RenderHandle<RenderEditable>,
        global_gesture_position: Offset,
        current_text_position: TextPosition,
    ) -> MagnifierInfo {
        let line_at_offset = render_editable.get_line_at_offset(app, current_text_position);
        let position_at_end_of_line =
            TextPosition::with_affinity(line_at_offset.extent_offset, TextAffinity::Upstream);
        let position_at_beginning_of_line = TextPosition::new(line_at_offset.base_offset);
        let local_line_boundaries = Rect::from_points(
            render_editable
                .get_local_rect_for_caret(app, position_at_beginning_of_line)
                .top_center(),
            render_editable
                .get_local_rect_for_caret(app, position_at_end_of_line)
                .bottom_center(),
        );
        let context = app.get(self).context;
        let overlay_state = Overlay::of(app, context, true);
        let overlay = overlay_state
            .context(app)
            .find_render_object(app)
            .and_then(|object| object.as_box());
        let transform_to_overlay = render_editable
            .as_object()
            .get_transform_to(app, overlay.map(|box_| box_.as_object()));
        let overlay_line_boundaries = transform_rect(&transform_to_overlay, local_line_boundaries);
        let local_caret_rect = render_editable.get_local_rect_for_caret(app, current_text_position);
        let overlay_caret_rect = transform_rect(&transform_to_overlay, local_caret_rect);
        let overlay_gesture_position = overlay
            .map(|box_| box_.global_to_local(app, global_gesture_position, None))
            .unwrap_or(global_gesture_position);
        MagnifierInfo::new(
            overlay_gesture_position,
            overlay_caret_rect,
            transform_rect(
                &transform_to_overlay,
                render_editable.as_object().paint_bounds(app),
            ),
            overlay_line_boundaries,
        )
    }

    fn handle_selection_end_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
    ) {
        let render_object = app.get(self).render_object;
        if !render_object.attached(app) {
            return;
        }
        app.get_mut(self).end_handle_drag_position = details.global_position.dy();
        let overlay = app.get(self).selection_overlay;
        let endpoints = overlay.selection_endpoints(app);
        let Some(last) = endpoints.last() else {
            return;
        };
        let center_of_line_local = last.point.dy() - render_object.preferred_line_height(app) / 2.0;
        let center_of_line_global = render_object
            .as_box()
            .local_to_global(app, Offset::new(0.0, center_of_line_local), None)
            .dy();
        app.get_mut(self).end_handle_drag_target =
            center_of_line_global - details.global_position.dy();
        let position = render_object.get_position_for_point(
            app,
            Offset::new(details.global_position.dx(), center_of_line_global),
        );
        if matches!(
            app.platform().target_platform(),
            TargetPlatform::IOS | TargetPlatform::MacOS
        ) && app.get(self).drag_start_selection.is_none()
        {
            app.get_mut(self).drag_start_selection = Some(self.selection(app));
        }
        let info = self.build_magnifier(app, render_object, details.global_position, position);
        overlay.show_magnifier(app, info);
    }

    fn handle_dy(self: Handle<Self>, app: &mut App, drag_dy: f64, handle_dy: f64) -> Option<f64> {
        let preferred_line_height = app.get(self).render_object.preferred_line_height(app);
        debug_assert!(
            preferred_line_height.is_finite(),
            "Preferred line height is expected to always be finite."
        );
        if preferred_line_height <= 0.0 || !drag_dy.is_finite() || !handle_dy.is_finite() {
            return None;
        }
        let distance_dragged = drag_dy - handle_dy;
        let drag_direction = if distance_dragged < 0.0 { -1.0 } else { 1.0 };
        let lines_dragged =
            drag_direction * (distance_dragged.abs() / preferred_line_height).floor();
        Some(handle_dy + lines_dragged * preferred_line_height)
    }

    fn handle_selection_end_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        details: DragUpdateDetails,
    ) {
        let render_object = app.get(self).render_object;
        if !render_object.attached(app) {
            return;
        }
        let local_position =
            render_object
                .as_box()
                .global_to_local(app, details.global_position, None);
        let current = app.get(self).end_handle_drag_position;
        let Some(next_end_handle_drag_position_local) = self.handle_dy(
            app,
            local_position.dy(),
            render_object
                .as_box()
                .global_to_local(app, Offset::new(0.0, current), None)
                .dy(),
        ) else {
            return;
        };
        app.get_mut(self).end_handle_drag_position = render_object
            .as_box()
            .local_to_global(
                app,
                Offset::new(0.0, next_end_handle_drag_position_local),
                None,
            )
            .dy();
        let handle_target_global = Offset::new(
            details.global_position.dx(),
            app.get(self).end_handle_drag_position + app.get(self).end_handle_drag_target,
        );
        let position = render_object.get_position_for_point(app, handle_target_global);
        let overlay = app.get(self).selection_overlay;
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                let drag_start = app
                    .get(self)
                    .drag_start_selection
                    .expect("drag start selection is set on Apple platforms");
                if drag_start.is_collapsed() {
                    let info =
                        self.build_magnifier(app, render_object, details.global_position, position);
                    overlay.update_magnifier(app, info);
                    self.handle_selection_handle_changed(
                        app,
                        TextSelection::from_position(position),
                    );
                    return;
                }
                let normalized = drag_start.extent_offset >= drag_start.base_offset;
                let new_selection = TextSelection::new(
                    if normalized {
                        drag_start.base_offset
                    } else {
                        drag_start.extent_offset
                    },
                    position.offset,
                );
                self.handle_selection_handle_changed(app, new_selection);
                let info = self.build_magnifier(
                    app,
                    render_object,
                    details.global_position,
                    new_selection.extent(),
                );
                overlay.update_magnifier(app, info);
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                if self.selection(app).is_collapsed() {
                    let info =
                        self.build_magnifier(app, render_object, details.global_position, position);
                    overlay.update_magnifier(app, info);
                    self.handle_selection_handle_changed(
                        app,
                        TextSelection::from_position(position),
                    );
                    return;
                }
                let selection = self.selection(app);
                let new_selection = TextSelection::new(selection.base_offset, position.offset);
                if new_selection.base_offset >= new_selection.extent_offset {
                    return;
                }
                self.handle_selection_handle_changed(app, new_selection);
                let info = self.build_magnifier(
                    app,
                    render_object,
                    details.global_position,
                    new_selection.extent(),
                );
                overlay.update_magnifier(app, info);
            }
        }
    }

    fn handle_selection_start_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
    ) {
        let render_object = app.get(self).render_object;
        if !render_object.attached(app) {
            return;
        }
        app.get_mut(self).start_handle_drag_position = details.global_position.dy();
        let overlay = app.get(self).selection_overlay;
        let endpoints = overlay.selection_endpoints(app);
        let Some(first) = endpoints.first() else {
            return;
        };
        let center_of_line_local =
            first.point.dy() - render_object.preferred_line_height(app) / 2.0;
        let center_of_line_global = render_object
            .as_box()
            .local_to_global(app, Offset::new(0.0, center_of_line_local), None)
            .dy();
        app.get_mut(self).start_handle_drag_target =
            center_of_line_global - details.global_position.dy();
        let position = render_object.get_position_for_point(
            app,
            Offset::new(details.global_position.dx(), center_of_line_global),
        );
        if matches!(
            app.platform().target_platform(),
            TargetPlatform::IOS | TargetPlatform::MacOS
        ) && app.get(self).drag_start_selection.is_none()
        {
            app.get_mut(self).drag_start_selection = Some(self.selection(app));
        }
        let info = self.build_magnifier(app, render_object, details.global_position, position);
        overlay.show_magnifier(app, info);
    }

    fn handle_selection_start_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        details: DragUpdateDetails,
    ) {
        let render_object = app.get(self).render_object;
        if !render_object.attached(app) {
            return;
        }
        let local_position =
            render_object
                .as_box()
                .global_to_local(app, details.global_position, None);
        let current = app.get(self).start_handle_drag_position;
        let Some(next_start_handle_drag_position_local) = self.handle_dy(
            app,
            local_position.dy(),
            render_object
                .as_box()
                .global_to_local(app, Offset::new(0.0, current), None)
                .dy(),
        ) else {
            return;
        };
        app.get_mut(self).start_handle_drag_position = render_object
            .as_box()
            .local_to_global(
                app,
                Offset::new(0.0, next_start_handle_drag_position_local),
                None,
            )
            .dy();
        let handle_target_global = Offset::new(
            details.global_position.dx(),
            app.get(self).start_handle_drag_position + app.get(self).start_handle_drag_target,
        );
        let position = render_object.get_position_for_point(app, handle_target_global);
        let overlay = app.get(self).selection_overlay;
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                let drag_start = app
                    .get(self)
                    .drag_start_selection
                    .expect("drag start selection is set on Apple platforms");
                if drag_start.is_collapsed() {
                    let info =
                        self.build_magnifier(app, render_object, details.global_position, position);
                    overlay.update_magnifier(app, info);
                    self.handle_selection_handle_changed(
                        app,
                        TextSelection::from_position(position),
                    );
                    return;
                }
                let normalized = drag_start.extent_offset >= drag_start.base_offset;
                let new_selection = TextSelection::new(
                    if normalized {
                        drag_start.extent_offset
                    } else {
                        drag_start.base_offset
                    },
                    position.offset,
                );
                let magnifier_position =
                    if new_selection.extent().offset < new_selection.base().offset {
                        new_selection.extent()
                    } else {
                        new_selection.base()
                    };
                let info = self.build_magnifier(
                    app,
                    render_object,
                    details.global_position,
                    magnifier_position,
                );
                overlay.update_magnifier(app, info);
                self.handle_selection_handle_changed(app, new_selection);
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                if self.selection(app).is_collapsed() {
                    let info =
                        self.build_magnifier(app, render_object, details.global_position, position);
                    overlay.update_magnifier(app, info);
                    self.handle_selection_handle_changed(
                        app,
                        TextSelection::from_position(position),
                    );
                    return;
                }
                let selection = self.selection(app);
                let new_selection = TextSelection::new(position.offset, selection.extent_offset);
                if new_selection.base_offset >= new_selection.extent_offset {
                    return;
                }
                let magnifier_position =
                    if new_selection.extent().offset < new_selection.base().offset {
                        new_selection.extent()
                    } else {
                        new_selection.base()
                    };
                let info = self.build_magnifier(
                    app,
                    render_object,
                    details.global_position,
                    magnifier_position,
                );
                overlay.update_magnifier(app, info);
                self.handle_selection_handle_changed(app, new_selection);
            }
        }
    }

    fn handle_any_drag_end(self: Handle<Self>, app: &mut App, _details: DragEndDetails) {
        let context = app.get(self).context;
        if !context.mounted(app) {
            return;
        }
        app.get_mut(self).drag_start_selection = None;
        let overlay = app.get(self).selection_overlay;
        let dragging_handles =
            overlay.is_dragging_start_handle(app) || overlay.is_dragging_end_handle(app);
        if dragging_handles {
            return;
        }
        overlay.hide_magnifier(app);
        if self.selection(app).is_collapsed() {
            return;
        }
        let builder = app.get(self).context_menu_builder.clone();
        if let Some(builder) = builder {
            overlay.show_toolbar(app, Some(context), Some(builder));
        } else {
            overlay.show_toolbar(app, None, None);
        }
    }

    fn handle_selection_handle_changed(
        self: Handle<Self>,
        app: &mut App,
        new_selection: TextSelection,
    ) {
        let delegate = app.get(self).selection_delegate;
        let value = app.get(self).value.copy_with().selection(new_selection);
        delegate.user_update_text_editing_value(app, value, SelectionChangedCause::Drag);
    }
}

/// A pair of overlay entries for the start and end selection handles.
#[derive(Clone, Copy)]
struct SelectionHandles {
    start: Handle<OverlayEntry>,
    end: Handle<OverlayEntry>,
}

/// An object that manages a pair of selection handles and a toolbar.
///
/// The selection handles are displayed in the [`Overlay`] that most closely
/// encloses the given [`BuildContext`].
pub struct SelectionOverlay {
    /// The context in which the selection UI should appear.
    pub context: BuildContext,
    /// Debugging information for explaining why the [`Overlay`] is required.
    pub debug_required_for: Option<WidgetRef>,
    start_handle_type: TextSelectionHandleType,
    line_height_at_start: f64,
    /// Whether the start handle is visible.
    pub start_handles_visible: Option<Handle<ValueNotifier<bool>>>,
    /// Called when the users start dragging the start selection handles.
    pub on_start_handle_drag_start: Option<ValueChanged<DragStartDetails>>,
    /// Called when the users drag the start selection handles to new locations.
    pub on_start_handle_drag_update: Option<ValueChanged<DragUpdateDetails>>,
    /// Called when the users lift their fingers after dragging the start selection handles.
    pub on_start_handle_drag_end: Option<ValueChanged<DragEndDetails>>,
    end_handle_type: TextSelectionHandleType,
    line_height_at_end: f64,
    /// Whether the end handle is visible.
    pub end_handles_visible: Option<Handle<ValueNotifier<bool>>>,
    /// Called when the users start dragging the end selection handles.
    pub on_end_handle_drag_start: Option<ValueChanged<DragStartDetails>>,
    /// Called when the users drag the end selection handles to new locations.
    pub on_end_handle_drag_update: Option<ValueChanged<DragUpdateDetails>>,
    /// Called when the users lift their fingers after dragging the end selection handles.
    pub on_end_handle_drag_end: Option<ValueChanged<DragEndDetails>>,
    /// Whether the toolbar is visible.
    pub toolbar_visible: Option<Handle<ValueNotifier<bool>>>,
    selection_endpoints: Vec<TextSelectionPoint>,
    /// Builds text selection handles and toolbar.
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    /// The delegate for manipulating the current selection in the owning text field.
    pub selection_delegate: Option<AnyTextSelectionDelegate>,
    /// Maintains the status of the clipboard for determining if its contents can be pasted.
    pub clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    /// The objects supplied to the location of start selection handle.
    pub start_handle_layer_link: Handle<LayerLink>,
    /// The objects supplied to the location of end selection handle.
    pub end_handle_layer_link: Handle<LayerLink>,
    /// The object supplied to the location of the toolbar.
    pub toolbar_layer_link: Handle<LayerLink>,
    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,
    /// A callback that's optionally invoked when a selection handle is tapped.
    pub on_selection_handle_tapped: Option<Listener>,
    toolbar_location: Option<Offset>,
    /// The configuration for the magnifier.
    pub magnifier_configuration: TextMagnifierConfiguration,
    magnifier_info: Handle<ValueNotifier<MagnifierInfo>>,
    magnifier_controller: Handle<MagnifierController>,
    start_handle_drag_in_progress: bool,
    is_dragging_start_handle: bool,
    end_handle_drag_in_progress: bool,
    is_dragging_end_handle: bool,
    handles: Option<SelectionHandles>,
    toolbar: Option<Handle<OverlayEntry>>,
    context_menu: Option<Handle<OverlayEntry>>,
    context_menu_builder: Option<WidgetBuilder>,
    spell_check_toolbar: Option<Handle<OverlayEntry>>,
    spell_check_toolbar_builder: Option<WidgetBuilder>,
    build_scheduled: bool,
}

impl SelectionOverlay {
    /// Controls the fade-in and fade-out animations for the toolbar and handles.
    pub const FADE_DURATION: Duration = Duration::from_millis(150);

    /// Creates an object that manages overlay entries for selection handles.
    ///
    /// The [`context`](Self::context) must have an [`Overlay`] as an ancestor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        context: BuildContext,
        start_handle_type: TextSelectionHandleType,
        line_height_at_start: f64,
        end_handle_type: TextSelectionHandleType,
        line_height_at_end: f64,
        selection_endpoints: Vec<TextSelectionPoint>,
        selection_controls: Option<Rc<dyn TextSelectionControls>>,
        selection_delegate: Option<AnyTextSelectionDelegate>,
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
        start_handle_layer_link: Handle<LayerLink>,
        end_handle_layer_link: Handle<LayerLink>,
        toolbar_layer_link: Handle<LayerLink>,
    ) -> Handle<Self> {
        let magnifier_info = app.create(ValueNotifier::new(MagnifierInfo::EMPTY));
        let magnifier_controller = MagnifierController::new(app);
        app.create(SelectionOverlay {
            context,
            debug_required_for: None,
            start_handle_type,
            line_height_at_start,
            start_handles_visible: None,
            on_start_handle_drag_start: None,
            on_start_handle_drag_update: None,
            on_start_handle_drag_end: None,
            end_handle_type,
            line_height_at_end,
            end_handles_visible: None,
            on_end_handle_drag_start: None,
            on_end_handle_drag_update: None,
            on_end_handle_drag_end: None,
            toolbar_visible: None,
            selection_endpoints,
            selection_controls,
            selection_delegate,
            clipboard_status,
            start_handle_layer_link,
            end_handle_layer_link,
            toolbar_layer_link,
            drag_start_behavior: DragStartBehavior::Start,
            on_selection_handle_tapped: None,
            toolbar_location: None,
            magnifier_configuration: TextMagnifierConfiguration::DISABLED,
            magnifier_info,
            magnifier_controller,
            start_handle_drag_in_progress: false,
            is_dragging_start_handle: false,
            end_handle_drag_in_progress: false,
            is_dragging_end_handle: false,
            handles: None,
            toolbar: None,
            context_menu: None,
            context_menu_builder: None,
            spell_check_toolbar: None,
            spell_check_toolbar_builder: None,
            build_scheduled: false,
        })
    }

    /// Dart `SelectionOverlay(startHandlesVisible:)`.
    pub fn start_handles_visible(
        self: Handle<Self>,
        app: &mut App,
        start_handles_visible: Handle<ValueNotifier<bool>>,
    ) -> Handle<Self> {
        app.get_mut(self).start_handles_visible = Some(start_handles_visible);
        self
    }

    /// Dart `SelectionOverlay(endHandlesVisible:)`.
    pub fn end_handles_visible(
        self: Handle<Self>,
        app: &mut App,
        end_handles_visible: Handle<ValueNotifier<bool>>,
    ) -> Handle<Self> {
        app.get_mut(self).end_handles_visible = Some(end_handles_visible);
        self
    }

    /// Dart `SelectionOverlay(toolbarVisible:)`.
    pub fn toolbar_visible(
        self: Handle<Self>,
        app: &mut App,
        toolbar_visible: Handle<ValueNotifier<bool>>,
    ) -> Handle<Self> {
        app.get_mut(self).toolbar_visible = Some(toolbar_visible);
        self
    }

    /// Dart `SelectionOverlay(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        self: Handle<Self>,
        app: &mut App,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> Handle<Self> {
        app.get_mut(self).magnifier_configuration = magnifier_configuration;
        self
    }

    /// Dart `SelectionOverlay(toolbarLocation:)`.
    pub fn toolbar_location(
        self: Handle<Self>,
        app: &mut App,
        toolbar_location: Offset,
    ) -> Handle<Self> {
        self.set_toolbar_location(app, Some(toolbar_location));
        self
    }

    /// Dart `SelectionOverlay(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        self: Handle<Self>,
        app: &mut App,
        drag_start_behavior: DragStartBehavior,
    ) -> Handle<Self> {
        app.get_mut(self).drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `SelectionOverlay(onSelectionHandleTapped:)`.
    pub fn on_selection_handle_tapped(
        self: Handle<Self>,
        app: &mut App,
        on_selection_handle_tapped: Listener,
    ) -> Handle<Self> {
        app.get_mut(self).on_selection_handle_tapped = Some(on_selection_handle_tapped);
        self
    }

    /// Dart `SelectionOverlay(debugRequiredFor:)`.
    pub fn debug_required_for(
        self: Handle<Self>,
        app: &mut App,
        widget: WidgetRef,
    ) -> Handle<Self> {
        app.get_mut(self).debug_required_for = Some(widget);
        self
    }

    pub(crate) fn set_on_start_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragStartDetails>,
    ) {
        app.get_mut(self).on_start_handle_drag_start = Some(callback);
    }

    pub(crate) fn set_on_start_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragUpdateDetails>,
    ) {
        app.get_mut(self).on_start_handle_drag_update = Some(callback);
    }

    pub(crate) fn set_on_start_handle_drag_end(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragEndDetails>,
    ) {
        app.get_mut(self).on_start_handle_drag_end = Some(callback);
    }

    pub(crate) fn set_on_end_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragStartDetails>,
    ) {
        app.get_mut(self).on_end_handle_drag_start = Some(callback);
    }

    pub(crate) fn set_on_end_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragUpdateDetails>,
    ) {
        app.get_mut(self).on_end_handle_drag_update = Some(callback);
    }

    pub(crate) fn set_on_end_handle_drag_end(
        self: Handle<Self>,
        app: &mut App,
        callback: ValueChanged<DragEndDetails>,
    ) {
        app.get_mut(self).on_end_handle_drag_end = Some(callback);
    }

    /// Whether the toolbar is currently visible.
    pub fn toolbar_is_visible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).toolbar.is_some()
            || app.get(self).context_menu.is_some()
            || app.get(self).spell_check_toolbar.is_some()
    }

    /// Whether the magnifier is currently visible.
    pub fn magnifier_is_visible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).magnifier_controller.shown(app)
    }

    /// Whether the magnifier currently exists.
    pub fn magnifier_exists(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .magnifier_controller
            .overlay_entry(app)
            .is_some()
    }

    pub(crate) fn spell_check_toolbar_is_shown(self: Handle<Self>, app: &App) -> bool {
        app.get(self).spell_check_toolbar.is_some()
    }

    pub(crate) fn has_handles(self: Handle<Self>, app: &App) -> bool {
        app.get(self).handles.is_some()
    }

    /// Shows the magnifier, and hides the toolbar if it was showing.
    pub fn show_magnifier(
        self: Handle<Self>,
        app: &mut App,
        initial_magnifier_info: MagnifierInfo,
    ) {
        let controller = app.get(self).magnifier_controller;
        if controller.overlay_entry(app).is_some() {
            return;
        }
        if self.toolbar_is_visible(app) {
            self.hide_toolbar(app);
        }
        let info = app.get(self).magnifier_info;
        info.set_value(app, initial_magnifier_info);
        let context = app.get(self).context;
        let configuration = app.get(self).magnifier_configuration.clone();
        let built = (configuration.magnifier_builder())(app, context, controller, info);
        let Some(built) = built else {
            return;
        };
        let below = if configuration.should_display_handles_in_magnifier {
            None
        } else {
            app.get(self).handles.map(|handles| handles.start)
        };
        controller.show(
            app,
            context,
            Rc::new(move |_app, _context| built.clone()),
            below,
        );
    }

    /// Hide the current magnifier.
    pub fn hide_magnifier(self: Handle<Self>, app: &mut App) {
        let controller = app.get(self).magnifier_controller;
        if controller.overlay_entry(app).is_none() {
            return;
        }
        controller.hide(app, true);
    }

    /// The type of start selection handle.
    pub fn start_handle_type(self: Handle<Self>, app: &App) -> TextSelectionHandleType {
        app.get(self).start_handle_type
    }

    /// Changing the value while the handles are visible causes them to rebuild.
    pub fn set_start_handle_type(
        self: Handle<Self>,
        app: &mut App,
        value: TextSelectionHandleType,
    ) {
        if app.get(self).start_handle_type == value {
            return;
        }
        app.get_mut(self).start_handle_type = value;
        self.mark_needs_build(app);
    }

    /// The line height at the selection start.
    pub fn line_height_at_start(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).line_height_at_start
    }

    /// Changing the value while the handles are visible causes them to rebuild.
    pub fn set_line_height_at_start(self: Handle<Self>, app: &mut App, value: f64) {
        if app.get(self).line_height_at_start == value {
            return;
        }
        app.get_mut(self).line_height_at_start = value;
        self.mark_needs_build(app);
    }

    /// Whether the selection start handle is currently being dragged.
    pub fn is_dragging_start_handle(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_dragging_start_handle || app.get(self).start_handle_drag_in_progress
    }

    fn can_drag_start_handle(self: Handle<Self>, app: &App) -> bool {
        !app.get(self).is_dragging_end_handle
            || (!matches!(
                app.platform().target_platform(),
                TargetPlatform::IOS | TargetPlatform::MacOS
            ) && !K_IS_WEB)
    }

    /// Called when the users start dragging the start selection handles.
    pub fn handle_start_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
    ) {
        debug_assert!(!app.get(self).is_dragging_start_handle);
        if app.get(self).handles.is_none() {
            app.get_mut(self).is_dragging_start_handle = false;
            return;
        }
        app.get_mut(self).start_handle_drag_in_progress = true;
        if !self.can_drag_start_handle(app) {
            return;
        }
        app.get_mut(self).is_dragging_start_handle = details.kind == Some(PointerDeviceKind::Touch);
        if let Some(callback) = app.get(self).on_start_handle_drag_start.clone() {
            callback(app, details);
        }
    }

    /// Called when the users drag the start selection handles to new locations.
    pub fn handle_start_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        details: DragUpdateDetails,
    ) {
        if app.get(self).handles.is_none() {
            app.get_mut(self).is_dragging_start_handle = false;
            return;
        }
        if !self.can_drag_start_handle(app) {
            return;
        }
        if !app.get(self).is_dragging_start_handle {
            app.get_mut(self).is_dragging_start_handle =
                details.kind == Some(PointerDeviceKind::Touch);
            let start_details = DragStartDetails::new(
                details.global_position,
                Some(details.local_position),
                details.source_time_stamp,
                details.kind,
            );
            if let Some(callback) = app.get(self).on_start_handle_drag_start.clone() {
                callback(app, start_details);
            }
        }
        if let Some(callback) = app.get(self).on_start_handle_drag_update.clone() {
            callback(app, details);
        }
    }

    /// Called when the users lift their fingers after dragging the start selection handles.
    pub fn handle_start_handle_drag_end(
        self: Handle<Self>,
        app: &mut App,
        details: DragEndDetails,
    ) {
        app.get_mut(self).is_dragging_start_handle = false;
        if app.get(self).handles.is_none() {
            return;
        }
        app.get_mut(self).start_handle_drag_in_progress = false;
        if !self.can_drag_start_handle(app) {
            return;
        }
        if let Some(callback) = app.get(self).on_start_handle_drag_end.clone() {
            callback(app, details);
        }
    }

    /// The type of end selection handle.
    pub fn end_handle_type(self: Handle<Self>, app: &App) -> TextSelectionHandleType {
        app.get(self).end_handle_type
    }

    /// Changing the value while the handles are visible causes them to rebuild.
    pub fn set_end_handle_type(self: Handle<Self>, app: &mut App, value: TextSelectionHandleType) {
        if app.get(self).end_handle_type == value {
            return;
        }
        app.get_mut(self).end_handle_type = value;
        self.mark_needs_build(app);
    }

    /// The line height at the selection end.
    pub fn line_height_at_end(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).line_height_at_end
    }

    /// Changing the value while the handles are visible causes them to rebuild.
    pub fn set_line_height_at_end(self: Handle<Self>, app: &mut App, value: f64) {
        if app.get(self).line_height_at_end == value {
            return;
        }
        app.get_mut(self).line_height_at_end = value;
        self.mark_needs_build(app);
    }

    /// Whether the selection end handle is currently being dragged.
    pub fn is_dragging_end_handle(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_dragging_end_handle || app.get(self).end_handle_drag_in_progress
    }

    fn can_drag_end_handle(self: Handle<Self>, app: &App) -> bool {
        !app.get(self).is_dragging_start_handle
            || (!matches!(
                app.platform().target_platform(),
                TargetPlatform::IOS | TargetPlatform::MacOS
            ) && !K_IS_WEB)
    }

    /// Called when the users start dragging the end selection handles.
    pub fn handle_end_handle_drag_start(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
    ) {
        debug_assert!(!app.get(self).is_dragging_end_handle);
        if app.get(self).handles.is_none() {
            app.get_mut(self).is_dragging_end_handle = false;
            return;
        }
        app.get_mut(self).end_handle_drag_in_progress = true;
        if !self.can_drag_end_handle(app) {
            return;
        }
        app.get_mut(self).is_dragging_end_handle = details.kind == Some(PointerDeviceKind::Touch);
        if let Some(callback) = app.get(self).on_end_handle_drag_start.clone() {
            callback(app, details);
        }
    }

    /// Called when the users drag the end selection handles to new locations.
    pub fn handle_end_handle_drag_update(
        self: Handle<Self>,
        app: &mut App,
        details: DragUpdateDetails,
    ) {
        if app.get(self).handles.is_none() {
            app.get_mut(self).is_dragging_end_handle = false;
            return;
        }
        if !self.can_drag_end_handle(app) {
            return;
        }
        if !app.get(self).is_dragging_end_handle {
            app.get_mut(self).is_dragging_end_handle =
                details.kind == Some(PointerDeviceKind::Touch);
            let start_details = DragStartDetails::new(
                details.global_position,
                Some(details.local_position),
                details.source_time_stamp,
                details.kind,
            );
            if let Some(callback) = app.get(self).on_end_handle_drag_start.clone() {
                callback(app, start_details);
            }
        }
        if let Some(callback) = app.get(self).on_end_handle_drag_update.clone() {
            callback(app, details);
        }
    }

    /// Called when the users lift their fingers after dragging the end selection handles.
    pub fn handle_end_handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        app.get_mut(self).is_dragging_end_handle = false;
        if app.get(self).handles.is_none() {
            return;
        }
        app.get_mut(self).end_handle_drag_in_progress = false;
        if !self.can_drag_end_handle(app) {
            return;
        }
        if let Some(callback) = app.get(self).on_end_handle_drag_end.clone() {
            callback(app, details);
        }
    }

    /// The text selection positions of selection start and end.
    pub fn selection_endpoints(self: Handle<Self>, app: &App) -> Vec<TextSelectionPoint> {
        app.get(self).selection_endpoints.clone()
    }

    /// Changing the endpoints while the handles are visible causes them to rebuild.
    pub fn set_selection_endpoints(
        self: Handle<Self>,
        app: &mut App,
        value: Vec<TextSelectionPoint>,
    ) {
        if app.get(self).selection_endpoints != value {
            self.mark_needs_build(app);
            if (app.get(self).is_dragging_end_handle || app.get(self).is_dragging_start_handle)
                && app.platform().target_platform() == TargetPlatform::Android
            {
                HapticFeedback::selection_click(app);
            }
        }
        app.get_mut(self).selection_endpoints = value;
    }

    /// The location of where the toolbar should be drawn relative to [`toolbar_layer_link`](Self::toolbar_layer_link).
    pub fn toolbar_location_value(self: Handle<Self>, app: &App) -> Option<Offset> {
        app.get(self).toolbar_location
    }

    /// Changing the location while the toolbar is visible causes it to rebuild.
    pub fn set_toolbar_location(self: Handle<Self>, app: &mut App, value: Option<Offset>) {
        if app.get(self).toolbar_location == value {
            return;
        }
        app.get_mut(self).toolbar_location = value;
        self.mark_needs_build(app);
    }

    /// Builds the handles by inserting them into the [`context`](Self::context)'s overlay.
    pub fn show_handles(self: Handle<Self>, app: &mut App) {
        if app.get(self).handles.is_some() {
            return;
        }
        let context = app.get(self).context;
        let overlay = Overlay::of(app, context, true);
        let this = self;
        let start = OverlayEntry::new(
            app,
            Rc::new(move |app, context| this.build_start_handle(app, context)),
            false,
            false,
            false,
        );
        let end = OverlayEntry::new(
            app,
            Rc::new(move |app, context| this.build_end_handle(app, context)),
            false,
            false,
            false,
        );
        app.get_mut(self).handles = Some(SelectionHandles { start, end });
        overlay.insert_all(app, vec![start, end], None, None);
    }

    /// Destroys the handles by removing them from overlay.
    pub fn hide_handles(self: Handle<Self>, app: &mut App) {
        let Some(handles) = app.get_mut(self).handles.take() else {
            return;
        };
        handles.start.remove(app);
        handles.start.dispose(app);
        handles.end.remove(app);
        handles.end.dispose(app);
    }

    /// Shows the toolbar by inserting it into the [`context`](Self::context)'s overlay.
    pub fn show_toolbar(
        self: Handle<Self>,
        app: &mut App,
        context: Option<BuildContext>,
        context_menu_builder: Option<WidgetBuilder>,
    ) {
        let Some(context_menu_builder) = context_menu_builder else {
            if app.get(self).toolbar.is_some() {
                return;
            }
            let this = self;
            let toolbar = OverlayEntry::new(
                app,
                Rc::new(move |app, context| this.build_toolbar(app, context)),
                false,
                false,
                false,
            );
            app.get_mut(self).toolbar = Some(toolbar);
            let overlay_context = app.get(self).context;
            let above = app.get(self).handles.map(|handles| handles.end);
            Overlay::of(app, overlay_context, true).insert(app, toolbar, None, above);
            return;
        };
        let Some(context) = context else {
            return;
        };
        if app.get(self).context_menu.is_some() {
            app.get_mut(self).context_menu_builder = Some(context_menu_builder);
            if let Some(entry) = app.get(self).context_menu {
                entry.mark_needs_build(app);
            }
            return;
        }
        app.get_mut(self).context_menu_builder = Some(Rc::clone(&context_menu_builder));
        let this = self;
        let field_context = context;
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app, overlay_context| {
                let child = app
                    .get(this)
                    .context_menu_builder
                    .clone()
                    .map(|builder| builder(app, overlay_context))
                    .unwrap_or_else(|| SizedBox::shrink().into_widget());
                let visibility = app.get(this).toolbar_visible;
                let layer_link = app.get(this).toolbar_layer_link;
                let offset = field_context
                    .find_render_object(app)
                    .and_then(|object| object.as_box())
                    .map(|box_| -box_.local_to_global(app, Offset::ZERO, None))
                    .unwrap_or(Offset::ZERO);
                let mut wrapper = SelectionToolbarWrapper::new(layer_link, offset, child);
                if let Some(visibility) = visibility {
                    wrapper = wrapper.visibility(visibility);
                }
                wrapper.into_widget()
            }),
            false,
            false,
            false,
        );
        app.get_mut(self).context_menu = Some(entry);
        Overlay::of(app, context, true).insert(app, entry, None, None);
    }

    /// Shows toolbar with spell check suggestions of misspelled words.
    pub fn show_spell_check_suggestions_toolbar(
        self: Handle<Self>,
        app: &mut App,
        context: Option<BuildContext>,
        builder: WidgetBuilder,
    ) {
        let Some(context) = context else {
            return;
        };
        app.get_mut(self).spell_check_toolbar_builder = Some(Rc::clone(&builder));
        if app.get(self).spell_check_toolbar.is_some() {
            if let Some(entry) = app.get(self).spell_check_toolbar {
                entry.mark_needs_build(app);
            }
            return;
        }
        let this = self;
        let field_context = context;
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app, overlay_context| {
                let child = app
                    .get(this)
                    .spell_check_toolbar_builder
                    .clone()
                    .map(|builder| builder(app, overlay_context))
                    .unwrap_or_else(|| SizedBox::shrink().into_widget());
                let layer_link = app.get(this).toolbar_layer_link;
                let offset = field_context
                    .find_render_object(app)
                    .and_then(|object| object.as_box())
                    .map(|box_| -box_.local_to_global(app, Offset::ZERO, None))
                    .unwrap_or(Offset::ZERO);
                SelectionToolbarWrapper::new(layer_link, offset, child).into_widget()
            }),
            false,
            false,
            false,
        );
        app.get_mut(self).spell_check_toolbar = Some(entry);
        Overlay::of(app, context, true).insert(app, entry, None, None);
    }

    /// Rebuilds the selection toolbar or handles if they are present.
    pub fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        if app.get(self).handles.is_none() && app.get(self).toolbar.is_none() {
            return;
        }
        if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::PersistentCallbacks {
            if app.get(self).build_scheduled {
                return;
            }
            app.get_mut(self).build_scheduled = true;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::mark_needs_build_after_frame),
            );
        } else {
            self.rebuild_overlay_entries(app);
        }
    }

    fn mark_needs_build_after_frame(self: Handle<Self>, app: &mut App, _time_stamp: Duration) {
        app.get_mut(self).build_scheduled = false;
        self.rebuild_overlay_entries(app);
    }

    fn rebuild_overlay_entries(self: Handle<Self>, app: &mut App) {
        if let Some(handles) = app.get(self).handles {
            handles.start.mark_needs_build(app);
            handles.end.mark_needs_build(app);
        }
        if let Some(toolbar) = app.get(self).toolbar {
            toolbar.mark_needs_build(app);
        }
        if let Some(entry) = app.get(self).context_menu {
            entry.mark_needs_build(app);
        } else if let Some(entry) = app.get(self).spell_check_toolbar {
            entry.mark_needs_build(app);
        }
    }

    /// Hides the entire overlay including the toolbar and the handles.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        let controller = app.get(self).magnifier_controller;
        controller.hide(app, true);
        self.hide_handles(app);
        if app.get(self).toolbar.is_some()
            || app.get(self).context_menu.is_some()
            || app.get(self).spell_check_toolbar.is_some()
        {
            self.hide_toolbar(app);
        }
    }

    /// Hides the toolbar part of the overlay.
    pub fn hide_toolbar(self: Handle<Self>, app: &mut App) {
        if let Some(entry) = app.get_mut(self).context_menu.take() {
            entry.remove(app);
            entry.dispose(app);
        }
        app.get_mut(self).context_menu_builder = None;
        if let Some(entry) = app.get_mut(self).spell_check_toolbar.take() {
            entry.remove(app);
            entry.dispose(app);
        }
        app.get_mut(self).spell_check_toolbar_builder = None;
        let Some(toolbar) = app.get_mut(self).toolbar.take() else {
            return;
        };
        toolbar.remove(app);
        toolbar.dispose(app);
    }

    /// Disposes this object and release resources.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        self.hide(app);
        app.get_mut(app.get(self).magnifier_info).dispose();
    }

    fn build_start_handle(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let controls = app.get(self).selection_controls.clone();
        let handle_type = app.get(self).start_handle_type;
        let dragging_end = app.get(self).is_dragging_end_handle;
        let handle = if controls.is_none()
            || (handle_type == TextSelectionHandleType::Collapsed && dragging_end)
        {
            SizedBox::shrink().into_widget()
        } else if let Some(controls) = controls {
            let line_height = app.get(self).line_height_at_start;
            let on_tap = app.get(self).on_selection_handle_tapped.clone();
            controls.build_handle(app, context, handle_type, line_height, on_tap)
        } else {
            SizedBox::shrink().into_widget()
        };
        TextFieldTapRegion::new(handle).into_widget()
    }

    fn build_end_handle(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let controls = app.get(self).selection_controls.clone();
        let handle_type = app.get(self).end_handle_type;
        let dragging_start = app.get(self).is_dragging_start_handle;
        let dragging_end = app.get(self).is_dragging_end_handle;
        let handle = if controls.is_none()
            || (handle_type == TextSelectionHandleType::Collapsed && dragging_start)
            || (handle_type == TextSelectionHandleType::Collapsed
                && !dragging_start
                && !dragging_end)
        {
            SizedBox::shrink().into_widget()
        } else if let Some(controls) = controls {
            let line_height = app.get(self).line_height_at_end;
            let on_tap = app.get(self).on_selection_handle_tapped.clone();
            controls.build_handle(app, context, handle_type, line_height, on_tap)
        } else {
            SizedBox::shrink().into_widget()
        };
        TextFieldTapRegion::new(handle).into_widget()
    }

    #[allow(deprecated)]
    fn build_toolbar(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let Some(controls) = app.get(self).selection_controls.clone() else {
            return TextFieldTapRegion::new(SizedBox::shrink()).into_widget();
        };
        let Some(delegate) = app.get(self).selection_delegate else {
            debug_assert!(
                false,
                "If not using contextMenuBuilder, must pass selectionDelegate."
            );
            return TextFieldTapRegion::new(SizedBox::shrink()).into_widget();
        };
        let overlay_context = app.get(self).context;
        let Some(render_box) = overlay_context
            .find_render_object(app)
            .and_then(|object| object.as_box())
        else {
            return TextFieldTapRegion::new(SizedBox::shrink()).into_widget();
        };
        let editing_region = Rect::from_points(
            render_box.local_to_global(app, Offset::ZERO, None),
            render_box.local_to_global(app, render_box.size(app).bottom_right(Offset::ZERO), None),
        );
        let endpoints = app.get(self).selection_endpoints.clone();
        if endpoints.is_empty() {
            return TextFieldTapRegion::new(SizedBox::shrink()).into_widget();
        }
        let line_height_at_end = app.get(self).line_height_at_end;
        let line_height_at_start = app.get(self).line_height_at_start;
        let is_multiline = endpoints[endpoints.len() - 1].point.dy() - endpoints[0].point.dy()
            > line_height_at_end / 2.0;
        let mid_x = if is_multiline {
            editing_region.width() / 2.0
        } else {
            (endpoints[0].point.dx() + endpoints[endpoints.len() - 1].point.dx()) / 2.0
        };
        let midpoint = Offset::new(mid_x, endpoints[0].point.dy() - line_height_at_start);
        let clipboard_status = app.get(self).clipboard_status;
        let toolbar_location = app.get(self).toolbar_location;
        let toolbar = controls.build_toolbar(
            app,
            context,
            editing_region,
            line_height_at_start,
            midpoint,
            &endpoints,
            delegate,
            clipboard_status,
            toolbar_location,
        );
        TextFieldTapRegion::new(toolbar).into_widget()
    }

    /// Update the current magnifier with new selection data.
    pub fn update_magnifier(self: Handle<Self>, app: &mut App, magnifier_info: MagnifierInfo) {
        let controller = app.get(self).magnifier_controller;
        if controller.overlay_entry(app).is_none() {
            return;
        }
        let info = app.get(self).magnifier_info;
        info.set_value(app, magnifier_info);
    }
}

/// Wrap the given child in the widgets common to both `contextMenuBuilder` and
/// [`TextSelectionControls::build_toolbar`].
///
/// Dart's `_SelectionToolbarWrapper`. Currently fades in but not out on all platforms.
///
/// The outer `TapRegion(groupId: SelectableRegion)` is omitted until `SelectableRegion` exists.
#[derive(Clone)]
pub(crate) struct SelectionToolbarWrapper {
    pub key: Option<KeyRef>,
    pub visibility: Option<Handle<ValueNotifier<bool>>>,
    pub layer_link: Handle<LayerLink>,
    pub offset: Offset,
    pub child: WidgetRef,
}

impl SelectionToolbarWrapper {
    pub fn new<K>(
        layer_link: Handle<LayerLink>,
        offset: Offset,
        child: impl IntoWidget<K>,
    ) -> SelectionToolbarWrapper {
        SelectionToolbarWrapper {
            key: None,
            visibility: None,
            layer_link,
            offset,
            child: child.into_widget(),
        }
    }

    pub fn visibility(
        mut self,
        visibility: Handle<ValueNotifier<bool>>,
    ) -> SelectionToolbarWrapper {
        self.visibility = Some(visibility);
        self
    }
}

impl Debug for SelectionToolbarWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionToolbarWrapper")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectionToolbarWrapper {
    type State = SelectionToolbarWrapperState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SelectionToolbarWrapperState {
        SelectionToolbarWrapperState {
            state: StateData::new(),
            controller: None,
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
        }
    }
}

/// Dart's `_SelectionToolbarWrapperState`.
pub(crate) struct SelectionToolbarWrapperState {
    state: StateData<SelectionToolbarWrapper>,
    controller: Option<Handle<AnimationController>>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
}

impl SelectionToolbarWrapperState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .expect("the controller is created in initState")
    }

    fn toolbar_visibility_changed(self: Handle<Self>, app: &mut App) {
        let visibility = self.widget(app).visibility;
        let visible = visibility
            .map(|visibility| *visibility.value(app))
            .unwrap_or(true);
        let controller = self.controller(app);
        if visible {
            controller.forward(app, None);
        } else {
            controller.reverse(app, None);
        }
    }
}

impl SingleTickerProviderStateMixin for SelectionToolbarWrapperState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for SelectionToolbarWrapperState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SelectionToolbarWrapperState {
    type Widget = SelectionToolbarWrapper;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            None,
            Some(SelectionOverlay::FADE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
        self.toolbar_visibility_changed(app);
        if let Some(visibility) = self.widget(app).visibility {
            visibility.add_listener(
                app,
                Listener::handle_method(self, Self::toolbar_visibility_changed),
            );
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SelectionToolbarWrapper) {
        if old_widget.visibility == self.widget(app).visibility {
            return;
        }
        if let Some(visibility) = old_widget.visibility {
            visibility.remove_listener(
                app,
                &Listener::handle_method(self, Self::toolbar_visibility_changed),
            );
        }
        self.toolbar_visibility_changed(app);
        if let Some(visibility) = self.widget(app).visibility {
            visibility.add_listener(
                app,
                Listener::handle_method(self, Self::toolbar_visibility_changed),
            );
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(visibility) = self.widget(app).visibility {
            visibility.remove_listener(
                app,
                &Listener::handle_method(self, Self::toolbar_visibility_changed),
            );
        }
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        app.get_mut(self).controller = None;
        app.destroy(controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let opacity = self.controller(app).view();
        let widget = self.widget(app);
        let (layer_link, offset, child) = (widget.layer_link, widget.offset, widget.child.clone());
        let text_direction = Directionality::of(app, context);
        TextFieldTapRegion::new(
            Directionality::new(
                text_direction,
                FadeTransition::new(opacity).child(
                    CompositedTransformFollower::new(layer_link)
                        .show_when_unlinked(false)
                        .offset(offset)
                        .child(child),
                ),
            ),
        )
        .into_widget()
    }
}

/// [GlobalKey] to the [`EditableText`] for which the
/// [`TextSelectionGestureDetectorBuilder`] will build a [`TextSelectionGestureDetector`].
pub trait TextSelectionGestureDetectorBuilderDelegate: Sized + 'static {
    /// [GlobalKey] to the [`EditableText`] for which the
    /// [`TextSelectionGestureDetectorBuilder`] will build a [`TextSelectionGestureDetector`].
    fn editable_text_key(self: Handle<Self>, app: &App) -> crate::framework::GlobalKey;

    /// Whether the text field should respond to force presses.
    fn force_press_enabled(self: Handle<Self>, app: &App) -> bool;

    /// Whether the user may select text in the text field.
    fn selection_enabled(self: Handle<Self>, app: &App) -> bool;

    /// This delegate as the erased [`AnyTextSelectionGestureDetectorBuilderDelegate`].
    fn as_text_selection_gesture_detector_builder_delegate(
        self: Handle<Self>,
    ) -> AnyTextSelectionGestureDetectorBuilderDelegate {
        AnyTextSelectionGestureDetectorBuilderDelegate {
            id: self.id(),
            vtable: const { &TextSelectionGestureDetectorBuilderDelegateVTable::of::<Self>() },
        }
    }
}

struct TextSelectionGestureDetectorBuilderDelegateVTable {
    editable_text_key: fn(&App, HandleId) -> crate::framework::GlobalKey,
    force_press_enabled: fn(&App, HandleId) -> bool,
    selection_enabled: fn(&App, HandleId) -> bool,
}

impl TextSelectionGestureDetectorBuilderDelegateVTable {
    const fn of<D: TextSelectionGestureDetectorBuilderDelegate>()
    -> TextSelectionGestureDetectorBuilderDelegateVTable {
        TextSelectionGestureDetectorBuilderDelegateVTable {
            editable_text_key: |app, id| D::editable_text_key(resolve_builder_delegate(id), app),
            force_press_enabled: |app, id| {
                D::force_press_enabled(resolve_builder_delegate(id), app)
            },
            selection_enabled: |app, id| D::selection_enabled(resolve_builder_delegate(id), app),
        }
    }
}

fn resolve_builder_delegate<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

/// Erased [`TextSelectionGestureDetectorBuilderDelegate`]: one identity and a static vtable.
#[derive(Clone, Copy)]
pub struct AnyTextSelectionGestureDetectorBuilderDelegate {
    id: HandleId,
    vtable: &'static TextSelectionGestureDetectorBuilderDelegateVTable,
}

impl AnyTextSelectionGestureDetectorBuilderDelegate {
    pub fn editable_text_key(self, app: &App) -> crate::framework::GlobalKey {
        (self.vtable.editable_text_key)(app, self.id)
    }

    pub fn force_press_enabled(self, app: &App) -> bool {
        (self.vtable.force_press_enabled)(app, self.id)
    }

    pub fn selection_enabled(self, app: &App) -> bool {
        (self.vtable.selection_enabled)(app, self.id)
    }
}

/// Field bag of [`TextSelectionGestureDetectorBuilder`].
pub struct TextSelectionGestureDetectorBuilderData {
    /// The delegate for this [`TextSelectionGestureDetectorBuilder`].
    pub delegate: AnyTextSelectionGestureDetectorBuilderDelegate,
    should_show_selection_toolbar: bool,
    should_show_selection_handles: bool,
    is_shift_pressed: bool,
    drag_start_scroll_offset: f64,
    drag_start_viewport_offset: f64,
    drag_start_selection: Option<TextSelection>,
    long_press_started_without_focus: bool,
}

impl TextSelectionGestureDetectorBuilderData {
    /// Creates a [`TextSelectionGestureDetectorBuilderData`].
    pub fn new(
        delegate: AnyTextSelectionGestureDetectorBuilderDelegate,
    ) -> TextSelectionGestureDetectorBuilderData {
        TextSelectionGestureDetectorBuilderData {
            delegate,
            should_show_selection_toolbar: true,
            should_show_selection_handles: true,
            is_shift_pressed: false,
            drag_start_scroll_offset: 0.0,
            drag_start_viewport_offset: 0.0,
            drag_start_selection: None,
            long_press_started_without_focus: false,
        }
    }
}

/// Accessors for [`TextSelectionGestureDetectorBuilderData`] on a leaf builder.
#[macro_export]
macro_rules! text_selection_gesture_detector_builder_accessors {
    ($field:ident) => {
        fn builder_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::TextSelectionGestureDetectorBuilderData {
            &app.get(self).$field
        }
        fn builder_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::TextSelectionGestureDetectorBuilderData {
            &mut app.get_mut(self).$field
        }
    };
}

/// Builds a [`TextSelectionGestureDetector`] to wrap an [`EditableText`].
pub trait TextSelectionGestureDetectorBuilder: Sized + 'static {
    /// The builder's field bag.
    fn builder_data(self: Handle<Self>, app: &App) -> &TextSelectionGestureDetectorBuilderData;
    /// The builder's field bag, mutably.
    fn builder_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TextSelectionGestureDetectorBuilderData;

    /// Handler for [`TextSelectionGestureDetector::on_tap_track_start`].
    fn on_tap_track_start(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_tap_track_start(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_tap_track_reset`].
    fn on_tap_track_reset(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_tap_track_reset(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_tap_down`].
    fn on_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        TextSelectionGestureDetectorBuilderBase::on_tap_down(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_force_press_start`].
    fn on_force_press_start(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        TextSelectionGestureDetectorBuilderBase::on_force_press_start(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_force_press_end`].
    fn on_force_press_end(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        TextSelectionGestureDetectorBuilderBase::on_force_press_end(self, app, details);
    }

    /// Whether the provided [`on_user_tap`](Self::on_user_tap) callback should be dispatched on every
    /// tap or only non-consecutive taps.
    fn on_user_tap_always_called(self: Handle<Self>, app: &App) -> bool {
        TextSelectionGestureDetectorBuilderBase::on_user_tap_always_called(self, app)
    }

    /// Handler for [`TextSelectionGestureDetector::on_user_tap`].
    fn on_user_tap(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_user_tap(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_tap_up`].
    fn on_single_tap_up(self: Handle<Self>, app: &mut App, details: TapDragUpDetails) {
        TextSelectionGestureDetectorBuilderBase::on_single_tap_up(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_tap_cancel`].
    fn on_single_tap_cancel(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_single_tap_cancel(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_start`].
    fn on_single_long_tap_start(self: Handle<Self>, app: &mut App, details: LongPressStartDetails) {
        TextSelectionGestureDetectorBuilderBase::on_single_long_tap_start(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_move_update`].
    fn on_single_long_tap_move_update(
        self: Handle<Self>,
        app: &mut App,
        details: LongPressMoveUpdateDetails,
    ) {
        TextSelectionGestureDetectorBuilderBase::on_single_long_tap_move_update(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_end`].
    fn on_single_long_tap_end(self: Handle<Self>, app: &mut App, details: LongPressEndDetails) {
        TextSelectionGestureDetectorBuilderBase::on_single_long_tap_end(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_cancel`].
    fn on_single_long_tap_cancel(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_single_long_tap_cancel(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_secondary_tap`].
    fn on_secondary_tap(self: Handle<Self>, app: &mut App) {
        TextSelectionGestureDetectorBuilderBase::on_secondary_tap(self, app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_secondary_tap_down`].
    fn on_secondary_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        TextSelectionGestureDetectorBuilderBase::on_secondary_tap_down(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_double_tap_down`].
    fn on_double_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        TextSelectionGestureDetectorBuilderBase::on_double_tap_down(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_triple_tap_down`].
    fn on_triple_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        TextSelectionGestureDetectorBuilderBase::on_triple_tap_down(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_start`].
    fn on_drag_selection_start(self: Handle<Self>, app: &mut App, details: TapDragStartDetails) {
        TextSelectionGestureDetectorBuilderBase::on_drag_selection_start(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_update`].
    fn on_drag_selection_update(self: Handle<Self>, app: &mut App, details: TapDragUpdateDetails) {
        TextSelectionGestureDetectorBuilderBase::on_drag_selection_update(self, app, details);
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_end`].
    fn on_drag_selection_end(self: Handle<Self>, app: &mut App, details: TapDragEndDetails) {
        TextSelectionGestureDetectorBuilderBase::on_drag_selection_end(self, app, details);
    }

    /// Returns a [`TextSelectionGestureDetector`] configured with the handlers
    /// provided by this builder.
    fn build_gesture_detector(
        self: Handle<Self>,
        app: &mut App,
        key: Option<KeyRef>,
        behavior: Option<HitTestBehavior>,
        child: WidgetRef,
    ) -> TextSelectionGestureDetector {
        TextSelectionGestureDetectorBuilderBase::build_gesture_detector(
            self, app, key, behavior, child,
        )
    }
}

/// Dart bodies of [`TextSelectionGestureDetectorBuilder`].
pub trait TextSelectionGestureDetectorBuilderBase: TextSelectionGestureDetectorBuilder {
    /// Shows the magnifier on supported platforms at the given offset, currently
    /// only Android and iOS.
    fn show_magnifier_if_supported_by_platform(
        self: Handle<Self>,
        app: &mut App,
        position_to_show: Offset,
    ) {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::IOS => {
                self.editable_text(app)
                    .show_magnifier(app, position_to_show);
            }
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }

    /// Hides the magnifier on supported platforms, currently only Android and iOS.
    fn hide_magnifier_if_supported_by_platform(self: Handle<Self>, app: &mut App) {
        if !self.is_editable_text_mounted(app) {
            return;
        }
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::IOS => {
                self.editable_text(app).hide_magnifier(app);
            }
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }

    /// Returns true if lastSecondaryTapDownPosition was on selection.
    fn last_secondary_tap_was_on_selection(self: Handle<Self>, app: &mut App) -> bool {
        let render_editable = self.render_editable(app);
        debug_assert!(
            render_editable
                .last_secondary_tap_down_position(app)
                .is_some()
        );
        let Some(selection) = render_editable.selection(app) else {
            return false;
        };
        let position = render_editable.get_position_for_point(
            app,
            render_editable
                .last_secondary_tap_down_position(app)
                .expect("lastSecondaryTapDownPosition"),
        );
        selection.start() <= position.offset && selection.end() >= position.offset
    }

    fn position_was_on_selection_exclusive(
        self: Handle<Self>,
        app: &mut App,
        text_position: TextPosition,
    ) -> bool {
        let Some(selection) = self.render_editable(app).selection(app) else {
            return false;
        };
        selection.start() < text_position.offset && selection.end() > text_position.offset
    }

    fn position_was_on_selection_inclusive(
        self: Handle<Self>,
        app: &mut App,
        text_position: TextPosition,
    ) -> bool {
        let Some(selection) = self.render_editable(app).selection(app) else {
            return false;
        };
        selection.start() <= text_position.offset && selection.end() >= text_position.offset
    }

    fn expand_selection(
        self: Handle<Self>,
        app: &mut App,
        offset: Offset,
        cause: SelectionChangedCause,
        from_selection: Option<TextSelection>,
    ) {
        let render_editable = self.render_editable(app);
        debug_assert!(render_editable.selection(app).is_some());
        let tapped_position = render_editable.get_position_for_point(app, offset);
        let selection = from_selection.unwrap_or_else(|| {
            render_editable
                .selection(app)
                .expect("expand_selection needs a selection")
        });
        let base_is_closer = (tapped_position.offset - selection.base_offset).abs()
            < (tapped_position.offset - selection.extent_offset).abs();
        let next_selection = selection
            .copy_with()
            .base_offset(if base_is_closer {
                selection.extent_offset
            } else {
                selection.base_offset
            })
            .extent_offset(tapped_position.offset);
        let editable = self.editable_text(app);
        let value = editable
            .text_editing_value(app)
            .copy_with()
            .selection(next_selection);
        editable.user_update_text_editing_value(app, value, cause);
    }

    fn extend_selection(
        self: Handle<Self>,
        app: &mut App,
        offset: Offset,
        cause: SelectionChangedCause,
    ) {
        let render_editable = self.render_editable(app);
        debug_assert!(render_editable.selection(app).is_some());
        let tapped_position = render_editable.get_position_for_point(app, offset);
        let selection = render_editable
            .selection(app)
            .expect("extend_selection needs a selection");
        let next_selection = selection.copy_with().extent_offset(tapped_position.offset);
        let editable = self.editable_text(app);
        let value = editable
            .text_editing_value(app)
            .copy_with()
            .selection(next_selection);
        editable.user_update_text_editing_value(app, value, cause);
    }

    /// Whether to show the selection toolbar.
    fn should_show_selection_toolbar(self: Handle<Self>, app: &App) -> bool {
        self.builder_data(app).should_show_selection_toolbar
    }

    /// Whether to show the selection handles.
    fn should_show_selection_handles(self: Handle<Self>, app: &App) -> bool {
        self.builder_data(app).should_show_selection_handles
    }

    /// The [`State`] of the [`EditableText`] for which the builder will provide a
    /// [`TextSelectionGestureDetector`].
    fn editable_text(self: Handle<Self>, app: &mut App) -> Handle<EditableTextState> {
        let key = self.builder_data(app).delegate.editable_text_key(app);
        key.current_state::<EditableTextState>(app)
            .expect("editableTextKey.currentState")
    }

    /// The [`RenderObject`] of the [`EditableText`] for which the builder will
    /// provide a [`TextSelectionGestureDetector`].
    fn render_editable(self: Handle<Self>, app: &mut App) -> RenderHandle<RenderEditable> {
        self.editable_text(app).render_editable(app)
    }

    /// Returns `true` if a widget with the global key [delegate.editableTextKey]
    /// is in the tree and the widget is mounted.
    fn is_editable_text_mounted(self: Handle<Self>, app: &mut App) -> bool {
        let key = self.builder_data(app).delegate.editable_text_key(app);
        key.current_context(app)
            .is_some_and(|context| context.mounted(app))
    }

    fn scroll_position(self: Handle<Self>, app: &mut App) -> f64 {
        let key = self.builder_data(app).delegate.editable_text_key(app);
        let Some(context) = key.current_context(app) else {
            return 0.0;
        };
        Scrollable::maybe_of(app, context, None)
            .map(|scrollable| scrollable.position(app).pixels(app))
            .unwrap_or(0.0)
    }

    fn scroll_direction(self: Handle<Self>, app: &mut App) -> Option<AxisDirection> {
        let key = self.builder_data(app).delegate.editable_text_key(app);
        let context = key.current_context(app)?;
        Some(
            Scrollable::maybe_of(app, context, None)?
                .widget(app)
                .axis_direction,
        )
    }

    /// Handler for [`TextSelectionGestureDetector::on_tap_track_start`].
    fn on_tap_track_start(self: Handle<Self>, app: &mut App) {
        let pressed = HardwareKeyboard::instance(app).logical_keys_pressed(app);
        let is_shift = pressed.contains(&LogicalKeyboardKey::SHIFT_LEFT)
            || pressed.contains(&LogicalKeyboardKey::SHIFT_RIGHT);
        self.builder_data_mut(app).is_shift_pressed = is_shift;
    }

    /// Handler for [`TextSelectionGestureDetector::on_tap_track_reset`].
    fn on_tap_track_reset(self: Handle<Self>, app: &mut App) {
        self.builder_data_mut(app).is_shift_pressed = false;
    }

    /// Handler for [`TextSelectionGestureDetector::on_tap_down`].
    fn on_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        let render_editable = self.render_editable(app);
        render_editable.handle_tap_down(
            app,
            &TapDownDetails::new(details.global_position, None, details.kind),
        );
        let kind = details.kind;
        let should_show = kind.is_none()
            || kind == Some(PointerDeviceKind::Touch)
            || kind == Some(PointerDeviceKind::Stylus);
        self.builder_data_mut(app).should_show_selection_toolbar = should_show;
        self.builder_data_mut(app).should_show_selection_handles = should_show;

        let is_shift_pressed = self.builder_data(app).is_shift_pressed;
        let is_shift_pressed_valid = is_shift_pressed
            && self
                .render_editable(app)
                .selection(app)
                .is_some_and(|selection| selection.is_valid());
        match app.platform().target_platform() {
            TargetPlatform::Android => {}
            TargetPlatform::Fuchsia | TargetPlatform::IOS => {}
            TargetPlatform::MacOS => {
                self.editable_text(app).hide_toolbar(app, true);
                if is_shift_pressed_valid {
                    let from_selection = if self.render_editable(app).has_focus(app) {
                        None
                    } else {
                        Some(TextSelection::collapsed(0, TextAffinity::Downstream))
                    };
                    self.expand_selection(
                        app,
                        details.global_position,
                        SelectionChangedCause::Tap,
                        from_selection,
                    );
                    return;
                }
                self.render_editable(app)
                    .select_position(app, SelectionChangedCause::Tap);
            }
            TargetPlatform::Linux | TargetPlatform::Windows => {
                self.editable_text(app).hide_toolbar(app, true);
                if is_shift_pressed_valid {
                    self.extend_selection(app, details.global_position, SelectionChangedCause::Tap);
                    return;
                }
                self.render_editable(app)
                    .select_position(app, SelectionChangedCause::Tap);
            }
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_force_press_start`].
    fn on_force_press_start(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        debug_assert!(self.builder_data(app).delegate.force_press_enabled(app));
        self.builder_data_mut(app).should_show_selection_toolbar = true;
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        self.render_editable(app).select_words_in_range(
            app,
            details.global_position,
            None,
            SelectionChangedCause::ForcePress,
        );
        let _ = self.editable_text(app).show_toolbar(app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_force_press_end`].
    fn on_force_press_end(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        debug_assert!(self.builder_data(app).delegate.force_press_enabled(app));
        self.render_editable(app).select_words_in_range(
            app,
            details.global_position,
            None,
            SelectionChangedCause::ForcePress,
        );
        if self.should_show_selection_toolbar(app) {
            let _ = self.editable_text(app).show_toolbar(app);
        }
    }

    /// Whether the provided [`on_user_tap`](TextSelectionGestureDetectorBuilder::on_user_tap) callback should be dispatched on every
    /// tap or only non-consecutive taps.
    fn on_user_tap_always_called(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    /// Handler for [`TextSelectionGestureDetector::on_user_tap`].
    fn on_user_tap(self: Handle<Self>, _app: &mut App) {}

    /// Handler for [`TextSelectionGestureDetector::on_single_tap_up`].
    fn on_single_tap_up(self: Handle<Self>, app: &mut App, details: TapDragUpDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            self.editable_text(app).request_keyboard(app);
            return;
        }
        let is_shift_pressed = self.builder_data(app).is_shift_pressed;
        let is_shift_pressed_valid = is_shift_pressed
            && self
                .render_editable(app)
                .selection(app)
                .is_some_and(|selection| selection.is_valid());
        match app.platform().target_platform() {
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {}
            TargetPlatform::Android => {
                self.editable_text(app).hide_toolbar(app, false);
                if is_shift_pressed_valid {
                    self.extend_selection(app, details.global_position, SelectionChangedCause::Tap);
                    return;
                }
                self.render_editable(app)
                    .select_position(app, SelectionChangedCause::Tap);
                let _ = self
                    .editable_text(app)
                    .show_spell_check_suggestions_toolbar(app);
            }
            TargetPlatform::Fuchsia => {
                self.editable_text(app).hide_toolbar(app, false);
                if is_shift_pressed_valid {
                    self.extend_selection(app, details.global_position, SelectionChangedCause::Tap);
                    return;
                }
                self.render_editable(app)
                    .select_position(app, SelectionChangedCause::Tap);
            }
            TargetPlatform::IOS => {
                if is_shift_pressed_valid {
                    let from_selection = if self.render_editable(app).has_focus(app) {
                        None
                    } else {
                        Some(TextSelection::collapsed(0, TextAffinity::Downstream))
                    };
                    self.expand_selection(
                        app,
                        details.global_position,
                        SelectionChangedCause::Tap,
                        from_selection,
                    );
                    return;
                }
                match details.kind {
                    PointerDeviceKind::Mouse
                    | PointerDeviceKind::Trackpad
                    | PointerDeviceKind::Stylus
                    | PointerDeviceKind::InvertedStylus => {
                        self.render_editable(app)
                            .select_position(app, SelectionChangedCause::Tap);
                        self.editable_text(app).hide_toolbar(app, true);
                    }
                    PointerDeviceKind::Touch | PointerDeviceKind::Unknown => {
                        let render_editable = self.render_editable(app);
                        let previous_selection =
                            render_editable.selection(app).unwrap_or_else(|| {
                                self.editable_text(app).text_editing_value(app).selection
                            });
                        let text_position =
                            render_editable.get_position_for_point(app, details.global_position);
                        let is_affinity_the_same =
                            text_position.affinity == previous_selection.affinity;
                        let word_at_cursor_index_is_misspelled = self
                            .editable_text(app)
                            .find_suggestion_span_at_cursor_index(app, text_position.offset)
                            .is_some();
                        if word_at_cursor_index_is_misspelled {
                            self.render_editable(app)
                                .select_word(app, SelectionChangedCause::Tap);
                            if previous_selection
                                != self.editable_text(app).text_editing_value(app).selection
                            {
                                let _ = self
                                    .editable_text(app)
                                    .show_spell_check_suggestions_toolbar(app);
                            } else {
                                self.editable_text(app).toggle_toolbar(app, false);
                            }
                        } else if ((self.position_was_on_selection_exclusive(app, text_position)
                            && !previous_selection.is_collapsed())
                            || (self.position_was_on_selection_inclusive(app, text_position)
                                && previous_selection.is_collapsed()
                                && is_affinity_the_same
                                && !self.render_editable(app).read_only(app)))
                            && self.render_editable(app).has_focus(app)
                        {
                            self.editable_text(app).toggle_toolbar(app, false);
                        } else {
                            self.render_editable(app)
                                .select_word_edge(app, SelectionChangedCause::Tap);
                            if previous_selection
                                == self.editable_text(app).text_editing_value(app).selection
                                && self.render_editable(app).has_focus(app)
                                && !self.render_editable(app).read_only(app)
                            {
                                self.editable_text(app).toggle_toolbar(app, false);
                            } else {
                                self.editable_text(app).hide_toolbar(app, false);
                            }
                        }
                    }
                }
            }
        }
        self.editable_text(app).request_keyboard(app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_tap_cancel`].
    fn on_single_tap_cancel(self: Handle<Self>, _app: &mut App) {}

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_start`].
    fn on_single_long_tap_start(self: Handle<Self>, app: &mut App, details: LongPressStartDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                if !self.render_editable(app).has_focus(app) {
                    self.builder_data_mut(app).long_press_started_without_focus = true;
                    self.render_editable(app)
                        .select_word(app, SelectionChangedCause::LongPress);
                } else if self.render_editable(app).read_only(app) {
                    self.render_editable(app)
                        .select_word(app, SelectionChangedCause::LongPress);
                    let context = self.editable_text(app).context(app);
                    if self.editable_text(app).mounted(app) {
                        Feedback::for_long_press(app, context);
                    }
                } else {
                    self.render_editable(app).select_position_at(
                        app,
                        details.global_position,
                        None,
                        SelectionChangedCause::LongPress,
                    );
                }
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                self.render_editable(app)
                    .select_word(app, SelectionChangedCause::LongPress);
                let context = self.editable_text(app).context(app);
                if self.editable_text(app).mounted(app) {
                    Feedback::for_long_press(app, context);
                }
            }
        }
        self.show_magnifier_if_supported_by_platform(app, details.global_position);
        let pixels = self.render_editable(app).offset(app).pixels(app);
        self.builder_data_mut(app).drag_start_viewport_offset = pixels;
        self.builder_data_mut(app).drag_start_scroll_offset = self.scroll_position(app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_move_update`].
    fn on_single_long_tap_move_update(
        self: Handle<Self>,
        app: &mut App,
        details: LongPressMoveUpdateDetails,
    ) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        let drag_start_viewport = self.builder_data(app).drag_start_viewport_offset;
        let drag_start_scroll = self.builder_data(app).drag_start_scroll_offset;
        let render_editable = self.render_editable(app);
        let editable_offset = if render_editable.max_lines(app) == Some(1) {
            Offset::new(
                render_editable.offset(app).pixels(app) - drag_start_viewport,
                0.0,
            )
        } else {
            Offset::new(
                0.0,
                render_editable.offset(app).pixels(app) - drag_start_viewport,
            )
        };
        let scroll_position = self.scroll_position(app);
        let scrollable_offset =
            match axis_direction_to_axis(self.scroll_direction(app).unwrap_or(AxisDirection::Left))
            {
                Axis::Horizontal => Offset::new(scroll_position - drag_start_scroll, 0.0),
                Axis::Vertical => Offset::new(0.0, scroll_position - drag_start_scroll),
            };
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                if self.builder_data(app).long_press_started_without_focus
                    || self.render_editable(app).read_only(app)
                {
                    self.render_editable(app).select_words_in_range(
                        app,
                        details.global_position
                            - details.offset_from_origin
                            - editable_offset
                            - scrollable_offset,
                        Some(details.global_position),
                        SelectionChangedCause::LongPress,
                    );
                } else {
                    self.render_editable(app).select_position_at(
                        app,
                        details.global_position,
                        None,
                        SelectionChangedCause::LongPress,
                    );
                }
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                self.render_editable(app).select_words_in_range(
                    app,
                    details.global_position
                        - details.offset_from_origin
                        - editable_offset
                        - scrollable_offset,
                    Some(details.global_position),
                    SelectionChangedCause::LongPress,
                );
            }
        }
        self.show_magnifier_if_supported_by_platform(app, details.global_position);
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_end`].
    fn on_single_long_tap_end(self: Handle<Self>, app: &mut App, _details: LongPressEndDetails) {
        self.on_single_long_tap_end_or_cancel(app);
        if self.should_show_selection_toolbar(app) {
            let _ = self.editable_text(app).show_toolbar(app);
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_single_long_tap_cancel`].
    fn on_single_long_tap_cancel(self: Handle<Self>, app: &mut App) {
        self.on_single_long_tap_end_or_cancel(app);
    }

    /// Handler for [`TextSelectionGestureDetector::on_secondary_tap`].
    fn on_secondary_tap(self: Handle<Self>, app: &mut App) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                if !self.last_secondary_tap_was_on_selection(app)
                    || !self.render_editable(app).has_focus(app)
                {
                    self.render_editable(app)
                        .select_word(app, SelectionChangedCause::Tap);
                }
                if self.should_show_selection_toolbar(app) {
                    self.editable_text(app).hide_toolbar(app, true);
                    let _ = self.editable_text(app).show_toolbar(app);
                }
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                if !self.render_editable(app).has_focus(app) {
                    self.render_editable(app)
                        .select_position(app, SelectionChangedCause::Tap);
                }
                self.editable_text(app).toggle_toolbar(app, true);
            }
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_secondary_tap_down`].
    fn on_secondary_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        self.render_editable(app).handle_secondary_tap_down(
            app,
            &TapDownDetails::new(details.global_position, None, details.kind),
        );
        self.builder_data_mut(app).should_show_selection_toolbar = true;
        self.builder_data_mut(app).should_show_selection_handles = details.kind.is_none()
            || details.kind == Some(PointerDeviceKind::Touch)
            || details.kind == Some(PointerDeviceKind::Stylus);
    }

    /// Handler for [`TextSelectionGestureDetector::on_double_tap_down`].
    fn on_double_tap_down(self: Handle<Self>, app: &mut App, _details: TapDragDownDetails) {
        if self.builder_data(app).delegate.selection_enabled(app) {
            self.render_editable(app)
                .select_word(app, SelectionChangedCause::DoubleTap);
            if self.should_show_selection_toolbar(app) {
                let _ = self.editable_text(app).show_toolbar(app);
            }
        }
    }

    fn on_single_long_tap_end_or_cancel(self: Handle<Self>, app: &mut App) {
        self.hide_magnifier_if_supported_by_platform(app);
        self.builder_data_mut(app).long_press_started_without_focus = false;
        self.builder_data_mut(app).drag_start_viewport_offset = 0.0;
        self.builder_data_mut(app).drag_start_scroll_offset = 0.0;
    }

    fn select_paragraphs_in_range(
        self: Handle<Self>,
        app: &mut App,
        from: Offset,
        to: Option<Offset>,
        cause: Option<SelectionChangedCause>,
    ) {
        let text = self.editable_text(app).text_editing_value(app).text;
        select_text_boundaries_in_range(self, app, BoundaryKind::Paragraph(&text), from, to, cause);
    }

    fn select_lines_in_range(
        self: Handle<Self>,
        app: &mut App,
        from: Offset,
        to: Option<Offset>,
        cause: Option<SelectionChangedCause>,
    ) {
        select_text_boundaries_in_range(self, app, BoundaryKind::Line, from, to, cause);
    }

    /// Handler for [`TextSelectionGestureDetector::on_triple_tap_down`].
    fn on_triple_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        if self.render_editable(app).max_lines(app) == Some(1) {
            self.editable_text(app)
                .select_all(app, SelectionChangedCause::Tap);
        } else {
            match app.platform().target_platform() {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::IOS
                | TargetPlatform::MacOS
                | TargetPlatform::Windows => {
                    self.select_paragraphs_in_range(
                        app,
                        details.global_position,
                        None,
                        Some(SelectionChangedCause::Tap),
                    );
                }
                TargetPlatform::Linux => {
                    self.select_lines_in_range(
                        app,
                        details.global_position,
                        None,
                        Some(SelectionChangedCause::Tap),
                    );
                }
            }
        }
        if self.should_show_selection_toolbar(app) {
            let _ = self.editable_text(app).show_toolbar(app);
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_start`].
    fn on_drag_selection_start(self: Handle<Self>, app: &mut App, details: TapDragStartDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        let kind = details.kind;
        let should_show = kind.is_none()
            || kind == Some(PointerDeviceKind::Touch)
            || kind == Some(PointerDeviceKind::Stylus);
        self.builder_data_mut(app).should_show_selection_toolbar = should_show;
        self.builder_data_mut(app).should_show_selection_handles = should_show;
        self.builder_data_mut(app).drag_start_selection = self.render_editable(app).selection(app);
        self.builder_data_mut(app).drag_start_scroll_offset = self.scroll_position(app);
        self.builder_data_mut(app).drag_start_viewport_offset =
            self.render_editable(app).offset(app).pixels(app);

        if TextSelectionGestureDetectorState::get_effective_consecutive_tap_count(
            app,
            details.consecutive_tap_count,
        ) > 1
        {
            return;
        }

        let selection = self.render_editable(app).selection(app);
        if self.builder_data(app).is_shift_pressed && selection.is_some_and(|s| s.is_valid()) {
            match app.platform().target_platform() {
                TargetPlatform::IOS | TargetPlatform::MacOS => {
                    self.expand_selection(
                        app,
                        details.global_position,
                        SelectionChangedCause::Drag,
                        None,
                    );
                }
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {
                    self.extend_selection(
                        app,
                        details.global_position,
                        SelectionChangedCause::Drag,
                    );
                }
            }
        } else {
            match app.platform().target_platform() {
                TargetPlatform::IOS => match details.kind {
                    Some(PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad) => {
                        self.render_editable(app).select_position_at(
                            app,
                            details.global_position,
                            None,
                            SelectionChangedCause::Drag,
                        );
                    }
                    Some(
                        PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus
                        | PointerDeviceKind::Touch
                        | PointerDeviceKind::Unknown,
                    )
                    | None => {}
                },
                TargetPlatform::Android | TargetPlatform::Fuchsia => match details.kind {
                    Some(PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad) => {
                        self.render_editable(app).select_position_at(
                            app,
                            details.global_position,
                            None,
                            SelectionChangedCause::Drag,
                        );
                    }
                    Some(
                        PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus
                        | PointerDeviceKind::Touch
                        | PointerDeviceKind::Unknown,
                    ) if self.render_editable(app).has_focus(app) => {
                        self.render_editable(app).select_position_at(
                            app,
                            details.global_position,
                            None,
                            SelectionChangedCause::Drag,
                        );
                        self.show_magnifier_if_supported_by_platform(
                            app,
                            details.global_position,
                        );
                    }
                    _ => {}
                },
                TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                    self.render_editable(app).select_position_at(
                        app,
                        details.global_position,
                        None,
                        SelectionChangedCause::Drag,
                    );
                }
            }
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_update`].
    fn on_drag_selection_update(self: Handle<Self>, app: &mut App, details: TapDragUpdateDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        if !self.builder_data(app).is_shift_pressed {
            let drag_start_viewport = self.builder_data(app).drag_start_viewport_offset;
            let drag_start_scroll = self.builder_data(app).drag_start_scroll_offset;
            let render_editable = self.render_editable(app);
            let editable_offset = if render_editable.max_lines(app) == Some(1) {
                Offset::new(
                    render_editable.offset(app).pixels(app) - drag_start_viewport,
                    0.0,
                )
            } else {
                Offset::new(
                    0.0,
                    render_editable.offset(app).pixels(app) - drag_start_viewport,
                )
            };
            let scroll_position = self.scroll_position(app);
            let scrollable_offset = match axis_direction_to_axis(
                self.scroll_direction(app).unwrap_or(AxisDirection::Left),
            ) {
                Axis::Horizontal => Offset::new(scroll_position - drag_start_scroll, 0.0),
                Axis::Vertical => Offset::new(0.0, scroll_position - drag_start_scroll),
            };
            let drag_start_global_position = details.global_position - details.offset_from_origin;
            let consecutive =
                TextSelectionGestureDetectorState::get_effective_consecutive_tap_count(
                    app,
                    details.consecutive_tap_count,
                );
            if consecutive == 2 {
                self.render_editable(app).select_words_in_range(
                    app,
                    drag_start_global_position - editable_offset - scrollable_offset,
                    Some(details.global_position),
                    SelectionChangedCause::Drag,
                );
                match details.kind {
                    Some(
                        PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus
                        | PointerDeviceKind::Touch
                        | PointerDeviceKind::Unknown,
                    ) => {
                        self.show_magnifier_if_supported_by_platform(app, details.global_position);
                    }
                    Some(PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad) | None => {}
                }
                return;
            }
            if consecutive == 3 {
                match app.platform().target_platform() {
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                        match details.kind {
                            Some(PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad) => {
                                self.select_paragraphs_in_range(
                                    app,
                                    drag_start_global_position
                                        - editable_offset
                                        - scrollable_offset,
                                    Some(details.global_position),
                                    Some(SelectionChangedCause::Drag),
                                );
                            }
                            Some(
                                PointerDeviceKind::Stylus
                                | PointerDeviceKind::InvertedStylus
                                | PointerDeviceKind::Touch
                                | PointerDeviceKind::Unknown,
                            )
                            | None => {}
                        }
                        return;
                    }
                    TargetPlatform::Linux => {
                        self.select_lines_in_range(
                            app,
                            drag_start_global_position - editable_offset - scrollable_offset,
                            Some(details.global_position),
                            Some(SelectionChangedCause::Drag),
                        );
                        return;
                    }
                    TargetPlatform::Windows | TargetPlatform::MacOS => {
                        self.select_paragraphs_in_range(
                            app,
                            drag_start_global_position - editable_offset - scrollable_offset,
                            Some(details.global_position),
                            Some(SelectionChangedCause::Drag),
                        );
                        return;
                    }
                }
            }
            match app.platform().target_platform() {
                TargetPlatform::IOS => match details.kind {
                    Some(PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad) => {
                        self.render_editable(app).select_position_at(
                            app,
                            drag_start_global_position - editable_offset - scrollable_offset,
                            Some(details.global_position),
                            SelectionChangedCause::Drag,
                        );
                    }
                    Some(
                        PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus
                        | PointerDeviceKind::Touch
                        | PointerDeviceKind::Unknown,
                    )
                    | None => {}
                },
                TargetPlatform::Android | TargetPlatform::Fuchsia => match details.kind {
                    Some(
                        PointerDeviceKind::Mouse
                        | PointerDeviceKind::Trackpad
                        | PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus,
                    ) => {
                        self.render_editable(app).select_position_at(
                            app,
                            drag_start_global_position - editable_offset - scrollable_offset,
                            Some(details.global_position),
                            SelectionChangedCause::Drag,
                        );
                    }
                    Some(PointerDeviceKind::Touch | PointerDeviceKind::Unknown)
                        if self.render_editable(app).has_focus(app) =>
                    {
                        self.render_editable(app).select_position_at(
                            app,
                            details.global_position,
                            None,
                            SelectionChangedCause::Drag,
                        );
                        self.show_magnifier_if_supported_by_platform(
                            app,
                            details.global_position,
                        );
                    }
                    _ => {}
                },
                TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => {
                    self.render_editable(app).select_position_at(
                        app,
                        drag_start_global_position - editable_offset - scrollable_offset,
                        Some(details.global_position),
                        SelectionChangedCause::Drag,
                    );
                }
            }
            return;
        }

        let drag_start = self.builder_data(app).drag_start_selection;
        let platform = app.platform().target_platform();
        if drag_start.is_some_and(|s| s.is_collapsed())
            || (platform != TargetPlatform::IOS && platform != TargetPlatform::MacOS)
        {
            self.extend_selection(app, details.global_position, SelectionChangedCause::Drag);
            return;
        }
        let drag_start = drag_start.expect("shift-drag keeps the start selection");
        let selection = self.editable_text(app).text_editing_value(app).selection;
        let next_extent = self
            .render_editable(app)
            .get_position_for_point(app, details.global_position);
        let is_shift_tap_drag_selection_forward = drag_start.base_offset < drag_start.extent_offset;
        let is_inverted = if is_shift_tap_drag_selection_forward {
            next_extent.offset < drag_start.base_offset
        } else {
            next_extent.offset > drag_start.base_offset
        };
        if is_inverted && selection.base_offset == drag_start.base_offset {
            let editable = self.editable_text(app);
            let value = editable
                .text_editing_value(app)
                .copy_with()
                .selection(TextSelection::new(
                    drag_start.extent_offset,
                    next_extent.offset,
                ));
            editable.user_update_text_editing_value(app, value, SelectionChangedCause::Drag);
        } else if !is_inverted
            && next_extent.offset != drag_start.base_offset
            && selection.base_offset != drag_start.base_offset
        {
            let editable = self.editable_text(app);
            let value = editable
                .text_editing_value(app)
                .copy_with()
                .selection(TextSelection::new(
                    drag_start.base_offset,
                    next_extent.offset,
                ));
            editable.user_update_text_editing_value(app, value, SelectionChangedCause::Drag);
        } else {
            self.extend_selection(app, details.global_position, SelectionChangedCause::Drag);
        }
    }

    /// Handler for [`TextSelectionGestureDetector::on_drag_selection_end`].
    fn on_drag_selection_end(self: Handle<Self>, app: &mut App, details: TapDragEndDetails) {
        if self.should_show_selection_toolbar(app)
            && TextSelectionGestureDetectorState::get_effective_consecutive_tap_count(
                app,
                details.consecutive_tap_count,
            ) == 2
        {
            let _ = self.editable_text(app).show_toolbar(app);
        }
        if self.builder_data(app).is_shift_pressed {
            self.builder_data_mut(app).drag_start_selection = None;
        }
        self.hide_magnifier_if_supported_by_platform(app);
    }

    /// Returns a [`TextSelectionGestureDetector`] configured with the handlers
    /// provided by this builder.
    fn build_gesture_detector(
        self: Handle<Self>,
        _app: &mut App,
        key: Option<KeyRef>,
        behavior: Option<HitTestBehavior>,
        child: WidgetRef,
    ) -> TextSelectionGestureDetector {
        let this = self;
        let force_press = self.builder_data(_app).delegate.force_press_enabled(_app);
        let mut detector = TextSelectionGestureDetector::new(child)
            .on_tap_track_start(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_tap_track_start(this, app)
            }))
            .on_tap_track_reset(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_tap_track_reset(this, app)
            }))
            .on_tap_down(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_tap_down(this, app, details)
            }))
            .on_secondary_tap(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_secondary_tap(this, app)
            }))
            .on_secondary_tap_down(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_secondary_tap_down(this, app, details)
            }))
            .on_single_tap_up(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_single_tap_up(this, app, details)
            }))
            .on_single_tap_cancel(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_single_tap_cancel(this, app)
            }))
            .on_user_tap(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_user_tap(this, app)
            }))
            .on_single_long_tap_start(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_single_long_tap_start(this, app, details)
            }))
            .on_single_long_tap_move_update(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_single_long_tap_move_update(
                    this, app, details,
                )
            }))
            .on_single_long_tap_end(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_single_long_tap_end(this, app, details)
            }))
            .on_single_long_tap_cancel(Listener::new(move |app| {
                TextSelectionGestureDetectorBuilder::on_single_long_tap_cancel(this, app)
            }))
            .on_double_tap_down(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_double_tap_down(this, app, details)
            }))
            .on_triple_tap_down(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_triple_tap_down(this, app, details)
            }))
            .on_drag_selection_start(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_drag_selection_start(this, app, details)
            }))
            .on_drag_selection_update(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_drag_selection_update(this, app, details)
            }))
            .on_drag_selection_end(Rc::new(move |app, details| {
                TextSelectionGestureDetectorBuilder::on_drag_selection_end(this, app, details)
            }))
            .on_user_tap_always_called(
                TextSelectionGestureDetectorBuilder::on_user_tap_always_called(self, _app),
            );
        if force_press {
            detector = detector
                .on_force_press_start(Rc::new(move |app, details| {
                    TextSelectionGestureDetectorBuilder::on_force_press_start(this, app, details)
                }))
                .on_force_press_end(Rc::new(move |app, details| {
                    TextSelectionGestureDetectorBuilder::on_force_press_end(this, app, details)
                }));
        }
        if let Some(key) = key {
            detector = detector.key(key);
        }
        if let Some(behavior) = behavior {
            detector = detector.behavior(behavior);
        }
        detector
    }
}

impl<T: TextSelectionGestureDetectorBuilder> TextSelectionGestureDetectorBuilderBase for T {}

#[derive(Clone, Copy)]
enum BoundaryKind<'a> {
    Paragraph(&'a str),
    Line,
}

impl BoundaryKind<'_> {
    fn leading(
        self,
        app: &mut App,
        render_editable: RenderHandle<RenderEditable>,
        position: i32,
    ) -> Option<i32> {
        match self {
            BoundaryKind::Paragraph(text) => paragraph_leading(text, position),
            BoundaryKind::Line => {
                let range =
                    render_editable.get_line_at_offset(app, TextPosition::new(position.max(0)));
                Some(range.start())
            }
        }
    }

    fn trailing(
        self,
        app: &mut App,
        render_editable: RenderHandle<RenderEditable>,
        position: i32,
    ) -> Option<i32> {
        match self {
            BoundaryKind::Paragraph(text) => paragraph_trailing(text, position),
            BoundaryKind::Line => {
                let range =
                    render_editable.get_line_at_offset(app, TextPosition::new(position.max(0)));
                Some(range.end())
            }
        }
    }
}

fn move_to_text_boundary<T: TextSelectionGestureDetectorBuilder>(
    this: Handle<T>,
    app: &mut App,
    extent: TextPosition,
    boundary: BoundaryKind<'_>,
) -> TextRange {
    debug_assert!(extent.offset >= 0);
    let text_len = utf16_len_for_boundary(&this.editable_text(app).text_editing_value(app).text);
    let render_editable = this.render_editable(app);
    let start_offset = if extent.offset == text_len {
        extent.offset - 1
    } else {
        extent.offset
    };
    let start = boundary
        .leading(app, render_editable, start_offset)
        .unwrap_or(0);
    let end = boundary
        .trailing(app, render_editable, extent.offset)
        .unwrap_or(text_len);
    TextRange::new(start, end)
}

fn select_text_boundaries_in_range<T: TextSelectionGestureDetectorBuilder>(
    this: Handle<T>,
    app: &mut App,
    boundary: BoundaryKind<'_>,
    from: Offset,
    to: Option<Offset>,
    cause: Option<SelectionChangedCause>,
) {
    let from_position = this.render_editable(app).get_position_for_point(app, from);
    let from_range = move_to_text_boundary(this, app, from_position, boundary);
    let to_position = match to {
        None => from_position,
        Some(to) => this.render_editable(app).get_position_for_point(app, to),
    };
    let to_range = if to_position == from_position {
        from_range
    } else {
        move_to_text_boundary(this, app, to_position, boundary)
    };
    let is_from_boundary_before_to_boundary = from_range.start < to_range.end;
    let new_selection = if is_from_boundary_before_to_boundary {
        TextSelection::new(from_range.start, to_range.end)
    } else {
        TextSelection::new(from_range.end, to_range.start)
    };
    let editable = this.editable_text(app);
    let value = editable
        .text_editing_value(app)
        .copy_with()
        .selection(new_selection);
    if let Some(cause) = cause {
        editable.user_update_text_editing_value(app, value, cause);
    } else {
        editable.user_update_text_editing_value(app, value, SelectionChangedCause::Tap);
    }
}

fn utf16_len_for_boundary(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn paragraph_leading(text: &str, position: i32) -> Option<i32> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if position < 0 || units.is_empty() {
        return None;
    }
    if position >= units.len() as i32 {
        return Some(units.len() as i32);
    }
    if position == 0 {
        return Some(0);
    }
    let mut index = position;
    if index > 1 && units[index as usize] == 0x0A && units[index as usize - 1] == 0x0D {
        index -= 2;
    } else if TextLayoutMetrics::is_line_terminator(units[index as usize] as i32) {
        index -= 1;
    }
    while index > 0 {
        if TextLayoutMetrics::is_line_terminator(units[index as usize] as i32) {
            return Some(index + 1);
        }
        index -= 1;
    }
    Some(index.max(0))
}

fn paragraph_trailing(text: &str, position: i32) -> Option<i32> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if position >= units.len() as i32 || units.is_empty() {
        return None;
    }
    if position < 0 {
        return Some(0);
    }
    let mut index = position;
    while !TextLayoutMetrics::is_line_terminator(units[index as usize] as i32) {
        index += 1;
        if index == units.len() as i32 {
            return Some(index);
        }
    }
    Some(
        if index < units.len() as i32 - 1
            && units[index as usize] == 0x0D
            && units[index as usize + 1] == 0x0A
        {
            index + 2
        } else {
            index + 1
        },
    )
}

/// A gesture detector to respond to non-exclusive event chains for a text field.
#[derive(Clone)]
pub struct TextSelectionGestureDetector {
    pub key: Option<KeyRef>,
    pub on_tap_track_start: Option<Listener>,
    pub on_tap_track_reset: Option<Listener>,
    pub on_tap_down: Option<GestureTapDragDownCallback>,
    pub on_force_press_start: Option<GestureForcePressStartCallback>,
    pub on_force_press_end: Option<GestureForcePressEndCallback>,
    pub on_secondary_tap: Option<GestureTapCallback>,
    pub on_secondary_tap_down: Option<GestureTapDownCallback>,
    pub on_single_tap_up: Option<GestureTapDragUpCallback>,
    pub on_single_tap_cancel: Option<GestureCancelCallback>,
    pub on_user_tap: Option<GestureTapCallback>,
    pub on_single_long_tap_start: Option<GestureLongPressStartCallback>,
    pub on_single_long_tap_move_update: Option<GestureLongPressMoveUpdateCallback>,
    pub on_single_long_tap_end: Option<GestureLongPressEndCallback>,
    pub on_single_long_tap_cancel: Option<GestureLongPressCancelCallback>,
    pub on_double_tap_down: Option<GestureTapDragDownCallback>,
    pub on_triple_tap_down: Option<GestureTapDragDownCallback>,
    pub on_drag_selection_start: Option<GestureTapDragStartCallback>,
    pub on_drag_selection_update: Option<GestureTapDragUpdateCallback>,
    pub on_drag_selection_end: Option<GestureTapDragEndCallback>,
    pub on_user_tap_always_called: bool,
    pub behavior: Option<HitTestBehavior>,
    pub child: WidgetRef,
}

impl Debug for TextSelectionGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectionGestureDetector")
            .field("onUserTapAlwaysCalled", &self.on_user_tap_always_called)
            .field("behavior", &self.behavior)
            .finish_non_exhaustive()
    }
}

impl TextSelectionGestureDetector {
    /// Create a [`TextSelectionGestureDetector`].
    pub fn new<K>(child: impl IntoWidget<K>) -> TextSelectionGestureDetector {
        TextSelectionGestureDetector {
            key: None,
            on_tap_track_start: None,
            on_tap_track_reset: None,
            on_tap_down: None,
            on_force_press_start: None,
            on_force_press_end: None,
            on_secondary_tap: None,
            on_secondary_tap_down: None,
            on_single_tap_up: None,
            on_single_tap_cancel: None,
            on_user_tap: None,
            on_single_long_tap_start: None,
            on_single_long_tap_move_update: None,
            on_single_long_tap_end: None,
            on_single_long_tap_cancel: None,
            on_double_tap_down: None,
            on_triple_tap_down: None,
            on_drag_selection_start: None,
            on_drag_selection_update: None,
            on_drag_selection_end: None,
            on_user_tap_always_called: false,
            behavior: None,
            child: child.into_widget(),
        }
    }

    /// Dart `TextSelectionGestureDetector(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextSelectionGestureDetector {
        self.key = Some(key);
        self
    }

    /// Dart `TextSelectionGestureDetector(onTapTrackStart:)`.
    pub fn on_tap_track_start(mut self, callback: Listener) -> TextSelectionGestureDetector {
        self.on_tap_track_start = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onTapTrackReset:)`.
    pub fn on_tap_track_reset(mut self, callback: Listener) -> TextSelectionGestureDetector {
        self.on_tap_track_reset = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onTapDown:)`.
    pub fn on_tap_down(
        mut self,
        callback: GestureTapDragDownCallback,
    ) -> TextSelectionGestureDetector {
        self.on_tap_down = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onForcePressStart:)`.
    pub fn on_force_press_start(
        mut self,
        callback: GestureForcePressStartCallback,
    ) -> TextSelectionGestureDetector {
        self.on_force_press_start = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onForcePressEnd:)`.
    pub fn on_force_press_end(
        mut self,
        callback: GestureForcePressEndCallback,
    ) -> TextSelectionGestureDetector {
        self.on_force_press_end = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSecondaryTap:)`.
    pub fn on_secondary_tap(
        mut self,
        callback: GestureTapCallback,
    ) -> TextSelectionGestureDetector {
        self.on_secondary_tap = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSecondaryTapDown:)`.
    pub fn on_secondary_tap_down(
        mut self,
        callback: GestureTapDownCallback,
    ) -> TextSelectionGestureDetector {
        self.on_secondary_tap_down = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleTapUp:)`.
    pub fn on_single_tap_up(
        mut self,
        callback: GestureTapDragUpCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_tap_up = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleTapCancel:)`.
    pub fn on_single_tap_cancel(
        mut self,
        callback: GestureCancelCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_tap_cancel = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onUserTap:)`.
    pub fn on_user_tap(mut self, callback: GestureTapCallback) -> TextSelectionGestureDetector {
        self.on_user_tap = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleLongTapStart:)`.
    pub fn on_single_long_tap_start(
        mut self,
        callback: GestureLongPressStartCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_long_tap_start = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleLongTapMoveUpdate:)`.
    pub fn on_single_long_tap_move_update(
        mut self,
        callback: GestureLongPressMoveUpdateCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_long_tap_move_update = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleLongTapEnd:)`.
    pub fn on_single_long_tap_end(
        mut self,
        callback: GestureLongPressEndCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_long_tap_end = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onSingleLongTapCancel:)`.
    pub fn on_single_long_tap_cancel(
        mut self,
        callback: GestureLongPressCancelCallback,
    ) -> TextSelectionGestureDetector {
        self.on_single_long_tap_cancel = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onDoubleTapDown:)`.
    pub fn on_double_tap_down(
        mut self,
        callback: GestureTapDragDownCallback,
    ) -> TextSelectionGestureDetector {
        self.on_double_tap_down = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onTripleTapDown:)`.
    pub fn on_triple_tap_down(
        mut self,
        callback: GestureTapDragDownCallback,
    ) -> TextSelectionGestureDetector {
        self.on_triple_tap_down = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onDragSelectionStart:)`.
    pub fn on_drag_selection_start(
        mut self,
        callback: GestureTapDragStartCallback,
    ) -> TextSelectionGestureDetector {
        self.on_drag_selection_start = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onDragSelectionUpdate:)`.
    pub fn on_drag_selection_update(
        mut self,
        callback: GestureTapDragUpdateCallback,
    ) -> TextSelectionGestureDetector {
        self.on_drag_selection_update = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onDragSelectionEnd:)`.
    pub fn on_drag_selection_end(
        mut self,
        callback: GestureTapDragEndCallback,
    ) -> TextSelectionGestureDetector {
        self.on_drag_selection_end = Some(callback);
        self
    }

    /// Dart `TextSelectionGestureDetector(onUserTapAlwaysCalled:)`.
    pub fn on_user_tap_always_called(mut self, value: bool) -> TextSelectionGestureDetector {
        self.on_user_tap_always_called = value;
        self
    }

    /// Dart `TextSelectionGestureDetector(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> TextSelectionGestureDetector {
        self.behavior = Some(behavior);
        self
    }
}

impl StatefulWidget for TextSelectionGestureDetector {
    type State = TextSelectionGestureDetectorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TextSelectionGestureDetectorState {
        TextSelectionGestureDetectorState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_TextSelectionGestureDetectorState`.
pub struct TextSelectionGestureDetectorState {
    state: StateData<TextSelectionGestureDetector>,
}

impl Debug for TextSelectionGestureDetectorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectionGestureDetectorState")
            .finish_non_exhaustive()
    }
}

fn force_press_start_cb(
    this: Handle<TextSelectionGestureDetectorState>,
) -> GestureForcePressStartCallback {
    Rc::new(move |app, details| this.force_press_started(app, details))
}

fn force_press_end_cb(
    this: Handle<TextSelectionGestureDetectorState>,
) -> GestureForcePressEndCallback {
    Rc::new(move |app, details| this.force_press_ended(app, details))
}

impl TextSelectionGestureDetectorState {
    /// Converts the details.consecutiveTapCount from a TapAndDrag*Details object,
    /// which can grow to be infinitely large, to a value between 1 and 3.
    pub fn get_effective_consecutive_tap_count(app: &App, raw_count: i32) -> i32 {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Linux => {
                if raw_count <= 3 {
                    raw_count
                } else if raw_count % 3 == 0 {
                    3
                } else {
                    raw_count % 3
                }
            }
            TargetPlatform::IOS | TargetPlatform::MacOS => raw_count.min(3),
            TargetPlatform::Windows => {
                if raw_count < 2 {
                    raw_count
                } else {
                    2 + raw_count % 2
                }
            }
        }
    }

    fn handle_tap_track_start(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = self.widget(app).on_tap_track_start.clone() {
            callback.call(app);
        }
    }

    fn handle_tap_track_reset(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = self.widget(app).on_tap_track_reset.clone() {
            callback.call(app);
        }
    }

    fn handle_tap_down(self: Handle<Self>, app: &mut App, details: TapDragDownDetails) {
        if let Some(callback) = self.widget(app).on_tap_down.clone() {
            callback(app, details.clone());
        }
        if Self::get_effective_consecutive_tap_count(app, details.consecutive_tap_count) == 2 {
            if let Some(callback) = self.widget(app).on_double_tap_down.clone() {
                callback(app, details);
            }
            return;
        }
        if Self::get_effective_consecutive_tap_count(app, details.consecutive_tap_count) == 3
            && let Some(callback) = self.widget(app).on_triple_tap_down.clone()
        {
            callback(app, details);
        }
    }

    fn handle_tap_up(self: Handle<Self>, app: &mut App, details: TapDragUpDetails) {
        if Self::get_effective_consecutive_tap_count(app, details.consecutive_tap_count) == 1 {
            if let Some(callback) = self.widget(app).on_single_tap_up.clone() {
                callback(app, details);
            }
            if let Some(callback) = self.widget(app).on_user_tap.clone() {
                callback.call(app);
            }
        } else if self.widget(app).on_user_tap_always_called
            && let Some(callback) = self.widget(app).on_user_tap.clone()
        {
            callback.call(app);
        }
    }

    fn handle_tap_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = self.widget(app).on_single_tap_cancel.clone() {
            callback.call(app);
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, details: TapDragStartDetails) {
        if let Some(callback) = self.widget(app).on_drag_selection_start.clone() {
            callback(app, details);
        }
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: TapDragUpdateDetails) {
        if let Some(callback) = self.widget(app).on_drag_selection_update.clone() {
            callback(app, details);
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: TapDragEndDetails) {
        if let Some(callback) = self.widget(app).on_drag_selection_end.clone() {
            callback(app, details);
        }
    }

    fn force_press_started(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        if let Some(callback) = self.widget(app).on_force_press_start.clone() {
            callback(app, details);
        }
    }

    fn force_press_ended(self: Handle<Self>, app: &mut App, details: ForcePressDetails) {
        if let Some(callback) = self.widget(app).on_force_press_end.clone() {
            callback(app, details);
        }
    }

    fn handle_long_press_start(self: Handle<Self>, app: &mut App, details: LongPressStartDetails) {
        if let Some(callback) = self.widget(app).on_single_long_tap_start.clone() {
            callback(app, details);
        }
    }

    fn handle_long_press_move_update(
        self: Handle<Self>,
        app: &mut App,
        details: LongPressMoveUpdateDetails,
    ) {
        if let Some(callback) = self.widget(app).on_single_long_tap_move_update.clone() {
            callback(app, details);
        }
    }

    fn handle_long_press_end(self: Handle<Self>, app: &mut App, details: LongPressEndDetails) {
        if let Some(callback) = self.widget(app).on_single_long_tap_end.clone() {
            callback(app, details);
        }
    }

    fn handle_long_press_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = self.widget(app).on_single_long_tap_cancel.clone() {
            callback.call(app);
        }
    }
}

impl State for TextSelectionGestureDetectorState {
    type Widget = TextSelectionGestureDetector;

    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut gestures: GestureRecognizerFactories = Vec::new();
        let this = self;
        gestures.push((
            TypeId::of::<TapGestureRecognizer>(),
            GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(
                TapGestureRecognizer::new,
                move |app, instance| {
                    instance.set_on_secondary_tap(app, this.widget(app).on_secondary_tap.clone());
                    instance.set_on_secondary_tap_down(
                        app,
                        this.widget(app).on_secondary_tap_down.clone(),
                    );
                },
            )
            .into_factory(),
        ));

        let widget = self.widget(app);
        if widget.on_single_long_tap_start.is_some()
            || widget.on_single_long_tap_move_update.is_some()
            || widget.on_single_long_tap_end.is_some()
            || widget.on_single_long_tap_cancel.is_some()
        {
            gestures.push((
                TypeId::of::<LongPressGestureRecognizer>(),
                GestureRecognizerFactoryWithHandlers::<LongPressGestureRecognizer>::new(
                    |app| {
                        LongPressGestureRecognizer::new(app)
                            .supported_devices(app, [PointerDeviceKind::Touch])
                    },
                    move |app, instance| {
                        instance.set_on_long_press_start(
                            app,
                            Some(Rc::new(move |app, details| {
                                this.handle_long_press_start(app, details);
                            })),
                        );
                        instance.set_on_long_press_move_update(
                            app,
                            Some(Rc::new(move |app, details| {
                                this.handle_long_press_move_update(app, details);
                            })),
                        );
                        instance.set_on_long_press_end(
                            app,
                            Some(Rc::new(move |app, details| {
                                this.handle_long_press_end(app, details);
                            })),
                        );
                        instance.set_on_long_press_cancel(
                            app,
                            Some(Listener::new(move |app| {
                                this.handle_long_press_cancel(app);
                            })),
                        );
                    },
                )
                .into_factory(),
            ));
        }

        if widget.on_drag_selection_start.is_some()
            || widget.on_drag_selection_update.is_some()
            || widget.on_drag_selection_end.is_some()
        {
            match app.platform().target_platform() {
                TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                    let eager = app.platform().target_platform() != TargetPlatform::IOS;
                    gestures.push(
                        (
                            TypeId::of::<TapAndHorizontalDragGestureRecognizer>(),
                            GestureRecognizerFactoryWithHandlers::<
                                TapAndHorizontalDragGestureRecognizer,
                            >::new(
                                TapAndHorizontalDragGestureRecognizer::new,
                                move |app, instance| {
                                    instance.set_drag_start_behavior(app, DragStartBehavior::Down);
                                    instance.set_eager_victory_on_drag(app, eager);
                                    instance.set_on_tap_track_start(
                                        app,
                                        Some(Listener::new(move |app| {
                                            this.handle_tap_track_start(app);
                                        })),
                                    );
                                    instance.set_on_tap_track_reset(
                                        app,
                                        Some(Listener::new(move |app| {
                                            this.handle_tap_track_reset(app);
                                        })),
                                    );
                                    instance.set_on_tap_down(
                                        app,
                                        Some(Rc::new(move |app, details| {
                                            this.handle_tap_down(app, details);
                                        })),
                                    );
                                    instance.set_on_drag_start(
                                        app,
                                        Some(Rc::new(move |app, details| {
                                            this.handle_drag_start(app, details);
                                        })),
                                    );
                                    instance.set_on_drag_update(
                                        app,
                                        Some(Rc::new(move |app, details| {
                                            this.handle_drag_update(app, details);
                                        })),
                                    );
                                    instance.set_on_drag_end(
                                        app,
                                        Some(Rc::new(move |app, details| {
                                            this.handle_drag_end(app, details);
                                        })),
                                    );
                                    instance.set_on_tap_up(
                                        app,
                                        Some(Rc::new(move |app, details| {
                                            this.handle_tap_up(app, details);
                                        })),
                                    );
                                    instance.set_on_cancel(
                                        app,
                                        Some(Listener::new(move |app| {
                                            this.handle_tap_cancel(app);
                                        })),
                                    );
                                },
                            )
                            .into_factory(),
                        ),
                    );
                }
                TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                    gestures.push((
                        TypeId::of::<TapAndPanGestureRecognizer>(),
                        GestureRecognizerFactoryWithHandlers::<TapAndPanGestureRecognizer>::new(
                            TapAndPanGestureRecognizer::new,
                            move |app, instance| {
                                instance.set_drag_start_behavior(app, DragStartBehavior::Down);
                                instance.set_on_tap_track_start(
                                    app,
                                    Some(Listener::new(move |app| {
                                        this.handle_tap_track_start(app);
                                    })),
                                );
                                instance.set_on_tap_track_reset(
                                    app,
                                    Some(Listener::new(move |app| {
                                        this.handle_tap_track_reset(app);
                                    })),
                                );
                                instance.set_on_tap_down(
                                    app,
                                    Some(Rc::new(move |app, details| {
                                        this.handle_tap_down(app, details);
                                    })),
                                );
                                instance.set_on_drag_start(
                                    app,
                                    Some(Rc::new(move |app, details| {
                                        this.handle_drag_start(app, details);
                                    })),
                                );
                                instance.set_on_drag_update(
                                    app,
                                    Some(Rc::new(move |app, details| {
                                        this.handle_drag_update(app, details);
                                    })),
                                );
                                instance.set_on_drag_end(
                                    app,
                                    Some(Rc::new(move |app, details| {
                                        this.handle_drag_end(app, details);
                                    })),
                                );
                                instance.set_on_tap_up(
                                    app,
                                    Some(Rc::new(move |app, details| {
                                        this.handle_tap_up(app, details);
                                    })),
                                );
                                instance.set_on_cancel(
                                    app,
                                    Some(Listener::new(move |app| {
                                        this.handle_tap_cancel(app);
                                    })),
                                );
                            },
                        )
                        .into_factory(),
                    ));
                }
            }
        }

        if widget.on_force_press_start.is_some() || widget.on_force_press_end.is_some() {
            let has_start = widget.on_force_press_start.is_some();
            let has_end = widget.on_force_press_end.is_some();
            gestures.push((
                TypeId::of::<ForcePressGestureRecognizer>(),
                GestureRecognizerFactoryWithHandlers::<ForcePressGestureRecognizer>::new(
                    ForcePressGestureRecognizer::new,
                    move |app, instance| {
                        instance.set_on_start(app, has_start.then_some(force_press_start_cb(this)));
                        instance.set_on_end(app, has_end.then_some(force_press_end_cb(this)));
                    },
                )
                .into_factory(),
            ));
        }

        let mut detector = RawGestureDetector::new().gestures(gestures);
        if let Some(behavior) = self.widget(app).behavior {
            detector = detector.behavior(behavior);
        }
        detector.child(self.widget(app).child.clone()).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::PointerDeviceKind;
    use reveal_foundation::AppCell;
    use reveal_gestures::{
        GestureBinding, PointerDownEvent, PointerEvent, PointerMoveEvent, PointerUpEvent,
    };

    use crate::test_harness::{binding_cell, binding_mount};
    use crate::widgets::basic::{ColoredBox, SizedBox};
    use reveal_embedder::Color;

    fn press(app: &mut App, pointer: i64, position: Offset, kind: PointerDeviceKind) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Down(PointerDownEvent {
                pointer,
                position,
                kind,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn move_to(app: &mut App, pointer: i64, from: Offset, to: Offset, kind: PointerDeviceKind) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Move(PointerMoveEvent {
                pointer,
                position: to,
                delta: to - from,
                kind,
                down: true,
                ..PointerMoveEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn release(app: &mut App, pointer: i64, position: Offset, kind: PointerDeviceKind) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Up(PointerUpEvent {
                pointer,
                position,
                kind,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    #[test]
    fn text_selection_overlay_is_a_type() {
        let _ = std::any::type_name::<TextSelectionOverlay>();
        let _ = std::any::type_name::<SelectionOverlay>();
    }

    #[test]
    fn clipboard_status_notifier_update_default_platform_is_not_pasteable() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = ClipboardStatusNotifier::new(&mut app);
        assert_eq!(notifier.value(&app), ClipboardStatus::Unknown);
        notifier.update(&mut app);
        assert_eq!(notifier.value(&app), ClipboardStatus::NotPasteable);
    }

    fn log_tap_down(log: Rc<RefCell<Vec<&'static str>>>) -> GestureTapDragDownCallback {
        Rc::new(move |_app, _details| {
            log.borrow_mut().push("onTapDown");
        })
    }

    fn log_tap_up(log: Rc<RefCell<Vec<&'static str>>>) -> GestureTapDragUpCallback {
        Rc::new(move |_app, _details| {
            log.borrow_mut().push("onSingleTapUp");
        })
    }

    fn log_drag_start(log: Rc<RefCell<Vec<&'static str>>>) -> GestureTapDragStartCallback {
        Rc::new(move |_app, _details| {
            log.borrow_mut().push("onDragSelectionStart");
        })
    }

    fn log_drag_update(log: Rc<RefCell<Vec<&'static str>>>) -> GestureTapDragUpdateCallback {
        Rc::new(move |_app, _details| {
            log.borrow_mut().push("onDragSelectionUpdate");
        })
    }

    fn log_drag_end(log: Rc<RefCell<Vec<&'static str>>>) -> GestureTapDragEndCallback {
        Rc::new(move |_app, _details| {
            log.borrow_mut().push("onDragSelectionEnd");
        })
    }

    #[test]
    fn a_mouse_click_fires_tap_down_and_single_tap_up() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let cell = binding_cell();
        let down = log_tap_down(Rc::clone(&log));
        let up = log_tap_up(Rc::clone(&log));
        binding_mount(
            &cell,
            TextSelectionGestureDetector::new(
                SizedBox::new()
                    .width(300.0)
                    .height(200.0)
                    .child(ColoredBox::new(Color::new(0xFFFFFFFF))),
            )
            .on_tap_down(down)
            .on_single_tap_up(up)
            .on_drag_selection_start(log_drag_start(Rc::new(RefCell::new(Vec::new()))))
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let at = Offset::new(20.0, 20.0);
        press(&mut app, 1, at, PointerDeviceKind::Mouse);
        release(&mut app, 1, at, PointerDeviceKind::Mouse);
        assert_eq!(*log.borrow(), ["onTapDown", "onSingleTapUp"]);
    }

    #[test]
    fn a_mouse_drag_fires_start_update_and_end() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let cell = binding_cell();
        let start = log_drag_start(Rc::clone(&log));
        let update = log_drag_update(Rc::clone(&log));
        let end = log_drag_end(Rc::clone(&log));
        binding_mount(
            &cell,
            TextSelectionGestureDetector::new(
                SizedBox::new()
                    .width(300.0)
                    .height(200.0)
                    .child(ColoredBox::new(Color::new(0xFFFFFFFF))),
            )
            .on_drag_selection_start(start)
            .on_drag_selection_update(update)
            .on_drag_selection_end(end)
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let start_at = Offset::new(20.0, 20.0);
        let end_at = Offset::new(80.0, 20.0);
        press(&mut app, 1, start_at, PointerDeviceKind::Mouse);
        move_to(
            &mut app,
            1,
            start_at,
            Offset::new(40.0, 20.0),
            PointerDeviceKind::Mouse,
        );
        move_to(
            &mut app,
            1,
            Offset::new(40.0, 20.0),
            end_at,
            PointerDeviceKind::Mouse,
        );
        release(&mut app, 1, end_at, PointerDeviceKind::Mouse);
        let events = log.borrow().clone();
        assert!(
            events.first() == Some(&"onDragSelectionStart"),
            "{events:?}"
        );
        assert!(events.contains(&"onDragSelectionUpdate"), "{events:?}");
        assert_eq!(events.last(), Some(&"onDragSelectionEnd"));
    }
}
