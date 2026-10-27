//! XAML `RepeatButton` with the `DefaultRepeatButtonStyle` of `RepeatButton_themeresources.xaml`.
//!
//! The template is one `ContentPresenter` (`x:Name="ContentPresenter"`) carrying `Background`, `BorderBrush`, `ButtonBorderThemeThickness`, `CornerRadius`, `ButtonPadding` and `Foreground`, with a `BrushTransition` of 83 ms on `Background`; the `CommonStates` storyboards swap those brushes per state.
//!
//! Behaviour follows `RepeatButton_Partial.cpp` and `ButtonBaseKeyProcess.h`: pointer and Space presses click immediately and start the repeat timer; Enter clicks once. The control owns its pressed state and focus node so activation keys reach its Press-mode handler before application shortcuts.

use crate::{
    BackgroundSizing, Brush, ButtonTemplate, ColorTransition, CommonState, ControlBorder,
    ControlStates, FocusVisual, ThemeResources, control_text_style,
};
use inset_embedder::{Color, FontWeight, Offset, PointerDeviceKind};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_gestures::{K_PRIMARY_BUTTON, PointerDownEvent, PointerMoveEvent};
use inset_rendering::HitTestBehavior;
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use std::{fmt, rc::Rc, time::Duration};

/// The default `Delay`, 500 ms: `RepeatButton_Delay` in `DependencyProperty.cpp`.
pub const REPEAT_BUTTON_DELAY: Duration = Duration::from_millis(500);
/// The default `Interval`, 33 ms: `RepeatButton_Interval` in `DependencyProperty.cpp`.
pub const REPEAT_BUTTON_INTERVAL: Duration = Duration::from_millis(33);
/// The `FocusVisualMargin` the style sets.
pub const REPEAT_BUTTON_FOCUS_VISUAL_MARGIN: [f64; 4] = [-3.0; 4];

/// The brushes the style resolves to in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepeatButtonBrushes {
    pub background: Color,
    pub border: Brush,
    pub foreground: Color,
}

/// The `{ThemeResource …}` values the `CommonStates` storyboards name for `state`.
pub fn repeat_button_brushes(
    resources: &ThemeResources,
    state: CommonState,
) -> RepeatButtonBrushes {
    let r = resources;
    match state {
        CommonState::Normal => RepeatButtonBrushes {
            background: r.repeat_button_background,
            border: Brush::ControlElevation(r.repeat_button_border_brush),
            foreground: r.repeat_button_foreground,
        },
        CommonState::PointerOver => RepeatButtonBrushes {
            background: r.repeat_button_background_pointer_over,
            border: Brush::ControlElevation(r.repeat_button_border_brush_pointer_over),
            foreground: r.repeat_button_foreground_pointer_over,
        },
        CommonState::Pressed => RepeatButtonBrushes {
            background: r.repeat_button_background_pressed,
            border: Brush::Solid(r.repeat_button_border_brush_pressed),
            foreground: r.repeat_button_foreground_pressed,
        },
        CommonState::Disabled => RepeatButtonBrushes {
            background: r.repeat_button_background_disabled,
            border: Brush::Solid(r.repeat_button_border_brush_disabled),
            foreground: r.repeat_button_foreground_disabled,
        },
    }
}

/// `KeyPress::ButtonBase::IsPress` with `AcceptsReturn`: Space, Enter or GamepadA.
fn is_press_key(key: LogicalKeyboardKey) -> bool {
    key == LogicalKeyboardKey::SPACE
        || key == LogicalKeyboardKey::ENTER
        || key == LogicalKeyboardKey::NUMPAD_ENTER
        || key == LogicalKeyboardKey::GAME_BUTTON_A
}

/// XAML `RepeatButton`: `Click` on the press, then again after `Delay` and every `Interval` while held.
#[derive(Clone)]
pub struct RepeatButton {
    /// The content rendered by the button's template.
    pub content: WidgetRef,

    /// Raised on the initial press and each repeat tick.
    pub click: Listener,

    /// XAML `Delay`: the time from the press to the first repeat.
    pub delay: Duration,
    /// XAML `Interval`: the time between repeats.
    pub interval: Duration,
    /// Whether this button accepts input.
    pub is_enabled: bool,

    /// Whether keyboard traversal includes this button; defaults to true.
    pub is_tab_stop: bool,

    /// An alternate ContentPresenter template using the same input and repeat state.
    pub template: Option<ButtonTemplate>,

    /// Identity of this button in its parent's child list.
    pub key: Option<KeyRef>,
}

impl RepeatButton {
    /// Creates a repeating button with the default WinUI style.
    pub fn new<K>(content: impl IntoWidget<K>, click: Listener) -> RepeatButton {
        RepeatButton {
            content: content.into_widget(),
            click,
            delay: REPEAT_BUTTON_DELAY,
            interval: REPEAT_BUTTON_INTERVAL,
            is_enabled: true,
            is_tab_stop: true,
            template: None,
            key: None,
        }
    }

