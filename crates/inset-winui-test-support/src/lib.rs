#![doc = include_str!("../README.md")]
#![feature(arbitrary_self_types)]

mod fixture;
pub mod gpu;
mod view;

pub use fixture::{Fixture, mount};
pub use view::CaptureView;
