//! A view's window as the app holds it ([`HostWindow`]): on Android, the task of the view's
//! activity. Its title and icon are the task's on its card in Recents (`task`), and closing it ends
//! the task, which destroys the activity and so removes the view.
//!
//! Android places a task's window and orders the tasks itself, at the user's word, so the window's
//! frame, showing and hiding take nothing from the app; and `NativeActivity` tells nothing of
//! whether a task is out of sight, so a window is never minimized or occluded.

use std::rc::Rc;
use std::time::Duration;

use android_activity::AndroidApp;
use inset_embedder::{HostWindow, IconImage, RawWindowHandle, Rect, View, ViewRef};
use raw_window_handle::HasWindowHandle;

use crate::task::TaskDescriptions;
use crate::view::AndroidView;

/// The task around a view: the view's activity's.
pub(crate) struct AndroidWindow {
    view: Rc<AndroidView>,
    activity: AndroidApp,
    descriptions: TaskDescriptions,
}

impl AndroidWindow {
    pub(crate) fn new(
        view: Rc<AndroidView>,
        activity: AndroidApp,
        descriptions: TaskDescriptions,
    ) -> AndroidWindow {
        AndroidWindow {
            view,
            activity,
            descriptions,
        }
    }

    pub(crate) fn android_view(&self) -> &Rc<AndroidView> {
        &self.view
    }
}

impl HostWindow for AndroidWindow {
    fn view(&self) -> ViewRef {
        Rc::clone(&self.view) as ViewRef
    }

    /// The window's size at the screen's origin: `NativeActivity` tells nothing of where its
    /// window is.
    fn frame(&self) -> Rect {
        let metrics = self.view.metrics();
        let [width, height] = metrics.physical_size;
        let ratio = metrics.device_pixel_ratio;
        Rect::from_ltwh(0.0, 0.0, width / ratio, height / ratio)
    }

    fn set_frame(&self, _frame: Rect, _animate: Option<Duration>) {}

    /// The label on the task's card in Recents.
    fn set_title(&self, title: &str) {
        self.descriptions.set_title(self.view.id(), title);
    }

    /// The icon on the task's card in Recents.
    fn set_icon(&self, icon: Option<IconImage>) {
        self.descriptions.set_icon(self.view.id(), icon);
    }

    fn show(&self) {}

    fn hide(&self) {}

    fn is_minimized(&self) -> bool {
        false
    }

    fn is_occluded(&self) -> bool {
        false
    }

    /// Ends the task, its card in Recents with it (`finishAndRemoveTask`).
    fn close(&self) {
        crate::task::finish_and_remove(&self.activity);
    }

    /// The activity's native window, while the system gives it one.
    fn native_handle(&self) -> Option<RawWindowHandle> {
        let window = self.activity.native_window()?;
        window.window_handle().ok().map(|handle| handle.as_raw())
    }
}
