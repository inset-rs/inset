//! Windows: what winit covers by itself, the clipboard's sequence number, which tells the app
//! another writer's change without reading the clipboard (`clipboard/text.rs`), Win32's menus at a
//! point and icons in the notification area (`menus`), the shell's file dialogs
//! (`file_dialogs`), whose loops run from the embedder's hidden window (`hidden_window`), the
//! shell's opener (`opener`), and WinRT's toasts (`notifications`).

// Win32 is reached through system calls, which Rust reaches through `unsafe`; this module and its
// children are the one place on Windows that writes it.
#![allow(unsafe_code)]

mod file_dialogs;
mod hidden_window;
mod menus;
mod notifications;
mod opener;

use inset_embedder::TargetPlatform;
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

use crate::clipboard::{ChangeProbe, TextClipboard};

use super::absent::{NoDisplayLink, NoVisibilityNotices, NoWindowSurfaces};
use super::{
    ApplicationControl, ApplicationLifecycle, FramePresentation, ImageDecoding, NativeComposition,
    OperatingSystem, PointerQueries, Screens, SystemChanges, SystemClipboard, SystemDrags,
    SystemMedia, SystemRequests, WindowChrome, WindowGeometry, WindowMotion, WindowVisibility,
    WorkQueue,
};

/// Windows. A type only: never made, its answers are associated functions.
pub(crate) enum Windows {}

impl OperatingSystem for Windows {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::Windows;
}

/// arboard's plain text, as winit has no clipboard, its changes told by the sequence number; a
/// copy that expires is cleared by the main loop.
impl SystemClipboard for Windows {
    type Clipboard = TextClipboard;

    fn clipboard(_look: Box<dyn Fn()>) -> TextClipboard {
        TextClipboard::new(ChangeProbe::Count(sequence_number))
    }
}

/// The clipboard's sequence number, which Windows moves at each change of its contents.
fn sequence_number() -> u32 {
    // SAFETY: the call takes nothing and reads a counter of the window station's; any thread may
    // make it.
    unsafe { GetClipboardSequenceNumber() }
}

impl FramePresentation for Windows {
    type DisplayLink = NoDisplayLink;
}

impl NativeComposition for Windows {
    type WindowSurfaces = NoWindowSurfaces;
}

impl WindowVisibility for Windows {
    type Notices = NoVisibilityNotices;
}

impl WindowGeometry for Windows {}
impl SystemMedia for Windows {}
impl SystemDrags for Windows {}
impl WindowMotion for Windows {}
impl WindowChrome for Windows {}
impl Screens for Windows {}
impl PointerQueries for Windows {}
impl ImageDecoding for Windows {}
impl WorkQueue for Windows {}
/// The system starts the app with what it opens on its command line, which is the app's own.
impl SystemRequests for Windows {}
impl SystemChanges for Windows {}
impl ApplicationControl for Windows {}
impl ApplicationLifecycle for Windows {}
