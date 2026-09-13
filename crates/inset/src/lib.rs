//! App-author crate. One dependency for a window, widgets, and [`Entity`].
//!
//! Layer crates (`inset-widgets`, `inset-foundation`, …) stay the source of
//! truth. This crate re-exports the types an application names. Widget kits
//! (`inset-cupertino`, `inset-winui`) are separate dependencies, and the
//! rendering layer stays behind `inset-widgets`. Hosts and ports still depend
//! on the layer crates directly.
//!
//! ```ignore
//! use inset::{App, IntoWidget, run_app};
//! use inset_cupertino::CupertinoApp;
//!
//! #[inset::main]
//! fn main(app: &mut App) {
//!     run_app(app, CupertinoApp::new().home(Home).into_widget());
//! }
//! ```

pub use inset_embedder::{
    Brightness, Color, FontFeature, FontWeight, Offset, Rect, Size, TextAlign, TextDirection,
};
pub use inset_embedder_default::DefaultEmbedder;
#[cfg(not(target_arch = "wasm32"))]
pub use inset_embedder_default::ImplicitViewConfig;
/// Change-notifier callback used by buttons. The pointer widget is [`inset_widgets::Listener`].
pub use inset_foundation::Listener;
pub use inset_foundation::*;
pub use inset_macros::main;
pub use inset_painting::{
    Alignment, AlignmentGeometry, AnyColor, BorderRadiusGeometry, BoxDecoration, BoxFit,
    EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan, TextStyle,
};
pub use inset_shell::Shell;
pub use inset_widgets::*;

/// Paths the `#[inset::main]` expansion names. Not an API.
#[doc(hidden)]
pub mod __private {
    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen;
}
