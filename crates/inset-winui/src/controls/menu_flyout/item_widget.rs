//! Menu item key processing and CascadingMenuHelper's delayed opening and closing.

use super::{
    MenuFlyout,
    item::{MenuFlyoutItem, MenuFlyoutItemKind},
    presenter::MenuFlyoutPresenterState,
    template::{self, MenuTemplateSettings, MenuVisualState},
};
use crate::*;
// Inset exports a `FocusState` of its own; the XAML one is meant here.
use crate::FocusState;
use inset_embedder::{Offset, PointerDeviceKind, Rect};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_rendering::HitTestBehavior;
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{rc::Rc, time::Duration};

/// One source item container in a retained presenter.
#[derive(Clone, Debug)]
pub(super) struct MenuItemWidget {
    /// Application-supplied item properties.
    pub item: MenuFlyoutItem,

    /// Current collection position.
    pub index: usize,

    /// Collection that owns this item.
    pub menu: Handle<MenuFlyout>,

    /// Container registry and keyboard navigation owner.
    pub presenter: Handle<MenuFlyoutPresenterState>,

    /// Shared template columns and padding state.
    pub settings: MenuTemplateSettings,
}

/// MenuFlyoutItem flags and, for SubItem, the cascading-menu lifetime.
pub(super) struct MenuItemState {
    /// Native widget state.
    state: StateData<MenuItemWidget>,

    /// This item's keyboard focus identity.
    focus: Option<AnyFocusNode>,

    /// Pointer-over visual state.
    hovered: bool,

    /// Active pointer or keyboard press.
    pressed: bool,

    /// Pointer press has not yet ended.
    pointer_down: bool,

    /// Space or Enter is down.
    key_down: bool,

    /// Gamepad accept is down.
    gamepad_down: bool,

    /// Pointer release qualified for invocation.
    should_perform_actions: bool,

    /// Whether the item has the focus.
    focused: bool,

    /// Retained child collection, created only for SubItem.
    submenu: Option<Handle<MenuFlyout>>,

    /// Source delayed opening timer.
    open_timer: Option<Timer>,

    /// Source delayed closing timer.
    close_timer: Option<Timer>,

    /// Context below FlyoutTarget used to open the child collection.
    target: Option<BuildContext>,
}

impl StatefulWidget for MenuItemWidget {
    type State = MenuItemState;

    fn key(&self) -> Option<&KeyRef> {
        self.item.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        MenuItemState {
            state: StateData::new(),
            focus: None,
            hovered: false,
            pressed: false,
            pointer_down: false,
            key_down: false,
            gamepad_down: false,
            should_perform_actions: false,
            focused: false,
            submenu: None,
            open_timer: None,
            close_timer: None,
            target: None,
        }
    }
}

impl MenuItemState {
    /// Clearing a closed presenter's items cancels submenu timers and pointer/key state.
    pub(super) fn prepare_close(self: Handle<Self>, app: &mut App) {
        self.cancel_open(app);
        self.cancel_close(app);
        self.clear_press(app);
        self.set_state(app, |state| state.hovered = false);
    }

