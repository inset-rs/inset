//! Android host: Waft's own glue over `NativeActivity`, the app's loop on a thread of its own, and
//! each activity's window as a view of the app.
//!
//! `NativeActivity` calls `ANativeActivity_onCreate` (`native_activity`) on Android's main thread
//! for each activity it makes. The process's first starts the app's thread (`app_thread`), which
//! calls the app's entry, `android_main`, once for the process, with an [`AndroidApp`]; the entry
//! hands that to [`AndroidEmbedder`], which runs the loop for the rest of the process. Each
//! activity's callbacks hand what Android tells them from the main thread to the app's thread
//! (`activity_events`), and its input queue is attached to the app's thread's looper
//! (`input_queues`). No thread stands for an activity: the app outlives each, and an activity is a
//! view of it while it lives. Everything above the glue, the window, the surface, input, the
//! display's refresh, is this crate's too, so the host answers the framework in dart:ui's terms
//! rather than a desktop windowing library's.
//!
//! A call into Java goes through the process's application or one activity (`application`,
//! `activity`), the one attached to a view now, since Android makes and destroys them while the
//! app runs on; what Java tells the app carries the view it is about (`activity_events`,
//! `text_input`, `window_insets`); and the views are a registry by id (`views`), each holding what
//! is its window's (`view`) and its activity's task, which is its window (`task_window`). A view
//! the app asks for is a document activity in a task of its own (`view_requests`).
//!
//! Elsewhere the crate still builds, with no host in it, so the workspace's tests cover
//! the input tables on any machine.

#![cfg_attr(target_os = "android", recursion_limit = "256")]

mod pointer;

pub use pointer::{buttons_of, device_id, kind_of_tool, scroll_delta};

#[cfg(target_os = "android")]
mod activity;
#[cfg(target_os = "android")]
mod activity_events;
#[cfg(target_os = "android")]
mod answer;
#[cfg(target_os = "android")]
mod app_thread;
#[cfg(target_os = "android")]
mod application;
#[cfg(target_os = "android")]
mod chrome;
#[cfg(target_os = "android")]
mod clipboard;
#[cfg(target_os = "android")]
mod composition;
#[cfg(target_os = "android")]
mod display;
#[cfg(target_os = "android")]
mod document_tree;
#[cfg(any(target_os = "android", test))]
mod editor_info;
#[cfg(target_os = "android")]
mod file_dialogs;
#[cfg(target_os = "android")]
mod frame_fence;
#[cfg(target_os = "android")]
mod gpu;
#[cfg(target_os = "android")]
mod gpu_interop;
#[cfg(target_os = "android")]
mod hardware_buffers;
#[cfg(target_os = "android")]
mod host;
#[cfg(target_os = "android")]
mod icon_pixels;
#[cfg(any(target_os = "android", test))]
mod image_codecs;
#[cfg(target_os = "android")]
mod images;
#[cfg(target_os = "android")]
mod input;
#[cfg(target_os = "android")]
mod input_queues;
#[cfg(target_os = "android")]
mod intents;
#[cfg(target_os = "android")]
mod log;
#[cfg(target_os = "android")]
mod loop_waker;
#[cfg(target_os = "android")]
mod main_thread;
#[cfg(target_os = "android")]
mod native_activity;
#[cfg(target_os = "android")]
mod notifications;
#[cfg(target_os = "android")]
mod open_requests;
#[cfg(target_os = "android")]
mod opener;
#[cfg(target_os = "android")]
mod picture;
#[cfg(target_os = "android")]
mod platform;
#[cfg(target_os = "android")]
mod queue_family_handover;
#[cfg(target_os = "android")]
mod refresh;
#[cfg(any(target_os = "android", test))]
mod saved_view;
#[cfg(target_os = "android")]
mod surface;
#[cfg(target_os = "android")]
mod surface_control;
#[cfg(target_os = "android")]
mod task_window;
#[cfg(target_os = "android")]
mod text_input;
#[cfg(any(target_os = "android", test))]
mod this_process;
#[cfg(target_os = "android")]
mod view;
#[cfg(target_os = "android")]
mod view_requests;
#[cfg(target_os = "android")]
mod views;
#[cfg(target_os = "android")]
mod vsync;
#[cfg(target_os = "android")]
mod vsync_waiter;
#[cfg(target_os = "android")]
mod vulkan_imports;
#[cfg(target_os = "android")]
mod window_insets;

#[cfg(target_os = "android")]
pub use app_thread::AndroidApp;
#[cfg(target_os = "android")]
pub use hardware_buffers::AndroidHardwareBuffer;
#[cfg(target_os = "android")]
pub use host::AndroidEmbedder;
#[cfg(target_os = "android")]
pub use platform::AndroidPlatform;
