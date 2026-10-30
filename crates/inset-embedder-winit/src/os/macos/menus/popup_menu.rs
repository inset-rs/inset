//! A menu at a point in a view: AppKit's `NSMenu`, popped up in the view's own coordinates, which
//! winit's view flips, so a point is the framework's offset as it is.
//!
//! The menu tracks the mouse in AppKit's own loop, which a perform request on the main run loop
//! starts once winit's current call into the embedder has returned, in the common modes, as
//! Flutter's `FlutterRunLoop` runs its blocks. The loop then runs outside every call of winit's,
//! whose run-loop observers and event source, in the common modes too, go on handing the main
//! loop its events meanwhile, so the framework keeps drawing while the menu is open; and outside
//! the main dispatch queue, which a block holding the menu would stop for as long as it is open,
//! and which serves the waits on a screen's vsync (`vsync_waiter.rs`). The item chosen is
//! reported once the menu closes (`main_loop/`, "Who calls whom").

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use inset_embedder::{MenuEntry, MenuId, Offset};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSView;
use objc2_foundation::{
    NSArray, NSObject, NSObjectNSDelayedPerforming, NSPoint, NSRunLoopCommonModes,
};
use winit::window::Window;

use super::super::{abort_on_panic, ns_view};
use super::{MenuTarget, Place, native_menu};
use crate::menus::MenuEvents;

/// What the menu's close hands the item chosen to.
pub(super) type Closed = Box<dyn FnOnce(Option<MenuId>) + Send>;

/// Opens `menu` at `at` in `window`'s view once winit's call has returned. Off the main thread, or
/// with no view, there is no menu to open.
pub(super) fn open(
    window: &dyn Window,
    at: Offset,
    menu: Vec<MenuEntry>,
    events: MenuEvents,
    closed: Closed,
) {
    let (Some(main_thread), Some(view)) = (MainThreadMarker::new(), ns_view(window)) else {
        closed(None);
        return;
    };
    let at = NSPoint::new(at.dx(), at.dy());
    open_in(main_thread, view, at, menu, events, closed);
}

/// Opens `menu` at `at` in `view`'s own coordinates once the current call has returned.
pub(super) fn open_in(
    main_thread: MainThreadMarker,
    view: Retained<NSView>,
    at: NSPoint,
    menu: Vec<MenuEntry>,
    events: MenuEvents,
    closed: Closed,
) {
    let opener = MenuOpener::new(
        main_thread,
        Opening {
            view,
            at,
            menu,
            events,
            closed: RefCell::new(Some(closed)),
        },
    );
    // SAFETY: the modes are a constant Foundation exports for the life of the process.
    let modes = NSArray::from_slice(&[unsafe { NSRunLoopCommonModes }]);
    // SAFETY: `open:` is defined on `MenuOpener` below with the one-argument signature a perform
    // request calls, and it ignores its argument; the run loop keeps the opener until it is called.
    unsafe {
        opener.performSelector_withObject_afterDelay_inModes(sel!(open:), None, 0.0, &modes);
    }
}

/// What the opener opens, where, and who hears what was chosen.
struct Opening {
    view: Retained<NSView>,
    at: NSPoint,
    menu: Vec<MenuEntry>,
    events: MenuEvents,
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
                let chosen = self.pop_up();
                if let Some(closed) = self.ivars().closed.borrow_mut().take() {
                    closed(chosen);
                }
            });
        }
    }
);

impl MenuOpener {
    fn new(main_thread: MainThreadMarker, opening: Opening) -> Retained<Self> {
        let this = main_thread.alloc::<MenuOpener>().set_ivars(opening);
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    /// Shows the menu, tracked in AppKit's own loop until it closes, and answers the id of the
    /// app's item chosen.
    fn pop_up(&self) -> Option<MenuId> {
        let opening = self.ivars();
        let main_thread = self.mtm();
        let chosen = Rc::new(Cell::new(None));
        let choice = Rc::clone(&chosen);
        let target = MenuTarget::new(
            main_thread,
            opening.events.clone(),
            Box::new(move |id| choice.set(Some(id))),
        );
        let menu = native_menu(main_thread, "", &opening.menu, &target, Place::Elsewhere);
        menu.popUpMenuPositioningItem_atLocation_inView(None, opening.at, Some(&opening.view));
        chosen.get()
    }
}
