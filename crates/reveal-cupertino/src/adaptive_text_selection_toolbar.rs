//! Flutter counterpart: `cupertino/adaptive_text_selection_toolbar.dart`.
//!
//! [`selectable`](CupertinoAdaptiveTextSelectionToolbar::selectable) waits on `SelectableRegion`.

use std::fmt::{self, Debug};

use reveal_embedder::TargetPlatform;
use reveal_foundation::{App, Handle, Listener};
use reveal_widgets::{
    BuildContext, ClipboardStatus, ContextMenuButtonItem, EditableText, EditableTextState,
    IntoWidget, KeyRef, SizedBox, StatelessWidget, TextSelectionToolbarAnchors, WidgetRef,
};

use crate::desktop_text_selection_toolbar::CupertinoDesktopTextSelectionToolbar;
use crate::desktop_text_selection_toolbar_button::CupertinoDesktopTextSelectionToolbarButton;
use crate::text_selection_toolbar::CupertinoTextSelectionToolbar;
use crate::text_selection_toolbar_button::CupertinoTextSelectionToolbarButton;

/// The default Cupertino context menu for text selection for the current platform with the
/// given children.
///
/// Builds the mobile Cupertino context menu on all mobile platforms, not just iOS, and builds
/// the desktop Cupertino context menu on all desktop platforms, not just MacOS.
///
/// See also:
///
/// * [`CupertinoAdaptiveTextSelectionToolbar::get_adaptive_buttons`], which builds the
///   Cupertino button Widgets for the current platform given [`ContextMenuButtonItem`]s.
pub struct CupertinoAdaptiveTextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// The children of the toolbar, typically buttons.
    pub children: Option<Vec<WidgetRef>>,
    /// The [`ContextMenuButtonItem`]s that will be turned into the correct button widgets
    /// for the current platform.
    pub button_items: Option<Vec<ContextMenuButtonItem>>,
    /// The location that the toolbar should attempt to position itself at.
    pub anchors: TextSelectionToolbarAnchors,
}

impl Debug for CupertinoAdaptiveTextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoAdaptiveTextSelectionToolbar")
            .finish_non_exhaustive()
    }
}

impl CupertinoAdaptiveTextSelectionToolbar {
    /// Create an instance of [`CupertinoAdaptiveTextSelectionToolbar`] with the given
    /// [`children`](Self::children).
    pub fn new(
        children: impl IntoIterator<Item = WidgetRef>,
        anchors: TextSelectionToolbarAnchors,
    ) -> CupertinoAdaptiveTextSelectionToolbar {
        CupertinoAdaptiveTextSelectionToolbar {
            key: None,
            children: Some(children.into_iter().collect()),
            button_items: None,
            anchors,
        }
    }

    /// Dart `CupertinoAdaptiveTextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoAdaptiveTextSelectionToolbar {
        self.key = Some(key);
        self
    }

    /// Create an instance of [`CupertinoAdaptiveTextSelectionToolbar`] whose children will be
    /// built from the given [`button_items`](Self::button_items).
    pub fn button_items(
        button_items: Vec<ContextMenuButtonItem>,
        anchors: TextSelectionToolbarAnchors,
    ) -> CupertinoAdaptiveTextSelectionToolbar {
        CupertinoAdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(button_items),
            anchors,
        }
    }

    /// Create an instance of [`CupertinoAdaptiveTextSelectionToolbar`] with the default children
    /// for an editable field.
    #[allow(clippy::too_many_arguments)]
    pub fn editable(
        clipboard_status: ClipboardStatus,
        on_copy: Option<Listener>,
        on_cut: Option<Listener>,
        on_paste: Option<Listener>,
        on_select_all: Option<Listener>,
        on_look_up: Option<Listener>,
        on_search_web: Option<Listener>,
        on_share: Option<Listener>,
        on_live_text_input: Option<Listener>,
        anchors: TextSelectionToolbarAnchors,
        target_platform: TargetPlatform,
    ) -> CupertinoAdaptiveTextSelectionToolbar {
        CupertinoAdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(EditableText::get_editable_button_items(
                Some(clipboard_status),
                on_copy,
                on_cut,
                on_paste,
                on_select_all,
                on_look_up,
                on_search_web,
                on_share,
                on_live_text_input,
                target_platform,
            )),
            anchors,
        }
    }

    /// Create an instance of [`CupertinoAdaptiveTextSelectionToolbar`] with the default children
    /// for an [`EditableText`].
    pub fn editable_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> CupertinoAdaptiveTextSelectionToolbar {
        CupertinoAdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(editable_text_state.context_menu_button_items(app)),
            anchors: editable_text_state.context_menu_anchors(app),
        }
    }

    /// Returns a List of Widgets generated by turning [`button_items`] into the default context
    /// menu buttons for Cupertino on the current platform.
    pub fn get_adaptive_buttons(
        app: &mut App,
        _context: BuildContext,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> Vec<WidgetRef> {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => button_items
                .into_iter()
                .map(|button_item| {
                    CupertinoTextSelectionToolbarButton::button_item(button_item).into_widget()
                })
                .collect(),
            TargetPlatform::Linux | TargetPlatform::Windows | TargetPlatform::MacOS => button_items
                .into_iter()
                .map(|button_item| {
                    CupertinoDesktopTextSelectionToolbarButton::button_item(button_item)
                        .into_widget()
                })
                .collect(),
        }
    }
}

impl StatelessWidget for CupertinoAdaptiveTextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let empty = match (&self.children, &self.button_items) {
            (Some(children), _) => children.is_empty(),
            (None, Some(items)) => items.is_empty(),
            (None, None) => true,
        };
        if empty {
            return SizedBox::shrink().into_widget();
        }

        let result_children = match &self.children {
            Some(children) => children.clone(),
            None => Self::get_adaptive_buttons(
                app,
                context,
                self.button_items.clone().expect("children or buttonItems"),
            ),
        };

        if result_children.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {
                CupertinoTextSelectionToolbar::new(
                    self.anchors.primary_anchor,
                    self.anchors
                        .secondary_anchor
                        .unwrap_or(self.anchors.primary_anchor),
                    result_children,
                )
                .into_widget()
            }
            TargetPlatform::Linux | TargetPlatform::Windows | TargetPlatform::MacOS => {
                CupertinoDesktopTextSelectionToolbar::new(
                    self.anchors.primary_anchor,
                    result_children,
                )
                .into_widget()
            }
        }
    }
}
