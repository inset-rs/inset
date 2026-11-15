//! What iOS reports differently from the desktop winit was written for: its window is the screen,
//! which the system covers in part with its bars; and what it adds, its pasteboard (`pasteboard`),
//! its document pickers (`file_dialogs`), opening (`opener`), what it asks the app to open
//! (`open_requests`) and composition layers (`composition`). Notifications are the center macOS
//! shares (`waft_embedder_apple::notifications`), and players and cameras its AVFoundation
//! (`waft_embedder_apple::media`).

mod composition;
mod file_dialogs;
mod open_requests;
mod opener;
mod pasteboard;
mod window_composition;

use std::rc::Rc;
use std::sync::Arc;

use valo_codec::Decoder;
use waft_embedder::image_codecs::ImageCodecs;
use waft_embedder::{
    Cameras, CaptionSettings, HardwareBuffer, MediaAssets, Notifications, Players, TargetPlatform,
    VideoFrameReaders, ViewPadding,
};
use winit::window::Window;

use pasteboard::Pasteboard;

use super::absent::{NoDisplayLink, NoTrackpadTouchObserver, NoVisibilityNotices};
use super::capabilities::{
    ApplicationControl, ApplicationIdentity, ApplicationLifecycle, EventLoopSetup,
    FramePresentation, GpuInterop, ImageDecoding, NativeMenus, OperatingSystem, PointerQueries,
    Screens, StandardStreams, SystemChanges, SystemClipboard, SystemDrags, SystemImageCodecs,
    SystemMedia, SystemNotifications, TrackpadTouches, WindowChrome, WindowGeometry, WindowMotion,
    WindowVisibility, WorkQueue,
};
use crate::notifications::NotificationActivations;
use waft_embedder_apple::{image_io_decoders, notifications};

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
    /// The status bar and the home indicator: the insets of the safe area winit reports.
    fn view_padding(window: &dyn Window) -> ViewPadding {
        let safe = window.safe_area();
        ViewPadding {
            left: f64::from(safe.left),
            top: f64::from(safe.top),
            right: f64::from(safe.right),
            bottom: f64::from(safe.bottom),
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

impl TrackpadTouches for Ios {
    type Observer = NoTrackpadTouchObserver;
}

/// AVFoundation's players and cameras, and Media Accessibility's caption settings, which macOS
/// shares.
impl SystemMedia for Ios {
    fn players() -> Option<Rc<dyn Players>> {
        waft_embedder_apple::media::players()
    }

    fn media_assets() -> Option<Rc<dyn MediaAssets>> {
        waft_embedder_apple::media::media_assets()
    }

    fn cameras() -> Option<Rc<dyn Cameras>> {
        waft_embedder_apple::media::cameras()
    }

    fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
        waft_embedder_apple::media::video_frame_readers()
    }

    /// Media Accessibility's.
    fn caption_settings() -> Option<Rc<dyn CaptionSettings>> {
        waft_embedder_apple::caption_appearance::caption_settings()
    }
}

/// Image I/O's, which macOS shares.
impl SystemImageCodecs for Ios {
    fn image_codecs() -> Option<Rc<dyn ImageCodecs>> {
        waft_embedder_apple::image_codecs::image_codecs()
    }
}

/// Metal's texture over the buffer's `IOSurface`, which macOS shares (`waft-embedder-apple`'s `gpu_interop`).
impl GpuInterop for Ios {
    fn import_texture(buffer: &dyn HardwareBuffer, device: &wgpu::Device) -> Option<wgpu::Texture> {
        waft_embedder_apple::gpu_interop::import_texture(buffer, device)
    }
}

impl ImageDecoding for Ios {
    fn system_decoders() -> Vec<Box<dyn Decoder>> {
        image_io_decoders()
    }
}

/// `UNUserNotificationCenter`'s (`waft-embedder-apple`'s `notifications`).
impl SystemNotifications for Ios {
    fn hear_notification_activations(activations: NotificationActivations) {
        notifications::hear_activations(Arc::new(move |activation| activations.send(activation)));
    }

    fn notifications(_activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        notifications::notifications()
    }
}

impl EventLoopSetup for Ios {}
impl WindowMotion for Ios {}
impl WindowChrome for Ios {}
impl SystemDrags for Ios {}
impl Screens for Ios {}
impl PointerQueries for Ios {}
impl NativeMenus for Ios {}
impl WorkQueue for Ios {}
impl SystemChanges for Ios {}
impl ApplicationControl for Ios {}
impl ApplicationLifecycle for Ios {}
impl ApplicationIdentity for Ios {}
impl StandardStreams for Ios {}
