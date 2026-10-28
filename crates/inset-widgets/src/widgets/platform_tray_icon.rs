//! An icon in the system's tray, present while the widget is in the tree: the tray counterpart of
//! [`PlatformMenuBar`](super::platform_menu_bar::PlatformMenuBar), whose items its menu takes.
//! Flutter has no tray; the widget draws on three models:
//!
//! - **Flutter's `PlatformMenuBar`** for its form: a widget with no look of its own that hands a
//!   value of [`PlatformMenuItem`]s to the host on each change, and dispatches a choice to the
//!   item's `on_selected` or intent, by ids it hands out (one id space per icon here).
//! - **SwiftUI's `MenuBarExtra`** for its lifetime: the icon is there while the widget is in the
//!   tree. It is added when the widget is mounted, once the host answers it, and removed when the
//!   widget is unmounted; a rebuild updates its image, tooltip and menu.
//! - **Electron's `Tray.setContextMenu`** for when its menu shows: on each system's own trigger.
//!   Where the icon offers a menu the system opens by itself (macOS, Linux), the menu is handed to
//!   the system; elsewhere (Windows) it is opened at the icon when the user asks for its menu
//!   (`TrayIconEvent::ContextMenuRequested`). An app that wants another trigger uses the icon's
//!   capabilities directly (`Platform::tray_icons`).
//!
//! The invariant: the icon lives exactly as long as the widget's state, an icon answered after
//! the widget left the tree is dropped at once, and a choice from a menu set before a later one
//! finds no item, as the ids only grow.

use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{IconImage, MenuEntry, MenuId, TrayIcon, TrayIconError, TrayIconEvent};
use inset_foundation::{App, Handle, VoidCallback};

use crate::WidgetsBinding;
use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::SizedBox;
use crate::widgets::platform_menu_bar::{
    PlatformMenuItemRef, same_items, select_menu_item, strip_dividers,
};

/// An icon in the system's tray, with its image, its tooltip, its menu of the same items
/// [`PlatformMenuBar`](super::platform_menu_bar::PlatformMenuBar) takes, and what a click and a
/// double click do. It shows nowhere where the host has no tray (`Platform::tray_icons` is
/// `None`) or the system did not show it.
///
/// As far as the framework is concerned, this widget has no visual representation: it returns
/// its `child` from its build function. Several may be in the tree at once, each its own icon.
pub struct PlatformTrayIcon {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// The icon's image; a template image takes the colour the system gives its tray, on macOS.
    pub image: IconImage,
    /// The text the system shows when the pointer rests on the icon.
    pub tooltip: Option<String>,
    /// The items of the icon's menu. Whenever they change, a new list should be provided.
    pub menus: Vec<PlatformMenuItemRef>,
    /// Called when the user clicks the icon, or chooses it from the keyboard.
    pub on_activated: Option<VoidCallback>,
    /// Called when the user double-clicks the icon, where the system reports it.
    pub on_double_clicked: Option<VoidCallback>,
}

impl PlatformTrayIcon {
    /// Creates a [`PlatformTrayIcon`] showing `image`.
    pub fn new(image: IconImage) -> PlatformTrayIcon {
        PlatformTrayIcon {
            key: None,
            child: None,
            image,
            tooltip: None,
            menus: Vec::new(),
            on_activated: None,
            on_double_clicked: None,
        }
    }

