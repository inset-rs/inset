//! MenuBar.cpp, MenuBarItem.cpp and their templates: one header group and reusable menus.

use crate::*;
// Inset exports a `FocusState` of its own; the XAML one is meant here.
use crate::FocusState;
use inset_embedder::{FontWeight, Offset, Rect, TextDirection};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{BoxConstraints, CrossAxisAlignment};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// MenuBarItemFlyout has the same public behavior as MenuFlyout; the controller already retains its presenter.
pub type MenuBarItemFlyout = MenuFlyout;

/// A menu header and its commands, with stable identity across collection rebuilds.
#[derive(Clone, Debug)]
pub struct MenuBarItem {
    /// Unique identity within the bar, independent of the displayed title.
    pub id: String,

    /// Text displayed by ContentButton.
    pub title: String,

    /// Commands copied into this header's MenuFlyout.
    pub items: Vec<MenuFlyoutItem>,

    /// Enables header activation and programmatic focus.
    pub is_enabled: bool,
}

impl MenuBarItem {
    /// Creates a header whose identity survives title and command changes.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        items: Vec<MenuFlyoutItem>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items,
            is_enabled: true,
        }
    }

    /// Enables or disables the header and its menu opening behavior.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }
}

/// Horizontal menu headers with one Tab entry and source arrow-key navigation.
#[derive(Clone, Debug)]
pub struct MenuBar {
    /// Headers in display order; ids must be unique.
    pub items: Vec<MenuBarItem>,

    /// Enables the bar and all enabled headers.
    pub is_enabled: bool,

    /// Optional replacement for MenuBarBackground.
    pub background: Option<Brush>,

    /// Identity across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl MenuBar {
    /// Creates a menu bar from its header descriptions.
    pub fn new(items: Vec<MenuBarItem>) -> Self {
        Self {
            items,
            is_enabled: true,
            background: None,
            key: None,
        }
    }

    /// Enables or disables all headers.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Replaces the bar's background brush.
    pub fn background(mut self, value: Brush) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets the widget's identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

/// Live header controls and the item remembered by TabNavigation.Once.
pub struct MenuBarState {
    /// Native widget state.
    state: StateData<MenuBar>,

    /// Live controls corresponding to the source Items collection.
    headers: HashMap<String, Handle<MenuHeaderState>>,

    /// Last focused item, retained when focus leaves the bar.
    last_focused: Option<String>,
}

impl StatefulWidget for MenuBar {
    type State = MenuBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        MenuBarState {
            state: StateData::new(),
            headers: HashMap::new(),
            last_focused: None,
        }
    }
}

impl MenuBarState {
    /// Whether a live header's flyout is open, including its opening frame.
    fn is_flyout_open(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .headers
            .values()
            .any(|header| header.menu(app).is_open(app))
    }

    /// Source MoveFocusTo uses one neighbor; OpenFlyoutFrom skips disabled neighbors.
    fn move_from(self: Handle<Self>, app: &mut App, id: &str, direction: isize, open: bool) {
        let items = self.widget(app).items.clone();
        let Some(index) = items.iter().position(|item| item.id == id) else {
            return;
        };
        let count = items.len();
        if count == 0 {
            return;
        }
        if open && let Some(header) = app.get(self).headers.get(id).copied() {
            header.menu(app).hide(app);
        }
        for step in 1..=if open { count } else { 1 } {
            let index =
                (index as isize + direction * step as isize).rem_euclid(count as isize) as usize;
            let Some(header) = app.get(self).headers.get(&items[index].id).copied() else {
                continue;
            };
            if header.enabled(app) {
                let state = FocusState::coerce_programmatic(app);
                header.request_focus(app, state);
                if open {
                    header.show(app);
                }
                break;
            }
        }
    }

    /// Remembers the last header for the next Tab entry into the bar.
    fn focused(self: Handle<Self>, app: &mut App, id: String) {
        if app.get(self).last_focused.as_ref() != Some(&id) {
            self.set_state(app, |state| state.last_focused = Some(id));
        }
    }
}

impl State for MenuBarState {
    type Widget = MenuBar;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let mut ids = HashSet::new();
        assert!(
            widget.items.iter().all(|item| ids.insert(&item.id)),
            "MenuBar item ids must be unique"
        );
        let remembered = app
            .get(self)
            .last_focused
            .as_ref()
            .and_then(|id| {
                widget
                    .items
                    .iter()
                    .find(|item| item.id == *id && item.is_enabled)
            })
            .or_else(|| widget.items.iter().find(|item| item.is_enabled))
            .map(|item| item.id.clone());
        let resources = ThemeResources::of(app, context);
        let items = widget
            .items
            .into_iter()
            .map(|item| {
                let key = Rc::new(ValueKey::new(item.id.clone()));
                MenuHeader {
                    tab_stop: remembered.as_ref() == Some(&item.id),
                    enabled: widget.is_enabled && item.is_enabled,
                    item,
                    owner: self,
                    key,
                }
                .into_widget()
            })
            .collect::<Vec<_>>();
        ConstrainedBox::new(BoxConstraints::new().min_height(resources.menu_bar_height))
            .child(
                ControlBorder::new_optional(
                    Some(
                        widget
                            .background
                            .unwrap_or(Brush::Solid(resources.menu_bar_background)),
                    ),
                    None,
                )
                .border_thickness(0.0)
                .child(
                    IntrinsicHeight::new().child(
                        Row::new()
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children(items),
                    ),
                ),
            )
            .into_widget()
    }
}

