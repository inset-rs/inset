//! Flutter counterpart: `services/text_input.dart`.
//!
//! The value types live in `reveal-embedder` because [`reveal_embedder::View`]
//! and [`reveal_embedder::EmbedderClient`] name them. This file is the
//! connection, the client, and [`TextInput`].

use reveal_embedder::{
    FontWeight, Matrix4, Offset, Rect, Size, TargetPlatform, TextAlign, TextDirection, ViewId,
    ViewRef,
};
use reveal_foundation::{App, Handle, HandleId, Listener};

pub use reveal_embedder::{
    AutofillConfiguration, FloatingCursorDragState, RawFloatingCursorPoint, SmartDashesType,
    SmartQuotesType, TextCapitalization, TextEditingValue, TextInputAction, TextInputConfiguration,
    TextInputType,
};

/// Indicates what triggered the change in selected text (including changes to
/// the cursor location).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionChangedCause {
    /// The user tapped on the text and that caused the selection (or the location
    /// of the cursor) to change.
    Tap,

    /// The user tapped twice in quick succession on the text and that caused
    /// the selection (or the location of the cursor) to change.
    DoubleTap,

    /// The user long-pressed the text and that caused the selection (or the
    /// location of the cursor) to change.
    LongPress,

    /// The user force-pressed the text and that caused the selection (or the
    /// location of the cursor) to change.
    ForcePress,

    /// The user used the keyboard to change the selection or the location of the
    /// cursor.
    ///
    /// Keyboard-triggered selection changes may be caused by the IME as well as
    /// by accessibility tools (e.g. TalkBack on Android).
    Keyboard,

    /// The user used the selection toolbar to change the selection or the
    /// location of the cursor.
    ///
    /// An example is when the user taps on select all in the tool bar.
    Toolbar,

    /// The user used the mouse to change the selection by dragging over a piece
    /// of text.
    Drag,

    /// The user used stylus handwriting to change the selection.
    ///
    /// Currently, this is only supported on iPadOS 14+ via the Scribble feature,
    /// or on Android API 34+ via the Scribe feature.
    StylusHandwriting,
}

/// Represents a selection rect for a character and it's position in the text.
///
/// This is used to report the current text selection rect and position data
/// to the engine for Scribble support on iPadOS 14.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionRect {
    /// The position of this selection rect within the text String.
    pub position: i32,
    /// The rectangle representing the bounds of this selection rect within the
    /// currently focused `RenderEditable`'s coordinate space.
    pub bounds: Rect,
    /// The direction text flows within this selection rect.
    pub direction: TextDirection,
}

impl SelectionRect {
    /// Constructor for creating a [`SelectionRect`] from a text `position` and
    /// `bounds`.
    pub const fn new(position: i32, bounds: Rect, direction: TextDirection) -> SelectionRect {
        SelectionRect {
            position,
            bounds,
            direction,
        }
    }
}

impl std::fmt::Display for SelectionRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SelectionRect({}, {:?})", self.position, self.bounds)
    }
}

/// Text styling information for the current input client.
///
/// See also:
///
///  * [`TextInputConnection::update_style`], which uses this class to send style
///    information to the platform.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputStyle {
    /// The name of the font to use when painting the text (e.g., Roboto).
    pub font_family: Option<String>,
    /// The size of fonts (in logical pixels) to use when painting the text.
    pub font_size: Option<f64>,
    /// The typeface thickness to use when painting the text (e.g., bold).
    pub font_weight: Option<FontWeight>,
    /// The directionality of the text.
    pub text_direction: TextDirection,
    /// How the text should be aligned horizontally.
    pub text_align: TextAlign,
    /// The amount of space (in logical pixels) to add between each letter.
    pub letter_spacing: Option<f64>,
    /// The amount of space (in logical pixels) to add at each sequence of
    /// white-space (i.e. between each word).
    pub word_spacing: Option<f64>,
    /// The line height in logical pixels.
    pub line_height: Option<f64>,
}

