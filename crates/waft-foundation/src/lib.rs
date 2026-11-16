//! Flutter counterpart: `package:flutter/foundation.dart`.
#![feature(arbitrary_self_types)]

mod app;
mod app_cell;
mod background;
mod basic_types;
mod change_notifier;
mod completer;
mod constants;
mod date_time;
mod entity;
mod erased;
mod executor;
mod handle_map;
mod key;
mod local_time;
mod observer_list;
mod signed_duration;
mod timers;

pub use app::*;
pub use app_cell::*;
pub use basic_types::*;
pub use change_notifier::*;
pub use completer::*;
pub use constants::*;
pub use date_time::DateTime;
pub use entity::*;
pub use executor::Task;
#[cfg(debug_assertions)]
pub use handle_map::DebugObject;
pub use key::*;
pub use observer_list::*;
pub use signed_duration::SignedDuration;
pub use timers::Timer;
pub use waft_embedder::TargetPlatform;
