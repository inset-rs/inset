//! The popup menu on macOS: AppKit's `NSMenu`, shown at the pointer.
//!
//! `popup_menu` blocks while the menu tracks the mouse and reports which entry was chosen;
//! the caller acts on the choice with whatever it was holding, and nothing runs
//! re-entrantly from inside the menu.

use std::cell::Cell;

use inset_embedder::PopupMenuEntry;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSControlStateValueOn, NSEvent, NSMenu, NSMenuItem};
use objc2_foundation::{NSObject, NSString};

/// Shows the entries as a menu at the pointer and waits for it to close. Returns the index
/// of the entry chosen, if any.
pub(crate) fn popup_menu(entries: &[PopupMenuEntry]) -> Option<usize> {
    let mtm = MainThreadMarker::new()?;
    let chooser = Chooser::new(mtm);
    let menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(""));
    menu.setAutoenablesItems(false);
    for (index, entry) in entries.iter().enumerate() {
        match entry {
            PopupMenuEntry::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
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
                // SAFETY: the chooser outlives the menu, which this function holds until
                // tracking ends.
                unsafe { item.setTarget(Some(&chooser)) };
                item.setTag(index as isize);
                item.setEnabled(*enabled);
                if *checked {
                    item.setState(NSControlStateValueOn);
                }
                menu.addItem(&item);
            }
        }
    }
    let location = NSEvent::mouseLocation();
    menu.popUpMenuPositioningItem_atLocation_inView(None, location, None);
    chooser.ivars().chosen.get()
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
            self.ivars().chosen.set(Some(sender.tag() as usize));
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
