//! Flutter counterpart: `widgets/system_context_menu.dart`.

use std::sync::atomic::{AtomicU64, Ordering};

use reveal_embedder::{Rect, TargetPlatform};
use reveal_foundation::{App, Handle, Listener};
use reveal_rendering::RenderBox;
use reveal_services::TextSelectionDelegate;
use reveal_services::{IOSSystemContextMenuItemData, SystemContextMenuController};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::SizedBox;
use crate::widgets::context_menu_button_item::ContextMenuButtonType;
use crate::widgets::editable_text::EditableTextState;
use crate::widgets::localizations::WidgetsLocalizations;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::text_selection_toolbar_anchors::TextSelectionToolbarAnchors;

static NEXT_CUSTOM_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

/// Displays the system context menu on top of the Flutter view.
///
/// Currently, only supports iOS 16.0 and above and displays nothing on other
/// platforms.
///
/// The context menu is the menu that appears, for example, when doing text
/// selection. Flutter typically draws this menu itself, but this class deals
/// with the platform-rendered context menu instead.
#[derive(Debug)]
pub struct SystemContextMenu {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,
    /// The [`Rect`] that the context menu should point to.
    pub anchor: Rect,
    /// A list of the items to be displayed in the system context menu.
    pub items: Vec<IOSSystemContextMenuItem>,
    /// Called when the system hides this context menu.
    pub on_system_hide: Option<Listener>,
}

impl SystemContextMenu {
    fn from_fields(
        anchor: Rect,
        items: Vec<IOSSystemContextMenuItem>,
        on_system_hide: Option<Listener>,
    ) -> SystemContextMenu {
        SystemContextMenu {
            key: None,
            anchor,
            items,
            on_system_hide,
        }
    }

    /// Creates an instance of [`SystemContextMenu`] for the field indicated by the
    /// given [`EditableTextState`].
    pub fn editable_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> SystemContextMenu {
        let (start_glyph_height, end_glyph_height) = editable_text_state.get_glyph_heights(app);
        let render_editable = editable_text_state.render_editable(app);
        let selection = editable_text_state.text_editing_value(app).selection;
        let endpoints = render_editable.get_endpoints_for_selection(app, selection);
        let anchor = TextSelectionToolbarAnchors::get_selection_rect(
            app,
            render_editable.as_box(),
            start_glyph_height,
            end_glyph_height,
            &endpoints,
        );
        let items = Self::get_default_items(app, editable_text_state);
        SystemContextMenu::from_fields(
            anchor,
            items,
            Some(Listener::new(move |app| {
                TextSelectionDelegate::hide_toolbar(editable_text_state, app, false);
            })),
        )
    }

    /// Dart `SystemContextMenu.editableText(items:)`.
    pub fn items(mut self, items: Vec<IOSSystemContextMenuItem>) -> SystemContextMenu {
        self.items = items;
        self
    }

    /// Dart `SystemContextMenu(key:)`.
    pub fn key(mut self, key: KeyRef) -> SystemContextMenu {
        self.key = Some(key);
        self
    }

    /// Whether the current device supports showing the system context menu.
    ///
    /// Currently, this is only supported on newer versions of iOS.
    pub fn is_supported(app: &mut App, context: BuildContext) -> bool {
        app.platform().target_platform() == TargetPlatform::IOS
            && MediaQuery::maybe_supports_showing_system_context_menu(app, context).unwrap_or(false)
    }

    /// Whether the given field supports showing the system context menu.
    pub fn is_supported_by_field(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> bool {
        let read_only = editable_text_state.widget(app).read_only;
        let context = editable_text_state.context(app);
        !read_only && Self::is_supported(app, context)
    }

    /// The default [`items`](Self::items) for the given [`EditableTextState`].
    pub fn get_default_items(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> Vec<IOSSystemContextMenuItem> {
        let mut items = Vec::new();
        for button in editable_text_state.context_menu_button_items(app) {
            match button.r#type {
                ContextMenuButtonType::Copy => items.push(IOSSystemContextMenuItem::Copy),
                ContextMenuButtonType::Cut => items.push(IOSSystemContextMenuItem::Cut),
                ContextMenuButtonType::Paste => items.push(IOSSystemContextMenuItem::Paste),
                ContextMenuButtonType::SelectAll => items.push(IOSSystemContextMenuItem::SelectAll),
                ContextMenuButtonType::LookUp => {
                    items.push(IOSSystemContextMenuItem::LookUp { title: None })
                }
                ContextMenuButtonType::SearchWeb => {
                    items.push(IOSSystemContextMenuItem::SearchWeb { title: None })
                }
                ContextMenuButtonType::Share => {
                    items.push(IOSSystemContextMenuItem::Share { title: None })
                }
                ContextMenuButtonType::LiveTextInput => {
                    items.push(IOSSystemContextMenuItem::LiveText)
                }
                ContextMenuButtonType::Delete | ContextMenuButtonType::Custom => {}
            }
        }
        items
    }
}

impl StatefulWidget for SystemContextMenu {
    type State = SystemContextMenuState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SystemContextMenuState {
        SystemContextMenuState {
            state: StateData::new(),
            controller: None,
        }
    }
}

/// State for [`SystemContextMenu`].
pub struct SystemContextMenuState {
    state: StateData<SystemContextMenu>,
    controller: Option<Handle<SystemContextMenuController>>,
}

impl State for SystemContextMenuState {
    type Widget = SystemContextMenu;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let on_hide = self.widget(app).on_system_hide.clone();
        let controller = SystemContextMenuController::new(app, on_hide);
        app.get_mut(self).controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).controller {
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(SystemContextMenu::is_supported(app, context));
        let items = self.widget(app).items.clone();
        let anchor = self.widget(app).anchor;
        if !items.is_empty() {
            let localizations = <dyn WidgetsLocalizations>::of(app, context);
            let item_datas: Vec<IOSSystemContextMenuItemData> = items
                .iter()
                .map(|item| item.get_data(localizations.as_ref()))
                .collect();
            let controller = app.get(self).controller.expect("created in initState");
            controller.show_with_items(app, anchor, item_datas);
        }
        SizedBox::shrink().into_widget()
    }
}

