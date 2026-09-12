//! Flutter counterpart: `widgets/editable_text.dart`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use inset_animation::{AnimationBehavior, AnimationController, Curves};
use inset_embedder::{
    AutofillConfiguration, BoxHeightStyle, BoxWidthStyle, Brightness, Clip, Color, FontWeight,
    Locale, Matrix4, Offset, PointerDeviceKind, Radius, Rect, Size, SmartDashesType,
    SmartQuotesType, TargetPlatform, TextAffinity, TextAlign, TextBaseline, TextCapitalization,
    TextDecoration, TextDirection, TextHeightBehavior, TextInputAction, TextInputConfiguration,
    TextInputType, TextPosition, TextRange, TextSelection, clamp_double,
};
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, Listenable, Listener, Timer,
    ValueNotifier,
};
use inset_gestures::{DragStartBehavior, PointerDownEvent, PointerUpEvent};
use inset_painting::{
    AxisDirection, EdgeInsets, InlineSpanRef, TextScaler, TextSpan, TextStyle, TextWidthBasis,
};
use inset_physics::{Simulation, Tolerance};
use inset_rendering::{
    AnyLayer, AnyRenderBox, AnyRenderObject, AnyViewportOffset, BoxConstraints, BoxHitTestResult,
    CompositionCallback, HitTestBehavior, LayerLink, LineBoundary, PaintingContext, RenderBox,
    RenderBoxData, RenderEditable, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    RenderProxyBoxWithHitTestBehavior, RevealedOffset, VerticalCaretMovementRun,
};
use inset_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
use inset_services::{
    AnyAutofillClient, AnyTextSelectionDelegate, AutofillClient, AutofillHints, CharacterBoundary,
    Clipboard, ClipboardData, DefaultProcessTextService, DocumentBoundary,
    FilteringTextInputFormatter, KeyboardInsertedContent, LiveText, MouseCursorRef,
    ParagraphBoundary, ProcessTextAction, ProcessTextService, SelectionChangedCause,
    SpellCheckResults, SuggestionSpan, TextBoundary, TextEditingValue, TextInput, TextInputClient,
    TextInputConnection, TextInputFormatter, TextInputFormatterRef, TextInputStyle,
    TextSelectionDelegate,
};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
use crate::framework::{
    BuildContext, GlobalKey, IntoWidget, KeyRef, LeafRenderObjectWidget, RenderObjectWidget,
    SingleChildRenderObjectWidget, State, StateData, StatefulWidget, WidgetRef,
};
use crate::state_accessors;
use crate::widgets::actions::{
    Action, ActionData, Actions, AnyAction, CallbackAction, ContextAction, DismissIntent,
    DoNothingAction, Intent, OnInvokeCallback,
};
use crate::widgets::app_lifecycle_listener::AppLifecycleListener;
use crate::widgets::autofill::{AutofillGroup, AutofillGroupState};
use crate::widgets::automatic_keep_alive::{
    AutomaticKeepAliveClientMixin, AutomaticKeepAliveClientMixinData,
};
use crate::widgets::basic::CompositedTransformTarget;
use crate::widgets::basic::WidgetBuilder;
use crate::widgets::basic::{Builder, Directionality};
use crate::widgets::context_menu_button_item::{ContextMenuButtonItem, ContextMenuButtonType};
use crate::widgets::default_text_editing_shortcuts::intent_for_macos_selector;
use crate::widgets::focus_manager::{FocusManager, FocusNode, FocusNodeLeaf, UnfocusDisposition};
use crate::widgets::focus_scope::{Focus, FocusScope};
use crate::widgets::focus_traversal::{DirectionalFocusAction, DirectionalFocusIntent};
use crate::widgets::magnifier::TextMagnifierConfiguration;
use crate::widgets::media_query::{MediaQuery, Orientation};
use crate::widgets::scroll_configuration::{ScrollBehaviorRef, ScrollConfiguration};
use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};
use crate::widgets::scroll_physics::{ScrollPhysics, ScrollPhysicsBase, ScrollPhysicsRef};
use crate::widgets::scrollable::{Scrollable, ScrollableState, ViewportBuilder};
use crate::widgets::scrollable_helpers::{ScrollAction, ScrollIncrementType, ScrollIntent};
use crate::widgets::spell_check::{
    SpellCheckConfiguration, build_text_span_with_spell_check_suggestions,
};
use crate::widgets::tap_region::{TapRegionGroupId, TextFieldTapRegion};
use crate::widgets::text::DefaultTextHeightBehavior;
use crate::widgets::text_editing_intents::{
    CopySelectionTextIntent, DeleteCharacterIntent, DeleteToLineBreakIntent,
    DeleteToNextWordBoundaryIntent, DirectionalCaretMovementIntent, DirectionalTextEditingIntent,
    DoNothingAndStopPropagationTextIntent, EditableTextTapOutsideIntent,
    EditableTextTapUpOutsideIntent, ExpandSelectionToDocumentBoundaryIntent,
    ExpandSelectionToLineBreakIntent, ExtendSelectionByCharacterIntent,
    ExtendSelectionToDocumentBoundaryIntent, ExtendSelectionToLineBreakIntent,
    ExtendSelectionToNextParagraphBoundaryIntent,
    ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent,
    ExtendSelectionToNextWordBoundaryIntent,
    ExtendSelectionToNextWordBoundaryOrCaretLocationIntent,
    ExtendSelectionVerticallyToAdjacentLineIntent, ExtendSelectionVerticallyToAdjacentPageIntent,
    PasteTextIntent, ReplaceTextIntent, ScrollToDocumentBoundaryIntent, SelectAllTextIntent,
    TransposeCharactersIntent, UpdateSelectionIntent,
};
use crate::widgets::text_selection::{
    ClipboardStatus, ClipboardStatusNotifier, LiveTextInputStatus, LiveTextInputStatusNotifier,
    TextSelectionControls, TextSelectionOverlay,
};
use crate::widgets::text_selection_toolbar_anchors::TextSelectionToolbarAnchors;
use crate::widgets::ticker_provider::{
    TickerMode, TickerProviderStateMixin, TickerProviderStateMixinData,
};
use crate::widgets::undo_history::{UndoHistory, UndoHistoryController};

/// A controller for an editable text field.
///
/// Whenever the user modifies the text field with an attached this controller,
/// the text field updates [`value`](Self::value) and the controller notifies its
/// listeners. Listeners can then read the [`text`](Self::text) and
/// [`selection`](Self::selection) properties to learn what the user has typed or
/// how the selection has been updated.
///
/// Similarly, you can modify the [`text`](Self::text) or [`selection`](Self::selection)
/// properties, and the listener callbacks will fire.
pub struct TextEditingController {
    change_notifier: ChangeNotifierData,
    value: TextEditingValue,
}

impl Debug for TextEditingController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEditingController")
            .field("value", &self.value)
            .finish()
    }
}

impl TextEditingController {
    /// Creates a controller for an editable text field, with no initial selection.
    ///
    /// This constructor treats a missing `text` argument as if it were the empty
    /// string.
    ///
    /// The initial selection is `TextSelection::collapsed(-1, …)`.
    pub fn new(app: &mut App) -> Handle<TextEditingController> {
        app.create(TextEditingController {
            change_notifier: ChangeNotifierData::new(),
            value: TextEditingValue::EMPTY,
        })
    }

    /// Dart `TextEditingController(text:)`.
    pub fn text(self: Handle<Self>, app: &mut App, text: impl Into<String>) -> Handle<Self> {
        app.get_mut(self).value = TextEditingValue::new().text(text);
        self
    }

    /// Creates a controller for an editable text field from an initial [`TextEditingValue`].
    pub fn from_value(
        app: &mut App,
        value: Option<TextEditingValue>,
    ) -> Handle<TextEditingController> {
        let value = value.unwrap_or(TextEditingValue::EMPTY);
        debug_assert!(
            !value.composing.is_valid() || value.is_composing_range_valid(),
            "New TextEditingValue {value:?} has an invalid non-empty composing range \
             {:?}. It is recommended to use a valid composing range, \
             even for readonly text fields.",
            value.composing
        );
        app.create(TextEditingController {
            change_notifier: ChangeNotifierData::new(),
            value,
        })
    }

    /// The current string the user is editing.
    pub fn text_value(self: Handle<Self>, app: &App) -> &str {
        &app.get(self).value.text
    }

    /// Updates the current text to the given `new_text`, and removes existing
    /// selection and composing range held by the controller.
    pub fn set_text(self: Handle<Self>, app: &mut App, new_text: impl Into<String>) {
        self.set_value(
            app,
            self.value(app)
                .clone()
                .copy_with()
                .text(new_text)
                .selection(TextSelection::collapsed(-1, TextAffinity::Downstream))
                .composing(TextRange::EMPTY),
        );
    }

    /// The current [`TextEditingValue`].
    pub fn value(self: Handle<Self>, app: &App) -> &TextEditingValue {
        &app.get(self).value
    }

    /// Setting this does not run `TextInputFormatter`s. Apply them manually if
    /// needed.
    pub fn set_value(self: Handle<Self>, app: &mut App, new_value: TextEditingValue) {
        debug_assert!(
            !new_value.composing.is_valid() || new_value.is_composing_range_valid(),
            "New TextEditingValue {new_value:?} has an invalid non-empty composing range \
             {:?}. It is recommended to use a valid composing range, \
             even for readonly text fields.",
            new_value.composing
        );
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }

    /// Builds [`TextSpan`] from current editing value.
    ///
    /// By default makes text in composing range appear as underlined. Descendants
    /// can override this method to customize appearance of text.
    pub fn build_text_span(
        self: Handle<Self>,
        app: &App,
        _context: BuildContext,
        style: Option<TextStyle>,
        with_composing: bool,
    ) -> TextSpan {
        let value = self.value(app).clone();
        debug_assert!(
            !value.composing.is_valid() || !with_composing || value.is_composing_range_valid()
        );
        let composing_region_out_of_range = !value.is_composing_range_valid() || !with_composing;
        if composing_region_out_of_range {
            return match style {
                Some(style) => TextSpan::new().style(style).text(value.text),
                None => TextSpan::new().text(value.text),
            };
        }
        let composing_style = style
            .as_ref()
            .map(|s| {
                s.merge(Some(
                    &TextStyle::new().decoration(TextDecoration::UNDERLINE),
                ))
            })
            .unwrap_or_else(|| TextStyle::new().decoration(TextDecoration::UNDERLINE));
        let mut span = TextSpan::new();
        if let Some(style) = style {
            span = span.style(style);
        }
        span.children(vec![
            TextSpan::new()
                .text(value.composing.text_before(&value.text))
                .into_span(),
            TextSpan::new()
                .style(composing_style)
                .text(value.composing.text_inside(&value.text))
                .into_span(),
            TextSpan::new()
                .text(value.composing.text_after(&value.text))
                .into_span(),
        ])
    }

    /// The currently selected range within [`text_value`](Self::text_value).
    pub fn selection(self: Handle<Self>, app: &App) -> TextSelection {
        app.get(self).value.selection
    }

    /// Sets the selection. If the new selection is outside the composing range,
    /// the composing range is cleared.
    pub fn set_selection(self: Handle<Self>, app: &mut App, new_selection: TextSelection) {
        let text_len = utf16_len(&app.get(self).value.text);
        if text_len < new_selection.end() || text_len < new_selection.start() {
            panic!("invalid text selection: {new_selection:?}");
        }
        let value = app.get(self).value.clone();
        let new_composing = if is_selection_within_composing_range(&value, new_selection) {
            value.composing
        } else {
            TextRange::EMPTY
        };
        self.set_value(
            app,
            value
                .copy_with()
                .selection(new_selection)
                .composing(new_composing),
        );
    }

    /// Set the [`value`](Self::value) to empty.
    pub fn clear(self: Handle<Self>, app: &mut App) {
        self.set_value(
            app,
            TextEditingValue::new()
                .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
        );
    }

    /// Set the composing region to an empty range.
    pub fn clear_composing(self: Handle<Self>, app: &mut App) {
        let value = self.value(app).clone();
        self.set_value(app, value.copy_with().composing(TextRange::EMPTY));
    }

    /// Discards any resources used by the object.
    pub fn dispose(&mut self) {
        self.change_notifier.dispose();
    }
}

fn is_selection_within_composing_range(value: &TextEditingValue, selection: TextSelection) -> bool {
    selection.start() >= value.composing.start && selection.end() <= value.composing.end
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn option_rc_eq<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl ChangeNotifier for TextEditingController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl crate::widgets::undo_history::UndoHistorySource<TextEditingValue> for TextEditingController {
    fn history_value(self: Handle<Self>, app: &App) -> &TextEditingValue {
        self.value(app)
    }
}

/// Toolbar configuration for `EditableText`.
///
/// Toolbar is a context menu that will show up when user right click or long
/// press the `EditableText`. It includes several options: cut, copy, paste,
/// and select all.
///
/// `EditableText` and its derived widgets have their own default [`ToolbarOptions`].
/// Create a custom [`ToolbarOptions`] if you want explicit control over the toolbar
/// option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToolbarOptions {
    /// Whether to show copy option in toolbar.
    pub copy: bool,
    /// Whether to show cut option in toolbar.
    pub cut: bool,
    /// Whether to show paste option in toolbar.
    pub paste: bool,
    /// Whether to show select all option in toolbar.
    pub select_all: bool,
}

impl ToolbarOptions {
    /// Create a toolbar configuration with given options.
    ///
    /// All options default to false if they are not explicitly set.
    pub const fn new() -> ToolbarOptions {
        ToolbarOptions {
            copy: false,
            cut: false,
            paste: false,
            select_all: false,
        }
    }

    /// An instance of [`ToolbarOptions`] with no options enabled.
    pub const EMPTY: ToolbarOptions = ToolbarOptions::new();

    /// Dart `ToolbarOptions(copy:)`.
    pub const fn copy(mut self, copy: bool) -> ToolbarOptions {
        self.copy = copy;
        self
    }

    /// Dart `ToolbarOptions(cut:)`.
    pub const fn cut(mut self, cut: bool) -> ToolbarOptions {
        self.cut = cut;
        self
    }

    /// Dart `ToolbarOptions(paste:)`.
    pub const fn paste(mut self, paste: bool) -> ToolbarOptions {
        self.paste = paste;
        self
    }

    /// Dart `ToolbarOptions(selectAll:)`.
    pub const fn select_all(mut self, select_all: bool) -> ToolbarOptions {
        self.select_all = select_all;
        self
    }
}

impl Default for ToolbarOptions {
    fn default() -> ToolbarOptions {
        ToolbarOptions::new()
    }
}

/// The default mime types to be used when allowedMimeTypes is not provided.
///
/// The default value supports inserting images of any supported format.
pub const K_DEFAULT_CONTENT_INSERTION_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/bmp",
    "image/jpg",
    "image/tiff",
    "image/gif",
    "image/jpeg",
    "image/webp",
];

struct CompositionCallbackWidget {
    composite_callback: CompositionCallback,
    enabled: bool,
    child: WidgetRef,
}

impl Debug for CompositionCallbackWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CompositionCallback")
            .field("enabled", &self.enabled)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl CompositionCallbackWidget {
    fn new(composite_callback: CompositionCallback, enabled: bool, child: WidgetRef) -> Self {
        CompositionCallbackWidget {
            composite_callback,
            enabled,
            child,
        }
    }
}

impl RenderObjectWidget for CompositionCallbackWidget {
    type RenderObject = RenderCompositionCallback;

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCompositionCallback::new(app, self.composite_callback.clone(), self.enabled)
            .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCompositionCallback>,
    ) {
        // _EditableTextState always uses the same callback.
        debug_assert!(Rc::ptr_eq(
            &render_object.get(app).composite_callback,
            &self.composite_callback
        ));
        render_object.set_enabled(app, self.enabled);
    }
}

impl SingleChildRenderObjectWidget for CompositionCallbackWidget {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// What `Layer::add_composition_callback` returns: Dart's `VoidCallback` that removes the callback.
type CancelCompositionCallback = Box<dyn FnOnce(&mut App)>;

struct RenderCompositionCallback {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    composite_callback: CompositionCallback,
    cancel_callback: Option<CancelCompositionCallback>,
    enabled: bool,
}

impl RenderCompositionCallback {
    fn new(
        app: &mut App,
        composite_callback: CompositionCallback,
        enabled: bool,
    ) -> RenderHandle<Self> {
        RenderHandle::new_box(
            app,
            RenderCompositionCallback {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                composite_callback,
                cancel_callback: None,
                enabled,
            },
        )
    }

    fn set_enabled(self: RenderHandle<Self>, app: &mut App, new_value: bool) {
        self.get_mut(app).enabled = new_value;
        if !new_value {
            if let Some(cancel) = self.get_mut(app).cancel_callback.take() {
                cancel(app);
            }
        } else if self.get(app).cancel_callback.is_none() {
            self.as_object().mark_needs_paint(app);
        }
    }
}

impl RenderObjectWithChildMixin for RenderCompositionCallback {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderProxyBoxMixin for RenderCompositionCallback {}

impl RenderProxyBoxWithHitTestBehavior for RenderCompositionCallback {
    fn behavior(self: RenderHandle<Self>, _app: &App) -> HitTestBehavior {
        HitTestBehavior::DeferToChild
    }
}

impl RenderObject for RenderCompositionCallback {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.get(app).enabled && self.get(app).cancel_callback.is_none() {
            let callback = self.get(app).composite_callback.clone();
            let cancel = context.add_composition_callback(app, callback);
            self.get_mut(app).cancel_callback = Some(cancel);
        }
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }

    /// Not in Dart: the callback holds the state's `Handle`, which is stale once the state is
    /// destroyed, where Dart's tear-off keeps the state alive to find `renderEditable` detached.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        if let Some(cancel) = self.get_mut(app).cancel_callback.take() {
            cancel(app);
        }
        inset_rendering::RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderCompositionCallback {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test(self, app, result, position)
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Configures the ability to insert media content through the soft keyboard.
///
/// The configuration provides a handler for any rich content inserted through
/// the system input method, and also provides the ability to limit the mime
/// types of the inserted content.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct ContentInsertionConfiguration {
    /// Called when a user inserts content through the virtual / on-screen keyboard,
    /// currently only used on Android.
    pub on_content_inserted: Rc<dyn Fn(&mut App, KeyboardInsertedContent)>,
    /// Used when a user inserts image-based content through the device keyboard,
    /// currently only used on Android.
    pub allowed_mime_types: Vec<String>,
}

impl ContentInsertionConfiguration {
    /// Creates a content insertion configuration with the specified options.
    ///
    /// A handler for inserted content, in the form of [`on_content_inserted`](Self::on_content_inserted),
    /// must be supplied.
    pub fn new(
        on_content_inserted: impl Fn(&mut App, KeyboardInsertedContent) + 'static,
    ) -> ContentInsertionConfiguration {
        ContentInsertionConfiguration {
            on_content_inserted: Rc::new(on_content_inserted),
            allowed_mime_types: K_DEFAULT_CONTENT_INSERTION_MIME_TYPES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }

    /// Dart `ContentInsertionConfiguration(allowedMimeTypes:)`.
    pub fn allowed_mime_types(mut self, allowed_mime_types: Vec<String>) -> Self {
        debug_assert!(
            !allowed_mime_types.is_empty(),
            "allowedMimeTypes cannot be an empty list"
        );
        self.allowed_mime_types = allowed_mime_types;
        self
    }
}

/// Signature for the callback that reports when the user changes the selection
/// (including the cursor location).
pub type SelectionChangedCallback =
    Rc<dyn Fn(&mut App, TextSelection, Option<SelectionChangedCause>)>;

/// Signature for the callback that reports the app private command results.
///
/// The map Flutter passes is dropped at [`TextInputClient::perform_private_command`].
pub type AppPrivateCommandCallback = Rc<dyn Fn(&mut App, &str)>;

/// Signature for a widget builder that builds a context menu for the given
/// [`EditableTextState`].
pub type EditableTextContextMenuBuilder =
    Rc<dyn Fn(&mut App, BuildContext, Handle<EditableTextState>) -> WidgetRef>;

const K_CURSOR_BLINK_HALF_PERIOD: Duration = Duration::from_millis(500);
const K_OBSCURE_SHOW_LATEST_CHAR_CURSOR_TICKS: i32 = 3;

static DEBUG_DETERMINISTIC_CURSOR: AtomicBool = AtomicBool::new(false);

/// A time-value pair that represents a key frame in an animation.
#[derive(Clone, Copy, Debug)]
struct KeyFrame {
    time: f64,
    value: f64,
}

impl KeyFrame {
    const fn new(time: f64, value: f64) -> KeyFrame {
        KeyFrame { time, value }
    }

    const IOS_BLINKING_CARET_KEY_FRAMES: &[KeyFrame] = &[
        KeyFrame::new(0.0, 1.0),
        KeyFrame::new(0.5, 1.0),
        KeyFrame::new(0.5375, 0.75),
        KeyFrame::new(0.575, 0.5),
        KeyFrame::new(0.6125, 0.25),
        KeyFrame::new(0.65, 0.0),
        KeyFrame::new(0.85, 0.0),
        KeyFrame::new(0.8875, 0.25),
        KeyFrame::new(0.925, 0.5),
        KeyFrame::new(0.9625, 0.75),
        KeyFrame::new(1.0, 1.0),
    ];
}

/// Discrete keyframe simulation used for the iOS caret blink.
struct DiscreteKeyFrameSimulation {
    key_frames: &'static [KeyFrame],
    max_duration: f64,
    last_key_frame_index: Cell<usize>,
    tolerance: Tolerance,
}

impl DiscreteKeyFrameSimulation {
    fn ios_blinking_caret() -> DiscreteKeyFrameSimulation {
        DiscreteKeyFrameSimulation::new(KeyFrame::IOS_BLINKING_CARET_KEY_FRAMES, 1.0)
    }

    fn new(key_frames: &'static [KeyFrame], max_duration: f64) -> DiscreteKeyFrameSimulation {
        debug_assert!(!key_frames.is_empty());
        debug_assert!(
            key_frames
                .last()
                .is_none_or(|frame| frame.time <= max_duration)
        );
        if cfg!(debug_assertions) {
            for i in 0..key_frames.len().saturating_sub(1) {
                debug_assert!(
                    key_frames[i].time <= key_frames[i + 1].time,
                    "The key frame sequence must be sorted by time."
                );
            }
        }
        DiscreteKeyFrameSimulation {
            key_frames,
            max_duration,
            last_key_frame_index: Cell::new(0),
            tolerance: Tolerance::default(),
        }
    }
}

impl Debug for DiscreteKeyFrameSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiscreteKeyFrameSimulation")
            .field("max_duration", &self.max_duration)
            .finish()
    }
}

