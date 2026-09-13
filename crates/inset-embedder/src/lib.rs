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
mod fonts;
mod geometry;
mod image;
mod key;
mod locale;
mod mouse_cursor;
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
mod window;

pub use client::EmbedderClient;
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
pub use mouse_cursor::SystemMouseCursorKind;
pub use painting::*;
pub use paragraph::*;
pub use platform::{
    AppExitResponse, AppLifecycleState, ApplicationSwitcherDescription, Brightness, Frame,
    HapticFeedbackType, InertPlatform, Platform, PlatformRef, PopupMenuEntry, SystemUiOverlayStyle,
    TargetPlatform, ViewRef,
};
pub use pointer::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
};
pub use restoration::{RestorationData, RestorationMap, RestorationUpdate};
pub use scene_builder::{Scene, SceneBuilder};
pub use system_context_menu::SystemContextMenuItem;
pub use text::*;
pub use text_editing::*;
pub use text_input::*;
pub use views::{
    GestureSettings, View, ViewConstraints, ViewFocusDirection, ViewFocusEvent, ViewFocusState,
    ViewId, ViewMetrics, ViewPadding,
};
pub use web_time::Instant;
pub use window::{
    HostWindow, RawWindowHandle, WindowBackground, WindowConfig, WindowError, WindowFuture,
    WindowLevel, WindowRef, WindowingOwner,
};
