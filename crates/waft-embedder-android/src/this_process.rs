//! A value made fresh in each process, which tells what this process made from what an earlier
//! one left: an activity's saved token (`saved_view.rs`), and the intent of a task a request
//! started (`view_requests.rs`), both of which Android hands back to a later process when it
//! brings a card in Recents back after the process died.
//!
//! The invariant a later editor will break: the value is the system's randomness, never a clock,
//! which may be set back between two processes.

use std::hash::{BuildHasher, RandomState};
use std::sync::LazyLock;

/// The standard library's random keys, which it takes from the system once a process.
static THIS_PROCESS: LazyLock<u64> = LazyLock::new(|| RandomState::new().hash_one(0u8));

/// This process's value.
pub(crate) fn this_process() -> u64 {
    *THIS_PROCESS
}