    /// A text `Content`, as `<RepeatButton Content="…"/>`.
    pub fn text(text: impl Into<String>, click: Listener) -> RepeatButton {
        RepeatButton::new(Text::new(text), click)
    }

    /// XAML `Delay`.
    pub fn delay(mut self, delay: Duration) -> RepeatButton {
        self.delay = delay;
        self
    }

    /// XAML `Interval`; `OnIntervalPropertyChanged` rejects a value that is not positive.
    pub fn interval(mut self, interval: Duration) -> RepeatButton {
        assert!(
            !interval.is_zero(),
            "RepeatButton.Interval must be positive"
        );
        self.interval = interval;
        self
    }

    /// XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> RepeatButton {
        self.is_enabled = enabled;
        self
    }

    /// XAML `IsTabStop`.
    pub fn is_tab_stop(mut self, is_tab_stop: bool) -> Self {
        self.is_tab_stop = is_tab_stop;
        self
    }

    /// Replaces the visual template while preserving repeat, keyboard and pointer handling.
    pub fn template(
        mut self,
        builder: impl Fn(&mut App, BuildContext, ControlStates, WidgetRef) -> WidgetRef + 'static,
    ) -> Self {
        self.template = Some(Rc::new(builder));
        self
    }

    /// Sets this button's identity.
    pub fn key(mut self, key: KeyRef) -> RepeatButton {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for RepeatButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepeatButton")
            .field("delay", &self.delay)
            .field("interval", &self.interval)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RepeatButton {
    type State = RepeatButtonState;
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }
    fn create_state(&self) -> RepeatButtonState {
        RepeatButtonState {
            state: StateData::new(),
            timer: None,
            pointer_causing_repeat: false,
            keyboard_causing_repeat: false,
            pointer_over: false,
            pressed: false,
            key_down: false,
            gamepad_a_key_down: false,
            focused: false,
            focus_node: None,
        }
    }
}

/// The repeat machinery of `RepeatButton_Partial.cpp`.
pub struct RepeatButtonState {
    state: StateData<RepeatButton>,
    /// `m_tpTimer`, `Some` while the `TimelineTimer` is enabled.
    timer: Option<Timer>,
    /// `m_pointerCausingRepeat`.
    pointer_causing_repeat: bool,
    /// `m_keyboardCausingRepeat`.
    keyboard_causing_repeat: bool,
    /// `IsPointerOver` for the pressed pointer, from its position against the control's bounds.
    pointer_over: bool,
    pressed: bool,
    key_down: bool,
    gamepad_a_key_down: bool,
    focused: bool,
    focus_node: Option<AnyFocusNode>,
}

impl RepeatButtonState {
    /// `RepeatButton::OnClick` → `ButtonBase::OnClick`: the `Click` event.
    fn on_click(self: Handle<Self>, app: &mut App) {
        self.widget(app).click.clone().call(app);
    }

    /// `RepeatButton::OnPointerPressed` over `ButtonBase::OnPointerPressed`: with the left button, the base raises `Click` (`ClickMode_Press`), then the pointer causes repeats.
    fn on_pointer_pressed(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        if !self.widget(app).is_enabled || event.buttons != K_PRIMARY_BUTTON {
            return;
        }
        self.set_state(app, |state| {
            state.pointer_over = true;
            state.pressed = true;
        });
        self.on_click(app);
        app.get_mut(self).pointer_causing_repeat = true;
        self.update_repeat_state(app);
    }

    /// `OnPointerEntered` / `OnPointerExited` for the active pointer: re-entering restarts the repeat (`UpdateRepeatState`); leaving lets the next tick stop it.
    fn on_pointer_moved(self: Handle<Self>, app: &mut App, event: PointerMoveEvent) {
        if !app.get(self).pointer_causing_repeat {
            return;
        }
        let over = self.contains(app, event.local_position());
        let was_over = std::mem::replace(&mut app.get_mut(self).pointer_over, over);
        if over && !was_over {
            self.update_repeat_state(app);
        }
    }

    /// `RepeatButton::OnPointerReleased`, and the capture lost: the pointer stops causing repeats.
    fn on_pointer_released(self: Handle<Self>, app: &mut App, kind: PointerDeviceKind) {
        self.set_state(app, |state| {
            state.pointer_causing_repeat = false;
            if kind == PointerDeviceKind::Touch {
                state.pointer_over = false;
            }
            if !state.key_down && !state.gamepad_a_key_down {
                state.pressed = false;
            }
        });
        self.update_repeat_state(app);
    }

