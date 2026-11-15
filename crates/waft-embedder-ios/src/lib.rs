//! iOS host: UIKit's application and scenes, one view of the app's in each scene's window, shaped
//! as Flutter's iOS embedding is (`FlutterAppDelegate`, `FlutterSceneDelegate`,
//! `FlutterViewController`).
//!
//! [`IosEmbedder::run`] hands the process to `UIApplicationMain` with Waft's application delegate
//! (`app_delegate`), which starts the app as the launch finishes, with Flutter's implicit view and
//! no window for it yet. UIKit then connects the app's scenes, each through Waft's scene delegate
//! (`scene_delegate`), as the `UIApplicationSceneManifest` cargo waft writes names it; each scene
//! gets a window whose root view controller shows one view of the app's (`view_controller`,
//! `ui_view`), the implicit view in the first. A view outlives its scene's connection: UIKit
//! disconnects a scene in the background and connects it again later, and the view's tree and last
//! picture wait for it (`views`).
//!
//! What UIKit tells a delegate, a view controller, a view or a notice's observer is handed to the
//! host as a [`host::HostEvent`] and served there, one at a time, on the main thread, where the
//! client is called (`host`). The host serves each at once when UIKit calls from its own loop, so a
//! resize draws its frame before UIKit commits the new size, and queues one UIKit calls back while
//! the host is serving another, so the client is never entered twice. Frames are drawn at the
//! display link's ticks (`display_link`), into a `CAMetalLayer` of each view's own beneath which its
//! scenes place their composition layers (`picture`); what the platform hands the app is the
//! platform's (`platform`), with what macOS shares from `waft-embedder-apple`.
//!
//! Elsewhere the crate still builds, with no host in it, so the workspace's tests cover the
//! reckonings that need no UIKit on any machine.

#[cfg(any(target_os = "ios", test))]
mod keyboard_inset;
#[cfg(any(target_os = "ios", test))]
mod keys;
#[cfg(any(target_os = "ios", test))]
mod lifecycle;
#[cfg(any(target_os = "ios", test))]
mod metrics;

#[cfg(target_os = "ios")]
mod app_delegate;
#[cfg(target_os = "ios")]
mod display_link;
#[cfg(target_os = "ios")]
mod embedder;
#[cfg(target_os = "ios")]
mod file_dialogs;
#[cfg(target_os = "ios")]
mod gpu;
#[cfg(target_os = "ios")]
mod host;
#[cfg(target_os = "ios")]
mod keyboard_notices;
#[cfg(target_os = "ios")]
mod open_requests;
#[cfg(target_os = "ios")]
mod opener;
#[cfg(target_os = "ios")]
mod pasteboard;
#[cfg(target_os = "ios")]
mod picture;
#[cfg(target_os = "ios")]
mod platform;
#[cfg(target_os = "ios")]
mod presses;
#[cfg(target_os = "ios")]
mod scene_delegate;
#[cfg(target_os = "ios")]
mod touches;
#[cfg(target_os = "ios")]
mod ui_view;
#[cfg(target_os = "ios")]
mod view;
#[cfg(target_os = "ios")]
mod view_controller;
#[cfg(target_os = "ios")]
mod views;
#[cfg(target_os = "ios")]
mod vsync_waiter;
#[cfg(target_os = "ios")]
mod wakes;

#[cfg(target_os = "ios")]
pub use embedder::IosEmbedder;
#[cfg(target_os = "ios")]
pub use platform::IosPlatform;
