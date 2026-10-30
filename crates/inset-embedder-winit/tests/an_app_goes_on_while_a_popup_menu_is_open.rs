//! While a popup menu tracks the pointer, in AppKit's own loop, the app goes on: its view keeps
//! getting the framework's frames from the main loop, and the platform's vsync waiter keeps
//! serving the waits on the view's screen. The menu opens after `PopupMenus::show` returns, its
//! loop outside every call into the app, and the future `show` answered resolves once it closes,
//! with no entry chosen when it closes with Escape.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window and a popup menu for about a second, and needs a login session's window server.

// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "an_app_goes_on_while_a_popup_menu_is_open";

/// Long enough for the window to open and the menu to close, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the menu did not close within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::MenuClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::ops::Range;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
    use std::sync::{Arc, Mutex, Weak};
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;

    use inset_embedder::{
        Dispatcher, EmbedderClient, Frame, Instant, KeyData, MenuEntry, MenuItem, Offset,
        PlatformRef, PointerReport, PopupMenuFuture, ScreenId, ViewId, VsyncObserver, VsyncWaiter,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    /// How long the menu stays open before Escape closes it.
    const MENU_OPEN_FOR: Duration = Duration::from_millis(600);
    /// The stretch of it counted: after the menu has surely opened, before Escape.
    const COUNTED: Range<Duration> = Duration::from_millis(100)..Duration::from_millis(500);
    /// The fewest frames, and waits served, in that stretch: a 60 Hz display gives about 24.
    const FEWEST: usize = 10;
    /// The longest a wait takes to be served: a few refreshes, at the slowest rate a display has.
    const PATIENCE: Duration = Duration::from_millis(500);

    /// Asks for a window, and once its view is on a screen, animates it, waits on its screen's
    /// refreshes from a thread of its own, and opens a popup menu.
    pub(crate) struct MenuClient {
        platform: PlatformRef,
        /// When each of the view's frames came.
        frames: Vec<Instant>,
        /// When each wait was served, by the waiting thread.
        waits: Arc<Mutex<Vec<Instant>>>,
        stop_waiting: Arc<AtomicBool>,
        /// The menu open, and when it was asked for.
        menu: Option<(PopupMenuFuture, Instant)>,
    }

    impl MenuClient {
        pub(crate) fn new(platform: PlatformRef) -> MenuClient {
            support::request_window(&platform, support::titled("drawing under a menu"));
            MenuClient {
                platform,
                frames: Vec::new(),
                waits: Arc::default(),
                stop_waiting: Arc::default(),
                menu: None,
            }
        }

        /// Shows a popup menu in `view`, closed with Escape after [`MENU_OPEN_FOR`].
        fn open_menu(&mut self, view: ViewId) {
            let menus = self.platform.popup_menus().expect("a popup menu on macOS");
            let entries = [MenuEntry::Item {
                id: 1,
                item: MenuItem::new("Drawing"),
            }];
            let shown = menus.show(view, Offset::new(20.0, 20.0), &entries);
            appkit::press_escape_after(MENU_OPEN_FOR);
            self.menu = Some((shown, Instant::now()));
        }

        /// Once the menu has closed, checks what went on while it was open.
        fn check_once_the_menu_closed(&mut self) {
            let Some((shown, opened_at)) = &mut self.menu else {
                return;
            };
            let waker = Waker::from(Arc::new(WakeClient(self.platform.dispatcher())));
            let Poll::Ready(chosen) = Pin::new(shown).poll(&mut Context::from_waker(&waker)) else {
                return;
            };
            self.stop_waiting.store(true, Ordering::SeqCst);
            let counted = (*opened_at + COUNTED.start)..(*opened_at + COUNTED.end);
            let frames = count_within(&self.frames, &counted);
            let waits = count_within(&self.waits.lock().unwrap(), &counted);
            println!(
                "{frames} frames and {waits} waits served in {:?} of the menu",
                COUNTED.end - COUNTED.start
            );
            if chosen.is_some() {
                support::fail(TEST, "Escape chose an entry");
            }
            if frames < FEWEST || waits < FEWEST {
                support::fail(
                    TEST,
                    &format!("{frames} frames and {waits} waits served while the menu was open"),
                );
            }
            support::pass(TEST);
        }
    }

    /// Wakes the client, which polls the menu's future then.
    struct WakeClient(Arc<dyn Dispatcher>);

    impl Wake for WakeClient {
        fn wake(self: Arc<Self>) {
            self.0.wake_now();
        }
    }

    fn count_within(times: &[Instant], within: &Range<Instant>) -> usize {
        times.iter().filter(|time| within.contains(time)).count()
    }

    /// Tells the waiting thread each call.
    struct Calls(Sender<()>);

    impl VsyncObserver for Calls {
        fn vsync(&self, _at: Instant) {
            let _ = self.0.send(());
        }
    }

    /// Waits on `screen` again and again until `stop`, noting when each wait was served.
    fn wait_on(
        waiter: Arc<dyn VsyncWaiter>,
        screen: ScreenId,
        served: Arc<Mutex<Vec<Instant>>>,
        stop: Arc<AtomicBool>,
    ) {
        let (sender, calls) = channel();
        let observer = Arc::new(Calls(sender));
        let weak = Arc::downgrade(&observer) as Weak<dyn VsyncObserver>;
        while !stop.load(Ordering::SeqCst) {
            waiter.next_vsync(screen, weak.clone());
            match calls.recv_timeout(PATIENCE) {
                Ok(()) => served.lock().unwrap().push(Instant::now()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => unreachable!("the observer is held"),
            }
        }
    }

    impl EmbedderClient for MenuClient {
        /// The view's animation: each frame asks for the next.
        fn frame(&mut self, views: &[ViewId], _frame: Frame) {
            for &view in views {
                self.frames.push(Instant::now());
                self.platform.request_frame(view);
            }
            self.check_once_the_menu_closed();
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.check_once_the_menu_closed();
        }

        fn view_added(&mut self, id: ViewId) {
            let view = self.platform.view(id).expect("the platform lists the view");
            let Some(screen) = view.metrics().screen else {
                support::fail(TEST, "the view is on no screen");
            };
            let waiter = self.platform.vsync_waiter().expect("a vsync waiter");
            let (waits, stop) = (Arc::clone(&self.waits), Arc::clone(&self.stop_waiting));
            std::thread::spawn(move || wait_on(waiter, screen, waits, stop));
            self.platform.request_frame(id);
            self.open_menu(id);
        }

        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_reports(&mut self, _reports: Vec<PointerReport>) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
