//! SplitButton.cpp and SplitButton.xaml: separate primary and flyout actions.

use crate::*;
use inset_embedder::{Color, FontWeight, PointerDeviceKind};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// A primary action and a separate button that opens an associated flyout.
#[derive(Clone)]
pub struct SplitButton {
    /// Content displayed by PrimaryButton.
    pub content: WidgetRef,

    /// Primary action, used for pointer activation and Space/Enter.
    pub click: Listener,

    /// Flyout opened by SecondaryButton, F4 or Alt+Down.
    pub flyout: Option<Handle<Flyout>>,

    /// Enables both actions and keyboard focus.
    pub is_enabled: bool,

    /// Includes this control in sequential keyboard traversal.
    pub is_tab_stop: bool,

    /// Optional caller-owned focus identity for the whole control.
    pub focus_node: Option<AnyFocusNode>,

    /// Identity across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl fmt::Debug for SplitButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitButton")
            .field("is_enabled", &self.is_enabled)
            .field("flyout", &self.flyout)
            .finish_non_exhaustive()
    }
}

impl SplitButton {
    /// Creates the source primary/secondary button template.
    pub fn new<K>(content: impl IntoWidget<K>, click: Listener) -> Self {
        Self {
            content: content.into_widget(),
            click,
            flyout: None,
            is_enabled: true,
            is_tab_stop: true,
            focus_node: None,
            key: None,
        }
    }

    /// Creates a split button with a text label.
    pub fn text(text: impl Into<String>, click: Listener) -> Self {
        Self::new(Text::new(text), click)
    }

    /// Sets the associated flyout without changing its ownership.
    pub fn flyout(mut self, flyout: Handle<Flyout>) -> Self {
        self.flyout = Some(flyout);
        self
    }

    /// Enables or disables both actions.
    pub fn is_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    /// Sets whether Tab visits the control.
    pub fn is_tab_stop(mut self, value: bool) -> Self {
        self.is_tab_stop = value;
        self
    }

    /// Supplies a caller-owned focus node.
    pub fn focus_node(mut self, node: AnyFocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Sets the widget's identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

/// CommonStates shared by SplitButton and ToggleSplitButton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitButtonVisualState {
    Normal,
    Disabled,
    FlyoutOpen,
    TouchPressed,
    PrimaryPointerOver,
    PrimaryPressed,
    SecondaryPointerOver,
    SecondaryPressed,
}

/// Pointer and keyboard state shared by the two source buttons.
pub(crate) struct SplitButtonState {
    /// Native state identity.
    state: StateData<SplitButtonPresenter>,

    /// Focus identity for the whole control, while internal buttons are not tab stops.
    focus: Option<AnyFocusNode>,

    /// Whether this state must dispose the focus node.
    owns_focus: bool,

    /// Pointer type most recently observed by either button.
    last_pointer: PointerDeviceKind,

    /// Space, Enter or gamepad accept is held.
    key_down: bool,

    /// IsPressed of PrimaryButton.
    primary_pressed: bool,

    /// IsPressed of SecondaryButton.
    secondary_pressed: bool,

    /// IsPointerOver of PrimaryButton.
    primary_hovered: bool,

    /// IsPointerOver of SecondaryButton.
    secondary_hovered: bool,

    /// Native focus-highlight visibility.
    focused: bool,

    /// Opening context beneath FlyoutTarget for the entire split button.
    target: Option<BuildContext>,
}

impl StatelessWidget for SplitButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        SplitButtonPresenter {
            button: self.clone(),
            is_checked: false,
        }
        .into_widget()
    }
}

/// Both source classes use the same template and input state, with a checked-state override.
#[derive(Clone, Debug)]
pub(crate) struct SplitButtonPresenter {
    /// Shared content, activation and focus configuration.
    pub(crate) button: SplitButton,

    /// ToggleSplitButton::InternalIsChecked; false for an ordinary SplitButton.
    pub(crate) is_checked: bool,
}

impl std::ops::Deref for SplitButtonPresenter {
    type Target = SplitButton;

    fn deref(&self) -> &Self::Target {
        &self.button
    }
}

impl StatefulWidget for SplitButtonPresenter {
    type State = SplitButtonState;

    fn create_state(&self) -> Self::State {
        SplitButtonState {
            state: StateData::new(),
            focus: None,
            owns_focus: false,
            last_pointer: PointerDeviceKind::Mouse,
            key_down: false,
            primary_pressed: false,
            secondary_pressed: false,
            primary_hovered: false,
            secondary_hovered: false,
            focused: false,
            target: None,
        }
    }
}

