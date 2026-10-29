//! AppKit's menus as the framework's: the menu bar (`menu_bar`), which waits for the framework's
//! keys before it takes their key equivalents (`key_equivalents`), a menu at a point in a view
//! (`popup_menu`), the Dock icon's menu (`dock_menu`) and the status items (`tray_icons`), each an
//! `NSMenu` built from the framework's value here.
//!
//! An item of the app's sends `itemChosen:` to the menu's [`MenuTarget`], which reports its id
//! where the menu's place says. A provided item acts as AppKit's own does: the app's and the
//! window's go to AppKit's standard actions through the responder chain (`hide:`, `terminate:`,
//! `performMiniaturize:` and the rest), and Edit's go to the target, under their standard
//! selectors (`copy:`, `paste:` and the rest), so AppKit knows the Edit menu for what it is, and
//! the target sends each to the focused view as an event. Every action only queues for the main
//! loop (`menus/`), since AppKit sends it from its own dispatch.
//!
//! A menu's items are enabled as the value says, not by AppKit's validation of the responder
//! chain, so the app decides.

mod dock_menu;
mod images;
mod kept_keys;
mod key_equivalents;
mod menu_bar;
mod popup_menu;
mod tray_icons;

use std::rc::Rc;

use inset_embedder::{
    CheckState, DockMenu, KeyData, MenuBar, MenuEntry, MenuId, MenuItem, MenuItemCheck, Offset,
    ProvidedItem, TrayIcons,
};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSControlStateValueMixed, NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags,
    NSMenu, NSMenuItem,
};
use objc2_foundation::{NSObject, NSProcessInfo, NSString};
use winit::event_loop::EventLoopBuilder;
use winit::window::Window;

use super::{AppKit, abort_on_panic};
use crate::menus::{EditAction, MenuEvent, MenuEvents};
use crate::os::NativeMenus;

pub(super) use key_equivalents::app_window_opened;

/// The Dock icon's menu, as the application's delegate answers it: the one the app last set.
pub(super) fn dock_menu() -> Option<Retained<NSMenu>> {
    dock_menu::shown()
}

impl NativeMenus for AppKit {
    const POPUP_MENUS: bool = true;

    fn open_popup_menu(
        window: &dyn Window,
        at: Offset,
        menu: Vec<MenuEntry>,
        events: MenuEvents,
        closed: Box<dyn FnOnce(Option<MenuId>) + Send>,
    ) {
        popup_menu::open(window, at, menu, events, closed);
    }

    fn menu_bar(events: MenuEvents) -> Option<Rc<dyn MenuBar>> {
        let main_thread = MainThreadMarker::new()?;
        Some(Rc::new(menu_bar::AppMenuBar::new(main_thread, events)))
    }

    fn dock_menu(events: MenuEvents) -> Option<Rc<dyn DockMenu>> {
        let main_thread = MainThreadMarker::new()?;
        Some(Rc::new(dock_menu::AppDockMenu::new(main_thread, events)))
    }

    fn tray_icons(events: MenuEvents) -> Option<Rc<dyn TrayIcons>> {
        let main_thread = MainThreadMarker::new()?;
        Some(Rc::new(tray_icons::StatusItems::new(main_thread, events)))
    }

    fn configure_event_loop(builder: &mut EventLoopBuilder) {
        menu_bar::leave_winit_menu_out(builder);
    }

    fn provides_edit_commands() -> bool {
        menu_bar::provides_edit_commands()
    }

    fn perform_key_equivalent() -> bool {
        key_equivalents::perform_key_equivalent()
    }

    fn keep_key_event(key: &KeyData) {
        key_equivalents::keep_key_event(key);
    }
}

/// Where a menu's choices go: the app's item chosen, by its id, and Edit's commands.
pub(super) struct Actions {
    chosen: Box<dyn Fn(MenuId)>,
    events: MenuEvents,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `MenuTarget` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetMenuTarget"]
    #[ivars = Actions]
    pub(super) struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(itemChosen:))]
        fn item_chosen(&self, sender: &NSMenuItem) {
            abort_on_panic(|| (self.ivars().chosen)(sender.tag() as MenuId));
        }

        #[unsafe(method(undo:))]
        fn undo(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Undo));
        }

        #[unsafe(method(redo:))]
        fn redo(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Redo));
        }

        #[unsafe(method(cut:))]
        fn cut(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Cut));
        }

        #[unsafe(method(copy:))]
        fn copy(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Copy));
        }

        #[unsafe(method(paste:))]
        fn paste(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Paste));
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::SelectAll));
        }

        #[unsafe(method(delete:))]
        fn delete(&self, _sender: Option<&AnyObject>) {
            abort_on_panic(|| self.edit(EditAction::Delete));
        }
    }
);