impl TextInputStyle {
    /// Creates text styling information for the current input client.
    pub fn new(text_direction: TextDirection, text_align: TextAlign) -> TextInputStyle {
        TextInputStyle {
            font_family: None,
            font_size: None,
            font_weight: None,
            text_direction,
            text_align,
            letter_spacing: None,
            word_spacing: None,
            line_height: None,
        }
    }
}

/// An interface to receive information from [`TextInput`].
///
/// See also:
///
///  * [`TextInput::attach`]
///  * `EditableText`, a [`TextInputClient`] implementation.
pub trait TextInputClient: Sized + 'static {
    /// The current state of the [`TextEditingValue`] held by this client.
    fn current_text_editing_value(self: Handle<Self>, app: &App) -> Option<TextEditingValue>;

    /// Requests that this client update its editing state to the given value.
    ///
    /// The new `value` is treated as user input and thus may subject to input
    /// formatting.
    fn update_editing_value(self: Handle<Self>, app: &mut App, value: TextEditingValue);

    /// Requests that this client perform the given action.
    fn perform_action(self: Handle<Self>, app: &mut App, action: TextInputAction);

    /// Request from the input method that this client perform the given private
    /// command.
    fn perform_private_command(self: Handle<Self>, app: &mut App, action: &str) {
        let _ = (self, app, action);
    }

    /// Updates the floating cursor position and state.
    fn update_floating_cursor(self: Handle<Self>, app: &mut App, point: RawFloatingCursorPoint) {
        let _ = (self, app, point);
    }

    /// Requests that this client display a prompt rectangle for the given text range,
    /// to indicate the range of text that will be changed by a pending autocorrection.
    fn show_autocorrection_prompt_rect(self: Handle<Self>, app: &mut App, start: i32, end: i32) {
        let _ = (self, app, start, end);
    }

    /// Platform notified framework of closed connection.
    ///
    /// [`TextInputClient`] should cleanup its connection and finalize editing.
    fn connection_closed(self: Handle<Self>, app: &mut App);

    /// Requests that the client show the editing toolbar, for example when the
    /// platform changes the selection through a non-flutter method such as
    /// scribble.
    fn show_toolbar(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Requests that the client add a text placeholder to reserve visual space
    /// in the text.
    fn insert_text_placeholder(self: Handle<Self>, app: &mut App, size: Size) {
        let _ = (self, app, size);
    }

    /// Requests that the client remove the text placeholder.
    fn remove_text_placeholder(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Performs the specified MacOS-specific selector from the
    /// `NSStandardKeyBindingResponding` protocol or user-specified selector
    /// from `DefaultKeyBinding.Dict`.
    fn perform_selector(self: Handle<Self>, app: &mut App, selector_name: &str) {
        let _ = (self, app, selector_name);
    }

    /// This client as the erased [`AnyTextInputClient`].
    fn as_text_input_client(self: Handle<Self>) -> AnyTextInputClient {
        AnyTextInputClient {
            id: self.id(),
            vtable: const { &TextInputClientVTable::of::<Self>() },
        }
    }
}

struct TextInputClientVTable {
    current_text_editing_value: fn(&App, HandleId) -> Option<TextEditingValue>,
    update_editing_value: fn(&mut App, HandleId, TextEditingValue),
    perform_action: fn(&mut App, HandleId, TextInputAction),
    perform_private_command: fn(&mut App, HandleId, &str),
    update_floating_cursor: fn(&mut App, HandleId, RawFloatingCursorPoint),
    show_autocorrection_prompt_rect: fn(&mut App, HandleId, i32, i32),
    connection_closed: fn(&mut App, HandleId),
    show_toolbar: fn(&mut App, HandleId),
    insert_text_placeholder: fn(&mut App, HandleId, Size),
    remove_text_placeholder: fn(&mut App, HandleId),
    perform_selector: fn(&mut App, HandleId, &str),
}

fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl TextInputClientVTable {
    const fn of<C: TextInputClient>() -> TextInputClientVTable {
        TextInputClientVTable {
            current_text_editing_value: |app, id| C::current_text_editing_value(resolve(id), app),
            update_editing_value: |app, id, value| C::update_editing_value(resolve(id), app, value),
            perform_action: |app, id, action| C::perform_action(resolve(id), app, action),
            perform_private_command: |app, id, action| {
                C::perform_private_command(resolve(id), app, action)
            },
            update_floating_cursor: |app, id, point| {
                C::update_floating_cursor(resolve(id), app, point)
            },
            show_autocorrection_prompt_rect: |app, id, start, end| {
                C::show_autocorrection_prompt_rect(resolve(id), app, start, end)
            },
            connection_closed: |app, id| C::connection_closed(resolve(id), app),
            show_toolbar: |app, id| C::show_toolbar(resolve(id), app),
            insert_text_placeholder: |app, id, size| {
                C::insert_text_placeholder(resolve(id), app, size)
            },
            remove_text_placeholder: |app, id| C::remove_text_placeholder(resolve(id), app),
            perform_selector: |app, id, selector_name| {
                C::perform_selector(resolve(id), app, selector_name)
            },
        }
    }
}

/// Erased [`TextInputClient`]: one identity and a static vtable.
#[derive(Clone, Copy)]
pub struct AnyTextInputClient {
    id: HandleId,
    vtable: &'static TextInputClientVTable,
}

impl AnyTextInputClient {
    pub fn current_text_editing_value(self, app: &App) -> Option<TextEditingValue> {
        (self.vtable.current_text_editing_value)(app, self.id)
    }

    pub fn update_editing_value(self, app: &mut App, value: TextEditingValue) {
        (self.vtable.update_editing_value)(app, self.id, value);
    }

    pub fn perform_action(self, app: &mut App, action: TextInputAction) {
        (self.vtable.perform_action)(app, self.id, action);
    }

    pub fn perform_private_command(self, app: &mut App, action: &str) {
        (self.vtable.perform_private_command)(app, self.id, action);
    }

    pub fn update_floating_cursor(self, app: &mut App, point: RawFloatingCursorPoint) {
        (self.vtable.update_floating_cursor)(app, self.id, point);
    }

    pub fn show_autocorrection_prompt_rect(self, app: &mut App, start: i32, end: i32) {
        (self.vtable.show_autocorrection_prompt_rect)(app, self.id, start, end);
    }

    pub fn connection_closed(self, app: &mut App) {
        (self.vtable.connection_closed)(app, self.id);
    }

    pub fn show_toolbar(self, app: &mut App) {
        (self.vtable.show_toolbar)(app, self.id);
    }

    pub fn insert_text_placeholder(self, app: &mut App, size: Size) {
        (self.vtable.insert_text_placeholder)(app, self.id, size);
    }

    pub fn remove_text_placeholder(self, app: &mut App) {
        (self.vtable.remove_text_placeholder)(app, self.id);
    }

    pub fn perform_selector(self, app: &mut App, selector_name: &str) {
        (self.vtable.perform_selector)(app, self.id, selector_name);
    }
}

impl PartialEq for AnyTextInputClient {
    fn eq(&self, other: &AnyTextInputClient) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyTextInputClient {}

/// An interface for interacting with a text input control.
///
/// See also:
///
///  * [`TextInput::attach`], a method used to establish a [`TextInputConnection`]
///    between the system's text input and a [`TextInputClient`].
pub struct TextInputConnection {
    #[allow(dead_code)]
    id: i32,
    client: AnyTextInputClient,
    cached_size: Option<Size>,
    cached_transform: Option<Matrix4>,
    cached_rect: Option<Rect>,
    cached_caret_rect: Option<Rect>,
    cached_selection_rects: Vec<SelectionRect>,
}

impl TextInputConnection {
    /// Whether this connection is currently interacting with the text input control.
    pub fn attached(self: Handle<Self>, app: &mut App) -> bool {
        let input = TextInput::instance(app);
        app.get(input)
            .current_connection
            .is_some_and(|current| current == self)
    }

    /// Requests that the text input control become visible.
    pub fn show(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.attached(app));
        TextInput::instance(app).show(app);
    }

    /// Requests that the text input control update itself according to the new
    /// [`TextInputConfiguration`].
    pub fn update_config(self: Handle<Self>, app: &mut App, configuration: TextInputConfiguration) {
        debug_assert!(self.attached(app));
        TextInput::instance(app).update_config(app, configuration);
    }

    /// Requests that the text input control change its internal state to match
    /// the given state.
    pub fn set_editing_state(self: Handle<Self>, app: &mut App, value: TextEditingValue) {
        debug_assert!(self.attached(app));
        TextInput::instance(app).set_editing_state(app, value);
    }

    /// Send the size and transform of the editable text to engine.
    ///
    /// 1. `editable_box_size`: size of the render editable box.
    ///
    /// 2. `transform`: a matrix that maps the local paint coordinate system
    ///    to the `PipelineOwner.rootNode`.
    pub fn set_editable_size_and_transform(
        self: Handle<Self>,
        app: &mut App,
        editable_box_size: Size,
        transform: Matrix4,
    ) {
        let unchanged = {
            let connection = app.get(self);
            connection.cached_size == Some(editable_box_size)
                && connection.cached_transform.as_ref() == Some(&transform)
        };
        if unchanged {
            return;
        }
        app.get_mut(self).cached_size = Some(editable_box_size);
        app.get_mut(self).cached_transform = Some(transform);
        TextInput::instance(app).set_editable_size_and_transform(app, editable_box_size, transform);
    }

    /// Send the smallest rect that covers the text in the client that's currently
    /// being composed.
    ///
    /// If any of the 4 coordinates of the given [`Rect`] is not finite, a [`Rect`] of
    /// size (-1, -1) will be sent instead.
    pub fn set_composing_rect(self: Handle<Self>, app: &mut App, rect: Rect) {
        if app.get(self).cached_rect == Some(rect) {
            return;
        }
        app.get_mut(self).cached_rect = Some(rect);
        let valid_rect = if rect.is_finite() {
            rect
        } else {
            Offset::ZERO & Size::new(-1.0, -1.0)
        };
        TextInput::instance(app).set_composing_text_rect(app, valid_rect);
    }

    /// Sends the coordinates of caret rect. This is used on macOS for positioning
    /// the accent selection menu.
    pub fn set_caret_rect(self: Handle<Self>, app: &mut App, rect: Rect) {
        if app.get(self).cached_caret_rect == Some(rect) {
            return;
        }
        app.get_mut(self).cached_caret_rect = Some(rect);
        let valid_rect = if rect.is_finite() {
            rect
        } else {
            Offset::ZERO & Size::new(-1.0, -1.0)
        };
        TextInput::instance(app).set_caret_rect(app, valid_rect);
    }

    /// Send the bounding boxes of the current selected glyphs in the client to
    /// the platform's text input plugin.
    pub fn set_selection_rects(
        self: Handle<Self>,
        app: &mut App,
        selection_rects: Vec<SelectionRect>,
    ) {
        if app.get(self).cached_selection_rects == selection_rects {
            return;
        }
        app.get_mut(self).cached_selection_rects = selection_rects;
    }

    /// Send text styling information.
    pub fn update_style(self: Handle<Self>, app: &mut App, style: TextInputStyle) {
        debug_assert!(self.attached(app));
        let _ = style;
    }

    /// Stop interacting with the text input control.
    ///
    /// After calling this method, the text input control might disappear if no
    /// other client attaches to it within this animation frame.
    pub fn close(self: Handle<Self>, app: &mut App) {
        if self.attached(app) {
            TextInput::instance(app).clear_client(app);
        }
        debug_assert!(!self.attached(app));
    }

    /// Platform sent a notification informing the connection is closed.
    ///
    /// [`TextInputConnection`] should clean current client connection.
    pub fn connection_closed_received(self: Handle<Self>, app: &mut App) {
        let input = TextInput::instance(app);
        app.get_mut(input).current_connection = None;
        debug_assert!(!self.attached(app));
    }
}

/// An low-level interface to the system's text input control.
///
/// To start interacting with the system's text input control, call [`attach`](Self::attach) to
/// establish a [`TextInputConnection`] between the system's text input control
/// and a [`TextInputClient`]. The majority of commands available for
/// interacting with the text input control reside in the returned
/// [`TextInputConnection`].
///
/// Dart's `TextInput._instance` is [`instance`](Self::instance), the App's singleton.
pub struct TextInput {
    current_connection: Option<Handle<TextInputConnection>>,
    current_configuration: Option<TextInputConfiguration>,
    #[allow(dead_code)]
    last_connection: Option<Handle<TextInputConnection>>,
    next_id: i32,
    hide_pending: bool,
}

impl Default for TextInput {
    fn default() -> TextInput {
        TextInput {
            current_connection: None,
            current_configuration: None,
            last_connection: None,
            next_id: 1,
            hide_pending: false,
        }
    }
}

impl TextInput {
    /// Dart's `TextInput._instance`.
    pub fn instance(app: &mut App) -> Handle<TextInput> {
        app.singleton::<TextInput>()
    }

    /// Ensure that a [`TextInput`] instance has been set up so that the platform
    /// can handle messages on the text input method channel.
    pub fn ensure_initialized(app: &mut App) {
        let _ = Self::instance(app);
    }

    /// Resets the internal ID counter for testing purposes.
    pub fn debug_reset_id(app: &mut App, to: i32) {
        if cfg!(debug_assertions) {
            let input = Self::instance(app);
            app.get_mut(input).next_id = to;
        }
    }

    /// Begin interacting with the text input control.
    pub fn attach(
        app: &mut App,
        client: AnyTextInputClient,
        configuration: TextInputConfiguration,
    ) -> Handle<TextInputConnection> {
        let input = Self::instance(app);
        let id = {
            let input = app.get_mut(input);
            let id = input.next_id;
            input.next_id += 1;
            id
        };
        let connection = app.create(TextInputConnection {
            id,
            client,
            cached_size: None,
            cached_transform: None,
            cached_rect: None,
            cached_caret_rect: None,
            cached_selection_rects: Vec::new(),
        });
        input.attach_connection(app, connection, configuration);
        connection
    }

    fn attach_connection(
        self: Handle<Self>,
        app: &mut App,
        connection: Handle<TextInputConnection>,
        configuration: TextInputConfiguration,
    ) {
        debug_ensure_input_action_works_on_platform(app, configuration.input_action);
        app.get_mut(self).current_connection = Some(connection);
        app.get_mut(self).current_configuration = Some(configuration.clone());
        app.get_mut(self).last_connection = Some(connection);
        platform_attach(app, &configuration);
    }

    fn show(self: Handle<Self>, app: &App) {
        if let Some(configuration) = app.get(self).current_configuration.clone() {
            platform_attach(app, &configuration);
        }
    }

    fn update_config(self: Handle<Self>, app: &mut App, configuration: TextInputConfiguration) {
        app.get_mut(self).current_configuration = Some(configuration.clone());
        platform_attach(app, &configuration);
    }

    fn set_editing_state(self: Handle<Self>, app: &App, value: TextEditingValue) {
        if let Some(view) = self.target_view(app) {
            view.set_text_input_editing_state(&value);
        }
    }

    fn set_editable_size_and_transform(
        self: Handle<Self>,
        app: &App,
        editable_box_size: Size,
        transform: Matrix4,
    ) {
        if let Some(view) = self.target_view(app) {
            view.set_text_input_client_geometry(editable_box_size, &transform);
        }
    }

    fn set_composing_text_rect(self: Handle<Self>, app: &App, rect: Rect) {
        if let Some(view) = self.target_view(app) {
            view.set_text_input_composing_rect(rect);
        }
    }

    fn set_caret_rect(self: Handle<Self>, app: &App, rect: Rect) {
        if let Some(view) = self.target_view(app) {
            view.set_text_input_caret_rect(rect);
        }
    }

    fn clear_client(self: Handle<Self>, app: &mut App) {
        platform_detach(app, self);
        app.get_mut(self).current_connection = None;
        self.schedule_hide(app);
    }

    fn hide(self: Handle<Self>, app: &App) {
        platform_detach(app, self);
    }

    fn schedule_hide(self: Handle<Self>, app: &mut App) {
        if app.get(self).hide_pending {
            return;
        }
        app.get_mut(self).hide_pending = true;
        app.schedule_microtask(Listener::new(move |app| {
            app.get_mut(self).hide_pending = false;
            if app.get(self).current_connection.is_none() {
                self.hide(app);
            }
        }));
    }

    fn target_view(self: Handle<Self>, app: &App) -> Option<ViewRef> {
        let configuration = app.get(self).current_configuration.as_ref()?;
        target_view(app, configuration)
    }

    /// Flutter `TextInputClient.updateEditingState`: the host has a new value.
    ///
    /// Does not echo the value back to the host (Flutter excludes
    /// `_PlatformTextInputControl`).
    pub fn update_editing_value(self: Handle<Self>, app: &mut App, value: TextEditingValue) {
        let Some(connection) = app.get(self).current_connection else {
            return;
        };
        let client = app.get(connection).client;
        client.update_editing_value(app, value);
    }

    /// Flutter `TextInputClient.performAction`.
    pub fn perform_action(self: Handle<Self>, app: &mut App, action: TextInputAction) {
        let Some(connection) = app.get(self).current_connection else {
            return;
        };
        let client = app.get(connection).client;
        client.perform_action(app, action);
    }

    /// Flutter `TextInputClient.onConnectionClosed`.
    pub fn connection_closed(self: Handle<Self>, app: &mut App) {
        let Some(connection) = app.get(self).current_connection else {
            return;
        };
        let client = app.get(connection).client;
        client.connection_closed(app);
        let _ = connection;
    }
}

fn target_view(app: &App, configuration: &TextInputConfiguration) -> Option<ViewRef> {
    let platform = app.platform();
    if let Some(id) = configuration.view_id
        && let Some(view) = platform.view(ViewId(id as u64))
    {
        return Some(view);
    }
    platform.implicit_view()
}

fn platform_attach(app: &App, configuration: &TextInputConfiguration) {
    if let Some(view) = target_view(app, configuration) {
        view.start_text_input(configuration);
    }
}

fn platform_detach(app: &App, input: Handle<TextInput>) {
    if let Some(view) = input.target_view(app) {
        view.stop_text_input();
    }
}

const ANDROID_SUPPORTED_INPUT_ACTIONS: &[TextInputAction] = &[
    TextInputAction::None,
    TextInputAction::Unspecified,
    TextInputAction::Done,
    TextInputAction::Send,
    TextInputAction::Go,
    TextInputAction::Search,
    TextInputAction::Next,
    TextInputAction::Previous,
    TextInputAction::Newline,
];

const IOS_SUPPORTED_INPUT_ACTIONS: &[TextInputAction] = &[
    TextInputAction::Unspecified,
    TextInputAction::Done,
    TextInputAction::Send,
    TextInputAction::Go,
    TextInputAction::Search,
    TextInputAction::Next,
    TextInputAction::Newline,
    TextInputAction::ContinueAction,
    TextInputAction::Join,
    TextInputAction::Route,
    TextInputAction::EmergencyCall,
];

fn debug_ensure_input_action_works_on_platform(app: &App, input_action: TextInputAction) {
    if !cfg!(debug_assertions) {
        return;
    }
    match app.platform().target_platform() {
        TargetPlatform::IOS => {
            debug_assert!(
                IOS_SUPPORTED_INPUT_ACTIONS.contains(&input_action),
                "The requested TextInputAction \"{input_action:?}\" is not supported on iOS."
            );
        }
        TargetPlatform::Android => {
            debug_assert!(
                ANDROID_SUPPORTED_INPUT_ACTIONS.contains(&input_action),
                "The requested TextInputAction \"{input_action:?}\" is not supported on Android."
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::TextAffinity;
    use reveal_foundation::AppCell;

    use super::*;
    use crate::text_editing::TextSelection;

    #[derive(Default)]
    struct RecordingClient {
        value: TextEditingValue,
        actions: Vec<TextInputAction>,
        closed: bool,
    }

    impl TextInputClient for RecordingClient {
        fn current_text_editing_value(self: Handle<Self>, app: &App) -> Option<TextEditingValue> {
            Some(app.get(self).value.clone())
        }

        fn update_editing_value(self: Handle<Self>, app: &mut App, value: TextEditingValue) {
            app.get_mut(self).value = value;
        }

        fn perform_action(self: Handle<Self>, app: &mut App, action: TextInputAction) {
            app.get_mut(self).actions.push(action);
        }

        fn connection_closed(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).closed = true;
        }
    }

    #[test]
    fn attach_then_host_value_reaches_the_client() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        let connection = TextInput::attach(
            &mut app,
            client.as_text_input_client(),
            TextInputConfiguration::new(),
        );
        assert!(connection.attached(&mut app));

        TextInput::instance(&mut app).update_editing_value(
            &mut app,
            TextEditingValue::new()
                .text("hi")
                .selection(TextSelection::collapsed(2, TextAffinity::Downstream)),
        );
        assert_eq!(client.current_text_editing_value(&app).unwrap().text, "hi");
    }

    #[test]
    fn close_schedules_hide_and_detaches() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        let connection = TextInput::attach(
            &mut app,
            client.as_text_input_client(),
            TextInputConfiguration::new(),
        );
        connection.close(&mut app);
        assert!(!connection.attached(&mut app));
        app.drain_microtasks();
    }

    #[test]
    fn connection_closed_from_the_host_notifies_the_client() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        TextInput::attach(
            &mut app,
            client.as_text_input_client(),
            TextInputConfiguration::new(),
        );
        TextInput::instance(&mut app).connection_closed(&mut app);
        assert!(app.get(client).closed);
    }

    #[test]
    fn perform_action_reaches_the_client() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        TextInput::attach(
            &mut app,
            client.as_text_input_client(),
            TextInputConfiguration::new().input_action(TextInputAction::Done),
        );
        TextInput::instance(&mut app).perform_action(&mut app, TextInputAction::Done);
        assert_eq!(app.get(client).actions, [TextInputAction::Done]);
    }

    #[test]
    fn composing_rect_that_is_not_finite_sends_negative_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        let connection = TextInput::attach(
            &mut app,
            client.as_text_input_client(),
            TextInputConfiguration::new(),
        );
        let invalid = Rect::from_ltrb(f64::NAN, 0.0, 1.0, 1.0);
        connection.set_composing_rect(&mut app, invalid);
        assert!(!app.get(connection).cached_rect.unwrap().is_finite());
    }

    #[test]
    fn selection_rect_prints_like_dart() {
        let rect = SelectionRect::new(3, Rect::from_ltwh(1.0, 2.0, 3.0, 4.0), TextDirection::Ltr);
        assert_eq!(
            rect.to_string(),
            format!("SelectionRect(3, {:?})", rect.bounds)
        );
    }
}
