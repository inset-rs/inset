//! Flutter's `dart:ui`: host traits (`Platform`, `View`) and value types
//! (`Offset`, `Color`, `Canvas`, `Paint`, `Shadow`).
//!
//! The framework depends on this crate. This crate does not depend on the
//! framework. Hosts implement [`Platform`] and [`View`], and invoke
//! [`EmbedderClient`] on the client the start closure returns. They do not
//! name `App`.

mod client;
mod fonts;
mod geometry;
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
mod text;
mod text_editing;
mod text_input;
mod views;

pub use client::EmbedderClient;
pub use fonts::{
    FontCollection, FontDemand, FontId, FontManager, FontSource, SystemFontSource, Typeface,
    default_font_families, set_default_font_manager,
};
pub use geometry::*;
pub use key::{KeyData, KeyEventDeviceType, KeyEventType};
pub use locale::Locale;
pub use mouse_cursor::SystemMouseCursorKind;
pub use painting::*;
pub use paragraph::*;
pub use platform::{
    AppExitResponse, AppLifecycleState, ApplicationSwitcherDescription, Brightness, Frame,
    HapticFeedbackType, InertPlatform, Platform, PlatformRef, SystemUiOverlayStyle, TargetPlatform,
    ViewRef,
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
