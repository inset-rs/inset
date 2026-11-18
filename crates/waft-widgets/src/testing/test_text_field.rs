//! Flutter's `TestTextField` (`packages/flutter/test/widgets/editable_text_tester.dart`): an
//! [`EditableText`] a case types into without a Material or Cupertino text field.
//!
//! The field owns a controller and a focus node unless it is given them, so a case can read
//! its text from the field alone. Its selection handles take no room and it has no context
//! menu of its own.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Color, Offset, Size};
use waft_foundation::{App, Handle, Ref};
use waft_rendering::HitTestBehavior;
use waft_services::AnyTextSelectionDelegate;

use crate::{
    BuildContext, EditableText, FocusNode, FocusNodeLeaf, FocusNodeRef, GlobalKey, IntoWidget,
    KeyRef, Semantics, SizedBox, State, StateData, StatefulWidget, TextEditingController,
    TextEditingControllerRef, TextSelectionControls, TextSelectionGestureDetectorBuilder,
    TextSelectionGestureDetectorBuilderData, TextSelectionGestureDetectorBuilderDelegate,
    TextSelectionHandleControls, TextSelectionHandleType, WidgetRef,
};

/// A minimal wrapper around [`EditableText`] for use in widget tests.
///
/// It provides text selection gestures through [`TextSelectionGestureDetector`], and fills the
/// members [`EditableText`] requires: a controller, a focus node, a style and two cursor
/// colors. Selection handles are not shown unless [`show_selection_handles`](Self::show_selection_handles)
/// is set.
///
/// [`TextSelectionGestureDetector`]: crate::TextSelectionGestureDetector
pub struct TestTextField {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The controller the field edits, or one the field owns.
    pub controller: Option<TextEditingControllerRef>,

    /// The focus node the field uses, or one the field owns.
    pub focus_node: Option<FocusNodeRef>,

    /// Called when the platform submits the field's text.
    pub on_submitted: Option<SubmittedCallback>,

    /// Whether the field takes the focus when it is first built.
    pub autofocus: bool,

    /// The number of lines the field shows, or `None` for no limit.
    pub max_lines: Option<i32>,

    /// Whether the field only shows its text.
    pub read_only: bool,

    /// The text style of the field's text.
    pub style: Option<waft_painting::TextStyle>,

    /// The color of the cursor while the field is focused, or red.
    pub cursor_color: Option<Color>,

    /// Whether the cursor fades in and out rather than blinking.
    pub cursor_opacity_animates: bool,

    /// Whether the field shows its cursor, or the platform's default.
    pub show_cursor: Option<bool>,

    /// Whether the field shows its selection handles.
    pub show_selection_handles: bool,

    /// The selection controls the field uses, or test handles that take no room.
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,

    /// Whether all text is selected when the field receives focus, or the platform's default.
    pub select_all_on_focus: Option<bool>,
}

/// `TestTextField.on_submitted`'s type.
pub type SubmittedCallback = Rc<dyn Fn(&mut App, String)>;

impl TestTextField {
    /// Creates a minimal [`EditableText`] wrapper for use in widget tests.
    pub fn new() -> TestTextField {
        TestTextField {
            key: None,
            controller: None,
            focus_node: None,
            on_submitted: None,
            autofocus: false,
            max_lines: Some(1),
            read_only: false,
            style: None,
            cursor_color: None,
            cursor_opacity_animates: false,
            show_cursor: None,
            show_selection_handles: false,
            selection_controls: None,
            select_all_on_focus: None,
        }
    }

    /// Dart `TestTextField(key:)`.
    pub fn key(mut self, key: &GlobalKey) -> TestTextField {
        self.key = Some(key.clone().into());
        self
    }

