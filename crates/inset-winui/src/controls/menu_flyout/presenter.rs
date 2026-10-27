//! MenuFlyoutPresenter's shared sizing and keyboard focus order.

use super::{
    MenuFlyout,
    item::{MenuFlyoutItem, MenuFlyoutItemKind},
    item_widget::{MenuItemState, MenuItemWidget},
    template::MenuTemplateSettings,
};
use crate::*;
// Inset exports a `FocusState` of its own; the XAML one is meant here.
use crate::FocusState;
use inset_foundation::{App, Handle};
use inset_painting::{PaintingBinding, TextPainter, TextSpan};
use inset_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisSize};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{collections::BTreeMap, rc::Rc};

/// Source Border > ScrollViewer > ItemsPresenter template.
#[derive(Debug)]
pub(super) struct MenuFlyoutPresenter {
    /// Persistent menu whose collection is being displayed.
    pub menu: Handle<MenuFlyout>,
}

/// Mounted item containers and native vertical scrolling state.
pub(super) struct MenuFlyoutPresenterState {
    /// Native widget state.
    state: StateData<MenuFlyoutPresenter>,

    /// Mounted containers indexed by their current position in ItemsSource.
    pub containers: BTreeMap<usize, Handle<MenuItemState>>,

    /// ScrollViewer's retained vertical position.
    scroll: Option<Handle<ScrollViewportController>>,

    /// Item whose child submenu is currently open.
    pub sub_item: Option<Handle<MenuItemState>>,
}

impl StatefulWidget for MenuFlyoutPresenter {
    type State = MenuFlyoutPresenterState;

    fn create_state(&self) -> Self::State {
        MenuFlyoutPresenterState {
            state: StateData::new(),
            containers: BTreeMap::new(),
            scroll: None,
            sub_item: None,
        }
    }
}

impl MenuFlyoutPresenterState {
    /// Source opening reinstalls ItemsSource and starts with the first focusable item.
    pub(super) fn prepare_open(self: Handle<Self>, app: &mut App) {
        app.get(self).scroll.unwrap().change_view(app, 0.0, true);
        self.set_state(app, |_| {});
    }

    /// Source OnClosed clears the item containers; retained widgets reset transient input instead.
    pub(super) fn prepare_close(self: Handle<Self>, app: &mut App) {
        let items: Vec<_> = app.get(self).containers.values().copied().collect();
        for item in items {
            item.prepare_close(app);
        }
        app.get_mut(self).sub_item = None;
    }

    /// MenuFlyoutPresenter::CycleFocus with `FocusState_Keyboard`, excluding separators and disabled items.
    pub(super) fn cycle_focus(self: Handle<Self>, app: &mut App, down: bool) {
        let items: Vec<_> = app
            .get(self)
            .containers
            .values()
            .copied()
            .filter(|item| app.contains(*item) && item.mounted(app) && item.is_focusable(app))
            .collect();
        if items.is_empty() {
            return;
        }
        let focused = items
            .iter()
            .position(|item| item.focus_node(app).has_primary_focus(app));
        let next = match focused {
            Some(index) if down => (index + 1) % items.len(),
            Some(index) => (index + items.len() - 1) % items.len(),
            None if down => 0,
            None => items.len() - 1,
        };
        items[next].request_focus(app, FocusState::Keyboard);
    }

    /// An item hover starts the current child menu's source close delay.
    pub(super) fn delay_close_child(self: Handle<Self>, app: &mut App) {
        if let Some(item) = app.get(self).sub_item {
            item.delay_close(app);
        }
    }

    /// Closes a previously opened peer submenu before showing another.
    pub(super) fn close_peer(self: Handle<Self>, app: &mut App, next: Handle<MenuItemState>) {
        if let Some(item) = app.get(self).sub_item
            && item != next
        {
            item.close_submenu(app);
        }
    }

    /// Source presenter consumes Tab and cycles only with the vertical arrow keys.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            return KeyEventResult::Ignored;
        }
        let result = match event.logical_key() {
            LogicalKeyboardKey::ARROW_UP => {
                self.cycle_focus(app, false);
                KeyEventResult::Handled
            }
            LogicalKeyboardKey::ARROW_DOWN => {
                self.cycle_focus(app, true);
                KeyEventResult::Handled
            }
            LogicalKeyboardKey::TAB => KeyEventResult::Handled,
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ESCAPE => {
                let menu = self.widget(app).menu;
                if let Some(owner) = app.get(menu).owner_item {
                    owner.close_submenu(app);
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            }
            _ => KeyEventResult::Ignored,
        };
        if result == KeyEventResult::Ignored {
            self.widget(app).menu.owner_key(app, event)
        } else {
            result
        }
    }
}