impl Simulation for DiscreteKeyFrameSimulation {
    fn dx(&self, _time: f64) -> f64 {
        0.0
    }

    fn is_done(&self, time: f64) -> bool {
        time >= self.max_duration
    }

    fn x(&self, time: f64) -> f64 {
        let length = self.key_frames.len();
        let last = self.last_key_frame_index.get();
        let (mut search_index, end_index) = if self.key_frames[last].time > time {
            (0, last)
        } else {
            (last, length)
        };
        while search_index < end_index.saturating_sub(1) {
            debug_assert!(self.key_frames[search_index].time <= time);
            let next = self.key_frames[search_index + 1];
            if time < next.time {
                break;
            }
            search_index += 1;
        }
        self.last_key_frame_index.set(search_index);
        self.key_frames[self.last_key_frame_index.get()].value
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

/// A basic text input field.
#[allow(clippy::type_complexity)]
pub struct EditableText {
    pub key: Option<KeyRef>,
    /// Controls the text being edited.
    pub controller: Handle<TextEditingController>,
    /// Controls whether this widget has keyboard focus.
    pub focus_node: Handle<FocusNode>,
    pub read_only: bool,
    pub obscuring_character: String,
    pub obscure_text: bool,
    pub autocorrect: bool,
    pub smart_dashes_type: SmartDashesType,
    pub smart_quotes_type: SmartQuotesType,
    pub enable_suggestions: bool,
    pub style: TextStyle,
    pub cursor_color: Color,
    pub background_cursor_color: Color,
    pub text_align: TextAlign,
    pub text_direction: Option<TextDirection>,
    pub locale: Option<Locale>,
    pub text_scale_factor: Option<f64>,
    pub text_scaler: Option<TextScaler>,
    pub max_lines: Option<i32>,
    pub min_lines: Option<i32>,
    pub expands: bool,
    pub force_line: bool,
    pub text_height_behavior: Option<TextHeightBehavior>,
    pub text_width_basis: TextWidthBasis,
    pub autofocus: bool,
    pub show_cursor: bool,
    pub show_selection_handles: bool,
    pub selection_color: Option<Color>,
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    pub keyboard_type: TextInputType,
    pub text_input_action: Option<TextInputAction>,
    pub text_capitalization: TextCapitalization,
    pub on_changed: Option<Rc<dyn Fn(&mut App, String)>>,
    pub on_editing_complete: Option<Rc<dyn Fn(&mut App)>>,
    pub on_submitted: Option<Rc<dyn Fn(&mut App, String)>>,
    pub on_app_private_command: Option<AppPrivateCommandCallback>,
    pub on_selection_changed: Option<SelectionChangedCallback>,
    pub on_selection_handle_tapped: Option<Rc<dyn Fn(&mut App)>>,
    pub group_id: TypeId,
    pub on_tap_outside: Option<Rc<dyn Fn(&mut App, PointerDownEvent)>>,
    pub on_tap_up_outside: Option<Rc<dyn Fn(&mut App, PointerUpEvent)>>,
    pub input_formatters: Option<Vec<TextInputFormatterRef>>,
    pub mouse_cursor: Option<MouseCursorRef>,
    pub renderer_ignores_pointer: bool,
    pub cursor_width: f64,
    pub cursor_height: Option<f64>,
    pub cursor_radius: Option<Radius>,
    pub cursor_opacity_animates: bool,
    pub cursor_offset: Option<Offset>,
    pub paint_cursor_above_text: bool,
    pub selection_height_style: BoxHeightStyle,
    pub selection_width_style: BoxWidthStyle,
    pub scroll_padding: EdgeInsets,
    pub keyboard_appearance: Brightness,
    pub drag_start_behavior: DragStartBehavior,
    pub enable_interactive_selection: bool,
    pub select_all_on_focus: Option<bool>,
    pub scroll_controller: Option<Handle<ScrollController>>,
    pub scroll_physics: Option<ScrollPhysicsRef>,
    pub autocorrection_text_rect_color: Option<Color>,
    pub toolbar_options: ToolbarOptions,
    pub autofill_hints: Vec<String>,
    pub autofill_client: Option<AnyAutofillClient>,
    pub clip_behavior: Clip,
    pub restoration_id: Option<String>,
    pub scroll_behavior: Option<ScrollBehaviorRef>,
    pub scribble_enabled: bool,
    pub stylus_handwriting_enabled: bool,
    pub enable_ime_personalized_learning: bool,
    pub content_insertion_configuration: Option<ContentInsertionConfiguration>,
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,
    pub spell_check_configuration: Option<SpellCheckConfiguration>,
    pub magnifier_configuration: TextMagnifierConfiguration,
    pub undo_controller: Option<Handle<UndoHistoryController>>,
    pub hint_locales: Option<Vec<Locale>>,
    pub enable_inline_prediction: Option<bool>,
}

impl EditableText {
    /// The default value for [`selection_height_style`](Self::selection_height_style).
    pub fn default_selection_height_style() -> BoxHeightStyle {
        if K_IS_WEB {
            BoxHeightStyle::Max
        } else {
            BoxHeightStyle::IncludeLineSpacingMiddle
        }
    }

    /// The default value for [`selection_width_style`](Self::selection_width_style).
    pub fn default_selection_width_style() -> BoxWidthStyle {
        if K_IS_WEB {
            BoxWidthStyle::Tight
        } else {
            BoxWidthStyle::Max
        }
    }

    /// The default value for [`stylus_handwriting_enabled`](Self::stylus_handwriting_enabled).
    pub const DEFAULT_STYLUS_HANDWRITING_ENABLED: bool = true;

    /// Whether the blinking cursor should be drawn as always-on for tests.
    pub fn debug_deterministic_cursor() -> bool {
        DEBUG_DETERMINISTIC_CURSOR.load(Ordering::Relaxed)
    }

    /// Dart's `EditableText.debugDeterministicCursor` setter.
    pub fn set_debug_deterministic_cursor(value: bool) {
        DEBUG_DETERMINISTIC_CURSOR.store(value, Ordering::Relaxed);
    }

    /// Creates a basic text input field.
    pub fn new(
        controller: Handle<TextEditingController>,
        focus_node: Handle<FocusNode>,
        style: TextStyle,
        cursor_color: Color,
        background_cursor_color: Color,
    ) -> EditableText {
        let obscure_text = false;
        let read_only = false;
        let max_lines = Some(1);
        let autofill_hints = Vec::new();
        let autocorrect = infer_autocorrect(&autofill_hints);
        let keyboard_type = infer_keyboard_type(&autofill_hints, max_lines);
        let input_formatters = Some(vec![FilteringTextInputFormatter::single_line_formatter()]);
        let toolbar_options = ToolbarOptions::new()
            .copy(true)
            .cut(true)
            .select_all(true)
            .paste(true);
        debug_assert!(!obscure_text || max_lines == Some(1));
        EditableText {
            key: None,
            controller,
            focus_node,
            read_only,
            obscuring_character: "\u{2022}".into(),
            obscure_text,
            autocorrect,
            smart_dashes_type: SmartDashesType::Enabled,
            smart_quotes_type: SmartQuotesType::Enabled,
            enable_suggestions: true,
            style,
            cursor_color,
            background_cursor_color,
            text_align: TextAlign::Start,
            text_direction: None,
            locale: None,
            text_scale_factor: None,
            text_scaler: None,
            max_lines,
            min_lines: None,
            expands: false,
            force_line: true,
            text_height_behavior: None,
            text_width_basis: TextWidthBasis::Parent,
            autofocus: false,
            show_cursor: !read_only,
            show_selection_handles: false,
            selection_color: None,
            selection_controls: None,
            keyboard_type,
            text_input_action: None,
            text_capitalization: TextCapitalization::None,
            on_changed: None,
            on_editing_complete: None,
            on_submitted: None,
            on_app_private_command: None,
            on_selection_changed: None,
            on_selection_handle_tapped: None,
            group_id: TypeId::of::<EditableText>(),
            on_tap_outside: None,
            on_tap_up_outside: None,
            input_formatters,
            mouse_cursor: None,
            renderer_ignores_pointer: false,
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: None,
            cursor_opacity_animates: false,
            cursor_offset: None,
            paint_cursor_above_text: false,
            selection_height_style: Self::default_selection_height_style(),
            selection_width_style: Self::default_selection_width_style(),
            scroll_padding: EdgeInsets::all(20.0),
            keyboard_appearance: Brightness::Light,
            drag_start_behavior: DragStartBehavior::Start,
            enable_interactive_selection: true,
            select_all_on_focus: None,
            scroll_controller: None,
            scroll_physics: None,
            autocorrection_text_rect_color: None,
            toolbar_options,
            autofill_hints,
            autofill_client: None,
            clip_behavior: Clip::HardEdge,
            restoration_id: None,
            scroll_behavior: None,
            scribble_enabled: true,
            stylus_handwriting_enabled: Self::DEFAULT_STYLUS_HANDWRITING_ENABLED,
            enable_ime_personalized_learning: true,
            content_insertion_configuration: None,
            context_menu_builder: None,
            spell_check_configuration: None,
            magnifier_configuration: TextMagnifierConfiguration::DISABLED,
            undo_controller: None,
            hint_locales: None,
            enable_inline_prediction: None,
        }
    }

    /// Dart `EditableText(key:)`.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Dart `EditableText(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self.show_cursor = !read_only;
        self.enable_interactive_selection = !read_only || !self.obscure_text;
        self.toolbar_options = default_toolbar_options(self.read_only, self.obscure_text);
        self
    }

    /// Dart `EditableText(obscuringCharacter:)`.
    pub fn obscuring_character(mut self, obscuring_character: impl Into<String>) -> Self {
        let obscuring_character = obscuring_character.into();
        debug_assert!(
            obscuring_character.chars().count() == 1,
            "obscuringCharacter must be a single character"
        );
        self.obscuring_character = obscuring_character;
        self
    }

    /// Dart `EditableText(obscureText:)`.
    pub fn obscure_text(mut self, obscure_text: bool) -> Self {
        debug_assert!(!obscure_text || self.max_lines == Some(1));
        self.obscure_text = obscure_text;
        self.smart_dashes_type = if obscure_text {
            SmartDashesType::Disabled
        } else {
            SmartDashesType::Enabled
        };
        self.smart_quotes_type = if obscure_text {
            SmartQuotesType::Disabled
        } else {
            SmartQuotesType::Enabled
        };
        self.enable_interactive_selection = !self.read_only || !obscure_text;
        self.toolbar_options = default_toolbar_options(self.read_only, obscure_text);
        self
    }

    /// Dart `EditableText(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> Self {
        self.autocorrect = autocorrect;
        self
    }

    /// Dart `EditableText(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> Self {
        self.smart_dashes_type = smart_dashes_type;
        self
    }

    /// Dart `EditableText(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> Self {
        self.smart_quotes_type = smart_quotes_type;
        self
    }

    /// Dart `EditableText(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> Self {
        self.enable_suggestions = enable_suggestions;
        self
    }

    /// Dart `EditableText(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;
        self
    }

    /// Dart `EditableText(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `EditableText(locale:)`.
    pub fn locale(mut self, locale: Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Dart `EditableText(textScaleFactor:)`.
    pub fn text_scale_factor(mut self, text_scale_factor: f64) -> Self {
        self.text_scale_factor = Some(text_scale_factor);
        self
    }

    /// Dart `EditableText(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Self {
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `EditableText(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> Self {
        debug_assert!(
            max_lines.is_none_or(|lines| self.min_lines.is_none_or(|min| lines >= min)),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(!self.expands || max_lines.is_none());
        debug_assert!(!self.obscure_text || max_lines == Some(1));
        self.max_lines = max_lines;
        self.keyboard_type = infer_keyboard_type(&self.autofill_hints, max_lines);
        self.input_formatters = match max_lines {
            Some(1) => {
                let mut formatters = vec![FilteringTextInputFormatter::single_line_formatter()];
                if let Some(existing) = self.input_formatters.take() {
                    formatters.extend(existing.into_iter().filter(|formatter| {
                        !Rc::ptr_eq(
                            &formatter.0,
                            &FilteringTextInputFormatter::single_line_formatter().0,
                        )
                    }));
                }
                Some(formatters)
            }
            _ => self.input_formatters.take().map(|formatters| {
                formatters
                    .into_iter()
                    .filter(|formatter| {
                        !Rc::ptr_eq(
                            &formatter.0,
                            &FilteringTextInputFormatter::single_line_formatter().0,
                        )
                    })
                    .collect()
            }),
        };
        self
    }

    /// Dart `EditableText(minLines:)`.
    pub fn min_lines(mut self, min_lines: i32) -> Self {
        debug_assert!(min_lines > 0);
        debug_assert!(
            self.max_lines.is_none_or(|max| max >= min_lines),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(!self.expands);
        self.min_lines = Some(min_lines);
        self
    }

    /// Dart `EditableText(expands:)`.
    pub fn expands(mut self, expands: bool) -> Self {
        debug_assert!(!expands || (self.max_lines.is_none() && self.min_lines.is_none()));
        self.expands = expands;
        self
    }

    /// Dart `EditableText(forceLine:)`.
    pub fn force_line(mut self, force_line: bool) -> Self {
        self.force_line = force_line;
        self
    }

    /// Dart `EditableText(textHeightBehavior:)`.
    pub fn text_height_behavior(mut self, text_height_behavior: TextHeightBehavior) -> Self {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }

    /// Dart `EditableText(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> Self {
        self.text_width_basis = text_width_basis;
        self
    }

    /// Dart `EditableText(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Dart `EditableText(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> Self {
        self.show_cursor = show_cursor;
        self
    }

    /// Dart `EditableText(showSelectionHandles:)`.
    pub fn show_selection_handles(mut self, show_selection_handles: bool) -> Self {
        self.show_selection_handles = show_selection_handles;
        self
    }

    /// Dart `EditableText(selectionColor:)`.
    pub fn selection_color(mut self, selection_color: Color) -> Self {
        self.selection_color = Some(selection_color);
        self
    }

    /// Dart `EditableText(selectionControls:)`.
    pub fn selection_controls(mut self, selection_controls: Rc<dyn TextSelectionControls>) -> Self {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `EditableText(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> Self {
        self.keyboard_type = keyboard_type;
        self
    }

    /// Dart `EditableText(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> Self {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `EditableText(textCapitalization:)`.
    pub fn text_capitalization(mut self, text_capitalization: TextCapitalization) -> Self {
        self.text_capitalization = text_capitalization;
        self
    }

    /// Dart `EditableText(onChanged:)`.
    pub fn on_changed(mut self, on_changed: impl Fn(&mut App, String) + 'static) -> Self {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `EditableText(onEditingComplete:)`.
    pub fn on_editing_complete(mut self, on_editing_complete: impl Fn(&mut App) + 'static) -> Self {
        self.on_editing_complete = Some(Rc::new(on_editing_complete));
        self
    }

    /// Dart `EditableText(onSubmitted:)`.
    pub fn on_submitted(mut self, on_submitted: impl Fn(&mut App, String) + 'static) -> Self {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `EditableText(onAppPrivateCommand:)`.
    pub fn on_app_private_command(
        mut self,
        on_app_private_command: impl Fn(&mut App, &str) + 'static,
    ) -> Self {
        self.on_app_private_command = Some(Rc::new(on_app_private_command));
        self
    }

    /// Dart `EditableText(onSelectionChanged:)`.
    pub fn on_selection_changed(
        mut self,
        on_selection_changed: impl Fn(&mut App, TextSelection, Option<SelectionChangedCause>) + 'static,
    ) -> Self {
        self.on_selection_changed = Some(Rc::new(on_selection_changed));
        self
    }

    /// Dart `EditableText(onSelectionHandleTapped:)`.
    pub fn on_selection_handle_tapped(
        mut self,
        on_selection_handle_tapped: impl Fn(&mut App) + 'static,
    ) -> Self {
        self.on_selection_handle_tapped = Some(Rc::new(on_selection_handle_tapped));
        self
    }

    /// Dart `EditableText(groupId:)`.
    pub fn group_id(mut self, group_id: TypeId) -> Self {
        self.group_id = group_id;
        self
    }

    /// Dart `EditableText(onTapOutside:)`.
    pub fn on_tap_outside(
        mut self,
        on_tap_outside: impl Fn(&mut App, PointerDownEvent) + 'static,
    ) -> Self {
        self.on_tap_outside = Some(Rc::new(on_tap_outside));
        self
    }

    /// Dart `EditableText(onTapUpOutside:)`.
    pub fn on_tap_up_outside(
        mut self,
        on_tap_up_outside: impl Fn(&mut App, PointerUpEvent) + 'static,
    ) -> Self {
        self.on_tap_up_outside = Some(Rc::new(on_tap_up_outside));
        self
    }

    /// Dart `EditableText(inputFormatters:)`.
    pub fn input_formatters(mut self, input_formatters: Vec<TextInputFormatterRef>) -> Self {
        self.input_formatters = if self.max_lines == Some(1) {
            let mut formatters = vec![FilteringTextInputFormatter::single_line_formatter()];
            formatters.extend(input_formatters);
            Some(formatters)
        } else {
            Some(input_formatters)
        };
        self
    }

    /// Dart `EditableText(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> Self {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `EditableText(rendererIgnoresPointer:)`.
    pub fn renderer_ignores_pointer(mut self, renderer_ignores_pointer: bool) -> Self {
        self.renderer_ignores_pointer = renderer_ignores_pointer;
        self
    }

    /// Dart `EditableText(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> Self {
        self.cursor_width = cursor_width;
        self
    }

    /// Dart `EditableText(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> Self {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `EditableText(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> Self {
        self.cursor_radius = Some(cursor_radius);
        self
    }

    /// Dart `EditableText(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(mut self, cursor_opacity_animates: bool) -> Self {
        self.cursor_opacity_animates = cursor_opacity_animates;
        self
    }

    /// Dart `EditableText(cursorOffset:)`.
    pub fn cursor_offset(mut self, cursor_offset: Offset) -> Self {
        self.cursor_offset = Some(cursor_offset);
        self
    }

    /// Dart `EditableText(paintCursorAboveText:)`.
    pub fn paint_cursor_above_text(mut self, paint_cursor_above_text: bool) -> Self {
        self.paint_cursor_above_text = paint_cursor_above_text;
        self
    }

    /// Dart `EditableText(selectionHeightStyle:)`.
    pub fn selection_height_style(mut self, selection_height_style: BoxHeightStyle) -> Self {
        self.selection_height_style = selection_height_style;
        self
    }

    /// Dart `EditableText(selectionWidthStyle:)`.
    pub fn selection_width_style(mut self, selection_width_style: BoxWidthStyle) -> Self {
        self.selection_width_style = selection_width_style;
        self
    }

    /// Dart `EditableText(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> Self {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Dart `EditableText(keyboardAppearance:)`.
    pub fn keyboard_appearance(mut self, keyboard_appearance: Brightness) -> Self {
        self.keyboard_appearance = keyboard_appearance;
        self
    }

    /// Dart `EditableText(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `EditableText(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(mut self, enable_interactive_selection: bool) -> Self {
        self.enable_interactive_selection = enable_interactive_selection;
        self
    }

    /// Dart `EditableText(selectAllOnFocus:)`.
    pub fn select_all_on_focus(mut self, select_all_on_focus: bool) -> Self {
        self.select_all_on_focus = Some(select_all_on_focus);
        self
    }

    /// Dart `EditableText(scrollController:)`.
    pub fn scroll_controller(mut self, scroll_controller: Handle<ScrollController>) -> Self {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `EditableText(scrollPhysics:)`.
    pub fn scroll_physics(mut self, scroll_physics: ScrollPhysicsRef) -> Self {
        self.scroll_physics = Some(scroll_physics);
        self
    }

    /// Dart `EditableText(autocorrectionTextRectColor:)`.
    pub fn autocorrection_text_rect_color(mut self, autocorrection_text_rect_color: Color) -> Self {
        self.autocorrection_text_rect_color = Some(autocorrection_text_rect_color);
        self
    }

    /// Dart `EditableText(toolbarOptions:)`.
    pub fn toolbar_options(mut self, toolbar_options: ToolbarOptions) -> Self {
        self.toolbar_options = toolbar_options;
        self
    }

    /// Dart `EditableText(autofillHints:)`.
    pub fn autofill_hints(mut self, autofill_hints: Vec<String>) -> Self {
        self.autofill_hints = autofill_hints;
        self.autocorrect = infer_autocorrect(&self.autofill_hints);
        self.keyboard_type = infer_keyboard_type(&self.autofill_hints, self.max_lines);
        self
    }

    /// Dart `EditableText(autofillClient:)`.
    pub fn autofill_client(mut self, autofill_client: AnyAutofillClient) -> Self {
        self.autofill_client = Some(autofill_client);
        self
    }

    /// Dart `EditableText(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `EditableText(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> Self {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `EditableText(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> Self {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `EditableText(scribbleEnabled:)`.
    pub fn scribble_enabled(mut self, scribble_enabled: bool) -> Self {
        self.scribble_enabled = scribble_enabled;
        self
    }

    /// Dart `EditableText(stylusHandwritingEnabled:)`.
    pub fn stylus_handwriting_enabled(mut self, stylus_handwriting_enabled: bool) -> Self {
        self.stylus_handwriting_enabled = stylus_handwriting_enabled;
        self
    }

    /// Dart `EditableText(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> Self {
        self.enable_ime_personalized_learning = enable_ime_personalized_learning;
        self
    }

    /// Dart `EditableText(contentInsertionConfiguration:)`.
    pub fn content_insertion_configuration(
        mut self,
        content_insertion_configuration: ContentInsertionConfiguration,
    ) -> Self {
        self.content_insertion_configuration = Some(content_insertion_configuration);
        self
    }

    /// Dart `EditableText(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: EditableTextContextMenuBuilder,
    ) -> Self {
        self.context_menu_builder = Some(context_menu_builder);
        self
    }

    /// Dart `EditableText(spellCheckConfiguration:)`.
    pub fn spell_check_configuration(
        mut self,
        spell_check_configuration: SpellCheckConfiguration,
    ) -> Self {
        if cfg!(debug_assertions)
            && spell_check_configuration.spell_check_enabled()
            && spell_check_configuration.misspelled_text_style.is_none()
        {
            debug_assert!(
                false,
                "spellCheckConfiguration must specify a misspelledTextStyle if spell check behavior is desired"
            );
        }
        self.spell_check_configuration = Some(spell_check_configuration);
        self
    }

    /// Dart `EditableText(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> Self {
        self.magnifier_configuration = magnifier_configuration;
        self
    }

    /// Dart `EditableText(undoController:)`.
    pub fn undo_controller(mut self, undo_controller: Handle<UndoHistoryController>) -> Self {
        self.undo_controller = Some(undo_controller);
        self
    }

    /// Dart `EditableText(hintLocales:)`.
    pub fn hint_locales(mut self, hint_locales: Vec<Locale>) -> Self {
        self.hint_locales = Some(hint_locales);
        self
    }

    /// Dart `EditableText(enableInlinePrediction:)`.
    pub fn enable_inline_prediction(mut self, enable_inline_prediction: bool) -> Self {
        self.enable_inline_prediction = Some(enable_inline_prediction);
        self
    }

    /// Returns the [`ContextMenuButtonItem`]s representing the buttons in this
    /// platform's default selection menu by default.
    #[allow(clippy::too_many_arguments)]
    pub fn get_editable_button_items(
        clipboard_status: Option<ClipboardStatus>,
        on_copy: Option<Listener>,
        on_cut: Option<Listener>,
        on_paste: Option<Listener>,
        on_select_all: Option<Listener>,
        on_look_up: Option<Listener>,
        on_search_web: Option<Listener>,
        on_share: Option<Listener>,
        on_live_text_input: Option<Listener>,
        target_platform: TargetPlatform,
    ) -> Vec<ContextMenuButtonItem> {
        let mut result_button_item = Vec::new();
        if on_paste.is_none() || clipboard_status != Some(ClipboardStatus::Unknown) {
            let show_share_before_select_all = target_platform == TargetPlatform::Android;
            if let Some(on_cut) = on_cut {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_cut)).r#type(ContextMenuButtonType::Cut),
                );
            }
            if let Some(on_copy) = on_copy {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_copy)).r#type(ContextMenuButtonType::Copy),
                );
            }
            if let Some(on_paste) = on_paste {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_paste)).r#type(ContextMenuButtonType::Paste),
                );
            }
            if let Some(on_share) = on_share.clone()
                && show_share_before_select_all
            {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_share)).r#type(ContextMenuButtonType::Share),
                );
            }
            if let Some(on_select_all) = on_select_all {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_select_all))
                        .r#type(ContextMenuButtonType::SelectAll),
                );
            }
            if let Some(on_look_up) = on_look_up {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_look_up))
                        .r#type(ContextMenuButtonType::LookUp),
                );
            }
            if let Some(on_search_web) = on_search_web {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_search_web))
                        .r#type(ContextMenuButtonType::SearchWeb),
                );
            }
            if let Some(on_share) = on_share
                && !show_share_before_select_all
            {
                result_button_item.push(
                    ContextMenuButtonItem::new(Some(on_share)).r#type(ContextMenuButtonType::Share),
                );
            }
        }
        if let Some(on_live_text_input) = on_live_text_input {
            result_button_item.push(
                ContextMenuButtonItem::new(Some(on_live_text_input))
                    .r#type(ContextMenuButtonType::LiveTextInput),
            );
        }
        result_button_item
    }

    fn user_selection_enabled(&self) -> bool {
        self.enable_interactive_selection && (!self.read_only || !self.obscure_text)
    }

    fn select_all_on_focus_or_default(&self, app: &App) -> bool {
        self.select_all_on_focus
            .unwrap_or_else(|| default_select_all_on_focus(app.platform().target_platform()))
    }
}