    /// The focus identity used by MenuFlyoutPresenter::CycleFocus.
    pub(super) fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        app.get(self).focus.unwrap()
    }

    /// `Focus(FocusState)` on this item as MenuFlyoutPresenter::CycleFocus calls it: focus moves and the item scrolls into view.
    pub(super) fn request_focus(self: Handle<Self>, app: &mut App, state: FocusState) {
        let node = self.focus_node(app);
        FocusState::name(app, node, state);
        default_traversal_request_focus_callback(app, node, None, None, None, None);
        self.set_state(app, |_| {});
    }

    /// Disabled entries and separators do not participate in focus cycling.
    pub(super) fn is_focusable(self: Handle<Self>, app: &App) -> bool {
        let item = &self.widget(app).item;
        item.is_enabled && !matches!(item.kind, MenuFlyoutItemKind::Separator)
    }

    /// Invokes checked-value changes before Click, then closes the owning root menu.
    fn invoke(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if !widget.item.is_enabled {
            return;
        }
        match &widget.item.kind {
            MenuFlyoutItemKind::Toggle {
                is_checked,
                changed,
            } => changed(app, !is_checked),
            MenuFlyoutItemKind::Radio {
                is_checked,
                checked,
            } if !is_checked => checked.call(app),
            MenuFlyoutItemKind::SubItem { .. } => {
                self.open_submenu(app);
                return;
            }
            MenuFlyoutItemKind::Separator => return,
            _ => {}
        }
        if let Some(click) = &widget.item.click {
            click.call(app);
        }
        if !widget.item.prevent_dismiss_on_pointer {
            widget.menu.root(app).hide(app);
        }
    }

    /// KeyDown/KeyUp from MenuFlyoutKeyPressProcess, with SubItem's immediate opening keys.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !self.is_focusable(app) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        let submenu = matches!(
            self.widget(app).item.kind,
            MenuFlyoutItemKind::SubItem { .. }
        );
        let activation = matches!(
            key,
            LogicalKeyboardKey::SPACE
                | LogicalKeyboardKey::ENTER
                | LogicalKeyboardKey::NUMPAD_ENTER
        );
        let gamepad = key == LogicalKeyboardKey::GAME_BUTTON_A;
        if matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            if key == LogicalKeyboardKey::ARROW_UP || key == LogicalKeyboardKey::ARROW_DOWN {
                self.clear_press(app);
                self.widget(app)
                    .presenter
                    .cycle_focus(app, key == LogicalKeyboardKey::ARROW_DOWN);
                return KeyEventResult::Handled;
            }
            if submenu {
                if (activation || key == LogicalKeyboardKey::ARROW_RIGHT)
                    && !HardwareKeyboard::instance(app).is_alt_pressed(app)
                {
                    self.open_submenu(app);
                    return KeyEventResult::Handled;
                }
            } else {
                if app.get(self).key_down || app.get(self).gamepad_down {
                    self.clear_press(app);
                }
                if activation || gamepad {
                    self.set_state(app, |state| {
                        state.pressed = true;
                        state.key_down = activation;
                        state.gamepad_down = gamepad;
                    });
                    return KeyEventResult::Handled;
                }
            }
            KeyEventResult::Ignored
        } else {
            if !submenu && (activation || gamepad) {
                let invoke = app.get(self).pressed && !app.get(self).pointer_down;
                self.clear_press(app);
                if invoke {
                    self.invoke(app);
                }
            }
            KeyEventResult::Handled
        }
    }

    /// Clears press flags on pointer exit, focus loss or disabling.
    fn clear_press(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.pressed = false;
            state.pointer_down = false;
            state.key_down = false;
            state.gamepad_down = false;
            state.should_perform_actions = false;
        });
    }

    /// Registers a delayed opening using WinUI's default MenuShowDelay of 400 ms.
    fn delay_open(self: Handle<Self>, app: &mut App) {
        self.cancel_open(app);
        let timer = Timer::new(
            app,
            Duration::from_millis(400),
            Listener::new(move |app| {
                if app.contains(self) && self.mounted(app) {
                    app.get_mut(self).open_timer = None;
                    self.open_submenu(app);
                }
            }),
        );
        app.get_mut(self).open_timer = Some(timer);
    }

    /// Stops a pending opening when the pointer leaves the item.
    fn cancel_open(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).open_timer.take() {
            timer.cancel(app);
        }
    }

    /// Starts the same source delay before closing a child menu.
    pub(super) fn delay_close(self: Handle<Self>, app: &mut App) {
        self.cancel_close(app);
        let timer = Timer::new(
            app,
            Duration::from_millis(400),
            Listener::new(move |app| {
                if app.contains(self) && self.mounted(app) {
                    app.get_mut(self).close_timer = None;
                    self.close_submenu(app);
                }
            }),
        );
        self.set_state(app, |state| state.close_timer = Some(timer));
    }

    /// Entering the owner or its submenu cancels delayed closing.
    pub(super) fn cancel_close(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).close_timer.take() {
            timer.cancel(app);
            self.set_state(app, |_| {});
        }
    }

    /// Opens the child menu after closing a previously opened peer.
    fn open_submenu(self: Handle<Self>, app: &mut App) {
        if !self.is_focusable(app) {
            return;
        }
        let Some(menu) = app.get(self).submenu else {
            return;
        };
        if menu.is_open(app) {
            return;
        }
        self.cancel_open(app);
        self.cancel_close(app);
        let presenter = self.widget(app).presenter;
        presenter.close_peer(app, self);
        app.get_mut(presenter).sub_item = Some(self);
        menu.show_at(app, app.get(self).target.unwrap());
    }

    /// Closes the complete child chain and returns focus to this owner item.
    pub(super) fn close_submenu(self: Handle<Self>, app: &mut App) {
        self.cancel_open(app);
        self.cancel_close(app);
        if let Some(menu) = app.get(self).submenu
            && menu.is_open(app)
        {
            menu.hide(app);
            self.focus_node(app).request_focus(app, None);
        }
        let presenter = self.widget(app).presenter;
        if app.get(presenter).sub_item == Some(self) {
            app.get_mut(presenter).sub_item = None;
        }
    }

    /// Source pointer-exit checks retain the child while crossing into its visible bounds.
    pub(super) fn pointer_left_presenter(self: Handle<Self>, app: &mut App, point: Offset) {
        let Some(menu) = app.get(self).submenu else {
            return;
        };
        if !menu.is_open(app) {
            return;
        }
        let child = menu.as_flyout(app).presenter_bounds(app);
        let owner = self
            .context(app)
            .find_render_object(app)
            .and_then(|object| object.as_box())
            .map(|object| {
                let origin = object.local_to_global(app, Offset::ZERO, None);
                origin & object.size(app)
            });
        let contains = |bounds: Option<Rect>| {
            bounds.is_some_and(|r| {
                point.dx() >= r.left
                    && point.dx() <= r.right
                    && point.dy() >= r.top
                    && point.dy() <= r.bottom
            })
        };
        if !contains(child) && !contains(owner) {
            self.delay_close(app);
        }
    }

    /// Builds the template with the current source CommonStates and SubMenuOpened state.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let shows_focus_visual = FocusState::current(app).shows_focus_visual();
        let state = app.get(self);
        let visual = MenuVisualState {
            common: if !widget.item.is_enabled {
                CommonState::Disabled
            } else if state.pressed {
                CommonState::Pressed
            } else if state.hovered {
                CommonState::PointerOver
            } else {
                CommonState::Normal
            },
            focused: state.focused && shows_focus_visual,
            submenu_open: state.close_timer.is_none()
                && state.submenu.is_some_and(|menu| menu.is_open(app)),
        };
        template::item_template(app, context, &widget.item, widget.settings, visual)
    }
}

