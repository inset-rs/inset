//! App-author crate. One dependency for a window, widgets, and [`Entity`].
//!
//! Layer crates (`inset-widgets`, `inset-foundation`, …) stay the source of
//! truth. This crate re-exports the types an application names. Widget kits
//! (`inset-cupertino`, `inset-winui`) are separate dependencies, and the
//! rendering layer stays behind `inset-widgets`. Hosts and ports still depend
//! on the layer crates directly.
//!
//! ```ignore
//! use inset::{App, IntoWidget, run_app};
//! use inset_cupertino::CupertinoApp;
//!
//! #[inset::main]
//! fn main(app: &mut App) {
//!     run_app(app, CupertinoApp::new().home(Home).into_widget());
//! }
//! ```

/// The engine layer whole, for the few types the facade's own names shadow: `ui::Image` is the
/// picture the `Image` widget shows, as `dart:ui`'s `Image` is to Flutter's.
pub use inset_embedder as ui;
pub use inset_embedder::{
    BlurStyle, HostWindow, Platform, PopupMenuEntry, WindowBackground, WindowConfig, WindowError,
    WindowLevel, WindowRef, WindowingOwner,
};
pub use inset_embedder::{
    Brightness, Color, DropChange, DropData, FontFeature, FontWeight, Offset, Radius, Rect, Size,
    TextAlign, TextDirection,
};
#[cfg(target_os = "android")]
pub use inset_embedder_default::AndroidApp;
#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
pub use inset_embedder_default::DefaultEmbedder;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub use inset_embedder_default::ImplicitViewConfig;
/// Change-notifier callback used by buttons. The pointer widget is [`inset_widgets::Listener`].
pub use inset_foundation::Listener;
pub use inset_foundation::*;
pub use inset_macros::main;
pub use inset_painting::{
    Alignment, AlignmentGeometry, AnyColor, Axis, Border, BorderRadius, BorderRadiusGeometry,
    BorderSide, BorderStyle, BoxDecoration, BoxFit, BoxShadow, BoxShape, Clip, EdgeInsets,
    EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan, TextStyle,
};
/// The frame scheduler, for what must wait for a frame to be drawn: Flutter's
/// `scheduler.dart`, reached through `WidgetsBinding.instance.addPostFrameCallback`.
pub use inset_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
pub use inset_shell::Shell;
pub use inset_widgets::*;