    /// Dart `TestTextField(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> TestTextField {
        self.controller = Some(controller);
        self
    }

    /// Dart `TestTextField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> TestTextField {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `TestTextField(onSubmitted:)`.
    pub fn on_submitted(
        mut self,
        on_submitted: impl Fn(&mut App, String) + 'static,
    ) -> TestTextField {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `TestTextField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> TestTextField {
        self.autofocus = autofocus;
        self
    }

    /// Dart `TestTextField(maxLines:)`.
    pub fn max_lines(mut self, max_lines: i32) -> TestTextField {
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `TestTextField(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> TestTextField {
        self.read_only = read_only;
        self
    }

    /// Dart `TestTextField(style:)`.
    pub fn style(mut self, style: waft_painting::TextStyle) -> TestTextField {
        self.style = Some(style);
        self
    }

    /// Dart `TestTextField(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: Color) -> TestTextField {
        self.cursor_color = Some(cursor_color);
        self
    }

    /// Dart `TestTextField(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(mut self, cursor_opacity_animates: bool) -> TestTextField {
        self.cursor_opacity_animates = cursor_opacity_animates;
        self
    }

    /// Dart `TestTextField(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> TestTextField {
        self.show_cursor = Some(show_cursor);
        self
    }

    /// Dart `TestTextField(showSelectionHandles:)`.
    pub fn show_selection_handles(mut self, show_selection_handles: bool) -> TestTextField {
        self.show_selection_handles = show_selection_handles;
        self
    }

    /// Dart `TestTextField(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> TestTextField {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `TestTextField(selectAllOnFocus:)`.
    pub fn select_all_on_focus(mut self, select_all_on_focus: bool) -> TestTextField {
        self.select_all_on_focus = Some(select_all_on_focus);
        self
    }
}

impl Default for TestTextField {
    fn default() -> TestTextField {
        TestTextField::new()
    }
}

impl Debug for TestTextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestTextField").finish_non_exhaustive()
    }
}

impl StatefulWidget for TestTextField {
    type State = TestTextFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TestTextFieldState {
        TestTextFieldState {
            state: StateData::new(),
            controller: None,
            focus_node: None,
            editable_text_key: GlobalKey::new(),
            selection_gesture_detector_builder: None,
        }
    }
}

/// The state of a [`TestTextField`].
pub struct TestTextFieldState {
    state: StateData<TestTextField>,
    controller: Option<Ref<TextEditingController>>,
    focus_node: Option<Ref<FocusNode>>,
    editable_text_key: GlobalKey,
    selection_gesture_detector_builder: Option<Ref<TestTextFieldSelectionGestureDetectorBuilder>>,
}

impl TestTextFieldState {
    /// The controller the field edits: the one it was given, or one it made.
    fn effective_controller(self: Handle<Self>, app: &mut App) -> TextEditingControllerRef {
        if let Some(controller) = &self.widget(app).controller {
            return controller.clone();
        }
        if app.get(self).controller.is_none() {
            let controller = TextEditingController::new(app);
            app.get_mut(self).controller = Some(controller);
        }
        app.get(self)
            .controller
            .as_ref()
            .expect("made above")
            .erase()
    }

    /// The focus node the field uses: the one it was given, or one it made.
    fn effective_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = &self.widget(app).focus_node {
            return focus_node.clone();
        }
        if app.get(self).focus_node.is_none() {
            let focus_node = FocusNode::new(app);
            app.get_mut(self).focus_node = Some(focus_node);
        }
        app.get(self)
            .focus_node
            .as_ref()
            .expect("made above")
            .erase()
    }
}

impl TextSelectionGestureDetectorBuilderDelegate for TestTextFieldState {
    fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).editable_text_key.clone()
    }

    fn force_press_enabled(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn selection_enabled(self: Handle<Self>, _app: &App) -> bool {
        true
    }
}