impl Debug for EditableText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditableText")
            .field("controller", &self.controller)
            .field("focus_node", &self.focus_node)
            .field("obscure_text", &self.obscure_text)
            .field("read_only", &self.read_only)
            .field("style", &self.style)
            .field("max_lines", &self.max_lines)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for EditableText {
    type State = EditableTextState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> EditableTextState {
        EditableTextState {
            state: StateData::new(),
            automatic_keep_alive_client: AutomaticKeepAliveClientMixinData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            cursor_timer: None,
            backing_cursor_blink_opacity_controller: None,
            cursor_visibility_notifier: None,
            clipboard_status: None,
            live_text_input_status: None,
            text_input_connection: None,
            composite_callback: None,
            last_known_remote_text_editing_value: None,
            internal_scroll_controller: None,
            toolbar_layer_link: None,
            start_handle_layer_link: None,
            end_handle_layer_link: None,
            did_auto_focus: false,
            process_text_service: Rc::new(DefaultProcessTextService),
            process_text_actions: Vec::new(),
            spell_check_configuration: SpellCheckConfiguration::disabled(),
            spell_check_results: None,
            current_autofill_scope: None,
            app_lifecycle_listener: None,
            just_resumed: false,
            had_focus_on_tap_down: false,
            actions: None,
            vertical_movement_run: None,
            run_selection: None,
            editable_key: Rc::new(GlobalKey::new()),
            scrollable_key: Rc::new(GlobalKey::new()),
            style: TextStyle::new(),
            tickers_enabled: true,
            batch_edit_depth: 0,
            current_prompt_rect_range: None,
            obscure_show_char_ticks_pending: 0,
            obscure_latest_char_index: None,
            last_orientation: None,
            binding_observer: None,
            registered_as_observer: false,
            has_focus: false,
            next_focus_change_is_internal: false,
            selection_overlay: None,
        }
    }
}

fn default_toolbar_options(read_only: bool, obscure_text: bool) -> ToolbarOptions {
    if obscure_text {
        if read_only {
            ToolbarOptions::EMPTY
        } else {
            ToolbarOptions::new().select_all(true).paste(true)
        }
    } else if read_only {
        ToolbarOptions::new().select_all(true).copy(true)
    } else {
        ToolbarOptions::new()
            .copy(true)
            .cut(true)
            .select_all(true)
            .paste(true)
    }
}

fn find_render_editable(
    app: &App,
    object: AnyRenderObject,
) -> Option<RenderHandle<RenderEditable>> {
    if let Some(editable) = object.downcast::<RenderEditable>(app) {
        return Some(editable);
    }
    let mut found = None;
    object.visit_children(app, &mut |child| {
        if found.is_none() {
            found = find_render_editable(app, child);
        }
    });
    found
}

fn infer_autocorrect(autofill_hints: &[String]) -> bool {
    if autofill_hints.is_empty() || K_IS_WEB {
        return true;
    }
    true
}

fn infer_keyboard_type(autofill_hints: &[String], max_lines: Option<i32>) -> TextInputType {
    if autofill_hints.is_empty() {
        return if max_lines == Some(1) {
            TextInputType::TEXT
        } else {
            TextInputType::MULTILINE
        };
    }
    match autofill_hints[0].as_str() {
        AutofillHints::EMAIL => TextInputType::EMAIL_ADDRESS,
        AutofillHints::TELEPHONE_NUMBER
        | AutofillHints::TELEPHONE_NUMBER_NATIONAL
        | AutofillHints::TELEPHONE_NUMBER_DEVICE => TextInputType::PHONE,
        AutofillHints::URL => TextInputType::URL,
        AutofillHints::CREDIT_CARD_NUMBER => TextInputType::NUMBER,
        _ => TextInputType::TEXT,
    }
}

fn default_select_all_on_focus(platform: TargetPlatform) -> bool {
    if K_IS_WEB {
        return true;
    }
    match platform {
        TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => true,
        TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => false,
    }
}

fn is_password_input(
    obscure_text: bool,
    keyboard_type: TextInputType,
    autofill_hints: &[String],
) -> bool {
    obscure_text
        || keyboard_type == TextInputType::VISIBLE_PASSWORD
        || keyboard_type.password == Some(true)
        || autofill_hints
            .iter()
            .any(|hint| hint == AutofillHints::PASSWORD || hint == AutofillHints::NEW_PASSWORD)
}

fn infer_spell_check_configuration(
    configuration: Option<&SpellCheckConfiguration>,
    obscure_text: bool,
    keyboard_type: TextInputType,
    autofill_hints: &[String],
) -> SpellCheckConfiguration {
    let disabled = is_password_input(obscure_text, keyboard_type, autofill_hints)
        || configuration.is_none()
        || configuration.is_some_and(|config| !config.spell_check_enabled());
    let service_configured = configuration
        .and_then(|config| config.spell_check_service.as_ref())
        .is_some();
    if disabled || !service_configured {
        return SpellCheckConfiguration::disabled();
    }
    configuration
        .cloned()
        .unwrap_or_else(SpellCheckConfiguration::disabled)
}

/// State for a [`EditableText`].
pub struct EditableTextState {
    state: StateData<EditableText>,
    automatic_keep_alive_client: AutomaticKeepAliveClientMixinData,
    ticker_provider: TickerProviderStateMixinData,
    cursor_timer: Option<Timer>,
    backing_cursor_blink_opacity_controller: Option<Handle<AnimationController>>,
    cursor_visibility_notifier: Option<Handle<ValueNotifier<bool>>>,
    clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    live_text_input_status: Option<Handle<LiveTextInputStatusNotifier>>,
    text_input_connection: Option<Handle<TextInputConnection>>,
    /// Dart's `_compositeCallback` tear-off; one `Rc` so the widget can assert identity.
    composite_callback: Option<CompositionCallback>,
    last_known_remote_text_editing_value: Option<TextEditingValue>,
    internal_scroll_controller: Option<Handle<ScrollController>>,
    toolbar_layer_link: Option<Handle<LayerLink>>,
    start_handle_layer_link: Option<Handle<LayerLink>>,
    end_handle_layer_link: Option<Handle<LayerLink>>,
    did_auto_focus: bool,
    process_text_service: Rc<dyn ProcessTextService>,
    process_text_actions: Vec<ProcessTextAction>,
    spell_check_configuration: SpellCheckConfiguration,
    spell_check_results: Option<SpellCheckResults>,
    current_autofill_scope: Option<Handle<AutofillGroupState>>,
    app_lifecycle_listener: Option<Handle<AppLifecycleListener>>,
    just_resumed: bool,
    had_focus_on_tap_down: bool,
    actions: Option<HashMap<TypeId, AnyAction>>,
    /// Dart keeps these on the one shared vertical action; both vertical intents map
    /// to that instance, so the run is shared here instead.
    vertical_movement_run: Option<VerticalCaretMovementRun>,
    run_selection: Option<TextSelection>,
    editable_key: Rc<GlobalKey>,
    scrollable_key: Rc<GlobalKey>,
    style: TextStyle,
    tickers_enabled: bool,
    batch_edit_depth: i32,
    current_prompt_rect_range: Option<TextRange>,
    obscure_show_char_ticks_pending: i32,
    obscure_latest_char_index: Option<i32>,
    last_orientation: Option<Orientation>,
    binding_observer: Option<WidgetsBindingObserverRef>,
    registered_as_observer: bool,
    has_focus: bool,
    next_focus_change_is_internal: bool,
    selection_overlay: Option<Handle<TextSelectionOverlay>>,
}

impl Debug for EditableTextState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditableTextState")
            .field("has_focus", &self.has_focus)
            .field("batch_edit_depth", &self.batch_edit_depth)
            .finish_non_exhaustive()
    }
}

impl EditableTextState {
    /// Detects whether the clipboard can paste.
    pub fn clipboard_status(self: Handle<Self>, app: &App) -> Handle<ClipboardStatusNotifier> {
        app.get(self)
            .clipboard_status
            .expect("clipboardStatus is created in initState")
    }

    /// The [`RenderEditable`] this state created.
    pub fn render_editable(self: Handle<Self>, app: &App) -> RenderHandle<RenderEditable> {
        let object = self
            .context(app)
            .find_render_object(app)
            .expect("EditableText has a render object");
        find_render_editable(app, object).expect("EditableText should mount a RenderEditable")
    }

    /// Gets the line heights at the start and end of the selection.
    pub fn get_glyph_heights(self: Handle<Self>, app: &mut App) -> (f64, f64) {
        let selection = self.value(app).selection;
        let render_editable = self.render_editable(app);
        let line_height = render_editable.preferred_line_height(app);
        let Some(span) = render_editable.text(app) else {
            return (line_height, line_height);
        };
        let prev_text = span.to_plain_text(true, true);
        let curr_text = self.value(app).text.clone();
        if prev_text != curr_text || !selection.is_valid() || selection.is_collapsed() {
            return (line_height, line_height);
        }
        let selected = selection.range().text_inside(&curr_text);
        let first_extent = selected
            .chars()
            .next()
            .map(|c| c.len_utf16() as i32)
            .unwrap_or(0);
        let last_extent = selected
            .chars()
            .next_back()
            .map(|c| c.len_utf16() as i32)
            .unwrap_or(0);
        let start_rect = render_editable.get_rect_for_composing_range(
            app,
            TextRange::new(selection.start(), selection.start() + first_extent),
        );
        let end_rect = render_editable.get_rect_for_composing_range(
            app,
            TextRange::new(selection.end() - last_extent, selection.end()),
        );
        (
            start_rect.map_or(line_height, |rect| rect.height()),
            end_rect.map_or(line_height, |rect| rect.height()),
        )
    }

    /// Returns the anchor points for the default context menu.
    pub fn context_menu_anchors(self: Handle<Self>, app: &mut App) -> TextSelectionToolbarAnchors {
        let render_editable = self.render_editable(app);
        if let Some(position) = render_editable.last_secondary_tap_down_position(app) {
            return TextSelectionToolbarAnchors::new(position);
        }
        let (start_glyph_height, end_glyph_height) = self.get_glyph_heights(app);
        let selection = self.value(app).selection;
        let points = render_editable.get_endpoints_for_selection(app, selection);
        TextSelectionToolbarAnchors::from_selection(
            app,
            render_editable.as_box(),
            start_glyph_height,
            end_glyph_height,
            &points,
        )
    }

    /// Returns the [`ContextMenuButtonItem`]s representing the buttons in this
    /// platform's default selection menu for [`EditableText`].
    pub fn context_menu_button_items(
        self: Handle<Self>,
        app: &mut App,
    ) -> Vec<ContextMenuButtonItem> {
        if let Some(items) = self.button_items_for_toolbar_options(app) {
            let mut items = items;
            items.extend(self.text_processing_action_button_items(app));
            return items;
        }
        let clipboard_status = self.clipboard_status(app).value(app);
        let platform = app.platform().target_platform();
        let mut items = EditableText::get_editable_button_items(
            Some(clipboard_status),
            self.copy_enabled(app)
                .then(|| Listener::handle_method(self, Self::copy_selection_from_toolbar)),
            self.cut_enabled(app)
                .then(|| Listener::handle_method(self, Self::cut_selection_from_toolbar)),
            self.paste_enabled(app)
                .then(|| Listener::handle_method(self, Self::paste_text_from_toolbar)),
            self.select_all_enabled(app)
                .then(|| Listener::handle_method(self, Self::select_all_from_toolbar)),
            self.look_up_enabled(app)
                .then(|| Listener::handle_method(self, Self::look_up_selection_from_toolbar)),
            self.search_web_enabled(app).then(|| {
                Listener::handle_method(self, Self::search_web_for_selection_from_toolbar)
            }),
            self.share_enabled(app)
                .then(|| Listener::handle_method(self, Self::share_selection_from_toolbar)),
            self.live_text_input_enabled(app)
                .then(|| Listener::handle_method(self, Self::start_live_text_input_from_toolbar)),
            platform,
        );
        items.extend(self.text_processing_action_button_items(app));
        items
    }

