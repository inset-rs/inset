//! Flutter counterpart: `services/browser_context_menu.dart`.
//!
//! On the web the browser's own context menu is toggled through a platform channel. Waft has no
//! such channel yet; this keeps the flag a web-only `SelectableRegion` reads.

use std::sync::atomic::{AtomicBool, Ordering};

use waft_foundation::Task;

/// Whether showing the browser's context menu is enabled.
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Controls the browser's context menu on the web platform.
///
/// The context menu is the menu that appears on right clicking or selecting text in the browser.
/// On all non-web platforms this does nothing.
pub struct BrowserContextMenu;

impl BrowserContextMenu {
    /// Whether showing the browser's context menu is enabled.
    ///
    /// Defaults to true.
    pub fn enabled() -> bool {
        ENABLED.load(Ordering::Relaxed)
    }

    /// Disables the browser's context menu.
    pub fn disable_context_menu() -> Task<()> {
        ENABLED.store(false, Ordering::Relaxed);
        Task::ready(())
    }

    /// Enables the browser's context menu.
    pub fn enable_context_menu() -> Task<()> {
        ENABLED.store(true, Ordering::Relaxed);
        Task::ready(())
    }
}
