//! Flutter counterpart: material_ui's `selectable_text.dart`.
//!
//! `SelectableText` is a read-only [`EditableText`] over a private controller that builds the
//! widget's span; the state rebuilds that controller whenever the data or span changes.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{
    AppLifecycleState, BoxHeightStyle, BoxWidthStyle, Color, Offset, Radius, TargetPlatform,
    TextAlign, TextDirection, TextEditingValue, TextHeightBehavior, TextSelection,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, ListenableObject, Listener,
    OptionRetained, Ref,
};
use waft_gestures::{DragStartBehavior, GestureTapCallback, TapDragUpDetails};
use waft_painting::{InlineSpan, StrutStyle, TextScaler, TextSpan, TextStyle, TextWidthBasis};
use waft_rendering::HitTestBehavior;
use waft_scheduler::SchedulerBinding;
use waft_services::SelectionChangedCause;
use waft_widgets::{
    AnyTextEditingController, BuildContext, DefaultSelectionStyle, DefaultTextStyle, EditableText,
    EditableTextContextMenuBuilder, EditableTextState, FocusNode, FocusNodeLeaf, FocusNodeRef,
    GlobalKey, IntoWidget, KeyRef, MediaQuery, RepaintBoundary, ScrollBehaviorRef,
    ScrollPhysicsRef, SelectionChangedCallback, Semantics, State, StateData, StatefulWidget,
    TextEditingControllerData, TextEditingControllerLeaf, TextMagnifierConfiguration,
    TextSelectionControls, TextSelectionGestureDetectorBuilder,
    TextSelectionGestureDetectorBuilderBase, TextSelectionGestureDetectorBuilderData,
    TextSelectionGestureDetectorBuilderDelegate, ToolbarOptions, WidgetRef,
    debug_check_has_directionality, debug_check_has_media_query,
};

use waft_cupertino::{
    CupertinoColors, CupertinoTheme, cupertino_desktop_text_selection_handle_controls,
    cupertino_text_selection_handle_controls,
};

use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::desktop_text_selection::desktop_text_selection_handle_controls;
use crate::magnifier::TextMagnifier;
use crate::text_selection::material_text_selection_handle_controls;
use crate::theme::Theme;

/// An eyeballed value that moves the cursor slightly left of where it is
/// rendered for text on Android so its positioning more accurately matches the
/// native iOS text cursor positioning.
///
/// This value is in device pixels, not logical pixels as is typically used
/// throughout the codebase.
pub const IOS_HORIZONTAL_OFFSET: i32 = -2;

/// Dart's `_TextSpanEditingController`.
struct TextSpanEditingController {
    change_notifier: ChangeNotifierData,
    text_editing_controller: TextEditingControllerData,
    text_span: TextSpan,
}

impl TextSpanEditingController {
    fn new(app: &mut App, text_span: TextSpan) -> Ref<TextSpanEditingController> {
        let text = text_span.to_plain_text(false, true);
        app.create(TextSpanEditingController {
            change_notifier: ChangeNotifierData::new(),
            text_editing_controller: TextEditingControllerData::new(
                TextEditingValue::new().text(text),
            ),
            text_span,
        })
    }
}

impl ChangeNotifier for TextSpanEditingController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl TextEditingControllerLeaf for TextSpanEditingController {
    waft_widgets::text_editing_controller_accessors!();

    fn build_text_span(
        self: Handle<Self>,
        app: &App,
        _context: BuildContext,
        style: Option<TextStyle>,
        _with_composing: bool,
    ) -> TextSpan {
        // This does not care about composing.
        let mut span = TextSpan::new();
        if let Some(style) = style {
            span = span.style(style);
        }
        span.children(vec![app.get(self).text_span.clone().into_span()])
    }

    fn set_text(self: Handle<Self>, _app: &mut App, _new_text: impl Into<String>) {
        // This should never be reached.
        unimplemented!();
    }
}

impl Debug for TextSpanEditingController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_TextSpanEditingController")
            .field("text_span", &self.text_span)
            .finish_non_exhaustive()
    }
}

/// Dart's `_SelectableTextSelectionGestureDetectorBuilder`.
struct SelectableTextSelectionGestureDetectorBuilder {
    builder: TextSelectionGestureDetectorBuilderData,
    state: Handle<SelectableTextState>,
}

impl TextSelectionGestureDetectorBuilder for SelectableTextSelectionGestureDetectorBuilder {
    waft_widgets::text_selection_gesture_detector_builder_accessors!(builder);

    fn on_single_tap_up(self: Handle<Self>, app: &mut App, details: TapDragUpDetails) {
        if !self.builder_data(app).delegate.selection_enabled(app) {
            return;
        }
        TextSelectionGestureDetectorBuilderBase::on_single_tap_up(self, app, details);
        let state = app.get(self).state;
        if let Some(on_tap) = state.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
    }
}

/// A run of selectable text with a single style.
///
/// Selectable text can't receive keyboard focus by traversal: its focus node skips
/// traversal.
///
/// The [`SelectableText`] widget displays a string of text with a single style.
/// The string might break across multiple lines or might all be displayed on
/// the same line depending on the layout constraints.
///
/// The [`style`](Self::style) argument is optional. When omitted, the text will use the
/// style from the closest enclosing [`DefaultTextStyle`]. If the given style's
/// `TextStyle::inherit` property is true (the default), the given style will
/// be merged with the closest enclosing [`DefaultTextStyle`]. This merging
/// behavior is useful, for example, to make the text bold while using the
/// default font family and size.
///
/// [`SelectableText::rich`] shows text with several styles, as a [`TextSpan`].
///
/// To make [`SelectableText`] react to touch events, use callback [`on_tap`](Self::on_tap)
/// to achieve the desired behavior.
///
/// See also:
///
///  * `Text`, which is the non selectable version of this widget.
///  * `TextField`, which is the editable version of this widget.
///  * `SelectionArea`, which enables the selection of multiple `Text` widgets and of other
///    widgets.
#[allow(clippy::type_complexity)]
pub struct SelectableText {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// The text to display.
    ///
    /// This will be `None` if a [`text_span`](Self::text_span) is provided instead.
    pub data: Option<String>,

    /// The text to display as a [`TextSpan`].
    ///
    /// This will be `None` if [`data`](Self::data) is provided instead.
    pub text_span: Option<TextSpan>,

    /// Defines the focus for this widget.
    ///
    /// Text is only selectable when widget is focused.
    ///
    /// The [`focus_node`](Self::focus_node) is a long-lived object that's typically managed
    /// by a [`StatefulWidget`] parent.
    ///
    /// If null, this widget will create its own [`FocusNode`] with `FocusNode::skip_traversal`
    /// parameter set to `true`, which causes the widget to be skipped over during focus
    /// traversal.
    pub focus_node: Option<FocusNodeRef>,

    /// The style to use for the text.
    ///
    /// If null, defaults to [`DefaultTextStyle`] of context.
    pub style: Option<TextStyle>,

    /// {@macro flutter.widgets.editableText.strutStyle}
    pub strut_style: Option<StrutStyle>,

    /// {@macro flutter.widgets.editableText.textAlign}
    pub text_align: Option<TextAlign>,

    /// {@macro flutter.widgets.editableText.textDirection}
    pub text_direction: Option<TextDirection>,

    /// {@macro flutter.widgets.editableText.textScaleFactor}
    #[deprecated(
        note = "Use text_scaler instead. Use of textScaleFactor was deprecated in preparation for the upcoming nonlinear text scaling support. This feature was deprecated after v3.12.0-2.0.pre."
    )]
    pub text_scale_factor: Option<f64>,

    /// {@macro flutter.painting.textPainter.textScaler}
    pub text_scaler: Option<TextScaler>,

    /// {@macro flutter.widgets.editableText.autofocus}
    pub autofocus: bool,

    /// {@macro flutter.widgets.editableText.minLines}
    pub min_lines: Option<i32>,

    /// {@macro flutter.widgets.editableText.maxLines}
    pub max_lines: Option<i32>,

    /// {@macro flutter.widgets.editableText.showCursor}
    pub show_cursor: bool,

    /// {@macro flutter.widgets.editableText.cursorWidth}
    pub cursor_width: f64,

    /// {@macro flutter.widgets.editableText.cursorHeight}
    pub cursor_height: Option<f64>,

    /// {@macro flutter.widgets.editableText.cursorRadius}
    pub cursor_radius: Option<Radius>,

    /// The color of the cursor.
    ///
    /// The cursor indicates the current text insertion point.
    ///
    /// If this is null it will default to [`DefaultSelectionStyle::cursor_color`].
    /// If that is null, it will use `CupertinoThemeData::primary_color` on iOS and macOS, or
    /// `ColorScheme::primary` on other platforms.
    pub cursor_color: Option<Color>,

    /// The color to use when painting the selection.
    ///
    /// If this property is null, this widget gets the selection color from the
    /// [`DefaultSelectionStyle`], or else a 40% opacity of the primary colour.
    pub selection_color: Option<Color>,

    /// Controls how tall the selection highlight boxes are computed to be.
    ///
    /// See `BoxHeightStyle` for details on available styles.
    pub selection_height_style: Option<BoxHeightStyle>,

    /// Controls how wide the selection highlight boxes are computed to be.
    ///
    /// See `BoxWidthStyle` for details on available styles.
    pub selection_width_style: Option<BoxWidthStyle>,

    /// {@macro flutter.widgets.editableText.enableInteractiveSelection}
    pub enable_interactive_selection: bool,

    /// {@macro flutter.widgets.editableText.selectionControls}
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,

    /// {@macro flutter.widgets.scrollable.dragStartBehavior}
    pub drag_start_behavior: DragStartBehavior,

    /// Configuration of toolbar options.
    ///
    /// Paste and cut will be disabled regardless.
    ///
    /// If not set, select all and copy will be enabled by default.
    #[deprecated(
        note = "Use `context_menu_builder` instead. This feature was deprecated after v3.3.0-0.5.pre."
    )]
    pub toolbar_options: Option<ToolbarOptions>,

    /// Called when the user taps on this selectable text.
    ///
    /// The selectable text builds a `GestureDetector` to handle input events like tap,
    /// to trigger focus requests, to move the caret, adjust the selection, etc.
    /// Handling some of those events by wrapping the selectable text with a competing
    /// `GestureDetector` is problematic.
    ///
    /// To unconditionally handle taps, without interfering with the selectable text's
    /// internal gesture detector, provide this callback.
    ///
    /// To be notified when the text field gains or loses the focus, provide a
    /// [`focus_node`](Self::focus_node) and add a listener to that.
    ///
    /// To listen to arbitrary pointer events without competing with the
    /// selectable text's internal gesture detector, use a `Listener`.
    pub on_tap: Option<GestureTapCallback>,

    /// {@macro flutter.widgets.editableText.scrollPhysics}
    pub scroll_physics: Option<ScrollPhysicsRef>,

    /// {@macro flutter.widgets.editableText.scrollBehavior}
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// {@macro flutter.widgets.Text.semanticsLabel}
    pub semantics_label: Option<String>,

    /// {@macro dart.ui.textHeightBehavior}
    pub text_height_behavior: Option<TextHeightBehavior>,

    /// {@macro flutter.painting.textPainter.textWidthBasis}
    pub text_width_basis: Option<TextWidthBasis>,

    /// {@macro flutter.widgets.editableText.onSelectionChanged}
    pub on_selection_changed: Option<SelectionChangedCallback>,

    /// {@macro flutter.widgets.EditableText.contextMenuBuilder}
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,

    /// The configuration for the magnifier used when the text is selected.
    ///
    /// By default, builds a `CupertinoTextMagnifier` on iOS and [`TextMagnifier`] on
    /// Android, and builds nothing on all other platforms. To suppress the magnifier,
    /// consider passing [`TextMagnifierConfiguration::DISABLED`].
    pub magnifier_configuration: Option<TextMagnifierConfiguration>,
}

#[allow(deprecated)]
impl SelectableText {
    /// Creates a selectable text widget.
    ///
    /// If the [`style`](Self::style) argument is null, the text will use the style from the
    /// closest enclosing [`DefaultTextStyle`].
    pub fn new(data: impl Into<String>) -> SelectableText {
        SelectableText::with_content(Some(data.into()), None)
    }

    /// Creates a selectable text widget with a [`TextSpan`].
    ///
    /// The [`text_span`](Self::text_span) parameter must not be null and only contain
    /// [`TextSpan`] in `TextSpan::children`. Other type of `InlineSpan` is not allowed.
    pub fn rich(text_span: TextSpan) -> SelectableText {
        SelectableText::with_content(None, Some(text_span))
    }

    fn with_content(data: Option<String>, text_span: Option<TextSpan>) -> SelectableText {
        SelectableText {
            key: None,
            data,
            text_span,
            focus_node: None,
            style: None,
            strut_style: None,
            text_align: None,
            text_direction: None,
            text_scale_factor: None,
            text_scaler: None,
            autofocus: false,
            min_lines: None,
            max_lines: None,
            show_cursor: false,
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: None,
            cursor_color: None,
            selection_color: None,
            selection_height_style: None,
            selection_width_style: None,
            enable_interactive_selection: true,
            selection_controls: None,
            drag_start_behavior: DragStartBehavior::Start,
            toolbar_options: None,
            on_tap: None,
            scroll_physics: None,
            scroll_behavior: None,
            semantics_label: None,
            text_height_behavior: None,
            text_width_basis: None,
            on_selection_changed: None,
            context_menu_builder: Some(Rc::new(Self::default_context_menu_builder)),
            magnifier_configuration: None,
        }
    }

    /// Dart `SelectableText(key:)`.
    pub fn key(mut self, key: KeyRef) -> SelectableText {
        self.key = Some(key);
        self
    }

    /// Dart `SelectableText(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> SelectableText {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `SelectableText(style:)`.
    pub fn style(mut self, style: TextStyle) -> SelectableText {
        self.style = Some(style);
        self
    }

    /// Dart `SelectableText(strutStyle:)`.
    pub fn strut_style(mut self, strut_style: StrutStyle) -> SelectableText {
        self.strut_style = Some(strut_style);
        self
    }