    fn copy_selection_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.copy_selection(app, SelectionChangedCause::Toolbar);
    }

    fn cut_selection_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.cut_selection(app, SelectionChangedCause::Toolbar);
    }

    fn paste_text_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.paste_text(app, SelectionChangedCause::Toolbar);
    }

    fn select_all_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.select_all(app, SelectionChangedCause::Toolbar);
    }

    fn look_up_selection_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.look_up_selection(app, SelectionChangedCause::Toolbar);
    }

    fn search_web_for_selection_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.search_web_for_selection(app, SelectionChangedCause::Toolbar);
    }

    fn share_selection_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.share_selection(app, SelectionChangedCause::Toolbar);
    }

    fn start_live_text_input_from_toolbar(self: Handle<Self>, app: &mut App) {
        self.start_live_text_input(app, SelectionChangedCause::Toolbar);
    }

    /// Look up the current selection, as in the "Look Up" edit menu button on iOS.
    pub fn look_up_selection(self: Handle<Self>, app: &mut App, _cause: SelectionChangedCause) {
        debug_assert!(!self.widget(app).obscure_text);
        if self.widget(app).obscure_text {
            return;
        }
        let value = self.value(app);
        let text = value.selection.range().text_inside(&value.text);
        if text.is_empty() {
            return;
        }
        app.platform().look_up(text);
    }

    /// Launch a web search on the current selection.
    pub fn search_web_for_selection(
        self: Handle<Self>,
        app: &mut App,
        _cause: SelectionChangedCause,
    ) {
        debug_assert!(!self.widget(app).obscure_text);
        if self.widget(app).obscure_text {
            return;
        }
        let value = self.value(app);
        let text = value.selection.range().text_inside(&value.text);
        if !text.is_empty() {
            app.platform().search_web(text);
        }
    }

    /// Launch the share interface for the current selection.
    pub fn share_selection(self: Handle<Self>, app: &mut App, _cause: SelectionChangedCause) {
        debug_assert!(!self.widget(app).obscure_text);
        if self.widget(app).obscure_text {
            return;
        }
        let value = self.value(app);
        let text = value.selection.range().text_inside(&value.text);
        if !text.is_empty() {
            app.platform().share(text);
        }
    }

    fn start_live_text_input(self: Handle<Self>, app: &mut App, cause: SelectionChangedCause) {
        if !self.live_text_input_enabled(app) {
            return;
        }
        if self.has_input_connection(app) {
            LiveText::start_live_text_input(app);
        }
        if cause == SelectionChangedCause::Toolbar {
            TextSelectionDelegate::hide_toolbar(self, app, true);
        }
    }

    /// Returns the [`ContextMenuButtonItem`]s for the given [`ToolbarOptions`].
    pub fn button_items_for_toolbar_options(
        self: Handle<Self>,
        app: &mut App,
    ) -> Option<Vec<ContextMenuButtonItem>> {
        let toolbar_options = self.widget(app).toolbar_options;
        if toolbar_options == ToolbarOptions::EMPTY {
            return None;
        }
        let mut items = Vec::new();
        if toolbar_options.cut && self.cut_enabled(app) {
            items.push(
                ContextMenuButtonItem::new(Some(Listener::handle_method(
                    self,
                    Self::cut_selection_from_toolbar,
                )))
                .r#type(ContextMenuButtonType::Cut),
            );
        }
        if toolbar_options.copy && self.copy_enabled(app) {
            items.push(
                ContextMenuButtonItem::new(Some(Listener::handle_method(
                    self,
                    Self::copy_selection_from_toolbar,
                )))
                .r#type(ContextMenuButtonType::Copy),
            );
        }
        if toolbar_options.paste && self.paste_enabled(app) {
            items.push(
                ContextMenuButtonItem::new(Some(Listener::handle_method(
                    self,
                    Self::paste_text_from_toolbar,
                )))
                .r#type(ContextMenuButtonType::Paste),
            );
        }
        if toolbar_options.select_all && self.select_all_enabled(app) {
            items.push(
                ContextMenuButtonItem::new(Some(Listener::handle_method(
                    self,
                    Self::select_all_from_toolbar,
                )))
                .r#type(ContextMenuButtonType::SelectAll),
            );
        }
        Some(items)
    }

    fn text_processing_action_button_items(
        self: Handle<Self>,
        app: &mut App,
    ) -> Vec<ContextMenuButtonItem> {
        let mut button_items = Vec::new();
        let value = self.value(app);
        let selection = value.selection;
        if self.widget(app).obscure_text || !selection.is_valid() || selection.is_collapsed() {
            return button_items;
        }
        let actions = app.get(self).process_text_actions.clone();
        let service = app.get(self).process_text_service.clone();
        let read_only = self.widget(app).read_only;
        let selected_text = selection.range().text_inside(&value.text).to_string();
        for action in actions {
            let selected_text = selected_text.clone();
            let service = Rc::clone(&service);
            let action_id = action.id.clone();
            button_items.push(
                ContextMenuButtonItem::new(Some(Listener::new(move |app| {
                    if selected_text.is_empty() {
                        return;
                    }
                    match service.process_text_action(app, &action_id, &selected_text, read_only) {
                        Some(processed) if self.allow_paste(app) => {
                            self.paste_text_content(
                                app,
                                SelectionChangedCause::Toolbar,
                                &processed,
                            );
                        }
                        _ => TextSelectionDelegate::hide_toolbar(self, app, true),
                    }
                })))
                .label(action.label.clone()),
            );
        }
        button_items
    }

    fn allow_paste(self: Handle<Self>, app: &App) -> bool {
        !self.widget(app).read_only && self.value(app).selection.is_valid()
    }

    fn paste_text_content(
        self: Handle<Self>,
        app: &mut App,
        cause: SelectionChangedCause,
        text: &str,
    ) {
        if !self.allow_paste(app) {
            return;
        }
        let value = self.value(app);
        let last = value
            .selection
            .base_offset
            .max(value.selection.extent_offset);
        let collapsed = value
            .copy_with()
            .selection(TextSelection::collapsed(last, TextAffinity::Downstream));
        self.user_update_text_editing_value(app, collapsed.replaced(value.selection, text), cause);
        if cause == SelectionChangedCause::Toolbar {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::bring_selection_into_view_after_frame),
            );
            TextSelectionDelegate::hide_toolbar(self, app, true);
        }
    }

    /// Configuration that determines how spell check will be performed.
    pub fn spell_check_configuration(self: Handle<Self>, app: &App) -> SpellCheckConfiguration {
        app.get(self).spell_check_configuration.clone()
    }

    /// Whether or not spell check is enabled.
    pub fn spell_check_enabled(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .spell_check_configuration
            .spell_check_enabled()
    }

    /// The `AutofillGroup` this [`EditableText`] belongs to, if any.
    pub fn current_autofill_scope(
        self: Handle<Self>,
        app: &App,
    ) -> Option<Handle<AutofillGroupState>> {
        app.get(self).current_autofill_scope
    }

    fn spell_check_results_received(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .spell_check_results
            .as_ref()
            .is_some_and(|results| !results.suggestion_spans.is_empty())
    }

    /// Whether the blinking cursor is actually visible at this precise moment.
    pub fn cursor_currently_visible(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .backing_cursor_blink_opacity_controller
            .is_some_and(|controller| controller.value(app) > 0.0)
    }

    /// The cursor blink interval (the amount of time the cursor is in the "on"
    /// state or the "off" state).
    pub fn cursor_blink_interval() -> Duration {
        K_CURSOR_BLINK_HALF_PERIOD
    }

    /// Express interest in interacting with the keyboard.
    pub fn request_keyboard(self: Handle<Self>, app: &mut App) {
        if app.get(self).has_focus {
            self.open_input_connection(app);
        } else {
            app.get_mut(self).next_focus_change_is_internal = true;
            let focus_node = self.widget(app).focus_node;
            focus_node.request_focus(app, None);
        }
    }

    /// The current status of the text selection handles.
    pub fn selection_overlay(
        self: Handle<Self>,
        app: &App,
    ) -> Option<Handle<TextSelectionOverlay>> {
        app.get(self).selection_overlay
    }

    fn web_context_menu_enabled() -> bool {
        K_IS_WEB
    }

    fn create_selection_overlay(self: Handle<Self>, app: &mut App) -> Handle<TextSelectionOverlay> {
        let value = self.value(app);
        let context = self.context(app);
        let debug_required_for = context.widget(app).clone();
        let toolbar_layer_link = self.toolbar_layer_link(app);
        let start_handle_layer_link = self.start_handle_layer_link(app);
        let end_handle_layer_link = self.end_handle_layer_link(app);
        let render_object = self.render_editable(app);
        let selection_delegate = self.as_text_selection_delegate();
        let magnifier_configuration = self.widget(app).magnifier_configuration.clone();
        let clipboard_status = self.clipboard_status(app);
        let drag_start_behavior = self.widget(app).drag_start_behavior;
        let selection_controls = self.widget(app).selection_controls.clone();
        let on_selection_handle_tapped = self.widget(app).on_selection_handle_tapped.clone();
        let context_menu_builder = self.widget(app).context_menu_builder.clone();
        let mut overlay = TextSelectionOverlay::new(
            app,
            value,
            context,
            toolbar_layer_link,
            start_handle_layer_link,
            end_handle_layer_link,
            render_object,
            selection_delegate,
            magnifier_configuration,
        )
        .clipboard_status(app, clipboard_status)
        .debug_required_for(app, debug_required_for)
        .drag_start_behavior(app, drag_start_behavior);
        if let Some(controls) = selection_controls {
            overlay = overlay.selection_controls(app, controls);
        }
        if let Some(on_tapped) = on_selection_handle_tapped {
            overlay =
                overlay.on_selection_handle_tapped(app, Listener::new(move |app| on_tapped(app)));
        }
        if !Self::web_context_menu_enabled()
            && let Some(builder) = context_menu_builder
        {
            let this = self;
            let widget_builder: WidgetBuilder =
                Rc::new(move |app, context| builder(app, context, this));
            overlay = overlay.context_menu_builder(app, widget_builder);
        }
        overlay
    }

    fn update_or_dispose_selection_overlay_if_needed(self: Handle<Self>, app: &mut App) {
        if let Some(overlay) = app.get(self).selection_overlay {
            if app.get(self).has_focus {
                overlay.update(app, self.value(app));
            } else {
                overlay.dispose(app);
                app.get_mut(self).selection_overlay = None;
            }
        }
    }

    fn get_offset_to_reveal_caret(self: Handle<Self>, app: &mut App, rect: Rect) -> RevealedOffset {
        let scroll_controller = self.scroll_controller(app);
        let position = scroll_controller.position(app);
        if !position.allow_implicit_scrolling(app) {
            return RevealedOffset::new(scroll_controller.offset(app), rect);
        }

        let editable_size = self.render_editable(app).size(app);
        let (additional_offset, unit_offset, reveal_rect) = if !self.is_multiline(app) {
            let additional = if rect.width() >= editable_size.width() {
                editable_size.width() / 2.0 - rect.center().dx()
            } else {
                clamp_double(0.0, rect.right - editable_size.width(), rect.left)
            };
            (additional, Offset::new(1.0, 0.0), rect)
        } else {
            let expanded = Rect::from_center(
                rect.center(),
                rect.width(),
                rect.height()
                    .max(self.render_editable(app).preferred_line_height(app)),
            );
            let additional = if expanded.height() >= editable_size.height() {
                editable_size.height() / 2.0 - expanded.center().dy()
            } else {
                clamp_double(0.0, expanded.bottom - editable_size.height(), expanded.top)
            };
            (additional, Offset::new(0.0, 1.0), expanded)
        };

        let target_offset = clamp_double(
            additional_offset + scroll_controller.offset(app),
            position.min_scroll_extent(app),
            position.max_scroll_extent(app),
        );
        let offset_delta = scroll_controller.offset(app) - target_offset;
        RevealedOffset::new(target_offset, reveal_rect.shift(unit_offset * offset_delta))
    }

    fn bring_selection_into_view_after_frame(
        self: Handle<Self>,
        app: &mut App,
        _time_stamp: Duration,
    ) {
        if !self.mounted(app) {
            return;
        }
        self.bring_into_view(app, self.value(app).selection.extent());
    }

    /// Brings the provided [`TextPosition`] into the visible area of the text
    /// input.
    pub fn bring_into_view(self: Handle<Self>, app: &mut App, position: TextPosition) {
        let local_rect = self
            .render_editable(app)
            .get_local_rect_for_caret(app, position);
        let target_offset = self.get_offset_to_reveal_caret(app, local_rect);
        self.scroll_controller(app)
            .jump_to(app, target_offset.offset);
        self.render_editable(app).show_on_screen(
            app,
            None,
            Some(target_offset.rect),
            Duration::ZERO,
            Curves::ease(),
        );
    }

    /// Shows the selection toolbar at the location of the current cursor.
    pub fn show_toolbar(self: Handle<Self>, app: &mut App) -> bool {
        if Self::web_context_menu_enabled() {
            return false;
        }
        let Some(overlay) = app.get(self).selection_overlay else {
            return false;
        };
        if overlay.toolbar_is_visible(app) {
            return false;
        }
        if let Some(status) = app.get(self).live_text_input_status {
            status.update(app);
        }
        self.clipboard_status(app).update(app);
        overlay.show_toolbar(app);
        true
    }

    /// Hides the text selection toolbar.
    pub fn hide_toolbar(self: Handle<Self>, app: &mut App, hide_handles: bool) {
        if hide_handles {
            if let Some(overlay) = app.get(self).selection_overlay {
                overlay.hide(app);
            }
        } else if let Some(overlay) = app.get(self).selection_overlay
            && overlay.toolbar_is_visible(app)
        {
            overlay.hide_toolbar(app);
        }
    }

    /// Toggles the visibility of the toolbar.
    pub fn toggle_toolbar(self: Handle<Self>, app: &mut App, hide_handles: bool) {
        if app.get(self).selection_overlay.is_none() {
            let overlay = self.create_selection_overlay(app);
            app.get_mut(self).selection_overlay = Some(overlay);
        }
        let overlay = app.get(self).selection_overlay.expect("created above");
        if overlay.toolbar_is_visible(app) {
            self.hide_toolbar(app, hide_handles);
        } else {
            let _ = self.show_toolbar(app);
        }
    }

    /// Shows toolbar with spell check suggestions of misspelled words that are
    /// available for click-and-replace.
    pub fn show_spell_check_suggestions_toolbar(self: Handle<Self>, app: &mut App) -> bool {
        if !self.spell_check_enabled(app)
            || Self::web_context_menu_enabled()
            || self.widget(app).read_only
            || app.get(self).selection_overlay.is_none()
            || !self.spell_check_results_received(app)
            || self
                .find_suggestion_span_at_cursor_index(app, self.value(app).selection.extent_offset)
                .is_none()
        {
            return false;
        }
        debug_assert!(
            app.get(self)
                .spell_check_configuration
                .spell_check_suggestions_toolbar_builder
                .is_some(),
            "spellCheckSuggestionsToolbarBuilder must be defined in SpellCheckConfiguration to show a toolbar with spell check suggestions"
        );
        let Some(builder) = app
            .get(self)
            .spell_check_configuration
            .spell_check_suggestions_toolbar_builder
            .clone()
        else {
            return false;
        };
        let this = self;
        let overlay = app.get(self).selection_overlay.expect("checked above");
        overlay.show_spell_check_suggestions_toolbar(
            app,
            Rc::new(move |app, context| builder(app, context, this)),
        );
        true
    }

    /// Shows the magnifier at the position given by `position_to_show`,
    /// if no magnifier exists.
    pub fn show_magnifier(self: Handle<Self>, app: &mut App, position_to_show: Offset) {
        let Some(overlay) = app.get(self).selection_overlay else {
            return;
        };
        if overlay.magnifier_exists(app) {
            overlay.update_magnifier(app, position_to_show);
        } else {
            overlay.show_magnifier(app, position_to_show);
        }
    }

    /// Hides the magnifier.
    pub fn hide_magnifier(self: Handle<Self>, app: &mut App) {
        if let Some(overlay) = app.get(self).selection_overlay {
            overlay.hide_magnifier(app);
        }
    }

    /// Finds specified [`SuggestionSpan`] that matches the provided index using
    /// binary search.
    pub fn find_suggestion_span_at_cursor_index(
        self: Handle<Self>,
        app: &App,
        cursor_index: i32,
    ) -> Option<SuggestionSpan> {
        if !self.spell_check_results_received(app) {
            return None;
        }
        let results = app.get(self).spell_check_results.as_ref()?;
        if results.suggestion_spans.last()?.range.end < cursor_index {
            return None;
        }
        let suggestion_spans = &results.suggestion_spans;
        let mut left_index: i32 = 0;
        let mut right_index = suggestion_spans.len() as i32 - 1;
        while left_index <= right_index {
            let mid_index = (left_index + right_index).div_euclid(2) as usize;
            let current_span_start = suggestion_spans[mid_index].range.start;
            let current_span_end = suggestion_spans[mid_index].range.end;
            if cursor_index <= current_span_end && cursor_index >= current_span_start {
                return Some(suggestion_spans[mid_index].clone());
            } else if cursor_index <= current_span_start {
                right_index = mid_index as i32 - 1;
            } else {
                left_index = mid_index as i32 + 1;
            }
        }
        None
    }

    fn post_frame_rebuild_toolbar(self: Handle<Self>, app: &mut App, _time_stamp: Duration) {
        if self.mounted(app)
            && let Some(overlay) = app.get(self).selection_overlay
            && overlay.toolbar_is_visible(app)
        {
            overlay.show_toolbar(app);
        }
    }

    fn post_frame_update_for_scroll(self: Handle<Self>, app: &mut App, _time_stamp: Duration) {
        if let Some(overlay) = app.get(self).selection_overlay {
            overlay.update_for_scroll(app);
        }
    }

    /// Begins a new batch edit.
    pub fn begin_batch_edit(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).batch_edit_depth += 1;
    }

    /// Ends the current batch edit started by the last call to
    /// [`begin_batch_edit`](Self::begin_batch_edit).
    pub fn end_batch_edit(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).batch_edit_depth -= 1;
        debug_assert!(
            app.get(self).batch_edit_depth >= 0,
            "Unbalanced call to endBatchEdit: beginBatchEdit must be called first."
        );
        self.update_remote_editing_value_if_needed(app);
    }

    /// Builds [`TextSpan`] from current editing value.
    pub fn build_text_span(self: Handle<Self>, app: &mut App, context: BuildContext) -> TextSpan {
        let widget = self.widget(app);
        let obscure_text = widget.obscure_text;
        let obscuring_character = widget.obscuring_character.clone();
        let read_only = widget.read_only;
        let controller = widget.controller;
        let style = app.get(self).style.clone();
        let value = self.value(app);
        if obscure_text {
            let n = value.text.chars().count();
            let text = obscuring_character.repeat(n);
            return TextSpan::new().style(style).text(text);
        }
        let with_composing = !read_only && app.get(self).has_focus;
        if self.spell_check_results_received(app) {
            debug_assert!(
                !value.composing.is_valid() || !with_composing || value.is_composing_range_valid()
            );
            let composing_region_out_of_range =
                !value.is_composing_range_valid() || !with_composing;
            let misspelled = app
                .get(self)
                .spell_check_configuration
                .misspelled_text_style
                .clone()
                .expect("spell check results require misspelledTextStyle");
            let results = app
                .get(self)
                .spell_check_results
                .clone()
                .expect("spellCheckResultsReceived");
            return build_text_span_with_spell_check_suggestions(
                app,
                &value,
                composing_region_out_of_range,
                Some(&style),
                &misspelled,
                &results,
            );
        }
        controller.build_text_span(app, context, Some(style), with_composing)
    }

    fn value(self: Handle<Self>, app: &App) -> TextEditingValue {
        self.widget(app).controller.value(app).clone()
    }

    fn set_value(self: Handle<Self>, app: &mut App, value: TextEditingValue) {
        self.widget(app).controller.set_value(app, value);
    }

    fn is_multiline(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).max_lines != Some(1)
    }

    fn text_direction(self: Handle<Self>, app: &mut App, context: BuildContext) -> TextDirection {
        self.widget(app)
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context))
    }

    fn has_input_connection(self: Handle<Self>, app: &mut App) -> bool {
        app.get(self)
            .text_input_connection
            .is_some_and(|connection| connection.attached(app))
    }

    fn should_create_input_connection(self: Handle<Self>, app: &App) -> bool {
        K_IS_WEB
            || app.platform().target_platform() == TargetPlatform::MacOS
            || !self.widget(app).read_only
    }

    fn scroll_controller(self: Handle<Self>, app: &mut App) -> Handle<ScrollController> {
        if let Some(controller) = self.widget(app).scroll_controller {
            return controller;
        }
        if let Some(controller) = app.get(self).internal_scroll_controller {
            return controller;
        }
        let controller = ScrollController::default(app);
        app.get_mut(self).internal_scroll_controller = Some(controller);
        controller
    }

    fn cursor_visibility_notifier(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<bool>> {
        app.get(self)
            .cursor_visibility_notifier
            .expect("created in initState")
    }

    fn start_handle_layer_link(self: Handle<Self>, app: &App) -> Handle<LayerLink> {
        app.get(self)
            .start_handle_layer_link
            .expect("created in initState")
    }

    fn end_handle_layer_link(self: Handle<Self>, app: &App) -> Handle<LayerLink> {
        app.get(self)
            .end_handle_layer_link
            .expect("created in initState")
    }

    fn toolbar_layer_link(self: Handle<Self>, app: &App) -> Handle<LayerLink> {
        app.get(self)
            .toolbar_layer_link
            .expect("created in initState")
    }

    fn cursor_blink_opacity_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> Handle<AnimationController> {
        if let Some(controller) = app.get(self).backing_cursor_blink_opacity_controller {
            return controller;
        }
        let controller = AnimationController::create(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::on_cursor_color_tick),
        );
        app.get_mut(self).backing_cursor_blink_opacity_controller = Some(controller);
        controller
    }

    fn observer(self: Handle<Self>, app: &mut App) -> WidgetsBindingObserverRef {
        if let Some(observer) = app.get(self).binding_observer.clone() {
            return observer;
        }
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).binding_observer = Some(observer.clone());
        observer
    }

    fn effective_autofill_client(self: Handle<Self>, app: &App) -> AnyAutofillClient {
        self.widget(app)
            .autofill_client
            .unwrap_or_else(|| self.as_autofill_client())
    }

    fn cursor_color(self: Handle<Self>, app: &mut App) -> Color {
        let cursor_color = self.widget(app).cursor_color;
        let opacity = cursor_color
            .a
            .min(self.cursor_blink_opacity_controller(app).value(app));
        cursor_color.with_alpha((255.0 * opacity).round() as i32)
    }

    fn show_blinking_cursor(self: Handle<Self>, app: &mut App) -> bool {
        app.get(self).has_focus
            && self.value(app).selection.is_collapsed()
            && self.widget(app).show_cursor
            && app.get(self).tickers_enabled
    }

    fn init_process_text_actions(self: Handle<Self>, app: &mut App) {
        let actions = app.get(self).process_text_service.query_text_actions(app);
        app.get_mut(self).process_text_actions.clear();
        app.get_mut(self).process_text_actions.extend(actions);
    }

    fn on_resume(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).just_resumed = true;
        let manager = FocusManager::instance(app);
        manager.remove_listener(
            app,
            &Listener::handle_method(self, Self::reset_just_resumed),
        );
        manager.add_listener(app, Listener::handle_method(self, Self::reset_just_resumed));
    }

    fn reset_just_resumed(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).just_resumed = false;
        FocusManager::instance(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::reset_just_resumed),
        );
    }

    fn on_tap_outside(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: PointerDownEvent,
    ) {
        app.get_mut(self).had_focus_on_tap_down = true;
        if let Some(on_tap_outside) = self.widget(app).on_tap_outside.clone() {
            on_tap_outside(app, event);
        } else {
            self.default_on_tap_outside(app, context, event);
        }
    }

    fn on_tap_up_outside(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: PointerUpEvent,
    ) {
        if !app.get(self).had_focus_on_tap_down {
            return;
        }
        app.get_mut(self).had_focus_on_tap_down = false;
        if let Some(on_tap_up_outside) = self.widget(app).on_tap_up_outside.clone() {
            on_tap_up_outside(app, event);
        } else {
            self.default_on_tap_up_outside(app, context, event);
        }
    }

    fn default_on_tap_outside(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: PointerDownEvent,
    ) {
        let focus_node = self.widget(app).focus_node.as_node();
        Actions::invoke(
            app,
            context,
            &EditableTextTapOutsideIntent::new(focus_node, event),
        );
    }

    fn default_on_tap_up_outside(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: PointerUpEvent,
    ) {
        let focus_node = self.widget(app).focus_node.as_node();
        Actions::invoke(
            app,
            context,
            &EditableTextTapUpOutsideIntent::new(focus_node, event),
        );
    }

    fn invoke_tap_outside(app: &mut App, intent: &EditableTextTapOutsideIntent) {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {
                match intent.pointer_down_event.kind {
                    PointerDeviceKind::Touch => {
                        if K_IS_WEB {
                            intent.focus_node.unfocus(app, UnfocusDisposition::Scope);
                        }
                    }
                    PointerDeviceKind::Mouse
                    | PointerDeviceKind::Stylus
                    | PointerDeviceKind::InvertedStylus
                    | PointerDeviceKind::Unknown => {
                        intent.focus_node.unfocus(app, UnfocusDisposition::Scope);
                    }
                    PointerDeviceKind::Trackpad => {
                        unimplemented!("Unexpected pointer down event for trackpad");
                    }
                }
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                intent.focus_node.unfocus(app, UnfocusDisposition::Scope);
            }
        }
    }

    // --------------------------- Text Editing Actions ---------------------------

    fn character_boundary(self: Handle<Self>, app: &mut App) -> Box<dyn TextBoundary> {
        let text = self.value(app).text;
        if self.widget(app).obscure_text {
            Box::new(CodePointBoundary::new(text))
        } else {
            Box::new(CharacterBoundary::new(text))
        }
    }

    fn next_word_boundary(self: Handle<Self>, app: &mut App) -> Box<dyn TextBoundary> {
        if self.widget(app).obscure_text {
            self.document_boundary(app)
        } else {
            self.render_editable(app)
                .word_boundaries(app)
                .move_by_word_boundary()
        }
    }

    fn linebreak(self: Handle<Self>, app: &mut App) -> Box<dyn TextBoundary> {
        if self.widget(app).obscure_text {
            self.document_boundary(app)
        } else {
            Box::new(LineBoundary::new(self.render_editable(app)))
        }
    }

    fn paragraph_boundary(self: Handle<Self>, app: &mut App) -> Box<dyn TextBoundary> {
        Box::new(ParagraphBoundary::new(&self.value(app).text))
    }

    fn document_boundary(self: Handle<Self>, app: &mut App) -> Box<dyn TextBoundary> {
        Box::new(DocumentBoundary::new(self.value(app).text))
    }

    /// Returns the closest boundary location to `extent` but not including `extent`
    /// itself (unless already at the start/end of the text), in the direction
    /// specified by `forward`.
    fn move_beyond_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        extent: TextPosition,
        forward: bool,
        text_boundary: &dyn TextBoundary,
    ) -> TextPosition {
        debug_assert!(extent.offset >= 0);
        let text_length = utf16_len(&self.value(app).text);
        let new_offset = if forward {
            text_boundary
                .get_trailing_text_boundary_at(app, extent.offset)
                .unwrap_or(text_length)
        } else {
            // if x is a boundary defined by `text_boundary`, most text boundaries (except
            // LineBoundary) guarantee `x == text_boundary.get_leading_text_boundary_at(x)`.
            // Use x - 1 here to make sure we don't get stuck at the fixed point x.
            text_boundary
                .get_leading_text_boundary_at(app, extent.offset - 1)
                .unwrap_or(0)
        };
        TextPosition::new(new_offset)
    }

    /// Returns the closest boundary location to `extent`, including `extent`
    /// itself, in the direction specified by `forward`.
    ///
    /// This method returns a fixed point of itself: applying it again on the returned
    /// [`TextPosition`] gives the same [`TextPosition`]. It's used exclusively for handling
    /// line boundaries, since performing "move to line start" more than once usually doesn't
    /// move you to the previous line.
    fn move_to_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        extent: TextPosition,
        forward: bool,
        text_boundary: &dyn TextBoundary,
    ) -> TextPosition {
        debug_assert!(extent.offset >= 0);
        let caret_offset = match extent.affinity {
            TextAffinity::Upstream => {
                if extent.offset < 1 && !forward {
                    debug_assert!(extent.offset == 0);
                    return TextPosition::new(0);
                }
                // When the text affinity is upstream, the caret is associated with the
                // grapheme before the offset.
                (extent.offset - 1).max(0)
            }
            TextAffinity::Downstream => extent.offset,
        };
        // The line boundary range does not include some control characters (most notably,
        // Line Feed), in which case there's `x ∉ get_text_boundary_at(x)`. In case
        // `caret_offset` points to one such control character, we define that these control
        // characters themselves are still part of the previous line, but also exclude them
        // from the line boundary range since they're non-printing. IOW, no additional
        // processing needed since the LineBoundary class does exactly that.
        let text_length = utf16_len(&self.value(app).text);
        if forward {
            TextPosition::with_affinity(
                text_boundary
                    .get_trailing_text_boundary_at(app, caret_offset)
                    .unwrap_or(text_length),
                TextAffinity::Upstream,
            )
        } else {
            TextPosition::new(
                text_boundary
                    .get_leading_text_boundary_at(app, caret_offset)
                    .unwrap_or(0),
            )
        }
    }

    /// Transpose the characters immediately before and after the current
    /// collapsed selection.
    ///
    /// When the cursor is at the end of the text, transposes the last two
    /// characters, if they exist.
    ///
    /// When the cursor is at the start of the text, does nothing.
    fn transpose_characters(self: Handle<Self>, app: &mut App) {
        let value = self.value(app);
        let text = value.text.clone();
        let selection = value.selection;
        let text_length = utf16_len(&text);
        let boundary = CharacterBoundary::new(text.clone());
        // Dart's `_value.text.characters.length <= 1`.
        let single_character = boundary
            .get_trailing_text_boundary_at(app, 0)
            .is_none_or(|end| end >= text_length);
        if single_character || !selection.is_collapsed() || selection.base_offset == 0 {
            return;
        }

        // Dart walks a `CharacterRange` back over two graphemes, or back one and forward
        // one; both leave a range covering exactly the two graphemes to swap.
        let at_end = selection.base_offset == text_length;
        let (start, middle, end) = if at_end {
            let middle = boundary
                .get_leading_text_boundary_at(app, text_length - 1)
                .unwrap_or(0);
            let start = boundary
                .get_leading_text_boundary_at(app, middle - 1)
                .unwrap_or(0);
            (start, middle, text_length)
        } else {
            let start = boundary
                .get_leading_text_boundary_at(app, selection.base_offset - 1)
                .unwrap_or(0);
            let end = boundary
                .get_trailing_text_boundary_at(app, selection.base_offset)
                .unwrap_or(text_length);
            (start, selection.base_offset, end)
        };
        let transposing = TextRange::new(start, end);
        let first = TextRange::new(start, middle).text_inside(&text);
        let second = TextRange::new(middle, end).text_inside(&text);

        let value = TextEditingValue::new()
            .text(format!(
                "{}{second}{first}{}",
                transposing.text_before(&text),
                transposing.text_after(&text)
            ))
            .selection(TextSelection::collapsed(end, TextAffinity::Downstream));
        self.user_update_text_editing_value(app, value, SelectionChangedCause::Keyboard);
    }

    fn replace_text(self: Handle<Self>, app: &mut App, intent: &ReplaceTextIntent) {
        let old_value = self.value(app);
        let new_value = intent
            .current_text_editing_value
            .replaced(intent.replacement_range, &intent.replacement_text);
        self.user_update_text_editing_value(app, new_value.clone(), intent.cause);

        // If there's no change in text and selection (e.g. when selecting and pasting
        // identical text), the widget won't be rebuilt on value update. Handle this by
        // calling did_change_text_editing_value() so caret and scroll updates can happen.
        if new_value == old_value {
            self.did_change_text_editing_value(app);
        }
    }

    /// Scrolls either to the beginning or end of the document depending on the
    /// intent's `forward` parameter.
    fn scroll_to_document_boundary(
        self: Handle<Self>,
        app: &mut App,
        intent: &ScrollToDocumentBoundaryIntent,
    ) {
        if intent.forward {
            let offset = utf16_len(&self.value(app).text);
            self.bring_into_view(app, TextPosition::new(offset));
        } else {
            self.bring_into_view(app, TextPosition::new(0));
        }
    }

    /// Handles [`ScrollIntent`] by scrolling the `Scrollable` inside of [`EditableText`].
    fn scroll(self: Handle<Self>, app: &mut App, intent: &ScrollIntent) {
        if intent.r#type != ScrollIncrementType::Page {
            return;
        }

        let controller = self.scroll_controller(app);
        let position = controller.position(app);
        if self.widget(app).max_lines == Some(1) {
            let max = position.max_scroll_extent(app);
            controller.jump_to(app, max);
            return;
        }

        // If the field isn't scrollable, do nothing. For example, when the lines of text is
        // less than max_lines, the field has nothing to scroll.
        if position.max_scroll_extent(app) == 0.0 && position.min_scroll_extent(app) == 0.0 {
            return;
        }

        let key = Rc::clone(&app.get(self).scrollable_key);
        let state = key
            .current_state::<ScrollableState>(app)
            .expect("EditableText mounts a Scrollable");
        let increment = ScrollAction::get_directional_increment(app, state, intent);
        let destination = (position.pixels(app) + increment).clamp(
            position.min_scroll_extent(app),
            position.max_scroll_extent(app),
        );
        if destination == position.pixels(app) {
            return;
        }
        controller.jump_to(app, destination);
    }

    fn update_selection(self: Handle<Self>, app: &mut App, intent: &UpdateSelectionIntent) {
        debug_assert!(
            intent.new_selection.start() <= utf16_len(&intent.current_text_editing_value.text),
            "invalid selection: it must not exceed the current text length"
        );
        debug_assert!(
            intent.new_selection.end() <= utf16_len(&intent.current_text_editing_value.text),
            "invalid selection: it must not exceed the current text length"
        );

        self.bring_into_view(app, intent.new_selection.extent());
        let value = intent
            .current_text_editing_value
            .clone()
            .copy_with()
            .selection(intent.new_selection);
        self.user_update_text_editing_value(app, value, intent.cause);
    }

    /// The value the editable was last laid out with, which the vertical caret run walks.
    ///
    /// Dart's `_textEditingValueforTextLayoutMetrics`.
    fn text_editing_value_for_text_layout_metrics(
        self: Handle<Self>,
        app: &mut App,
    ) -> TextEditingValue {
        let key = Rc::clone(&app.get(self).editable_key);
        let widget = key.current_widget(app).expect("Editable must be mounted");
        widget
            .as_any()
            .downcast_ref::<Editable>()
            .expect("Editable must be mounted")
            .value
            .clone()
    }

    fn hide_toolbar_if_visible(self: Handle<Self>, app: &mut App) -> Option<Rc<dyn Any>> {
        if app
            .get(self)
            .selection_overlay
            .is_some_and(|overlay| overlay.toolbar_is_visible(app))
        {
            self.hide_toolbar(app, false);
            return None;
        }
        let context = self.context(app);
        Actions::invoke(app, context, &DismissIntent)
    }

    /// Ends the vertical caret run when the selection moved somewhere the run did not put it.
    ///
    /// Dart keeps the run on the single shared vertical action; both intents map to that one
    /// instance, so the run lives on the state here to stay shared.
    fn stop_current_vertical_run_if_selection_changes(self: Handle<Self>, app: &mut App) {
        let Some(run_selection) = app.get(self).run_selection else {
            debug_assert!(app.get(self).vertical_movement_run.is_none());
            return;
        };
        app.get_mut(self).run_selection = Some(self.value(app).selection);
        let current_selection = self.widget(app).controller.selection(app);
        let continue_current_run = current_selection.is_valid()
            && current_selection.is_collapsed()
            && current_selection.base_offset == run_selection.base_offset
            && current_selection.extent_offset == run_selection.extent_offset;
        if !continue_current_run {
            app.get_mut(self).vertical_movement_run = None;
            app.get_mut(self).run_selection = None;
        }
    }

    /// Dart's `_actions`, built once and kept for the life of the state.
    fn ensure_actions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> HashMap<TypeId, AnyAction> {
        if let Some(actions) = app.get(self).actions.clone() {
            return actions;
        }
        let mut actions: HashMap<TypeId, AnyAction> = HashMap::new();

        let do_nothing = DoNothingAction::new(app);
        do_nothing.set_consumes_key(app, false);
        actions.insert(
            TypeId::of::<DoNothingAndStopPropagationTextIntent>(),
            Action::as_action(do_nothing),
        );

        let replace_text = CallbackAction::<ReplaceTextIntent>::new(
            app,
            Rc::new(move |app, intent| {
                self.replace_text(app, intent);
                None
            }),
        );
        actions.insert(
            TypeId::of::<ReplaceTextIntent>(),
            Action::as_action(replace_text),
        );

        let update_selection = CallbackAction::<UpdateSelectionIntent>::new(
            app,
            Rc::new(move |app, intent| {
                self.update_selection(app, intent);
                None
            }),
        );
        actions.insert(
            TypeId::of::<UpdateSelectionIntent>(),
            Action::as_action(update_selection),
        );

        let directional_focus = DirectionalFocusAction::for_text_field(app);
        actions.insert(
            TypeId::of::<DirectionalFocusIntent>(),
            Action::as_action(directional_focus),
        );

        let dismiss = CallbackAction::<DismissIntent>::new(
            app,
            Rc::new(move |app, _intent| self.hide_toolbar_if_visible(app)),
        );
        actions.insert(TypeId::of::<DismissIntent>(), Action::as_action(dismiss));

        // Delete
        let delete_character = DeleteTextAction::<DeleteCharacterIntent>::new(
            app,
            self,
            EditableTextState::character_boundary,
            EditableTextState::move_beyond_text_boundary,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<DeleteCharacterIntent>(),
            Action::as_action(delete_character),
        );
        let delete_word = DeleteTextAction::<DeleteToNextWordBoundaryIntent>::new(
            app,
            self,
            EditableTextState::next_word_boundary,
            EditableTextState::move_beyond_text_boundary,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<DeleteToNextWordBoundaryIntent>(),
            Action::as_action(delete_word),
        );
        let delete_to_line_break = DeleteTextAction::<DeleteToLineBreakIntent>::new(
            app,
            self,
            EditableTextState::linebreak,
            EditableTextState::move_to_text_boundary,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<DeleteToLineBreakIntent>(),
            Action::as_action(delete_to_line_break),
        );

        // Extend/Move Selection
        let by_character = UpdateTextSelectionAction::<ExtendSelectionByCharacterIntent>::new(
            app,
            self,
            EditableTextState::character_boundary,
            EditableTextState::move_beyond_text_boundary,
            false,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionByCharacterIntent>(),
            Action::as_action(by_character),
        );
        let by_word = UpdateTextSelectionAction::<ExtendSelectionToNextWordBoundaryIntent>::new(
            app,
            self,
            EditableTextState::next_word_boundary,
            EditableTextState::move_beyond_text_boundary,
            true,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToNextWordBoundaryIntent>(),
            Action::as_action(by_word),
        );
        let by_paragraph =
            UpdateTextSelectionAction::<ExtendSelectionToNextParagraphBoundaryIntent>::new(
                app,
                self,
                EditableTextState::paragraph_boundary,
                EditableTextState::move_beyond_text_boundary,
                true,
            );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToNextParagraphBoundaryIntent>(),
            Action::as_action(by_paragraph),
        );
        let to_line_break = UpdateTextSelectionAction::<ExtendSelectionToLineBreakIntent>::new(
            app,
            self,
            EditableTextState::linebreak,
            EditableTextState::move_to_text_boundary,
            true,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToLineBreakIntent>(),
            Action::as_action(to_line_break),
        );
        let by_line = UpdateTextSelectionVerticallyAction::<
            ExtendSelectionVerticallyToAdjacentLineIntent,
        >::new(app, self, false);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionVerticallyToAdjacentLineIntent>(),
            Action::as_action(by_line),
        );
        let by_page = UpdateTextSelectionVerticallyAction::<
            ExtendSelectionVerticallyToAdjacentPageIntent,
        >::new(app, self, true);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionVerticallyToAdjacentPageIntent>(),
            Action::as_action(by_page),
        );
        let by_paragraph_or_caret = UpdateTextSelectionAction::<
            ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent,
        >::new(
            app,
            self,
            EditableTextState::paragraph_boundary,
            EditableTextState::move_beyond_text_boundary,
            true,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent>(),
            Action::as_action(by_paragraph_or_caret),
        );
        let to_document = UpdateTextSelectionAction::<ExtendSelectionToDocumentBoundaryIntent>::new(
            app,
            self,
            EditableTextState::document_boundary,
            EditableTextState::move_beyond_text_boundary,
            true,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToDocumentBoundaryIntent>(),
            Action::as_action(to_document),
        );
        let by_word_or_caret = UpdateTextSelectionAction::<
            ExtendSelectionToNextWordBoundaryOrCaretLocationIntent,
        >::new(
            app,
            self,
            EditableTextState::next_word_boundary,
            EditableTextState::move_beyond_text_boundary,
            true,
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExtendSelectionToNextWordBoundaryOrCaretLocationIntent>(),
            Action::as_action(by_word_or_caret),
        );

        let scroll_to_boundary =
            WebComposingDisablingCallbackAction::<ScrollToDocumentBoundaryIntent>::new(
                app,
                self,
                Rc::new(move |app, intent| {
                    self.scroll_to_document_boundary(app, intent);
                    None
                }),
            );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ScrollToDocumentBoundaryIntent>(),
            Action::as_action(scroll_to_boundary),
        );

        let scroll = CallbackAction::<ScrollIntent>::new(
            app,
            Rc::new(move |app, intent| {
                self.scroll(app, intent);
                None
            }),
        );
        actions.insert(TypeId::of::<ScrollIntent>(), Action::as_action(scroll));

        // Expand Selection
        let expand_to_line_break =
            UpdateTextSelectionAction::<ExpandSelectionToLineBreakIntent>::new(
                app,
                self,
                EditableTextState::linebreak,
                EditableTextState::move_to_text_boundary,
                true,
            )
            .expanding(app, false);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExpandSelectionToLineBreakIntent>(),
            Action::as_action(expand_to_line_break),
        );
        let expand_to_document =
            UpdateTextSelectionAction::<ExpandSelectionToDocumentBoundaryIntent>::new(
                app,
                self,
                EditableTextState::document_boundary,
                EditableTextState::move_to_text_boundary,
                true,
            )
            .expanding(app, true);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<ExpandSelectionToDocumentBoundaryIntent>(),
            Action::as_action(expand_to_document),
        );

        // Copy Paste
        let select_all = SelectAllAction::new(app, self);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<SelectAllTextIntent>(),
            Action::as_action(select_all),
        );
        let copy = CopySelectionAction::new(app, self);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<CopySelectionTextIntent>(),
            Action::as_action(copy),
        );
        let paste = PasteSelectionAction::new(app, self);
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<PasteTextIntent>(),
            Action::as_action(paste),
        );

        let transpose = CallbackAction::<TransposeCharactersIntent>::new(
            app,
            Rc::new(move |app, _intent| {
                self.transpose_characters(app);
                None
            }),
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<TransposeCharactersIntent>(),
            Action::as_action(transpose),
        );

        let tap_outside = CallbackAction::<EditableTextTapOutsideIntent>::new(
            app,
            Rc::new(|app, intent| {
                Self::invoke_tap_outside(app, intent);
                None
            }),
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<EditableTextTapOutsideIntent>(),
            Action::as_action(tap_outside),
        );
        let tap_up_outside = CallbackAction::<EditableTextTapUpOutsideIntent>::new(
            app,
            Rc::new(|_app, _intent| None),
        );
        insert_overridable(
            app,
            &mut actions,
            context,
            TypeId::of::<EditableTextTapUpOutsideIntent>(),
            Action::as_action(tap_up_outside),
        );

        app.get_mut(self).actions = Some(actions.clone());
        actions
    }

    fn on_changed_clipboard_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn on_changed_live_text_input_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn did_change_text_editing_value(self: Handle<Self>, app: &mut App) {
        if app.get(self).has_focus && !self.value(app).selection.is_valid() {
            let controller = self.widget(app).controller;
            controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::did_change_text_editing_value),
            );
            if let Some(selection) = self.adjusted_selection_when_focused(app) {
                controller.set_selection(app, selection);
            }
            controller.add_listener(
                app,
                Listener::handle_method(self, Self::did_change_text_editing_value),
            );
        }
        self.update_remote_editing_value_if_needed(app);
        self.start_or_stop_cursor_timer_if_needed(app);
        self.update_or_dispose_selection_overlay_if_needed(app);
        // Dart also notes here that RenderEditable should learn about
        // ValueNotifier<TextEditingValue> so this set_state can go.
        self.set_state(app, |_| {});
        self.stop_current_vertical_run_if_selection_changes(app);
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        let has_focus = self.widget(app).focus_node.has_focus(app);
        app.get_mut(self).has_focus = has_focus;
        self.open_or_close_input_connection_if_needed(app);
        self.start_or_stop_cursor_timer_if_needed(app);
        self.update_or_dispose_selection_overlay_if_needed(app);
        if has_focus {
            let observer = self.observer(app);
            if !app.get(self).registered_as_observer {
                WidgetsBinding::instance(app).add_observer(app, observer);
                app.get_mut(self).registered_as_observer = true;
            }
            let updated = self.adjusted_selection_when_focused(app);
            if let Some(selection) = updated {
                self.handle_selection_changed(app, selection, None);
            }
        } else if app.get(self).registered_as_observer {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).remove_observer(app, &observer);
            app.get_mut(self).registered_as_observer = false;
            self.set_state(app, |state| {
                state.current_prompt_rect_range = None;
            });
        }
        AutomaticKeepAliveClientMixin::update_keep_alive(self, app);
    }

    fn adjusted_selection_when_focused(self: Handle<Self>, app: &mut App) -> Option<TextSelection> {
        let widget = self.widget(app);
        let should_select_all = widget.select_all_on_focus_or_default(app)
            && widget.enable_interactive_selection
            && !self.is_multiline(app)
            && !app.get(self).next_focus_change_is_internal
            && !app.get(self).just_resumed;
        app.get_mut(self).just_resumed = false;
        app.get_mut(self).next_focus_change_is_internal = false;
        let value = self.value(app);
        if should_select_all {
            Some(TextSelection::new(0, utf16_len(&value.text)))
        } else if !value.selection.is_valid() {
            Some(TextSelection::collapsed(
                utf16_len(&value.text),
                TextAffinity::Downstream,
            ))
        } else {
            None
        }
    }

    fn update_remote_editing_value_if_needed(self: Handle<Self>, app: &mut App) {
        if app.get(self).batch_edit_depth > 0 || !self.has_input_connection(app) {
            return;
        }
        let local_value = self.value(app);
        if app.get(self).last_known_remote_text_editing_value.as_ref() == Some(&local_value) {
            return;
        }
        let connection = app.get(self).text_input_connection.expect("attached");
        connection.set_editing_state(app, local_value.clone());
        app.get_mut(self).last_known_remote_text_editing_value = Some(local_value);
    }

    fn get_text_input_style(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> TextInputStyle {
        let style = app.get(self).style.clone();
        let text_direction = self.text_direction(app, context);
        let text_align = self.widget(app).text_align;
        let letter_spacing =
            MediaQuery::maybe_letter_spacing_override_of(app, context).or(style.letter_spacing);
        let word_spacing =
            MediaQuery::maybe_word_spacing_override_of(app, context).or(style.word_spacing);
        let mut input_style = TextInputStyle::new(text_direction, text_align);
        input_style.font_family = style.font_family.clone();
        input_style.font_size = style.font_size;
        input_style.font_weight = style.font_weight;
        input_style.letter_spacing = letter_spacing;
        input_style.word_spacing = word_spacing;
        input_style
    }

    fn open_input_connection(self: Handle<Self>, app: &mut App) {
        if !self.should_create_input_connection(app) {
            return;
        }
        if !self.has_input_connection(app) {
            let local_value = self.value(app);
            let config = self
                .effective_autofill_client(app)
                .text_input_configuration(app);
            let connection = TextInput::attach(app, self.as_text_input_client(), config);
            app.get_mut(self).text_input_connection = Some(connection);
            self.update_size_and_transform(app);
            self.schedule_periodic_post_frame_callbacks(app, Duration::ZERO);
            if let Some(context) = self.mounted(app).then(|| self.context(app)) {
                let style = self.get_text_input_style(app, context);
                connection.update_style(app, style);
            }
            connection.set_editing_state(app, local_value.clone());
            connection.show(app);
            app.get_mut(self).last_known_remote_text_editing_value = Some(local_value);
        } else if let Some(connection) = app.get(self).text_input_connection {
            connection.show(app);
        }
    }

    fn close_input_connection_if_needed(self: Handle<Self>, app: &mut App) {
        if self.has_input_connection(app) {
            if let Some(connection) = app.get(self).text_input_connection {
                connection.close(app);
            }
            app.get_mut(self).text_input_connection = None;
            app.get_mut(self).last_known_remote_text_editing_value = None;
        }
    }

    fn open_or_close_input_connection_if_needed(self: Handle<Self>, app: &mut App) {
        let has_focus = app.get(self).has_focus;
        let focus_node = self.widget(app).focus_node;
        if has_focus && focus_node.consume_keyboard_token(app) {
            self.open_input_connection(app);
        } else if !has_focus {
            self.close_input_connection_if_needed(app);
            let controller = self.widget(app).controller;
            controller.clear_composing(app);
        }
    }

    fn check_needs_adjust_affinity(
        self: Handle<Self>,
        app: &App,
        value: &TextEditingValue,
    ) -> bool {
        let current = self.value(app);
        value.text == current.text
            && value.selection.is_collapsed() == current.selection.is_collapsed()
            && value.selection.start() == current.selection.start()
            && value.selection.affinity != current.selection.affinity
    }

    fn format_and_set_value(
        self: Handle<Self>,
        app: &mut App,
        mut value: TextEditingValue,
        cause: Option<SelectionChangedCause>,
        user_interaction: bool,
    ) {
        let old_value = self.value(app);
        let text_changed = old_value.text != value.text;
        let text_committed = !old_value.composing.is_collapsed() && value.composing.is_collapsed();
        let selection_changed = old_value.selection != value.selection;
        if (text_changed || text_committed)
            && let Some(formatters) = self.widget(app).input_formatters.as_ref()
        {
            let current = self.value(app);
            for formatter in formatters {
                value = formatter.format_edit_update(&current, &value);
            }
        }
        let old_selection = self.value(app).selection;
        self.begin_batch_edit(app);
        self.set_value(app, value.clone());
        if selection_changed
            || (user_interaction
                && matches!(
                    cause,
                    Some(SelectionChangedCause::LongPress | SelectionChangedCause::Keyboard)
                ))
        {
            self.handle_selection_changed(app, value.selection, cause);
            let _ = old_selection;
        }
        let current_text = self.value(app).text.clone();
        if old_value.text != current_text
            && let Some(on_changed) = self.widget(app).on_changed.clone()
        {
            on_changed(app, current_text);
        }
        self.end_batch_edit(app);
    }

    fn handle_selection_changed(
        self: Handle<Self>,
        app: &mut App,
        selection: TextSelection,
        cause: Option<SelectionChangedCause>,
    ) {
        let text = self.widget(app).controller.value(app).text.clone();
        if utf16_len(&text) < selection.end() || utf16_len(&text) < selection.start() {
            return;
        }
        self.widget(app).controller.set_selection(app, selection);
        match cause {
            None
            | Some(SelectionChangedCause::DoubleTap)
            | Some(SelectionChangedCause::Drag)
            | Some(SelectionChangedCause::ForcePress)
            | Some(SelectionChangedCause::LongPress)
            | Some(SelectionChangedCause::StylusHandwriting)
            | Some(SelectionChangedCause::Tap)
            | Some(SelectionChangedCause::Toolbar) => {
                self.request_keyboard(app);
            }
            Some(SelectionChangedCause::Keyboard) => {}
        }
        if self.widget(app).selection_controls.is_none()
            && self.widget(app).context_menu_builder.is_none()
        {
            if let Some(overlay) = app.get(self).selection_overlay {
                overlay.dispose(app);
            }
            app.get_mut(self).selection_overlay = None;
        } else {
            if app.get(self).selection_overlay.is_none() {
                let overlay = self.create_selection_overlay(app);
                app.get_mut(self).selection_overlay = Some(overlay);
            } else if let Some(overlay) = app.get(self).selection_overlay {
                overlay.update(app, self.value(app));
            }
            if let Some(overlay) = app.get(self).selection_overlay {
                overlay.set_handles_visible(app, self.widget(app).show_selection_handles);
                overlay.show_handles(app);
            }
        }
        if let Some(on_selection_changed) = self.widget(app).on_selection_changed.clone() {
            on_selection_changed(app, selection, cause);
        }
        if self.show_blinking_cursor(app) && app.get(self).cursor_timer.is_some() {
            self.stop_cursor_blink(app, false);
            self.start_cursor_blink(app);
        }
    }

    fn finalize_editing(
        self: Handle<Self>,
        app: &mut App,
        action: TextInputAction,
        should_unfocus: bool,
    ) {
        if let Some(on_editing_complete) = self.widget(app).on_editing_complete.clone() {
            on_editing_complete(app);
        } else {
            self.widget(app).controller.clear_composing(app);
            if should_unfocus {
                let focus = self.widget(app).focus_node.as_node();
                match action {
                    TextInputAction::Next => {
                        focus.next_focus(app);
                    }
                    TextInputAction::Previous => {
                        focus.previous_focus(app);
                    }
                    _ => {
                        focus.unfocus(
                            app,
                            crate::widgets::focus_manager::UnfocusDisposition::Scope,
                        );
                    }
                }
            }
        }
        let on_submitted = self.widget(app).on_submitted.clone();
        if let Some(on_submitted) = on_submitted {
            let text = self.value(app).text;
            on_submitted(app, text);
        }
    }

    fn on_cursor_color_tick(self: Handle<Self>, app: &mut App) {
        let show_cursor = self.widget(app).show_cursor;
        let visible = show_cursor
            && (EditableText::debug_deterministic_cursor()
                || self.cursor_blink_opacity_controller(app).value(app) > 0.0);
        self.cursor_visibility_notifier(app).set_value(app, visible);
    }

    fn start_cursor_blink(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).show_cursor || !app.get(self).tickers_enabled {
            return;
        }
        if let Some(timer) = app.get(self).cursor_timer {
            timer.cancel(app);
        }
        self.cursor_blink_opacity_controller(app)
            .set_value(app, 1.0);
        if EditableText::debug_deterministic_cursor() {
            return;
        }
        if self.widget(app).cursor_opacity_animates {
            let future = self.cursor_blink_opacity_controller(app).animate_with(
                app,
                Box::new(DiscreteKeyFrameSimulation::ios_blinking_caret()),
            );
            future.when_complete(app, Listener::handle_method(self, Self::on_cursor_tick));
        } else {
            let timer = Timer::new(
                app,
                K_CURSOR_BLINK_HALF_PERIOD,
                Listener::handle_method(self, Self::on_cursor_tick),
            );
            app.get_mut(self).cursor_timer = Some(timer);
        }
    }

    fn on_cursor_tick(self: Handle<Self>, app: &mut App) {
        if app.get(self).obscure_show_char_ticks_pending > 0 {
            app.get_mut(self).obscure_show_char_ticks_pending -= 1;
            if app.get(self).obscure_show_char_ticks_pending == 0 {
                self.set_state(app, |_| {});
            }
        }
        if self.widget(app).cursor_opacity_animates {
            if let Some(timer) = app.get(self).cursor_timer {
                timer.cancel(app);
            }
            let future = self.cursor_blink_opacity_controller(app).animate_with(
                app,
                Box::new(DiscreteKeyFrameSimulation::ios_blinking_caret()),
            );
            future.when_complete(app, Listener::handle_method(self, Self::on_cursor_tick));
        } else {
            if app.get(self).tickers_enabled {
                let timer = Timer::new(
                    app,
                    K_CURSOR_BLINK_HALF_PERIOD,
                    Listener::handle_method(self, Self::on_cursor_tick),
                );
                app.get_mut(self).cursor_timer = Some(timer);
            }
            let controller = self.cursor_blink_opacity_controller(app);
            let next = if controller.value(app) == 0.0 {
                1.0
            } else {
                0.0
            };
            controller.set_value(app, next);
        }
    }

    fn stop_cursor_blink(self: Handle<Self>, app: &mut App, reset_char_ticks: bool) {
        self.cursor_blink_opacity_controller(app)
            .set_value(app, 0.0);
        if let Some(timer) = app.get(self).cursor_timer {
            timer.cancel(app);
        }
        app.get_mut(self).cursor_timer = None;
        if reset_char_ticks {
            app.get_mut(self).obscure_show_char_ticks_pending = 0;
        }
    }

    fn start_or_stop_cursor_timer_if_needed(self: Handle<Self>, app: &mut App) {
        if !self.show_blinking_cursor(app) {
            self.stop_cursor_blink(app, true);
        } else if app.get(self).cursor_timer.is_none() {
            self.start_cursor_blink(app);
        }
    }

    fn effective_text_scaler(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> TextScaler {
        match (
            self.widget(app).text_scaler.clone(),
            self.widget(app).text_scale_factor,
        ) {
            (Some(scaler), _) => scaler,
            (None, Some(factor)) => TextScaler::linear(factor),
            (None, None) => MediaQuery::text_scaler_of(app, context),
        }
    }

    fn apply_autofocus(self: Handle<Self>, app: &mut App, _elapsed: Duration) {
        if !self.mounted(app) {
            return;
        }
        app.get_mut(self).next_focus_change_is_internal = true;
        let focus_node = self.widget(app).focus_node;
        let context = self.context(app);
        FocusScope::of(app, context, true).autofocus(app, focus_node.as_node());
    }

    fn post_frame_open_connection(self: Handle<Self>, app: &mut App, _elapsed: Duration) {
        if self.mounted(app) {
            self.open_input_connection(app);
        }
    }

    fn post_frame_update_style(self: Handle<Self>, app: &mut App, _elapsed: Duration) {
        if !self.mounted(app) || !self.has_input_connection(app) {
            return;
        }
        let context = self.context(app);
        if let Some(connection) = app.get(self).text_input_connection {
            let style = self.get_text_input_style(app, context);
            connection.update_style(app, style);
        }
    }

    fn composite_callback(self: Handle<Self>, app: &mut App, _layer: AnyLayer) {
        // The callback can be invoked when the layer is detached.
        // The input connection can be closed by the platform in which case this
        // widget doesn't rebuild.
        if !self.render_editable(app).as_object().attached(app) || !self.has_input_connection(app) {
            return;
        }
        debug_assert!(self.mounted(app));
        self.update_size_and_transform(app);
    }

    // Must be called after layout.
    // See https://github.com/flutter/flutter/issues/126312
    fn update_size_and_transform(self: Handle<Self>, app: &mut App) {
        let render_editable = self.render_editable(app);
        let size = render_editable.size(app);
        let transform = render_editable.as_object().get_transform_to(app, None);
        let connection = self.text_input_connection(app);
        connection.set_editable_size_and_transform(app, size, transform);
    }

    fn schedule_periodic_post_frame_callbacks(
        self: Handle<Self>,
        app: &mut App,
        _duration: Duration,
    ) {
        if !self.has_input_connection(app) {
            return;
        }
        // `_updateSelectionRects` waits with scribble (PORTING.md).
        self.update_composing_rect_if_needed(app);
        self.update_caret_rect_if_needed(app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::handle_method(self, Self::schedule_periodic_post_frame_callbacks),
        );
    }

    // Sends the current composing rect to the embedder's text input plugin.
    //
    // In cases where the composing rect hasn't been updated in the embedder due
    // to the lag of asynchronous messages over the channel, the position of the
    // current caret rect is used instead.
    //
    // See: [_updateCaretRectIfNeeded]
    fn update_composing_rect_if_needed(self: Handle<Self>, app: &mut App) {
        let composing_range = self.value(app).composing;
        debug_assert!(self.mounted(app));
        let render_editable = self.render_editable(app);
        let composing_rect = render_editable.get_rect_for_composing_range(app, composing_range);
        // Send the caret location instead if there's no marked text yet.
        let composing_rect = composing_rect.unwrap_or_else(|| {
            let offset = if composing_range.is_valid() {
                composing_range.start
            } else {
                0
            };
            render_editable.get_local_rect_for_caret(app, TextPosition::new(offset))
        });
        self.text_input_connection(app)
            .set_composing_rect(app, composing_rect);
    }

    // Sends the current caret rect to the embedder's text input plugin.
    //
    // The position of the caret rect is updated periodically such that if the
    // user initiates composing input, the current cursor rect can be used for
    // the first character until the composing rect can be sent.
    //
    // On selection changes, the start of the selection is used. This ensures
    // that regardless of the direction the selection was created, the cursor is
    // set to the position where next text input occurs. This position is used to
    // position the IME's candidate selection menu.
    //
    // See: [_updateComposingRectIfNeeded]
    fn update_caret_rect_if_needed(self: Handle<Self>, app: &mut App) {
        let render_editable = self.render_editable(app);
        let Some(selection) = render_editable.selection(app) else {
            return;
        };
        if !selection.is_valid() {
            return;
        }
        let current_text_position = TextPosition::new(selection.start());
        let caret_rect = render_editable.get_local_rect_for_caret(app, current_text_position);
        self.text_input_connection(app)
            .set_caret_rect(app, caret_rect);
    }

    /// Dart's `_textInputConnection!`.
    fn text_input_connection(self: Handle<Self>, app: &App) -> Handle<TextInputConnection> {
        app.get(self)
            .text_input_connection
            .expect("EditableText has an input connection")
    }

    fn build_editable(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        offset: AnyViewportOffset,
    ) -> WidgetRef {
        let inline_span = self.build_text_span(app, context).into_span();
        let value = self.value(app);
        let cursor_color = self.cursor_color(app);
        let background_cursor_color = self.widget(app).background_cursor_color;
        let show_cursor = self.cursor_visibility_notifier(app);
        let force_line = self.widget(app).force_line;
        let read_only = self.widget(app).read_only;
        let has_focus = app.get(self).has_focus;
        let max_lines = self.widget(app).max_lines;
        let min_lines = self.widget(app).min_lines;
        let expands = self.widget(app).expands;
        let selection_color = self.widget(app).selection_color;
        let text_scaler = self.effective_text_scaler(app, context);
        let text_align = self.widget(app).text_align;
        let text_direction = self.text_direction(app, context);
        let locale = self.widget(app).locale.clone();
        let obscuring_character = self.widget(app).obscuring_character.clone();
        let obscure_text = self.widget(app).obscure_text;
        let text_height_behavior = self
            .widget(app)
            .text_height_behavior
            .or_else(|| DefaultTextHeightBehavior::maybe_of(app, context));
        let text_width_basis = self.widget(app).text_width_basis;
        let renderer_ignores_pointer = self.widget(app).renderer_ignores_pointer;
        let cursor_width = self.widget(app).cursor_width;
        let cursor_height = self.widget(app).cursor_height;
        let cursor_radius = self.widget(app).cursor_radius;
        let cursor_offset = self.widget(app).cursor_offset.unwrap_or(Offset::ZERO);
        let selection_height_style = self.widget(app).selection_height_style;
        let selection_width_style = self.widget(app).selection_width_style;
        let paint_cursor_above_text = self.widget(app).paint_cursor_above_text;
        let enable_interactive_selection = self.widget(app).user_selection_enabled();
        let device_pixel_ratio = MediaQuery::device_pixel_ratio_of(app, context);
        let prompt_rect_range = app.get(self).current_prompt_rect_range;
        let prompt_rect_color = self.widget(app).autocorrection_text_rect_color;
        let clip_behavior = self.widget(app).clip_behavior;
        // `CompositedTransformTarget(link: _toolbarLayerLink, child: ..)` around the editable.
        let editable = Editable {
            key: Some(Rc::clone(&app.get(self).editable_key) as KeyRef),
            inline_span,
            value,
            start_handle_layer_link: self.start_handle_layer_link(app),
            end_handle_layer_link: self.end_handle_layer_link(app),
            cursor_color: Some(cursor_color),
            background_cursor_color: Some(background_cursor_color),
            show_cursor,
            force_line,
            read_only,
            has_focus,
            max_lines,
            min_lines,
            expands,
            selection_color,
            text_scaler,
            text_align,
            text_direction,
            locale,
            obscuring_character,
            obscure_text,
            text_height_behavior,
            text_width_basis,
            offset,
            renderer_ignores_pointer,
            cursor_width,
            cursor_height,
            cursor_radius,
            cursor_offset,
            paint_cursor_above_text,
            selection_height_style,
            selection_width_style,
            enable_interactive_selection,
            text_selection_delegate: self.as_text_selection_delegate(),
            device_pixel_ratio,
            prompt_rect_range,
            prompt_rect_color,
            clip_behavior,
        }
        .into_widget();
        CompositedTransformTarget::new(self.toolbar_layer_link(app))
            .child(editable)
            .into_widget()
    }
}