    /// `RepeatButton::OnKeyDown` starts repetition for Space before `ButtonBase` processes the key.
    fn on_key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !self.widget(app).is_enabled {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if !is_press_key(key) {
            if !matches!(event, KeyEvent::Up(_))
                && (app.get(self).key_down || app.get(self).gamepad_a_key_down)
            {
                self.set_state(app, |state| {
                    state.key_down = false;
                    state.gamepad_a_key_down = false;
                    state.pressed = false;
                });
            }
            return KeyEventResult::Ignored;
        }
        match event {
            KeyEvent::Down(_) | KeyEvent::Repeat(_) => {
                if key == LogicalKeyboardKey::SPACE {
                    app.get_mut(self).keyboard_causing_repeat = true;
                    self.update_repeat_state(app);
                }
                if !app.get(self).pointer_causing_repeat
                    && !app.get(self).key_down
                    && !app.get(self).gamepad_a_key_down
                {
                    self.set_state(app, |state| {
                        if key == LogicalKeyboardKey::GAME_BUTTON_A {
                            state.gamepad_a_key_down = true;
                        } else {
                            state.key_down = true;
                        }
                        state.pressed = true;
                    });
                    self.on_click(app);
                }
            }
            KeyEvent::Up(_) => {
                self.set_state(app, |state| {
                    if key == LogicalKeyboardKey::GAME_BUTTON_A {
                        state.gamepad_a_key_down = false;
                    } else {
                        state.key_down = false;
                    }
                    if !state.pointer_causing_repeat {
                        state.pressed = false;
                    }
                    if key == LogicalKeyboardKey::SPACE {
                        state.keyboard_causing_repeat = false;
                    }
                });
                self.update_repeat_state(app);
            }
        }
        KeyEventResult::Handled
    }

    /// `RepeatButton::OnLostFocus` and `OnIsEnabledChanged`: neither input causes repeats any more.
    fn reset_repeat(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.keyboard_causing_repeat = false;
            state.pointer_causing_repeat = false;
            state.key_down = false;
            state.gamepad_a_key_down = false;
            state.pressed = false;
        });
        self.update_repeat_state(app);
    }

    /// `RepeatButton::UpdateRepeatState`.
    fn update_repeat_state(self: Handle<Self>, app: &mut App) {
        if self.should_repeat(app) {
            self.start_timer(app);
        } else {
            self.stop_timer(app);
        }
    }

    /// `(m_pointerCausingRepeat && IsPointerOver) || m_keyboardCausingRepeat`, the condition of `UpdateRepeatState` and of `TickCallback`.
    fn should_repeat(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self);
        (state.pointer_causing_repeat && state.pointer_over) || state.keyboard_causing_repeat
    }

    /// `RepeatButton::StartTimer`: a timer not already running starts at `Delay`.
    fn start_timer(self: Handle<Self>, app: &mut App) {
        if app.get(self).timer.is_some() {
            return;
        }
        let delay = self.widget(app).delay;
        self.schedule(app, delay);
    }

    /// `RepeatButton::StopTimer`.
    fn stop_timer(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
    }

    /// `TimelineTimer::Start` with the interval it currently holds.
    fn schedule(self: Handle<Self>, app: &mut App, duration: Duration) {
        let timer = Timer::new(app, duration, Listener::new(move |app| self.tick(app)));
        app.get_mut(self).timer = Some(timer);
    }

    /// `TimelineTimer::TimerCallback`: `RepeatButton::TickCallback` raises `Click` while the repeat condition holds and the timer restarts at `Interval`; otherwise it stops.
    fn tick(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).timer = None;
        if !app.get(self).pressed || !self.should_repeat(app) {
            return;
        }
        self.on_click(app);
        if !self.mounted(app) || !app.get(self).pressed || !self.should_repeat(app) {
            return;
        }
        let interval = self.widget(app).interval;
        self.schedule(app, interval);
    }

    /// `IsPointerOver`: whether `local` lies inside the control's bounds.
    fn contains(self: Handle<Self>, app: &App, local: Offset) -> bool {
        self.context(app)
            .find_render_object(app)
            .and_then(|object| object.as_box())
            .is_some_and(|render_box| render_box.size(app).contains(local))
    }
}

