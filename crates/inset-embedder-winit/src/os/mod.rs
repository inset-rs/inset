//! What a system adds beyond winit. Each system has its own module with the same
//! functions, and the target picks the module, so the host calls one name and carries no
//! `cfg` of its own; a system winit alone covers gets the generic answers.

#[cfg(not(target_os = "macos"))]
mod generic;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(target_os = "macos"))]
pub(crate) use generic::*;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
