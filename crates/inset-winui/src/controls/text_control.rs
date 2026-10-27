//! TextBoxBase integration shared by the source TextBox and PasswordBox templates.

use super::combo_box::ComboBoxTextBoxStyle;
use super::number_box::NumberBoxTextBoxStyle;
use super::{PasswordRevealMode, TextBox, TextWrapping};
use crate::*;
use inset_embedder::FontWeight;
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{BoxConstraints, HitTestBehavior};
use inset_services::{
    FilteringTextInputFormatter, HardwareKeyboard, KeyEvent, LengthLimitingTextInputFormatter,
    LogicalKeyboardKey, TextInputFormatterRef, TextInputType,
};
use inset_widgets::*;
use std::{any::TypeId, collections::HashMap, rc::Rc};

/// Native editor configuration beneath the public WinUI controls.
#[derive(Clone, Debug)]
pub(super) struct TextControl {
    /// TextBoxBase properties and native editing callbacks.
    config: TextBox,

    /// Password specialization, absent for a plain TextBox.
    password: Option<(PasswordRevealMode, char)>,

    /// NumberBox's `NumberBoxTextBoxStyle`, absent for the default TextBox template.
    number_box: Option<NumberBoxTextBoxStyle>,

    /// ComboBox's `ComboBoxTextBoxStyle`, absent for the default TextBox template.
    combo_box: Option<ComboBoxTextBoxStyle>,
}

impl TextControl {
    /// Creates the shared editor while keeping each public control's specialization explicit.
    pub(super) fn new(config: TextBox, password: Option<(PasswordRevealMode, char)>) -> Self {
        Self {
            config,
            password,
            number_box: None,
            combo_box: None,
        }
    }

    /// Draws NumberBox's `InputBox` template instead of the default one.
    pub(super) fn number_box_style(mut self, style: NumberBoxTextBoxStyle) -> Self {
        self.number_box = Some(style);
        self
    }

    /// Draws ComboBox's `EditableText` template instead of the default one.
    pub(super) fn combo_box_style(mut self, style: ComboBoxTextBoxStyle) -> Self {
        self.combo_box = Some(style);
        self
    }
}

/// TextBoxBase focus and template state, with the PasswordBox reveal flags.
pub(super) struct TextControlState {
    /// Native state identity.
    state: StateData<TextControl>,

    /// Global key consumed by the native selection gesture delegate.
    editable_key: GlobalKey,

    /// Native selection gesture builder; never duplicates caret hit testing.
    gestures: Option<Handle<TextControlSelectionBuilder>>,

    /// Locally owned focus node when no node was supplied.
    owned_focus: Option<Handle<FocusNode>>,

    /// Focus node whose listener is currently attached.
    focus: Option<Handle<FocusNode>>,

    /// Controller whose listener is currently attached.
    controller: Option<AnyTextEditingController>,

    /// Native editing history retained across theme and template changes.
    undo: Option<Handle<UndoHistoryController>>,

    /// Source password policy rejects copy and cut even while the text is revealed.
    password_clipboard_action: Option<AnyAction>,

    /// Last text, used for the source empty-to-nonempty reveal rule.
    previous_text: String,

    /// Current TextBoxBase focus state.
    focused: bool,

    /// Current TextBoxBase pointer-over state.
    hovered: bool,

    /// Whether the source arrange pass found more than five em of width.
    has_space_for_button: bool,

    /// Square helper-button width, updated by the source SizeChanged rule.
    helper_width: f64,

    /// PasswordBox allows reveal only after entry from an empty value.
    can_show_reveal_button: bool,

    /// Whether the eye is currently pressed.
    reveal_button_pressed: bool,

    /// Whether Peek is currently revealing the password.
    password_revealed: bool,
}

/// Default Flutter selection gestures around the WinUI ContentElement.
struct TextControlSelectionBuilder {
    /// Native gesture state and delegate.
    builder: TextSelectionGestureDetectorBuilderData,
}

impl TextSelectionGestureDetectorBuilder for TextControlSelectionBuilder {
    inset_widgets::text_selection_gesture_detector_builder_accessors!(builder);
}

