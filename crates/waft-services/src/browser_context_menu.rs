//! Flutter counterpart: `services/browser_context_menu.dart`.

use waft_foundation::{App, Handle, K_IS_WEB, Task};

/// Controls the browser's context menu on the web platform.
///
/// The context menu is the menu that appears on right clicking or selecting text in the browser,
/// for example.
///
/// On web, by default, the browser's context menu is enabled and Flutter's context menus are
/// hidden.
///
/// On all non-web platforms, this does nothing.
pub struct BrowserContextMenu {
    enabled: bool,
}

impl Default for BrowserContextMenu {
    fn default() -> BrowserContextMenu {
        BrowserContextMenu { enabled: true }
    }
}

impl BrowserContextMenu {
    fn instance(app: &mut App) -> Handle<BrowserContextMenu> {
        app.singleton::<BrowserContextMenu>()
    }

    /// Whether showing the browser's context menu is enabled.
    ///
    /// When true, any event that the browser typically uses to trigger its context menu (e.g.
    /// right click) will do so. When false, the browser's context menu will not show.
    ///
    /// It's possible for this to be true but for the browser's context menu to not show due
    /// to direct manipulation of the DOM. For example, handlers for the browser's `contextmenu`
    /// event could be added/removed in the browser's JavaScript console, and this boolean
    /// wouldn't know about it. This boolean only indicates the results of calling
    /// [`disable_context_menu`](Self::disable_context_menu) and
    /// [`enable_context_menu`](Self::enable_context_menu) here.
    ///
    /// Defaults to true.
    pub fn enabled(app: &mut App) -> bool {
        let instance = Self::instance(app);
        app.get(instance).enabled
    }

    /// Disable the browser's context menu.
    ///
    /// By default, when the app starts, the browser's context menu is already enabled.
    pub fn disable_context_menu(app: &mut App) -> Task<()> {
        debug_assert!(K_IS_WEB, "This has no effect on platforms other than web.");
        let instance = Self::instance(app);
        app.get_mut(instance).enabled = false;
        Task::ready(())
    }

    /// Enable the browser's context menu.
    ///
    /// By default, when the app starts, the browser's context menu is already enabled.
    /// Typically this method would be called after first calling
    /// [`disable_context_menu`](Self::disable_context_menu).
    pub fn enable_context_menu(app: &mut App) -> Task<()> {
        debug_assert!(K_IS_WEB, "This has no effect on platforms other than web.");
        let instance = Self::instance(app);
        app.get_mut(instance).enabled = true;
        Task::ready(())
    }
}
