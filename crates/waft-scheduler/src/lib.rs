//! Flutter counterpart: `package:flutter/scheduler.dart`.
#![feature(arbitrary_self_types)]

mod binding;
mod priority;
#[cfg(test)]
mod test_support;
mod ticker;

pub use binding::*;
pub use priority::*;
pub use ticker::*;