impl TextSelectionGestureDetectorBuilderDelegate for TextControlState {
    fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).editable_key.clone()
    }

    fn force_press_enabled(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn selection_enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).config.is_enabled
    }
}

impl StatefulWidget for TextControl {
    type State = TextControlState;

    fn create_state(&self) -> Self::State {
        TextControlState {
            state: StateData::new(),
            editable_key: GlobalKey::new(),
            gestures: None,
            owned_focus: None,
            focus: None,
            controller: None,
            undo: None,
            password_clipboard_action: None,
            previous_text: String::new(),
            focused: false,
            hovered: false,
            has_space_for_button: false,
            helper_width: 30.0,
            can_show_reveal_button: false,
            reveal_button_pressed: false,
            password_revealed: false,
        }
    }
}

impl TextControlState {
    /// Applies the source clear-button or reveal-button eligibility predicates.
    fn button_visible(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        let s = app.get(self);
        // `ComboBoxTextBoxStyle` has no `DeleteButton`.
        if !widget.config.is_enabled
            || !s.focused
            || !s.has_space_for_button
            || widget.combo_box.is_some()
        {
            return false;
        }

        if let Some((mode, _)) = widget.password {
            mode == PasswordRevealMode::Peek
                && s.can_show_reveal_button
                && !s.previous_text.is_empty()
        } else {
            !s.previous_text.is_empty()
                && !widget.config.is_read_only
                && !widget.config.accepts_return
                && widget.config.text_wrapping == TextWrapping::NoWrap
        }
    }

    /// Observes native controller assignments without manufacturing WinUI changing events.
    fn text_did_change(self: Handle<Self>, app: &mut App) {
        let text = self
            .widget(app)
            .config
            .controller
            .text_value(app)
            .to_owned();
        let changed = app.get(self).previous_text != text;

        self.set_state(app, |s| {
            if changed && s.previous_text.is_empty() && !text.is_empty() {
                s.can_show_reveal_button = true;
            }

            if changed && !s.reveal_button_pressed {
                s.password_revealed = false;
            }

            if text.is_empty() {
                s.can_show_reveal_button = false;
            }
            s.previous_text = text;
        });
    }

    /// A new focus session resets PasswordBox's reveal eligibility.
    fn focus_did_change(self: Handle<Self>, app: &mut App) {
        let focused = app.get(self).focus.unwrap().has_focus(app);
        if focused == app.get(self).focused {
            return;
        }
        self.set_state(app, |s| {
            s.focused = focused;
            s.can_show_reveal_button = false;
            s.password_revealed = false;
            s.reveal_button_pressed = false;
        });
    }

    /// TextBox::OnDeleteButtonClick retains editor focus and clears its native value.
    fn clear(self: Handle<Self>, app: &mut App) {
        if self.widget(app).password.is_some() || !self.button_visible(app) {
            return;
        }

        let callback = self.widget(app).config.text_changed.clone();
        self.widget(app).config.controller.clear(app);
        if let Some(callback) = callback {
            callback(app, String::new());
        }
    }

    /// PasswordBox::RevealPassword gates Peek and honors Hidden/Visible.
    fn reveal(self: Handle<Self>, app: &mut App, pressed: bool) {
        if self
            .widget(app)
            .password
            .is_none_or(|(mode, _)| mode != PasswordRevealMode::Peek)
        {
            return;
        }

        let revealed = pressed && self.button_visible(app);
        if app.get(self).password_revealed != revealed
            || app.get(self).reveal_button_pressed != pressed
        {
            self.set_state(app, |s| {
                s.reveal_button_pressed = pressed;
                s.password_revealed = revealed;
            });
        }
    }

    /// PasswordBox's Alt+F8 reveal gesture ends on F8 release.
    fn key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if self.widget(app).password.is_none() || event.logical_key() != LogicalKeyboardKey::F8 {
            return KeyEventResult::Ignored;
        }

