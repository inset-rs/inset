//! Dropping a file dialog's future before the user answers closes its panel: the sheet the open
//! panel put on the view's window is cancelled and goes away, rather than staying on screen for an
//! app that stopped waiting.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window and a sheet, drops the future, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_dropped_file_dialog_closes_its_panel";

/// Long enough for the panel to show and be cancelled, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

#[cfg(target_os = "macos")]
fn main() {
    use waft_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the panel did not close within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default().run(client::ClosingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use waft_embedder::{
        EmbedderClient, FileDialogFuture, Frame, KeyData, OpenOptions, PickedFile, PlatformRef,
        PointerReport, ViewId,
    };

    use crate::TEST;
    use crate::support::{self, TICK, appkit};

    const TITLE: &str = "a dialog's window";

    /// How long the panel is left up before the app stops waiting for it.
    const SHOW_FOR: Duration = Duration::from_millis(300);

    /// How long the panel may stay after its future is dropped.
    const PATIENCE: Duration = Duration::from_secs(5);

    /// What the app has done so far.
    enum Step {
        /// Waiting for the panel to show as a sheet.
        Showing { asked_at: Instant },
        /// The future was dropped at this moment; the panel should go.
        Dropped { at: Instant },
    }

    /// Opens an open panel on its window's sheet, drops its future before choosing, and checks the
    /// panel went with it.
    pub(crate) struct ClosingClient {
        platform: PlatformRef,
        step: Step,
        /// The dialog's future, until the app stops waiting for it.
        dialog: Option<FileDialogFuture<Option<PickedFile>>>,
    }

    impl ClosingClient {
        pub(crate) fn new(platform: PlatformRef) -> ClosingClient {
            support::request_window(&platform, support::titled(TITLE));
            ClosingClient {
                platform,
                step: Step::Showing {
                    asked_at: Instant::now(),
                },
                dialog: None,
            }
        }

        fn wake_after(&self, wait: Duration) {
            self.platform
                .dispatcher()
                .wake_at(Some(Instant::now() + wait));
        }

        /// Drops the future once the panel showed, and passes once it went.
        fn close_once_shown(&mut self, asked_at: Instant) {
            if asked_at.elapsed() < SHOW_FOR {
                self.wake_after(SHOW_FOR - asked_at.elapsed());
                return;
            }
            if !appkit::shows_a_sheet(TITLE) {
                support::fail(TEST, "the panel never showed as a sheet");
            }
            // The drop under test: an app that stops waiting for the dialog.
            self.dialog = None;
            self.step = Step::Dropped { at: Instant::now() };
            self.wake_after(TICK);
        }

        fn pass_once_gone(&self, dropped_at: Instant) {
            if !appkit::shows_a_sheet(TITLE) {
                support::pass(TEST);
            }
            if dropped_at.elapsed() > PATIENCE {
                support::fail(TEST, "the panel stayed after its future was dropped");
            }
            self.wake_after(TICK);
        }
    }

    impl EmbedderClient for ClosingClient {
        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}

        fn wake(&mut self, _elapsed: Duration) {
            match self.step {
                Step::Showing { asked_at } => self.close_once_shown(asked_at),
                Step::Dropped { at } => self.pass_once_gone(at),
            }
        }

        fn view_added(&mut self, id: ViewId) {
            let dialogs = self
                .platform
                .file_dialogs()
                .expect("macOS has file dialogs");
            self.dialog = Some(dialogs.open_file(Some(id), OpenOptions::default()));
            self.wake_after(SHOW_FOR);
        }

        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_reports(&mut self, _reports: Vec<PointerReport>) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}
