//! Flutter counterpart: `package:flutter/scheduler.dart`.
#![feature(arbitrary_self_types)]

mod binding;
mod priority;
mod ticker;

pub use binding::*;
pub use priority::*;
pub use ticker::*;
