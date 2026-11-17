//! Reply, Google's Material 3 mail sample, ported from Jetpack Compose
//! (github.com/android/compose-samples, `Reply/`, Apache-2.0; `assets/reply/LICENSE`).
//!
//! It is written to be copied as the start of an app, in three layers:
//!
//! - logic: [`data`], the mail and contacts as plain Rust;
//! - state: [`state`], the inbox as an entity the screens share;
//! - UI: [`theme`], the [`components`] and the screens ([`app`], [`navigation`], [`inbox`],
//!   [`empty_coming_soon`]), which read the state while they build and call it from callbacks.
//!
//! The screens lay themselves out with Waft's adaptive layout: the navigation suite for the bar,
//! rail or drawer, and the pane scaffold for the list beside the open mail.

pub mod app;
pub mod components;
pub mod data;
pub mod empty_coming_soon;
pub mod inbox;
pub mod navigation;
pub mod state;
pub mod theme;

pub use app::ReplyApp;
