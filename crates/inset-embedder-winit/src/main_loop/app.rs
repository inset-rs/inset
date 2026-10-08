//! What the system says of the application as a whole and asks of it: its lifecycle, its
//! appearance, the files it is asked to open and the reopen.

use inset_embedder::EmbedderClient;
use winit::window::{Theme, Window};

use crate::lifecycle::LifecycleNotice;
use crate::system_requests::SystemRequest;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// The system's requests of the application: files and reopens go to the handlers the
    /// app set, and a quit to the framework.
    pub(super) fn on_system_request(&mut self, request: SystemRequest) {
        match request {
            SystemRequest::OpenFiles(files) => self.state.handlers().open_files(files),
            SystemRequest::Reopen(reopen) => self.state.handlers().reopen(reopen),
            SystemRequest::Quit => self.on_quit_requested(),
        }
    }

    /// The app set its open-files handler while files were waiting for one: they reach it now,
    /// outside every framework call.
    pub(super) fn hand_over_files(&mut self) {
        self.state.handlers().hand_over_files();
    }

    /// The system said the application became active or stopped being so, or became visible or
    /// stopped being so: the framework hears the lifecycle state it is in now.
    pub(super) fn on_lifecycle_notice(&mut self, notice: LifecycleNotice) {
        let state = self.lifecycle.state_after(notice);
        self.client.app_lifecycle_state_changed(state);
    }

    /// The system's appearance, read again from `window`'s theme: a window just opened brings
    /// it, and a system whose configuration carries it reports a change as a resize.
    pub(super) fn read_theme_of(&mut self, window: &Window) {
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
}
