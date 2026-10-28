//! A menu at a point: Win32's `TrackPopupMenuEx`, built from the framework's value each time it
//! opens, at a point in a window or, for a notification-area icon, at the pointer. A menu asked
//! for is posted to the hidden window, whose procedure opens it (`hidden_window`), outside every
//! call into the app, and the item chosen is handed to the one who asked once it closes.
//!
//! Every entry has its number (`menus/numbered.rs`), an item's command id, which the menu answers
//! for the item chosen. A provided item acts as the system's own: Edit's send the focused view its
//! command, and a window's minimize or maximize it; the rest have no counterpart on Windows and
//! are left out, as are the window's where the menu is at an icon. A shortcut is written after its
//! item's label, which is all a Windows menu does with it: the framework's shortcuts act on keys.

use std::ptr;

use inset_embedder::{
    CheckState, MenuEntry, MenuId, MenuItem, MenuItemCheck, Offset, ProvidedItem,
};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, InsertMenuItemW, IsZoomed, MENUITEMINFOW,
    MFS_CHECKED, MFS_GRAYED, MFT_RADIOCHECK, MFT_SEPARATOR, MFT_STRING, MIIM_BITMAP, MIIM_FTYPE,
    MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, PostMessageW, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, SetForegroundWindow, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenuEx, WM_NULL,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::super::hidden_window::{hidden_window, run_soon, wide};
use super::bitmaps::{Bitmap, menu_item_bitmap};
use crate::menus::numbered::{Choice, Numbered, choice, numbered};
use crate::menus::{EditAction, MenuEvent, MenuEvents, accelerator_text};

/// What the menu's close hands the item chosen to.
pub(super) type Closed = Box<dyn FnOnce(Option<MenuId>) + Send>;

/// A menu asked for, waiting to open.
struct AskedFor {
    place: Place,
    menu: Vec<MenuEntry>,
    events: MenuEvents,
    closed: Closed,
}

/// Where a menu opens.
#[derive(Clone, Copy)]
enum Place {
    /// At `at` on the screen, in physical pixels, owned by `window`, whose own items it may have.
    Window { window: HWND, at: POINT },
    /// At the pointer, owned by the hidden window, for an icon of the notification area.
    Pointer,
}

/// Opens `menu` at `at`, in logical pixels from the top-left corner of `window`'s content.
pub(super) fn open_in_window(
    window: &Window,
    at: Offset,
    menu: Vec<MenuEntry>,
    events: MenuEvents,
    closed: Closed,
) {
    let Some(handle) = hwnd(window) else {
        closed(None);
        return;
    };
    let scale = window.scale_factor();
    let mut point = POINT {
        x: (at.dx() * scale).round() as i32,
        y: (at.dy() * scale).round() as i32,
    };
    // SAFETY: the window is winit's, alive while the call lasts; the point is the call's own.
    unsafe { ClientToScreen(handle, &mut point) };
    ask_for(AskedFor {
        place: Place::Window {
            window: handle,
            at: point,
        },
        menu,
        events,
        closed,
    });
}

/// Opens `menu` at the pointer, for an icon of the notification area.
pub(super) fn open_at_pointer(menu: Vec<MenuEntry>, events: MenuEvents, closed: Closed) {
    ask_for(AskedFor {
        place: Place::Pointer,
        menu,
        events,
        closed,
    });
}

fn ask_for(asked_for: AskedFor) {
    run_soon(Box::new(move || open(asked_for)));
}

/// Opens the menu asked for, from the hidden window's procedure, and hands on the item chosen.
fn open(asked_for: AskedFor) {
    let chosen = track(&asked_for);
    let answer = match chosen {
        Some(Choice::App(id)) => Some(id),
        Some(Choice::Provided(item)) => {
            act(item, asked_for.place, &asked_for.events);
            None
        }
        None => None,
    };
    (asked_for.closed)(answer);
}