    /// Dart `SelectableText(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> SelectableText {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `SelectableText(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> SelectableText {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `SelectableText(textScaleFactor:)`.
    #[deprecated(
        note = "Use text_scaler instead. Use of textScaleFactor was deprecated in preparation for the upcoming nonlinear text scaling support. This feature was deprecated after v3.12.0-2.0.pre."
    )]
    pub fn text_scale_factor(mut self, text_scale_factor: f64) -> SelectableText {
        self.text_scale_factor = Some(text_scale_factor);
        self
    }

    /// Dart `SelectableText(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> SelectableText {
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `SelectableText(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> SelectableText {
        self.show_cursor = show_cursor;
        self
    }

    /// Dart `SelectableText(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> SelectableText {
        self.autofocus = autofocus;
        self
    }

    /// Dart `SelectableText(toolbarOptions:)`.
    #[deprecated(
        note = "Use `context_menu_builder` instead. This feature was deprecated after v3.3.0-0.5.pre."
    )]
    pub fn toolbar_options(mut self, toolbar_options: ToolbarOptions) -> SelectableText {
        self.toolbar_options = Some(toolbar_options);
        self
    }

    /// Dart `SelectableText(minLines:)`.
    pub fn min_lines(mut self, min_lines: i32) -> SelectableText {
        debug_assert!(min_lines > 0);
        self.min_lines = Some(min_lines);
        self
    }

    /// Dart `SelectableText(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> SelectableText {
        debug_assert!(max_lines.is_none_or(|lines| lines > 0));
        self.max_lines = max_lines;
        self
    }

    /// Dart `SelectableText(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> SelectableText {
        self.cursor_width = cursor_width;
        self
    }

    /// Dart `SelectableText(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> SelectableText {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `SelectableText(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> SelectableText {
        self.cursor_radius = Some(cursor_radius);
        self
    }

    /// Dart `SelectableText(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: Color) -> SelectableText {
        self.cursor_color = Some(cursor_color);
        self
    }

    /// Dart `SelectableText(selectionColor:)`.
    pub fn selection_color(mut self, selection_color: Color) -> SelectableText {
        self.selection_color = Some(selection_color);
        self
    }

    /// Dart `SelectableText(selectionHeightStyle:)`.
    pub fn selection_height_style(
        mut self,
        selection_height_style: BoxHeightStyle,
    ) -> SelectableText {
        self.selection_height_style = Some(selection_height_style);
        self
    }

    /// Dart `SelectableText(selectionWidthStyle:)`.
    pub fn selection_width_style(mut self, selection_width_style: BoxWidthStyle) -> SelectableText {
        self.selection_width_style = Some(selection_width_style);
        self
    }

    /// Dart `SelectableText(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> SelectableText {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `SelectableText(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(
        mut self,
        enable_interactive_selection: bool,
    ) -> SelectableText {
        self.enable_interactive_selection = enable_interactive_selection;
        self
    }

    /// Dart `SelectableText(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> SelectableText {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `SelectableText(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> SelectableText {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `SelectableText(scrollPhysics:)`.
    pub fn scroll_physics(mut self, scroll_physics: ScrollPhysicsRef) -> SelectableText {
        self.scroll_physics = Some(scroll_physics);
        self
    }

    /// Dart `SelectableText(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> SelectableText {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `SelectableText(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: impl Into<String>) -> SelectableText {
        self.semantics_label = Some(semantics_label.into());
        self
    }

    /// Dart `SelectableText(textHeightBehavior:)`.
    pub fn text_height_behavior(
        mut self,
        text_height_behavior: TextHeightBehavior,
    ) -> SelectableText {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }

    /// Dart `SelectableText(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> SelectableText {
        self.text_width_basis = Some(text_width_basis);
        self
    }

    /// Dart `SelectableText(onSelectionChanged:)`.
    pub fn on_selection_changed(
        mut self,
        on_selection_changed: impl Fn(&mut App, TextSelection, Option<SelectionChangedCause>) + 'static,
    ) -> SelectableText {
        self.on_selection_changed = Some(Rc::new(on_selection_changed));
        self
    }

    /// Dart `SelectableText(contextMenuBuilder:)`; `None` builds no context menu.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: Option<EditableTextContextMenuBuilder>,
    ) -> SelectableText {
        self.context_menu_builder = context_menu_builder;
        self
    }

    /// Dart `SelectableText(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> SelectableText {
        self.magnifier_configuration = Some(magnifier_configuration);
        self
    }

    /// {@macro flutter.widgets.editableText.selectionEnabled}
    pub fn selection_enabled(&self) -> bool {
        self.enable_interactive_selection
    }

    fn default_context_menu_builder(
        app: &mut App,
        _context: BuildContext,
        editable_text_state: Handle<EditableTextState>,
    ) -> WidgetRef {
        AdaptiveTextSelectionToolbar::editable_text(app, editable_text_state).into_widget()
    }

    /// Dart's constructor asserts across fields.
    fn debug_check_constructor(&self) {
        debug_assert!(
            self.max_lines
                .is_none_or(|max| self.min_lines.is_none_or(|min| max >= min)),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(
            self.text_scaler.is_none() || self.text_scale_factor.is_none(),
            "textScaleFactor is deprecated and cannot be specified when textScaler is specified."
        );
    }

    /// Dart's `textSpan ?? TextSpan(text: data)`, the span the state's controller holds.
    fn effective_text_span(&self) -> TextSpan {
        self.text_span.clone().unwrap_or_else(|| {
            let mut span = TextSpan::new();
            if let Some(data) = &self.data {
                span = span.text(data.clone());
            }
            span
        })
    }
}

impl Debug for SelectableText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectableText")
            .field("data", &self.data)
            .field("semantics_label", &self.semantics_label)
            .field("focus_node", &self.focus_node)
            .field("style", &self.style)
            .field("autofocus", &self.autofocus)
            .field("show_cursor", &self.show_cursor)
            .field("min_lines", &self.min_lines)
            .field("max_lines", &self.max_lines)
            .field("text_align", &self.text_align)
            .field("text_direction", &self.text_direction)
            .field("cursor_width", &self.cursor_width)
            .field("selection_enabled", &self.selection_enabled())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectableText {
    type State = SelectableTextState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        SelectableText::debug_check_constructor(self);
    }

    fn create_state(&self) -> SelectableTextState {
        SelectableTextState {
            state: StateData::new(),
            controller: None,
            focus_node: None,
            show_selection_handles: false,
            selection_gesture_detector_builder: None,
            force_press_enabled: None,
            editable_text_key: GlobalKey::new(),
        }
    }
}

/// Dart's `_SelectableTextState`.
pub struct SelectableTextState {
    state: StateData<SelectableText>,
    controller: Option<Ref<TextSpanEditingController>>,
    focus_node: Option<Ref<FocusNode>>,
    show_selection_handles: bool,
    selection_gesture_detector_builder: Option<Ref<SelectableTextSelectionGestureDetectorBuilder>>,
    /// Dart's `late bool forcePressEnabled`, which `build` sets.
    force_press_enabled: Option<bool>,
    editable_text_key: GlobalKey,
}

impl SelectableTextState {
    fn editable_text(self: Handle<Self>, app: &mut App) -> Option<Handle<EditableTextState>> {
        let key = app.get(self).editable_text_key.clone();
        key.current_state::<EditableTextState>(app)
    }

    fn controller(self: Handle<Self>, app: &App) -> Handle<TextSpanEditingController> {
        app.get(self)
            .controller
            .handle()
            .expect("created in init_state")
    }

    fn effective_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            return focus_node;
        }
        if let Some(focus_node) = app.get(self).focus_node.as_ref() {
            return focus_node.erase();
        }
        let focus_node = FocusNode::new(app);
        focus_node.set_skip_traversal(app, true);
        app.get_mut(self).focus_node = Some(focus_node.clone());
        focus_node.into_erased()
    }

    fn selection_gesture_detector_builder(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<SelectableTextSelectionGestureDetectorBuilder> {
        app.get(self)
            .selection_gesture_detector_builder
            .handle()
            .expect("created in init_state")
    }

    fn create_controller(self: Handle<Self>, app: &mut App) {
        let text_span = self.widget(app).effective_text_span();
        let controller = TextSpanEditingController::new(app, text_span);
        app.get_mut(self).controller = Some(controller.clone());
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::on_controller_changed),
        );
    }

    fn on_controller_changed(self: Handle<Self>, app: &mut App) {
        let has_focus = self.effective_focus_node(app).has_focus(app);
        let show_selection_handles =
            !has_focus || !self.controller(app).selection(app).is_collapsed();
        if show_selection_handles == app.get(self).show_selection_handles {
            return;
        }
        self.set_state(app, |state| {
            state.show_selection_handles = show_selection_handles;
        });
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        if !self.effective_focus_node(app).has_focus(app)
            && SchedulerBinding::lifecycle_state(app) == Some(AppLifecycleState::Resumed)
        {
            // We should only clear the selection when this SelectableText loses
            // focus while the application is currently running. It is possible
            // that the application is not currently running, for example on desktop
            // platforms, clicking on a different window switches the focus to
            // the new window causing the Flutter application to go inactive. In this
            // case we want to retain the selection so it remains when we return to
            // the Flutter application.
            let controller = self.controller(app);
            let text = controller.value(app).text.clone();
            controller.set_value(app, TextEditingValue::new().text(text));
        }
    }

    fn handle_selection_changed(
        self: Handle<Self>,
        app: &mut App,
        selection: TextSelection,
        cause: Option<SelectionChangedCause>,
    ) {
        let will_show_selection_handles = self.should_show_selection_handles(app, cause);
        if will_show_selection_handles != app.get(self).show_selection_handles {
            self.set_state(app, |state| {
                state.show_selection_handles = will_show_selection_handles;
            });
        }

        if let Some(on_selection_changed) = self.widget(app).on_selection_changed.clone() {
            on_selection_changed(app, selection, cause);
        }

        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                if cause == Some(SelectionChangedCause::LongPress)
                    && let Some(editable_text) = self.editable_text(app)
                {
                    editable_text.bring_into_view(app, selection.base());
                }
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                // Do nothing.
            }
        }
    }

    fn handle_selection_handle_tapped(self: Handle<Self>, app: &mut App) {
        if self.controller(app).selection(app).is_collapsed() {
            self.editable_text(app)
                .expect("the EditableText is mounted")
                .toggle_toolbar(app, true);
        }
    }

    fn should_show_selection_handles(
        self: Handle<Self>,
        app: &App,
        cause: Option<SelectionChangedCause>,
    ) -> bool {
        // When the text field is activated by something that doesn't trigger the
        // selection overlay, we shouldn't show the handles either.
        if !self
            .selection_gesture_detector_builder(app)
            .should_show_selection_toolbar(app)
        {
            return false;
        }

        let controller = self.controller(app);
        if controller.selection(app).is_collapsed() {
            return false;
        }

        if cause == Some(SelectionChangedCause::Keyboard) {
            return false;
        }

        if cause == Some(SelectionChangedCause::LongPress) {
            return true;
        }

        if !controller.text_value(app).is_empty() {
            return true;
        }

        false
    }
}