impl State for MenuItemState {
    type Widget = MenuItemWidget;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let focus = FocusNode::new(app).as_node();
        focus.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.on_key(app, event))),
        );
        app.get_mut(self).focus = Some(focus);
        app.get_mut(widget.presenter)
            .containers
            .insert(widget.index, self);
        if let MenuFlyoutItemKind::SubItem { items } = widget.item.kind {
            let root = widget.menu.root(app);
            let menu = MenuFlyout::create(app, items, Some(root));
            app.get_mut(menu).owner_item = Some(self);
            app.get_mut(self).submenu = Some(menu);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &MenuItemWidget) {
        let widget = self.widget(app).clone();
        if old.index != widget.index
            && app.get(old.presenter).containers.get(&old.index) == Some(&self)
        {
            app.get_mut(old.presenter).containers.remove(&old.index);
        }
        app.get_mut(widget.presenter)
            .containers
            .insert(widget.index, self);
        if !widget.item.is_enabled {
            self.clear_press(app);
            self.close_submenu(app);
        }
        match (app.get(self).submenu, &widget.item.kind) {
            (Some(menu), MenuFlyoutItemKind::SubItem { items }) => menu.items(app, items.clone()),
            (Some(menu), _) => {
                menu.dispose(app);
                app.get_mut(self).submenu = None;
            }
            (None, MenuFlyoutItemKind::SubItem { items }) => {
                let root = widget.menu.root(app);
                let menu = MenuFlyout::create(app, items.clone(), Some(root));
                app.get_mut(menu).owner_item = Some(self);
                app.get_mut(self).submenu = Some(menu);
            }
            _ => {}
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.cancel_open(app);
        if let Some(timer) = app.get_mut(self).close_timer.take() {
            timer.cancel(app);
        }
        if let Some(menu) = app.get(self).submenu {
            menu.dispose(app);
        }
        let widget = self.widget(app);
        let presenter = widget.presenter;
        let index = widget.index;
        if app.contains(presenter) && app.get(presenter).containers.get(&index) == Some(&self) {
            app.get_mut(presenter).containers.remove(&index);
        }
        let focus = self.focus_node(app);
        focus.dispose(app);
        app.destroy(focus.id());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        if matches!(widget.item.kind, MenuFlyoutItemKind::Separator) {
            return template::separator(app, context);
        }
        let focus = self.focus_node(app);
        let template = if let Some(submenu) = app.get(self).submenu {
            ListenableBuilder::new(Rc::new(submenu.as_flyout(app)), move |app, context, _| {
                self.template(app, context)
            })
            .into_widget()
        } else {
            self.template(app, context)
        };
        let mut gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(template);
        if widget.item.is_enabled {
            gesture = gesture
                .on_tap_down(Rc::new(move |app, _| {
                    self.set_state(app, |state| {
                        state.pointer_down = true;
                        state.pressed = true;
                    });
                }))
                .on_tap_up(Rc::new(move |app, _| {
                    self.set_state(app, |state| {
                        state.should_perform_actions =
                            state.pressed && !state.key_down && !state.gamepad_down;
                        state.pointer_down = false;
                        state.pressed = false;
                    });
                }))
                .on_tap_cancel(Listener::new(move |app| self.clear_press(app)))
                .on_tap(Listener::new(move |app| {
                    if app.get(self).should_perform_actions {
                        app.get_mut(self).should_perform_actions = false;
                        self.invoke(app);
                    }
                }));
        }
        let control = FocusableActionDetector::new(gesture)
            .focus_node(focus)
            .enabled(widget.item.is_enabled)
            .on_focus_change(move |app, focused| {
                self.set_state(app, |state| state.focused = focused);
                if !focused {
                    self.clear_press(app);
                }
            });
        let control = MouseRegion::new()
            .on_enter(Rc::new(move |app, event| {
                if !self.is_focusable(app) {
                    return;
                }
                self.set_state(app, |state| state.hovered = true);
                let widget = self.widget(app).clone();
                if let Some(owner) = app.get(widget.menu).owner_item {
                    owner.cancel_close(app);
                }
                if app.get(self).submenu.is_some() && event.kind != PointerDeviceKind::Touch {
                    self.cancel_close(app);
                    self.delay_open(app);
                } else {
                    widget.presenter.delay_close_child(app);
                }
            }))
            .on_exit(Rc::new(move |app, event| {
                self.cancel_open(app);
                self.clear_press(app);
                self.set_state(app, |state| state.hovered = false);
                if event.kind == PointerDeviceKind::Mouse
                    && app.get(self.widget(app).menu).root.is_none()
                {
                    self.pointer_left_presenter(app, event.position);
                }
            }))
            .child(control);
        let control = control.into_widget();
        FlyoutTarget::new(Builder::new(move |app, context| {
            app.get_mut(self).target = Some(context);
            control.clone()
        }))
        .into_widget()
    }
}
