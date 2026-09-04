//! Flutter counterpart: `package:flutter/foundation.dart`.
#![feature(arbitrary_self_types)]

mod app;
mod basic_types;
mod change_notifier;
mod constants;
mod key;
mod observer_list;
mod timers;

pub use app::*;
pub use basic_types::*;
pub use change_notifier::*;
pub use constants::*;
pub use key::*;
pub use observer_list::*;
pub use reveal_embedder::TargetPlatform;
pub use timers::Timer;