impl State for TestTextFieldState {
    type Widget = TestTextField;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let builder = app.create(TestTextFieldSelectionGestureDetectorBuilder {
            builder: TextSelectionGestureDetectorBuilderData::new(
                self.as_text_selection_gesture_detector_builder_delegate(),
            ),
        });
        app.get_mut(self).selection_gesture_detector_builder = Some(builder);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(focus_node) = app.get(self).focus_node.clone() {
            focus_node.dispose(app);
        }
        if let Some(controller) = app.get(self).controller.clone() {
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        const RED: Color = Color::new(0xFFF44336);
        let controller = self.effective_controller(app);
        let focus_node = self.effective_focus_node(app);
        let widget = self.widget(app);
        let mut field = EditableText::new(
            controller,
            focus_node.clone(),
            widget.style.clone().unwrap_or_default(),
            widget.cursor_color.unwrap_or(RED),
            RED,
        )
        .key(app.get(self).editable_text_key.clone().into())
        .renderer_ignores_pointer(true)
        .selection_controls(
            widget
                .selection_controls
                .clone()
                .unwrap_or_else(|| Rc::new(TestTextSelectionHandleControls)),
        )
        .autofocus(widget.autofocus)
        .read_only(widget.read_only)
        .show_selection_handles(widget.show_selection_handles)
        .cursor_opacity_animates(widget.cursor_opacity_animates);
        if let Some(max_lines) = widget.max_lines {
            field = field.max_lines(Some(max_lines));
        }
        if let Some(show_cursor) = widget.show_cursor {
            field = field.show_cursor(show_cursor);
        }
        if let Some(select_all_on_focus) = widget.select_all_on_focus {
            field = field.select_all_on_focus(select_all_on_focus);
        }
        if let Some(on_submitted) = widget.on_submitted.clone() {
            field = field.on_submitted(move |app, value| on_submitted(app, value));
        }
        let builder = app
            .get(self)
            .selection_gesture_detector_builder
            .as_ref()
            .expect("made in init_state")
            .handle();
        let gestured = TextSelectionGestureDetectorBuilder::build_gesture_detector(
            builder,
            app,
            None,
            Some(HitTestBehavior::Translucent),
            field.into_widget(),
        );
        Semantics::new()
            .enabled(true)
            .on_tap(Rc::new(move |app: &mut App| {
                focus_node.handle().request_focus(app, None)
            }))
            .child(gestured)
            .into_widget()
    }
}

/// Dart's `_TestTextFieldSelectionGestureDetectorBuilder`: the base builder, with no `onTap` of
/// its own.
struct TestTextFieldSelectionGestureDetectorBuilder {
    builder: TextSelectionGestureDetectorBuilderData,
}

impl TextSelectionGestureDetectorBuilder for TestTextFieldSelectionGestureDetectorBuilder {
    crate::text_selection_gesture_detector_builder_accessors!(builder);
}

/// Dart's `TestTextSelectionHandleControls`: handles that take no room.
pub struct TestTextSelectionHandleControls;

#[allow(deprecated)]
impl TextSelectionControls for TestTextSelectionHandleControls {
    fn build_handle(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _handle_type: TextSelectionHandleType,
        _text_line_height: f64,
        _on_tap: Option<waft_foundation::Listener>,
    ) -> WidgetRef {
        SizedBox::shrink().into_widget()
    }

    fn get_handle_anchor(
        &self,
        _handle_type: TextSelectionHandleType,
        _text_line_height: f64,
    ) -> Offset {
        Offset::ZERO
    }

    fn build_toolbar(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _global_editable_region: waft_embedder::Rect,
        _text_line_height: f64,
        _selection_midpoint: Offset,
        _endpoints: &[waft_rendering::TextSelectionPoint],
        _delegate: AnyTextSelectionDelegate,
        _clipboard_status: Option<Ref<crate::ClipboardStatusNotifier>>,
        _last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        SizedBox::shrink().into_widget()
    }

    fn get_handle_size(&self, _text_line_height: f64) -> Size {
        Size::ZERO
    }

    fn as_text_selection_handle_controls(&self) -> Option<&dyn TextSelectionHandleControls> {
        Some(self)
    }
}

impl TextSelectionHandleControls for TestTextSelectionHandleControls {}

/// A [`TestTextField`]'s selection controls: test handles that take no room.
pub fn test_text_selection_handle_controls() -> Rc<dyn TextSelectionControls> {
    Rc::new(TestTextSelectionHandleControls)
}