    pub fn key(mut self, key: KeyRef) -> PlatformTrayIcon {
        self.key = Some(key);
        self
    }

    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> PlatformTrayIcon {
        self.child = Some(child.into_widget());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> PlatformTrayIcon {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn menus(mut self, menus: Vec<PlatformMenuItemRef>) -> PlatformTrayIcon {
        self.menus = menus;
        self
    }

    pub fn on_activated(mut self, on_activated: VoidCallback) -> PlatformTrayIcon {
        self.on_activated = Some(on_activated);
        self
    }

    pub fn on_double_clicked(mut self, on_double_clicked: VoidCallback) -> PlatformTrayIcon {
        self.on_double_clicked = Some(on_double_clicked);
        self
    }
}

impl Debug for PlatformTrayIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformTrayIcon")
            .field("tooltip", &self.tooltip)
            .field("menus", &self.menus)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PlatformTrayIcon {
    type State = PlatformTrayIconState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> PlatformTrayIconState {
        PlatformTrayIconState {
            state: StateData::new(),
            icon: None,
            entries: Vec::new(),
            id_map: HashMap::new(),
            serial: 0,
            descendants: Vec::new(),
        }
    }
}

pub struct PlatformTrayIconState {
    state: StateData<PlatformTrayIcon>,
    /// The icon, once the host has added it.
    icon: Option<Box<dyn TrayIcon>>,
    /// The menu as the icon was last given it, which a menu opened at the icon shows.
    entries: Vec<MenuEntry>,
    /// The items of that menu, by the ids handed out for them.
    id_map: HashMap<MenuId, PlatformMenuItemRef>,
    /// An ever increasing value used to dole out this icon's ids.
    serial: MenuId,
    /// The menu's items and their descendants, to tell a changed menu by.
    descendants: Vec<PlatformMenuItemRef>,
}

impl PlatformTrayIconState {
    /// Asks the host for the icon, which becomes this state's once it exists.
    fn add_icon(self: Handle<Self>, app: &mut App) {
        let platform = app.platform();
        let Some(tray_icons) = platform.tray_icons() else {
            return;
        };
        let widget = self.widget(app);
        let adding = tray_icons.add(&widget.image, widget.tooltip.as_deref());
        app.spawn(async move |cx| {
            let added = adding.await;
            cx.update(|app| self.icon_added(app, added));
        });
    }

    /// The host answered the icon: it is shown as the widget is now, and heard. An icon the
    /// system did not show shows nowhere, and one answered after the widget left the tree goes
    /// again at once.
    fn icon_added(
        self: Handle<Self>,
        app: &mut App,
        added: Result<Box<dyn TrayIcon>, TrayIconError>,
    ) {
        let Ok(icon) = added else {
            return;
        };
        if !self.mounted(app) {
            return;
        }
        let widget = self.widget(app);
        icon.set_image(&widget.image);
        icon.set_tooltip(widget.tooltip.as_deref());
        let id = icon.id();
        app.get_mut(self).icon = Some(icon);
        WidgetsBinding::instance(app).add_tray_icon_listener(
            app,
            id,
            Rc::new(move |app, event| self.handle_event(app, event)),
        );
        self.update_menu(app);
    }

    /// Builds the menu from the widget's items, with ids of this icon's own, and hands it to the
    /// system where the icon offers a menu the system opens by itself.
    fn update_menu(self: Handle<Self>, app: &mut App) {
        let menus = self.widget(app).menus.clone();
        let delegate = WidgetsBinding::instance(app).platform_menu_delegate(app);
        let mut serial = app.get(self).serial;
        let mut id_map = HashMap::new();
        let mut entries = Vec::new();
        for item in &menus {
            entries.extend(Rc::clone(item).to_menu_entries(&*delegate, &mut |item| {
                serial += 1;
                id_map.insert(serial, item);
                serial
            }));
        }
        strip_dividers(&mut entries);
        let state = app.get_mut(self);
        state.serial = serial;
        state.id_map = id_map;
        state.entries = entries;
        let state = app.get(self);
        if let Some(attached) = state.icon.as_ref().and_then(|icon| icon.attached_menu()) {
            attached.set_menu(&state.entries);
        }
    }

    /// What the host reports of the icon.
    fn handle_event(self: Handle<Self>, app: &mut App, event: TrayIconEvent) {
        match event {
            TrayIconEvent::Activated { .. } => {
                if let Some(on_activated) = self.widget(app).on_activated.clone() {
                    on_activated(app);
                }
            }
            TrayIconEvent::DoubleClicked => {
                if let Some(on_double_clicked) = self.widget(app).on_double_clicked.clone() {
                    on_double_clicked(app);
                }
            }
            TrayIconEvent::ContextMenuRequested { .. } => self.open_menu_at_icon(app),
            TrayIconEvent::MenuSelected(id) => self.select(app, id),
            TrayIconEvent::SecondaryActivated { .. } | TrayIconEvent::Scrolled { .. } => {}
        }
    }

    /// The user asked for the icon's menu: it opens at the icon where the system opens none by
    /// itself.
    fn open_menu_at_icon(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let Some(icon) = &state.icon else {
            return;
        };
        if icon.attached_menu().is_some() || state.entries.is_empty() {
            return;
        }
        let Some(at_icon) = icon.menu_at_icon() else {
            return;
        };
        let shown = at_icon.show(&state.entries);
        app.spawn(async move |cx| {
            if let Some(id) = shown.await {
                cx.update(|app| {
                    if self.mounted(app) {
                        self.select(app, id);
                    }
                });
            }
        });
    }

    /// The item `id` of the icon's menu was chosen: its callback or intent runs. An id of a menu
    /// set before the last finds nothing.
    fn select(self: Handle<Self>, app: &mut App, id: MenuId) {
        let item = app.get(self).id_map.get(&id).cloned();
        if let Some(item) = item {
            select_menu_item(app, &item);
        }
    }
}

impl State for PlatformTrayIconState {
    type Widget = PlatformTrayIcon;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let mut descendants = Vec::new();
        for item in &self.widget(app).menus {
            descendants.push(Rc::clone(item));
            descendants.extend(item.descendants());
        }
        app.get_mut(self).descendants = descendants;
        self.add_icon(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(icon) = app.get_mut(self).icon.take() {
            WidgetsBinding::instance(app).remove_tray_icon_listener(app, icon.id());
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &PlatformTrayIcon) {
        let widget = self.widget(app);
        let (image, tooltip) = (widget.image.clone(), widget.tooltip.clone());
        if let Some(icon) = &app.get(self).icon {
            if image != old_widget.image {
                icon.set_image(&image);
            }
            if tooltip != old_widget.tooltip {
                icon.set_tooltip(tooltip.as_deref());
            }
        }
        let mut new_descendants = Vec::new();
        for item in &self.widget(app).menus {
            new_descendants.push(Rc::clone(item));
            new_descendants.extend(item.descendants());
        }
        if !same_items(&new_descendants, &app.get(self).descendants) {
            app.get_mut(self).descendants = new_descendants;
            if app.get(self).icon.is_some() {
                self.update_menu(app);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.widget(app)
            .child
            .clone()
            .unwrap_or_else(|| SizedBox::new().into_widget())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::time::Duration;

    use inset_embedder::{Offset, TargetPlatform};
    use inset_foundation::AppCell;
    use inset_test::{MockTrayIcons, TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::test_harness::pump_every_view;
    use crate::widgets::platform_menu_bar::{
        PlainPlatformMenuItem, PlatformMenuItemGroup, PlatformProvidedMenuItem,
        PlatformProvidedMenuItemType,
    };

    type Heard = Rc<RefCell<Vec<&'static str>>>;

    fn app_with_tray(tray: &MockTrayIcons) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::new(TestView::new(800.0, 600.0)))
            .with_tray_icons(Rc::new(tray.clone()));
        AppCell::with_platform(Rc::new(platform))
    }

    /// Mounts `widget` as the app's content, and lets the icon the host answers arrive.
    fn mount(cell: &AppCell, widget: WidgetRef) {
        run_app(&mut cell.borrow_mut(), widget);
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
        cell.elapse(Duration::ZERO);
    }

    fn image() -> IconImage {
        IconImage {
            width: 1,
            height: 1,
            rgba: Arc::from([0u8, 0, 0, 255]),
            template: true,
        }
    }

    fn heard(heard: &Heard, what: &'static str) -> VoidCallback {
        let heard = Rc::clone(heard);
        Rc::new(move |_app| heard.borrow_mut().push(what))
    }

    /// An icon whose menu is Open, a group holding Save, disabled, and the system's Copy, and
    /// whose clicks are heard into `heard`.
    fn icon(heard_into: &Heard, tooltip: &str) -> PlatformTrayIcon {
        PlatformTrayIcon::new(image())
            .tooltip(tooltip)
            .menus(vec![
                Rc::new(PlainPlatformMenuItem::new("Open").on_selected(heard(heard_into, "Open"))),
                Rc::new(PlatformMenuItemGroup::new(vec![Rc::new(
                    PlainPlatformMenuItem::new("Save"),
                )])),
                Rc::new(PlatformProvidedMenuItem::new(
                    PlatformProvidedMenuItemType::Copy,
                )),
            ])
            .on_activated(heard(heard_into, "activated"))
            .on_double_clicked(heard(heard_into, "double clicked"))
    }

    /// Reports `event` of the icon `id` as the host does.
    fn report(cell: &AppCell, id: u64, event: TrayIconEvent) {
        {
            let mut app = cell.borrow_mut();
            let callback = app
                .platform_callbacks()
                .on_tray_icon_event
                .clone()
                .expect("the binding hears tray icon events");
            callback(&mut app, id, event);
        }
        cell.elapse(Duration::ZERO);
    }

    fn the_menu() -> Vec<MenuEntry> {
        vec![
            MenuEntry::Item {
                id: 1,
                item: inset_embedder::MenuItem::new("Open"),
            },
            MenuEntry::Separator,
            MenuEntry::Item {
                id: 2,
                item: inset_embedder::MenuItem::new("Save").enabled(false),
            },
            MenuEntry::Separator,
            MenuEntry::Provided {
                id: 3,
                item: inset_embedder::ProvidedItem::Copy,
                enabled: true,
            },
        ]
    }

    #[test]
    fn where_the_system_opens_the_menu_it_is_handed_the_menu_and_reports_the_choice() {
        let tray = MockTrayIcons::new().without_menus_at_icon();
        let cell = app_with_tray(&tray);
        let heard = Heard::default();
        mount(&cell, icon(&heard, "Notes").into_widget());

        let [shown] = &tray.icons()[..] else {
            panic!("one icon: {:?}", tray.icons());
        };
        assert_eq!(shown.tooltip.as_deref(), Some("Notes"));
        assert_eq!(shown.attached_menu, the_menu());
        report(&cell, shown.id, TrayIconEvent::MenuSelected(1));
        report(
            &cell,
            shown.id,
            TrayIconEvent::ContextMenuRequested {
                position: Offset::ZERO,
            },
        );
        assert!(
            tray.icons()[0].menus_opened.is_empty(),
            "the system opens the menu itself"
        );
        report(
            &cell,
            shown.id,
            TrayIconEvent::Activated {
                position: Offset::ZERO,
            },
        );
        report(&cell, shown.id, TrayIconEvent::DoubleClicked);
        assert_eq!(*heard.borrow(), ["Open", "activated", "double clicked"]);
    }

    #[test]
    fn elsewhere_the_menu_opens_at_the_icon_when_the_user_asks_for_it() {
        let tray = MockTrayIcons::new()
            .without_attached_menus()
            .answering(Some(1));
        let cell = app_with_tray(&tray);
        let heard = Heard::default();
        mount(&cell, icon(&heard, "Notes").into_widget());

        let id = tray.icons()[0].id;
        assert!(heard.borrow().is_empty());
        report(
            &cell,
            id,
            TrayIconEvent::ContextMenuRequested {
                position: Offset::ZERO,
            },
        );
        assert_eq!(tray.icons()[0].menus_opened, [the_menu()]);
        assert_eq!(*heard.borrow(), ["Open"], "the choice reaches its item");
    }

    #[test]
    fn the_icon_is_there_while_the_widget_is_and_follows_its_rebuilds() {
        let tray = MockTrayIcons::new();
        let cell = app_with_tray(&tray);
        let heard = Heard::default();
        mount(&cell, icon(&heard, "Notes").into_widget());
        let id = tray.icons()[0].id;

        mount(
            &cell,
            icon(&heard, "Notes, 3 new")
                .menus(vec![Rc::new(PlainPlatformMenuItem::new("Quit"))])
                .into_widget(),
        );
        let [shown] = &tray.icons()[..] else {
            panic!("the same icon: {:?}", tray.icons());
        };
        assert_eq!(shown.id, id, "a rebuild keeps the icon");
        assert_eq!(shown.tooltip.as_deref(), Some("Notes, 3 new"));
        let [MenuEntry::Item { item, .. }] = &shown.attached_menu[..] else {
            panic!("the new menu: {:?}", shown.attached_menu);
        };
        assert_eq!(item.label, "Quit");

        mount(&cell, SizedBox::new().into_widget());
        assert!(tray.icons().is_empty(), "the icon goes with the widget");
    }

    #[test]
    fn several_icons_are_each_their_own() {
        let tray = MockTrayIcons::new();
        let cell = app_with_tray(&tray);
        let (first, second) = (Heard::default(), Heard::default());
        mount(
            &cell,
            icon(&first, "First")
                .child(icon(&second, "Second"))
                .into_widget(),
        );
        let icons = tray.icons();
        assert_eq!(icons.len(), 2);
        report(&cell, icons[1].id, TrayIconEvent::MenuSelected(1));
        assert!(first.borrow().is_empty());
        assert_eq!(*second.borrow(), ["Open"], "ids are each icon's own");
    }

    #[test]
    fn an_icon_the_system_refuses_shows_nowhere() {
        let tray = MockTrayIcons::new().refusing("no tray");
        let cell = app_with_tray(&tray);
        mount(&cell, icon(&Heard::default(), "Notes").into_widget());
        assert!(tray.icons().is_empty());
    }
}
