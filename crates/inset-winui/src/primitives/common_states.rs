//! XAML's `CommonStates` visual state group (`Normal`, `PointerOver`, `Pressed`, `Disabled`) as the framework machinery that computes it from the pointer, plus whether the system focus visual shows.

use crate::{FocusState, InputDevice};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::HitTestBehavior;
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{any::TypeId, collections::HashMap, fmt, rc::Rc};

/// The `CommonStates` group every control template declares.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CommonState {
    #[default]
    Normal,
    PointerOver,
    Pressed,
    Disabled,
}

/// What the state tracker knows: the common state and whether the system focus visual shows,
/// which it does while the control has the focus and the focus arrived by keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlStates {
    pub common: CommonState,
    pub focused: bool,
}

/// Builds the control's template for the current states.
pub type StatesBuilder = Rc<dyn Fn(&mut App, BuildContext, ControlStates) -> WidgetRef>;

/// Observes a ButtonBase.IsPressed transition.
pub type PressedChangedCallback = Rc<dyn Fn(&mut App, bool)>;

/// Tracks pointer over, press, keyboard activation and focus for a control and runs `click` on activation; the `ButtonBase` of XAML.
#[derive(Clone)]
pub struct CommonStates {
    pub click: Listener,
    pub builder: StatesBuilder,
    pub is_enabled: bool,
    /// Whether sequential keyboard navigation stops on this control.
    pub is_tab_stop: bool,
    /// An optional owner-managed focus node for container lookup and arrow navigation.
    pub focus_node: Option<AnyFocusNode>,
    /// `ButtonBase::SetAcceptsReturn`: whether Enter activates as Space does. `ButtonBase::Initialize` sets it; `CheckBox` and `RadioButton` clear it.
    pub accepts_return: bool,
    /// Observes ButtonBase.IsPressed transitions after the state has changed.
    pub pressed_changed: Option<PressedChangedCallback>,
    pub key: Option<KeyRef>,
}

impl CommonStates {
    pub fn new(
        click: Listener,
        builder: impl Fn(&mut App, BuildContext, ControlStates) -> WidgetRef + 'static,
    ) -> CommonStates {
        CommonStates {
            click,
            builder: Rc::new(builder),
            is_enabled: true,
            is_tab_stop: true,
            focus_node: None,
            accepts_return: true,
            pressed_changed: None,
            key: None,
        }
    }

    pub fn accepts_return(mut self, accepts_return: bool) -> CommonStates {
        self.accepts_return = accepts_return;
        self
    }

    pub fn is_enabled(mut self, enabled: bool) -> CommonStates {
        self.is_enabled = enabled;
        self
    }

    /// XAML `IsTabStop`; programmatic focus remains available when traversal skips the control.
    pub fn is_tab_stop(mut self, value: bool) -> Self {
        self.is_tab_stop = value;
        self
    }

    /// Uses a caller-owned node; the caller disposes it after this widget unmounts.
    pub fn focus_node(mut self, node: AnyFocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Observes the source IsPressed property without taking focus from the editor.
    pub fn pressed_changed(mut self, callback: impl Fn(&mut App, bool) + 'static) -> Self {
        self.pressed_changed = Some(Rc::new(callback));
        self
    }

    pub fn key(mut self, key: KeyRef) -> CommonStates {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for CommonStates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommonStates")
            .field("is_enabled", &self.is_enabled)
            .field("accepts_return", &self.accepts_return)
            .finish_non_exhaustive()
    }
}

pub struct CommonStatesData {
    state: StateData<CommonStates>,
    pressed: bool,
    hovered: bool,
    focused: bool,
    focus_node: Option<AnyFocusNode>,
    owns_focus_node: bool,
    pointer_down: bool,
    space_or_enter_key_down: bool,
    gamepad_a_key_down: bool,
    actions: HashMap<TypeId, AnyAction>,
}

impl StatefulWidget for CommonStates {
    type State = CommonStatesData;
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }
    fn create_state(&self) -> CommonStatesData {
        CommonStatesData {
            state: StateData::new(),
            pressed: false,
            hovered: false,
            focused: false,
            focus_node: None,
            owns_focus_node: false,
            pointer_down: false,
            space_or_enter_key_down: false,
            gamepad_a_key_down: false,
            actions: HashMap::new(),
        }
    }
}