impl SplitButtonState {
    /// SplitButton::UpdateVisualStates chooses interaction state before checked colors.
    fn visual_state(self: Handle<Self>, app: &App) -> SplitButtonVisualState {
        use SplitButtonVisualState::*;
        let widget = self.widget(app);
        let state = app.get(self);
        if !widget.is_enabled {
            Disabled
        } else if widget.flyout.is_some_and(|flyout| flyout.is_open(app)) {
            FlyoutOpen
        } else if state.last_pointer == PointerDeviceKind::Touch || state.key_down {
            if state.primary_pressed || state.secondary_pressed || state.key_down {
                TouchPressed
            } else {
                Normal
            }
        } else if state.primary_pressed {
            PrimaryPressed
        } else if state.primary_hovered {
            PrimaryPointerOver
        } else if state.secondary_pressed {
            SecondaryPressed
        } else if state.secondary_hovered {
            SecondaryPointerOver
        } else {
            Normal
        }
    }

    /// SplitButton::OpenFlyout uses this per-opening override and Auto show mode.
    fn open_flyout(self: Handle<Self>, app: &mut App) {
        if let Some(flyout) = self.widget(app).flyout {
            flyout.show_at_with_options(
                app,
                app.get(self).target.unwrap(),
                FlyoutShowOptions {
                    placement: Some(FlyoutPlacementMode::BottomEdgeAlignedLeft),
                    ..Default::default()
                },
            );
        }
    }

