//! Flutter counterpart: `package:flutter/rendering.dart`.
//!
//! [`arbitrary_self_types`](https://github.com/rust-lang/rust/issues/44874)
//! lets authored methods take `self: RenderHandle<Self>`.

#![feature(arbitrary_self_types)]

#[path = "box.rs"]
mod box_;
mod debug;
mod object;
mod pipeline_owner;
mod proxy_box;
mod shifted_box;
mod sliver;
mod viewport_offset;

pub use box_::*;
pub use debug::*;
pub use object::*;
pub use pipeline_owner::*;
pub use proxy_box::*;
pub use shifted_box::*;
pub use sliver::*;
pub use viewport_offset::*;
