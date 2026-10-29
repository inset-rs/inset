//! What iOS reports differently from the desktop winit was written for: its window is the
//! screen, and winit's size for it is the safe area rather than the whole layer; and what it adds,
//! its pasteboard (`pasteboard`), its document pickers (`file_dialogs`), opening (`opener`), what
//! it asks the app to open (`open_requests`) and native surfaces (`composition`). Notifications are
//! the center macOS shares (`apple/notifications`).

mod composition;
mod file_dialogs;
mod open_requests;
mod opener;
mod pasteboard;

use std::rc::Rc;

use inset_embedder::{Notifications, TargetPlatform, ViewPadding};
use valo_codec::Decoder;
use winit::window::Window;

use pasteboard::Pasteboard;

use super::absent::{NoDisplayLink, NoVisibilityNotices};
use super::apple::{image_io_decoders, notifications};
use super::{
    ApplicationControl, ApplicationLifecycle, FramePresentation, ImageDecoding, NativeMenus,
    OperatingSystem, PointerQueries, Screens, SystemChanges, SystemClipboard, SystemNotifications,
    WindowChrome, WindowGeometry, WindowMotion, WindowVisibility, WorkQueue,
};
use crate::notifications::NotificationActivations;

/// iOS. A type only: never made, its answers are associated functions.
pub(crate) enum Ios {}

impl OperatingSystem for Ios {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::IOS;

    /// The system sizes a phone's window; winit on iOS would size it to a request instead.
    const WINDOW_IS_THE_SCREEN: bool = true;

    /// The screen view's window is the only one, as on an iPhone.
    const APP_MAY_OPEN_WINDOWS: bool = false;

    const APP_MAY_QUIT: bool = false;
}

impl WindowGeometry for Ios {
    /// The whole window: winit's `inner_size` is the safe area there, while the layer covers
    /// the whole window, which `outer_size` reports.
    fn surface_size(window: &Window) -> [u32; 2] {
        let size = window.outer_size();
        [size.width, size.height]
    }

    /// The status bar and the home indicator: what lies between the window and the safe
    /// area winit reports as its inner size and position.
    fn view_padding(window: &Window, [width, height]: [f64; 2]) -> ViewPadding {
        let safe = window.inner_size();
        let origin = window.inner_position().unwrap_or_default();
        let left = f64::from(origin.x);
        let top = f64::from(origin.y);
        ViewPadding {
            left,
            top,
            right: (width - f64::from(safe.width) - left).max(0.0),
            bottom: (height - f64::from(safe.height) - top).max(0.0),
        }
    }
}

/// The general pasteboard, which UIKit says changed while the app is active, and which the main
/// loop looks at again as the app becomes active.
impl SystemClipboard for Ios {
    type Clipboard = Pasteboard;

    fn clipboard(look: Box<dyn Fn()>) -> Pasteboard {
        Pasteboard::general(look)
    }
}

impl FramePresentation for Ios {
    type DisplayLink = NoDisplayLink;
}

impl WindowVisibility for Ios {
    type Notices = NoVisibilityNotices;
}

impl ImageDecoding for Ios {
    fn system_decoders() -> Vec<Box<dyn Decoder>> {
        image_io_decoders()
    }
}

/// `UNUserNotificationCenter`'s (`apple/notifications`).
impl SystemNotifications for Ios {
    fn hear_notification_activations(activations: NotificationActivations) {
        notifications::hear_activations(activations);
    }

    fn notifications(_activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        notifications::notifications()
    }
}

impl WindowMotion for Ios {}
impl WindowChrome for Ios {}
impl Screens for Ios {}
impl PointerQueries for Ios {}
impl NativeMenus for Ios {}
impl WorkQueue for Ios {}
impl SystemChanges for Ios {}
impl ApplicationControl for Ios {}
impl ApplicationLifecycle for Ios {}
