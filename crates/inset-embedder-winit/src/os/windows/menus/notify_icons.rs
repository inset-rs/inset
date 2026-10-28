//! The app's icons in the notification area: `Shell_NotifyIcon`'s, at version 4, each telling the
//! hidden window what the user did to it (`menu_window`), which reports it to the app as the
//! system says it: a click or a key as `Activated`, a right click or the menu key as
//! `ContextMenuRequested`, a middle click as `SecondaryActivated`, a double click, each at the
//! point Windows gives, in the screens' logical pixels. An icon has no menu Windows opens by
//! itself; the app opens one at the pointer (`popup_menu`).
//!
//! An icon is removed when its handle drops, and added again when the taskbar comes back after
//! Explorer restarted, which forgets every icon. Version 4 carries an icon's id in 16 bits, so
//! the ids count from 1 to 65 535.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future;

use inset_embedder::{
    AttachedMenu, IconImage, MenuAtIcon, MenuEntry, Offset, PopupMenuFuture, TrayIcon,
    TrayIconError, TrayIconEvent, TrayIconFuture, TrayIconId, TrayIcons,
};
use windows_sys::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NIN_SELECT, NINF_KEY, NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
    NOTIFYICONDATAW_0, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_MBUTTONUP};

use super::bitmaps::{Icon, icon};
use super::menu_window::{ICON_NOTIFICATION, menu_window};
use super::popup_menu::open_at_pointer;
use crate::menus::{MenuEvent, MenuEvents};
use crate::popup_menus::menu_answer;

/// A key's choice of an icon.
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;
/// The pixels a logical pixel is at a scale of 1.
const BASE_DPI: f64 = 96.0;

/// What the notification area shows of the app's icons. Main thread.
#[derive(Default)]
struct Shown {
    /// Where the icons' events go, once an icon was added.
    events: Option<MenuEvents>,
    icons: HashMap<u16, ShownIcon>,
}

struct ShownIcon {
    icon: Option<Icon>,
    tooltip: Option<String>,
}

thread_local! {
    static SHOWN: RefCell<Shown> = RefCell::default();
}

/// The app's icons in the notification area.
pub(super) struct NotifyIcons {
    events: MenuEvents,
    last_id: Cell<u16>,
}

impl NotifyIcons {
    pub(super) fn new(events: MenuEvents) -> NotifyIcons {
        NotifyIcons {
            events,
            last_id: Cell::new(0),
        }
    }
}

impl TrayIcons for NotifyIcons {
    /// An icon exists once the notification area takes it, which it answers at once.
    fn add(&self, image: &IconImage, tooltip: Option<&str>) -> TrayIconFuture {
        let id = self.last_id.get().wrapping_add(1).max(1);
        self.last_id.set(id);
        SHOWN.with_borrow_mut(|shown| {
            shown.events = Some(self.events.clone());
            shown.icons.insert(
                id,
                ShownIcon {
                    icon: icon(image),
                    tooltip: tooltip.map(str::to_owned),
                },
            );
        });
        let added = if add_to_notification_area(id) {
            Ok(Box::new(NotifyIcon {
                id,
                events: self.events.clone(),
            }) as Box<dyn TrayIcon>)
        } else {
            SHOWN.with_borrow_mut(|shown| shown.icons.remove(&id));
            Err(TrayIconError::Failed(
                "the notification area refused the icon".to_owned(),
            ))
        };
        Box::pin(future::ready(added))
    }
}

/// One icon, removed from the notification area when dropped.
struct NotifyIcon {
    id: u16,
    events: MenuEvents,
}

impl TrayIcon for NotifyIcon {
    fn id(&self) -> TrayIconId {
        TrayIconId::from(self.id)
    }

    fn set_image(&self, image: &IconImage) {
        change(self.id, |shown| shown.icon = icon(image));
    }

    fn set_tooltip(&self, tooltip: Option<&str>) {
        change(self.id, |shown| shown.tooltip = tooltip.map(str::to_owned));
    }

    /// Windows opens no menu at an icon by itself.
    fn attached_menu(&self) -> Option<&dyn AttachedMenu> {
        None
    }

    fn menu_at_icon(&self) -> Option<&dyn MenuAtIcon> {
        Some(self)
    }
}

impl MenuAtIcon for NotifyIcon {
    /// At the pointer, as Windows' own icons open theirs.
    fn show(&self, menu: &[MenuEntry]) -> PopupMenuFuture {
        let (answer, chosen) = menu_answer();
        let events = self.events.clone();
        open_at_pointer(
            menu.to_vec(),
            self.events.clone(),
            Box::new(move |chosen| {
                events.closed();
                answer.give(chosen);
            }),
        );
        Box::pin(chosen)
    }
}

