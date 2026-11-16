//! Test scaffolding: a [`TestPlatform`] to run an `App` on, a [`TestView`] for it to draw
//! into, whose [`TestViewHost`] records what it asks of the system, a [`MockClipboard`] for it to copy to and paste from, the system's menus
//! ([`MockMenuBar`], [`MockPopupMenus`], [`MockDockMenu`], [`MockTrayIcons`]) for it to set and
//! choose in, file dialogs ([`MockFileDialogs`], with [`MemoryFile`] and [`MemoryFolder`])
//! for it to open and save through, a hardware buffer allocator ([`MockHardwareBufferAllocator`]) for it to allocate buffers with,
//! and a compositor, frame readers, players, media assets and
//! cameras ([`MockCompositor`], [`MockVideoFrameReaders`], [`MockPlayers`], [`MockMediaAssets`],
//! [`MockCameras`]) for it to show and read media through, image codecs ([`MockImageCodecs`])
//! for it to decode and encode images with, with the user's caption settings ([`MockCaptionSettings`]) to draw subtitles
//! in, so a test supplies only the piece it is about. The shape of `flutter_test`'s `TestPlatformDispatcher` and
//! `TestFlutterView`: real implementations of the host traits that answer what a test
//! configures and record what the framework asks of them. Its fonts are flutter_test's: every
//! family is FlutterTest ([`TestFontSource`]) unless the test gives the platform other fonts.
//! In a debug build, [`object_snapshot`] and [`assert_no_new_objects`] check that a piece of a
//! test leaves no object behind in the `App`'s arena.
//!
//! A dev-dependency of a crate with tests:
//!
//! ```toml
//! [dev-dependencies]
//! waft-test = "0.1"
//! ```

mod caption_style;
mod clipboard;
mod file_dialogs;
mod hardware_buffers;
mod image_codecs;
mod media;
mod menus;
mod notifications;
#[cfg(debug_assertions)]
mod objects;
mod opener;
mod platform;
mod test_font_manager;
mod time_zone;
mod view;
mod view_host;
mod windows;

pub use caption_style::MockCaptionSettings;
pub use clipboard::MockClipboard;
pub use file_dialogs::{AskedDialog, MemoryFile, MemoryFolder, MockFileDialogs};
pub use hardware_buffers::{AskedAllocation, MockHardwareBuffer, MockHardwareBufferAllocator};
pub use image_codecs::{CodecCall, ImageAnswers, ImageFacts, MockImageCodecs};
pub use media::{
    AssetAnswers, FrameAsked, MadeLayer, MockCameras, MockCaptureSession, MockCompositor,
    MockMediaAsset, MockMediaAssets, MockPlayer, MockPlayers, MockVideoFrame,
    MockVideoFrameReaders, PlayerCall, VideoOutputKind,
};
pub use menus::{
    MockDockMenu, MockMenuBar, MockPopupMenus, MockTrayIcons, ShownMenu, ShownTrayIcon,
};
pub use notifications::MockNotifications;
#[cfg(debug_assertions)]
pub use objects::{ObjectSnapshot, assert_no_new_objects, object_snapshot};
pub use opener::{MockOpener, Opened};
pub use platform::{TestDispatcher, TestPlatform};
pub use test_font_manager::TestFontSource;
pub use time_zone::TestTimeZone;
pub use view::{IMPLICIT_VIEW_ID, MetricsListener, TestView};
pub use view_host::{TestViewHost, TextInputCall};
pub use windows::{TestWindow, TestWindows};