/// MenuBarItem's template with its collection owner and focus-traversal membership.
#[derive(Clone, Debug)]
struct MenuHeader {
    /// Application-supplied header and commands.
    item: MenuBarItem,

    /// The bar whose Items collection defines navigation order.
    owner: Handle<MenuBarState>,

    /// Whether Tab enters this header on its next visit.
    tab_stop: bool,

    /// Effective enabled state, including the parent bar.
    enabled: bool,

    /// Stable identity derived from the application item id.
    key: KeyRef,
}

/// Owns the header's focus and reusable flyout, as MenuBarItem does.
struct MenuHeaderState {
    /// Native widget state.
    state: StateData<MenuHeader>,

    /// Header focus, separate from the non-tab-stop ContentButton.
    focus: Option<AnyFocusNode>,

    /// The source MenuBarItemFlyout presenter and item collection.
    menu: Option<Handle<MenuBarItemFlyout>>,

    /// Context below FlyoutTarget, excluding the header's outer margin.
    target: Option<BuildContext>,

    /// Whether the header has the focus.
    focused: bool,
}

impl StatefulWidget for MenuHeader {
    type State = MenuHeaderState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_state(&self) -> Self::State {
        MenuHeaderState {
            state: StateData::new(),
            focus: None,
            menu: None,
            target: None,
            focused: false,
        }
    }
}

impl MenuHeaderState {
    /// Effective IsEnabled, including the containing MenuBar.
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).enabled
    }

    /// The owned MenuBarItemFlyout collection.
    fn menu(self: Handle<Self>, app: &App) -> Handle<MenuFlyout> {
        app.get(self).menu.unwrap()
    }

    /// The outer control's keyboard focus identity.
    fn focus(self: Handle<Self>, app: &App) -> AnyFocusNode {
        app.get(self).focus.unwrap()
    }

    /// `Focus(FocusState)` on this header as MoveFocusTo calls it.
    fn request_focus(self: Handle<Self>, app: &mut App, state: FocusState) {
        let node = self.focus(app);
        FocusState::name(app, node, state);
        node.request_focus(app, None);
        self.set_state(app, |_| {});
    }

    /// ShowMenuFlyout uses a point at the button's bottom edge and excludes the button itself.
    fn show(self: Handle<Self>, app: &mut App) {
        if !self.enabled(app) || self.widget(app).item.items.is_empty() {
            return;
        }
        let Some(context) = app.get(self).target else {
            return;
        };
        let Some(render) = context
            .find_render_object(app)
            .and_then(|render| render.as_box())
        else {
            return;
        };
        let size = render.size(app);
        let rtl = Directionality::of(app, context) == TextDirection::Rtl;
        self.menu(app).show_at_with_options(
            app,
            context,
            FlyoutShowOptions {
                position: Some(Offset::new(
                    if rtl { size.width() } else { 0.0 },
                    size.height(),
                )),
                exclusion_rect: Some(Rect::from_ltwh(0.0, 0.0, size.width(), size.height())),
                placement: Some(FlyoutPlacementMode::Bottom),
                ..Default::default()
            },
        );
    }

    /// Native grouped headers explicitly perform the source Invoke open/close operation.
    fn press(self: Handle<Self>, app: &mut App) {
        if !self.enabled(app) {
            return;
        }
        if self.menu(app).is_open(app) {
            self.menu(app).hide(app);
        } else {
            self.show(app);
        }
    }

    /// Header keys move one neighbor; popup keys skip disabled headers and switch menus.
    fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent, from_menu: bool) -> KeyEventResult {
        if (!from_menu && !self.enabled(app))
            || !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_))
        {
            return KeyEventResult::Ignored;
        }
        if !from_menu && HardwareKeyboard::instance(app).is_alt_pressed(app) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if !from_menu
            && matches!(
                key,
                LogicalKeyboardKey::ARROW_DOWN
                    | LogicalKeyboardKey::ENTER
                    | LogicalKeyboardKey::SPACE
            )
        {
            self.show(app);
            return KeyEventResult::Handled;
        }
        if matches!(
            key,
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT
        ) {
            let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
            let forward = (key == LogicalKeyboardKey::ARROW_RIGHT) != rtl;
            let widget = self.widget(app).clone();
            widget.owner.move_from(
                app,
                &widget.item.id,
                if forward { 1 } else { -1 },
                from_menu,
            );
            return KeyEventResult::Handled;
        }
        KeyEventResult::Ignored
    }
}

