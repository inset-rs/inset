#![doc = include_str!("../README.md")]
#![feature(arbitrary_self_types)]
#![recursion_limit = "256"]

mod fixture;
pub mod gpu;
mod view;

pub use fixture::{Fixture, mount};
pub use view::{CaptureView, LastFrame};
