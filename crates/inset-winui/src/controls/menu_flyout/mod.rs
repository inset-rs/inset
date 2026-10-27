//! MenuFlyout's collection, presenter and cascading item behavior.

mod item;
mod item_widget;
mod presenter;
mod template;

use crate::InputDevice;
pub use item::{MenuCheckedChanged, MenuFlyoutItem, MenuFlyoutItemKind};
use presenter::{MenuFlyoutPresenter, MenuFlyoutPresenterState};

use crate::*;
use inset_embedder::Size;
use inset_foundation::{App, Handle, Listener};
use inset_services::KeyEvent;
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// MenuFlyoutPresenter's style: at least `FlyoutThemeMinWidth` wide and `MenuFlyoutThemeMinHeight`
/// tall, and no maximum.
fn menu_flyout_presenter_sizes(r: &ThemeResources) -> (Size, Size) {
    (
        Size::new(r.flyout_theme_min_width, r.menu_flyout_theme_min_height),
        Size::new(f64::INFINITY, f64::INFINITY),
    )
}

/// MenuBar receives unhandled keys from the menu collection that originated them.
pub(crate) type MenuKeyHandler =
    Rc<dyn Fn(&mut App, Handle<MenuFlyout>, &KeyEvent) -> KeyEventResult>;

/// Persistent owner of a collection displayed by a MenuFlyoutPresenter.
pub struct MenuFlyout {
    /// Shared flyout lifecycle and overlay ownership.
    flyout: Handle<Flyout>,

    /// Configuration supplied by the application, in display order.
    items: Vec<MenuFlyoutItem>,

    /// Mounted presenter, retained between openings.
    presenter: Option<Handle<MenuFlyoutPresenterState>>,

    /// Root menu closed when a command is invoked in any submenu.
    root: Option<Handle<MenuFlyout>>,

    /// Item that owns this submenu, when this is not the root collection.
    owner_item: Option<Handle<item_widget::MenuItemState>>,

    /// Opening device, captured once for each menu opening.
    input: InputDevice,

    /// Prevents late use after the owning state disposes the menu.
    disposed: bool,

    /// Owner navigation after the menu's own keyboard handling.
    pub(crate) key_handler: Option<MenuKeyHandler>,
}

impl fmt::Debug for MenuFlyout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuFlyout")
            .field("items", &self.items)
            .field("disposed", &self.disposed)
            .finish_non_exhaustive()
    }
}

impl MenuFlyout {
    /// Records the current raw pointer before a header opens its menu on press or entry.
    pub(crate) fn pointer_input(app: &mut App, kind: inset_embedder::PointerDeviceKind) {
        InputDevice::pointer(app, kind);
    }

    /// Creates a root menu; checked values follow the kit's owner-state convention.
    pub fn new(app: &mut App, items: Vec<MenuFlyoutItem>) -> Handle<Self> {
        Self::create(app, items, None)
    }

    /// Creates a child collection with the same root invocation owner.
    fn create(
        app: &mut App,
        items: Vec<MenuFlyoutItem>,
        root: Option<Handle<Self>>,
    ) -> Handle<Self> {
        InputDevice::initialize(app);
        let flyout = Flyout::new(app, SizedBox::shrink());
        let menu = app.create(Self {
            flyout,
            items,
            presenter: None,
            root,
            owner_item: None,
            input: InputDevice::None,
            disposed: false,
            key_handler: None,
        });
        let base = app.get_mut(flyout);
        base.is_sub_menu = root.is_some();
        base.presenter_sizes = menu_flyout_presenter_sizes;
        base.on_opening = Some(Listener::new(move |app| menu.on_opening(app)));
        base.on_closed = Some(Listener::new(move |app| {
            if let Some(presenter) = app.get(menu).presenter {
                presenter.prepare_close(app);
            }
        }));
        base.flyout_presenter_style = Some(Rc::new(move |_, _, _| {
            MenuFlyoutPresenter { menu }.into_widget()
        }));
        menu
    }

    /// Returns the shared flyout object for DropDownButton, placement and lifecycle callbacks.
    pub fn as_flyout(self: Handle<Self>, app: &App) -> Handle<Flyout> {
        app.get(self).flyout
    }

    /// Replaces the item configuration while preserving keyed item state.
    pub fn items(self: Handle<Self>, app: &mut App, items: Vec<MenuFlyoutItem>) {
        assert!(
            !app.get(self).disposed,
            "cannot update a disposed MenuFlyout"
        );
        app.get_mut(self).items = items;
        self.as_flyout(app).rebuild_content(app);
    }

    /// Shows from a context beneath FlyoutTarget.
    pub fn show_at(self: Handle<Self>, app: &mut App, target: BuildContext) {
        self.as_flyout(app).show_at(app, target);
    }

    /// Shows with per-opening placement and input policy.
    pub fn show_at_with_options(
        self: Handle<Self>,
        app: &mut App,
        target: BuildContext,
        options: FlyoutShowOptions,
    ) {
        self.as_flyout(app)
            .show_at_with_options(app, target, options);
    }

    /// Hides this menu and its open descendants.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        self.as_flyout(app).hide(app);
    }

    /// Whether this menu is currently shown.
    pub fn is_open(self: Handle<Self>, app: &App) -> bool {
        self.as_flyout(app).is_open(app)
    }

    /// Releases the retained presenter and its child menus.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }
        app.get_mut(self).disposed = true;
        self.as_flyout(app).dispose(app);
    }

    /// Captures the source padding mode and resets the collection's scroll position.
    fn on_opening(self: Handle<Self>, app: &mut App) {
        let input = if let Some(root) = app.get(self).root {
            app.get(root).input
        } else {
            InputDevice::current(app)
        };
        app.get_mut(self).input = input;
        self.as_flyout(app)
            .menu_point_placement(app, input == InputDevice::Touch);
        if let Some(presenter) = app.get(self).presenter {
            presenter.prepare_open(app);
        }
    }

    /// Whether the currently focused entry owns children and must keep its arrow keys.
    pub(crate) fn focused_item_has_children(self: Handle<Self>, app: &App) -> bool {
        app.get(self).presenter.is_some_and(|presenter| {
            app.get(presenter).containers.values().any(|item| {
                item.focus_node(app).has_primary_focus(app)
                    && matches!(&item.widget(app).item.kind, MenuFlyoutItemKind::SubItem { items } if !items.is_empty())
            })
        })
    }

    /// Bubbles keys from nested collections to a MenuBar owner, when present.
    fn owner_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        let root = self.root(app);
        app.get(root)
            .key_handler
            .clone()
            .map_or(KeyEventResult::Ignored, |handler| handler(app, self, event))
    }

    /// The root collection owns command dismissal across the entire chain.
    fn root(self: Handle<Self>, app: &App) -> Handle<Self> {
        app.get(self).root.unwrap_or(self)
    }
}