/// Shows the menu until it closes, and answers what the item chosen does.
fn track(asked_for: &AskedFor) -> Option<Choice> {
    let numbered = numbered(&asked_for.menu);
    let menu = NativeMenu::build(&numbered, asked_for.place)?;
    let (owner, at) = match asked_for.place {
        Place::Window { window, at } => (window, at),
        Place::Pointer => {
            let owner = hidden_window()?;
            let mut at = POINT::default();
            // SAFETY: the point is the call's own; the hidden window is the embedder's. Windows
            // asks that the owner of a notification-area icon's menu be the foreground window,
            // or the menu does not close when the user clicks elsewhere.
            unsafe {
                GetCursorPos(&mut at);
                SetForegroundWindow(owner);
            }
            (owner, at)
        }
    };
    // SAFETY: the menu is alive until `menu` drops, after the call; the owner is a live window
    // of this thread's.
    let number = unsafe {
        TrackPopupMenuEx(
            menu.handle,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            at.x,
            at.y,
            owner,
            ptr::null(),
        )
    };
    if let Place::Pointer = asked_for.place {
        // SAFETY: as Windows asks after a notification-area icon's menu, so the next one opens
        // at once.
        unsafe { PostMessageW(owner, WM_NULL, 0, 0) };
    }
    u32::try_from(number)
        .ok()
        .filter(|&number| number != 0)
        .and_then(|number| choice(&asked_for.menu, number))
}

/// A provided item chosen acts as the system's own.
fn act(item: ProvidedItem, place: Place, events: &MenuEvents) {
    if let Some(action) = EditAction::of(item) {
        events.send(MenuEvent::Edit(action));
        return;
    }
    let Place::Window { window, .. } = place else {
        return;
    };
    let command = match item {
        ProvidedItem::Minimize => SW_MINIMIZE,
        ProvidedItem::Zoom if is_maximized(window) => SW_RESTORE,
        ProvidedItem::Zoom => SW_MAXIMIZE,
        _ => return,
    };
    // SAFETY: the window is winit's, which the menu was opened in.
    unsafe { ShowWindow(window, command) };
}

fn is_maximized(window: HWND) -> bool {
    // SAFETY: the window is winit's, which the menu was opened in.
    unsafe { IsZoomed(window) != 0 }
}

/// A menu built from the framework's value, destroyed with its submenus when dropped, and then
/// the bitmaps its items showed.
struct NativeMenu {
    handle: HMENU,
    bitmaps: Vec<Bitmap>,
}

