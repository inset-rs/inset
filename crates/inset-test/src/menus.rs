//! The system's menus in memory, as a host draws none: a menu bar and a Dock menu that keep what
//! the framework set, menus opened at a point or at a tray icon that keep what they were shown and
//! answer the choice the test gave, and tray icons that keep their image, tooltip and attached
//! menu while they are there. A test chooses an item of the menu bar as the user would through
//! the client's `menu_selected`. The menu bar keeps the keys the app offers it, and takes those
//! whose key is one of the shortcuts the test gave.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use inset_embedder::{
    AttachedMenu, DockMenu, IconImage, KeyData, KeyEventType, MenuAtIcon, MenuBar, MenuEntry,
    MenuId, MenuShortcut, Offset, PopupMenuFuture, PopupMenus, TrayIcon, TrayIconError,
    TrayIconFuture, TrayIconId, TrayIcons, ViewId,
};

/// A menu bar that keeps the menus last set. Every clone is the same menu bar.
#[derive(Clone, Default)]
pub struct MockMenuBar(Rc<RefCell<MenuBarState>>);

#[derive(Default)]
struct MenuBarState {
    menus: Vec<MenuEntry>,
    sets: usize,
    /// The shortcuts whose keys the menu bar takes.
    shortcuts: Vec<MenuShortcut>,
    /// The keys offered, in order.
    offered: Vec<KeyData>,
}

impl MockMenuBar {
    pub fn new() -> MockMenuBar {
        MockMenuBar::default()
    }

    /// A menu bar that takes a key-down offered (`perform_key_equivalent`) whose logical key is
    /// one of `shortcuts`' keys. A key's data carries no modifiers, which a host's menu bar reads
    /// from the system's own event, so the mock matches the key alone.
    pub fn taking(self, shortcuts: impl IntoIterator<Item = MenuShortcut>) -> MockMenuBar {
        self.0.borrow_mut().shortcuts = shortcuts.into_iter().collect();
        self
    }

    /// The keys offered, in order, taken or not.
    pub fn offered(&self) -> Vec<KeyData> {
        self.0.borrow().offered.clone()
    }

    /// The menus last set.
    pub fn menus(&self) -> Vec<MenuEntry> {
        self.0.borrow().menus.clone()
    }

    /// How many times the menus were set.
    pub fn sets(&self) -> usize {
        self.0.borrow().sets
    }

    /// The id of the entry reached by `path`, its labels from a top-level menu down, as the user
    /// finds it.
    pub fn id_of(&self, path: &[&str]) -> Option<MenuId> {
        id_at(&self.0.borrow().menus, path)
    }
}

/// The id of the entry of `entries` reached by `path`'s labels.
fn id_at(entries: &[MenuEntry], path: &[&str]) -> Option<MenuId> {
    let (first, rest) = path.split_first()?;
    entries.iter().find_map(|entry| match entry {
        MenuEntry::Submenu {
            id, label, entries, ..
        } if label == first => {
            if rest.is_empty() {
                Some(*id)
            } else {
                id_at(entries, rest)
            }
        }
        MenuEntry::Item { id, item } if item.label == *first && rest.is_empty() => Some(*id),
        _ => None,
    })
}

impl MenuBar for MockMenuBar {
    fn set_menus(&self, menus: &[MenuEntry]) {
        let mut state = self.0.borrow_mut();
        state.menus = menus.to_vec();
        state.sets += 1;
    }

    fn perform_key_equivalent(&self, key: &KeyData) -> bool {
        let mut state = self.0.borrow_mut();
        state.offered.push(key.clone());
        key.event_type != KeyEventType::Up
            && state
                .shortcuts
                .iter()
                .any(|shortcut| shortcut.key == key.logical)
    }
}

/// A Dock menu that keeps the menu last set. Every clone is the same menu.
#[derive(Clone, Default)]
pub struct MockDockMenu(Rc<RefCell<Vec<MenuEntry>>>);

impl MockDockMenu {
    pub fn new() -> MockDockMenu {
        MockDockMenu::default()
    }

    /// The menu last set.
    pub fn menu(&self) -> Vec<MenuEntry> {
        self.0.borrow().clone()
    }
}

impl DockMenu for MockDockMenu {
    fn set_menu(&self, menu: &[MenuEntry]) {
        *self.0.borrow_mut() = menu.to_vec();
    }
}

/// One menu the framework opened at a point in a view.
#[derive(Clone, Debug, PartialEq)]
pub struct ShownMenu {
    pub view: ViewId,
    pub at: Offset,
    pub menu: Vec<MenuEntry>,
}

/// Menus opened at a point, each answered at once with the choice the test gave, and kept. Every
/// clone is the same.
#[derive(Clone, Default)]
pub struct MockPopupMenus(Rc<PopupMenusState>);

#[derive(Default)]
struct PopupMenusState {
    shown: RefCell<Vec<ShownMenu>>,
    choice: Cell<Option<MenuId>>,
}