        match event {
            KeyEvent::Down(_) if HardwareKeyboard::instance(app).is_alt_pressed(app) => {
                self.reveal(app, true)
            }
            KeyEvent::Up(_) => self.reveal(app, false),
            _ => return KeyEventResult::Ignored,
        }
        KeyEventResult::Handled
    }

    /// Connects replacement controller and focus handles using stable listener identities.
    fn sync_handles(self: Handle<Self>, app: &mut App) {
        let controller = self.widget(app).config.controller;
        if app.get(self).controller != Some(controller) {
            if let Some(old) = app.get(self).controller {
                old.remove_listener(app, &Listener::handle_method(self, Self::text_did_change));
            }

            controller.add_listener(app, Listener::handle_method(self, Self::text_did_change));
            let text = controller.text_value(app).to_owned();
            let s = app.get_mut(self);
            s.controller = Some(controller);
            s.previous_text = text;
            s.can_show_reveal_button = false;
            s.password_revealed = false;
        }

        let external = self.widget(app).config.focus_node;
        let focus = external.unwrap_or_else(|| {
            if let Some(node) = app.get(self).owned_focus {
                return node;
            }

            let node = FocusNode::new(app);
            app.get_mut(self).owned_focus = Some(node);
            node
        });
        if app.get(self).focus != Some(focus) {
            if let Some(old) = app.get(self).focus {
                old.remove_listener(app, &Listener::handle_method(self, Self::focus_did_change));
            }

            focus.add_listener(app, Listener::handle_method(self, Self::focus_did_change));
            let focused = focus.has_focus(app);
            let s = app.get_mut(self);
            s.focus = Some(focus);
            s.focused = focused;
            s.can_show_reveal_button = false;
            s.password_revealed = false;
        }
    }

    /// Constructs the source helper-button template with its own native ButtonBase state.
    fn helper(self: Handle<Self>, app: &App) -> WidgetRef {
        let password = self.widget(app).password.is_some();
        let width = app.get(self).helper_width;
        let width = self
            .widget(app)
            .number_box
            .map_or(width, |style| style.delete_button_width(width));
        let corner = self.widget(app).config.corner_radius;
        CommonStates::new(
            Listener::new(move |app| self.clear(app)),
            move |app, context, states| {
                let r = ThemeResources::of(app, context);
                let corner = corner.unwrap_or(r.control_corner_radius[0]);
                let (background, border, foreground) = match states.common {
                    CommonState::PointerOver => (
                        r.text_control_button_background_pointer_over,
                        r.text_control_button_border_brush_pointer_over,
                        r.text_control_button_foreground_pointer_over,
                    ),
                    CommonState::Pressed => (
                        r.text_control_button_background_pressed,
                        r.text_control_button_border_brush_pressed,
                        r.text_control_button_foreground_pressed,
                    ),
                    _ => (
                        r.text_control_button_background,
                        r.text_control_button_border_brush,
                        r.text_control_button_foreground,
                    ),
                };
                SizedBox::new()
                    .width(width)
                    .child(
                        Padding::new(insets(r.text_box_inner_button_margin)).child(
                            ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                                .border_thickness_ltrb(r.text_control_border_theme_thickness)
                                .corner_radius(corner)
                                .child(
                                    Center::new().width_factor(1.0).height_factor(1.0).child(
                                        FontIcon::symbol(if password {
                                            FluentSymbol::Eye
                                        } else {
                                            FluentSymbol::Dismiss
                                        })
                                        .font_size(if password {
                                            r.password_box_icon_font_size
                                        } else {
                                            r.text_box_icon_font_size
                                        })
                                        .foreground(foreground),
                                    ),
                                ),
                        ),
                    )
                    .into_widget()
            },
        )
        .is_tab_stop(false)
        .pressed_changed(move |app, pressed| {
            if password {
                self.reveal(app, pressed);
            }
        })
        .into_widget()
    }
}

impl State for TextControlState {
    type Widget = TextControl;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.sync_handles(app);
        let undo = UndoHistoryController::new(app);
        app.get_mut(self).undo = Some(undo);
        if self.widget(app).password.is_some() {
            let action = DoNothingAction::new(app);
            app.get_mut(self).password_clipboard_action = Some(action.as_action());
        }