impl NativeMenu {
    fn build(numbered: &[Numbered<'_>], place: Place) -> Option<NativeMenu> {
        // SAFETY: a new, empty menu, which `NativeMenu` destroys.
        let handle = unsafe { CreatePopupMenu() };
        if handle.is_null() {
            return None;
        }
        let mut menu = NativeMenu {
            handle,
            bitmaps: Vec::new(),
        };
        menu.add_entries(handle, numbered, place);
        Some(menu)
    }

    fn add_entries(&mut self, into: HMENU, numbered: &[Numbered<'_>], place: Place) {
        let mut position = 0;
        for entry in numbered {
            if self.add_entry(into, position, entry, place) {
                position += 1;
            }
        }
    }

    /// Adds `entry` to `into` at `position`; answers whether it had an item to add.
    fn add_entry(
        &mut self,
        into: HMENU,
        position: u32,
        entry: &Numbered<'_>,
        place: Place,
    ) -> bool {
        match entry.entry {
            MenuEntry::Separator => {
                let info = MENUITEMINFOW {
                    fMask: MIIM_FTYPE,
                    fType: MFT_SEPARATOR,
                    ..item_info()
                };
                insert(into, position, &info);
            }
            MenuEntry::Submenu { label, enabled, .. } => {
                // SAFETY: a new, empty menu, which the item takes; destroying the menu it is in
                // destroys it too.
                let submenu = unsafe { CreatePopupMenu() };
                if submenu.is_null() {
                    return false;
                }
                self.add_entries(submenu, &entry.entries, place);
                let mut label = wide(&mnemonic_free(label));
                let info = MENUITEMINFOW {
                    fMask: MIIM_STRING | MIIM_SUBMENU | MIIM_STATE | MIIM_ID,
                    fState: if *enabled { 0 } else { MFS_GRAYED },
                    wID: entry.number,
                    hSubMenu: submenu,
                    dwTypeData: label.as_mut_ptr(),
                    ..item_info()
                };
                insert(into, position, &info);
            }
            MenuEntry::Item { item, .. } => self.add_app_item(into, position, entry.number, item),
            MenuEntry::Provided { item, enabled, .. } => {
                let Some(label) = provided_label(*item, place) else {
                    return false;
                };
                let mut label = wide(&label);
                let info = MENUITEMINFOW {
                    fMask: MIIM_STRING | MIIM_STATE | MIIM_ID,
                    fState: if *enabled { 0 } else { MFS_GRAYED },
                    wID: entry.number,
                    dwTypeData: label.as_mut_ptr(),
                    ..item_info()
                };
                insert(into, position, &info);
            }
        }
        true
    }

    fn add_app_item(&mut self, into: HMENU, position: u32, number: u32, item: &MenuItem) {
        let mut text = mnemonic_free(&item.label);
        if let Some(shortcut) = item.shortcut.as_ref().and_then(accelerator_text) {
            text.push('\t');
            text.push_str(&shortcut);
        }
        let mut label = wide(&text);
        let checked = matches!(
            item.check,
            MenuItemCheck::Checkbox(CheckState::On) | MenuItemCheck::Radio(true)
        );
        let mut info = MENUITEMINFOW {
            fMask: MIIM_FTYPE | MIIM_STRING | MIIM_STATE | MIIM_ID,
            fType: if matches!(item.check, MenuItemCheck::Radio(_)) {
                MFT_STRING | MFT_RADIOCHECK
            } else {
                MFT_STRING
            },
            fState: if item.enabled { 0 } else { MFS_GRAYED }
                | if checked { MFS_CHECKED } else { 0 },
            wID: number,
            dwTypeData: label.as_mut_ptr(),
            ..item_info()
        };
        if let Some(bitmap) = item.icon.as_ref().and_then(menu_item_bitmap) {
            info.fMask |= MIIM_BITMAP;
            info.hbmpItem = bitmap.handle();
            self.bitmaps.push(bitmap);
        }
        insert(into, position, &info);
    }
}

impl Drop for NativeMenu {
    fn drop(&mut self) {
        // SAFETY: the menu is the embedder's own, and no longer shown; its submenus go with it.
        unsafe { DestroyMenu(self.handle) };
    }
}

fn item_info() -> MENUITEMINFOW {
    MENUITEMINFOW {
        cbSize: size_of::<MENUITEMINFOW>() as u32,
        ..MENUITEMINFOW::default()
    }
}

fn insert(into: HMENU, position: u32, info: &MENUITEMINFOW) {
    // SAFETY: the menu is the embedder's own; the item's text, submenu and bitmap outlive the
    // call, which copies the text.
    unsafe { InsertMenuItemW(into, position, 1, info) };
}

/// A provided item's label where Windows has its counterpart; none where it has not.
fn provided_label(item: ProvidedItem, place: Place) -> Option<String> {
    let label = match item {
        ProvidedItem::Undo => "Undo\tCtrl+Z",
        ProvidedItem::Redo => "Redo\tCtrl+Y",
        ProvidedItem::Cut => "Cut\tCtrl+X",
        ProvidedItem::Copy => "Copy\tCtrl+C",
        ProvidedItem::Paste => "Paste\tCtrl+V",
        ProvidedItem::Delete => "Delete\tDel",
        ProvidedItem::SelectAll => "Select All\tCtrl+A",
        ProvidedItem::Minimize | ProvidedItem::Zoom => {
            let Place::Window { window, .. } = place else {
                return None;
            };
            match item {
                ProvidedItem::Minimize => "Minimize",
                _ if is_maximized(window) => "Restore",
                _ => "Maximize",
            }
        }
        ProvidedItem::About
        | ProvidedItem::Services
        | ProvidedItem::Hide
        | ProvidedItem::HideOthers
        | ProvidedItem::ShowAll
        | ProvidedItem::Quit
        | ProvidedItem::ToggleFullScreen
        | ProvidedItem::BringAllToFront => return None,
    };
    Some(label.to_owned())
}

/// `label` with its ampersands doubled: Windows reads one as the mark of the next letter's
/// mnemonic.
fn mnemonic_free(label: &str) -> String {
    label.replace('&', "&&")
}

/// The window's handle; `None` for a window winit gives none.
fn hwnd(window: &Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return None;
    };
    Some(win32.hwnd.get() as HWND)
}
