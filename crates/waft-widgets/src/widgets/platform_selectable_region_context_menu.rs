//! Flutter counterpart: `widgets/platform_selectable_region_context_menu.dart` and its
//! `_platform_selectable_region_context_menu_io.dart` stub.
//!
//! A widget that provides the web's native selection context menu for its child subtree. The
//! non-web implementation is a stub: every member throws, and nothing names it yet, since the
//! web implementation is deferred (`## Deferred` in `PORTING.md`).

use crate::framework::{BuildContext, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::selection_container::AnySelectionContainerDelegate;
use waft_foundation::App;

/// Function signature for `ui_web.platformViewRegistry.registerViewFactory`.
pub type RegisterViewFactory = fn(String, Box<dyn Fn(i32) -> Box<dyn std::any::Any>>, bool);

/// A widget that provides native selection context menu for its child subtree.
///
/// This widget currently only supports Flutter web. Using this widget on non-web platforms
/// throws [`UnimplementedError`](std::unimplemented).
pub struct PlatformSelectableRegionContextMenu {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl PlatformSelectableRegionContextMenu {
    /// Creates a [`PlatformSelectableRegionContextMenu`].
    pub fn new<K>(child: impl crate::framework::IntoWidget<K>) -> Self {
        PlatformSelectableRegionContextMenu {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Attaches the client to the platform-appropriate context menus.
    pub fn attach(_client: AnySelectionContainerDelegate) {
        unimplemented!()
    }

    /// Detaches the client from the platform-appropriate selection context menus.
    pub fn detach(_client: AnySelectionContainerDelegate) {
        unimplemented!()
    }

    /// The client currently attached to the platform context menu.
    pub fn debug_active_client() -> Option<AnySelectionContainerDelegate> {
        unimplemented!()
    }

    /// Resets the view factory registration to its initial state.
    pub fn debug_reset_registry() {
        unimplemented!()
    }
}

impl std::fmt::Debug for PlatformSelectableRegionContextMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformSelectableRegionContextMenu")
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for PlatformSelectableRegionContextMenu {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        unimplemented!()
    }
}