impl MockPopupMenus {
    pub fn new() -> MockPopupMenus {
        MockPopupMenus::default()
    }

    /// Answers every menu opened from now on with `choice`, as a user choosing the item or, for
    /// none, closing the menu.
    pub fn answering(self, choice: Option<MenuId>) -> MockPopupMenus {
        self.0.choice.set(choice);
        self
    }

    /// The menus opened, in order.
    pub fn shown(&self) -> Vec<ShownMenu> {
        self.0.shown.borrow().clone()
    }
}

impl PopupMenus for MockPopupMenus {
    fn show(&self, view: ViewId, at: Offset, menu: &[MenuEntry]) -> PopupMenuFuture {
        self.0.shown.borrow_mut().push(ShownMenu {
            view,
            at,
            menu: menu.to_vec(),
        });
        Box::pin(std::future::ready(self.0.choice.get()))
    }
}

/// Tray icons kept while they are there, each with what the framework set, its menus opened at it
/// answered with the choice the test gave. An icon is answered at once, and offers both kinds of
/// menu, as macOS's status items do, unless the test leaves one out, as Windows and Linux do, or
/// has the tray refuse icons. Every clone is the same tray.
#[derive(Clone)]
pub struct MockTrayIcons(Rc<TrayState>);

struct TrayState {
    next_id: Cell<TrayIconId>,
    icons: RefCell<Vec<Rc<IconState>>>,
    choice: Cell<Option<MenuId>>,
    attached_menus: Cell<bool>,
    menus_at_icon: Cell<bool>,
    /// Why the tray refuses icons, if it does.
    refusal: RefCell<Option<String>>,
}

impl Default for MockTrayIcons {
    fn default() -> MockTrayIcons {
        MockTrayIcons(Rc::new(TrayState {
            next_id: Cell::default(),
            icons: RefCell::default(),
            choice: Cell::default(),
            attached_menus: Cell::new(true),
            menus_at_icon: Cell::new(true),
            refusal: RefCell::default(),
        }))
    }
}

/// What a tray icon shows.
struct IconState {
    id: TrayIconId,
    image: RefCell<IconImage>,
    tooltip: RefCell<Option<String>>,
    attached: RefCell<Vec<MenuEntry>>,
    shown: RefCell<Vec<Vec<MenuEntry>>>,
}

/// A tray icon as the test reads it back.
#[derive(Clone, Debug, PartialEq)]
pub struct ShownTrayIcon {
    pub id: TrayIconId,
    pub image: IconImage,
    pub tooltip: Option<String>,
    /// The menu the system would open by itself.
    pub attached_menu: Vec<MenuEntry>,
    /// The menus the app opened at it, in order.
    pub menus_opened: Vec<Vec<MenuEntry>>,
}

impl MockTrayIcons {
    pub fn new() -> MockTrayIcons {
        MockTrayIcons::default()
    }

    /// Answers every menu opened at an icon from now on with `choice`.
    pub fn answering(self, choice: Option<MenuId>) -> MockTrayIcons {
        self.0.choice.set(choice);
        self
    }

    /// Icons with no menu the system opens by itself, as Windows' notification-area icons.
    pub fn without_attached_menus(self) -> MockTrayIcons {
        self.0.attached_menus.set(false);
        self
    }

    /// Icons the app cannot open a menu at, as Linux's StatusNotifierItems.
    pub fn without_menus_at_icon(self) -> MockTrayIcons {
        self.0.menus_at_icon.set(false);
        self
    }

    /// A tray that shows no icon, answering each with `reason`.
    pub fn refusing(self, reason: impl Into<String>) -> MockTrayIcons {
        *self.0.refusal.borrow_mut() = Some(reason.into());
        self
    }

    /// The icons there now, in the order added.
    pub fn icons(&self) -> Vec<ShownTrayIcon> {
        self.0
            .icons
            .borrow()
            .iter()
            .map(|icon| ShownTrayIcon {
                id: icon.id,
                image: icon.image.borrow().clone(),
                tooltip: icon.tooltip.borrow().clone(),
                attached_menu: icon.attached.borrow().clone(),
                menus_opened: icon.shown.borrow().clone(),
            })
            .collect()
    }
}

impl TrayIcons for MockTrayIcons {
    fn add(&self, image: &IconImage, tooltip: Option<&str>) -> TrayIconFuture {
        if let Some(reason) = self.0.refusal.borrow().clone() {
            return Box::pin(std::future::ready(Err(TrayIconError::Failed(reason))));
        }
        let id = self.0.next_id.get() + 1;
        self.0.next_id.set(id);
        let icon = Rc::new(IconState {
            id,
            image: RefCell::new(image.clone()),
            tooltip: RefCell::new(tooltip.map(str::to_owned)),
            attached: RefCell::default(),
            shown: RefCell::default(),
        });
        self.0.icons.borrow_mut().push(Rc::clone(&icon));
        let icon: Box<dyn TrayIcon> = Box::new(MockTrayIcon {
            tray: Rc::clone(&self.0),
            icon,
        });
        Box::pin(std::future::ready(Ok(icon)))
    }
}

