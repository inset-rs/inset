//! Flutter counterpart: `widgets/date.dart`.

use std::rc::Rc;

use waft_foundation::{App, DateTime};

/// Signature for predicating dates for enabled date selections.
///
/// See `showDatePicker` and `showCupertinoModalPopup`, which have a
/// [`SelectableDayPredicate`] parameter used to specify allowable days in the date picker. It
/// takes the [`App`], whose platform gives the local time zone the day's fields are read in.
pub type SelectableDayPredicate = Rc<dyn Fn(&App, DateTime) -> bool>;