impl MenuTarget {
    /// A target that reports an app's item chosen to `chosen`, and Edit's commands to `events`.
    pub(super) fn new(
        main_thread: MainThreadMarker,
        events: MenuEvents,
        chosen: Box<dyn Fn(MenuId)>,
    ) -> Retained<MenuTarget> {
        let this = main_thread
            .alloc::<MenuTarget>()
            .set_ivars(Actions { chosen, events });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    fn edit(&self, action: EditAction) {
        self.ivars().events.send(MenuEvent::Edit(action));
    }
}

/// Where a menu is built for, which decides what a provided item can be there.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    /// The menu bar, the one place the system's Services menu is.
    MenuBar,
    /// Every other menu: at a point, at a status item, the Dock's.
    Elsewhere,
}

/// `entries` as an `NSMenu` titled `title`, each item's action sent to `target`, enabled as the
/// value says.
pub(super) fn native_menu(
    main_thread: MainThreadMarker,
    title: &str,
    entries: &[MenuEntry],
    target: &MenuTarget,
    place: Place,
) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(main_thread.alloc(), &NSString::from_str(title));
    menu.setAutoenablesItems(false);
    for entry in entries {
        if let Some(item) = native_item(main_thread, entry, target, place) {
            menu.addItem(&item);
        }
    }
    menu
}

/// `entry` as an `NSMenuItem`; none for a provided item AppKit has no counterpart of here.
pub(super) fn native_item(
    main_thread: MainThreadMarker,
    entry: &MenuEntry,
    target: &MenuTarget,
    place: Place,
) -> Option<Retained<NSMenuItem>> {
    match entry {
        MenuEntry::Separator => Some(NSMenuItem::separatorItem(main_thread)),
        MenuEntry::Submenu {
            id,
            label,
            enabled,
            entries,
        } => {
            let item = titled_item(main_thread, label, None, "");
            item.setTag(*id as isize);
            item.setEnabled(*enabled);
            let submenu = native_menu(main_thread, label, entries, target, place);
            item.setSubmenu(Some(&submenu));
            Some(item)
        }
        MenuEntry::Item { id, item } => Some(app_item(main_thread, *id, item, target)),
        MenuEntry::Provided { id, item, enabled } => {
            let native = provided_item(main_thread, *item, target, place)?;
            native.setTag(*id as isize);
            native.setEnabled(*enabled);
            Some(native)
        }
    }
}

/// An item of the app's, which sends `itemChosen:` to `target`.
fn app_item(
    main_thread: MainThreadMarker,
    id: MenuId,
    item: &MenuItem,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    let key_equivalent = item
        .shortcut
        .as_ref()
        .and_then(key_equivalents::key_equivalent);
    let (key, modifiers) = key_equivalent
        .clone()
        .unwrap_or_else(|| (String::new(), NSEventModifierFlags::empty()));
    let native = titled_item(main_thread, &item.label, Some(sel!(itemChosen:)), &key);
    if key_equivalent.is_some() {
        native.setKeyEquivalentModifierMask(modifiers);
    }
    // SAFETY: the target, which the item holds weakly, is a `MenuTarget`, the type that answers
    // the item's action; whoever shows the menu keeps it.
    unsafe { native.setTarget(Some(target)) };
    native.setTag(id as isize);
    native.setEnabled(item.enabled);
    native.setState(match item.check {
        MenuItemCheck::Plain
        | MenuItemCheck::Checkbox(CheckState::Off)
        | MenuItemCheck::Radio(false) => NSControlStateValueOff,
        MenuItemCheck::Checkbox(CheckState::On) | MenuItemCheck::Radio(true) => {
            NSControlStateValueOn
        }
        MenuItemCheck::Checkbox(CheckState::Mixed) => NSControlStateValueMixed,
    });
    if let Some(icon) = &item.icon {
        native.setImage(images::ns_image(icon, images::MENU_ITEM_HEIGHT).as_deref());
    }
    native
}

