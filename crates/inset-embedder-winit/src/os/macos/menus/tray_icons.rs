//! The status items: AppKit's `NSStatusItem`s in the menu bar, each the app's tray icon, removed
//! from the status bar when its handle is dropped. Its button reports what the user did as AppKit
//! sends it: a click as `Activated`, a right click or a Control-click as `ContextMenuRequested`, a
//! second click of a double click as `DoubleClicked`, at where the pointer was. A menu set as its
//! attached menu AppKit opens by itself at a click, which then reports nothing; a menu at the icon
//! the app opens under the button.

use std::cell::Cell;
use std::future;

use inset_embedder::{
    AttachedMenu, IconImage, MenuAtIcon, MenuEntry, PopupMenuFuture, TrayIcon, TrayIconEvent,
    TrayIconFuture, TrayIconId, TrayIcons,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSEventMask, NSEventModifierFlags, NSEventType, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength, NSView,
};
use objc2_foundation::{NSObject, NSPoint, NSString};

use super::super::{abort_on_panic, screens};
use super::images::{STATUS_ITEM_HEIGHT, ns_image};
use super::popup_menu::open_in;
use super::{MenuTarget, Place, native_menu};
use crate::menus::{MenuEvent, MenuEvents};
use crate::popup_menus::menu_answer;

/// The app's status items.
pub(super) struct StatusItems {
    main_thread: MainThreadMarker,
    events: MenuEvents,
    last_id: Cell<TrayIconId>,
}

impl StatusItems {
    pub(super) fn new(main_thread: MainThreadMarker, events: MenuEvents) -> StatusItems {
        StatusItems {
            main_thread,
            events,
            last_id: Cell::new(0),
        }
    }
}

impl TrayIcons for StatusItems {
    /// A status item exists once AppKit answers it: the icon is answered at once.
    fn add(&self, image: &IconImage, tooltip: Option<&str>) -> TrayIconFuture {
        let id = self.last_id.get() + 1;
        self.last_id.set(id);
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let clicks = ClickTarget::new(self.main_thread, id, self.events.clone());
        if let Some(button) = item.button(self.main_thread) {
            // SAFETY: the target, which the button holds weakly, is a `ClickTarget`, the type that
            // answers `clicked:`; the status item keeps it.
            unsafe {
                button.setTarget(Some(&clicks));
                button.setAction(Some(sel!(clicked:)));
            }
            button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        }
        let chosen = self.events.clone();
        let menu_target = MenuTarget::new(
            self.main_thread,
            self.events.clone(),
            Box::new(move |chosen_id| {
                chosen.send(MenuEvent::Tray(id, TrayIconEvent::MenuSelected(chosen_id)))
            }),
        );
        let icon = StatusItem {
            id,
            main_thread: self.main_thread,
            item,
            _clicks: clicks,
            menu_target,
            events: self.events.clone(),
        };
        icon.set_image(image);
        icon.set_tooltip(tooltip);
        Box::pin(future::ready(Ok(Box::new(icon) as Box<dyn TrayIcon>)))
    }
}

/// One status item, removed from the status bar when dropped.
struct StatusItem {
    id: TrayIconId,
    main_thread: MainThreadMarker,
    item: Retained<NSStatusItem>,
    /// What the button's clicks go to, kept as long as the item.
    _clicks: Retained<ClickTarget>,
    /// What the attached menu's items go to.
    menu_target: Retained<MenuTarget>,
    events: MenuEvents,
}

impl TrayIcon for StatusItem {
    fn id(&self) -> TrayIconId {
        self.id
    }

    fn set_image(&self, image: &IconImage) {
        if let Some(button) = self.item.button(self.main_thread) {
            button.setImage(ns_image(image, STATUS_ITEM_HEIGHT).as_deref());
        }
    }

    fn set_tooltip(&self, tooltip: Option<&str>) {
        if let Some(button) = self.item.button(self.main_thread) {
            button.setToolTip(tooltip.map(NSString::from_str).as_deref());
        }
    }

    /// The item's own menu, which AppKit opens on a click.
    fn attached_menu(&self) -> Option<&dyn AttachedMenu> {
        Some(self)
    }

    fn menu_at_icon(&self) -> Option<&dyn MenuAtIcon> {
        Some(self)
    }
}

impl AttachedMenu for StatusItem {
    fn set_menu(&self, menu: &[MenuEntry]) {
        let built = (!menu.is_empty()).then(|| {
            native_menu(
                self.main_thread,
                "",
                menu,
                &self.menu_target,
                Place::Elsewhere,
            )
        });
        self.item.setMenu(built.as_deref());
    }
}

impl MenuAtIcon for StatusItem {
    /// Under the button, as AppKit opens a status item's own menu.
    fn show(&self, menu: &[MenuEntry]) -> PopupMenuFuture {
        let (answer, chosen) = menu_answer();
        let Some(button) = self.item.button(self.main_thread) else {
            answer.give(None);
            return Box::pin(chosen);
        };
        let view: &NSView = &button;
        let view = view.retain();
        let bounds = view.bounds();
        let below = if view.isFlipped() {
            bounds.size.height
        } else {
            0.0
        };
        let events = self.events.clone();
        open_in(
            self.main_thread,
            view,
            NSPoint::new(0.0, below),
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

impl Drop for StatusItem {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}

/// A status item's button's clicks, and where they go.
struct Clicks {
    id: TrayIconId,
    events: MenuEvents,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `ClickTarget` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetStatusItemClickTarget"]
    #[ivars = Clicks]
    struct ClickTarget;

    impl ClickTarget {
        #[unsafe(method(clicked:))]
        fn clicked(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| {
                let clicks = self.ivars();
                if let Some(event) = self.click() {
                    clicks.events.send(MenuEvent::Tray(clicks.id, event));
                }
            });
        }
    }
);

impl ClickTarget {
    fn new(main_thread: MainThreadMarker, id: TrayIconId, events: MenuEvents) -> Retained<Self> {
        let this = main_thread
            .alloc::<ClickTarget>()
            .set_ivars(Clicks { id, events });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    /// The click AppKit is delivering, as the app hears it.
    fn click(&self) -> Option<TrayIconEvent> {
        let event = NSApplication::sharedApplication(self.mtm()).currentEvent()?;
        let position = screens::pointer_location()?;
        let secondary = event.r#type() == NSEventType::RightMouseUp
            || event
                .modifierFlags()
                .contains(NSEventModifierFlags::Control);
        Some(if secondary {
            TrayIconEvent::ContextMenuRequested { position }
        } else if event.clickCount() == 2 {
            TrayIconEvent::DoubleClicked
        } else {
            TrayIconEvent::Activated { position }
        })
    }
}
