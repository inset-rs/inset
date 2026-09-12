//! Flutter counterpart: `package:flutter/gestures.dart`.

#![feature(arbitrary_self_types)]

mod arena;
mod binding;
mod constants;
mod converter;
mod debug;
mod drag;
mod drag_details;
mod eager;
mod events;
mod force_press;
mod gesture_details;
mod gesture_settings;
mod hit_test;
mod long_press;
mod lsq_solver;
mod monodrag;
mod multidrag;
mod pointer_router;
mod pointer_signal_resolver;
mod recognizer;
mod tap;
mod tap_and_drag;
mod team;
mod velocity;
mod velocity_tracker;

pub use arena::*;
pub use binding::*;
pub use constants::*;
pub use converter::*;
pub use debug::*;
pub use drag::*;
pub use drag_details::*;
pub use eager::*;
pub use events::*;
pub use force_press::*;
pub use gesture_details::*;
pub use gesture_settings::*;
pub use hit_test::*;
pub use long_press::*;
pub use lsq_solver::*;
pub use monodrag::*;
pub use multidrag::*;
pub use pointer_router::*;
pub use pointer_signal_resolver::*;
pub use recognizer::*;
pub use tap::*;
pub use tap_and_drag::*;
pub use team::*;
pub use velocity::*;
pub use velocity_tracker::*;
