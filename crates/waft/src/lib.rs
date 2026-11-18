//! App-author crate. One dependency for a window, widgets, and [`Entity`].
//!
//! Layer crates (`waft-widgets`, `waft-foundation`, …) stay the source of
//! truth. This crate re-exports the types an application names. Widget kits
//! (`waft-cupertino`, `waft-winui`) are separate dependencies, and the
//! rendering layer stays behind `waft-widgets`. Hosts and ports still depend
//! on the layer crates directly.
//!
//! ```ignore
//! use waft::{App, IntoWidget, run_app};
//! use waft_cupertino::CupertinoApp;
//!
//! #[waft::main]
//! fn main(app: &mut App) {
//!     run_app(app, CupertinoApp::new().home(Home).into_widget());
//! }
//! ```

/// The engine layer whole, for the few types the facade's own names shadow: `ui::Image` is the
/// picture the `Image` widget shows, as `dart:ui`'s `Image` is to Flutter's.
pub use waft_embedder as ui;
pub use waft_embedder::{
    BlendMode, BlurStyle, Brightness, Canvas, Color, DragChange, DragReport, FontFeature,
    FontWeight, Offset, Paint, PaintingStyle, Path, PathFillType, Picture, Radius, Rect, Shader,
    Size, Stroke, StrokeCap, TextAlign, TextDirection,
};
/// The host's HTTP client, which a request and its body go through.
pub use waft_embedder::{
    Body, BodyReadAllFuture, BodyReadFuture, BodyStream, BodyTrailersFuture, DnsErrorPayload,
    FieldSizePayload, HttpClient, HttpError, HttpFuture, RedirectPolicy, SendOptions,
    TlsAlertReceivedPayload,
};
/// Text input as a field names it: the keyboard's type and action, the platform's smart dashes
/// and quotes, capitalization, and the value, position, range and selection a controller holds.
pub use waft_embedder::{
    BoxHeightStyle, BoxWidthStyle, SmartDashesType, SmartQuotesType, TextCapitalization,
    TextEditingValue, TextInputAction, TextInputType, TextPosition, TextRange, TextSelection,
};
/// The system's other windows, cameras, subtitles and video: a `View` and its focus, the
/// composition layer a `CompositionLayerView` draws, and what a camera or video widget is given.
pub use waft_embedder::{
    Camera, CameraDevice, CameraFormat, CameraOutputs, CaptionStyle, CompositionLayer, Cue,
    SeekTolerance, VideoFrameReader, VideoOutput, ViewFocusEvent, ViewId, ViewRef,
};
/// What the system tells the app besides taps: a tray icon's events, a platform menu item's
/// check mark, a menu or tray icon's image, the system bars' style, a clipboard or edit command
/// the binding handles, and the data a restorable property saves.
pub use waft_embedder::{
    CheckState, ClipboardEvent, EditCommand, IconImage, MenuItemCheck, RestorationData,
    SystemUiOverlayStyle, TrayIconEvent, TrayIconId,
};
/// The system's file dialogs and what their picks answer, as an app names them: the types
/// `FileDialogs` hands back, whether a pick is a path or the system's own handle.
pub use waft_embedder::{
    EntryKind, FileDialogError, FileDialogFuture, FileDialogs, FileFinishFuture, FileHandle,
    FileOpenFuture, FileReadFuture, FileSizeFuture, FileStream, FileType, FileWriteFuture,
    FileWriter, FolderEntriesFuture, FolderEntry, FolderHandle, FolderOptions, OpenOptions,
    PickedFile, PickedFolder, SaveOptions,
};
/// The host's file system, which a path read whole, written whole or opened at an offset goes
/// through: dart:io's `File` and `RandomAccessFile` with Rust names.
pub use waft_embedder::{
    File, FileKind, FileMetadataFuture, FileOperationFuture, Files, ListFilesFuture, ListedEntry,
    Metadata, OpenFileFuture, OpenMode, ReadFileFuture, WriteFileFuture,
};
/// Drawing a widget asks for: the filters an `Image` or `BackdropFilter` takes, a rounded or
/// superelliptical rectangle a clipper answers, and the alignment a `WidgetSpan` places its
/// placeholder by.
pub use waft_embedder::{FilterQuality, ImageFilter, PlaceholderAlignment, RRect, RSuperellipse};
pub use waft_embedder::{
    HostWindow, LinuxWindowConfig, MacosTitleBar, MacosWindowConfig, MenuEntry, MenuItem, Platform,
    RequestId, TitleBarAction, ViewError, WindowBackground, WindowConfig, WindowLevel, WindowRef,
    WindowSizeLimits, WindowsIconResource, WindowsRelaunch, WindowsWindowConfig,
};
pub use waft_embedder::{Locale, PointerDeviceKind};
#[cfg(target_os = "android")]
pub use waft_embedder_default::AndroidApp;
#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
pub use waft_embedder_default::DefaultEmbedder;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub use waft_embedder_default::{LinuxSettings, WindowsSettings};
/// Change-notifier callback used by buttons. The pointer widget is [`waft_widgets::Listener`].
pub use waft_foundation::Listener;
pub use waft_foundation::*;
/// The gesture recognizers and the details their callbacks carry, which a [`GestureDetector`]
/// takes: Flutter's `package:flutter/gestures.dart`, which its widget library imports rather than
/// re-exports.
pub use waft_gestures::*;
pub use waft_macros::{linux_app_id, main};
pub use waft_painting::{
    Alignment, AlignmentGeometry, AnyColor, Axis, Border, BorderRadius, BorderRadiusGeometry,
    BorderSide, BorderStyle, BoxDecoration, BoxFit, BoxShadow, BoxShape, Clip, EdgeInsets,
    EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan, TextStyle,
};
/// The frame scheduler, for what must wait for a frame to be drawn: Flutter's
/// `scheduler.dart`, reached through `WidgetsBinding.instance.addPostFrameCallback`.
pub use waft_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
pub use waft_shell::Shell;
/// The field-and-accessor body a [`State`] asks for, for a struct whose bag is `state`.
pub use waft_widgets::state_accessors;
pub use waft_widgets::*;
