//! Browser canvas host: one canvas, `requestAnimationFrame`, WebGPU.
//!
//! On wasm this is the event loop. Elsewhere the crate still builds so workspace
//! tests can cover the key and pointer tables.

#![cfg_attr(target_arch = "wasm32", recursion_limit = "256")]

mod font_fallback;
mod google_fonts;
mod keys;
mod pointer;

pub use font_fallback::fallback_family;
pub use google_fonts::{Slice, css2_url, parse_slices};
pub use keys::{character_of, logical_key_id, physical_key_usage};
pub use pointer::{BACK, FORWARD, MIDDLE, PRIMARY, SECONDARY, flutter_button, flutter_buttons};

#[cfg(target_arch = "wasm32")]
mod fonts;
#[cfg(target_arch = "wasm32")]
mod gpu;
#[cfg(target_arch = "wasm32")]
mod host;
#[cfg(target_arch = "wasm32")]
mod dispatcher;
#[cfg(target_arch = "wasm32")]
mod images;
#[cfg(target_arch = "wasm32")]
mod platform;
#[cfg(target_arch = "wasm32")]
mod text_input;

#[cfg(target_arch = "wasm32")]
pub use host::WebEmbedder;

// Enables getrandom's `wasm_js` backend for wgpu; this crate does not call it.
#[cfg(target_arch = "wasm32")]
use getrandom as _;