impl CommonStatesData {
    /// Publishes IsPressed changes after updating the visual state.
    fn update_states(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut Self)) {
        let old_pressed = app.get(self).pressed;
        self.set_state(app, change);
        let pressed = app.get(self).pressed;
        if old_pressed != pressed
            && let Some(callback) = self.widget(app).pressed_changed.clone()
        {
            callback(app, pressed);
        }
    }

    fn clear_state_flags(self: Handle<Self>, app: &mut App) {
        self.update_states(app, |state| {
            state.pressed = false;
            state.pointer_down = false;
            state.space_or_enter_key_down = false;
            state.gamepad_a_key_down = false;
        });
    }

    /// `KeyPress::ButtonBase`: a release-mode button presses on key-down and clicks on key-up.
    fn on_key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !self.widget(app).is_enabled {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        let is_space_or_enter = key == LogicalKeyboardKey::SPACE
            || (self.widget(app).accepts_return
                && matches!(
                    key,
                    LogicalKeyboardKey::ENTER | LogicalKeyboardKey::NUMPAD_ENTER
                ));
        let is_press = is_space_or_enter || key == LogicalKeyboardKey::GAME_BUTTON_A;
        match event {
            KeyEvent::Down(_) | KeyEvent::Repeat(_) => {
                let state = app.get(self);
                if is_press {
                    if !state.pointer_down
                        && !state.space_or_enter_key_down
                        && !state.gamepad_a_key_down
                    {
                        self.update_states(app, |state| {
                            state.space_or_enter_key_down = is_space_or_enter;
                            state.gamepad_a_key_down = !is_space_or_enter;
                            state.pressed = true;
                        });
                    }
                } else if state.space_or_enter_key_down || state.gamepad_a_key_down {
                    self.update_states(app, |state| {
                        state.pressed = false;
                        state.space_or_enter_key_down = false;
                        state.gamepad_a_key_down = false;
                    });
                }
            }
            KeyEvent::Up(_) if is_press => {
                if is_space_or_enter {
                    app.get_mut(self).space_or_enter_key_down = false;
                } else {
                    app.get_mut(self).gamepad_a_key_down = false;
                }
                if !app.get(self).pointer_down {
                    if app.get(self).pressed {
                        self.activate(app);
                    }
                    self.update_states(app, |state| state.pressed = false);
                }
            }
            KeyEvent::Up(_) => {}
        }
        // The app's activation shortcut must not turn an ignored repeat into another click.
        if is_press {
            KeyEventResult::Handled
        } else if matches!(
            key,
            LogicalKeyboardKey::ENTER | LogicalKeyboardKey::NUMPAD_ENTER
        ) {
            KeyEventResult::SkipRemainingHandlers
        } else {
            KeyEventResult::Ignored
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if widget.is_enabled {
            widget.click.call(app);
        }
    }

    fn states(self: Handle<Self>, app: &mut App) -> ControlStates {
        let shows_focus_visual = FocusState::current(app).shows_focus_visual();
        let widget = self.widget(app);
        let data = app.get(self);
        if !widget.is_enabled {
            return ControlStates {
                common: CommonState::Disabled,
                focused: false,
            };
        }
        ControlStates {
            common: if data.pressed {
                CommonState::Pressed
            } else if data.hovered {
                CommonState::PointerOver
            } else {
                CommonState::Normal
            },
            focused: data.focused && shows_focus_visual,
        }
    }
}

impl State for CommonStatesData {
    type Widget = CommonStates;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let external = self.widget(app).focus_node;
        let node = external.unwrap_or_else(|| FocusNode::new(app).as_node());
        app.get_mut(self).owns_focus_node = external.is_none();
        node.set_skip_traversal(app, !self.widget(app).is_tab_stop);
        node.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.on_key_event(app, event))),
        );
        app.get_mut(self).focus_node = Some(node);
        let action = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app, _| {
                self.activate(app);
                None
            }),
        );
        app.get_mut(self)
            .actions
            .insert(TypeId::of::<ActivateIntent>(), action.as_action());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(node) = app.get_mut(self).focus_node.take() {
            if app.get(self).owns_focus_node {
                node.dispose(app);
                app.destroy(node.id());
            } else {
                node.set_on_key_event(app, None);
            }
        }
        for action in std::mem::take(&mut app.get_mut(self).actions).into_values() {
            app.destroy(action.id());
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CommonStates) {
        if old_widget.focus_node != self.widget(app).focus_node {
            if let Some(node) = app.get_mut(self).focus_node.take() {
                if app.get(self).owns_focus_node {
                    node.dispose(app);
                    // FocusableActionDetector detaches the previous node when its child updates.
                    SchedulerBinding::add_post_frame_callback(
                        app,
                        FrameCallback::new(move |app, _| {
                            if app.contains(node.id()) {
                                app.destroy(node.id());
                            }
                        }),
                        View::maybe_id_of(app, self.context(app)),
                    );
                } else {
                    node.set_on_key_event(app, None);
                }
            }
            let external = self.widget(app).focus_node;
            let node = external.unwrap_or_else(|| FocusNode::new(app).as_node());
            node.set_on_key_event(
                app,
                Some(Rc::new(move |app, _, event| self.on_key_event(app, event))),
            );
            app.get_mut(self).focus_node = Some(node);
            app.get_mut(self).owns_focus_node = external.is_none();
        }
        if let Some(node) = app.get(self).focus_node {
            node.set_skip_traversal(app, !self.widget(app).is_tab_stop);
        }
        if !self.widget(app).is_enabled {
            self.clear_state_flags(app);
            app.get_mut(self).hovered = false;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let states = self.states(app);
        let body = (widget.builder)(app, context, states);
        let mut gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(body);
        if widget.is_enabled {
            gesture = gesture
                .on_tap_down(Rc::new(move |app, _| {
                    self.update_states(app, |state| {
                        state.pointer_down = true;
                        state.pressed = true;
                    })
                }))
                .on_tap_up(Rc::new(move |app, _| {
                    self.update_states(app, |state| {
                        state.pointer_down = false;
                        if !state.space_or_enter_key_down && !state.gamepad_a_key_down {
                            state.pressed = false;
                        }
                    })
                }))
                .on_tap_cancel(Listener::new(move |app| {
                    self.update_states(app, |state| {
                        state.pointer_down = false;
                        state.pressed = false;
                    })
                }))
                .on_tap(Listener::new(move |app| {
                    let state = app.get(self);
                    if !state.space_or_enter_key_down && !state.gamepad_a_key_down {
                        self.activate(app);
                    }
                }));
        }
        FocusableActionDetector::new(gesture)
            .focus_node(app.get(self).focus_node.unwrap())
            .enabled(widget.is_enabled)
            .on_focus_change(move |app, focused| {
                if !focused {
                    self.clear_state_flags(app);
                }
                self.update_states(app, |state| state.focused = focused);
            })
            .actions(app.get(self).actions.clone())
            .on_show_hover_highlight(move |app, value| {
                self.update_states(app, |state| state.hovered = value)
            })
            .into_widget()
    }
}
