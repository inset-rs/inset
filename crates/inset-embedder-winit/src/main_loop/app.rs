//! What the system says of the application as a whole and asks of it: its lifecycle, its
//! appearance, the files and links it is asked to open and the reopen; and what it says changed about the
//! machine: the screens and the user's languages.

use std::time::Instant;

use inset_embedder::{EmbedderClient, OpenRequest, PickedFile};
use winit::window::{Theme, Window};

use crate::lifecycle::LifecycleNotice;
use crate::system_changes::SystemChange;
use crate::system_requests::SystemRequest;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// The system's requests of the application: files and links to open go to the client, a
    /// reopen to the handler the app set, and a quit to the framework.
    pub(super) fn on_system_request(&mut self, request: SystemRequest) {
        match request {
            SystemRequest::Open { files, links } => self.client.open_requested(OpenRequest {
                files: files.into_iter().map(PickedFile::Path).collect(),
                links,
            }),
            SystemRequest::Reopen(reopen) => self.state.handlers().reopen(reopen),
            SystemRequest::NotificationActivated(activation) => {
                self.client.notification_activated(activation);
            }
            SystemRequest::Quit => self.on_quit_requested(),
        }
    }

    /// The system said the application became active or stopped being so, or became visible or
    /// stopped being so: the framework hears the lifecycle state it is in now.
    pub(super) fn on_lifecycle_notice(&mut self, notice: LifecycleNotice) {
        let state = self.lifecycle.state_after(notice);
        self.client.app_lifecycle_state_changed(state);
        if let LifecycleNotice::WillBecomeActive { .. } = notice {
            self.look_at_clipboard();
        }
    }

    /// The app hears the clipboard changed, if another writer changed it since the loop last
    /// looked: as the app becomes active and as one of its windows takes the focus, since no
    /// system here tells an application that another one wrote it; and once a copy of the app's
    /// expired and was cleared (`clipboard/`).
    pub(super) fn look_at_clipboard(&mut self) {
        if self.platform.clipboard_has_changed() {
            self.client.clipboard_changed();
        }
    }

    /// Clears the app's copy whose time has come by `now`, where the main loop keeps the expiry,
    /// and the app hears the clipboard changed.
    pub(super) fn clear_expired_copy(&mut self, now: Instant) {
        if self.platform.clear_expired_clipboard(now) {
            self.client.clipboard_changed();
        }
    }

    /// The system's appearance, read again from `window`'s theme: a window just opened brings
    /// it, and a system whose configuration carries it reports a change as a resize.
    pub(super) fn read_theme_of(&mut self, window: &dyn Window) {
        if let Some(theme) = window.theme() {
            self.on_theme_changed(theme);
        }
    }

    /// The system's theme is `theme`: the app hears of it when it changed.
    pub(super) fn on_theme_changed(&mut self, theme: Theme) {
        if self.state.appearance().take(theme) {
            self.client.platform_brightness_changed();
        }
    }

    /// The system said the screens or the user's languages changed: the app hears of it, as
    /// Flutter's macOS embedder tells the engine's displays and locales.
    pub(super) fn on_system_change(&mut self, change: SystemChange) {
        match change {
            SystemChange::Screens => self.on_screens_changed(),
            SystemChange::Locales => self.on_locales_changed(),
        }
    }

    /// The app hears the screens changed, and then of each view whose window is on another
    /// screen now, as a screen it was on can be gone, or have become the mirror of another.
    fn on_screens_changed(&mut self) {
        self.client.screens_changed();
        self.sync_screens();
    }

    /// The user's languages, read again: the app hears of them when they differ from what it
    /// was told.
    fn on_locales_changed(&mut self) {
        if self.state.locales().read_again() {
            self.client.locales_changed();
        }
    }
}
