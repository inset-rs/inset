//! Android host: the activity's own loop, native window and input.
//!
//! The entry point is `android_main`, which the platform's activity glue calls on a thread
//! of its own with an [`AndroidApp`]. The application hands that to [`AndroidEmbedder`],
//! which runs the loop until the activity is destroyed.
//!
//! The glue is [`android_activity`], the crate that owns `android_main`, the activity's
//! lifecycle and its looper. Everything above it — the window, the surface, input, the
//! display's refresh — is this crate's, so the host answers the framework in dart:ui's
//! terms rather than a desktop windowing library's.
//!
//! Elsewhere the crate still builds, with no host in it, so the workspace's tests cover
//! the input tables on any machine.

#![cfg_attr(target_os = "android", recursion_limit = "256")]

mod pointer;

pub use pointer::{buttons_of, device_id, kind_of_tool, scroll_delta};

#[cfg(target_os = "android")]
mod chrome;
#[cfg(target_os = "android")]
mod gpu;
#[cfg(target_os = "android")]
mod host;
#[cfg(target_os = "android")]
mod images;
#[cfg(target_os = "android")]
mod input;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
mod platform;
#[cfg(target_os = "android")]
mod refresh;
#[cfg(target_os = "android")]
mod surface;
#[cfg(target_os = "android")]
mod view;
#[cfg(target_os = "android")]
mod vsync;

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;
#[cfg(target_os = "android")]
pub use host::AndroidEmbedder;
#[cfg(target_os = "android")]
pub use platform::AndroidPlatform;