impl State for EditableTextState {
    type Widget = EditableText;

    state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).composite_callback = Some(Rc::new(move |app, layer| {
            self.composite_callback(app, layer)
        }));
        let clipboard_status = if K_IS_WEB {
            ClipboardStatusNotifier::web(app)
        } else {
            ClipboardStatusNotifier::new(app)
        };
        let live_text = if K_IS_WEB {
            None
        } else {
            Some(LiveTextInputStatusNotifier::new(app))
        };
        let cursor_visibility = app.create(ValueNotifier::new(self.widget(app).show_cursor));
        let toolbar_link = LayerLink::new(app);
        let start_link = LayerLink::new(app);
        let end_link = LayerLink::new(app);
        app.get_mut(self).clipboard_status = Some(clipboard_status);
        app.get_mut(self).live_text_input_status = live_text;
        app.get_mut(self).cursor_visibility_notifier = Some(cursor_visibility);
        app.get_mut(self).toolbar_layer_link = Some(toolbar_link);
        app.get_mut(self).start_handle_layer_link = Some(start_link);
        app.get_mut(self).end_handle_layer_link = Some(end_link);
        if let Some(live_text) = live_text {
            live_text.add_listener(
                app,
                Listener::handle_method(self, Self::on_changed_live_text_input_status),
            );
        }
        clipboard_status.add_listener(
            app,
            Listener::handle_method(self, Self::on_changed_clipboard_status),
        );
        let controller = self.widget(app).controller;
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::did_change_text_editing_value),
        );
        let focus_node = self.widget(app).focus_node;
        focus_node.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
        let widget = self.widget(app);
        let inferred = infer_spell_check_configuration(
            widget.spell_check_configuration.as_ref(),
            widget.obscure_text,
            widget.keyboard_type,
            &widget.autofill_hints,
        );
        app.get_mut(self).spell_check_configuration = inferred;
        let listener = AppLifecycleListener::new(app).on_resume(
            app,
            Rc::new({
                let this = self;
                move |app| this.on_resume(app)
            }),
        );
        app.get_mut(self).app_lifecycle_listener = Some(listener);
        self.init_process_text_actions(app);
        AutomaticKeepAliveClientMixin::init_state(self, app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let bold = MediaQuery::bold_text_of(app, context);
        let style = self.widget(app).style.clone();
        app.get_mut(self).style = if bold {
            style.merge(Some(&TextStyle::new().font_weight(FontWeight::BOLD)))
        } else {
            style
        };
        let new_autofill_group = AutofillGroup::maybe_of(app, context);
        if app.get(self).current_autofill_scope != new_autofill_group {
            if let Some(scope) = app.get(self).current_autofill_scope {
                let autofill_id = self.as_autofill_client().autofill_id(app);
                scope.unregister(app, &autofill_id);
            }
            app.get_mut(self).current_autofill_scope = new_autofill_group;
            if let Some(scope) = new_autofill_group {
                let client = self.effective_autofill_client(app);
                scope.register(app, client);
            }
        }
        if !app.get(self).did_auto_focus && self.widget(app).autofocus {
            app.get_mut(self).did_auto_focus = true;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::apply_autofocus),
            );
        }
        let new_ticker_enabled = TickerMode::values_of(app, context).enabled;
        if app.get(self).tickers_enabled != new_ticker_enabled {
            app.get_mut(self).tickers_enabled = new_ticker_enabled;
            if self.show_blinking_cursor(app) {
                self.start_cursor_blink(app);
            } else if !new_ticker_enabled && app.get(self).cursor_timer.is_some() {
                self.stop_cursor_blink(app, true);
            }
        }
        if self.has_input_connection(app) {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::post_frame_update_style),
            );
        }
        let platform = app.platform().target_platform();
        if platform != TargetPlatform::IOS && platform != TargetPlatform::Android {
            return;
        }
        let orientation = MediaQuery::orientation_of(app, context);
        if app.get(self).last_orientation.is_none() {
            app.get_mut(self).last_orientation = Some(orientation);
            return;
        }
        if app.get(self).last_orientation != Some(orientation) {
            app.get_mut(self).last_orientation = Some(orientation);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &EditableText) {
        if self.widget(app).controller != old_widget.controller {
            old_widget.controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::did_change_text_editing_value),
            );
            let controller = self.widget(app).controller;
            controller.add_listener(
                app,
                Listener::handle_method(self, Self::did_change_text_editing_value),
            );
            self.update_remote_editing_value_if_needed(app);
        }
        if self.widget(app).focus_node != old_widget.focus_node {
            old_widget.focus_node.remove_listener(
                app,
                &Listener::handle_method(self, Self::handle_focus_changed),
            );
            let focus_node = self.widget(app).focus_node;
            focus_node.add_listener(
                app,
                Listener::handle_method(self, Self::handle_focus_changed),
            );
            AutomaticKeepAliveClientMixin::update_keep_alive(self, app);
        }
        if self.widget(app).autofill_client != old_widget.autofill_client
            && let Some(scope) = app.get(self).current_autofill_scope
        {
            let old_id = old_widget
                .autofill_client
                .map(|client| client.autofill_id(app))
                .unwrap_or_else(|| self.as_autofill_client().autofill_id(app));
            scope.unregister(app, &old_id);
            let client = self.effective_autofill_client(app);
            scope.register(app, client);
        }
        if !self.should_create_input_connection(app) {
            self.close_input_connection_if_needed(app);
        } else if old_widget.read_only && app.get(self).has_focus {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::post_frame_open_connection),
            );
        }
        if self.has_input_connection(app) {
            let obscure_changed = old_widget.obscure_text != self.widget(app).obscure_text;
            if obscure_changed || old_widget.keyboard_type != self.widget(app).keyboard_type {
                if obscure_changed {
                    app.get_mut(self).obscure_show_char_ticks_pending = 0;
                    app.get_mut(self).obscure_latest_char_index = None;
                }
                if let Some(connection) = app.get(self).text_input_connection {
                    let config = self
                        .effective_autofill_client(app)
                        .text_input_configuration(app);
                    connection.update_config(app, config);
                }
            }
        }
        let widget = self.widget(app);
        if old_widget
            .spell_check_configuration
            .as_ref()
            .map(|c| c.spell_check_enabled())
            != widget
                .spell_check_configuration
                .as_ref()
                .map(|c| c.spell_check_enabled())
            || old_widget.obscure_text != widget.obscure_text
            || old_widget.keyboard_type != widget.keyboard_type
            || old_widget.autofill_hints != widget.autofill_hints
        {
            let inferred = infer_spell_check_configuration(
                widget.spell_check_configuration.as_ref(),
                widget.obscure_text,
                widget.keyboard_type,
                &widget.autofill_hints,
            );
            app.get_mut(self).spell_check_configuration = inferred;
            if !self.spell_check_enabled(app) {
                app.get_mut(self).spell_check_results = None;
            }
        }
        if self.widget(app).style != old_widget.style {
            let context = self.context(app);
            let bold = MediaQuery::bold_text_of(app, context);
            let style = self.widget(app).style.clone();
            app.get_mut(self).style = if bold {
                style.merge(Some(&TextStyle::new().font_weight(FontWeight::BOLD)))
            } else {
                style
            };
            if self.has_input_connection(app) {
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::handle_method(self, Self::post_frame_update_style),
                );
            }
        }
        if self.widget(app).show_cursor != old_widget.show_cursor {
            self.start_or_stop_cursor_timer_if_needed(app);
        }
        let selection_overlay = app.get(self).selection_overlay;
        if let Some(overlay) = selection_overlay
            && overlay.toolbar_is_visible(app)
            && !option_rc_eq(
                &self.widget(app).context_menu_builder,
                &old_widget.context_menu_builder,
            )
            && self.widget(app).context_menu_builder.is_none()
                == old_widget.context_menu_builder.is_none()
        {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::post_frame_rebuild_toolbar),
            );
        }
        if app.get(self).selection_overlay.is_some()
            && (self.widget(app).context_menu_builder.is_none()
                != old_widget.context_menu_builder.is_none()
                || !option_rc_eq(
                    &self.widget(app).selection_controls,
                    &old_widget.selection_controls,
                )
                || !option_rc_eq(
                    &self.widget(app).on_selection_handle_tapped,
                    &old_widget.on_selection_handle_tapped,
                )
                || self.widget(app).drag_start_behavior != old_widget.drag_start_behavior
                || !self
                    .widget(app)
                    .magnifier_configuration
                    .same_configuration(&old_widget.magnifier_configuration))
        {
            let overlay = app.get(self).selection_overlay.expect("checked above");
            let should_show_toolbar = overlay.toolbar_is_visible(app);
            let should_show_handles = overlay.handles_visible_value(app);
            overlay.dispose(app);
            let created = self.create_selection_overlay(app);
            app.get_mut(self).selection_overlay = Some(created);
            if should_show_toolbar || should_show_handles {
                let this = self;
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _| {
                        if let Some(overlay) = app.get(this).selection_overlay {
                            if should_show_toolbar {
                                overlay.show_toolbar(app);
                            }
                            if should_show_handles {
                                overlay.show_handles(app);
                            }
                        }
                    }),
                );
            }
        } else if self.widget(app).controller.selection(app) != old_widget.controller.selection(app)
            && let Some(overlay) = app.get(self).selection_overlay
        {
            overlay.update(app, self.value(app));
        }
        if let Some(overlay) = app.get(self).selection_overlay {
            overlay.set_handles_visible(app, self.widget(app).show_selection_handles);
        }
        #[allow(deprecated)]
        let can_paste = self
            .widget(app)
            .selection_controls
            .as_ref()
            .is_some_and(|controls| controls.can_paste(app, self.as_text_selection_delegate()));
        if self.widget(app).enable_interactive_selection && self.paste_enabled(app) && can_paste {
            self.clipboard_status(app).update(app);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).internal_scroll_controller {
            controller.dispose(app);
        }
        if let Some(scope) = app.get(self).current_autofill_scope {
            let autofill_id = self.as_autofill_client().autofill_id(app);
            scope.unregister(app, &autofill_id);
        }
        let editing_controller = self.widget(app).controller;
        editing_controller.remove_listener(
            app,
            &Listener::handle_method(self, Self::did_change_text_editing_value),
        );
        self.close_input_connection_if_needed(app);
        debug_assert!(!self.has_input_connection(app));
        if let Some(timer) = app.get(self).cursor_timer {
            timer.cancel(app);
        }
        app.get_mut(self).cursor_timer = None;
        if let Some(controller) = app.get(self).backing_cursor_blink_opacity_controller {
            controller.dispose(app);
        }
        app.get_mut(self).backing_cursor_blink_opacity_controller = None;
        if let Some(overlay) = app.get(self).selection_overlay {
            overlay.dispose(app);
        }
        app.get_mut(self).selection_overlay = None;
        let focus_node = self.widget(app).focus_node;
        focus_node.remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        if app.get(self).registered_as_observer {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).remove_observer(app, &observer);
            app.get_mut(self).registered_as_observer = false;
        }
        if let Some(live_text) = app.get(self).live_text_input_status {
            live_text.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_changed_live_text_input_status),
            );
            live_text.dispose(app);
        }
        if let Some(clipboard) = app.get(self).clipboard_status {
            clipboard.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_changed_clipboard_status),
            );
            clipboard.dispose(app);
        }
        if let Some(notifier) = app.get(self).cursor_visibility_notifier {
            app.get_mut(notifier).dispose();
        }
        // Dart lets the links go with the state; the recordings that still name them retain
        // them past this.
        let this = app.get_mut(self);
        let links = [
            this.toolbar_layer_link.take(),
            this.start_handle_layer_link.take(),
            this.end_handle_layer_link.take(),
        ];
        for link in links.into_iter().flatten() {
            app.destroy(link);
        }
        if let Some(listener) = app.get(self).app_lifecycle_listener {
            listener.dispose(app);
        }
        FocusManager::instance(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::reset_just_resumed),
        );
        debug_assert!(
            app.get(self).batch_edit_depth <= 0,
            "unfinished batch edits: {}",
            app.get(self).batch_edit_depth
        );
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = AutomaticKeepAliveClientMixin::build(self, app);
        let is_multiline = self.is_multiline(app);
        let platform = app.platform().target_platform();
        let physics = self.widget(app).scroll_physics.clone().or_else(|| {
            if !is_multiline && platform == TargetPlatform::IOS {
                Some(Rc::new(NeverUserScrollableScrollPhysics::new()))
            } else {
                None
            }
        });
        let scroll_behavior = self.widget(app).scroll_behavior.clone().unwrap_or_else(|| {
            ScrollBehaviorRef::new(
                ScrollConfiguration::of(app, context)
                    .copy_with()
                    .scrollbars(is_multiline)
                    .overscroll(false),
            )
        });
        let restoration_id = self.widget(app).restoration_id.clone();
        let drag_start_behavior = self.widget(app).drag_start_behavior;
        let controller = self.scroll_controller(app);
        let focus_node = self.widget(app).focus_node;
        let state = self;
        let viewport_builder: ViewportBuilder =
            Rc::new(move |app, context, offset| state.build_editable(app, context, offset));
        let mut scrollable = Scrollable::new(viewport_builder)
            .key(Rc::clone(&app.get(self).scrollable_key) as KeyRef)
            .axis_direction(if is_multiline {
                AxisDirection::Down
            } else {
                AxisDirection::Right
            })
            .controller(controller.as_controller())
            .drag_start_behavior(drag_start_behavior)
            .scroll_behavior(scroll_behavior);
        if let Some(physics) = physics {
            scrollable = scrollable.physics(physics);
        }
        if let Some(restoration_id) = restoration_id {
            scrollable = scrollable.restoration_id(restoration_id);
        }
        let focused = Focus::new(scrollable)
            .focus_node(focus_node.as_node())
            .include_semantics(false);
        let editing_controller = self.widget(app).controller;
        let undo_controller = self.widget(app).undo_controller;
        let state = self;
        let mut history = UndoHistory::new(
            editing_controller,
            move |app, value| {
                state.user_update_text_editing_value(app, value, SelectionChangedCause::Keyboard);
            },
            focus_node,
            focused,
        )
        .should_change_undo_stack(move |old_value, new_value| {
            if !new_value.selection.is_valid() {
                return false;
            }
            let Some(old_value) = old_value else {
                return true;
            };
            match platform {
                TargetPlatform::IOS
                | TargetPlatform::MacOS
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {
                    if !new_value.composing.is_collapsed() {
                        return false;
                    }
                }
                TargetPlatform::Android => {}
            }
            old_value.text != new_value.text || old_value.composing != new_value.composing
        })
        .undo_stack_modifier(move |value| {
            if platform == TargetPlatform::Android {
                value.copy_with().composing(TextRange::EMPTY)
            } else {
                value
            }
        });
        if let Some(undo_controller) = undo_controller {
            history = history.controller(undo_controller);
        }
        let focused = history.into_widget();
        let actions = self.ensure_actions(app, context);
        let has_focus = app.get(self).has_focus;
        let group_id = self.widget(app).group_id;
        let state = self;
        let composite_callback = app
            .get(self)
            .composite_callback
            .clone()
            .expect("set in init_state");
        let enabled = self.has_input_connection(app);
        let child = Actions::new(
            actions,
            Builder::new(move |_app, tap_context| {
                let mut region = TextFieldTapRegion::new(focused.clone())
                    .group_id(TapRegionGroupId::Type(group_id));
                if has_focus {
                    #[allow(clippy::redundant_locals)]
                    let tap_context = tap_context;
                    region = region.on_tap_outside(Rc::new(move |app, event| {
                        state.on_tap_outside(app, tap_context, event);
                    }));
                }
                region = region.on_tap_up_outside(Rc::new(move |app, event| {
                    state.on_tap_up_outside(app, tap_context, event);
                }));
                if cfg!(debug_assertions) {
                    region = region.debug_label("EditableText");
                }
                region.into_widget()
            }),
        )
        .into_widget();
        CompositionCallbackWidget::new(composite_callback, enabled, child).into_widget()
    }
}