impl Drop for NotifyIcon {
    fn drop(&mut self) {
        notify(NIM_DELETE, self.id);
        SHOWN.with_borrow_mut(|shown| shown.icons.remove(&self.id));
    }
}

/// Changes what the icon `id` shows, and tells the notification area.
fn change(id: u16, change: impl FnOnce(&mut ShownIcon)) {
    SHOWN.with_borrow_mut(|shown| shown.icons.get_mut(&id).map(change));
    notify(NIM_MODIFY, id);
}

/// Adds the icon `id`, and has it report at version 4; answers whether the notification area
/// took it, which it does not while the taskbar is not there.
fn add_to_notification_area(id: u16) -> bool {
    notify(NIM_ADD, id) && notify(NIM_SETVERSION, id)
}

/// Sends the notification area `message` about the icon `id`, as it is now; answers whether it
/// took the message.
fn notify(message: NOTIFY_ICON_MESSAGE, id: u16) -> bool {
    let Some(window) = menu_window() else {
        return false;
    };
    let data = SHOWN.with_borrow(|shown| {
        let shown_icon = shown.icons.get(&id);
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: u32::from(id),
            uFlags: NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: ICON_NOTIFICATION,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            ..NOTIFYICONDATAW::default()
        };
        if let Some(icon) = shown_icon.and_then(|shown| shown.icon.as_ref()) {
            data.uFlags |= NIF_ICON;
            data.hIcon = icon.handle();
        }
        let tooltip = shown_icon.and_then(|shown| shown.tooltip.as_deref());
        copy_tooltip(tooltip.unwrap_or_default(), &mut data.szTip);
        data
    });
    // SAFETY: the data names the embedder's hidden window, and its icon is alive for the call,
    // which copies it.
    unsafe { Shell_NotifyIconW(message, &data) != 0 }
}

/// `tooltip` into the notification area's buffer, cut to fit with its terminating NUL.
fn copy_tooltip(tooltip: &str, buffer: &mut [u16]) {
    let fits = buffer.len() - 1;
    for (slot, unit) in buffer.iter_mut().zip(tooltip.encode_utf16().take(fits)) {
        *slot = unit;
    }
}

/// What the notification area sent about an icon, at version 4: the event and the icon's id in
/// `lparam`'s low and high words, and the point in `wparam`'s.
pub(super) fn notified(wparam: WPARAM, lparam: LPARAM) {
    let message = (lparam & 0xFFFF) as u32;
    let id = ((lparam >> 16) & 0xFFFF) as u16;
    let point = POINT {
        x: i32::from((wparam & 0xFFFF) as u16 as i16),
        y: i32::from(((wparam >> 16) & 0xFFFF) as u16 as i16),
    };
    let event = match message {
        NIN_SELECT | NIN_KEYSELECT => TrayIconEvent::Activated {
            position: logical(point),
        },
        WM_CONTEXTMENU => TrayIconEvent::ContextMenuRequested {
            position: logical(point),
        },
        WM_MBUTTONUP => TrayIconEvent::SecondaryActivated {
            position: logical(point),
        },
        WM_LBUTTONDBLCLK => TrayIconEvent::DoubleClicked,
        _ => return,
    };
    SHOWN.with_borrow(|shown| {
        if let Some(events) = &shown.events
            && shown.icons.contains_key(&id)
        {
            events.send(MenuEvent::Tray(TrayIconId::from(id), event));
        }
    });
}

/// The taskbar came back after Explorer restarted, without the app's icons: they are added again.
pub(super) fn taskbar_created() {
    let ids: Vec<u16> = SHOWN.with_borrow(|shown| shown.icons.keys().copied().collect());
    for id in ids {
        add_to_notification_area(id);
    }
}

/// `point`, in the screens' physical pixels, in the logical pixels of the monitor it is on, as
/// the app's screens measure theirs (`screens.rs`).
fn logical(point: POINT) -> Offset {
    // SAFETY: the call reads the point and answers a monitor, the nearest if none holds it.
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let (mut dpi_x, mut dpi_y) = (0, 0);
    // SAFETY: the monitor is one Windows answered; the numbers are the call's own.
    let scale = if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
        == 0
        && dpi_x > 0
    {
        f64::from(dpi_x) / BASE_DPI
    } else {
        1.0
    };
    Offset::new(f64::from(point.x) / scale, f64::from(point.y) / scale)
}
