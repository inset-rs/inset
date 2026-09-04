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
mod mouse_cursor;
mod painting;
mod paragraph;
mod platform;
mod pointer;
mod text;
mod views;

pub use client::EmbedderClient;
pub use fonts::{FontCollection, FontDemand, FontId, FontSource};
pub use geometry::*;
pub use mouse_cursor::SystemMouseCursorKind;
pub use painting::*;
pub use paragraph::*;
pub use platform::{
    Brightness, Frame, InertPlatform, Platform, PlatformRef, TargetPlatform, ViewRef,
};
pub use pointer::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
};
pub use text::*;
pub use views::{GestureSettings, View, ViewConstraints, ViewId, ViewMetrics, ViewPadding};