impl MenuHeaderState {
    /// ContentRoot, Background and ContentButton share the source visual states.
    fn content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let theme = ThemeResources::of(app, context);
        let resources = theme.clone();
        let menu = self.menu(app);
        let title = widget.item.title.clone();
        let content = CommonStates::new(Listener::new(|_| {}), move |app, _, states| {
            let selected = menu.is_open(app);
            let (background, border) = match states.common {
                CommonState::Pressed => (
                    resources.menu_bar_item_background_pressed,
                    resources.menu_bar_item_border_brush_pressed,
                ),
                CommonState::PointerOver => (
                    resources.menu_bar_item_background_pointer_over,
                    resources.menu_bar_item_border_brush_pointer_over,
                ),
                _ if selected => (
                    resources.menu_bar_item_background_selected,
                    resources.menu_bar_item_border_brush_selected,
                ),
                _ => (
                    resources.menu_bar_item_background,
                    resources.menu_bar_item_border_brush,
                ),
            };
            let foreground = if states.common == CommonState::Disabled {
                theme.button_foreground_disabled
            } else if states.common == CommonState::Pressed {
                theme.button_foreground_pressed
            } else {
                resources.menu_bar_item_foreground
            };
            ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                .border_thickness_ltrb(resources.menu_bar_item_border_thickness)
                .corner_radius_corners(resources.control_corner_radius)
                .child(
                    Padding::new(EdgeInsetsGeometry::from_ltrb(
                        resources.menu_bar_item_button_padding[0],
                        resources.menu_bar_item_button_padding[1],
                        resources.menu_bar_item_button_padding[2],
                        resources.menu_bar_item_button_padding[3],
                    ))
                    .child(
                        Center::new().width_factor(1.0).height_factor(1.0).child(
                            Text::new(title.clone()).style(control_text_style(
                                &theme,
                                theme.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                foreground,
                            )),
                        ),
                    ),
                )
                .into_widget()
        })
        .is_enabled(widget.enabled)
        .is_tab_stop(false);
        content.into_widget()
    }
}

impl State for MenuHeaderState {
    type Widget = MenuHeader;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let node = FocusNode::new(app).as_node();
        node.set_skip_traversal(app, !widget.tab_stop);
        node.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.key(app, event, false))),
        );
        app.get_mut(self).focus = Some(node);
        let menu = MenuFlyout::new(app, widget.item.items.clone());
        app.get_mut(self).menu = Some(menu);
        app.get_mut(widget.owner)
            .headers
            .insert(widget.item.id.clone(), self);
        let flyout = menu.as_flyout(app);
        app.get_mut(flyout).tap_region_group = Some(widget.owner.id());
        flyout.return_focus_to(app, node);
        app.get_mut(flyout).opening = Some(Listener::new(move |app| {
            self.focus(app).request_focus(app, None);
        }));
        app.get_mut(menu).key_handler = Some(Rc::new(move |app, source, event| {
            if source.focused_item_has_children(app) {
                KeyEventResult::Ignored
            } else {
                self.key(app, event, true)
            }
        }));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &MenuHeader) {
        let widget = self.widget(app).clone();
        self.focus(app).set_skip_traversal(app, !widget.tab_stop);
        self.menu(app).items(app, widget.item.items.clone());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        app.get_mut(widget.owner).headers.remove(&widget.item.id);
        self.menu(app).dispose(app);
        let node = self.focus(app);
        node.dispose(app);
        app.destroy(node.id());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let flyout = self.menu(app).as_flyout(app);
        let content = ListenableBuilder::new(Rc::new(flyout), move |app, context, _| {
            self.content(app, context)
        })
        .into_widget();
        let content = MouseRegion::new()
            .on_enter(Rc::new(move |app, event| {
                MenuFlyout::pointer_input(app, event.kind);
                if self.widget(app).owner.is_flyout_open(app) {
                    self.show(app);
                }
            }))
            .child(
                inset_widgets::Listener::new()
                    .on_pointer_down(Rc::new(move |app, event| {
                        MenuFlyout::pointer_input(app, event.kind);
                        self.press(app);
                    }))
                    .child(FlyoutTarget::new(Builder::new(move |app, context| {
                        app.get_mut(self).target = Some(context);
                        content.clone()
                    }))),
            );
        let content = FocusableActionDetector::new(content)
            .focus_node(self.focus(app))
            .enabled(widget.enabled)
            .on_focus_change(move |app, focused| {
                self.set_state(app, |state| state.focused = focused);
                if focused {
                    let widget = self.widget(app).clone();
                    widget.owner.focused(app, widget.item.id);
                }
            });
        let shows_focus_visual =
            app.get(self).focused && FocusState::current(app).shows_focus_visual();
        Margin::new(
            r.menu_bar_item_margin,
            TapRegion::new(
                FocusVisual::new(content, ThemeResources::of(app, context).theme)
                    .visible(shows_focus_visual)
                    .margin([-3.0; 4])
                    .corner_radius(r.control_corner_radius[0]),
            )
            .group_id(widget.owner.id())
            .enabled(widget.enabled),
        )
        .into_widget()
    }
}
