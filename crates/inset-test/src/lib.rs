//! Test scaffolding: a [`TestPlatform`] to run an `App` on, a [`TestView`] for it to draw
//! into, a [`MockClipboard`] for it to copy to and paste from, the system's menus
//! ([`MockMenuBar`], [`MockPopupMenus`], [`MockDockMenu`], [`MockTrayIcons`]) for it to set and
//! choose in, file dialogs ([`MockFileDialogs`], with [`MemoryFile`] and [`MemoryFolder`])
//! for it to open and save through, and a compositor, players and cameras ([`MockCompositor`],
//! [`MockPlayers`], [`MockCameras`]) for it to show media through, so a test supplies only the
//! piece it is about. The shape of `flutter_test`'s `TestPlatformDispatcher` and
//! `TestFlutterView`: real implementations of the host traits that answer what a test
//! configures and record what the framework asks of them.
//!
//! A dev-dependency of a crate with tests:
//!
//! ```toml
//! [dev-dependencies]
//! inset-test = "0.4"
//! ```

mod clipboard;
mod file_dialogs;
mod media;
mod menus;
mod notifications;
mod opener;
mod platform;
mod view;
mod windows;

pub use clipboard::MockClipboard;
pub use file_dialogs::{AskedDialog, MemoryFile, MemoryFolder, MockFileDialogs};
pub use media::{
    MockCameras, MockCaptureSession, MockCompositor, MockPlayer, MockPlayers, OpenedSurface,
    PlayerCall, SessionCall,
};
pub use menus::{
    MockDockMenu, MockMenuBar, MockPopupMenus, MockTrayIcons, ShownMenu, ShownTrayIcon,
};
pub use notifications::MockNotifications;
pub use opener::{MockOpener, Opened};
pub use platform::{TestDispatcher, TestPlatform};
pub use view::{IMPLICIT_VIEW_ID, TestView};
pub use windows::{TestWindow, TestWindows};
