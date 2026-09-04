//! Flutter counterpart: `widgets/framework.dart`, split by concern:
//!
//! - `widget`: `Widget`, its kinds, keys, and the `IntoWidget` erasure.
//! - `state`: `State`.
//! - `element`: `Element`, `ElementData`, the erased `AnyElement` edge, `BuildContext`.
//! - `build_owner`: `BuildScope`, `BuildOwner`, the inactive-element list.
//! - `elements`: the concrete element classes.
//!
//! Diagnostics, `ErrorWidget`, notifications, `reassemble`, and the multi-child element wait;
//! see `PORTING.md`.

mod build_owner;
mod element;
mod elements;
mod inherited_model;
mod state;
#[cfg(test)]
mod tests;
mod widget;

pub use build_owner::*;
pub use element::*;
pub use elements::*;
pub use inherited_model::*;
pub use state::*;
pub use widget::*;
