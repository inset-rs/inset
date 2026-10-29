//! Menus the system draws, as values: the menu bar (macOS's), a context menu at a point in a view,
//! a tray icon's menus, and the Dock icon's menu. Not a dart:ui file: Flutter's macOS embedder
//! takes the menu bar over the `flutter/menu` channel as a list of maps, which `inset-widgets`'
//! `PlatformMenuBar` builds this value in place of.
//!
//! A menu is a value rebuilt from the app's state and handed to the host whole, each time it
//! changes, as Flutter's `Menu.setMenus` hands it: an item's check or enabled state follows the
//! app's state at each rebuild, and the host replaces the system's menu with the new value. One
//! value serves every place a menu shows. A choice comes back by the id the app gave the item:
//! the menu bar's through [`EmbedderClient::menu_selected`](crate::EmbedderClient::menu_selected),
//! the Dock menu's through
//! [`EmbedderClient::dock_menu_selected`](crate::EmbedderClient::dock_menu_selected), a tray
//! icon's attached menu's as [`TrayIconEvent::MenuSelected`], and a menu the app opens itself
//! through the future it is answered.
//!
//! Each place is the system's own: the platform answers `None` for a place its system does not
//! provide, and the app or the framework draws its own there; a host never stands in for a
//! system. A [`ProvidedItem`] is an item the system itself provides, as AppKit's standard menu
//! items are; an app's own menu bar replaces the host's standard one whole, so a provided item is
//! the default, never a lock.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::{KeyData, Offset, ViewId};

/// The id the app gives a menu's entry, which a choice comes back by.
pub type MenuId = u32;

/// One entry of a menu.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuEntry {
    /// A menu of its own, opened from this entry: a menu bar's top-level menus are these.
    Submenu {
        id: MenuId,
        label: String,
        enabled: bool,
        entries: Vec<MenuEntry>,
    },
    /// An item of the app's.
    Item { id: MenuId, item: MenuItem },
    /// An item the system provides, which acts as the system's own does.
    Provided {
        id: MenuId,
        item: ProvidedItem,
        enabled: bool,
    },
    /// A line between groups of entries.
    Separator,
}

/// An item of the app's: what it shows, and whether it can be chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub label: String,
    pub enabled: bool,
    pub check: MenuItemCheck,
    /// The keys that choose it, which the menu shows beside its label.
    pub shortcut: Option<MenuShortcut>,
    pub icon: Option<IconImage>,
}

impl MenuItem {
    /// An item labelled `label` that can be chosen, with no check, shortcut or icon.
    pub fn new(label: impl Into<String>) -> MenuItem {
        MenuItem {
            label: label.into(),
            enabled: true,
            check: MenuItemCheck::Plain,
            shortcut: None,
            icon: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> MenuItem {
        self.enabled = enabled;
        self
    }

    pub fn check(mut self, check: MenuItemCheck) -> MenuItem {
        self.check = check;
        self
    }

    pub fn shortcut(mut self, shortcut: MenuShortcut) -> MenuItem {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn icon(mut self, icon: IconImage) -> MenuItem {
        self.icon = Some(icon);
        self
    }
}

/// Whether an item shows a check, and which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItemCheck {
    /// No check: an action.
    Plain,
    /// A setting the item toggles.
    Checkbox(CheckState),
    /// One of a group of settings, of which one is on: `true` for the one.
    Radio(bool),
}

/// A checkbox's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Off,
    On,
    /// Some of what it stands for is on, some off.
    Mixed,
}

/// The keys that choose an item: a key with modifiers held, as Flutter's `ShortcutSerialization`
/// carries a trigger and its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuShortcut {
    /// The key, by Flutter's `LogicalKeyboardKey.keyId`: a character's Unicode value for a key that
    /// types one.
    pub key: u64,
    pub modifiers: ShortcutModifiers,
}

/// The modifier keys a shortcut holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutModifiers {
    /// Command on Apple's systems, the Windows key elsewhere.
    pub meta: bool,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

/// An image a menu item, a tray icon or a window shows: RGBA pixels, row by row.
#[derive(Clone, PartialEq)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    /// Four bytes a pixel, not premultiplied.
    pub rgba: Arc<[u8]>,
    /// Drawn by its alpha alone, in the colour the system gives the place, as macOS draws a
    /// template image in its menu bar; a system without templates shows the colours.
    pub template: bool,
}

impl fmt::Debug for IconImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IconImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("template", &self.template)
            .finish_non_exhaustive()
    }
}

/// An item the system itself provides, which acts as the system's own does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProvidedItem {
    // The app's.
    About,
    /// The system's services, as a submenu.
    Services,
    Hide,
    HideOthers,
    ShowAll,
    /// Quits the app, through the same request the system's own quit makes.
    Quit,
    // The focused window's.
    Minimize,
    Zoom,
    ToggleFullScreen,
    BringAllToFront,
    // The focused view's edit commands, sent to it as events: `clipboard_event` for cut, copy and
    // paste, `edit_command` for the others.
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Delete,
}

/// An edit command of a provided item, sent to the focused view: Flutter's text editing intents
/// for undo, redo, select all and delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditCommand {
    Undo,
    Redo,
    SelectAll,
    /// Deletes what is selected, as the delete key does.
    Delete,
}

/// The app's menu bar: macOS's. Where the system has none, the platform answers `None` and the app
/// draws its own.
pub trait MenuBar {
    /// Replaces the app's menu bar whole: its top-level menus, each a [`MenuEntry::Submenu`]. A
    /// choice reaches the app as `EmbedderClient::menu_selected(id)`.
    fn set_menus(&self, menus: &[MenuEntry]);