/// An icon of a [`MockTrayIcons`], gone from it when dropped.
struct MockTrayIcon {
    tray: Rc<TrayState>,
    icon: Rc<IconState>,
}

impl TrayIcon for MockTrayIcon {
    fn id(&self) -> TrayIconId {
        self.icon.id
    }

    fn set_image(&self, image: &IconImage) {
        *self.icon.image.borrow_mut() = image.clone();
    }

    fn set_tooltip(&self, tooltip: Option<&str>) {
        *self.icon.tooltip.borrow_mut() = tooltip.map(str::to_owned);
    }

    fn attached_menu(&self) -> Option<&dyn AttachedMenu> {
        self.tray.attached_menus.get().then_some(self)
    }

    fn menu_at_icon(&self) -> Option<&dyn MenuAtIcon> {
        self.tray.menus_at_icon.get().then_some(self)
    }
}

impl AttachedMenu for MockTrayIcon {
    fn set_menu(&self, menu: &[MenuEntry]) {
        *self.icon.attached.borrow_mut() = menu.to_vec();
    }
}

impl MenuAtIcon for MockTrayIcon {
    fn show(&self, menu: &[MenuEntry]) -> PopupMenuFuture {
        self.icon.shown.borrow_mut().push(menu.to_vec());
        Box::pin(std::future::ready(self.tray.choice.get()))
    }
}

impl Drop for MockTrayIcon {
    fn drop(&mut self) {
        self.tray
            .icons
            .borrow_mut()
            .retain(|icon| icon.id != self.icon.id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use inset_embedder::{MenuItem, ShortcutModifiers};

    use super::*;

    fn image() -> IconImage {
        IconImage {
            width: 1,
            height: 1,
            rgba: Arc::from([0u8, 0, 0, 255]),
            template: true,
        }
    }

    #[test]
    fn the_menu_bar_finds_an_item_by_its_labels() {
        let bar = MockMenuBar::new();
        bar.set_menus(&[MenuEntry::Submenu {
            id: 1,
            label: "File".to_owned(),
            enabled: true,
            entries: vec![MenuEntry::Item {
                id: 2,
                item: MenuItem::new("Open"),
            }],
        }]);
        assert_eq!(bar.id_of(&["File", "Open"]), Some(2));
        assert_eq!(bar.id_of(&["File"]), Some(1));
        assert_eq!(bar.id_of(&["Edit"]), None);
        assert_eq!(bar.sets(), 1);
    }

    #[test]
    fn the_menu_bar_keeps_the_keys_offered_and_takes_its_shortcuts_keys() {
        let open = MenuShortcut {
            key: 'o' as u64,
            modifiers: ShortcutModifiers {
                meta: true,
                ..ShortcutModifiers::default()
            },
        };
        let bar = MockMenuBar::new().taking([open]);
        let key = |logical: char, event_type| KeyData {
            event_type,
            logical: logical as u64,
            physical: 0x0007_0012,
            ..KeyData::default()
        };
        assert!(bar.perform_key_equivalent(&key('o', KeyEventType::Down)));
        assert!(!bar.perform_key_equivalent(&key('p', KeyEventType::Down)));
        assert!(!bar.perform_key_equivalent(&key('o', KeyEventType::Up)));
        assert_eq!(bar.offered().len(), 3);
    }

    #[test]
    fn an_icon_offers_the_menus_its_tray_was_given() {
        let windows = now(MockTrayIcons::new()
            .without_attached_menus()
            .add(&image(), None))
        .expect("an icon");
        assert!(windows.attached_menu().is_none() && windows.menu_at_icon().is_some());
        let linux = now(MockTrayIcons::new()
            .without_menus_at_icon()
            .add(&image(), None))
        .expect("an icon");
        assert!(linux.attached_menu().is_some() && linux.menu_at_icon().is_none());
        let refused = now(MockTrayIcons::new().refusing("no tray").add(&image(), None));
        assert_eq!(
            refused.err(),
            Some(TrayIconError::Failed("no tray".to_owned()))
        );
    }

    #[test]
    fn a_tray_icon_is_there_until_dropped() {
        let tray = MockTrayIcons::new().answering(Some(5));
        let icon = now(tray.add(&image(), Some("Status"))).expect("an icon");
        icon.attached_menu()
            .expect("an attached menu")
            .set_menu(&[MenuEntry::Separator]);
        let choice = icon.menu_at_icon().expect("a menu at the icon").show(&[]);
        assert_eq!(now(choice), Some(5), "the test's choice answers it");
        let shown = tray.icons();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].tooltip.as_deref(), Some("Status"));
        assert_eq!(shown[0].attached_menu, [MenuEntry::Separator]);
        drop(icon);
        assert!(tray.icons().is_empty());
    }

    /// `future`'s output: these answer in the call.
    fn now<T>(mut future: std::pin::Pin<Box<dyn std::future::Future<Output = T>>>) -> T {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("answers in the call"),
        }
    }
}