impl AutomaticKeepAliveClientMixin for EditableTextState {
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData {
        &app.get(self).automatic_keep_alive_client
    }

    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData {
        &mut app.get_mut(self).automatic_keep_alive_client
    }

    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
        app.get(self).has_focus
    }
}

impl TickerProviderStateMixin for EditableTextState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl TickerProviderObject for EditableTextState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl WidgetsBindingObserverObject for EditableTextState {
    fn did_change_metrics(self: Handle<Self>, app: &mut App) {
        if !self.mounted(app) {
            return;
        }
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::handle_method(self, Self::post_frame_update_for_scroll),
        );
    }
}

impl TextInputClient for EditableTextState {
    fn current_text_editing_value(self: Handle<Self>, app: &App) -> Option<TextEditingValue> {
        Some(self.value(app))
    }

    fn update_editing_value(self: Handle<Self>, app: &mut App, mut value: TextEditingValue) {
        if !self.should_create_input_connection(app) {
            return;
        }
        if self.check_needs_adjust_affinity(app, &value) {
            value = value.copy_with().selection(
                value
                    .selection
                    .copy_with()
                    .affinity(self.value(app).selection.affinity),
            );
        }
        if self.widget(app).read_only {
            value = self.value(app).copy_with().selection(value.selection);
        }
        app.get_mut(self).last_known_remote_text_editing_value = Some(value.clone());
        if value == self.value(app) {
            return;
        }
        let current = self.value(app);
        if value.text == current.text && value.composing == current.composing {
            self.handle_selection_changed(
                app,
                value.selection,
                Some(SelectionChangedCause::Keyboard),
            );
        } else {
            app.get_mut(self).current_prompt_rect_range = None;
            let current = self.value(app);
            let reveal_obscured_input = self.has_input_connection(app)
                && self.widget(app).obscure_text
                && utf16_len(&value.text) == utf16_len(&current.text) + 1;
            app.get_mut(self).obscure_show_char_ticks_pending = if reveal_obscured_input {
                K_OBSCURE_SHOW_LATEST_CHAR_CURSOR_TICKS
            } else {
                0
            };
            app.get_mut(self).obscure_latest_char_index = if reveal_obscured_input {
                Some(current.selection.base_offset)
            } else {
                None
            };
            self.format_and_set_value(app, value, Some(SelectionChangedCause::Keyboard), false);
        }
        if self.show_blinking_cursor(app) && app.get(self).cursor_timer.is_some() {
            self.stop_cursor_blink(app, false);
            self.start_cursor_blink(app);
        }
    }

