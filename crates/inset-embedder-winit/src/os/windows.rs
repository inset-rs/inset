//! Windows: what winit covers by itself, and the clipboard's sequence number, which tells the app
//! another writer's change without reading the clipboard (`clipboard/text.rs`).

// `GetClipboardSequenceNumber` is a system call, which Rust reaches through `unsafe`; this module
// is the one place on Windows that writes it.
#![allow(unsafe_code)]

use inset_embedder::TargetPlatform;
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

use crate::clipboard::{ChangeProbe, TextClipboard};

use super::absent::{NoDisplayLink, NoVisibilityNotices, NoWindowSurfaces};
use super::{
    ApplicationControl, ApplicationLifecycle, FramePresentation, ImageDecoding, NativeComposition,
    NativeMenus, OperatingSystem, PointerQueries, Screens, SystemChanges, SystemClipboard,
    SystemRequests, WindowChrome, WindowGeometry, WindowMotion, WindowVisibility, WorkQueue,
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
impl WindowMotion for Windows {}
impl WindowChrome for Windows {}
impl Screens for Windows {}
impl PointerQueries for Windows {}
impl NativeMenus for Windows {}
impl ImageDecoding for Windows {}
impl WorkQueue for Windows {}
impl SystemRequests for Windows {}
impl SystemChanges for Windows {}
impl ApplicationControl for Windows {}
impl ApplicationLifecycle for Windows {}
