//! Flutter counterpart: `foundation/basic_types.dart` (sync signatures).

use std::rc::Rc;

use crate::App;

/// Signature for callbacks that report that an underlying value has changed.
///
/// Flutter's `ValueChanged<T>` is `void Function(T value)`. Receives [`App`]
/// because a Rust callback cannot capture what it mutates.
pub type ValueChanged<T> = Rc<dyn Fn(&mut App, T)>;

/// Signature for callbacks that report that a value has been set.
///
/// This is the same signature as [`ValueChanged`], but is used when the
/// callback is called even if the underlying value has not changed.
pub type ValueSetter<T> = Rc<dyn Fn(&mut App, T)>;

/// Signature for callbacks that are to report a value on demand.
pub type ValueGetter<T> = Rc<dyn Fn(&App) -> T>;