    fn perform_action(self: Handle<Self>, app: &mut App, action: TextInputAction) {
        match action {
            TextInputAction::Newline => {
                if !self.is_multiline(app) {
                    self.finalize_editing(app, action, true);
                }
            }
            TextInputAction::Done
            | TextInputAction::Go
            | TextInputAction::Next
            | TextInputAction::Previous
            | TextInputAction::Search
            | TextInputAction::Send => {
                self.finalize_editing(app, action, true);
            }
            TextInputAction::ContinueAction
            | TextInputAction::EmergencyCall
            | TextInputAction::Join
            | TextInputAction::None
            | TextInputAction::Route
            | TextInputAction::Unspecified => {
                self.finalize_editing(app, action, false);
            }
        }
    }

    fn perform_private_command(self: Handle<Self>, app: &mut App, action: &str) {
        if let Some(callback) = self.widget(app).on_app_private_command.clone() {
            callback(app, action);
        }
    }

    fn connection_closed(self: Handle<Self>, app: &mut App) {
        if self.has_input_connection(app) {
            if let Some(connection) = app.get(self).text_input_connection {
                connection.connection_closed_received(app);
            }
            app.get_mut(self).text_input_connection = None;
            app.get_mut(self).last_known_remote_text_editing_value = None;
            let focus_node = self.widget(app).focus_node;
            focus_node.unfocus(
                app,
                crate::widgets::focus_manager::UnfocusDisposition::Scope,
            );
        }
    }

    fn show_autocorrection_prompt_rect(self: Handle<Self>, app: &mut App, start: i32, end: i32) {
        self.set_state(app, |state| {
            state.current_prompt_rect_range = Some(TextRange::new(start, end));
        });
    }

    fn show_toolbar(self: Handle<Self>, app: &mut App) {
        let _ = EditableTextState::show_toolbar(self, app);
    }

    fn perform_selector(self: Handle<Self>, app: &mut App, selector_name: &str) {
        let Some(intent) = intent_for_macos_selector(selector_name) else {
            return;
        };
        let Some(primary_context) = FocusManager::instance(app)
            .primary_focus(app)
            .and_then(|focus| focus.context(app))
        else {
            return;
        };
        Actions::invoke(app, primary_context, &*intent);
    }
}

impl AutofillClient for EditableTextState {
    fn autofill_id(self: Handle<Self>, _app: &App) -> String {
        format!("{:?}", self.id())
    }

    fn text_input_configuration(self: Handle<Self>, app: &App) -> TextInputConfiguration {
        let context = self.mounted(app).then(|| self.context(app));
        // Configuration reads only widget + value; view id is omitted without `&mut App`.
        let _ = context;
        let widget = self.widget(app);
        let input_action = widget
            .text_input_action
            .unwrap_or(if widget.max_lines != Some(1) {
                TextInputAction::Newline
            } else {
                TextInputAction::Done
            });
        let autofill_configuration = AutofillConfiguration::new(
            format!("{:?}", self.id()),
            widget.autofill_hints.clone(),
            self.value(app),
        );
        let mut config = TextInputConfiguration::new()
            .input_type(widget.keyboard_type)
            .read_only(widget.read_only)
            .obscure_text(widget.obscure_text)
            .autocorrect(widget.autocorrect)
            .enable_suggestions(widget.enable_suggestions)
            .enable_interactive_selection(widget.enable_interactive_selection)
            .input_action(input_action)
            .text_capitalization(widget.text_capitalization)
            .keyboard_appearance(widget.keyboard_appearance)
            .enable_ime_personalized_learning(widget.enable_ime_personalized_learning);
        config.smart_dashes_type = widget.smart_dashes_type;
        config.smart_quotes_type = widget.smart_quotes_type;
        config.autofill_configuration = autofill_configuration;
        config.allowed_mime_types = widget
            .content_insertion_configuration
            .as_ref()
            .map(|c| c.allowed_mime_types.clone())
            .unwrap_or_default();
        config.hint_locales = widget.hint_locales.clone().unwrap_or_default();
        config.enable_inline_prediction = widget.enable_inline_prediction;
        config
    }

    fn autofill(self: Handle<Self>, app: &mut App, new_editing_value: TextEditingValue) {
        self.update_editing_value(app, new_editing_value);
    }
}

impl TextSelectionDelegate for EditableTextState {
    fn text_editing_value(self: Handle<Self>, app: &App) -> TextEditingValue {
        self.value(app)
    }

    fn user_update_text_editing_value(
        self: Handle<Self>,
        app: &mut App,
        value: TextEditingValue,
        cause: SelectionChangedCause,
    ) {
        if value == self.value(app) {
            if !app.get(self).has_focus {
                app.get_mut(self).next_focus_change_is_internal = true;
                let focus_node = self.widget(app).focus_node;
                focus_node.request_focus(app, None);
                if app.get(self).selection_overlay.is_none() {
                    let overlay = self.create_selection_overlay(app);
                    app.get_mut(self).selection_overlay = Some(overlay);
                }
            }
            return;
        }
        self.format_and_set_value(app, value, Some(cause), true);
    }

    fn hide_toolbar(self: Handle<Self>, app: &mut App, hide_handles: bool) {
        EditableTextState::hide_toolbar(self, app, hide_handles);
    }

    fn bring_into_view(self: Handle<Self>, app: &mut App, position: TextPosition) {
        EditableTextState::bring_into_view(self, app, position);
    }

    fn cut_enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.toolbar_options.cut && !widget.read_only && !widget.obscure_text
    }

    fn copy_enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.toolbar_options.copy && !widget.obscure_text
    }

    fn paste_enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.toolbar_options.paste && !widget.read_only
    }

    fn select_all_enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.toolbar_options.select_all
            && (!widget.read_only || !widget.obscure_text)
            && widget.enable_interactive_selection
    }

    fn look_up_enabled(self: Handle<Self>, app: &App) -> bool {
        if app.platform().target_platform() != TargetPlatform::IOS {
            return false;
        }
        let value = self.value(app);
        !self.widget(app).obscure_text
            && !value.selection.is_collapsed()
            && !value
                .selection
                .range()
                .text_inside(&value.text)
                .trim()
                .is_empty()
    }

    fn search_web_enabled(self: Handle<Self>, app: &App) -> bool {
        self.look_up_enabled(app)
    }

    fn share_enabled(self: Handle<Self>, app: &App) -> bool {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::IOS => {
                let value = self.value(app);
                !self.widget(app).obscure_text
                    && !value.selection.is_collapsed()
                    && !value
                        .selection
                        .range()
                        .text_inside(&value.text)
                        .trim()
                        .is_empty()
            }
            _ => false,
        }
    }

    fn live_text_input_enabled(self: Handle<Self>, app: &App) -> bool {
        let Some(status) = app.get(self).live_text_input_status else {
            return false;
        };
        status.value(app) == LiveTextInputStatus::Enabled
            && !self.widget(app).obscure_text
            && !self.widget(app).read_only
            && self.value(app).selection.is_collapsed()
    }

    fn copy_selection(self: Handle<Self>, app: &mut App, cause: SelectionChangedCause) {
        let value = self.value(app);
        if value.selection.is_collapsed() || self.widget(app).obscure_text {
            return;
        }
        let text = value.text.clone();
        Clipboard::set_data(
            app,
            ClipboardData::new(value.selection.range().text_inside(&text).to_string()),
        );
        if cause == SelectionChangedCause::Toolbar {
            self.bring_into_view(app, self.value(app).selection.extent());
            TextSelectionDelegate::hide_toolbar(self, app, false);

            match app.platform().target_platform() {
                TargetPlatform::IOS
                | TargetPlatform::MacOS
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {}
                TargetPlatform::Android | TargetPlatform::Fuchsia => {
                    let value = self.value(app);
                    let collapsed = value.copy_with().selection(TextSelection::collapsed(
                        value.selection.end(),
                        TextAffinity::Downstream,
                    ));
                    self.user_update_text_editing_value(
                        app,
                        collapsed,
                        SelectionChangedCause::Toolbar,
                    );
                }
            }
        }
        self.clipboard_status(app).update(app);
    }

    fn cut_selection(self: Handle<Self>, app: &mut App, cause: SelectionChangedCause) {
        if self.widget(app).read_only || self.widget(app).obscure_text {
            return;
        }
        let value = self.value(app);
        if value.selection.is_collapsed() {
            return;
        }
        let text = value.selection.range().text_inside(&value.text).to_string();
        Clipboard::set_data(app, ClipboardData::new(text));
        let next = value.replaced(value.selection, "");
        self.user_update_text_editing_value(app, next, cause);
        if cause == SelectionChangedCause::Toolbar {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, Self::bring_selection_into_view_after_frame),
            );
            TextSelectionDelegate::hide_toolbar(self, app, true);
        }
        self.clipboard_status(app).update(app);
    }

    fn paste_text(self: Handle<Self>, app: &mut App, cause: SelectionChangedCause) {
        if !self.allow_paste(app) {
            return;
        }
        let Some(data) = Clipboard::get_data(app, Clipboard::K_TEXT_PLAIN) else {
            return;
        };
        let Some(text) = data.text else {
            return;
        };
        self.paste_text_content(app, cause, &text);
    }

    fn select_all(self: Handle<Self>, app: &mut App, cause: SelectionChangedCause) {
        if self.widget(app).read_only && self.widget(app).obscure_text {
            return;
        }
        let value = self.value(app);
        let next = value
            .copy_with()
            .selection(TextSelection::new(0, utf16_len(&value.text)));
        self.user_update_text_editing_value(app, next, cause);

        if cause == SelectionChangedCause::Toolbar {
            match app.platform().target_platform() {
                TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {}
                TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => {
                    TextSelectionDelegate::hide_toolbar(self, app, true);
                }
            }
            match app.platform().target_platform() {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {
                    self.bring_into_view(app, self.value(app).selection.extent());
                }
                TargetPlatform::MacOS | TargetPlatform::IOS => {}
            }
        }
    }
}

/// Leaf render-object widget that configures [`RenderEditable`].
///
/// Flutter's `_Editable` is a `MultiChildRenderObjectWidget` so `WidgetSpan`
/// children can mount. Ours [`RenderEditable`] is a leaf, so this widget is too.
struct Editable {
    key: Option<KeyRef>,
    inline_span: InlineSpanRef,
    value: TextEditingValue,
    start_handle_layer_link: Handle<LayerLink>,
    end_handle_layer_link: Handle<LayerLink>,
    cursor_color: Option<Color>,
    background_cursor_color: Option<Color>,
    show_cursor: Handle<ValueNotifier<bool>>,
    force_line: bool,
    read_only: bool,
    has_focus: bool,
    max_lines: Option<i32>,
    min_lines: Option<i32>,
    expands: bool,
    selection_color: Option<Color>,
    text_scaler: TextScaler,
    text_align: TextAlign,
    text_direction: TextDirection,
    locale: Option<Locale>,
    obscuring_character: String,
    obscure_text: bool,
    text_height_behavior: Option<TextHeightBehavior>,
    text_width_basis: TextWidthBasis,
    offset: AnyViewportOffset,
    renderer_ignores_pointer: bool,
    cursor_width: f64,
    cursor_height: Option<f64>,
    cursor_radius: Option<Radius>,
    cursor_offset: Offset,
    paint_cursor_above_text: bool,
    selection_height_style: BoxHeightStyle,
    selection_width_style: BoxWidthStyle,
    enable_interactive_selection: bool,
    text_selection_delegate: AnyTextSelectionDelegate,
    device_pixel_ratio: f64,
    prompt_rect_range: Option<TextRange>,
    prompt_rect_color: Option<Color>,
    clip_behavior: Clip,
}

impl Debug for Editable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Editable")
            .field("has_focus", &self.has_focus)
            .field("read_only", &self.read_only)
            .finish_non_exhaustive()
    }
}

impl Editable {
    fn apply_to_render_object(&self, app: &mut App, render_object: RenderHandle<RenderEditable>) {
        render_object.set_text(app, Some(self.inline_span.clone()));
        render_object.set_cursor_color(app, self.cursor_color);
        render_object.set_start_handle_layer_link(app, self.start_handle_layer_link);
        render_object.set_end_handle_layer_link(app, self.end_handle_layer_link);
        render_object.set_background_cursor_color(app, self.background_cursor_color);
        render_object.set_show_cursor(app, self.show_cursor);
        render_object.set_force_line(app, self.force_line);
        render_object.set_read_only(app, self.read_only);
        render_object.set_has_focus(app, self.has_focus);
        render_object.set_max_lines(app, self.max_lines);
        render_object.set_min_lines(app, self.min_lines);
        render_object.set_expands(app, self.expands);
        render_object.set_selection_color(app, self.selection_color);
        render_object.set_text_scaler(app, self.text_scaler.clone());
        render_object.set_text_align(app, self.text_align);
        render_object.set_text_direction(app, self.text_direction);
        render_object.set_selection(app, Some(self.value.selection));
        render_object.set_offset(app, self.offset);
        render_object.get_mut(app).ignore_pointer = self.renderer_ignores_pointer;
        render_object.set_text_height_behavior(app, self.text_height_behavior);
        render_object.set_text_width_basis(app, self.text_width_basis);
        render_object.set_obscuring_character(app, self.obscuring_character.clone());
        render_object.set_obscure_text(app, self.obscure_text);
        render_object.set_cursor_width(app, self.cursor_width);
        render_object.set_cursor_height(app, self.cursor_height);
        render_object.set_cursor_radius(app, self.cursor_radius);
        render_object.set_cursor_offset(app, self.cursor_offset);
        render_object.set_selection_height_style(app, self.selection_height_style);
        render_object.set_selection_width_style(app, self.selection_width_style);
        render_object
            .set_enable_interactive_selection(app, Some(self.enable_interactive_selection));
        render_object.set_text_selection_delegate(app, self.text_selection_delegate);
        render_object.set_device_pixel_ratio(app, self.device_pixel_ratio);
        render_object.set_paint_cursor_above_text(app, self.paint_cursor_above_text);
        render_object.set_prompt_rect_color(app, self.prompt_rect_color);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_prompt_rect_range(app, self.prompt_rect_range);
        let _ = &self.locale;
    }
}

