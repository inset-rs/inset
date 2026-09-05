//! Flutter counterpart: `foundation/basic_types.dart` (the callback signatures).

use std::rc::Rc;

use crate::App;
use crate::executor::Task;

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

/// Signature of callbacks that have no arguments and return no data, but that return a
/// `Future` to indicate when their work is complete.
///
/// The callback runs its body up to the first `await` before returning and hands back the
/// continuation as a [`Task`]; a body with nothing to await returns [`Task::ready`].
pub type AsyncCallback = Rc<dyn Fn(&mut App) -> Task<()>>;

/// Signature for callbacks that report that a value has been set and return a `Future` that
/// completes when the value has been saved.
pub type AsyncValueSetter<T> = Rc<dyn Fn(&mut App, T) -> Task<()>>;

/// Signature for callbacks that are to asynchronously report a value on demand.
pub type AsyncValueGetter<T> = Rc<dyn Fn(&mut App) -> Task<T>>;
