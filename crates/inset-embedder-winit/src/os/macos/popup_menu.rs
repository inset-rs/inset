//! The popup menu on macOS: AppKit's `NSMenu`, shown at the pointer.
//!
//! The menu tracks the mouse in AppKit's own loop, which a perform request on the main run loop
//! starts once winit's current call into the embedder has returned, in the common modes, as
//! Flutter's `FlutterRunLoop` runs its blocks. The loop then runs outside every call of winit's,
//! whose run-loop observers and event source, in the common modes too, go on handing the main
//! loop its events meanwhile, so the framework keeps drawing while the menu is open; and outside
//! the main dispatch queue, which a block holding the menu would stop for as long as it is open,
//! and which serves the waits on a screen's vsync (`vsync_waiter.rs`). The entry chosen is
//! reported once the menu closes (`main_loop/`, "Who calls whom").

use std::cell::{Cell, RefCell};

use inset_embedder::PopupMenuEntry;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSControlStateValueOn, NSEvent, NSMenu, NSMenuItem};
use objc2_foundation::{
    NSArray, NSObject, NSObjectNSDelayedPerforming, NSRunLoopCommonModes, NSString,
};

use super::{AppKit, abort_on_panic};
use crate::os::NativeMenus;

/// What the menu's close hands the entry chosen to.
type Closed = Box<dyn FnOnce(Option<usize>) + Send>;

impl NativeMenus for AppKit {
    /// AppKit's `NSMenu` at the pointer, opened by a perform request on the main run loop in its
    /// common modes. Off the main thread there is no menu to open.
    fn open_popup_menu(entries: Vec<PopupMenuEntry>, closed: Closed) {
        let Some(mtm) = MainThreadMarker::new() else {
            closed(None);
            return;
        };
        let opener = MenuOpener::new(mtm, entries, closed);
        // SAFETY: the modes are a constant Foundation exports for the life of the process.
        let modes = NSArray::from_slice(&[unsafe { NSRunLoopCommonModes }]);
        // SAFETY: `open:` is defined on `MenuOpener` below with the one-argument signature a
        // perform request calls, and it ignores its argument; the run loop keeps the opener until
        // it is called.
        unsafe {
            opener.performSelector_withObject_afterDelay_inModes(sel!(open:), None, 0.0, &modes);
        }
    }
}

/// What the opener opens, and who hears what was chosen.
struct Opening {
    entries: Vec<PopupMenuEntry>,
    closed: RefCell<Option<Closed>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `MenuOpener` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetPopupMenuOpener"]
    #[ivars = Opening]
    struct MenuOpener;

    impl MenuOpener {
        #[unsafe(method(open:))]
        fn open(&self, _argument: Option<&AnyObject>) {
            abort_on_panic(|| {
                let opening = self.ivars();
                let chosen = popup_menu(&opening.entries);
                if let Some(closed) = opening.closed.borrow_mut().take() {
                    closed(chosen);
                }
            });
        }
    }
);

impl MenuOpener {
    fn new(mtm: MainThreadMarker, entries: Vec<PopupMenuEntry>, closed: Closed) -> Retained<Self> {
        let this = mtm.alloc::<MenuOpener>().set_ivars(Opening {
            entries,
            closed: RefCell::new(Some(closed)),
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

/// Shows `entries` as an `NSMenu` at the pointer, tracked in AppKit's own loop until it closes,
/// and answers the index of the entry chosen. Main thread.
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