/// Measures the same Caption text used by the source accelerator column.
fn template_settings(
    app: &mut App,
    context: BuildContext,
    items: &[MenuFlyoutItem],
    narrow: bool,
) -> MenuTemplateSettings {
    let theme = ThemeResources::of(app, context);
    let mut settings = MenuTemplateSettings {
        narrow,
        ..Default::default()
    };
    let fonts = PaintingBinding::instance(app).fonts(app);
    for item in items {
        settings.has_checks |= matches!(
            item.kind,
            MenuFlyoutItemKind::Toggle { .. } | MenuFlyoutItemKind::Radio { .. }
        );
        settings.has_icons |= item.icon.is_some();
        if matches!(
            item.kind,
            MenuFlyoutItemKind::SubItem { .. } | MenuFlyoutItemKind::Separator
        ) {
            continue;
        }
        let text = &item.keyboard_accelerator_text_override;
        settings.has_accelerators |= !text.is_empty();
        let mut painter = TextPainter::new();
        painter.set_text(Some(Rc::new(TextSpan::new().text(text.clone()).style(
            TextBlockStyle::Caption.text_style(&theme, theme.text_fill_color_secondary),
        ))));
        painter.set_text_direction(Some(Directionality::of(app, context)));
        painter.set_text_scaler(MediaQuery::text_scaler_of(app, context));
        painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
        settings.accelerator_width = settings.accelerator_width.max(painter.width());
        painter.dispose();
    }
    settings
}

impl State for MenuFlyoutPresenterState {
    type Widget = MenuFlyoutPresenter;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).scroll = Some(ScrollViewportController::new(app));
        let menu = self.widget(app).menu;
        app.get_mut(menu).presenter = Some(self);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).scroll.unwrap().dispose(app);
        let menu = self.widget(app).menu;
        app.get_mut(menu).presenter = None;
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let menu = self.widget(app).menu;
        let data = app.get(menu);
        let items = data.items.clone();
        let input = data.input;
        let theme = ThemeResources::of(app, context);
        let resources = &theme;
        let settings = template_settings(app, context, &items, input.is_narrow());
        let min_width = if input == crate::InputDevice::Touch {
            resources.flyout_theme_touch_min_width
        } else {
            resources.flyout_theme_min_width
        }
        .min(MediaQuery::size_of(app, context).width());
        let contents = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(items.into_iter().enumerate().map(|(index, item)| {
                MenuItemWidget {
                    item,
                    index,
                    menu,
                    presenter: self,
                    settings,
                }
                .into_widget()
            }));
        let container = ControlBorder::new(
            Brush::Acrylic(theme.acrylic_in_app_fill_color_default_brush),
            Brush::Solid(resources.menu_flyout_presenter_border_brush),
        )
        .border_thickness_ltrb(resources.menu_flyout_presenter_border_theme_thickness)
        .corner_radius_corners(resources.overlay_corner_radius)
        .child(Margin::new(
            resources.menu_flyout_presenter_theme_padding,
            ConstrainedBox::new(BoxConstraints::new().min_width(min_width)).child(
                ScrollBarViewport::new(app.get(self).scroll.unwrap(), contents),
            ),
        ));
        Focus::new(
            MouseRegion::new()
                .child(IntrinsicWidth::new().child(container))
                .on_enter(Rc::new(move |app, _| {
                    if let Some(owner) = app.get(menu).owner_item {
                        owner.cancel_close(app);
                    }
                }))
                .on_exit(Rc::new(move |app, event| {
                    if event.kind == inset_embedder::PointerDeviceKind::Mouse
                        && let Some(item) = app.get(self).sub_item
                    {
                        item.pointer_left_presenter(app, event.position);
                    }
                })),
        )
        .can_request_focus(false)
        .on_key_event(Rc::new(move |app, _, event| self.on_key(app, event)))
        .into_widget()
    }
}