    /// Offers the menu bar a key-down the app kept back when it came and has since left
    /// unhandled, as Chrome sends on a key its page did not consume; answers whether an item took
    /// it, which then reaches the app as `EmbedderClient::menu_selected`. For an app that answers
    /// a key later than the host must, such as one that waits on another thread: the host's own
    /// order, the framework first and the menu bar after, needs no call.
    ///
    /// `key` is the key as the host reported it (`EmbedderClient::key_data`), which the
    /// framework's key event gives back. The host finds the system's own event by it, which
    /// carries the keyboard layout and the modifiers, and keeps it only a short while: once that
    /// event is gone, or the key was offered already, the answer is `false`.
    fn perform_key_equivalent(&self, key: &KeyData) -> bool;
}

/// A menu the system opens at a point in a view, answered with the item chosen.
pub trait PopupMenus {
    /// Shows `menu` at `at`, in logical pixels from the top-left corner of `view`; answers the id
    /// of the item chosen once it closes, `None` when it closed with none chosen, or the view is
    /// gone.
    ///
    /// The host opens the menu after this call returns and runs the menu's own loop outside every
    /// call into the app, so the app's frames, timers and events go on while it is open, as they
    /// do in Flutter, whose platform calls answer later.
    fn show(&self, view: ViewId, at: Offset, menu: &[MenuEntry]) -> PopupMenuFuture;
}

/// A menu on screen: the id of the item chosen, once it closes. Flutter's `Future<int?>` from a
/// platform call.
pub type PopupMenuFuture = Pin<Box<dyn Future<Output = Option<MenuId>>>>;

/// The menu of the app's Dock icon: macOS's.
pub trait DockMenu {
    /// Replaces the menu whole; the system shows it above its own items. A choice reaches the app
    /// as `EmbedderClient::dock_menu_selected(id)`.
    fn set_menu(&self, menu: &[MenuEntry]);
}

/// The app's icons in the system's tray: macOS's status items, Windows' notification area,
/// Linux's StatusNotifierItems.
pub trait TrayIcons {
    /// Adds an icon showing `image`, answered once the icon exists, as the system made it: the
    /// icon is then what the app asks which menus it offers, and it is removed when dropped. A
    /// host that makes the icon at once answers at once; one that makes it elsewhere, on another
    /// thread or in another process, answers when it is there. What happens to the icon reaches
    /// the app as `EmbedderClient::tray_icon_event`, by the icon's id.
    fn add(&self, image: &IconImage, tooltip: Option<&str>) -> TrayIconFuture;
}

/// The icon a [`TrayIcons::add`] made, or why the system did not show it.
pub type TrayIconFuture = Pin<Box<dyn Future<Output = Result<Box<dyn TrayIcon>, TrayIconError>>>>;

/// Why a tray icon did not appear.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayIconError {
    /// The system did not show it: its tray refused the icon, or the host could not reach the
    /// tray, as when a Linux desktop's bus refuses the item.
    Failed(String),
}

impl fmt::Display for TrayIconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrayIconError::Failed(reason) => write!(f, "the tray icon did not appear: {reason}"),
        }
    }
}

impl std::error::Error for TrayIconError {}

/// The id of a tray icon, which its events come by.
pub type TrayIconId = u64;

/// An icon in the system's tray, removed when dropped. What it offers is what its system allows;
/// its conventions, such as a menu opened on a right click, are the app's or the framework's.
pub trait TrayIcon {
    fn id(&self) -> TrayIconId;

    fn set_image(&self, image: &IconImage);

    fn set_tooltip(&self, tooltip: Option<&str>);

    /// A menu the system opens by itself, when it chooses: macOS on a click, Linux when the
    /// desktop decides. `None` where the system opens none, as on Windows.
    fn attached_menu(&self) -> Option<&dyn AttachedMenu>;

    /// A menu the app opens at the icon, now: macOS and Windows. `None` where the system lets an
    /// app open none, as on Linux.
    fn menu_at_icon(&self) -> Option<&dyn MenuAtIcon>;
}

/// A tray icon's menu the system opens by itself.
pub trait AttachedMenu {
    /// Replaces the menu whole; `[]` takes it away. A choice reaches the app as
    /// [`TrayIconEvent::MenuSelected`].
    fn set_menu(&self, menu: &[MenuEntry]);
}

/// A tray icon's menu the app opens.
pub trait MenuAtIcon {
    /// Shows `menu` at the icon; answers the id of the item chosen once it closes.
    fn show(&self, menu: &[MenuEntry]) -> PopupMenuFuture;
}

/// What happened to a tray icon, as its system reports it: each system reports some of these.
/// Positions are in the coordinates of [`HostWindow::frame`](crate::HostWindow::frame) on macOS
/// and Windows; a Linux desktop gives its own, which on Wayland are often zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrayIconEvent {
    /// The icon was clicked, or chosen from the keyboard.
    Activated {
        position: Offset,
    },
    /// The user asked for the icon's menu: Windows' right click, or its keyboard's menu key.
    ContextMenuRequested {
        position: Offset,
    },
    /// The icon was clicked with the middle button, where the desktop says so.
    SecondaryActivated {
        position: Offset,
    },
    DoubleClicked,
    /// The wheel turned over the icon, by its steps.
    Scrolled {
        delta: f64,
    },
    /// An item of the icon's attached menu was chosen.
    MenuSelected(MenuId),
}
