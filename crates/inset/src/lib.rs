//! App-author crate. One dependency for a window, widgets, and [`Entity`].
//!
//! Layer crates (`inset-widgets`, `inset-foundation`, …) stay the source of
//! truth. This crate re-exports the types an application names. Hosts and
//! ports still depend on the layer crates directly.
//!
//! ```ignore
//! use inset::{
//!     App, Column, DefaultEmbedder, IntoWidget, Shell, StatelessWidget, Text, WidgetRef,
//!     run_app,
//! };
//! ```

pub use inset_embedder::{
    Brightness, Color, FontFeature, FontWeight, Offset, Rect, Size, TextAlign, TextDirection,
};
pub use inset_embedder_default::DefaultEmbedder;
#[cfg(not(target_arch = "wasm32"))]
pub use inset_embedder_default::ImplicitViewConfig;
pub use inset_foundation::*;
pub use inset_painting::{
    Alignment, AlignmentGeometry, AnyColor, BorderRadiusGeometry, BoxDecoration, BoxFit,
    EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan, TextStyle,
};
pub use inset_rendering::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
pub use inset_shell::Shell;
pub use inset_widgets::*;
#[cfg(feature = "cupertino")]
pub use inset_cupertino::*;
/// Change-notifier callback used by buttons. The pointer widget is [`inset_widgets::Listener`].
pub use inset_foundation::Listener;
