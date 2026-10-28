//! The `org.kde.StatusNotifierItem` interface of one icon: what the desktop reads of it, its image
//! as a pixmap and its tooltip, which the app changes (`Appearance`), and what the desktop tells it
//! the user did, which reaches the app as the item reports it: `Activate` as `Activated`,
//! `SecondaryActivate` as `SecondaryActivated`, `ContextMenu` as `ContextMenuRequested`, and a
//! vertical `Scroll` as `Scrolled`. The positions are the desktop's, in its screen's pixels, and
//! on Wayland often nothing but zero.

use std::sync::{Arc, Mutex};

use inset_embedder::{IconImage, Offset, TrayIconEvent, TrayIconId};
use zbus::interface;
use zbus::zvariant::OwnedObjectPath;

use super::{MENU_PATH, locked};
use crate::menus::pixels::argb;
use crate::menus::{MenuEvent, MenuEvents};

/// A pixmap as the item gives it: width, height, and ARGB pixels in network byte order.
type Pixmap = (i32, i32, Vec<u8>);

/// What the item shows, which the app changes and the desktop reads.
pub(super) struct Appearance {
    pixmap: Option<Pixmap>,
    tooltip: Option<String>,
}

impl Appearance {
    pub(super) fn new(image: &IconImage, tooltip: Option<&str>) -> Appearance {
        let mut appearance = Appearance {
            pixmap: None,
            tooltip: None,
        };
        appearance.set_image(image);
        appearance.set_tooltip(tooltip);
        appearance
    }

    pub(super) fn set_image(&mut self, image: &IconImage) {
        self.pixmap = Some((
            i32::try_from(image.width).unwrap_or(0),
            i32::try_from(image.height).unwrap_or(0),
            argb(image),
        ));
    }

    pub(super) fn set_tooltip(&mut self, tooltip: Option<&str>) {
        self.tooltip = tooltip.map(str::to_owned);
    }
}

/// One icon as the desktop reaches it.
pub(super) struct Item {
    id: TrayIconId,
    /// The app's name, which the desktop may show and remembers the item by.
    title: String,
    appearance: Arc<Mutex<Appearance>>,
    events: MenuEvents,
}

impl Item {
    pub(super) fn new(
        id: TrayIconId,
        appearance: Arc<Mutex<Appearance>>,
        events: MenuEvents,
    ) -> Item {
        Item {
            id,
            title: application_name(),
            appearance,
            events,
        }
    }

    fn report(&self, event: TrayIconEvent) {
        self.events.send(MenuEvent::Tray(self.id, event));
    }
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }

    #[zbus(property)]
    fn id(&self) -> &str {
        &self.title
    }

    #[zbus(property)]
    fn title(&self) -> &str {
        &self.title
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }

    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<Pixmap> {
        locked(&self.appearance).pixmap.iter().cloned().collect()
    }

    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Vec<Pixmap> {
        Vec::new()
    }

    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> Vec<Pixmap> {
        Vec::new()
    }

    #[zbus(property)]
    fn attention_movie_name(&self) -> &str {
        ""
    }

    /// No icon of its own, the tooltip as its title, and no description.
    #[zbus(property)]
    fn tool_tip(&self) -> (String, Vec<Pixmap>, String, String) {
        let tooltip = locked(&self.appearance).tooltip.clone().unwrap_or_default();
        (String::new(), Vec::new(), tooltip, String::new())
    }

    /// The item answers clicks itself, and its menu is the DBus menu's.
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(MENU_PATH).expect("a valid object path")
    }

    fn activate(&self, x: i32, y: i32) {
        self.report(TrayIconEvent::Activated {
            position: position(x, y),
        });
    }

    fn secondary_activate(&self, x: i32, y: i32) {
        self.report(TrayIconEvent::SecondaryActivated {
            position: position(x, y),
        });
    }

    fn context_menu(&self, x: i32, y: i32) {
        self.report(TrayIconEvent::ContextMenuRequested {
            position: position(x, y),
        });
    }

    /// A vertical turn of the wheel; the app hears no horizontal one.
    fn scroll(&self, delta: i32, orientation: &str) {
        if orientation.eq_ignore_ascii_case("vertical") {
            self.report(TrayIconEvent::Scrolled {
                delta: f64::from(delta),
            });
        }
    }
}

fn position(x: i32, y: i32) -> Offset {
    Offset::new(f64::from(x), f64::from(y))
}

/// The name of the app's executable, which an item has where it has no other.
fn application_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}
