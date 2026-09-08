//! Flutter counterpart: `widgets/text_selection.dart`
//! (`ClipboardStatus` / `ClipboardStatusNotifier`, `LiveTextInputStatus` /
//! `LiveTextInputStatusNotifier`, `TextSelectionHandleType`,
//! `TextSelectionControls`, `TextSelectionHandleControls`,
//! `TextSelectionOverlay`, `SelectionOverlay`).
//!
//! Toolbar layout widgets and `TextSelectionGestureDetector` wait; see PORTING.md.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{
    Offset, PointerDeviceKind, Rect, Size, TargetPlatform, TextAffinity, TextDirection,
    TextEditingValue, TextPosition, TextRange, TextSelection,
};
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, Listenable, Listener, ValueChanged,
    ValueListenable, ValueNotifier,
};
use reveal_gestures::{DragEndDetails, DragStartBehavior, DragStartDetails, DragUpdateDetails};
use reveal_painting::transform_rect;
use reveal_rendering::{LayerLink, RenderBox, RenderEditable, RenderHandle, TextSelectionPoint};
use reveal_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};
use reveal_services::{AnyTextSelectionDelegate, Clipboard, HapticFeedback, SelectionChangedCause};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
use crate::framework::{BuildContext, IntoWidget, State, WidgetRef};
use crate::widgets::basic::{SizedBox, WidgetBuilder};
use crate::widgets::magnifier::{MagnifierController, MagnifierInfo, TextMagnifierConfiguration};
use crate::widgets::overlay::{Overlay, OverlayEntry};
use crate::widgets::tap_region::TextFieldTapRegion;

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
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app, context| {
                let child = app
                    .get(this)
                    .context_menu_builder
                    .clone()
                    .map(|builder| builder(app, context))
                    .unwrap_or_else(|| SizedBox::shrink().into_widget());
                TextFieldTapRegion::new(child).into_widget()
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
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app, context| {
                let child = app
                    .get(this)
                    .spell_check_toolbar_builder
                    .clone()
                    .map(|builder| builder(app, context))
                    .unwrap_or_else(|| SizedBox::shrink().into_widget());
                TextFieldTapRegion::new(child).into_widget()
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

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_foundation::AppCell;

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
}
