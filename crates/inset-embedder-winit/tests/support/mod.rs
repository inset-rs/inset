//! What the window tests share: a watchdog for a test that would otherwise wait for ever, a clock
//! that ticks whether or not a window is in sight, the windows asked for whose views have arrived,
//! and what they ask of AppKit that the embedder offers no call for.

#![allow(dead_code, reason = "each test binary uses a part of it")]

use std::time::Duration;

use inset_embedder::{Instant, PlatformRef, RequestId, ViewRequest, WindowConfig, WindowRef};

/// How long a test that steps through time waits between its steps' checks.
pub const TICK: Duration = Duration::from_millis(16);

/// Asks the host to wake the app once `TICK` has passed, for a test that steps through time
/// whether or not any window is in sight: a frame of a window out of sight waits until it shows.
pub fn wake_after_a_tick(platform: &PlatformRef) {
    platform.dispatcher().wake_at(Some(Instant::now() + TICK));
}

/// Ends the process as a failure of `test` after `after`, from a thread of its own, with
/// what `reason` says then.
pub fn fail_after(
    test: &'static str,
    after: Duration,
    reason: impl FnOnce() -> String + Send + 'static,
) {
    std::thread::spawn(move || {
        std::thread::sleep(after);
        fail(test, &reason());
    });
}

/// Ends the process as a failure of `test`.
pub fn fail(test: &str, reason: &str) -> ! {
    eprintln!("test {test} ... FAILED: {reason}");
    std::process::exit(1);
}

/// Ends the process as a pass of `test`.
pub fn pass(test: &str) -> ! {
    println!("test {test} ... ok");
    std::process::exit(0);
}

/// Asks for a window configured as `config`; its view arrives through `view_added`.
pub fn request_window(platform: &PlatformRef, config: WindowConfig) -> RequestId {
    platform
        .request_view(ViewRequest {
            config,
            initial_data: None,
        })
        .expect("a desktop host makes windows")
}

/// A window of `title` at 300 by 200.
pub fn titled(title: &str) -> WindowConfig {
    WindowConfig {
        title: title.into(),
        size: [300.0, 200.0],
        ..WindowConfig::default()
    }
}

/// Moves the requests whose views have arrived from `requested` to `windows`, as the views'
/// windows, in the order they were asked for.
pub fn take_opened_windows(
    platform: &PlatformRef,
    requested: &mut Vec<RequestId>,
    windows: &mut Vec<WindowRef>,
) {
    let views = platform.views();
    requested.retain(|request_id| {
        let arrived = views
            .iter()
            .find(|view| view.request_id() == Some(*request_id));
        match arrived.and_then(|view| view.window()) {
            Some(window) => {
                windows.push(window);
                false
            }
            None => true,
        }
    });
}

/// What the tests ask of AppKit that the embedder offers no call for.
pub mod appkit {
    use std::cell::Cell;
    use std::ptr::{self, NonNull};
    use std::time::Duration;

    use block2::RcBlock;
    use dispatch2::{DispatchQueue, DispatchTime};
    use objc2::rc::{Retained, Weak};
    use objc2::{ClassType, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSApplicationDelegate, NSEvent, NSEventMask, NSEventModifierFlags,
        NSEventType, NSMenu, NSMenuItem, NSView, NSWindow,
    };
    use objc2_foundation::{NSObjectProtocol, NSPoint, NSString};
    use objc2_quartz_core::CAMetalLayer;

    /// The virtual key code of Escape (`kVK_Escape`).
    const ESCAPE_KEY_CODE: u16 = 0x35;
    /// The virtual key code of the O key (`kVK_ANSI_O`).
    const O_KEY_CODE: u16 = 0x1F;

    fn application() -> Retained<NSApplication> {
        NSApplication::sharedApplication(MainThreadMarker::new().expect("the main thread"))
    }