        let delegate = self.as_text_selection_gesture_detector_builder_delegate();
        let gestures = app.create(TextControlSelectionBuilder {
            builder: TextSelectionGestureDetectorBuilderData::new(delegate),
        });
        app.get_mut(self).gestures = Some(gestures);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &TextControl) {
        self.sync_handles(app);
        if !self.widget(app).config.is_enabled
            || self
                .widget(app)
                .password
                .is_none_or(|(mode, _)| mode != PasswordRevealMode::Peek)
        {
            let s = app.get_mut(self);
            s.password_revealed = false;
            s.reveal_button_pressed = false;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).controller {
            controller.remove_listener(app, &Listener::handle_method(self, Self::text_did_change));
        }

        if let Some(focus) = app.get(self).focus {
            focus.remove_listener(app, &Listener::handle_method(self, Self::focus_did_change));
        }

        if let Some(focus) = app.get(self).owned_focus {
            focus.dispose(app);
            app.destroy(focus);
        }

        if let Some(gestures) = app.get(self).gestures {
            app.destroy(gestures);
        }

        if let Some(action) = app.get(self).password_clipboard_action {
            app.destroy(action.id());
        }

        if let Some(undo) = app.get(self).undo {
            let on_undo = undo.on_undo(app);
            let on_redo = undo.on_redo(app);
            undo.dispose(app);
            app.destroy(on_undo);
            app.destroy(on_redo);
            app.destroy(undo);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let config = widget.config;
        let theme = ThemeResources::of(app, context);
        let r = &theme;
        let focused = app.get(self).focused;

        let (background, border, foreground, placeholder) = if !config.is_enabled {
            (
                Brush::Solid(r.text_control_background_disabled),
                Brush::Solid(r.text_control_border_brush_disabled),
                r.text_control_foreground_disabled,
                r.text_control_placeholder_foreground_disabled,
            )
        } else if focused {
            (
                Brush::Solid(r.text_control_background_focused),
                Brush::VerticalElevation {
                    stops: r.text_control_border_brush_focused,
                    length: 2.0,
                },
                r.text_control_foreground_focused,
                r.text_control_placeholder_foreground_focused,
            )
        } else if app.get(self).hovered {
            (
                Brush::Solid(r.text_control_background_pointer_over),
                Brush::VerticalElevation {
                    stops: r.text_control_border_brush_pointer_over,
                    length: 2.0,
                },
                r.text_control_foreground_pointer_over,
                r.text_control_placeholder_foreground_pointer_over,
            )
        } else {
            (
                config
                    .background
                    .unwrap_or(Brush::Solid(r.text_control_background)),
                config.border_brush.unwrap_or(Brush::VerticalElevation {
                    stops: r.text_control_border_brush,
                    length: 2.0,
                }),
                config.foreground.unwrap_or(r.text_control_foreground),
                r.text_control_placeholder_foreground,
            )
        };
        let border_thickness = if focused && config.is_enabled {
            r.text_control_border_theme_thickness_focused
        } else {
            r.text_control_border_theme_thickness
        };

        let password = widget.password;
        let number_box = widget.number_box;
        let obscured = password.is_some_and(|(mode, _)| {
            mode != PasswordRevealMode::Visible && !app.get(self).password_revealed
        });
        let multiline = password.is_none()
            && (config.accepts_return || config.text_wrapping == TextWrapping::Wrap);

        let mut formatters = config.input_formatters.clone();
        if !config.accepts_return {
            formatters.insert(0, FilteringTextInputFormatter::single_line_formatter());
        }

        if let Some(max) = config.max_length.filter(|max| *max > 0) {
            let enforcement = LengthLimitingTextInputFormatter::get_default_max_length_enforcement(
                Some(app.platform().target_platform()),
            );
            formatters.push(TextInputFormatterRef(Rc::new(
                LengthLimitingTextInputFormatter::new(Some(max))
                    .max_length_enforcement(enforcement),
            )));
        }

        let style = control_text_style(
            &theme,
            config
                .font_size
                .unwrap_or(r.control_content_theme_font_size),
            FontWeight::NORMAL,
            foreground,
        );
        let mut editable = EditableText::new(
            config.controller,
            app.get(self).focus.unwrap(),
            style.clone(),
            foreground,
            foreground,
        )
        .read_only(config.is_read_only || !config.is_enabled)
        .obscure_text(obscured)
        .obscuring_character(password.map_or('●', |(_, c)| c).to_string())
        .max_lines(if multiline { config.max_lines } else { Some(1) })
        .input_formatters(formatters)
        .text_align(config.text_alignment)
        .autofocus(config.autofocus)
        .renderer_ignores_pointer(true)
        .undo_controller(app.get(self).undo.unwrap())
        .selection_color(
            config
                .selection_highlight_color
                .unwrap_or(r.text_control_selection_highlight_color),
        )
        .show_selection_handles(false)
        .select_all_on_focus(false)
        .keyboard_type(if multiline {
            TextInputType::MULTILINE
        } else {
            TextInputType::TEXT
        })
        .autocorrect(password.is_none())
        .enable_suggestions(password.is_none())
        .key(Rc::new(app.get(self).editable_key.clone()));
        // A WinUI text box keeps the focus on Enter: the platform's done action ends the
        // composition without the unfocusing Flutter defaults to.
        let controller = config.controller;
        editable = editable.on_editing_complete(move |app| controller.clear_composing(app));
        if password.is_some() {
            editable = editable.toolbar_options(ToolbarOptions {
                copy: false,
                cut: false,
                paste: true,
                select_all: true,
            });
        }

        if let Some(callback) = config.text_changed.clone() {
            editable = editable.on_changed(move |app, text| callback(app, text));
        }

        if let Some(callback) = config.selection_changed.clone() {
            editable = editable
                .on_selection_changed(move |app, selection, cause| callback(app, selection, cause));
        }

        editable = if let Some(builder) = config.context_menu_builder.clone() {
            editable.context_menu_builder(builder)
        } else {
            let undo = app.get(self).undo.unwrap();
            editable.context_menu_builder(Rc::new(move |app, context, editor| {
                super::text_edit_menu::build(app, context, editor, undo, password.is_some())
            }))
        };

        // EditableText exposes its copy/cut action for an enclosing control to override.
        let editable = if let Some(action) = app.get(self).password_clipboard_action {
            Actions::new(
                HashMap::from([(TypeId::of::<CopySelectionTextIntent>(), action)]),
                editable,
            )
            .into_widget()
        } else {
            editable.into_widget()
        };

        // ContentElement owns scrolling through EditableText and retains the template's base margin.
        let mut content: WidgetRef = Padding::new(insets(
            config.padding.unwrap_or(r.text_control_theme_padding),
        ))
        .child(editable)
        .into_widget();
        if config.controller.text_value(app).is_empty() && !config.placeholder_text.is_empty() {
            let placeholder = IgnorePointer::new().child(
                Padding::new(insets(
                    config.padding.unwrap_or(r.text_control_theme_padding),
                ))
                .child(
                    Text::new(config.placeholder_text.clone())
                        .style(control_text_style(
                            &theme,
                            config
                                .font_size
                                .unwrap_or(r.control_content_theme_font_size),
                            FontWeight::NORMAL,
                            config.placeholder_foreground.unwrap_or(placeholder),
                        ))
                        .text_align(config.text_alignment)
                        .soft_wrap(config.text_wrapping == TextWrapping::Wrap),
                ),
            );
            content = Stack::new()
                .children([
                    content,
                    Positioned::new(placeholder)
                        .left(0.0)
                        .right(0.0)
                        .top(0.0)
                        .into_widget(),
                ])
                .into_widget();
        }

        let gestures = app.get(self).gestures.unwrap();
        let content = TextSelectionGestureDetectorBuilder::build_gesture_detector(
            gestures,
            app,
            None,
            Some(HitTestBehavior::Translucent),
            content,
        );

        let mut cells = Vec::new();
        if let Some(header) = config.header.clone() {
            // HeaderContentPresenter.
            cells.push(
                GridCell::new(
                    Padding::new(insets(if password.is_some() {
                        r.password_box_top_header_margin
                    } else {
                        r.text_box_top_header_margin
                    }))
                    .child(DefaultTextStyle::new(
                        control_text_style(
                            &theme,
                            config
                                .font_size
                                .unwrap_or(r.control_content_theme_font_size),
                            FontWeight::NORMAL,
                            if config.is_enabled {
                                r.text_control_header_foreground
                            } else {
                                r.text_control_header_foreground_disabled
                            },
                        ),
                        header,
                    )),
                )
                .column_span(2)
                .into_widget(),
            );
        }

        // BorderElement spans the editor and helper columns; its arranged size supplies source sizing rules.
        let font_size = config
            .font_size
            .unwrap_or(r.control_content_theme_font_size);
        let border = SizeObserver::new(
            ConstrainedBox::new(
                BoxConstraints::new()
                    .min_width(
                        number_box.map_or(r.text_control_theme_min_width, |style| style.min_width),
                    )
                    .min_height(r.text_control_theme_min_height),
            )
            .child(
                ControlBorder::new(background, border)
                    .border_thickness_ltrb(border_thickness)
                    .corner_radius_corners(widget.combo_box.map_or(
                        [config.corner_radius.unwrap_or(r.control_corner_radius[0]); 4],
                        |style| style.corner_radius,
                    )),
            ),
            Rc::new(move |app, _, size| {
                if !app.contains(self) || !self.mounted(app) {
                    return;
                }
                let space = size.width() > font_size * 5.0;
                if app.get(self).has_space_for_button != space
                    || app.get(self).helper_width != size.height()
                {
                    self.set_state(app, |s| {
                        s.has_space_for_button = space;
                        s.helper_width = size.height();
                        if !space {
                            s.password_revealed = false;
                        }
                    });
                }
            }),
        );
        let columns = if number_box.is_some() { 3 } else { 2 };
        cells.push(
            GridCell::new(border)
                .row(1)
                .column_span(columns)
                .into_widget(),
        );
        cells.push(
            GridCell::new(
                Padding::new(insets(r.text_control_border_theme_thickness)).child(content),
            )
            .row(1)
            .into_widget(),
        );
        if self.button_visible(app) {
            cells.push(
                GridCell::new(self.helper(app))
                    .row(1)
                    .column(1)
                    .into_widget(),
            );
        }
        if let Some(indicator) = number_box.and_then(|style| style.popup_indicator(&theme)) {
            // NumberBox's PopupIndicator.
            cells.push(GridCell::new(indicator).row(1).column(2).into_widget());
        }

        if let Some(description) = config.description {
            // DescriptionPresenter.
            cells.push(
                GridCell::new(DefaultTextStyle::new(
                    control_text_style(
                        &theme,
                        config
                            .font_size
                            .unwrap_or(r.control_content_theme_font_size),
                        FontWeight::NORMAL,
                        r.system_control_description_text_foreground_brush,
                    ),
                    description,
                ))
                .row(2)
                .column_span(2)
                .into_widget(),
            );
        }

        let template = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::star(1.0)),
                RowDefinition::new(GridLength::AUTO),
            ])
            .column_definitions(
                [
                    ColumnDefinition::new(GridLength::star(1.0)),
                    ColumnDefinition::new(GridLength::AUTO),
                ]
                .into_iter()
                .chain(number_box.map(NumberBoxTextBoxStyle::spin_buttons_column)),
            )
            .children(cells);
        TextFieldTapRegion::new(
            ExcludeFocus::new(
                IgnorePointer::new().ignoring(!config.is_enabled).child(
                    MouseRegion::new()
                        .on_enter(Rc::new(move |app, _| {
                            self.set_state(app, |s| s.hovered = true)
                        }))
                        .on_exit(Rc::new(move |app, _| {
                            self.set_state(app, |s| s.hovered = false)
                        }))
                        .child(
                            Focus::new(template)
                                .can_request_focus(false)
                                .skip_traversal(true)
                                .on_key_event(Rc::new(move |app, _, event| {
                                    self.key_event(app, event)
                                })),
                        ),
                ),
            )
            .excluding(!config.is_enabled),
        )
        .into_widget()
    }
}

/// Converts XAML thickness order into native logical padding.
fn insets(value: [f64; 4]) -> EdgeInsetsGeometry {
    EdgeInsetsGeometry::from_ltrb(value[0], value[1], value[2], value[3])
}
