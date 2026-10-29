//! Flutter's `dart:ui`: host traits (`Platform`, `View`) and value types
//! (`Offset`, `Color`, `Canvas`, `Paint`, `Shadow`).
//!
//! The framework depends on this crate. This crate does not depend on the
//! framework. Hosts implement [`Platform`] and [`View`], and invoke
//! [`EmbedderClient`] on the client the start closure returns. They do not
//! name `App`.

// A test image carries a wgpu texture, and proving one `Sync` walks a type graph deeper than the
// default limit.
#![recursion_limit = "256"]

mod client;
mod clipboard;
mod composition;
mod drag_drop;
mod file_dialogs;
mod fonts;
mod geometry;
mod image;
mod key;
mod locale;
mod media;
mod menus;
mod mouse_cursor;
mod notifications;
mod opener;
mod painting;
mod paragraph;
mod platform;
mod pointer;
mod restoration;
mod scene_builder;
mod system_context_menu;
#[cfg(feature = "test-support")]
pub mod test_support;
mod text;
mod text_editing;
mod text_input;
mod views;
mod vsync;
mod window;

pub use client::EmbedderClient;
pub use clipboard::{
    Clipboard, ClipboardError, ClipboardEvent, ClipboardItem, ClipboardItemsFuture,
    ClipboardReadFuture, ClipboardWriteFuture, Content, ContentFuture, CopyOptions, CopyRequest,
    DataTransferItem, FileSink, URI_LIST, WriteFuture,
};
pub use composition::{Buffer, Compositor, Surface, SurfaceFrame, SurfaceParent};
pub use drag_drop::{DragChange, DragData};
pub use file_dialogs::{
    EntryKind, FileDialogError, FileDialogFuture, FileDialogs, FileFinishFuture, FileHandle,
    FileOpenFuture, FileReadFuture, FileSizeFuture, FileStream, FileType, FileWriteFuture,
    FileWriter, FolderEntriesFuture, FolderEntry, FolderHandle, FolderOptions, OpenOptions,
    PickedFile, PickedFolder, SaveOptions,
};
pub use fonts::{
    FontCollection, FontDemand, FontId, FontManager, FontSource, SystemFontSource, Typeface,
    default_font_families, set_default_font_manager,
};
pub use geometry::*;
pub use image::{
    ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame, ImageFrameFuture, ImageRepetition,
};
pub use key::{KeyData, KeyEventDeviceType, KeyEventType};
pub use locale::Locale;
pub use media::{
    Camera, CameraFacing, CameraId, CameraState, CameraStatus, Cameras, CaptureSession,
    FrameReader, MediaChangeFuture, MediaError, MediaPlayer, MovieOptions, MovieOutput, Photo,
    PhotoFuture, PhotoOutput, PixelBuffer, PlayerState, PlayerStatus, Players, RecordingFuture,
};
pub use menus::{
    AttachedMenu, CheckState, DockMenu, EditCommand, IconImage, MenuAtIcon, MenuBar, MenuEntry,
    MenuId, MenuItem, MenuItemCheck, MenuShortcut, PopupMenuFuture, PopupMenus, ProvidedItem,
    ShortcutModifiers, TrayIcon, TrayIconError, TrayIconEvent, TrayIconFuture, TrayIconId,
    TrayIcons,
};
pub use mouse_cursor::SystemMouseCursorKind;
pub use notifications::{
    Notification, NotificationAction, NotificationActivation, NotificationError,
    NotificationPermission, NotificationReplies, NotificationScheduling, Notifications,
    PermissionFuture, PermissionState, PostFuture,
};
pub use opener::{OpenError, OpenFuture, OpenRequest, Opener, file_uri_path};
pub use painting::*;
pub use paragraph::*;
pub use platform::{
    AppExit, AppExitReply, AppExitResponse, AppExitType, AppLifecycleState,
    ApplicationSwitcherDescription, Brightness, Dispatcher, Frame, HapticFeedbackType, Haptics,
    InertDispatcher, InertPlatform, MouseCursor, Platform, PlatformRef, Restoration, Screen,
    ScreenId, Screens, SystemChrome, SystemContextMenu, SystemUiOverlayStyle, TargetPlatform,
    TextServices, ViewRef,
};
pub use pointer::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
};
pub use restoration::{RestorationData, RestorationMap, RestorationUpdate};
pub use scene_builder::{EngineLayer, Scene, SceneBuilder, SurfacePlacement};
pub use system_context_menu::SystemContextMenuItem;
pub use text::*;
pub use text_editing::*;
pub use text_input::*;
pub use views::{
    GestureSettings, TextInputHost, View, ViewConstraints, ViewFocusDirection, ViewFocusEvent,
    ViewFocusState, ViewId, ViewMetrics, ViewPadding,
};
pub use vsync::{VsyncObserver, VsyncWaiter};
pub use web_time::Instant;
pub use window::{
    HostWindow, RawWindowHandle, RequestId, ViewError, ViewRequest, WindowBackground, WindowConfig,
    WindowLevel, WindowRef,
};