    fn window_titled(title: &str) -> Option<Retained<NSWindow>> {
        let title = NSString::from_str(title);
        application()
            .windows()
            .iter()
            .find(|window| window.title() == title)
    }

    /// Whether AppKit lists a window titled `title` among the application's windows.
    pub fn lists_window_titled(title: &str) -> bool {
        window_titled(title).is_some()
    }

    /// Whether a Metal layer covers the content of the window titled `title`: the layer wgpu
    /// draws a view's surface into, which it adds to the content view's layer as it makes the
    /// surface.
    pub fn has_a_metal_layer(title: &str) -> bool {
        let Some(content) = window_titled(title)
            .and_then(|window| window.contentView())
            .and_then(|view| view.layer())
        else {
            return false;
        };
        let frame = content.frame();
        let centre = NSPoint::new(
            frame.origin.x + frame.size.width / 2.0,
            frame.origin.y + frame.size.height / 2.0,
        );
        content
            .hitTest(centre)
            .is_some_and(|layer| layer.isKindOfClass(CAMetalLayer::class()))
    }

    /// Clicks the close button of the window titled `title`, as a person would.
    pub fn press_close_button(title: &str) {
        window_titled(title).expect("the window").performClose(None);
    }

    /// Asks the application to quit as Cmd-Q does: `terminate:`, which asks its delegate first.
    pub fn press_quit() {
        application().terminate(None);
    }

    pub fn minimize(title: &str) {
        window_titled(title).expect("the window").miniaturize(None);
    }

    pub fn restore(title: &str) {
        window_titled(title)
            .expect("the window")
            .deminiaturize(None);
    }

    pub fn hide_application() {
        application().hide(None);
    }

    pub fn unhide_application() {
        application().unhide(None);
    }

    /// Puts the window titled `title` in front of every application's windows without
    /// activating this one, so that no window of another application covers it.
    pub fn order_in_front_of_everything(title: &str) {
        window_titled(title)
            .expect("the window")
            .orderFrontRegardless();
    }

