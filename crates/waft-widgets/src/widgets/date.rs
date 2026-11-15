//! Flutter counterpart: `widgets/date.dart`.

use std::rc::Rc;

use waft_foundation::DateTime;

/// Signature for predicating dates for enabled date selections.
///
/// See `showDatePicker` and `showCupertinoModalPopup`, which have a
/// [`SelectableDayPredicate`] parameter used to specify allowable days in the date picker.
pub type SelectableDayPredicate = Rc<dyn Fn(DateTime) -> bool>;
