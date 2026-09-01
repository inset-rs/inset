//! Flutter counterpart: `package:flutter/painting.dart`.

mod alignment;
mod basic_types;
mod border_radius;
mod borders;
mod box_fit;
mod box_shadow;
mod clip;
mod colors;
mod debug;
mod draw;
mod edge_insets;
mod fractional_offset;
mod geometry;
mod matrix_utils;
mod text_scaler;

pub use alignment::*;
pub use basic_types::*;
pub use border_radius::*;
pub use borders::*;
pub use box_fit::*;
pub use box_shadow::*;
pub use clip::*;
pub use colors::*;
pub use debug::*;
pub use draw::{draw_drrect, draw_rrect};
pub use edge_insets::*;
pub use fractional_offset::*;
pub use geometry::*;
pub use matrix_utils::*;
pub use text_scaler::*;
