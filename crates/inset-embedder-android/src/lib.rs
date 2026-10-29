//! Android host: the app's own loop, and each activity's window and input as a view of it.
//!
//! The entry point is `android_main`, which the platform's activity glue calls on a thread
//! of its own with an [`AndroidApp`] for each activity the system creates. The application
//! hands that to [`AndroidEmbedder`], which attaches the activity to the app, starting the app
//! on a thread of its own for the first, and runs the activity's thread until the activity is
//! destroyed. The app outlives its activities, as Android's process does.
//!
//! The glue is [`android_activity`], the crate that owns `android_main`, each activity's
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
mod activity;
#[cfg(target_os = "android")]
mod activity_thread;
#[cfg(target_os = "android")]
mod answer;
#[cfg(target_os = "android")]
mod app_loop;
#[cfg(target_os = "android")]
mod app_thread;
#[cfg(target_os = "android")]
mod bitmap;
#[cfg(target_os = "android")]
mod chrome;
#[cfg(target_os = "android")]
mod clipboard;
#[cfg(target_os = "android")]
mod display;
#[cfg(target_os = "android")]
mod document_tree;
#[cfg(target_os = "android")]
mod file_dialogs;
#[cfg(target_os = "android")]
mod gpu;
#[cfg(target_os = "android")]
mod host;
#[cfg(target_os = "android")]
mod host_window;
#[cfg(target_os = "android")]
mod images;
#[cfg(target_os = "android")]
mod input;
#[cfg(target_os = "android")]
mod intents;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
mod notifications;
#[cfg(target_os = "android")]
mod open_requests;
#[cfg(target_os = "android")]
mod opener;
#[cfg(target_os = "android")]
mod platform;
#[cfg(target_os = "android")]
mod refresh;
#[cfg(target_os = "android")]
mod shortcuts;
#[cfg(target_os = "android")]
mod surface;
#[cfg(target_os = "android")]
mod task;
#[cfg(target_os = "android")]
mod view;
#[cfg(target_os = "android")]
mod vsync;

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;
#[cfg(target_os = "android")]
pub use host::AndroidEmbedder;
#[cfg(target_os = "android")]
pub use notifications::NotificationChannelGroups;
#[cfg(target_os = "android")]
pub use platform::AndroidPlatform;
#[cfg(target_os = "android")]
pub use shortcuts::{PinError, PinFuture, PinnedShortcut, PinnedShortcuts};
