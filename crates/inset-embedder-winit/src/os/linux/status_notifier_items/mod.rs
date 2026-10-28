//! The app's icons in the desktop's tray: StatusNotifierItems (`item`), each with its menu as a
//! DBus menu the desktop draws (`dbus_menu`), as KDE, the AppIndicator extension of GNOME and most
//! panels show them. Each icon is a connection of its own to the session bus, under a name of its
//! own, registered with the StatusNotifierWatcher; an icon the bus or the watcher refuses is
//! answered as an error. Dropping its handle releases the name and closes the connection, and the
//! watcher forgets it. The tray is `None` where no watcher is on the bus when the app starts: the
//! desktop shows no tray.
//!
//! What the desktop asks of an item and its menu arrives on zbus's thread, and reaches the main
//! loop as a [`MenuEvent`](crate::menus::MenuEvent); what the app changes is told to the desktop
//! from the main thread, by a signal. Every call to the bus gives up after [`PATIENCE`], so a bus
//! that does not answer cannot hold the main thread for long.

mod dbus_menu;
mod item;

use std::cell::Cell;
use std::future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use inset_embedder::{
    AttachedMenu, IconImage, MenuAtIcon, MenuEntry, TrayIcon, TrayIconError, TrayIconFuture,
    TrayIconId, TrayIcons,
};
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::blocking::fdo::DBusProxy;
use zbus::export::serde::Serialize;
use zbus::names::BusName;
use zbus::zvariant::DynamicType;

use self::dbus_menu::{DbusMenu, MenuState};
use self::item::{Appearance, Item};
use crate::menus::MenuEvents;

/// The longest a call to the session bus may take.
const PATIENCE: Duration = Duration::from_secs(1);
const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM_PATH: &str = "/StatusNotifierItem";
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const MENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// The app's icons in the tray.
pub(super) struct StatusNotifierItems {
    events: MenuEvents,
    last_id: Cell<TrayIconId>,
}

impl StatusNotifierItems {
    /// The tray, if a StatusNotifierWatcher is on the session bus.
    pub(super) fn new(events: MenuEvents) -> Option<StatusNotifierItems> {
        watcher_is_on_the_bus().then(|| StatusNotifierItems {
            events,
            last_id: Cell::new(0),
        })
    }
}

fn watcher_is_on_the_bus() -> bool {
    let Ok(connection) = Builder::session().map(|builder| builder.method_timeout(PATIENCE)) else {
        return false;
    };
    let Ok(connection) = connection.build() else {
        return false;
    };
    let watcher = BusName::try_from(WATCHER).expect("a valid bus name");
    DBusProxy::new(&connection)
        .ok()
        .and_then(|bus| bus.name_has_owner(watcher).ok())
        .unwrap_or(false)
}

impl TrayIcons for StatusNotifierItems {
    /// An item exists once the watcher has registered it, which the host waits for, a call at a
    /// time, each for at most [`PATIENCE`]: the icon, or why it is not there, is answered at once.
    fn add(&self, image: &IconImage, tooltip: Option<&str>) -> TrayIconFuture {
        let id = self.last_id.get() + 1;
        self.last_id.set(id);
        let appearance = Arc::new(Mutex::new(Appearance::new(image, tooltip)));
        let menu = Arc::new(Mutex::new(MenuState::default()));
        let name = format!("org.kde.StatusNotifierItem-{}-{id}", std::process::id());
        let registered = register(
            &name,
            Item::new(id, Arc::clone(&appearance), self.events.clone()),
            DbusMenu::new(id, Arc::clone(&menu), self.events.clone()),
        );
        let added = match registered {
            Ok(connection) => Ok(Box::new(StatusNotifierItem {
                id,
                name,
                connection: Some(connection),
                appearance,
                menu,
            }) as Box<dyn TrayIcon>),
            Err(error) => Err(TrayIconError::Failed(error.to_string())),
        };
        Box::pin(future::ready(added))
    }
}

/// Puts the item and its menu on the bus under `name`, and registers it with the watcher.
fn register(name: &str, item: Item, menu: DbusMenu) -> zbus::Result<Connection> {
    let connection = Builder::session()?
        .method_timeout(PATIENCE)
        .name(name)?
        .serve_at(ITEM_PATH, item)?
        .serve_at(MENU_PATH, menu)?
        .build()?;
    connection.call_method(
        Some(WATCHER),
        WATCHER_PATH,
        Some(WATCHER),
        "RegisterStatusNotifierItem",
        &(name,),
    )?;
    Ok(connection)
}

/// One icon, which leaves the bus when dropped.
struct StatusNotifierItem {
    id: TrayIconId,
    name: String,
    /// The item's own connection, until it is dropped.
    connection: Option<Connection>,
    appearance: Arc<Mutex<Appearance>>,
    menu: Arc<Mutex<MenuState>>,
}

impl StatusNotifierItem {
    /// Tells the desktop `signal` of `interface`, at `path`, with `body`.
    fn signal<B>(&self, path: &str, interface: &str, signal: &str, body: &B)
    where
        B: Serialize + DynamicType,
    {
        if let Some(connection) = &self.connection {
            let _ = connection.emit_signal(None::<BusName<'_>>, path, interface, signal, body);
        }
    }
}

impl TrayIcon for StatusNotifierItem {
    fn id(&self) -> TrayIconId {
        self.id
    }

    fn set_image(&self, image: &IconImage) {
        locked(&self.appearance).set_image(image);
        self.signal(ITEM_PATH, ITEM_INTERFACE, "NewIcon", &());
    }

    fn set_tooltip(&self, tooltip: Option<&str>) {
        locked(&self.appearance).set_tooltip(tooltip);
        self.signal(ITEM_PATH, ITEM_INTERFACE, "NewToolTip", &());
    }

    /// The menu the desktop opens when it chooses.
    fn attached_menu(&self) -> Option<&dyn AttachedMenu> {
        Some(self)
    }

    /// A StatusNotifierItem cannot open its menu itself.
    fn menu_at_icon(&self) -> Option<&dyn MenuAtIcon> {
        None
    }
}

impl AttachedMenu for StatusNotifierItem {
    fn set_menu(&self, menu: &[MenuEntry]) {
        let revision = locked(&self.menu).replace(menu);
        self.signal(
            MENU_PATH,
            MENU_INTERFACE,
            "LayoutUpdated",
            &(revision, 0i32),
        );
    }
}

impl Drop for StatusNotifierItem {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let _ = connection.release_name(self.name.as_str());
            let _ = connection.close();
        }
    }
}

/// What the main thread and zbus's share, which neither holds across a call that may panic.
fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
