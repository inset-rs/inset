//! The menus AppKit shows tell the app what is chosen in them. The system's standard menu bar, the
//! application's menu, Edit and Window, stands until the app sets its own, its Edit items sending
//! AppKit's standard actions; an item of the app's menu bar and of the Dock icon's menu reaches the
//! app by its id; the menu bar takes a key equivalent at once only while none of the app's windows
//! is key, which leaves the key to the framework first; and a status item, answered at once and
//! offering both kinds of menu, shows in the menu bar until its handle is dropped.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "the_menus_report_what_is_chosen";

/// Long enough for the window to open and the choices to arrive, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the choices did not arrive within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::MenusClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use inset_embedder::{
        EmbedderClient, Frame, IconImage, KeyData, MenuEntry, MenuId, MenuItem, MenuShortcut,
        PlatformRef, PointerReport, ShortcutModifiers, ViewId,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    /// The menu bar's item, chosen by Command-O.
    const OPEN: MenuId = 7;
    /// The Dock menu's item.
    const NEW_WINDOW: MenuId = 9;
    const STATUS_ITEM_TOOLTIP: &str = "an Inset status item";

    /// Once its window is open, sets the menus, chooses in them, and waits for the choices.
    pub(crate) struct MenusClient {
        platform: PlatformRef,
        /// The menu bar's items chosen, as the app heard them.
        chosen: Vec<MenuId>,
        /// The Dock menu's items chosen.
        chosen_in_dock: Vec<MenuId>,
        /// How many of each are to come, once they are all asked for.
        expected: Option<(usize, usize)>,
    }

    impl MenusClient {
        pub(crate) fn new(platform: PlatformRef) -> MenusClient {
            support::request_window(&platform, support::titled("menus"));
            MenusClient {
                platform,
                chosen: Vec::new(),
                chosen_in_dock: Vec::new(),
                expected: None,
            }
        }

        fn check_the_standard_menu_bar(&self) {
            let titles = appkit::menu_bar_titles();
            if titles.len() != 3 || titles[1..] != ["Edit", "Window"] {
                support::fail(TEST, &format!("the standard menu bar is {titles:?}"));
            }
            if appkit::menu_bar_class_name() != "InsetMainMenu" {
                support::fail(TEST, "the menu bar is not the embedder's");
            }
            let copy = appkit::menu_bar_action("Edit", "Copy");
            if copy != "copy:" {
                support::fail(TEST, &format!("Edit's Copy sends {copy}"));
            }
        }

        /// Sets the app's own menu bar and chooses its item, once from the menu and once by its
        /// key; answers whether the menu took the key, which it does only while no window is key.
        fn choose_in_the_apps_menu_bar(&self) -> bool {
            let menu_bar = self.platform.menu_bar().expect("a menu bar on macOS");
            let open = MenuItem::new("Open").shortcut(MenuShortcut {
                key: 'o' as u64,
                modifiers: ShortcutModifiers {
                    meta: true,
                    ..ShortcutModifiers::default()
                },
            });
            menu_bar.set_menus(&[MenuEntry::Submenu {
                id: 1,
                label: "File".to_owned(),
                enabled: true,
                entries: vec![MenuEntry::Item {
                    id: OPEN,
                    item: open,
                }],
            }]);
            if appkit::menu_bar_titles() != ["File"] {
                support::fail(TEST, "the app's menu bar did not replace the standard one");
            }
            appkit::choose_in_menu_bar("File", "Open");
            let taken = appkit::offer_command_key_to_menu_bar("o");
            println!(
                "the menu bar {} Command-O with {} window key",
                if taken { "took" } else { "declined" },
                if appkit::a_window_is_key() { "a" } else { "no" },
            );
            if taken == appkit::a_window_is_key() {
                support::fail(
                    TEST,
                    &format!(
                        "the menu bar {} Command-O while a window {} key",
                        if taken { "took" } else { "declined" },
                        if taken { "was" } else { "was not" },
                    ),
                );
            }
            taken
        }

        fn choose_in_the_dock_menu(&self) {
            let dock_menu = self.platform.dock_menu().expect("a Dock menu on macOS");
            dock_menu.set_menu(&[MenuEntry::Item {
                id: NEW_WINDOW,
                item: MenuItem::new("New Window"),
            }]);
            appkit::choose_in_dock_menu("New Window");
        }

        fn check_a_status_item_comes_and_goes(&self) {
            let tray_icons = self.platform.tray_icons().expect("status items on macOS");
            let image = IconImage {
                width: 2,
                height: 2,
                rgba: Arc::from([0u8, 0, 0, 255].repeat(4)),
                template: true,
            };
            let mut adding = tray_icons.add(&image, Some(STATUS_ITEM_TOOLTIP));
            let Poll::Ready(added) = adding
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            else {
                support::fail(TEST, "a status item is not answered at once");
            };
            let icon = added.unwrap_or_else(|error| support::fail(TEST, &error.to_string()));
            if icon.attached_menu().is_none() || icon.menu_at_icon().is_none() {
                support::fail(TEST, "a status item offers neither menu or only one");
            }
            if let Some(menu) = icon.attached_menu() {
                menu.set_menu(&[MenuEntry::Item {
                    id: 1,
                    item: MenuItem::new("Show"),
                }]);
            }
            if !appkit::shows_status_item_with_tooltip(STATUS_ITEM_TOOLTIP) {
                support::fail(TEST, "the status item does not show");
            }
            drop(icon);
            if appkit::shows_status_item_with_tooltip(STATUS_ITEM_TOOLTIP) {
                support::fail(TEST, "the status item outlived its handle");
            }
        }

        /// Passes once every choice asked for has arrived, each with its id.
        fn check_once_all_arrived(&self) {
            let Some((menu_bar, dock)) = self.expected else {
                return;
            };
            if self.chosen.len() < menu_bar || self.chosen_in_dock.len() < dock {
                return;
            }
            if self.chosen.iter().any(|&id| id != OPEN)
                || self.chosen_in_dock.iter().any(|&id| id != NEW_WINDOW)
            {
                support::fail(
                    TEST,
                    &format!(
                        "chosen {:?} in the menu bar and {:?} in the Dock menu",
                        self.chosen, self.chosen_in_dock
                    ),
                );
            }
            support::pass(TEST);
        }
    }

    impl EmbedderClient for MenusClient {
        fn view_added(&mut self, _id: ViewId) {
            self.check_the_standard_menu_bar();
            let key_taken = self.choose_in_the_apps_menu_bar();
            self.choose_in_the_dock_menu();
            self.check_a_status_item_comes_and_goes();
            self.expected = Some((1 + usize::from(key_taken), 1));
            self.check_once_all_arrived();
        }

        fn menu_selected(&mut self, id: MenuId) {
            self.chosen.push(id);
            self.check_once_all_arrived();
        }

        fn dock_menu_selected(&mut self, id: MenuId) {
            self.chosen_in_dock.push(id);
            self.check_once_all_arrived();
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}
        fn wake(&mut self, _elapsed: Duration) {}
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