    /// SplitButton key-down visuals and key-up activation/opening rules.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        let key = event.logical_key();
        let activation = matches!(
            key,
            LogicalKeyboardKey::SPACE
                | LogicalKeyboardKey::ENTER
                | LogicalKeyboardKey::NUMPAD_ENTER
                | LogicalKeyboardKey::GAME_BUTTON_A
        );
        if matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            if activation {
                self.set_state(app, |state| state.key_down = true);
                return KeyEventResult::Handled;
            }
        } else if activation {
            self.set_state(app, |state| state.key_down = false);
            let widget = self.widget(app).clone();
            if widget.is_enabled {
                widget.click.call(app);
                return KeyEventResult::Handled;
            }
        } else if self.widget(app).is_enabled
            && (key == LogicalKeyboardKey::F4
                || (key == LogicalKeyboardKey::ARROW_DOWN
                    && HardwareKeyboard::instance(app).is_alt_pressed(app)))
        {
            self.open_flyout(app);
            return KeyEventResult::Handled;
        }
        KeyEventResult::Ignored
    }

    /// Tracks source IsPressed changes from the internal buttons.
    fn pressed_changed(self: Handle<Self>, app: &mut App, primary: bool, pressed: bool) {
        let old = if primary {
            app.get(self).primary_pressed
        } else {
            app.get(self).secondary_pressed
        };
        if old != pressed {
            self.set_state(app, |state| {
                if primary {
                    state.primary_pressed = pressed;
                } else {
                    state.secondary_pressed = pressed;
                }
            });
        }
    }

    /// Tracks source pointer device changes on enter, exit, press, release and cancellation.
    fn pointer_kind(self: Handle<Self>, app: &mut App, kind: PointerDeviceKind) {
        if app.get(self).last_pointer != kind {
            self.set_state(app, |state| state.last_pointer = kind);
        }
    }

    /// Creates or attaches the whole-control focus node.
    fn attach_focus(self: Handle<Self>, app: &mut App) {
        let external = self.widget(app).focus_node;
        let node = external.unwrap_or_else(|| FocusNode::new(app).as_node());
        node.set_skip_traversal(app, !self.widget(app).is_tab_stop);
        node.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.on_key(app, event))),
        );
        app.get_mut(self).focus = Some(node);
        app.get_mut(self).owns_focus = external.is_none();
    }

    /// Builds one internal Button with the source template's unstyled content presenter.
    fn button(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &mut App,
        primary: bool,
        foreground: Color,
        disabled_background: Color,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let content = if primary {
            widget.content.clone()
        } else {
            SizedBox::new()
                .width(12.0)
                .height(12.0)
                .child(
                    Center::new().child(
                        FontIcon::symbol(FluentSymbol::ChevronDown)
                            .font_size(8.0)
                            .foreground(foreground),
                    ),
                )
                .into_widget()
        };
        let padding = r.split_button_padding;
        let text_style = control_text_style(
            r,
            r.control_content_theme_font_size,
            FontWeight::NORMAL,
            foreground,
        );
        let button = CommonStates::new(
            Listener::new(move |app| {
                if primary {
                    self.widget(app).click.clone().call(app);
                } else {
                    self.open_flyout(app);
                }
            }),
            move |_, _, states| {
                let content = DefaultTextStyle::new(text_style.clone(), content.clone());
                let presenter = if primary {
                    Padding::new(EdgeInsetsGeometry::from_ltrb(
                        padding[0], padding[1], padding[2], padding[3],
                    ))
                    .child(
                        Center::new()
                            .width_factor(1.0)
                            .height_factor(1.0)
                            .child(content),
                    )
                    .into_widget()
                } else {
                    Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 0.0, 12.0, 0.0))
                        .child(
                            Align::new()
                                .alignment(Alignment::CENTER_RIGHT.into())
                                .width_factor(1.0)
                                .height_factor(1.0)
                                .child(content),
                        )
                        .into_widget()
                };
                Grid::new()
                    .background(Brush::Solid(if states.common == CommonState::Disabled {
                        disabled_background
                    } else {
                        Color::new(0)
                    }))
                    .children([presenter])
                    .into_widget()
            },
        )
        .is_enabled(widget.is_enabled)
        .is_tab_stop(false)
        .pressed_changed(move |app, pressed| self.pressed_changed(app, primary, pressed));
        let button = MouseRegion::new()
            .on_enter(Rc::new(move |app, event| {
                self.pointer_kind(app, event.kind);
                self.set_state(app, |state| {
                    if primary {
                        state.primary_hovered = true;
                    } else {
                        state.secondary_hovered = true;
                    }
                });
            }))
            .on_exit(Rc::new(move |app, event| {
                self.pointer_kind(app, event.kind);
                self.set_state(app, |state| {
                    if primary {
                        state.primary_hovered = false;
                    } else {
                        state.secondary_hovered = false;
                    }
                });
            }))
            .child(button);
        inset_widgets::Listener::new()
            .on_pointer_down(Rc::new(move |app, event| {
                self.pointer_kind(app, event.kind)
            }))
            .on_pointer_up(Rc::new(move |app, event| {
                self.pointer_kind(app, event.kind)
            }))
            .on_pointer_cancel(Rc::new(move |app, event| {
                self.pointer_kind(app, event.kind)
            }))
            .child(button)
            .into_widget()
    }

    /// RootGrid layers appear in the same order as SplitButton.xaml.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = ThemeResources::of(app, context);
        let r = &theme;
        let state = self.visual_state(app);
        let mut primary_fill = r.split_button_background;
        let mut secondary_fill = r.split_button_background;
        let mut primary_foreground = r.split_button_foreground;
        let mut secondary_foreground = r.split_button_foreground_secondary;
        let mut primary_border = Brush::ControlElevation(r.split_button_border_brush);
        let mut secondary_border = primary_border;
        use SplitButtonVisualState::*;
        match state {
            Disabled => {
                primary_fill = Color::new(0);
                secondary_fill = Color::new(0);
                primary_foreground = r.split_button_foreground_disabled;
                secondary_foreground = r.split_button_foreground_disabled;
                primary_border = Brush::Solid(r.split_button_border_brush_disabled);
                secondary_border = primary_border;
            }
            FlyoutOpen | TouchPressed => {
                primary_fill = r.split_button_background_pressed;
                secondary_fill = primary_fill;
                primary_foreground = r.split_button_foreground_pressed;
                secondary_foreground = r.split_button_foreground_secondary_pressed;
                primary_border = Brush::Solid(r.split_button_border_brush_pressed);
                secondary_border = primary_border;
            }
            PrimaryPointerOver => {
                primary_fill = r.split_button_background_pointer_over;
                primary_foreground = r.split_button_foreground_pointer_over;
            }
            PrimaryPressed => {
                primary_fill = r.split_button_background_pressed;
                primary_foreground = r.split_button_foreground_pressed;
                primary_border = Brush::Solid(r.split_button_border_brush_pressed);
            }
            SecondaryPointerOver => {
                secondary_fill = r.split_button_background_pointer_over;
                secondary_foreground = r.split_button_foreground_pointer_over;
            }
            SecondaryPressed => {
                secondary_fill = r.split_button_background_pressed;
                secondary_foreground = r.split_button_foreground_secondary_pressed;
                secondary_border = Brush::Solid(r.split_button_border_brush_pressed);
            }
            Normal => {}
        }
        let mut divider = r.split_button_border_brush_divider;
        if self.widget(app).is_checked && state != Disabled {
            primary_fill = r.split_button_background_checked;
            secondary_fill = primary_fill;
            primary_foreground = r.split_button_foreground_checked;
            secondary_foreground = primary_foreground;
            primary_border = Brush::ControlElevation(r.split_button_border_brush_checked);
            secondary_border = primary_border;
            divider = r.split_button_border_brush_checked_divider;

            match state {
                FlyoutOpen | TouchPressed => {
                    primary_fill = r.split_button_background_checked_pressed;
                    secondary_fill = primary_fill;
                    primary_foreground = r.split_button_foreground_checked_pressed;
                    secondary_foreground = primary_foreground;
                    primary_border = Brush::Solid(r.split_button_border_brush_checked_pressed);
                    secondary_border = primary_border;
                }
                PrimaryPointerOver => {
                    primary_fill = r.split_button_background_checked_pointer_over;
                    primary_foreground = r.split_button_foreground_checked_pointer_over;
                }
                PrimaryPressed => {
                    primary_fill = r.split_button_background_checked_pressed;
                    primary_foreground = r.split_button_foreground_checked_pressed;
                }
                SecondaryPointerOver => {
                    secondary_fill = r.split_button_background_checked_pointer_over;
                    secondary_foreground = r.split_button_foreground_checked_pointer_over;
                }
                SecondaryPressed => {
                    secondary_fill = r.split_button_background_checked_pressed;
                    secondary_foreground = r.split_button_foreground_checked_pressed;
                }
                Normal | Disabled => {}
            }
        }
        let primary = self.button(
            &theme,
            app,
            true,
            primary_foreground,
            r.split_button_background_disabled,
        );
        let secondary = self.button(
            &theme,
            app,
            false,
            secondary_foreground,
            r.split_button_background_disabled,
        );
        let key_down = app.get(self).key_down;
        let root = Grid::new()
            .background(Brush::Solid(Color::new(0)))
            .corner_radius_corners(r.control_corner_radius)
            .column_definitions([
                ColumnDefinition::new(GridLength::STAR)
                    .min_width(r.split_button_primary_button_size),
                ColumnDefinition::new(GridLength::pixel(1.0)),
                ColumnDefinition::new(GridLength::pixel(r.split_button_secondary_button_size)),
            ])
            .children([
                GridCell::new(Grid::new().background(Brush::Solid(primary_fill)))
                    .column_span(2)
                    .into_widget(),
                GridCell::new(Grid::new().background(Brush::Solid(divider)))
                    .column(1)
                    .into_widget(),
                GridCell::new(Grid::new().background(Brush::Solid(secondary_fill)))
                    .column(2)
                    .into_widget(),
                GridCell::new(primary).into_widget(),
                GridCell::new(secondary)
                    .column(if key_down { 0 } else { 2 })
                    .column_span(if key_down { 3 } else { 1 })
                    .into_widget(),
                GridCell::new(
                    Grid::new()
                        .border_brush(primary_border)
                        .border_thickness_ltrb([1.0, 1.0, 0.0, 1.0])
                        .corner_radius_corners([4.0, 0.0, 0.0, 4.0]),
                )
                .into_widget(),
                GridCell::new(
                    Grid::new()
                        .border_brush(secondary_border)
                        .border_thickness_ltrb([0.0, 1.0, 1.0, 1.0])
                        .corner_radius_corners([0.0, 4.0, 4.0, 0.0]),
                )
                .column(2)
                .into_widget(),
            ]);
        FocusVisual::new(root, theme.theme)
            .visible(app.get(self).focused)
            .margin([-1.0; 4])
            .corner_radius(r.control_corner_radius[0])
            .into_widget()
    }
}

