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
    BlurStyle, HostWindow, MacosTitleBar, MacosWindowConfig, MenuEntry, MenuItem, Platform,
    RequestId, TitleBarAction, ViewError, WindowBackground, WindowConfig, WindowLevel, WindowRef,
    WindowSizeLimits,
};
pub use waft_embedder::{
    Brightness, Color, DragChange, DragReport, FontFeature, FontWeight, Offset, Radius, Rect, Size,
    TextAlign, TextDirection,
};
#[cfg(target_os = "android")]
pub use waft_embedder_default::AndroidApp;
#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
pub use waft_embedder_default::DefaultEmbedder;
/// Change-notifier callback used by buttons. The pointer widget is [`waft_widgets::Listener`].
pub use waft_foundation::Listener;
pub use waft_foundation::*;
pub use waft_macros::main;
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
pub use waft_widgets::*;
