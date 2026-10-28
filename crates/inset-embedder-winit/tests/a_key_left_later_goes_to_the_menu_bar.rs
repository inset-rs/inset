//! A key the app claims when it comes and leaves later, as WaOS does for a program that answers on
//! its own thread, goes on to the menu bar when the app hands it back: the item whose key
//! equivalent it is takes it, from the key-down AppKit delivered, and the app hears the item
//! chosen. A key handed back twice is taken once.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window, presses Command-O in it without activating the application, and needs a login
//! session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_key_left_later_goes_to_the_menu_bar";

/// Long enough for the window to open and the key to go round, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the item was not chosen within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::KeyClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, KeyEventType, MenuEntry, MenuId, MenuItem, MenuShortcut,
        PlatformRef, PointerDataPacket, ShortcutModifiers, ViewId,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    const TITLE: &str = "a key left later";
    /// The menu bar's item, whose key equivalent is Command-O.
    const OPEN: MenuId = 7;
    /// The logical key of O.
    const O: u64 = 0x6f;

    /// Sets a menu bar with Command-O, presses it in its window, claims the key, and hands it back
    /// on a later turn.
    pub(crate) struct KeyClient {
        platform: PlatformRef,
        /// The key claimed, until it is handed back.
        claimed: Option<KeyData>,
    }

    impl KeyClient {
        pub(crate) fn new(platform: PlatformRef) -> KeyClient {
            support::request_window(&platform, support::titled(TITLE));
            KeyClient {
                platform,
                claimed: None,
            }
        }

        fn set_the_menu_bar(&self) {
            let open = MenuItem::new("Open").shortcut(MenuShortcut {
                key: O,
                modifiers: ShortcutModifiers {
                    meta: true,
                    ..ShortcutModifiers::default()
                },
            });
            self.platform
                .menu_bar()
                .expect("a menu bar on macOS")
                .set_menus(&[MenuEntry::Submenu {
                    id: 1,
                    label: "File".to_owned(),
                    enabled: true,
                    entries: vec![MenuEntry::Item {
                        id: OPEN,
                        item: open,
                    }],
                }]);
        }

        /// Hands the claimed key back to the menu bar, twice.
        fn hand_the_key_back(&mut self) {
            let Some(key) = self.claimed.take() else {
                return;
            };
            let menu_bar = self.platform.menu_bar().expect("a menu bar on macOS");
            if !menu_bar.perform_key_equivalent(&key) {
                support::fail(TEST, "the menu bar did not take the key handed back");
            }
            if menu_bar.perform_key_equivalent(&key) {
                support::fail(TEST, "the menu bar took the key twice");
            }
            let unknown = KeyData {
                time_stamp: key.time_stamp + Duration::from_secs(60),
                ..key
            };
            if menu_bar.perform_key_equivalent(&unknown) {
                support::fail(TEST, "the menu bar took a key it never saw");
            }
        }
    }

    impl EmbedderClient for KeyClient {
        fn view_added(&mut self, _id: ViewId) {
            self.set_the_menu_bar();
            appkit::press_command_o_in(TITLE);
        }

        /// Claims Command-O, as an app does that answers it later.
        fn key_data(&mut self, data: KeyData) -> bool {
            if data.event_type != KeyEventType::Down || data.synthesized || data.logical != O {
                return false;
            }
            self.claimed = Some(data);
            self.platform.dispatcher().wake_now();
            true
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.hand_the_key_back();
        }

        fn menu_selected(&mut self, id: MenuId) {
            if id != OPEN {
                support::fail(TEST, &format!("the menu bar chose {id}"));
            }
            support::pass(TEST);
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