/// Describes a context menu button that will be rendered in the iOS system
/// context menu and not by Flutter itself.
///
/// Dart's sealed `IOSSystemContextMenuItem` subclasses are the variants.
#[derive(Clone, Debug)]
pub enum IOSSystemContextMenuItem {
    /// The system's built-in copy button.
    Copy,
    /// The system's built-in cut button.
    Cut,
    /// The system's built-in paste button.
    Paste,
    /// The system's built-in select-all button.
    SelectAll,
    /// The system's built-in look-up button.
    LookUp {
        /// Optional title; defaults to [`WidgetsLocalizations::look_up_button_label`].
        title: Option<String>,
    },
    /// The system's built-in search-web button.
    SearchWeb {
        /// Optional title; defaults to [`WidgetsLocalizations::search_web_button_label`].
        title: Option<String>,
    },
    /// The system's built-in share button.
    Share {
        /// Optional title; defaults to [`WidgetsLocalizations::share_button_label`].
        title: Option<String>,
    },
    /// The system's built-in Live Text button.
    LiveText,
    /// A developer-defined button.
    Custom {
        /// Title shown in the menu.
        title: String,
        /// Callback when the item is selected.
        on_pressed: Listener,
    },
}

impl IOSSystemContextMenuItem {
    /// Returns the representation of this class used by the host.
    pub fn get_data(
        &self,
        localizations: &dyn WidgetsLocalizations,
    ) -> IOSSystemContextMenuItemData {
        match self {
            Self::Copy => IOSSystemContextMenuItemData::Copy,
            Self::Cut => IOSSystemContextMenuItemData::Cut,
            Self::Paste => IOSSystemContextMenuItemData::Paste,
            Self::SelectAll => IOSSystemContextMenuItemData::SelectAll,
            Self::LookUp { title } => IOSSystemContextMenuItemData::LookUp {
                title: title
                    .clone()
                    .unwrap_or_else(|| localizations.look_up_button_label()),
            },
            Self::SearchWeb { title } => IOSSystemContextMenuItemData::SearchWeb {
                title: title
                    .clone()
                    .unwrap_or_else(|| localizations.search_web_button_label()),
            },
            Self::Share { title } => IOSSystemContextMenuItemData::Share {
                title: title
                    .clone()
                    .unwrap_or_else(|| localizations.share_button_label()),
            },
            Self::LiveText => IOSSystemContextMenuItemData::LiveText,
            Self::Custom { title, on_pressed } => IOSSystemContextMenuItemData::Custom {
                title: title.clone(),
                on_pressed: on_pressed.clone(),
                callback_id: NEXT_CUSTOM_CALLBACK_ID
                    .fetch_add(1, Ordering::Relaxed)
                    .to_string(),
            },
        }
    }
}

impl PartialEq for IOSSystemContextMenuItem {
    fn eq(&self, other: &IOSSystemContextMenuItem) -> bool {
        match (self, other) {
            (Self::Copy, Self::Copy)
            | (Self::Cut, Self::Cut)
            | (Self::Paste, Self::Paste)
            | (Self::SelectAll, Self::SelectAll)
            | (Self::LiveText, Self::LiveText) => true,
            (Self::LookUp { title: a }, Self::LookUp { title: b })
            | (Self::SearchWeb { title: a }, Self::SearchWeb { title: b })
            | (Self::Share { title: a }, Self::Share { title: b }) => a == b,
            (
                Self::Custom {
                    title: a,
                    on_pressed: a_cb,
                },
                Self::Custom {
                    title: b,
                    on_pressed: b_cb,
                },
            ) => a == b && a_cb == b_cb,
            _ => false,
        }
    }
}