/// AppKit's own item for `item`, its action sent through the responder chain, or, for Edit's,
/// to `target`; none for the Services menu anywhere but the menu bar.
fn provided_item(
    main_thread: MainThreadMarker,
    item: ProvidedItem,
    target: &MenuTarget,
    place: Place,
) -> Option<Retained<NSMenuItem>> {
    let app = application_name();
    let command = NSEventModifierFlags::Command;
    let (title, action, key, modifiers): (String, Sel, &str, NSEventModifierFlags) = match item {
        ProvidedItem::About => (
            format!("About {app}"),
            sel!(orderFrontStandardAboutPanel:),
            "",
            NSEventModifierFlags::empty(),
        ),
        ProvidedItem::Services => {
            if place != Place::MenuBar {
                return None;
            }
            let native = titled_item(main_thread, "Services", None, "");
            let services =
                NSMenu::initWithTitle(main_thread.alloc(), &NSString::from_str("Services"));
            native.setSubmenu(Some(&services));
            return Some(native);
        }
        ProvidedItem::Hide => (format!("Hide {app}"), sel!(hide:), "h", command),
        ProvidedItem::HideOthers => (
            "Hide Others".to_owned(),
            sel!(hideOtherApplications:),
            "h",
            command | NSEventModifierFlags::Option,
        ),
        ProvidedItem::ShowAll => (
            "Show All".to_owned(),
            sel!(unhideAllApplications:),
            "",
            NSEventModifierFlags::empty(),
        ),
        ProvidedItem::Quit => (format!("Quit {app}"), sel!(terminate:), "q", command),
        ProvidedItem::Minimize => (
            "Minimize".to_owned(),
            sel!(performMiniaturize:),
            "m",
            command,
        ),
        ProvidedItem::Zoom => (
            "Zoom".to_owned(),
            sel!(performZoom:),
            "",
            NSEventModifierFlags::empty(),
        ),
        ProvidedItem::ToggleFullScreen => (
            "Enter Full Screen".to_owned(),
            sel!(toggleFullScreen:),
            "f",
            command | NSEventModifierFlags::Control,
        ),
        ProvidedItem::BringAllToFront => (
            "Bring All to Front".to_owned(),
            sel!(arrangeInFront:),
            "",
            NSEventModifierFlags::empty(),
        ),
        ProvidedItem::Undo => ("Undo".to_owned(), sel!(undo:), "z", command),
        ProvidedItem::Redo => (
            "Redo".to_owned(),
            sel!(redo:),
            "z",
            command | NSEventModifierFlags::Shift,
        ),
        ProvidedItem::Cut => ("Cut".to_owned(), sel!(cut:), "x", command),
        ProvidedItem::Copy => ("Copy".to_owned(), sel!(copy:), "c", command),
        ProvidedItem::Paste => ("Paste".to_owned(), sel!(paste:), "v", command),
        ProvidedItem::SelectAll => ("Select All".to_owned(), sel!(selectAll:), "a", command),
        ProvidedItem::Delete => (
            "Delete".to_owned(),
            sel!(delete:),
            "",
            NSEventModifierFlags::empty(),
        ),
    };
    let native = titled_item(main_thread, &title, Some(action), key);
    if !key.is_empty() {
        native.setKeyEquivalentModifierMask(modifiers);
    }
    if EditAction::of(item).is_some() {
        // SAFETY: the target answers each of Edit's standard selectors, and whoever shows the
        // menu keeps it.
        unsafe { native.setTarget(Some(target)) };
    }
    Some(native)
}

/// An item titled `title`, sending `action`, chosen by `key` with Command unless its mask says
/// otherwise.
fn titled_item(
    main_thread: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
) -> Retained<NSMenuItem> {
    // SAFETY: each action is one AppKit's responders answer, or one `MenuTarget` defines, with the
    // one-argument signature AppKit sends menu actions with.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            main_thread.alloc(),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    }
}

/// The application's name, which the app's provided items carry: the process's, as AppKit titles
/// the application menu with it.
fn application_name() -> String {
    NSProcessInfo::processInfo().processName().to_string()
}