impl State for RepeatButtonState {
    type Widget = RepeatButton;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        crate::InputDevice::initialize(app);
        let node = FocusNode::new(app).as_node();
        node.set_skip_traversal(app, !self.widget(app).is_tab_stop);
        node.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.on_key_event(app, event))),
        );
        app.get_mut(self).focus_node = Some(node);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RepeatButton) {
        if self.widget(app).is_tab_stop != old_widget.is_tab_stop {
            app.get(self)
                .focus_node
                .expect("initialized focus node")
                .set_skip_traversal(app, !self.widget(app).is_tab_stop);
        }
        // `RepeatButton::OnIsEnabledChanged`.
        if self.widget(app).is_enabled != old_widget.is_enabled {
            self.reset_repeat(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_timer(app);
        if let Some(node) = app.get_mut(self).focus_node.take() {
            node.dispose(app);
            app.destroy(node.id());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let state = app.get(self);
        let states = ControlStates {
            common: if !widget.is_enabled {
                CommonState::Disabled
            } else if state.pressed {
                CommonState::Pressed
            } else if state.pointer_over {
                CommonState::PointerOver
            } else {
                CommonState::Normal
            },
            focused: state.focused && widget.is_enabled,
        };
        let presenter = if let Some(template) = widget.template {
            template(app, context, states, widget.content)
        } else {
            template(app, context, widget.content, states)
        };
        let pointer = PointerListener::new()
            .behavior(HitTestBehavior::Opaque)
            .on_pointer_down(Rc::new(move |app, event| {
                self.on_pointer_pressed(app, event)
            }))
            .on_pointer_move(Rc::new(move |app, event| self.on_pointer_moved(app, event)))
            .on_pointer_up(Rc::new(move |app, event| {
                self.on_pointer_released(app, event.kind)
            }))
            .on_pointer_cancel(Rc::new(move |app, event| {
                self.on_pointer_released(app, event.kind);
                self.set_state(app, |state| state.pressed = false);
                self.stop_timer(app);
            }))
            .child(presenter);
        FocusableActionDetector::new(pointer)
            .enabled(widget.is_enabled)
            .focus_node(app.get(self).focus_node.expect("initialized focus node"))
            .on_show_hover_highlight(move |app, value| {
                self.set_state(app, |state| state.pointer_over = value);
                if value {
                    self.update_repeat_state(app);
                }
            })
            .on_focus_change(move |app, has_focus| {
                let value = has_focus && crate::FocusState::current(app).shows_focus_visual();
                self.set_state(app, |state| state.focused = value);
                if !has_focus {
                    self.reset_repeat(app);
                }
            })
            .into_widget()
    }
}

/// The `ControlTemplate` of `DefaultRepeatButtonStyle` for one set of states.
fn template(
    app: &mut App,
    context: BuildContext,
    content: WidgetRef,
    states: ControlStates,
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let brushes = repeat_button_brushes(&resources, states.common);
    let presenter = repeat_button_presenter(
        &resources,
        brushes,
        RepeatButtonSetters::default_style(&resources),
        content,
    );
    FocusVisual::new(presenter, resources.theme)
        .visible(states.focused)
        .margin(REPEAT_BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(resources.control_corner_radius[0])
        .into_widget()
}

/// The values the template of `DefaultRepeatButtonStyle` takes from the style's setters, which
/// a style based on it can change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RepeatButtonSetters {
    /// `BorderThickness`: left, top, right, bottom.
    pub(crate) border_thickness: [f64; 4],
    /// `CornerRadius`.
    pub(crate) corner_radius: f64,
    /// `Padding`: left, top, right, bottom.
    pub(crate) padding: [f64; 4],
    /// `FontSize` of the content's text.
    pub(crate) font_size: f64,
}

impl RepeatButtonSetters {
    /// `DefaultRepeatButtonStyle`'s own, with the theme's values.
    pub(crate) fn default_style(r: &ThemeResources) -> RepeatButtonSetters {
        RepeatButtonSetters {
            border_thickness: [r.button_border_theme_thickness[0]; 4],
            corner_radius: r.control_corner_radius[0],
            padding: r.button_padding,
            font_size: r.control_content_theme_font_size,
        }
    }
}

/// `ContentPresenter`, the template of `DefaultRepeatButtonStyle`: `content` centered in a
/// border drawn with `brushes` and `setters`, its background fading between states over 83 ms
/// (`BrushTransition`).
pub(crate) fn repeat_button_presenter(
    r: &ThemeResources,
    brushes: RepeatButtonBrushes,
    setters: RepeatButtonSetters,
    content: WidgetRef,
) -> WidgetRef {
    let text = control_text_style(r, setters.font_size, FontWeight::W400, brushes.foreground);
    // `ContentPresenter`: `HorizontalContentAlignment` and `VerticalContentAlignment` are Center.
    let label = Center::new()
        .width_factor(1.0)
        .height_factor(1.0)
        .child(DefaultTextStyle::new(text, content))
        .into_widget();
    ColorTransition::new(
        brushes.background,
        r.control_faster_animation_duration,
        move |_app, background| {
            ControlBorder::new(Brush::Solid(background), brushes.border)
                .border_thickness_ltrb(setters.border_thickness)
                .corner_radius(setters.corner_radius)
                .background_sizing(BackgroundSizing::InnerBorderEdge)
                .padding(setters.padding)
                .child(label.clone())
                .into_widget()
        },
    )
    .into_widget()
}
