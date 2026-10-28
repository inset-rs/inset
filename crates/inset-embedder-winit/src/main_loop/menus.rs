//! The main loop's menus: what a menu of the system's tells the app, each on a turn of its own
//! (`menus/`). An edit command goes to the view whose window has the focus, as AppKit sends
//! `copy:` to the key window's first responder; with no such view, it goes nowhere.

use std::rc::Rc;

use inset_embedder::{
    ClipboardEvent, CopyRequest, EditCommand, EmbedderClient, Platform, View, ViewId,
};

use crate::clipboard::listed_now;
use crate::menus::{EditAction, MenuEvent};

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    pub(super) fn on_menu_event(&mut self, event: MenuEvent) {
        match event {
            MenuEvent::Selected(id) => self.client.menu_selected(id),
            MenuEvent::DockSelected(id) => self.client.dock_menu_selected(id),
            MenuEvent::Tray(icon, event) => self.client.tray_icon_event(icon, event),
            MenuEvent::Edit(action) => self.on_edit_action(action),
        }
    }

    /// A provided edit command, for the focused view: cut, copy and paste as the user's own
    /// clipboard commands, the rest as edit commands.
    fn on_edit_action(&mut self, action: EditAction) {
        let Some(view) = self.focused_view() else {
            return;
        };
        match action {
            EditAction::Cut => {
                let request = self.copy_request();
                self.client
                    .clipboard_event(view, ClipboardEvent::Cut(request));
            }
            EditAction::Copy => {
                let request = self.copy_request();
                self.client
                    .clipboard_event(view, ClipboardEvent::Copy(request));
            }
            EditAction::Paste => {
                let items = self
                    .platform
                    .clipboard()
                    .map(listed_now)
                    .unwrap_or_default();
                self.client
                    .clipboard_event(view, ClipboardEvent::Paste(items));
            }
            EditAction::Undo => self.client.edit_command(view, EditCommand::Undo),
            EditAction::Redo => self.client.edit_command(view, EditCommand::Redo),
            EditAction::SelectAll => self.client.edit_command(view, EditCommand::SelectAll),
            EditAction::Delete => self.client.edit_command(view, EditCommand::Delete),
        }
    }

    /// Where the user's copy or cut is written: the system's clipboard.
    fn copy_request(&self) -> CopyRequest {
        let platform = Rc::clone(&self.platform);
        CopyRequest::new(move |items, options| match platform.clipboard() {
            Some(clipboard) => clipboard.write(items, options),
            None => Box::pin(std::future::ready(Err(
                inset_embedder::ClipboardError::Unsupported,
            ))),
        })
    }

    /// The view whose window has the focus, if one has.
    fn focused_view(&self) -> Option<ViewId> {
        self.views()
            .views()
            .into_iter()
            .find(|view| {
                self.views()
                    .by_view(view.id())
                    .is_some_and(|open| open.window.has_focus())
            })
            .map(|view| view.id())
    }
}
