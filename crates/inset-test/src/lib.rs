//! Test scaffolding: a [`TestPlatform`] to run an `App` on and a [`TestView`] for it to draw
//! into, so a test supplies only the piece it is about. The shape of `flutter_test`'s
//! `TestPlatformDispatcher` and `TestFlutterView`: real implementations of the host traits
//! that answer what a test configures and record what the framework asks of them.
//!
//! A dev-dependency of a crate with tests:
//!
//! ```toml
//! [dev-dependencies]
//! inset-test = "0.4"
//! ```

mod platform;
mod view;

pub use platform::{TestDispatcher, TestPlatform};
pub use view::TestView;