impl State for SplitButtonState {
    type Widget = SplitButtonPresenter;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        crate::InputDevice::initialize(app);
        self.attach_focus(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &SplitButtonPresenter) {
        if !self.widget(app).is_enabled {
            app.get_mut(self).primary_pressed = false;
            app.get_mut(self).secondary_pressed = false;
        }
        if old.focus_node != self.widget(app).focus_node {
            let node = app.get(self).focus.unwrap();
            node.set_on_key_event(app, None);
            if app.get(self).owns_focus {
                node.dispose(app);
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _| {
                        if app.contains(node.id()) {
                            app.destroy(node.id());
                        }
                    }),
                    View::maybe_id_of(app, self.context(app)),
                );
            }
            self.attach_focus(app);
        }
        app.get(self)
            .focus
            .unwrap()
            .set_skip_traversal(app, !self.widget(app).is_tab_stop);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let node = app.get(self).focus.unwrap();
        if app.get(self).owns_focus {
            node.dispose(app);
            app.destroy(node.id());
        } else {
            node.set_on_key_event(app, None);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let content = if let Some(flyout) = widget.flyout {
            ListenableBuilder::new(Rc::new(flyout), move |app, context, _| {
                self.template(app, context)
            })
            .into_widget()
        } else {
            self.template(app, context)
        };
        let content = FocusableActionDetector::new(content)
            .focus_node(app.get(self).focus.unwrap())
            .enabled(widget.is_enabled)
            .on_focus_change(move |app, focused| {
                let value = focused && crate::FocusState::current(app).shows_focus_visual();
                self.set_state(app, |state| state.focused = value)
            });
        let content = content.into_widget();
        FlyoutTarget::new(Builder::new(move |app, context| {
            app.get_mut(self).target = Some(context);
            content.clone()
        }))
        .into_widget()
    }
}