impl State for SelectableTextState {
    type Widget = SelectableText;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let builder = app.create(SelectableTextSelectionGestureDetectorBuilder {
            builder: TextSelectionGestureDetectorBuilderData::new(
                self.as_text_selection_gesture_detector_builder_delegate(),
            ),
            state: self,
        });
        app.get_mut(self).selection_gesture_detector_builder = Some(builder);
        self.create_controller(app);
        let focus_node = self.effective_focus_node(app);
        focus_node.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SelectableText) {
        let widget = self.widget(app);
        let text_span_changed = match (&widget.text_span, &old_widget.text_span) {
            (None, None) => false,
            (Some(new), Some(old)) => !new.eq_span(old),
            _ => true,
        };
        if widget.data != old_widget.data || text_span_changed {
            let controller = self.controller(app);
            controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_controller_changed),
            );
            controller.as_controller().dispose(app);
            self.create_controller(app);
        }
        if self.widget(app).focus_node != old_widget.focus_node {
            if let Some(old_focus) = old_widget
                .focus_node
                .clone()
                .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            {
                old_focus.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
            if let Some(new_focus) = self
                .widget(app)
                .focus_node
                .clone()
                .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            {
                new_focus.add_listener(
                    app,
                    Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
        }
        let has_focus = self.effective_focus_node(app).has_focus(app);
        let show_selection_handles =
            !(has_focus && self.controller(app).selection(app).is_collapsed());
        app.get_mut(self).show_selection_handles = show_selection_handles;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let focus_node = self.effective_focus_node(app);
        focus_node.remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        if let Some(focus_node) = app.get(self).focus_node.handle() {
            focus_node.dispose(app);
        }
        self.controller(app).as_controller().dispose(app);
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // TODO(garyq): Assert to block WidgetSpans from being used here are removed,
        // but we still do not yet have nice handling of things like carets, clipboard,
        // and other features. We should add proper support. Currently, caret handling
        // is blocked on SkParagraph switch and https://github.com/flutter/engine/pull/27010
        // should be landed in SkParagraph after the switch is complete.
        debug_assert!(debug_check_has_media_query(app, context));
        debug_assert!(debug_check_has_directionality(
            app, context, None, None, None
        ));
        debug_assert!(
            !self
                .widget(app)
                .style
                .as_ref()
                .is_some_and(|style| !style.inherit
                    && (style.font_size.is_none() || style.text_baseline.is_none())),
            "inherit false style must supply fontSize and textBaseline"
        );

        let theme = Theme::of(app, context);
        let selection_style = DefaultSelectionStyle::of(app, context);
        let focus_node = self.effective_focus_node(app);

        let widget = self.widget(app);
        let mut text_selection_controls = widget.selection_controls.clone();
        let paint_cursor_above_text;
        let cursor_opacity_animates;
        let mut cursor_offset = None;
        let cursor_color: Color;
        let selection_color: Color;
        let mut cursor_radius = widget.cursor_radius;
        let widget_cursor_color = widget.cursor_color;
        let style_cursor_color = selection_style
            .cursor_color
            .as_ref()
            .map(|color| color.color());
        let style_selection_color = selection_style
            .selection_color
            .as_ref()
            .map(|color| color.color());

        let force_press_enabled;
        match theme.platform {
            TargetPlatform::IOS => {
                let cupertino_theme = CupertinoTheme::of(app, context);
                force_press_enabled = true;
                text_selection_controls
                    .get_or_insert_with(cupertino_text_selection_handle_controls);
                paint_cursor_above_text = true;
                cursor_opacity_animates = true;
                cursor_color = widget_cursor_color
                    .or(style_cursor_color)
                    .unwrap_or_else(|| cupertino_theme.primary_color().color());
                selection_color = style_selection_color
                    .unwrap_or_else(|| cupertino_theme.primary_color().color().with_opacity(0.40));
                cursor_radius.get_or_insert(Radius::circular(2.0));
                cursor_offset = Some(Offset::new(
                    f64::from(IOS_HORIZONTAL_OFFSET)
                        / MediaQuery::device_pixel_ratio_of(app, context),
                    0.0,
                ));
            }
            TargetPlatform::MacOS => {
                let cupertino_theme = CupertinoTheme::of(app, context);
                force_press_enabled = false;
                text_selection_controls
                    .get_or_insert_with(cupertino_desktop_text_selection_handle_controls);
                paint_cursor_above_text = true;
                cursor_opacity_animates = true;
                cursor_color = widget_cursor_color
                    .or(style_cursor_color)
                    .unwrap_or_else(|| cupertino_theme.primary_color().color());
                selection_color = style_selection_color
                    .unwrap_or_else(|| cupertino_theme.primary_color().color().with_opacity(0.40));
                cursor_radius.get_or_insert(Radius::circular(2.0));
                cursor_offset = Some(Offset::new(
                    f64::from(IOS_HORIZONTAL_OFFSET)
                        / MediaQuery::device_pixel_ratio_of(app, context),
                    0.0,
                ));
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia => {
                force_press_enabled = false;
                text_selection_controls.get_or_insert_with(material_text_selection_handle_controls);
                paint_cursor_above_text = false;
                cursor_opacity_animates = false;
                cursor_color = widget_cursor_color
                    .or(style_cursor_color)
                    .unwrap_or(theme.color_scheme.primary);
                selection_color = style_selection_color
                    .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
            }
            TargetPlatform::Linux | TargetPlatform::Windows => {
                force_press_enabled = false;
                text_selection_controls.get_or_insert_with(desktop_text_selection_handle_controls);
                paint_cursor_above_text = false;
                cursor_opacity_animates = false;
                cursor_color = widget_cursor_color
                    .or(style_cursor_color)
                    .unwrap_or(theme.color_scheme.primary);
                selection_color = style_selection_color
                    .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
            }
        }
        app.get_mut(self).force_press_enabled = Some(force_press_enabled);

        let default_text_style = DefaultTextStyle::of(app, context);
        let widget = self.widget(app);
        let controller = app
            .get(self)
            .controller
            .clone()
            .expect("created in init_state");
        let mut effective_text_style = widget.style.clone();
        if effective_text_style
            .as_ref()
            .is_none_or(|style| style.inherit)
        {
            let span_style = app.get(&controller).text_span.style.clone();
            effective_text_style = Some(
                default_text_style
                    .style
                    .merge(widget.style.as_ref().or(span_style.as_ref())),
            );
        }
        let effective_scaler = widget
            .text_scaler
            .clone()
            .or_else(|| widget.text_scale_factor.map(TextScaler::linear));

        let this = self;
        let mut editable = EditableText::new(
            controller.erase::<AnyTextEditingController>(),
            focus_node.clone(),
            effective_text_style.expect("filled above"),
            cursor_color,
            CupertinoColors::INACTIVE_GRAY.color(),
        )
        .key(app.get(self).editable_text_key.clone().into())
        .read_only(true)
        .text_width_basis(
            widget
                .text_width_basis
                .unwrap_or(default_text_style.text_width_basis),
        )
        .show_selection_handles(app.get(self).show_selection_handles)
        .show_cursor(widget.show_cursor)
        .text_align(
            widget
                .text_align
                .or(default_text_style.text_align)
                .unwrap_or(TextAlign::Start),
        )
        .autofocus(widget.autofocus)
        .force_line(false)
        .max_lines(widget.max_lines.or(default_text_style.max_lines))
        .selection_color(widget.selection_color.unwrap_or(selection_color))
        .on_selection_changed(move |app, selection, cause| {
            this.handle_selection_changed(app, selection, cause);
        })
        .on_selection_handle_tapped(Listener::handle_method(
            this,
            Self::handle_selection_handle_tapped,
        ))
        .renderer_ignores_pointer(true)
        .cursor_width(widget.cursor_width)
        .cursor_opacity_animates(cursor_opacity_animates)
        .paint_cursor_above_text(paint_cursor_above_text)
        .enable_interactive_selection(widget.enable_interactive_selection)
        .magnifier_configuration(
            widget
                .magnifier_configuration
                .clone()
                .unwrap_or_else(|| TextMagnifier::adaptive_magnifier_configuration(app)),
        )
        .drag_start_behavior(widget.drag_start_behavior);
        if let Some(toolbar_options) = widget.toolbar_options {
            editable = editable.toolbar_options(toolbar_options);
        }
        if let Some(text_height_behavior) = widget
            .text_height_behavior
            .or(default_text_style.text_height_behavior)
        {
            editable = editable.text_height_behavior(text_height_behavior);
        }
        if let Some(text_direction) = widget.text_direction {
            editable = editable.text_direction(text_direction);
        }
        if let Some(effective_scaler) = effective_scaler {
            editable = editable.text_scaler(effective_scaler);
        }
        if let Some(min_lines) = widget.min_lines {
            editable = editable.min_lines(min_lines);
        }
        if let Some(strut_style) = widget.strut_style.clone() {
            editable = editable.strut_style(strut_style);
        }
        if widget.selection_enabled()
            && let Some(text_selection_controls) = text_selection_controls
        {
            editable = editable.selection_controls(text_selection_controls);
        }
        if let Some(cursor_height) = widget.cursor_height {
            editable = editable.cursor_height(cursor_height);
        }
        if let Some(cursor_radius) = cursor_radius {
            editable = editable.cursor_radius(cursor_radius);
        }
        if let Some(selection_height_style) = widget.selection_height_style {
            editable = editable.selection_height_style(selection_height_style);
        }
        if let Some(selection_width_style) = widget.selection_width_style {
            editable = editable.selection_width_style(selection_width_style);
        }
        if let Some(cursor_offset) = cursor_offset {
            editable = editable.cursor_offset(cursor_offset);
        }
        if let Some(scroll_physics) = widget.scroll_physics.clone() {
            editable = editable.scroll_physics(scroll_physics);
        }
        if let Some(scroll_behavior) = widget.scroll_behavior.clone() {
            editable = editable.scroll_behavior(scroll_behavior);
        }
        if let Some(context_menu_builder) = widget.context_menu_builder.clone() {
            editable = editable.context_menu_builder(context_menu_builder);
        }
        let child = RepaintBoundary::new().child(editable);

        let semantics_label = self.widget(app).semantics_label.clone();
        let mut semantics = Semantics::new()
            .exclude_semantics(semantics_label.is_some())
            .on_long_press(Rc::new(move |app| {
                this.effective_focus_node(app).request_focus(app, None);
            }));
        if let Some(semantics_label) = semantics_label {
            semantics = semantics.label(semantics_label);
        }
        let builder = self.selection_gesture_detector_builder(app);
        semantics
            .child(TextSelectionGestureDetectorBuilder::build_gesture_detector(
                builder,
                app,
                None,
                Some(HitTestBehavior::Translucent),
                child.into_widget(),
            ))
            .into_widget()
    }
}

impl TextSelectionGestureDetectorBuilderDelegate for SelectableTextState {
    fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).editable_text_key.clone()
    }

    fn force_press_enabled(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .force_press_enabled
            .expect("build sets force_press_enabled")
    }

    fn selection_enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).selection_enabled()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_cupertino::{CupertinoButton, cupertino_text_selection_controls};
    use waft_embedder::Size;
    use waft_embedder::{TextAffinity, TextPosition};
    use waft_foundation::UniqueKey;
    use waft_rendering::{RenderBox as _, RenderEditable};
    use waft_services::{
        LogicalKeyboardKey, PhysicalKeyboardKey, SystemMouseCursors, TextInputClient,
    };
    use waft_test::{MockClipboard, TestPlatform};
    use waft_widgets::{
        AnyElement, AnyTextEditingController, Center, Column, DefaultTextStyle, Focus,
        FocusNodeLeaf, IntoWidget, MediaQueryData, ScrollBehaviorRef, SizedBox, Text,
    };

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_n_widgets,
        finds_nothing, finds_one_widget, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::test_support::material_app;
    use crate::text_button::TextButton;
    use crate::text_selection::material_text_selection_controls;
    use crate::theme_data::ThemeData;

    const K_THREE_LINES: &str =
        "First line of text is\nSecond line goes until\nThird line of stuff";
    const K_MORE_THAN_FOUR_LINES: &str = "First line of text is\nSecond line goes until\nThird line of stuff\nFourth line won't display and ends at";
    const ATWATER: &str = "Atwater Peel Sherbrooke Bonaventure";

    /// A tester on `platform` whose host clipboard the case reads back.
    fn tester_with_clipboard(platform: TargetPlatform) -> (WidgetTester, MockClipboard) {
        let clipboard = MockClipboard::new();
        let tester = WidgetTester::with_platform(
            TestPlatform::new()
                .on(platform)
                .with_clipboard(Rc::new(clipboard.clone())),
        );
        (tester, clipboard)
    }

    /// Dart's `MaterialApp(home: Material(child: child))`.
    fn material_home<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        let theme = ThemeData::new(&tester.cell.borrow());
        material_app(theme, Material::new().child(child).into_widget())
    }

    /// Dart's `overlay(child: child)`: the child centred on a `Material` under the app's
    /// overlay.
    fn overlay<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        let theme = ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow());
        material_app(
            theme,
            Center::new()
                .child(Material::new().child(child))
                .into_widget(),
        )
    }

    fn editable_elements(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<EditableText>().evaluate(tester)
    }

    fn controller_at(
        tester: &WidgetTester,
        index: usize,
    ) -> waft_widgets::TextEditingControllerRef {
        tester.widget(editable_elements(tester)[index], |widget: &EditableText| {
            widget.controller.clone()
        })
    }

    fn controller(tester: &WidgetTester) -> AnyTextEditingController {
        controller_at(tester, 0).handle()
    }

    fn selection(tester: &WidgetTester) -> TextSelection {
        controller(tester).selection(&tester.cell.borrow())
    }

    fn editable_state(tester: &WidgetTester) -> Handle<EditableTextState> {
        let element = editable_elements(tester)[0];
        element
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    fn render_editable_at(
        tester: &WidgetTester,
        index: usize,
    ) -> waft_rendering::RenderHandle<RenderEditable> {
        tester.all::<RenderEditable>()[index]
    }

    /// Dart's `textOffsetToPosition(tester, offset)`.
    fn text_offset_to_position(tester: &WidgetTester, offset: i32) -> Offset {
        text_offset_to_position_at(tester, offset, 0)
    }

    fn text_offset_to_position_at(tester: &WidgetTester, offset: i32, index: usize) -> Offset {
        let editable = render_editable_at(tester, index);
        let mut app = tester.cell.borrow_mut();
        let endpoints = editable.get_endpoints_for_selection(
            &mut app,
            TextSelection::collapsed(offset, TextAffinity::Downstream),
        );
        assert_eq!(endpoints.len(), 1);
        editable
            .as_box()
            .local_to_global(&app, endpoints[0].point, None)
            + Offset::new(0.0, -2.0)
    }

    /// Dart's `tester.getTopLeft(find.byType(SelectableText)) + Offset(dx, 5.0)`.
    fn selectable_text_start_plus(tester: &WidgetTester, dx: f64) -> Offset {
        top_left_of_selectable_text(tester, 0) + Offset::new(dx, 5.0)
    }

    fn top_left_of_selectable_text(tester: &WidgetTester, index: usize) -> Offset {
        let element = find::by_type::<SelectableText>().evaluate(tester)[index];
        tester.get_rect(element).top_left()
    }

    fn center_of_selectable_text(tester: &WidgetTester) -> Offset {
        let element = find::by_type::<SelectableText>().evaluate(tester)[0];
        tester.get_rect(element).center()
    }

    fn selection_overlay(tester: &WidgetTester) -> Handle<waft_widgets::TextSelectionOverlay> {
        let state = editable_state(tester);
        state
            .selection_overlay(&tester.cell.borrow())
            .expect("a selection overlay")
    }

    fn handles_are_visible(tester: &WidgetTester) -> bool {
        selection_overlay(tester).handles_are_visible(&tester.cell.borrow())
    }

    fn toolbar_is_visible(tester: &WidgetTester) -> bool {
        selection_overlay(tester).toolbar_is_visible(&tester.cell.borrow())
    }

    fn physical_of(logical: LogicalKeyboardKey) -> PhysicalKeyboardKey {
        [
            (
                LogicalKeyboardKey::SHIFT_LEFT,
                PhysicalKeyboardKey::SHIFT_LEFT,
            ),
            (
                LogicalKeyboardKey::CONTROL_LEFT,
                PhysicalKeyboardKey::CONTROL_LEFT,
            ),
            (
                LogicalKeyboardKey::CONTROL_RIGHT,
                PhysicalKeyboardKey::CONTROL_RIGHT,
            ),
            (
                LogicalKeyboardKey::ARROW_LEFT,
                PhysicalKeyboardKey::ARROW_LEFT,
            ),
            (
                LogicalKeyboardKey::ARROW_RIGHT,
                PhysicalKeyboardKey::ARROW_RIGHT,
            ),
            (LogicalKeyboardKey::ARROW_UP, PhysicalKeyboardKey::ARROW_UP),
            (
                LogicalKeyboardKey::ARROW_DOWN,
                PhysicalKeyboardKey::ARROW_DOWN,
            ),
            (LogicalKeyboardKey::KEY_A, PhysicalKeyboardKey::KEY_A),
            (LogicalKeyboardKey::KEY_C, PhysicalKeyboardKey::KEY_C),
        ]
        .into_iter()
        .find_map(|(l, p)| (l == logical).then_some(p))
        .expect("a key the cases press")
    }

    /// Dart's `expectMaterialSelectionToolbar()`.
    fn expect_material_selection_toolbar(tester: &WidgetTester, platform: TargetPlatform) {
        if platform == TargetPlatform::Android {
            expect(tester, &find::by_type::<TextButton>(), finds_n_widgets(3));
            expect(tester, &find::text("Copy"), finds_one_widget());
            expect(tester, &find::text("Share"), finds_one_widget());
            expect(tester, &find::text("Select all"), finds_one_widget());
        } else {
            expect(tester, &find::by_type::<TextButton>(), finds_n_widgets(2));
            expect(tester, &find::text("Copy"), finds_one_widget());
            expect(tester, &find::text("Select all"), finds_one_widget());
        }
    }

    /// Dart's `expectCupertinoSelectionToolbar()`.
    fn expect_cupertino_selection_toolbar(tester: &WidgetTester, platform: TargetPlatform) {
        if platform == TargetPlatform::IOS {
            expect(
                tester,
                &find::by_type::<CupertinoButton>(),
                finds_n_widgets(4),
            );
            expect(tester, &find::text("Copy"), finds_one_widget());
            expect(tester, &find::text("Look Up"), finds_one_widget());
            expect(tester, &find::text("Search Web"), finds_one_widget());
            expect(tester, &find::text("Share..."), finds_one_widget());
        } else {
            expect(
                tester,
                &find::by_type::<CupertinoButton>(),
                finds_one_widget(),
            );
            expect(tester, &find::text("Copy"), finds_one_widget());
        }
    }

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    #[test]
    fn has_expected_defaults() {
        let selectable_text = SelectableText::new("selectable text");
        assert!(!selectable_text.show_cursor);
        assert!(!selectable_text.autofocus);
        assert_eq!(
            selectable_text.drag_start_behavior,
            DragStartBehavior::Start
        );
        assert_eq!(selectable_text.cursor_width, 2.0);
        assert!(selectable_text.cursor_height.is_none());
        assert!(selectable_text.enable_interactive_selection);
    }

    #[test]
    fn rich_selectable_text_holds_its_span_and_no_data() {
        let selectable_text = SelectableText::rich(TextSpan::new().children(vec![
            TextSpan::new().text("First line!").into_span(),
            TextSpan::new().text("Second line!\n").into_span(),
        ]));
        assert!(selectable_text.data.is_none());
        assert!(selectable_text.text_span.is_some());
        assert!(!selectable_text.show_cursor);
        assert!(selectable_text.enable_interactive_selection);
    }

    #[test]
    fn uses_default_selection_style_for_selection_and_cursor_colors_if_provided() {
        test_widgets(|tester| {
            let selection_color = Colors::ORANGE.primary();
            let cursor_color = Colors::RED.primary();
            let root = material_home(
                tester,
                DefaultSelectionStyle::new(SelectableText::new("text"))
                    .selection_color(selection_color)
                    .cursor_color(cursor_color),
            );
            tester.pump_widget(root);
            tester.pump();
            let (selection, cursor) = tester
                .widget::<EditableText, _>(editable_elements(tester)[0], |widget| {
                    (widget.selection_color, widget.cursor_color)
                });
            assert_eq!(selection, Some(selection_color));
            assert_eq!(cursor, cursor_color);
        });
    }

    #[test]
    fn selectable_text_can_have_custom_selection_color() {
        test_widgets(|tester| {
            let selection_color = Colors::ORANGE.primary();
            let default_selection_color = Colors::RED.primary();
            let root = material_home(
                tester,
                DefaultSelectionStyle::new(
                    SelectableText::new("text").selection_color(selection_color),
                )
                .selection_color(default_selection_color),
            );
            tester.pump_widget(root);
            tester.pump();
            let selection = tester
                .widget::<EditableText, _>(editable_elements(tester)[0], |w| w.selection_color);
            assert_eq!(selection, Some(selection_color));
        });
    }

    #[test]
    fn can_switch_between_text_height_behavior() {
        test_widgets(|tester| {
            let text_height_behavior = TextHeightBehavior {
                apply_height_to_first_ascent: false,
                apply_height_to_last_descent: false,
                ..TextHeightBehavior::default()
            };
            let root = overlay(tester, SelectableText::new("selectable text"));
            tester.pump_widget(root);
            let editable = render_editable_at(tester, 0);
            assert_eq!(editable.text_height_behavior(&tester.cell.borrow()), None);

            let root = overlay(
                tester,
                SelectableText::new("selectable text").text_height_behavior(text_height_behavior),
            );
            tester.pump_widget(root);
            let editable = render_editable_at(tester, 0);
            assert_eq!(
                editable.text_height_behavior(&tester.cell.borrow()),
                Some(text_height_behavior)
            );
        });
    }

    #[test]
    fn cursor_blinks_when_show_cursor_is_true() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("some text").show_cursor(true));
            tester.pump_widget(root);
            let at = center_of_selectable_text(tester);
            tester.tap_at(at, 1);

            let editable_text = editable_state(tester);
            let visible = |tester: &WidgetTester| {
                editable_text.cursor_currently_visible(&tester.cell.borrow())
            };
            let interval = EditableTextState::cursor_blink_interval();

            // Check that the cursor visibility toggles after each blink interval.
            let initial_show_cursor = visible(tester);
            tester.pump_by(interval);
            assert_eq!(visible(tester), !initial_show_cursor);
            tester.pump_by(interval);
            assert_eq!(visible(tester), initial_show_cursor);
            tester.pump_by(interval / 10);
            assert_eq!(visible(tester), initial_show_cursor);
            tester.pump_by(interval);
            assert_eq!(visible(tester), !initial_show_cursor);
            tester.pump_by(interval);
            assert_eq!(visible(tester), initial_show_cursor);
        });
    }

    #[test]
    fn caret_position_is_updated_on_tap() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("abc def ghi"));
            tester.pump_widget(root);
            assert_eq!(selection(tester).base_offset, -1);
            assert_eq!(selection(tester).extent_offset, -1);

            // Tap to reposition the caret.
            let tap_index = 4;
            let e_pos = text_offset_to_position(tester, tap_index);
            tester.tap_at(e_pos, 1);
            tester.pump();

            assert_eq!(selection(tester).base_offset, tap_index);
            assert_eq!(selection(tester).extent_offset, tap_index);
        });
    }

    #[test]
    fn enable_interactive_selection_false_tap() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                SelectableText::new("abc def ghi").enable_interactive_selection(false),
            );
            tester.pump_widget(root);
            assert_eq!(selection(tester).base_offset, -1);
            assert_eq!(selection(tester).extent_offset, -1);

            // Tap would ordinarily reposition the caret.
            let e_pos = text_offset_to_position(tester, 4);
            tester.tap_at(e_pos, 1);
            tester.pump();

            assert_eq!(selection(tester).base_offset, -1);
            assert_eq!(selection(tester).extent_offset, -1);
        });
    }

    #[test]
    fn enable_interactive_selection_false_long_press() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                SelectableText::new("abc def ghi").enable_interactive_selection(false),
            );
            tester.pump_widget(root);
            assert_eq!(selection(tester).base_offset, -1);
            assert_eq!(selection(tester).extent_offset, -1);

            // Long press the 'e' to select 'def'.
            let e_pos = text_offset_to_position(tester, 5);
            let mut gesture = TestPointer::touch(7);
            tester.down(&mut gesture, e_pos);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump();

            assert!(selection(tester).is_collapsed());
            assert_eq!(selection(tester).base_offset, -1);
            assert_eq!(selection(tester).extent_offset, -1);
        });
    }

    #[test]
    fn can_long_press_to_select() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("abc def ghi"));
            tester.pump_widget(root);
            assert!(selection(tester).is_collapsed());

            // Long press the 'e' to select 'def'.
            let e_pos = text_offset_to_position(tester, 5);
            tester.long_press_at(e_pos, 1);
            tester.pump();

            // 'def' is selected.
            assert_eq!(selection(tester).base_offset, 4);
            assert_eq!(selection(tester).extent_offset, 7);

            // Tapping elsewhere immediately collapses and moves the cursor.
            let at = text_offset_to_position(tester, 9);
            tester.tap_at(at, 2);
            tester.pump();

            assert!(selection(tester).is_collapsed());
            assert_eq!(selection(tester).base_offset, 9);
        });
    }

    #[test]
    fn mouse_long_press_is_just_like_a_tap() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("abc def ghi"));
            tester.pump_widget(root);

            // Long press the 'e' using a mouse device.
            let e_index = 5;
            let e_pos = text_offset_to_position(tester, e_index);
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, e_pos);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump();

            // The cursor is placed just like a regular tap.
            assert_eq!(selection(tester).base_offset, e_index);
            assert_eq!(selection(tester).extent_offset, e_index);
        });
    }

    #[test]
    fn selectable_text_has_adaptive_size() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("s"));
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester),
                waft_embedder::Size::new(17.0, 14.0)
            );

            let root = overlay(tester, SelectableText::new("very very long"));
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester),
                waft_embedder::Size::new(199.0, 14.0)
            );
        });
    }

    #[test]
    #[allow(deprecated)]
    fn can_scale_with_text_scale_factor() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("selectable text"));
            tester.pump_widget(root);
            assert_eq!(selectable_text_size(tester).height(), 14.0);

            let root = overlay(
                tester,
                SelectableText::new("selectable text").text_scale_factor(1.9),
            );
            tester.pump_widget(root);
            assert_eq!(selectable_text_size(tester).height(), 27.0);
        });
    }

    #[test]
    fn can_switch_between_text_width_basis() {
        test_widgets(|tester| {
            const TEXT: &str = "I can face roll keyboardkeyboardaszzaaaaszzaaaaszzaaaaszzaaaa";
            let root = overlay(
                tester,
                SelectableText::new(TEXT).text_width_basis(TextWidthBasis::Parent),
            );
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester),
                waft_embedder::Size::new(800.0, 28.0)
            );

            let root = overlay(
                tester,
                SelectableText::new(TEXT).text_width_basis(TextWidthBasis::LongestLine),
            );
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester),
                waft_embedder::Size::new(633.0, 28.0)
            );
        });
    }

    #[test]
    fn selectable_text_basic() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("selectable"));
            tester.pump_widget(root);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert!(selection(tester).is_collapsed());

            let at = center_of_selectable_text(tester);
            tester.tap_at(at, 1);
            tester.pump();

            // Collapse selection should not paint.
            assert!(!handles_are_visible(tester));
            // Long press on the 't' character of text 'selectable' to show context menu.
            let d_pos = text_offset_to_position(tester, 5);
            tester.long_press_at(d_pos, 2);
            tester.pump();

            // Context menu should not have paste and cut.
            expect(tester, &find::text("Copy"), finds_one_widget());
            expect(tester, &find::text("Paste"), finds_nothing());
            expect(tester, &find::text("Cut"), finds_nothing());
        });
    }

    #[test]
    #[allow(deprecated)]
    fn selectable_text_can_disable_toolbar_options() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                SelectableText::new("a selectable text").toolbar_options(ToolbarOptions {
                    select_all: true,
                    ..ToolbarOptions::default()
                }),
            );
            tester.pump_widget(root);
            let d_pos = text_offset_to_position(tester, 5);
            tester.long_press_at(d_pos, 1);
            tester.pump();
            // Context menu should not have copy.
            expect(tester, &find::text("Copy"), finds_nothing());
            expect(tester, &find::text("Select all"), finds_one_widget());
        });
    }

    #[test]
    fn can_select_text_by_dragging_with_a_mouse() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);

            let e_pos = text_offset_to_position(tester, 5);
            let g_pos = text_offset_to_position(tester, 8);

            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, e_pos);
            tester.pump();
            tester.move_to(&mut gesture, g_pos, true);
            tester.pump();
            tester.up(&gesture);
            tester.pump_and_settle();

            assert_eq!(selection(tester).base_offset, 5);
            assert_eq!(selection(tester).extent_offset, 8);
        });
    }

    #[test]
    fn can_use_selection_toolbar() {
        test_widgets(|_tester| {
            let (mut tester, clipboard) = tester_with_clipboard(TargetPlatform::Android);
            clipboard.set_clipboard_data(Some("Clipboard data".to_owned()));
            let test_value = "abc def ghi";
            let root = material_home(&mut tester, SelectableText::new(test_value));
            tester.pump_widget(root);

            // Tap the selection handle to bring up the "paste / select all" menu.
            let at = text_offset_to_position(&tester, test_value.find('e').unwrap() as i32);
            tester.tap_at(at, 1);
            tester.pump();
            // Skip past the frame where the opacity is zero.
            tester.pump_by(Duration::from_millis(200));
            let render_editable = render_editable_at(&tester, 0);
            let controller = controller(&tester);
            let handle_point = {
                let mut app = tester.cell.borrow_mut();
                let selection = controller.selection(&app);
                let endpoints = render_editable.get_endpoints_for_selection(&mut app, selection);
                render_editable
                    .as_box()
                    .local_to_global(&app, endpoints[0].point, None)
            };
            // Tapping on the part of the handle's GestureDetector where it overlaps
            // with the text itself does not show the menu, so add a small vertical
            // offset to tap below the text.
            tester.tap_at(handle_point + Offset::new(1.0, 13.0), 2);
            tester.pump();
            // Skip past the frame where the opacity is zero.
            tester.pump_by(Duration::from_millis(200));

            // Select all should select all the text.
            let select_all = find::text("Select all").evaluate(&tester)[0];
            let at = tester.get_rect(select_all).center();
            tester.tap_at(at, 3);
            tester.pump();
            assert_eq!(selection(&tester).base_offset, 0);
            assert_eq!(selection(&tester).extent_offset, test_value.len() as i32);

            // Copy should reset the selection.
            let copy = find::text("Copy").evaluate(&tester)[0];
            let at = tester.get_rect(copy).center();
            tester.tap_at(at, 4);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert!(selection(&tester).is_collapsed());
        });
    }

    fn selectable_text_builder(
        tester: &mut WidgetTester,
        text: &str,
        max_lines: Option<i32>,
        min_lines: Option<i32>,
    ) -> WidgetRef {
        let mut selectable_text = SelectableText::new(text)
            .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
            .max_lines(max_lines);
        if let Some(min_lines) = min_lines {
            selectable_text = selectable_text.min_lines(min_lines);
        }
        overlay(tester, selectable_text)
    }

    fn selectable_text_size(tester: &WidgetTester) -> waft_embedder::Size {
        let element = find::by_type::<SelectableText>().evaluate(tester)[0];
        let render = tester.render_box_of(element);
        render.size(&tester.cell.borrow())
    }

    #[test]
    fn selectable_height_with_max_line() {
        test_widgets(|tester| {
            let root = selectable_text_builder(tester, "", Some(1), None);
            tester.pump_widget(root);
            let empty_input_size = selectable_text_size(tester);

            let root = selectable_text_builder(tester, "No wrapping here.", Some(1), None);
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                empty_input_size.height()
            );

            // Even when entering multiline text, SelectableText doesn't grow. It's a single
            // line input.
            let root = selectable_text_builder(tester, K_THREE_LINES, Some(1), None);
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                empty_input_size.height()
            );

            // maxLines: 3 makes the SelectableText 3 lines tall.
            let root = selectable_text_builder(tester, "", Some(3), None);
            tester.pump_widget(root);
            assert!(selectable_text_size(tester).height() > empty_input_size.height());
            let three_line_input_size = selectable_text_size(tester);

            // Filling with 3 lines of text stays the same size.
            let root = selectable_text_builder(tester, K_THREE_LINES, Some(3), None);
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                three_line_input_size.height()
            );

            // An extra line won't increase the size because we max at 3.
            let root = selectable_text_builder(tester, K_MORE_THAN_FOUR_LINES, Some(3), None);
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                three_line_input_size.height()
            );

            // But now it will... but it will max at four.
            let root = selectable_text_builder(tester, K_MORE_THAN_FOUR_LINES, Some(4), None);
            tester.pump_widget(root);
            assert!(selectable_text_size(tester).height() > three_line_input_size.height());
            let four_line_input_size = selectable_text_size(tester);

            // Now it won't max out until the end.
            let root = selectable_text_builder(tester, "", None, None);
            tester.pump_widget(root);
            assert_eq!(selectable_text_size(tester), empty_input_size);
            let root = selectable_text_builder(tester, K_THREE_LINES, None, None);
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                three_line_input_size.height()
            );
            let root = selectable_text_builder(tester, K_MORE_THAN_FOUR_LINES, None, None);
            tester.pump_widget(root);
            assert!(selectable_text_size(tester).height() > four_line_input_size.height());
        });
    }

    #[test]
    #[should_panic(expected = "minLines can't be greater than maxLines")]
    fn min_lines_cannot_be_greater_than_max_lines() {
        let _ = SelectableText::new("abcd")
            .max_lines(Some(3))
            .min_lines(4)
            .into_widget();
    }

    #[test]
    fn selectable_height_with_min_line() {
        test_widgets(|tester| {
            let root = selectable_text_builder(tester, "", Some(1), None);
            tester.pump_widget(root);
            let empty_input_size = selectable_text_size(tester);

            // Even if the text is a one liner, minimum height of SelectableText will
            // determined by minLines.
            let root = selectable_text_builder(tester, "No wrapping here.", Some(3), Some(2));
            tester.pump_widget(root);
            assert_eq!(
                selectable_text_size(tester).height(),
                empty_input_size.height() * 2.0
            );
        });
    }

    #[test]
    fn tapping_outside_selectable_text_clears_the_selection() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                Center::new().child(Column::new().children(vec![
                    SelectableText::new("first selectable text").into_widget(),
                    SelectableText::new("second selectable text").into_widget(),
                ])),
            );
            tester.pump_widget(root);
            // Setting the app lifecycle state to AppLifecycleState.resumed to simulate
            // an applications default running mode, i.e. the application window is focused.
            SchedulerBinding::handle_app_lifecycle_state_changed(
                &mut tester.cell.borrow_mut(),
                AppLifecycleState::Resumed,
            );
            tester.pump_and_settle();

            // First tap on the first SelectableText sets the cursor.
            let at = text_offset_to_position(tester, 5);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let controller_a = controller_at(tester, 0);
            let controller_b = controller_at(tester, 1);
            let read = |tester: &WidgetTester, controller: AnyTextEditingController| {
                controller.selection(&tester.cell.borrow())
            };
            let empty = TextSelection::collapsed(-1, TextAffinity::Downstream);

            assert_eq!(
                read(tester, controller_a.handle()),
                TextSelection::collapsed(5, TextAffinity::Downstream)
            );
            assert_eq!(read(tester, controller_b.handle()), empty);

            // Tapping on the second SelectableText sets the cursor on it, and clears the
            // selection from the first SelectableText.
            let selectable_text_start = top_left_of_selectable_text(tester, 1);
            tester.tap_at(selectable_text_start, 2);
            tester.pump_and_settle();
            assert_eq!(read(tester, controller_a.handle()), empty);
            assert_eq!(
                read(tester, controller_b.handle()),
                TextSelection::collapsed(0, TextAffinity::Downstream)
            );

            // Setting the app lifecycle state to AppLifecycleState.inactive to simulate
            // a lose of window focus. Selection should remain the same.
            SchedulerBinding::handle_app_lifecycle_state_changed(
                &mut tester.cell.borrow_mut(),
                AppLifecycleState::Inactive,
            );
            tester.pump_and_settle();
            assert_eq!(read(tester, controller_a.handle()), empty);
            assert_eq!(
                read(tester, controller_b.handle()),
                TextSelection::collapsed(0, TextAffinity::Downstream)
            );
        });
    }

    #[test]
    fn selectable_text_is_skipped_during_focus_traversal() {
        test_widgets(|tester| {
            let (first_field_focus, last_field_focus) = {
                let mut app = tester.cell.borrow_mut();
                (FocusNode::new(&mut app), FocusNode::new(&mut app))
            };
            // Dart's two `TextField`s, which this case only needs to take focus by traversal.
            let root = material_home(
                tester,
                Center::new().child(Column::new().children(vec![
                    Focus::new(SizedBox::new().width(10.0).height(10.0))
                        .focus_node(first_field_focus.erase())
                        .autofocus(true)
                        .into_widget(),
                    SelectableText::new("some text").into_widget(),
                    Focus::new(SizedBox::new().width(10.0).height(10.0))
                        .focus_node(last_field_focus.erase())
                        .into_widget(),
                ])),
            );
            tester.pump_widget(root);
            tester.pump();

            assert!(first_field_focus.has_focus(&mut tester.cell.borrow_mut()));
            assert!(!last_field_focus.has_focus(&mut tester.cell.borrow_mut()));

            first_field_focus
                .as_node()
                .next_focus(&mut tester.cell.borrow_mut());
            tester.pump();

            // Expecting focus to skip straight to the second field.
            assert!(!first_field_focus.has_focus(&mut tester.cell.borrow_mut()));
            assert!(last_field_focus.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    /// Dart's `setupWidget` of the 'Keyboard Tests' group.
    fn setup_keyboard_widget(tester: &mut WidgetTester, text: &str) {
        let root = material_home(tester, SelectableText::new(text).max_lines(Some(3)));
        tester.pump_widget(root);
        let at = center_of_selectable_text(tester);
        tester.tap_at(at, 1);
        tester.pump_and_settle();
    }

    fn selection_delta(tester: &WidgetTester) -> i32 {
        let selection = selection(tester);
        selection.extent_offset - selection.base_offset
    }

    #[test]
    fn keyboard_shift_test_1() {
        test_widgets(|tester| {
            setup_keyboard_widget(tester, "a big house");

            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::ARROW_LEFT),
                LogicalKeyboardKey::ARROW_LEFT,
            );
            assert_eq!(selection_delta(tester), -1);
        });
    }

    #[test]
    fn keyboard_shift_test_2() {
        test_widgets(|tester| {
            setup_keyboard_widget(tester, "abcdefghi");

            controller(tester).set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(3, TextAffinity::Downstream),
            );
            tester.pump();

            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::ARROW_RIGHT),
                LogicalKeyboardKey::ARROW_RIGHT,
            );
            tester.pump_and_settle();
            assert_eq!(selection_delta(tester), 1);
        });
    }

    #[test]
    fn keyboard_control_shift_test() {
        test_widgets(|tester| {
            setup_keyboard_widget(tester, "their big house");

            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::CONTROL_LEFT),
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::ARROW_LEFT),
                LogicalKeyboardKey::ARROW_LEFT,
            );
            tester.pump_and_settle();

            assert_eq!(selection_delta(tester), -5);
        });
    }

    #[test]
    fn keyboard_down_and_up_test() {
        test_widgets(|tester| {
            setup_keyboard_widget(tester, "a big house");

            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::ARROW_UP),
                LogicalKeyboardKey::ARROW_UP,
            );
            tester.pump_and_settle();

            assert_eq!(selection_delta(tester), -11);

            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::ARROW_UP),
                LogicalKeyboardKey::ARROW_UP,
            );
            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::ARROW_DOWN),
                LogicalKeyboardKey::ARROW_DOWN,
            );
            tester.pump_and_settle();

            assert_eq!(selection_delta(tester), 0);
        });
    }

    #[test]
    fn copy_test() {
        test_widgets(|_tester| {
            let (mut tester, clipboard) = tester_with_clipboard(TargetPlatform::Android);
            let test_value = "a big house\njumped over a mouse";
            let root = material_home(
                &mut tester,
                SelectableText::new(test_value).max_lines(Some(3)),
            );
            tester.pump_widget(root);
            let at = center_of_selectable_text(&tester);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            controller(&tester).set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();

            // Select the first 5 characters.
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            for _ in 0..5 {
                tester.send_key_event(
                    physical_of(LogicalKeyboardKey::ARROW_RIGHT),
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump_and_settle();
            }
            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );

            // Copy them.
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::CONTROL_RIGHT),
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::KEY_C),
                LogicalKeyboardKey::KEY_C,
            );
            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::KEY_C),
                LogicalKeyboardKey::KEY_C,
            );
            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::CONTROL_RIGHT),
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();

            assert_eq!(clipboard.clipboard_data().as_deref(), Some("a big"));
        });
    }

    #[test]
    fn select_all_test() {
        test_widgets(|tester| {
            let test_value = "a big house\njumped over a mouse";
            let root = material_home(tester, SelectableText::new(test_value).max_lines(Some(3)));
            tester.pump_widget(root);
            let at = center_of_selectable_text(tester);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            // Select All.
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::CONTROL_LEFT),
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.send_key_event(
                physical_of(LogicalKeyboardKey::KEY_A),
                LogicalKeyboardKey::KEY_A,
            );
            tester.send_key_up_event(
                physical_of(LogicalKeyboardKey::CONTROL_LEFT),
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.pump_and_settle();

            assert_eq!(selection(tester).base_offset, 0);
            assert_eq!(selection(tester).extent_offset, 31);
        });
    }

    #[test]
    fn keyboard_selection_should_call_on_selection_changed() {
        test_widgets(|tester| {
            let new_selection: Rc<RefCell<Option<TextSelection>>> = Rc::new(RefCell::new(None));
            let test_value = "a big house\njumped over a mouse";
            let recorded = Rc::clone(&new_selection);
            let root = material_home(
                tester,
                SelectableText::new(test_value)
                    .max_lines(Some(3))
                    .on_selection_changed(move |_app, selection, _cause| {
                        assert!(recorded.borrow().is_none());
                        *recorded.borrow_mut() = Some(selection);
                    }),
            );
            tester.pump_widget(root);
            let at = center_of_selectable_text(tester);
            tester.tap_at(at, 1);
            tester.pump_and_settle();
            let tapped = new_selection.borrow_mut().take().expect("a tap selects");
            assert_eq!(tapped.base_offset, 31);
            assert_eq!(tapped.extent_offset, 31);

            controller(tester).set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();

            // Select the first 5 characters.
            tester.send_key_down_event(
                physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            for i in 0..5 {
                tester.send_key_event(
                    physical_of(LogicalKeyboardKey::ARROW_RIGHT),
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump_and_settle();
                let selection = new_selection.borrow_mut().take().expect("a key selects");
                assert_eq!(selection.base_offset, 0);
                assert_eq!(selection.extent_offset, i + 1);
            }
        });
    }

    #[test]
    fn on_tap_is_called_upon_tap() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            let counted = Rc::clone(&tap_count);
            let root = overlay(
                tester,
                SelectableText::new("something")
                    .on_tap(Listener::new(move |_app| counted.set(counted.get() + 1))),
            );
            tester.pump_widget(root);

            assert_eq!(tap_count.get(), 0);
            for pointer in 1..=3 {
                let at = center_of_selectable_text(tester);
                tester.tap_at(at, pointer);
                // Wait a bit so they're all single taps and not double taps.
                tester.pump_by(Duration::from_millis(300));
            }
            assert_eq!(tap_count.get(), 3);
        });
    }

    #[test]
    fn selectable_text_style_is_merged_with_default_text_style() {
        test_widgets(|tester| {
            let default_style = TextStyle::new().color(Colors::BLUE.shade500());
            let build_frame = |tester: &mut WidgetTester, style: TextStyle| {
                material_home(
                    tester,
                    DefaultTextStyle::new(
                        default_style.clone(),
                        Center::new().child(SelectableText::new("something").style(style)),
                    ),
                )
            };
            let editable_style = |tester: &WidgetTester| {
                tester.widget::<EditableText, _>(editable_elements(tester)[0], |w| w.style.clone())
            };

            // Empty TextStyle is overridden by theme.
            let root = build_frame(tester, TextStyle::new());
            tester.pump_widget(root);
            let style = editable_style(tester);
            assert_eq!(style.color, default_style.color);
            assert_eq!(style.background, default_style.background);
            assert_eq!(style.shadows, default_style.shadows);
            assert_eq!(style.decoration, default_style.decoration);
            assert_eq!(style.word_spacing, default_style.word_spacing);

            // Properties set on TextStyle override theme.
            let set_color = Colors::RED.primary();
            let root = build_frame(tester, TextStyle::new().color(set_color));
            tester.pump_widget(root);
            assert_eq!(editable_style(tester).color, Some(set_color.into()));

            // inherit: false causes nothing to be merged in from theme.
            let root = build_frame(
                tester,
                TextStyle::new()
                    .font_size(24.0)
                    .text_baseline(waft_embedder::TextBaseline::Alphabetic)
                    .inherit(false),
            );
            tester.pump_widget(root);
            assert!(editable_style(tester).color.is_none());
        });
    }

    /// Dart's `'Atwater Peel Sherbrooke Bonaventure'` centred on a `Material` in a
    /// `MaterialApp`, with `selection_controls` when a case gives them.
    fn atwater(
        tester: &mut WidgetTester,
        selection_controls: Option<Rc<dyn TextSelectionControls>>,
    ) -> WidgetRef {
        let mut selectable_text = SelectableText::new(ATWATER);
        if let Some(selection_controls) = selection_controls {
            selectable_text = selectable_text.selection_controls(selection_controls);
        }
        material_home(tester, Center::new().child(selectable_text))
    }

    #[test]
    fn tap_moves_cursor_to_the_edge_of_the_word_it_tapped() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let root = atwater(tester, None);
            tester.pump_widget(root);

            let at = selectable_text_start_plus(tester, 50.0);
            tester.tap_at(at, 1);
            tester.pump();

            // We moved the cursor.
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(7, TextAffinity::Upstream)
            );

            // But don't trigger the toolbar.
            expect(tester, &find::by_type::<CupertinoButton>(), finds_nothing());
        });
    }

    #[test]
    fn tap_moves_cursor_to_the_position_tapped() {
        for platform in ALL_PLATFORMS
            .into_iter()
            .filter(|platform| *platform != TargetPlatform::IOS)
        {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, None);
                tester.pump_widget(root);

                let at = selectable_text_start_plus(tester, 50.0);
                tester.tap_at(at, 1);
                tester.pump();

                // We moved the cursor.
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(4, TextAffinity::Upstream),
                    "{platform:?}"
                );

                // But don't trigger the toolbar.
                expect(tester, &find::by_type::<TextButton>(), finds_nothing());
            });
        }
    }

    #[test]
    fn two_slow_taps_do_not_trigger_a_word_selection_on_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let root = atwater(tester, None);
            tester.pump_widget(root);

            let at = selectable_text_start_plus(tester, 50.0);
            tester.tap_at(at, 1);
            tester.pump_by(Duration::from_millis(500));
            tester.tap_at(at, 2);
            tester.pump();

            // Plain collapsed selection.
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(7, TextAffinity::Upstream)
            );

            // No toolbar.
            expect(tester, &find::by_type::<CupertinoButton>(), finds_nothing());
        });
    }

    #[test]
    fn two_slow_taps_do_not_trigger_a_word_selection() {
        for platform in ALL_PLATFORMS
            .into_iter()
            .filter(|platform| *platform != TargetPlatform::IOS)
        {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, None);
                tester.pump_widget(root);

                let at = selectable_text_start_plus(tester, 50.0);
                tester.tap_at(at, 1);
                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(at, 2);
                tester.pump();

                // Plain collapsed selection.
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(4, TextAffinity::Upstream),
                    "{platform:?}"
                );

                // No toolbar.
                expect(tester, &find::by_type::<CupertinoButton>(), finds_nothing());
            });
        }
    }

    #[test]
    fn double_tap_selects_word_and_first_tap_of_double_tap_moves_cursor() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, None);
                tester.pump_widget(root);

                // This tap just puts the cursor somewhere different than where the double
                // tap will occur to test that the double tap moves the existing cursor first.
                let at = selectable_text_start_plus(tester, 50.0);
                tester.tap_at(at, 1);
                tester.pump_by(Duration::from_millis(500));

                let at = selectable_text_start_plus(tester, 150.0);
                tester.tap_at(at, 2);
                tester.pump_by(Duration::from_millis(50));

                // First tap moved the cursor.
                // On iOS, this moves the cursor to the closest word edge.
                // On macOS, this moves the cursor to the tapped position.
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(
                        if platform == TargetPlatform::IOS {
                            12
                        } else {
                            11
                        },
                        TextAffinity::Upstream
                    ),
                    "{platform:?}"
                );
                tester.tap_at(at, 3);
                tester.pump();

                // Second tap selects the word around the cursor.
                assert_eq!(selection(tester), TextSelection::new(8, 12), "{platform:?}");

                expect_cupertino_selection_toolbar(tester, platform);
            });
        }
    }

    #[test]
    fn double_tap_selects_word_and_first_tap_of_double_tap_moves_cursor_and_shows_toolbar_android()
    {
        test_widgets(|tester| {
            let root = atwater(tester, None);
            tester.pump_widget(root);

            let at = selectable_text_start_plus(tester, 50.0);
            tester.tap_at(at, 1);
            tester.pump_by(Duration::from_millis(500));

            let at = selectable_text_start_plus(tester, 150.0);
            tester.tap_at(at, 2);
            tester.pump_by(Duration::from_millis(50));

            // First tap moved the cursor.
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(11, TextAffinity::Upstream)
            );
            tester.tap_at(at, 3);
            tester.pump();

            // Second tap selects the word around the cursor.
            assert_eq!(selection(tester), TextSelection::new(8, 12));

            expect_material_selection_toolbar(tester, TargetPlatform::Android);
        });
    }

    #[test]
    fn double_tap_on_top_of_cursor_also_selects_word_android() {
        test_widgets(|tester| {
            let root = atwater(tester, None);
            tester.pump_widget(root);

            // Tap to put the cursor after the "w".
            let index = 3;
            let at = text_offset_to_position(tester, index);
            tester.tap_at(at, 1);
            tester.pump_by(Duration::from_millis(500));

            assert_eq!(
                selection(tester),
                TextSelection::collapsed(index, TextAffinity::Downstream)
            );

            // Double tap on the same location.
            tester.tap_at(at, 2);
            tester.pump_by(Duration::from_millis(50));

            // First tap doesn't change the selection.
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(index, TextAffinity::Downstream)
            );

            // Second tap selects the word around the cursor.
            tester.tap_at(at, 3);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(0, 7));

            expect_material_selection_toolbar(tester, TargetPlatform::Android);
        });
    }

    #[test]
    fn long_press_selects_word_and_shows_toolbar_ios() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, None);
                tester.pump_widget(root);

                let at = selectable_text_start_plus(tester, 50.0);
                tester.long_press_at(at, 1);
                tester.pump();

                // The long pressed word is selected.
                assert_eq!(selection(tester), TextSelection::new(0, 7), "{platform:?}");

                expect_cupertino_selection_toolbar(tester, platform);
            });
        }
    }

    #[test]
    fn long_press_selects_word_and_shows_toolbar_android() {
        test_widgets(|tester| {
            let root = atwater(tester, None);
            tester.pump_widget(root);

            let at = selectable_text_start_plus(tester, 50.0);
            tester.long_press_at(at, 1);
            tester.pump();

            assert_eq!(selection(tester), TextSelection::new(0, 7));

            expect_material_selection_toolbar(tester, TargetPlatform::Android);
        });
    }

    #[test]
    fn long_press_selects_word_and_shows_custom_toolbar_cupertino() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, Some(cupertino_text_selection_controls()));
                tester.pump_widget(root);

                let at = selectable_text_start_plus(tester, 50.0);
                tester.long_press_at(at, 1);
                tester.pump();

                // The long pressed word is selected.
                assert_eq!(selection(tester), TextSelection::new(0, 7), "{platform:?}");

                // Toolbar shows one button (copy).
                expect(
                    tester,
                    &find::by_type::<CupertinoButton>(),
                    finds_one_widget(),
                );
                expect(tester, &find::text("Copy"), finds_one_widget());
            });
        }
    }

    #[test]
    fn long_press_selects_word_and_shows_custom_toolbar_material() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = atwater(tester, Some(material_text_selection_controls()));
                tester.pump_widget(root);

                let at = selectable_text_start_plus(tester, 50.0);
                tester.long_press_at(at, 1);
                tester.pump();

                assert_eq!(selection(tester), TextSelection::new(0, 7), "{platform:?}");

                // Collapsed toolbar shows 2 buttons: copy, select all
                assert_eq!(
                    find::by_type::<TextButton>().evaluate(tester).len(),
                    2,
                    "{platform:?}"
                );
                expect(tester, &find::text("Copy"), finds_one_widget());
                expect(tester, &find::text("Select all"), finds_one_widget());
            });
        }
    }

    #[test]
    fn text_selection_controls_is_passed_to_editable_text() {
        test_widgets(|tester| {
            let controls = material_text_selection_controls();
            let root = atwater(tester, Some(Rc::clone(&controls)));
            tester.pump_widget(root);

            let passed = tester.widget::<EditableText, _>(editable_elements(tester)[0], |w| {
                w.selection_controls.clone()
            });
            assert!(passed.is_some_and(|passed| Rc::ptr_eq(&passed, &controls)));
        });
    }

    fn abc_def_ghi(tester: &mut WidgetTester) -> WidgetRef {
        material_home(tester, SelectableText::new("abc def ghi"))
    }

    #[test]
    fn long_press_shows_handles_and_toolbar() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);

            // Long press at 'e' in 'def'.
            let e_pos = text_offset_to_position(tester, 5);
            tester.long_press_at(e_pos, 1);
            tester.pump_and_settle();

            assert!(handles_are_visible(tester));
            assert!(toolbar_is_visible(tester));
        });
    }

    #[test]
    fn double_tap_shows_handles_and_toolbar() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);

            // Double tap at 'e' in 'def'.
            let e_pos = text_offset_to_position(tester, 5);
            tester.tap_at(e_pos, 1);
            tester.pump_by(Duration::from_millis(50));
            tester.tap_at(e_pos, 2);
            tester.pump();

            assert!(handles_are_visible(tester));
            assert!(toolbar_is_visible(tester));
        });
    }

    #[test]
    fn mouse_tap_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);

            let e_pos = text_offset_to_position(tester, 5);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, e_pos);
            tester.pump();
            tester.up(&gesture);
            tester.pump();

            assert!(!toolbar_is_visible(tester));
            assert!(!handles_are_visible(tester));
        });
    }

    #[test]
    fn mouse_long_press_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);

            let e_pos = text_offset_to_position(tester, 5);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, e_pos);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump();

            assert!(!toolbar_is_visible(tester));
            assert!(!handles_are_visible(tester));
        });
    }

    #[test]
    fn mouse_double_tap_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);

            // Double tap to trigger the selectable text.
            let at = center_of_selectable_text(tester);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, at);
            tester.pump_by(Duration::from_millis(50));
            tester.up(&gesture);
            tester.pump();
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump();
            tester.up(&gesture);
            tester.pump();

            assert!(!toolbar_is_visible(tester));
            assert!(!handles_are_visible(tester));
        });
    }

    #[test]
    fn on_selection_changed_is_called_when_selection_changes() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let counted = Rc::clone(&count);
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").on_selection_changed(
                    move |_app, _selection, _cause| counted.set(counted.get() + 1),
                ),
            );
            tester.pump_widget(root);

            // Long press to select 'abc'.
            let a_location = text_offset_to_position(tester, 1);
            tester.long_press_at(a_location, 1);
            tester.pump();
            assert_eq!(count.get(), 1);
            // Long press to select 'def'.
            let at = text_offset_to_position(tester, 5);
            tester.long_press_at(at, 2);
            tester.pump();
            assert_eq!(count.get(), 2);
            // Tap on 'Select all' option to select the whole text.
            let select_all = find::text("Select all").evaluate(tester)[0];
            let at = tester.get_rect(select_all).center();
            tester.tap_at(at, 3);
            assert_eq!(count.get(), 3);
        });
    }

    #[test]
    fn selectable_text_rich_span_style_is_merged_with_default_text_style() {
        test_widgets(|tester| {
            let text_style = TextStyle::new()
                .color(Color::new(0xff00ff00))
                .font_size(12.0);
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let root = material_app(
                theme,
                SelectableText::rich(TextSpan::new().text("Abcd").style(text_style.clone()))
                    .into_widget(),
            );
            tester.pump_widget(root);

            let font_size = tester
                .widget::<EditableText, _>(editable_elements(tester)[0], |w| w.style.font_size);
            assert_eq!(font_size, text_style.font_size);
        });
    }

    #[test]
    fn a_second_tap_at_the_same_place_does_not_call_on_selection_changed() {
        test_widgets(|tester| {
            let selection_seen: Rc<RefCell<Option<TextSelection>>> = Rc::new(RefCell::new(None));
            let count = Rc::new(Cell::new(0));
            let (recorded, counted) = (Rc::clone(&selection_seen), Rc::clone(&count));
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let root = material_app(
                theme,
                SelectableText::new("I love Flutter!")
                    .on_selection_changed(move |_app, selection, _cause| {
                        *recorded.borrow_mut() = Some(selection);
                        counted.set(counted.get() + 1);
                    })
                    .into_widget(),
            );
            tester.pump_widget(root);

            assert!(selection_seen.borrow().is_none());
            assert_eq!(count.get(), 0);

            // Tap to put the cursor before the "F".
            let index = 7;
            let at = text_offset_to_position(tester, index);
            tester.tap_at(at, 1);
            tester.pump_by(Duration::from_millis(500));
            assert_eq!(
                *selection_seen.borrow(),
                Some(TextSelection::collapsed(index, TextAffinity::Downstream))
            );
            // The `onSelectionChanged` will be triggered one time.
            assert_eq!(count.get(), 1);

            // Tap on the same location again.
            tester.tap_at(at, 2);
            tester.pump_by(Duration::from_millis(50));
            assert_eq!(
                *selection_seen.borrow(),
                Some(TextSelection::collapsed(index, TextAffinity::Downstream))
            );
            // The `onSelectionChanged` will not be triggered.
            assert_eq!(count.get(), 1);
        });
    }

    #[test]
    fn changing_the_data_gives_the_editable_text_a_new_controller() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);
            let first = controller(tester);
            assert_eq!(first.text_value(&tester.cell.borrow()), "abc def ghi");

            let root = material_home(tester, SelectableText::new("jkl"));
            tester.pump_widget(root);
            let second = controller(tester);
            assert!(first != second);
            assert_eq!(second.text_value(&tester.cell.borrow()), "jkl");

            let root = material_home(tester, SelectableText::new("jkl"));
            tester.pump_widget(root);
            assert!(controller(tester) == second);
        });
    }

    #[test]
    fn the_editable_text_is_read_only_and_does_not_force_a_line() {
        test_widgets(|tester| {
            let root = abc_def_ghi(tester);
            tester.pump_widget(root);
            let (read_only, force_line, max_lines) = tester
                .widget::<EditableText, _>(editable_elements(tester)[0], |w| {
                    (w.read_only, w.force_line, w.max_lines)
                });
            assert!(read_only);
            assert!(!force_line);
            assert_eq!(max_lines, None);
        });
    }

    #[test]
    fn no_text_keyboard_when_widget_is_focused() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("selectable text"));
            tester.pump_widget(root);
            tester.tap(find::by_type::<SelectableText>());
            tester.cell.checkpoint();
            assert!(!tester.test_text_input().has_any_clients());
        });
    }

    #[test]
    fn scroll_behavior_can_be_overridden() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new(K_MORE_THAN_FOUR_LINES)
                    .drag_start_behavior(DragStartBehavior::Down)
                    .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                    .max_lines(Some(2)),
            );
            tester.pump_widget(root);
            let editable_behavior = tester
                .widget::<EditableText, _>(find::by_type::<EditableText>(), |widget| {
                    widget.scroll_behavior.clone()
                });
            assert!(editable_behavior.is_none());
            let scroll_behavior = tester.widget::<waft_widgets::Scrollable, _>(
                find::by_type::<waft_widgets::Scrollable>(),
                |widget| widget.scroll_behavior.clone(),
            );
            assert!(scroll_behavior.is_some());

            let behavior =
                ScrollBehaviorRef::new(ScrollBehaviorRef::plain().copy_with().scrollbars(false));
            let root = material_home(
                tester,
                SelectableText::new(K_MORE_THAN_FOUR_LINES)
                    .drag_start_behavior(DragStartBehavior::Down)
                    .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                    .max_lines(Some(2))
                    .scroll_behavior(behavior.clone()),
            );
            tester.pump_widget(root);
            let editable_behavior = tester
                .widget::<EditableText, _>(find::by_type::<EditableText>(), |widget| {
                    widget.scroll_behavior.clone()
                });
            assert_eq!(editable_behavior, Some(behavior.clone()));
            let scroll_behavior = tester
                .widget::<waft_widgets::Scrollable, _>(
                    find::by_type::<waft_widgets::Scrollable>(),
                    |widget| widget.scroll_behavior.clone(),
                )
                .expect("the scrollable has a behavior");
            assert_eq!(scroll_behavior, behavior);
        });
    }

    #[test]
    fn caret_works_when_max_lines_is_null() {
        test_widgets(|tester| {
            let root = overlay(tester, SelectableText::new("x"));
            tester.pump_widget(root);
            assert_eq!(selection(tester).base_offset, -1);

            tester.tap_at(text_offset_to_position(tester, 0), 1);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert_eq!(selection(tester).base_offset, 0);
        });
    }

    #[test]
    fn double_tap_hold_selects_word() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = material_home(tester, Center::new().child(SelectableText::new(ATWATER)));
                tester.pump_widget(root);
                let start = top_left_of_selectable_text(tester, 0);
                tester.tap_at(start + Offset::new(150.0, 5.0), 1);
                tester.pump_by(Duration::from_millis(50));

                let mut gesture = TestPointer::touch(2);
                tester.down(&mut gesture, start + Offset::new(150.0, 5.0));
                tester.pump_by(Duration::from_millis(500));
                assert_eq!(selection(tester), TextSelection::new(8, 12));
                expect_cupertino_selection_toolbar(tester, platform);

                tester.up(&gesture);
                tester.pump();
                assert_eq!(selection(tester), TextSelection::new(8, 12));
                expect_cupertino_selection_toolbar(tester, platform);
            });
        }
    }

    #[test]
    fn tap_after_a_double_tap_select_is_not_affected_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let root = material_home(tester, Center::new().child(SelectableText::new(ATWATER)));
            tester.pump_widget(root);
            let start = top_left_of_selectable_text(tester, 0);

            tester.tap_at(start + Offset::new(150.0, 5.0), 1);
            tester.pump_by(Duration::from_millis(50));
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(12, TextAffinity::Upstream)
            );
            tester.tap_at(start + Offset::new(150.0, 5.0), 2);
            tester.pump_by(Duration::from_millis(500));
            tester.tap_at(start + Offset::new(100.0, 5.0), 3);
            tester.pump();
            assert_eq!(
                selection(tester),
                TextSelection::collapsed(7, TextAffinity::Downstream)
            );
            assert!(
                find::by_type::<CupertinoButton>()
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    #[test]
    fn tap_after_a_double_tap_select_is_not_affected_macos() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::MacOS),
            |tester| {
                let root = material_home(tester, Center::new().child(SelectableText::new(ATWATER)));
                tester.pump_widget(root);
                let start = top_left_of_selectable_text(tester, 0);

                tester.tap_at(start + Offset::new(150.0, 5.0), 1);
                tester.pump_by(Duration::from_millis(50));
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(11, TextAffinity::Upstream)
                );
                tester.tap_at(start + Offset::new(150.0, 5.0), 2);
                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(start + Offset::new(100.0, 5.0), 3);
                tester.pump();
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(7, TextAffinity::Downstream)
                );
                assert!(
                    find::by_type::<CupertinoButton>()
                        .evaluate(tester)
                        .is_empty()
                );
            },
        );
    }

    #[test]
    fn long_press_tap_cannot_initiate_a_double_tap_on_macos() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::MacOS),
            |tester| {
                let root = material_home(tester, Center::new().child(SelectableText::new(ATWATER)));
                tester.pump_widget(root);
                let start = top_left_of_selectable_text(tester, 0);

                tester.long_press_at(start + Offset::new(50.0, 5.0), 1);
                tester.pump_by(Duration::from_millis(50));
                editable_state(tester).hide_toolbar(&mut tester.cell.borrow_mut(), false);
                tester.pump_and_settle();

                tester.tap_at(start + Offset::new(50.0, 5.0), 2);
                tester.pump();
                assert_eq!(
                    selection(tester),
                    TextSelection::collapsed(4, TextAffinity::Upstream)
                );
                assert!(
                    find::by_type::<CupertinoButton>()
                        .evaluate(tester)
                        .is_empty()
                );
            },
        );
    }

    #[test]
    fn long_press_tap_cannot_initiate_a_double_tap_on_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let root = material_home(tester, Center::new().child(SelectableText::new(ATWATER)));
            tester.pump_widget(root);
            let start = top_left_of_selectable_text(tester, 0);

            tester.long_press_at(start + Offset::new(50.0, 5.0), 1);
            tester.pump_by(Duration::from_millis(50));
            editable_state(tester).hide_toolbar(&mut tester.cell.borrow_mut(), false);
            tester.pump_and_settle();

            tester.tap_at(start + Offset::new(50.0, 5.0), 2);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(0, 7));
            expect(
                tester,
                &find::by_type::<CupertinoButton>(),
                finds_n_widgets(4),
            );
        });
    }

    #[test]
    fn selectable_text_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            let root = material_home(tester, Center::new().child(SelectableText::new("test")));
            tester.pump_widget(root);
            let at = tester.get_center(find::by_type::<EditableText>());
            let mut pointer = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut pointer, at);
            tester.pump();
            tester.move_to(&mut pointer, at, false);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
        });
    }

    #[test]
    fn the_handles_show_after_pressing_select_all() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = material_home(tester, SelectableText::new("abc def ghi"));
                tester.pump_widget(root);
                let at = text_offset_to_position(tester, 5);
                tester.long_press_at(at, 1);
                tester.pump_and_settle();
                expect(tester, &find::text("Select all"), finds_one_widget());
                expect(tester, &find::text("Copy"), finds_one_widget());
                expect(tester, &find::text("Paste"), finds_nothing());
                expect(tester, &find::text("Cut"), finds_nothing());
                assert!(handles_are_visible(tester));
                assert!(toolbar_is_visible(tester));

                tester.tap(find::text("Select all"));
                tester.pump();
                expect(tester, &find::text("Copy"), finds_one_widget());
                expect(tester, &find::text("Select all"), finds_nothing());
                expect(tester, &find::text("Paste"), finds_nothing());
                expect(tester, &find::text("Cut"), finds_nothing());
                assert!(handles_are_visible(tester));
            });
        }
    }

    #[test]
    fn the_select_all_calls_on_selection_changed() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
                let recorded = Rc::clone(&seen);
                let root = material_home(
                    tester,
                    SelectableText::new("abc def ghi").on_selection_changed(
                        move |_app, selection, _cause| {
                            assert!(recorded.borrow().is_none());
                            *recorded.borrow_mut() = Some(selection);
                        },
                    ),
                );
                tester.pump_widget(root);
                let at = text_offset_to_position(tester, 5);
                tester.long_press_at(at, 1);
                tester.pump_and_settle();
                let selection = seen.borrow_mut().take().expect("the long press selects");
                assert_eq!(selection, TextSelection::new(4, 7));

                tester.tap(find::text("Select all"));
                tester.pump();
                let selection = seen.borrow_mut().take().expect("select all selects");
                assert_eq!(selection, TextSelection::new(0, 11));
            });
        }
    }

    #[test]
    fn selectable_text_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Center::new()
                    .child(SizedBox::shrink().child(SelectableText::new("XYZ")))
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(
                tester.get_size(find::by_type::<SelectableText>()),
                Size::ZERO
            );

            let state = editable_state(tester);
            state.update_editing_value(
                &mut tester.cell.borrow_mut(),
                TextEditingValue::new()
                    .text("XYZ")
                    .selection(TextSelection::new(0, 3)),
            );
            tester.pump();
        });
    }

    /// The global endpoints of `selection`, as Dart's `globalize` gives them.
    fn global_endpoints(tester: &WidgetTester, selection: TextSelection) -> Vec<Offset> {
        let render = render_editable_at(tester, 0);
        let mut app = tester.cell.borrow_mut();
        render
            .get_endpoints_for_selection(&mut app, selection)
            .into_iter()
            .map(|endpoint| render.as_box().local_to_global(&app, endpoint.point, None))
            .collect()
    }

    #[test]
    fn can_drag_handles_to_change_selection() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);
            let at = text_offset_to_position(tester, 5);
            tester.long_press_at(at, 7);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert_eq!(selection(tester), TextSelection::new(4, 7));

            let endpoints = global_endpoints(tester, TextSelection::new(4, 7));
            assert_eq!(endpoints.len(), 2);
            let mut gesture = TestPointer::touch(7);
            tester.down(&mut gesture, endpoints[1] + Offset::new(1.0, 1.0));
            tester.pump();
            tester.move_to(&mut gesture, text_offset_to_position(tester, 11), true);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(4, 11));

            tester.down(&mut gesture, endpoints[0] + Offset::new(-1.0, 1.0));
            tester.pump();
            tester.move_to(&mut gesture, text_offset_to_position(tester, 0), true);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(0, 11));
        });
    }

    #[test]
    fn dragging_handles_calls_on_selection_changed() {
        test_widgets(|tester| {
            let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
            let recorded = Rc::clone(&seen);
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi")
                    .drag_start_behavior(DragStartBehavior::Down)
                    .on_selection_changed(move |_app, selection, _cause| {
                        assert!(recorded.borrow().is_none());
                        *recorded.borrow_mut() = Some(selection);
                    }),
            );
            tester.pump_widget(root);
            let at = text_offset_to_position(tester, 5);
            tester.long_press_at(at, 7);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            let first = seen.borrow_mut().take().expect("the long press selects");
            assert_eq!(first, TextSelection::new(4, 7));

            let endpoints = global_endpoints(tester, first);
            let mut gesture = TestPointer::touch(7);
            tester.down(&mut gesture, endpoints[1] + Offset::new(1.0, 1.0));
            tester.pump();
            tester.move_to(&mut gesture, text_offset_to_position(tester, 9), true);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            let dragged = seen.borrow_mut().take().expect("the handle selects");
            assert_eq!(dragged, TextSelection::new(4, 9));
        });
    }

    #[test]
    fn cannot_drag_one_handle_past_the_other() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);
            let at = text_offset_to_position(tester, 5);
            tester.long_press_at(at, 7);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert_eq!(selection(tester), TextSelection::new(4, 7));

            let endpoints = global_endpoints(tester, TextSelection::new(4, 7));
            let mut gesture = TestPointer::touch(7);
            tester.down(&mut gesture, endpoints[1] + Offset::new(1.0, 1.0));
            tester.pump();
            tester.move_to(&mut gesture, text_offset_to_position(tester, 5), true);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(4, 5));

            tester.move_to(&mut gesture, text_offset_to_position(tester, 2), true);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(4, 5));
        });
    }

    #[test]
    fn can_drag_handles_to_change_selection_in_multiline() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new(K_THREE_LINES)
                    .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                    .max_lines(Some(3))
                    .drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);
            let at =
                text_offset_to_position(tester, K_THREE_LINES.find("until").unwrap() as i32 + 1);
            tester.long_press_at(at, 7);
            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert_eq!(selection(tester), TextSelection::new(39, 44));

            let endpoints = global_endpoints(tester, TextSelection::new(39, 44));
            assert_eq!(endpoints.len(), 2);
            let mut gesture = TestPointer::touch(7);
            tester.down(&mut gesture, endpoints[1] + Offset::new(1.0, 1.0));
            tester.pump();
            tester.move_to(
                &mut gesture,
                text_offset_to_position(tester, K_THREE_LINES.find("Third").unwrap() as i32 + 5)
                    + Offset::new(0.0, 3.0),
                true,
            );
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(selection(tester), TextSelection::new(39, 50));

            tester.down(&mut gesture, endpoints[0] + Offset::new(-1.0, 1.0));
            tester.pump();
            tester.move_to(
                &mut gesture,
                text_offset_to_position(tester, K_THREE_LINES.find("First").unwrap() as i32 + 5),
                true,
            );
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(selection(tester).base_offset, 5);
            assert_eq!(selection(tester).extent_offset, 50);
        });
    }

    #[test]
    fn continuous_dragging_does_not_cause_flickering() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi")
                    .drag_start_behavior(DragStartBehavior::Down)
                    .style(TextStyle::new().font_size(10.0)),
            );
            tester.pump_widget(root);
            let changed = Rc::new(Cell::new(0));
            let count = Rc::clone(&changed);
            controller(tester).add_listener(
                &mut tester.cell.borrow_mut(),
                Listener::new(move |_app| count.set(count.get() + 1)),
            );

            let c = text_offset_to_position(tester, 2);
            let g = text_offset_to_position(tester, 8);
            let h = text_offset_to_position(tester, 9);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, c);
            tester.pump();
            tester.move_to(&mut pointer, g, true);
            tester.pump_and_settle();
            assert!(changed.get() > 0);
            changed.set(0);
            assert_eq!(selection(tester), TextSelection::new(2, 8));

            tester.move_to(&mut pointer, g + Offset::new(4.0, 0.0), true);
            tester.pump_and_settle();
            assert_eq!(changed.get(), 0);

            tester.move_to(&mut pointer, h, true);
            tester.pump();
            tester.up(&pointer);
            tester.pump_and_settle();
            assert_eq!(changed.get(), 1);
            assert_eq!(selection(tester), TextSelection::new(2, 9));
        });
    }

    #[test]
    fn dragging_in_opposite_direction_also_works() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);
            let e = text_offset_to_position(tester, 5);
            let g = text_offset_to_position(tester, 8);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, g);
            tester.pump();
            tester.move_to(&mut pointer, e, true);
            tester.pump();
            tester.up(&pointer);
            tester.pump_and_settle();
            assert_eq!(selection(tester), TextSelection::new(8, 5));
        });
    }

    #[test]
    fn slow_mouse_dragging_also_selects_text() {
        test_widgets(|tester| {
            let root = material_home(
                tester,
                SelectableText::new("abc def ghi").drag_start_behavior(DragStartBehavior::Down),
            );
            tester.pump_widget(root);
            let e = text_offset_to_position(tester, 5);
            let g = text_offset_to_position(tester, 8);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, e);
            tester.pump_by(Duration::from_secs(2));
            tester.move_to(&mut pointer, g, true);
            tester.pump();
            tester.up(&pointer);
            assert_eq!(selection(tester), TextSelection::new(5, 8));
        });
    }

    #[test]
    fn selecting_a_space_selects_the_previous_word_on_mobile() {
        for platform in [TargetPlatform::IOS, TargetPlatform::Android] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
                let recorded = Rc::clone(&seen);
                let root = material_app(
                    ThemeData::new(&tester.cell.borrow()),
                    SelectableText::new(" blah blah")
                        .on_selection_changed(move |_app, selection, _cause| {
                            *recorded.borrow_mut() = Some(selection)
                        })
                        .into_widget(),
                );
                tester.pump_widget(root);
                assert!(seen.borrow().is_none());

                tester.tap_at(text_offset_to_position(tester, 10), 1);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (10, 10));

                tester.long_press_at(text_offset_to_position(tester, 5), 2);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(1, 5)));

                tester.tap_at(text_offset_to_position(tester, 10), 3);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (10, 10));
                tester.long_press_at(text_offset_to_position(tester, 0), 4);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(0, 1)));
            });
        }
    }

    #[test]
    fn double_tapping_a_space_selects_the_previous_word_on_mobile() {
        for platform in [TargetPlatform::IOS, TargetPlatform::Android] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
                let recorded = Rc::clone(&seen);
                let root = material_app(
                    ThemeData::builder()
                        .use_material3(false)
                        .build(&tester.cell.borrow()),
                    Center::new()
                        .child(
                            SelectableText::new(" blah blah  \n  blah").on_selection_changed(
                                move |_app, selection, _cause| {
                                    *recorded.borrow_mut() = Some(selection)
                                },
                            ),
                        )
                        .into_widget(),
                );
                tester.pump_widget(root);

                tester.tap_at(text_offset_to_position(tester, 19), 1);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (19, 19));
                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(text_offset_to_position(tester, 5), 2);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(text_offset_to_position(tester, 5), 3);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(1, 5)));

                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(text_offset_to_position(tester, 0), 4);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(text_offset_to_position(tester, 0), 5);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(0, 1)));

                tester.tap_at(text_offset_to_position(tester, 19), 6);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (19, 19));
                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(text_offset_to_position(tester, 14), 7);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(text_offset_to_position(tester, 14), 8);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(6, 14)));
            });
        }
    }

    #[test]
    #[should_panic(expected = "No Overlay widget found")]
    fn throw_if_no_overlay_widget_exists_above() {
        test_widgets(|tester| {
            let home: waft_widgets::WidgetRef = Center::new()
                .child(Material::new().child(SelectableText::new("I love Flutter!")))
                .into_widget();
            let media = waft_widgets::MediaQuery::new(
                MediaQueryData::new().size(Size::new(800.0, 600.0)),
                home,
            )
            .into_widget();
            let root = waft_widgets::Directionality::new(TextDirection::Ltr, media);
            tester.pump_widget(root);
            let start = top_left_of_selectable_text(tester, 0);
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, start);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
        });
    }

    #[test]
    fn page_view_beats_selectable_text_drag_gestures_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let page_controller = {
                let mut app = tester.cell.borrow_mut();
                waft_widgets::PageController::new(&mut app, 0, true, 1.0, None, None)
            };
            let test_value = "abc def ghi jkl mno pqr stu vwx yz";
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        waft_widgets::PageView::new()
                            .controller(page_controller.clone())
                            .children([
                                Center::new()
                                    .child(SelectableText::new(test_value))
                                    .into_widget(),
                                SizedBox::new()
                                    .height(200.0)
                                    .child(Center::new().child(Text::new("Page 2")))
                                    .into_widget(),
                            ]),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            tester.pump_and_settle();

            let g = text_offset_to_position(tester, test_value.find('g').unwrap() as i32);
            let p = text_offset_to_position(tester, test_value.find('p').unwrap() as i32);
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, g);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            tester.down(&mut gesture, g);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, p, true);
            tester.pump();
            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(
                selection(tester),
                TextSelection::new(
                    test_value.find('g').unwrap() as i32,
                    test_value.find('p').unwrap() as i32 + 3
                )
            );
            assert_eq!(
                page_controller.handle().page(&tester.cell.borrow()),
                Some(0.0)
            );

            let rect = tester.get_rect(find::by_type::<SelectableText>());
            tester.drag_from(
                rect.center_right() - Offset::new(0.1, 0.0),
                Offset::new(-500.0, 0.0),
                9,
            );
            tester.pump_and_settle();
            assert_eq!(
                page_controller.handle().page(&tester.cell.borrow()),
                Some(1.0)
            );
        });
    }

    #[test]
    fn the_select_all_calls_on_selection_changed_with_a_mouse_on_windows_and_linux() {
        for platform in [TargetPlatform::Windows, TargetPlatform::Linux] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
                let recorded = Rc::clone(&seen);
                let root = material_home(
                    tester,
                    SelectableText::new("abc def ghi").on_selection_changed(
                        move |_app, selection, _cause| {
                            assert!(recorded.borrow().is_none());
                            *recorded.borrow_mut() = Some(selection);
                        },
                    ),
                );
                tester.pump_widget(root);

                let e = text_offset_to_position(tester, 5);
                let mut gesture = TestPointer::mouse(1, waft_gestures::K_SECONDARY_BUTTON);
                tester.down(&mut gesture, e);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle();
                let first = seen.borrow_mut().take().expect("the right click selects");
                assert!(first.is_collapsed());
                assert_eq!(first.base_offset, 5);

                tester.tap(find::text("Select all"));
                tester.pump();
                let second = seen.borrow_mut().take().expect("select all selects");
                assert_eq!(second.base_offset, 0);
                assert_eq!(second.extent_offset, 11);
            });
        }
    }

    #[test]
    fn selecting_a_space_selects_the_space_on_non_mobile_platforms() {
        for platform in [
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
            TargetPlatform::Linux,
            TargetPlatform::Fuchsia,
        ] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let seen: Rc<RefCell<Option<TextSelection>>> = Rc::default();
                let recorded = Rc::clone(&seen);
                let root = material_app(
                    ThemeData::new(&tester.cell.borrow()),
                    Material::new()
                        .child(Center::new().child(
                            SelectableText::new(" blah blah").on_selection_changed(
                                move |_app, selection, _cause| {
                                    *recorded.borrow_mut() = Some(selection)
                                },
                            ),
                        ))
                        .into_widget(),
                );
                tester.pump_widget(root);
                assert!(seen.borrow().is_none());

                tester.tap_at(text_offset_to_position(tester, 10), 1);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (10, 10));

                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(text_offset_to_position(tester, 5), 2);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(text_offset_to_position(tester, 5), 3);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(5, 6)));

                tester.tap_at(text_offset_to_position(tester, 0), 4);
                tester.pump_and_settle();
                tester.tap_at(text_offset_to_position(tester, 10), 5);
                let current = seen.borrow_mut().take().expect("the tap selects");
                assert_eq!((current.base_offset, current.extent_offset), (10, 10));

                tester.pump_by(Duration::from_millis(500));
                tester.tap_at(text_offset_to_position(tester, 0), 6);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(text_offset_to_position(tester, 0), 7);
                tester.pump_and_settle();
                assert_eq!(seen.borrow_mut().take(), Some(TextSelection::new(0, 1)));
            });
        }
    }

    /// Pumps `widget` centred on a `Material` in a Material 2 app, as the selectable
    /// text tests do.
    fn pump_selectable(tester: &mut WidgetTester, widget: SelectableText) {
        let root = material_app(
            crate::theme_data::ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow()),
            Center::new()
                .child(Material::new().child(widget))
                .into_widget(),
        );
        tester.pump_widget(root);
    }

    fn selectable_size(tester: &WidgetTester) -> Size {
        tester.get_size(find::by_type::<SelectableText>().evaluate(tester)[0])
    }

    #[test]
    fn selectable_text_baseline_alignment_no_strut() {
        test_widgets(|tester| {
            let key_a: KeyRef = UniqueKey::new().into();
            let key_b: KeyRef = UniqueKey::new().into();
            let root = material_app(
                crate::theme_data::ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow()),
                Center::new()
                    .child(
                        Material::new().child(
                            waft_widgets::Row::new()
                                .cross_axis_alignment(waft_widgets::CrossAxisAlignment::Baseline)
                                .text_baseline(waft_embedder::TextBaseline::Alphabetic)
                                .children([
                                    waft_widgets::Expanded::new(
                                        SelectableText::new("A")
                                            .key(key_a.clone())
                                            .strut_style(StrutStyle::DISABLED)
                                            .style(
                                                TextStyle::new()
                                                    .font_family("FlutterTest")
                                                    .font_size(10.0),
                                            ),
                                    )
                                    .into_widget(),
                                    Text::new("abc")
                                        .style(
                                            TextStyle::new()
                                                .font_family("FlutterTest")
                                                .font_size(20.0),
                                        )
                                        .into_widget(),
                                    waft_widgets::Expanded::new(
                                        SelectableText::new("B")
                                            .key(key_b.clone())
                                            .strut_style(StrutStyle::DISABLED)
                                            .style(
                                                TextStyle::new()
                                                    .font_family("FlutterTest")
                                                    .font_size(30.0),
                                            ),
                                    )
                                    .into_widget(),
                                ]),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            let row_bottom = tester
                .get_bottom_left(find::by_type::<waft_widgets::Row>())
                .dy();
            assert_eq!(
                tester.get_bottom_left(find::by_key(key_a)).dy(),
                row_bottom - 5.0
            );
            assert_eq!(
                tester.get_bottom_left(find::text("abc")).dy(),
                row_bottom - 2.5
            );
            assert_eq!(tester.get_bottom_left(find::by_key(key_b)).dy(), row_bottom);
        });
    }

    #[test]
    fn strut_basic_single_line() {
        test_widgets(|tester| {
            pump_selectable(tester, SelectableText::new("something"));
            assert_eq!(selectable_size(tester), Size::new(129.0, 14.0));
        });
    }

    #[test]
    fn strut_text_style_increases_height() {
        test_widgets(|tester| {
            pump_selectable(
                tester,
                SelectableText::new("something").style(TextStyle::new().font_size(20.0)),
            );
            assert_eq!(selectable_size(tester), Size::new(183.0, 20.0));

            pump_selectable(
                tester,
                SelectableText::new("something")
                    .style(TextStyle::new().font_size(20.0))
                    .strut_style(StrutStyle::DISABLED),
            );
            assert_eq!(selectable_size(tester), Size::new(183.0, 20.0));
        });
    }

    #[test]
    fn strut_basic_multi_line() {
        test_widgets(|tester| {
            pump_selectable(tester, SelectableText::new("something").max_lines(Some(6)));
            assert_eq!(selectable_size(tester), Size::new(129.0, 84.0));
        });
    }

    #[test]
    fn strut_no_force_small_strut() {
        test_widgets(|tester| {
            pump_selectable(
                tester,
                SelectableText::new("something")
                    .max_lines(Some(6))
                    .strut_style(StrutStyle::new().font_size(5.0)),
            );
            assert_eq!(selectable_size(tester), Size::new(129.0, 84.0));
        });
    }

    #[test]
    fn strut_no_force_large_strut() {
        test_widgets(|tester| {
            pump_selectable(
                tester,
                SelectableText::new("something")
                    .max_lines(Some(6))
                    .strut_style(StrutStyle::new().font_size(25.0)),
            );
            assert_eq!(selectable_size(tester), Size::new(129.0, 150.0));
        });
    }

    #[test]
    fn strut_height_override() {
        test_widgets(|tester| {
            pump_selectable(
                tester,
                SelectableText::new("something")
                    .max_lines(Some(3))
                    .strut_style(StrutStyle::new().font_size(8.0).force_strut_height(true)),
            );
            assert_eq!(selectable_size(tester), Size::new(129.0, 24.0));
        });
    }

    #[test]
    fn strut_forces_field_taller() {
        test_widgets(|tester| {
            pump_selectable(
                tester,
                SelectableText::new("something")
                    .max_lines(Some(3))
                    .style(TextStyle::new().font_size(10.0))
                    .strut_style(StrutStyle::new().font_size(18.0).force_strut_height(true)),
            );
            assert_eq!(selectable_size(tester), Size::new(93.0, 54.0));
        });
    }

    #[test]
    fn selectable_text_baseline_alignment() {
        test_widgets(|tester| {
            let key_a: KeyRef = UniqueKey::new().into();
            let key_b: KeyRef = UniqueKey::new().into();
            let root = material_app(
                crate::theme_data::ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow()),
                Center::new()
                    .child(
                        Material::new().child(
                            waft_widgets::Row::new()
                                .cross_axis_alignment(waft_widgets::CrossAxisAlignment::Baseline)
                                .text_baseline(waft_embedder::TextBaseline::Alphabetic)
                                .children([
                                    waft_widgets::Expanded::new(
                                        SelectableText::new("A").key(key_a.clone()).style(
                                            TextStyle::new()
                                                .font_family("FlutterTest")
                                                .font_size(10.0),
                                        ),
                                    )
                                    .into_widget(),
                                    Text::new("abc")
                                        .style(
                                            TextStyle::new()
                                                .font_family("FlutterTest")
                                                .font_size(20.0),
                                        )
                                        .into_widget(),
                                    waft_widgets::Expanded::new(
                                        SelectableText::new("B").key(key_b.clone()).style(
                                            TextStyle::new()
                                                .font_family("FlutterTest")
                                                .font_size(30.0),
                                        ),
                                    )
                                    .into_widget(),
                                ]),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            let row_bottom = tester
                .get_bottom_left(find::by_type::<waft_widgets::Row>())
                .dy();
            assert_eq!(
                tester.get_bottom_left(find::by_key(key_a)).dy(),
                row_bottom - 5.0
            );
            assert_eq!(
                tester.get_bottom_left(find::text("abc")).dy(),
                row_bottom - 2.5
            );
            assert_eq!(tester.get_bottom_left(find::by_key(key_b)).dy(), row_bottom);
        });
    }
    #[test]
    fn can_align_to_center() {
        test_widgets(|tester| {
            let root = material_app(
                crate::theme_data::ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow()),
                waft_widgets::MediaQuery::new(
                    MediaQueryData::new().size(Size::new(800.0, 600.0)),
                    SizedBox::new()
                        .width(300.0)
                        .child(SelectableText::new("abcd").text_align(TextAlign::Center)),
                )
                .into_widget(),
            );
            tester.pump_widget(root);
            let editable = render_editable_at(tester, 0);
            let caret_at = |tester: &mut WidgetTester, offset: i32| {
                let mut app = tester.cell.borrow_mut();
                let rect = editable.get_local_rect_for_caret(&mut app, TextPosition::new(offset));
                editable
                    .as_box()
                    .local_to_global(&app, rect.top_left(), None)
            };
            assert_eq!(caret_at(tester, 2).dx(), 399.0);
        });
    }

    #[test]
    fn can_align_to_center_within_center() {
        test_widgets(|tester| {
            let root = material_app(
                crate::theme_data::ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow()),
                waft_widgets::MediaQuery::new(
                    MediaQueryData::new().size(Size::new(800.0, 600.0)),
                    SizedBox::new().width(300.0).child(
                        Center::new()
                            .child(SelectableText::new("abcd").text_align(TextAlign::Center)),
                    ),
                )
                .into_widget(),
            );
            tester.pump_widget(root);
            let editable = render_editable_at(tester, 0);
            let caret_at = |tester: &mut WidgetTester, offset: i32| {
                let mut app = tester.cell.borrow_mut();
                let rect = editable.get_local_rect_for_caret(&mut app, TextPosition::new(offset));
                editable
                    .as_box()
                    .local_to_global(&app, rect.top_left(), None)
            };
            assert_eq!(caret_at(tester, 2).dx(), 399.0);
        });
    }
}
