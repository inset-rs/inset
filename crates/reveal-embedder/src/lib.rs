//! Flutter's `dart:ui` scheduling seam. Value types (`Offset`, `Color`, …)
//! stay in `reveal-geometry` until they move here with `Canvas`.
//!
//! The framework depends on this crate. This crate does not depend on the
//! framework. Hosts implement [`Platform`] and [`View`], and invoke
//! [`EmbedderClient`] on the client the start closure returns. They do not
//! name `App`.

mod client;
mod platform;
mod views;

pub use client::EmbedderClient;
pub use platform::{Frame, InertPlatform, Platform, PlatformRef, ViewRef};
pub use views::{View, ViewConstraints, ViewId, ViewMetrics, ViewPadding};
