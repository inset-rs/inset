//! Flutter counterpart: `package:flutter/rendering.dart`.
//!
//! [`arbitrary_self_types`](https://github.com/rust-lang/rust/issues/44874)
//! lets authored methods take `self: RenderHandle<Self>`.

#![feature(arbitrary_self_types)]
#![allow(recursion_depth_exceeding_limit)] // valo `DisplayList` nests `Op::DrawDisplayList`

mod animated_size;
mod binding;
#[path = "box.rs"]
mod box_;
mod custom_layout;
mod custom_paint;
mod debug;
mod editable;
mod flex;
mod image_filter_config;
mod layer;
mod layout_helper;
mod mouse_tracker;
mod object;
mod painting_context;
mod paragraph;
mod pipeline_owner;
mod proxy_box;
mod shifted_box;
mod sliver;
mod sliver_fixed_extent_list;
mod sliver_list;
mod sliver_multi_box_adaptor;
mod sliver_padding;
mod sliver_persistent_header;
mod stack;
mod tweens;
mod view;
mod viewport;
mod viewport_offset;

pub use animated_size::*;
pub use binding::*;
pub use box_::*;
pub use custom_layout::*;
pub use custom_paint::*;
pub use debug::*;
pub use editable::*;
pub use flex::*;
pub use image_filter_config::*;
pub use layer::*;
pub use layout_helper::*;
pub use mouse_tracker::*;
pub use object::*;
pub use painting_context::*;
pub use paragraph::*;
pub use pipeline_owner::*;
pub use proxy_box::*;
pub use shifted_box::*;
pub use sliver::*;
pub use sliver_fixed_extent_list::*;
pub use sliver_list::*;
pub use sliver_multi_box_adaptor::*;
pub use sliver_padding::*;
pub use sliver_persistent_header::*;
pub use stack::*;
pub use tweens::*;
pub use view::*;
pub use viewport::*;
pub use viewport_offset::*;
