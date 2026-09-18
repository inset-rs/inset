//! What a system adds beyond winit. Each system has its own module with the same
//! functions and types, and the target picks the module, so the host calls one name and
//! carries no `cfg` of its own; a system winit alone covers gets the generic answers.
//!
//! The one type every module supplies is `Vsync`: the system's signal for each refresh of
//! a window's display, started with `Vsync::start` and paused with `set_paused`, or `None`
//! from a system that has none to give, which the host paces by a timer instead. Every module
//! also supplies `run_off_main`: the system's work queue, or a thread where it has none.
//!
//! The rest is what a system reports differently from the desktop winit was written for:
//! `surface_size` and `view_padding`, for a phone whose window is the screen and whose
//! safe area winit does not report, and `present_in_transaction`, which only AppKit has.

#[cfg(not(target_os = "macos"))]
mod generic;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(target_os = "macos"))]
pub(crate) use generic::*;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
