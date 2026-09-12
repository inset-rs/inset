//! Flutter `cupertino/debug.dart`.

use inset_foundation::App;
use inset_widgets::{BuildContext, Localizations};

use crate::localizations::CupertinoLocalizations;

/// Asserts that the given context has a [`Localizations`] ancestor that contains a
/// [`CupertinoLocalizations`] delegate.
///
/// To call this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_cupertino_localizations(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_cupertino_localizations(app: &mut App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && Localizations::of::<dyn CupertinoLocalizations>(app, context).is_none()
    {
        panic!(
            "No CupertinoLocalizations found.\n\
             {:?} widgets require CupertinoLocalizations to be provided by a Localizations \
             widget ancestor.\n\
             The cupertino library uses Localizations to generate messages, labels, and \
             abbreviations.\n\
             To introduce a CupertinoLocalizations, either use a CupertinoApp at the root of \
             your application to include them automatically, or add a Localization widget \
             with a CupertinoLocalizations delegate.",
            context.widget(app)
        );
    }
    true
}