    /// Presses Escape, as a person closing the menu that tracks the pointer would: the key goes to
    /// the front of the application's events, which a menu's own loop reads too.
    pub fn press_escape() {
        let escape = NSString::from_str("\u{1b}");
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(),
            0.0,
            0,
            None,
            &escape,
            &escape,
            false,
            ESCAPE_KEY_CODE,
        )
        .expect("an Escape key event");
        application().postEvent_atStart(&event, true);
    }

    /// Presses Escape `after` from now, from the main queue, which runs while a menu tracks the
    /// pointer.
    pub fn press_escape_after(after: Duration) {
        let when = DispatchTime::try_from(after).expect("a delay that fits");
        let _ = DispatchQueue::main().after(when, press_escape);
    }

    /// The titles of the menu bar's menus, in order.
    pub fn menu_bar_titles() -> Vec<String> {
        let Some(menu) = application().mainMenu() else {
            return Vec::new();
        };
        menu.itemArray()
            .iter()
            .map(|item| item.title().to_string())
            .collect()
    }

    /// The name of the menu bar's class.
    pub fn menu_bar_class_name() -> String {
        application()
            .mainMenu()
            .map(|menu| menu.class().name().to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn menu_bar_item(menu: &str, item: &str) -> (Retained<NSMenu>, isize) {
        let submenu = application()
            .mainMenu()
            .and_then(|bar| bar.itemWithTitle(&NSString::from_str(menu)))
            .and_then(|item| item.submenu())
            .expect("the menu in the menu bar");
        let index = submenu.indexOfItemWithTitle(&NSString::from_str(item));
        assert!(index >= 0, "the item {item} in {menu}");
        (submenu, index)
    }

    /// The name of the action the item `item` of the menu bar's menu `menu` sends.
    pub fn menu_bar_action(menu: &str, item: &str) -> String {
        let (submenu, index) = menu_bar_item(menu, item);
        let item: Retained<NSMenuItem> = submenu.itemAtIndex(index).expect("the item");
        item.action()
            .map(|action| action.name().to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Chooses the item `item` of the menu bar's menu `menu`, as a click on it would.
    pub fn choose_in_menu_bar(menu: &str, item: &str) {
        let (submenu, index) = menu_bar_item(menu, item);
        submenu.performActionForItemAtIndex(index);
    }

    /// Chooses the item `item` of the menu the Dock icon shows, which AppKit asks the
    /// application's delegate for.
    pub fn choose_in_dock_menu(item: &str) {
        let app = application();
        let delegate = app.delegate().expect("the application's delegate");
        let menu = delegate.applicationDockMenu(&app).expect("a Dock menu");
        let index = menu.indexOfItemWithTitle(&NSString::from_str(item));
        assert!(index >= 0, "the item {item} in the Dock menu");
        menu.performActionForItemAtIndex(index);
    }

    /// Offers the menu bar Command and `character` as AppKit offers a key equivalent, and answers
    /// whether it took it.
    pub fn offer_command_key_to_menu_bar(character: &str) -> bool {
        let characters = NSString::from_str(character);
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::Command,
            0.0,
            0,
            None,
            &characters,
            &characters,
            false,
            0,
        )
        .expect("a key event");
        application()
            .mainMenu()
            .expect("a menu bar")
            .performKeyEquivalent(&event)
    }

    /// Whether one of the application's windows is key.
    pub fn a_window_is_key() -> bool {
        application().keyWindow().is_some()
    }

    /// Presses Command-O in the window titled `title` as AppKit delivers a key to the key window,
    /// while the window is not key: activating the application would take the keyboard from the
    /// user's own app. A local monitor hands the key-down, which AppKit is delivering as its
    /// current event, to the window itself, and nothing else sees it.
    #[allow(
        unsafe_code,
        reason = "AppKit's local event monitor answers through a raw pointer"
    )]
    pub fn press_command_o_in(title: &str) {
        let window = window_titled(title).expect("the window");
        let window_number = window.windowNumber();
        let delivered = Cell::new(false);
        let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            if delivered.replace(true) {
                return event.as_ptr();
            }
            // SAFETY: AppKit hands the monitor a live event, for the length of the call.
            window.sendEvent(unsafe { event.as_ref() });
            ptr::null_mut()
        });
        // SAFETY: the handler answers the event it was given, or null for one it delivered.
        let monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &handler)
        };
        // The monitor stays for the test's life, which ends with the process.
        std::mem::forget(monitor);
        let characters = NSString::from_str("o");
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::Command,
            0.0,
            window_number,
            None,
            &characters,
            &characters,
            false,
            O_KEY_CODE,
        )
        .expect("a key event");
        application().postEvent_atStart(&event, true);
    }

    /// Whether a status item whose button's tooltip is `tooltip` shows in the menu bar: each
    /// status item's button is in a window of the application's own, ordered out once the item
    /// is removed.
    pub fn shows_status_item_with_tooltip(tooltip: &str) -> bool {
        let tooltip = NSString::from_str(tooltip);
        application().windows().iter().any(|window| {
            window.isVisible()
                && window
                    .contentView()
                    .is_some_and(|view| has_tooltip(&view, &tooltip))
        })
    }

    fn has_tooltip(view: &NSView, tooltip: &NSString) -> bool {
        view.toolTip().is_some_and(|own| &*own == tooltip)
            || view
                .subviews()
                .iter()
                .any(|subview| has_tooltip(&subview, tooltip))
    }

    /// A window as AppKit has it, held weakly: whether it is still alive tells whether anything
    /// still holds it.
    pub struct WeakWindow(Weak<NSWindow>);

    impl WeakWindow {
        pub fn titled(title: &str) -> WeakWindow {
            WeakWindow(Weak::from_retained(
                &window_titled(title).expect("the window"),
            ))
        }

        pub fn is_alive(&self) -> bool {
            self.0.load().is_some()
        }
    }
}