impl RenderObjectWidget for Editable {
    type RenderObject = RenderEditable;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let render_object = RenderEditable::new(
            app,
            self.text_direction,
            self.start_handle_layer_link,
            self.end_handle_layer_link,
            self.offset,
            self.text_selection_delegate,
        );
        self.apply_to_render_object(app, render_object);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderEditable>,
    ) {
        self.apply_to_render_object(app, render_object);
    }
}

impl LeafRenderObjectWidget for Editable {}

/// Prevents scrolling with user input, but still allows programmatic scrolling.
struct NeverUserScrollableScrollPhysics {
    pub parent: Option<ScrollPhysicsRef>,
}

impl NeverUserScrollableScrollPhysics {
    fn new() -> NeverUserScrollableScrollPhysics {
        NeverUserScrollableScrollPhysics { parent: None }
    }
}

impl ScrollPhysics for NeverUserScrollableScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(NeverUserScrollableScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn allow_user_scrolling(&self) -> bool {
        false
    }
}

impl Debug for NeverUserScrollableScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NeverUserScrollableScrollPhysics")
            .field("parent", &self.parent)
            .finish()
    }
}

/// Signature for a function that determines the target location of the given
/// [`TextPosition`] after applying the given [`TextBoundary`].
///
/// Dart's `_ApplyTextBoundary`.
type ApplyTextBoundary =
    fn(Handle<EditableTextState>, &mut App, TextPosition, bool, &dyn TextBoundary) -> TextPosition;

/// Builds the [`TextBoundary`] an action moves by, from the state's current value.
///
/// Dart passes the state's `TextBoundary Function()` tear-offs.
type GetTextBoundary = fn(Handle<EditableTextState>, &mut App) -> Box<dyn TextBoundary>;

/// Dart's `_makeOverridable`, at the one place the map needs it.
fn insert_overridable(
    app: &mut App,
    actions: &mut HashMap<TypeId, AnyAction>,
    context: BuildContext,
    intent: TypeId,
    action: AnyAction,
) {
    actions.insert(intent, AnyAction::overridable(app, action, context));
}

/// Dart `String.codeUnitAt`, or `None` past either end.
fn code_unit_at(text: &str, index: i32) -> Option<i32> {
    if index < 0 {
        return None;
    }
    text.encode_utf16().nth(index as usize).map(i32::from)
}

/// A [`TextBoundary`] that clamps to code point boundaries, so an obscured field walks by
/// the bullets it shows rather than by the graphemes it hides.
///
/// Dart's `_CodePointBoundary`.
#[derive(Clone, Debug)]
struct CodePointBoundary {
    text: Vec<u16>,
}

impl CodePointBoundary {
    fn new(text: impl AsRef<str>) -> CodePointBoundary {
        CodePointBoundary {
            text: text.as_ref().encode_utf16().collect(),
        }
    }

    fn len(&self) -> i32 {
        self.text.len() as i32
    }

    /// Returns true if the given position falls in the center of a surrogate pair.
    fn breaks_surrogate_pair(&self, position: i32) -> bool {
        debug_assert!(position > 0 && position < self.len() && self.len() > 1);
        let high = self.text[position as usize - 1];
        let low = self.text[position as usize];
        (0xD800..0xDC00).contains(&high) && (0xDC00..0xE000).contains(&low)
    }
}

impl TextBoundary for CodePointBoundary {
    fn get_leading_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if self.text.is_empty() || position < 0 {
            return None;
        }
        if position == 0 {
            return Some(0);
        }
        if position >= self.len() {
            return Some(self.len());
        }
        if self.len() <= 1 {
            return Some(position);
        }
        Some(if self.breaks_surrogate_pair(position) {
            position - 1
        } else {
            position
        })
    }

    fn get_trailing_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if self.text.is_empty() || position >= self.len() {
            return None;
        }
        if position < 0 {
            return Some(0);
        }
        if position == self.len() - 1 {
            return Some(self.len());
        }
        if self.len() <= 1 {
            return Some(position);
        }
        Some(if self.breaks_surrogate_pair(position + 1) {
            position + 2
        } else {
            position + 1
        })
    }
}

// -------------------------------  Text Actions -------------------------------

/// Deletes from the caret to the boundary `get_text_boundary` names.
///
/// Dart's `_DeleteTextAction`.
struct DeleteTextAction<T: DirectionalTextEditingIntent> {
    action: ActionData,
    state: Handle<EditableTextState>,
    get_text_boundary: GetTextBoundary,
    apply_text_boundary: ApplyTextBoundary,
    intent: PhantomData<T>,
}

impl<T: DirectionalTextEditingIntent> DeleteTextAction<T> {
    fn new(
        app: &mut App,
        state: Handle<EditableTextState>,
        get_text_boundary: GetTextBoundary,
        apply_text_boundary: ApplyTextBoundary,
    ) -> Handle<DeleteTextAction<T>> {
        app.create(DeleteTextAction {
            action: ActionData::new(),
            state,
            get_text_boundary,
            apply_text_boundary,
            intent: PhantomData,
        })
    }

    fn hide_toolbar_if_text_changed(self: Handle<Self>, app: &mut App, intent: &ReplaceTextIntent) {
        let state = app.get(self).state;
        let visible = app
            .get(state)
            .selection_overlay
            .is_some_and(|overlay| overlay.toolbar_is_visible(app));
        if !visible {
            return;
        }
        let old_value = intent.current_text_editing_value.clone();
        let new_value = old_value.replaced(intent.replacement_range, &intent.replacement_text);
        if old_value.text != new_value.text {
            // Hide the toolbar if the text was changed, but only hide the toolbar overlay;
            // the selection handle's visibility will be handled by handle_selection_changed.
            state.hide_toolbar(app, false);
        }
    }
}

impl<T: DirectionalTextEditingIntent> Action for DeleteTextAction<T> {
    type Intent = T;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl<T: DirectionalTextEditingIntent> ContextAction for DeleteTextAction<T> {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        _intent: &T,
        _context: Option<BuildContext>,
    ) -> bool {
        let state = app.get(self).state;
        !state.widget(app).read_only && state.value(app).selection.is_valid()
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        let selection = state.value(app).selection;
        if !selection.is_valid() {
            return None;
        }
        let context = context.expect("a delete needs a context");
        let text_length = utf16_len(&state.value(app).text);
        // Expands the selection to ensure the range covers full graphemes.
        let atomic_boundary = state.character_boundary(app);
        if !selection.is_collapsed() {
            let range = TextRange::new(
                atomic_boundary
                    .get_leading_text_boundary_at(app, selection.start())
                    .unwrap_or(text_length),
                atomic_boundary
                    .get_trailing_text_boundary_at(app, selection.end() - 1)
                    .unwrap_or(0),
            );
            let replace_text_intent = ReplaceTextIntent::new(
                state.value(app),
                String::new(),
                range,
                SelectionChangedCause::Keyboard,
            );
            self.hide_toolbar_if_text_changed(app, &replace_text_intent);
            return Actions::invoke(app, context, &replace_text_intent);
        }

        let boundary = (app.get(self).get_text_boundary)(state, app);
        let apply = app.get(self).apply_text_boundary;
        let target = apply(state, app, selection.base(), intent.forward(), &*boundary).offset;

        let range_to_delete = TextSelection::new(
            if intent.forward() {
                atomic_boundary
                    .get_leading_text_boundary_at(app, selection.base_offset)
                    .unwrap_or(text_length)
            } else {
                atomic_boundary
                    .get_trailing_text_boundary_at(app, selection.base_offset - 1)
                    .unwrap_or(0)
            },
            target,
        );
        let replace_text_intent = ReplaceTextIntent::new(
            state.value(app),
            String::new(),
            range_to_delete.range(),
            SelectionChangedCause::Keyboard,
        );
        self.hide_toolbar_if_text_changed(app, &replace_text_intent);
        Actions::invoke(app, context, &replace_text_intent)
    }
}

/// Moves or extends the selection to the boundary `get_text_boundary` names.
///
/// Dart's `_UpdateTextSelectionAction`.
struct UpdateTextSelectionAction<T: DirectionalCaretMovementIntent> {
    action: ActionData,
    state: Handle<EditableTextState>,
    ignore_non_collapsed_selection: bool,
    is_expand: bool,
    extent_at_index: bool,
    get_text_boundary: GetTextBoundary,
    apply_text_boundary: ApplyTextBoundary,
    intent: PhantomData<T>,
}

/// Dart's `_UpdateTextSelectionAction.NEWLINE_CODE_UNIT`.
const NEWLINE_CODE_UNIT: i32 = 10;

impl<T: DirectionalCaretMovementIntent> UpdateTextSelectionAction<T> {
    fn new(
        app: &mut App,
        state: Handle<EditableTextState>,
        get_text_boundary: GetTextBoundary,
        apply_text_boundary: ApplyTextBoundary,
        ignore_non_collapsed_selection: bool,
    ) -> Handle<UpdateTextSelectionAction<T>> {
        app.create(UpdateTextSelectionAction {
            action: ActionData::new(),
            state,
            ignore_non_collapsed_selection,
            is_expand: false,
            extent_at_index: false,
            get_text_boundary,
            apply_text_boundary,
            intent: PhantomData,
        })
    }

    /// Dart's named `isExpand` and `extentAtIndex` arguments.
    fn expanding(
        self: Handle<Self>,
        app: &mut App,
        extent_at_index: bool,
    ) -> Handle<UpdateTextSelectionAction<T>> {
        app.get_mut(self).is_expand = true;
        app.get_mut(self).extent_at_index = extent_at_index;
        self
    }

    /// Returns true iff the given position is at a wordwrap boundary in the
    /// upstream position.
    fn is_at_wordwrap_upstream(self: Handle<Self>, app: &mut App, position: TextPosition) -> bool {
        let state = app.get(self).state;
        let end = TextPosition::with_affinity(
            state
                .render_editable(app)
                .get_line_at_offset(app, position)
                .end(),
            TextAffinity::Upstream,
        );
        let text = state.value(app).text;
        end == position
            && end.offset != utf16_len(&text)
            && code_unit_at(&text, position.offset) != Some(NEWLINE_CODE_UNIT)
    }

    /// Returns true if the given position at a wordwrap boundary in the
    /// downstream position.
    fn is_at_wordwrap_downstream(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> bool {
        let state = app.get(self).state;
        let start = TextPosition::new(
            state
                .render_editable(app)
                .get_line_at_offset(app, position)
                .start(),
        );
        let text = state.value(app).text;
        start == position
            && start.offset != 0
            && code_unit_at(&text, position.offset - 1) != Some(NEWLINE_CODE_UNIT)
    }
}

impl<T: DirectionalCaretMovementIntent> Action for UpdateTextSelectionAction<T> {
    type Intent = T;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl<T: DirectionalCaretMovementIntent> ContextAction for UpdateTextSelectionAction<T> {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        _intent: &T,
        _context: Option<BuildContext>,
    ) -> bool {
        let state = app.get(self).state;
        if K_IS_WEB
            && state.widget(app).user_selection_enabled()
            && state.value(app).composing.is_valid()
        {
            return false;
        }
        state.value(app).selection.is_valid()
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        let selection = state.value(app).selection;
        debug_assert!(selection.is_valid());
        let context = context.expect("a selection move needs a context");

        let collapse_selection =
            intent.collapse_selection() || !state.widget(app).user_selection_enabled();
        if !selection.is_collapsed()
            && !app.get(self).ignore_non_collapsed_selection
            && collapse_selection
        {
            let offset = if intent.forward() {
                selection.end()
            } else {
                selection.start()
            };
            return Actions::invoke(
                app,
                context,
                &UpdateSelectionIntent::new(
                    state.value(app),
                    TextSelection::collapsed(offset, TextAffinity::Downstream),
                    SelectionChangedCause::Keyboard,
                ),
            );
        }

        let mut extent = selection.extent();
        // If continues_at_wrap is true and extent is at the relevant wordwrap, then move it
        // just to the other side of the wordwrap.
        if intent.continues_at_wrap() {
            if intent.forward() && self.is_at_wordwrap_upstream(app, extent) {
                extent = TextPosition::new(extent.offset);
            } else if !intent.forward() && self.is_at_wordwrap_downstream(app, extent) {
                extent = TextPosition::with_affinity(extent.offset, TextAffinity::Upstream);
            }
        }

        let is_expand = app.get(self).is_expand;
        let extent_at_index = app.get(self).extent_at_index;
        let should_target_base = is_expand
            && if intent.forward() {
                selection.base_offset > selection.extent_offset
            } else {
                selection.base_offset < selection.extent_offset
            };
        let boundary = (app.get(self).get_text_boundary)(state, app);
        let apply = app.get(self).apply_text_boundary;
        let new_extent = apply(
            state,
            app,
            if should_target_base {
                selection.base()
            } else {
                extent
            },
            intent.forward(),
            &*boundary,
        );
        let new_selection =
            if collapse_selection || (!is_expand && new_extent.offset == selection.base_offset) {
                TextSelection::from_position(new_extent)
            } else if is_expand {
                selection.expand_to(new_extent, extent_at_index || selection.is_collapsed())
            } else {
                selection.extend_to(new_extent)
            };

        let should_collapse_to_base = intent.collapse_at_reversal()
            && (selection.base_offset - selection.extent_offset)
                * (selection.base_offset - new_selection.extent_offset)
                < 0;
        let new_range = if should_collapse_to_base {
            TextSelection::from_position(selection.base())
        } else {
            new_selection
        };
        Actions::invoke(
            app,
            context,
            &UpdateSelectionIntent::new(
                state.value(app),
                new_range,
                SelectionChangedCause::Keyboard,
            ),
        )
    }
}

/// Moves the caret to the adjacent line or page, keeping the column across a run of moves.
///
/// Dart's `_UpdateTextSelectionVerticallyAction`.
struct UpdateTextSelectionVerticallyAction<T: DirectionalCaretMovementIntent> {
    action: ActionData,
    state: Handle<EditableTextState>,
    by_page: bool,
    intent: PhantomData<T>,
}

impl<T: DirectionalCaretMovementIntent> UpdateTextSelectionVerticallyAction<T> {
    fn new(
        app: &mut App,
        state: Handle<EditableTextState>,
        by_page: bool,
    ) -> Handle<UpdateTextSelectionVerticallyAction<T>> {
        app.create(UpdateTextSelectionVerticallyAction {
            action: ActionData::new(),
            state,
            by_page,
            intent: PhantomData,
        })
    }
}

impl<T: DirectionalCaretMovementIntent> Action for UpdateTextSelectionVerticallyAction<T> {
    type Intent = T;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl<T: DirectionalCaretMovementIntent> ContextAction for UpdateTextSelectionVerticallyAction<T> {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        _intent: &T,
        _context: Option<BuildContext>,
    ) -> bool {
        let state = app.get(self).state;
        if K_IS_WEB
            && state.widget(app).user_selection_enabled()
            && state.value(app).composing.is_valid()
        {
            return false;
        }
        state.value(app).selection.is_valid()
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        debug_assert!(state.value(app).selection.is_valid());
        let context = context.expect("a vertical move needs a context");

        let collapse_selection =
            intent.collapse_selection() || !state.widget(app).user_selection_enabled();
        let value = state.text_editing_value_for_text_layout_metrics(app);
        if !value.selection.is_valid() {
            return None;
        }

        let mut taken = app.get_mut(state).vertical_movement_run.take();
        if taken.as_mut().is_some_and(|run| !run.is_valid(app)) {
            taken = None;
            app.get_mut(state).run_selection = None;
        }

        let mut current_run = match taken {
            Some(run) => run,
            None => {
                let render_editable = state.render_editable(app);
                let extent = render_editable
                    .selection(app)
                    .expect("the render editable has a selection")
                    .extent();
                render_editable.start_vertical_caret_movement(app, extent)
            }
        };

        let should_move = if app.get(self).by_page {
            let height = state.render_editable(app).size(app).height();
            let offset = if intent.forward() { 1.0 } else { -1.0 } * height;
            current_run.move_by_offset(app, offset)
        } else if intent.forward() {
            current_run.move_next(app)
        } else {
            current_run.move_previous(app)
        };
        let new_extent = if should_move {
            current_run.current()
        } else if intent.forward() {
            TextPosition::new(utf16_len(&value.text))
        } else {
            TextPosition::new(0)
        };
        let new_selection = if collapse_selection {
            TextSelection::from_position(new_extent)
        } else {
            value.selection.extend_to(new_extent)
        };

        Actions::invoke(
            app,
            context,
            &UpdateSelectionIntent::new(value, new_selection, SelectionChangedCause::Keyboard),
        );
        if state.value(app).selection == new_selection {
            app.get_mut(state).vertical_movement_run = Some(current_run);
            app.get_mut(state).run_selection = Some(new_selection);
        }
        None
    }
}

/// A [`CallbackAction`] that a composing web field disables.
///
/// Dart's `_WebComposingDisablingCallbackAction`.
struct WebComposingDisablingCallbackAction<T: Intent> {
    action: ActionData,
    state: Handle<EditableTextState>,
    on_invoke: OnInvokeCallback<T>,
}

impl<T: Intent> WebComposingDisablingCallbackAction<T> {
    fn new(
        app: &mut App,
        state: Handle<EditableTextState>,
        on_invoke: OnInvokeCallback<T>,
    ) -> Handle<WebComposingDisablingCallbackAction<T>> {
        app.create(WebComposingDisablingCallbackAction {
            action: ActionData::new(),
            state,
            on_invoke,
        })
    }
}

impl<T: Intent> Action for WebComposingDisablingCallbackAction<T> {
    type Intent = T;
    crate::action_accessors!();

    fn is_enabled(self: Handle<Self>, app: &mut App, _intent: &T) -> bool {
        let state = app.get(self).state;
        // Dart falls through to `super.isActionEnabled`, which a CallbackAction leaves true.
        !(K_IS_WEB
            && state.widget(app).user_selection_enabled()
            && state.value(app).composing.is_valid())
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &T) -> Option<Rc<dyn Any>> {
        let on_invoke = Rc::clone(&app.get(self).on_invoke);
        on_invoke(app, intent)
    }
}

/// Dart's `_SelectAllAction`.
struct SelectAllAction {
    action: ActionData,
    state: Handle<EditableTextState>,
}

impl SelectAllAction {
    fn new(app: &mut App, state: Handle<EditableTextState>) -> Handle<SelectAllAction> {
        app.create(SelectAllAction {
            action: ActionData::new(),
            state,
        })
    }
}

impl Action for SelectAllAction {
    type Intent = SelectAllTextIntent;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl ContextAction for SelectAllAction {
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &SelectAllTextIntent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        if !state.widget(app).user_selection_enabled() {
            return None;
        }
        let context = context.expect("a select all needs a context");
        let value = state.value(app);
        let selection = TextSelection::new(0, utf16_len(&value.text));
        Actions::invoke(
            app,
            context,
            &UpdateSelectionIntent::new(value, selection, intent.cause),
        )
    }
}

/// Dart's `_CopySelectionAction`.
struct CopySelectionAction {
    action: ActionData,
    state: Handle<EditableTextState>,
}

impl CopySelectionAction {
    fn new(app: &mut App, state: Handle<EditableTextState>) -> Handle<CopySelectionAction> {
        app.create(CopySelectionAction {
            action: ActionData::new(),
            state,
        })
    }
}

impl Action for CopySelectionAction {
    type Intent = CopySelectionTextIntent;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl ContextAction for CopySelectionAction {
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &CopySelectionTextIntent,
        _context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        let selection = state.value(app).selection;
        if !selection.is_valid() || selection.is_collapsed() {
            return None;
        }
        if !state.widget(app).user_selection_enabled() {
            return None;
        }
        if intent.collapse_selection() {
            state.cut_selection(app, intent.cause());
        } else {
            state.copy_selection(app, intent.cause());
        }
        None
    }
}

/// Dart's `_PasteSelectionAction`.
struct PasteSelectionAction {
    action: ActionData,
    state: Handle<EditableTextState>,
}

impl PasteSelectionAction {
    fn new(app: &mut App, state: Handle<EditableTextState>) -> Handle<PasteSelectionAction> {
        app.create(PasteSelectionAction {
            action: ActionData::new(),
            state,
        })
    }
}

impl Action for PasteSelectionAction {
    type Intent = PasteTextIntent;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl ContextAction for PasteSelectionAction {
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &PasteTextIntent,
        _context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let state = app.get(self).state;
        if !state.widget(app).user_selection_enabled() {
            return None;
        }
        state.paste_text(app, intent.cause);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::AppCell;

    #[test]
    fn controller_starts_empty() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = TextEditingController::new(&mut app);
        assert_eq!(controller.text_value(&app), "");
        assert!(!controller.selection(&app).is_valid());
    }

    #[test]
    fn controller_set_text_clears_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = TextEditingController::new(&mut app);
        controller.set_text(&mut app, "hi");
        assert_eq!(controller.text_value(&app), "hi");
        assert!(!controller.selection(&app).is_valid());
    }

    #[test]
    fn controller_from_value_keeps_composing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = TextEditingController::from_value(
            &mut app,
            Some(
                TextEditingValue::new()
                    .text("abc")
                    .selection(TextSelection::collapsed(3, TextAffinity::Downstream))
                    .composing(TextRange::new(0, 3)),
            ),
        );
        assert!(controller.value(&app).is_composing_range_valid());
        assert_eq!(controller.value(&app).composing, TextRange::new(0, 3));
    }

    use inset_painting::PaintingBinding;

    use crate::test_harness::Harness;
    use crate::widgets::media_query::MediaQueryData;

    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn find_editable(app: &App, object: AnyRenderObject) -> Option<RenderHandle<RenderEditable>> {
        if let Some(editable) = object.downcast::<RenderEditable>(app) {
            return Some(editable);
        }
        let mut found = None;
        object.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find_editable(app, child);
            }
        });
        found
    }

    #[test]
    fn editable_text_builds_a_render_editable() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let controller = TextEditingController::new(&mut app).text(&mut app, "hello");
        let focus_node = FocusNode::new(&mut app);
        let editable = EditableText::new(
            controller,
            focus_node,
            TextStyle::new(),
            Color::from_argb(255, 0, 0, 0),
            Color::from_argb(255, 128, 128, 128),
        );
        let tree = MediaQuery::new(
            MediaQueryData::new(),
            Directionality::new(TextDirection::Ltr, editable),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);
        let root = harness.render_root(&app).as_object();
        let render_editable =
            find_editable(&app, root).expect("EditableText should mount a RenderEditable");
        let size = render_editable.size(&app);
        assert!(
            size.width() > 0.0 && size.height() > 0.0,
            "RenderEditable should have a size, got {size:?}"
        );
    }
}
