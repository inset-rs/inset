//! The popup menu on macOS: AppKit's `NSMenu`, shown at the pointer.
//!
//! `AppKit::popup_menu` blocks while the menu tracks the mouse, in AppKit's own loop, and reports
//! which entry was chosen; the caller acts on the choice with whatever it was holding. winit
//! calls the embedder nothing inside the menu, but display links and the main dispatch queue run
//! there (`main_loop/`, "Who calls whom").

use std::cell::Cell;

use inset_embedder::PopupMenuEntry;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSControlStateValueOn, NSEvent, NSMenu, NSMenuItem};
use objc2_foundation::{NSObject, NSString};

use super::{AppKit, abort_on_panic};
use crate::os::NativeMenus;

impl NativeMenus for AppKit {
    /// AppKit's `NSMenu` at the pointer, tracked in AppKit's own loop until it closes.
    fn popup_menu(entries: &[PopupMenuEntry]) -> Option<usize> {
        let mtm = MainThreadMarker::new()?;
        let chooser = Chooser::new(mtm);
        let menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(""));
        menu.setAutoenablesItems(false);
        for (index, entry) in entries.iter().enumerate() {
            menu.addItem(&menu_item(mtm, index, entry, &chooser));
        }
        let location = NSEvent::mouseLocation();
        menu.popUpMenuPositioningItem_atLocation_inView(None, location, None);
        chooser.ivars().chosen.get()
    }
}

/// `entry` as an `NSMenu` item, which reports `index` to `chooser` when chosen.
fn menu_item(
    mtm: MainThreadMarker,
    index: usize,
    entry: &PopupMenuEntry,
    chooser: &Chooser,
) -> Retained<NSMenuItem> {
    match entry {
        PopupMenuEntry::Separator => NSMenuItem::separatorItem(mtm),
        PopupMenuEntry::Item {
            label,
            enabled,
            checked,
        } => {
            // SAFETY: `choose:` is defined on `Chooser` below with the one-argument
            // signature AppKit sends menu actions with.
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(label),
                    Some(sel!(choose:)),
                    &NSString::from_str(""),
                )
            };
            // SAFETY: the target, which the item holds weakly, is a `Chooser`, the type that
            // answers the item's `choose:` action.
            unsafe { item.setTarget(Some(chooser)) };
            item.setTag(index as isize);
            item.setEnabled(*enabled);
            if *checked {
                item.setState(NSControlStateValueOn);
            }
            item
        }
    }
}

struct Chosen {
    chosen: Cell<Option<usize>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Chooser` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetPopupMenuChooser"]
    #[ivars = Chosen]
    struct Chooser;

    impl Chooser {
        #[unsafe(method(choose:))]
        fn choose(&self, sender: &NSMenuItem) {
            abort_on_panic(|| self.ivars().chosen.set(Some(sender.tag() as usize)));
        }
    }
);

impl Chooser {
    fn new(mtm: MainThreadMarker) -> Retained<Chooser> {
        let this = mtm.alloc::<Chooser>().set_ivars(Chosen {
            chosen: Cell::new(None),
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}
