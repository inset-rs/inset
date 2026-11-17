//! Port of Flutter's `widgets/debug.dart`; being written.

use waft_foundation::App;

use crate::framework::{BuildContext, downcast_widget};
use crate::widgets::basic::Directionality;
use crate::widgets::localizations::{Localizations, WidgetsLocalizations};
use crate::widgets::lookup_boundary::LookupBoundary;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::overlay::Overlay;
use crate::widgets::table::Table;

/// Asserts that the given context has a [`MediaQuery`] ancestor.
///
/// Used by various widgets to make sure that they are only used in an appropriate context.
///
/// To invoke this function, use the following pattern, typically in the relevant Widget's
/// build method:
///
/// ```text
/// debug_assert!(debug_check_has_media_query(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked in all cases.
/// This prevents bugs from hiding until a particular codepath is hit.
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_media_query(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && downcast_widget::<MediaQuery>(&**context.widget(app)).is_none()
        && context
            .get_element_for_inherited_widget_of_exact_type::<MediaQuery>(app)
            .is_none()
    {
        panic!(
            "No MediaQuery widget ancestor found.\n{} widgets require a MediaQuery widget \
             ancestor.\nNo MediaQuery ancestor could be found starting from the context that \
             was passed to MediaQuery.of(). This can happen because the context used is not a \
             descendant of a View widget, which introduces a MediaQuery.",
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has a [`Directionality`] ancestor.
///
/// Used by various widgets to make sure that they are only used in an appropriate context.
///
/// To invoke this function, use the following pattern, typically in the relevant Widget's
/// build method:
///
/// ```text
/// debug_assert!(debug_check_has_directionality(app, context, None, None, None));
/// ```
///
/// To improve the error messages you can add some extra color using the arguments.
///
///  * `why`: explain why the direction is needed, for example "to resolve the 'alignment'
///    argument". Should be an adverb phrase describing why.
///  * `hint`: explain why this might be happening, for example "The default value of the
///    'alignment' argument of the $runtimeType widget is an AlignmentDirectional value.".
///    Should be a fully punctuated sentence.
///  * `alternative`: provide additional advice specific to the situation, especially an
///    alternative to providing a Directionality ancestor. For example, "Alternatively,
///    consider specifying the 'textDirection' argument.". Should be a fully punctuated
///    sentence.
///
/// Each one can be `None`, in which case it is skipped (this is the default). If they are
/// given, they are included in the order above, interspersed with the more generic advice
/// regarding [`Directionality`].
///
/// Always place this before any early returns, so that the invariant is checked in all cases.
/// This prevents bugs from hiding until a particular codepath is hit.
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_directionality(
    app: &App,
    context: BuildContext,
    why: Option<&str>,
    hint: Option<&str>,
    alternative: Option<&str>,
) -> bool {
    if cfg!(debug_assertions)
        && downcast_widget::<Directionality>(&**context.widget(app)).is_none()
        && context
            .get_element_for_inherited_widget_of_exact_type::<Directionality>(app)
            .is_none()
    {
        let why = why.map_or(String::new(), |why| format!(" {why}"));
        let mut parts = vec![
            "No Directionality widget found.".to_owned(),
            format!(
                "{} widgets require a Directionality widget ancestor{why}.\n",
                context.widget(app).type_name()
            ),
        ];
        parts.extend(hint.map(str::to_owned));
        parts.push(
            "Typically, the Directionality widget is introduced by the WidgetsApp widget at the \
             top of your application widget tree. It determines the ambient reading direction \
             and is used, for example, to determine how to lay out text, how to interpret \
             \"start\" and \"end\" values, and to resolve EdgeInsetsDirectional, \
             AlignmentDirectional, and other *Directional objects."
                .to_owned(),
        );
        parts.extend(alternative.map(str::to_owned));
        panic!("{}", parts.join("\n"));
    }
    true
}

/// Asserts that the given context has a [`Localizations`] ancestor that contains
/// a [`WidgetsLocalizations`] delegate.
///
/// To call this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_widgets_localizations(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_widgets_localizations(app: &mut App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && Localizations::of::<dyn WidgetsLocalizations>(app, context).is_none()
    {
        panic!(
            "No WidgetsLocalizations found.\n{} widgets require WidgetsLocalizations to be \
             provided by a Localizations widget ancestor.\nThe widgets library uses \
             Localizations to generate messages, labels, and abbreviations.\nTo introduce a \
             WidgetsLocalizations, either use a WidgetsApp at the root of your application to \
             include them automatically, or add a Localization widget with a \
             WidgetsLocalizations delegate.",
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has a [`Table`] ancestor.
///
/// Used by various widgets to make sure that they are only used in an appropriate context.
///
/// To invoke this function, use the following pattern, typically in the relevant Widget's
/// build method:
///
/// ```text
/// debug_assert!(debug_check_has_table(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked in all cases.
/// This prevents bugs from hiding until a particular codepath is hit.
///
/// This method can be expensive (it walks the element tree).
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_table(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && downcast_widget::<Table>(&**context.widget(app)).is_none()
        && context
            .find_ancestor_widget_of_exact_type::<Table>(app)
            .is_none()
    {
        panic!(
            "No Table widget found.\n{} widgets require a Table widget ancestor.",
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has an [`Overlay`] ancestor.
///
/// To call this function, use the following pattern, typically in the relevant Widget's build
/// method:
///
/// ```text
/// debug_assert!(debug_check_has_overlay(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked in all cases.
/// This prevents bugs from hiding until a particular codepath is hit.
///
/// This method can be expensive (it walks the element tree).
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_overlay(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && LookupBoundary::find_ancestor_widget_of_exact_type::<Overlay>(app, context).is_none()
    {
        let hidden_by_boundary =
            LookupBoundary::debug_is_hiding_ancestor_widget_of_exact_type::<Overlay>(app, context);
        panic!(
            "No Overlay widget found{}.{}\n{} widgets require an Overlay widget ancestor \
             within the closest LookupBoundary.\nAn overlay lets widgets float on top of other \
             widget children.\nTo introduce an Overlay widget, you can either directly include \
             one, or use a widget that contains an Overlay itself, such as a Navigator or \
             WidgetsApp.",
            if hidden_by_boundary {
                " within the closest LookupBoundary"
            } else {
                ""
            },
            if hidden_by_boundary {
                "\nThere is an ancestor Overlay widget, but it is hidden by a LookupBoundary."
            } else {
                ""
            },
            context.widget(app).type_name(),
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use waft_embedder::{Locale, TextDirection};
    use waft_foundation::AppCell;

    use super::*;
    use crate::framework::{IntoWidget, WidgetRef};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::localizations::DefaultWidgetsLocalizations;
    use crate::widgets::media_query::MediaQueryData;

    /// A leaf that runs `check` on its own context when it builds.
    fn checking(check: fn(&App, BuildContext) -> bool) -> WidgetRef {
        Builder::new(move |app, context| {
            assert!(check(app, context));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    /// The panic message of mounting `tree`, if it panics.
    fn mount_panic(tree: WidgetRef) -> Option<String> {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let error = catch_unwind(AssertUnwindSafe(|| {
            let harness = Harness::mount(&mut app, tree);
            harness.pump(&mut app);
        }))
        .err()?;
        Some(match error.downcast::<&str>() {
            Ok(message) => message.to_string(),
            Err(error) => *error.downcast::<String>().expect("a panic message"),
        })
    }

    fn check_directionality(app: &App, context: BuildContext) -> bool {
        debug_check_has_directionality(
            app,
            context,
            Some("to resolve the 'alignment' argument"),
            None,
            None,
        )
    }

    #[test]
    fn debug_check_has_directionality_passes_under_a_directionality_and_panics_without_one() {
        assert_eq!(
            mount_panic(
                Directionality::new(TextDirection::Ltr, checking(check_directionality))
                    .into_widget()
            ),
            None
        );
        let message = mount_panic(checking(check_directionality)).expect("no Directionality");
        let mut lines = message.lines();
        assert_eq!(lines.next(), Some("No Directionality widget found."));
        assert_eq!(
            lines.next(),
            Some(
                "Builder widgets require a Directionality widget ancestor to resolve the \
                 'alignment' argument."
            )
        );
        assert_eq!(lines.next(), Some(""));
        assert!(
            lines
                .next()
                .is_some_and(|hint| hint.starts_with("Typically, the Directionality widget")),
            "{message}"
        );
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn debug_check_has_widgets_localizations_passes_under_widgets_localizations_and_panics_without_them()
     {
        let checked = Rc::new(Cell::new(false));
        let checking_localizations = |checked: Rc<Cell<bool>>| {
            Builder::new(move |app, context| {
                assert!(debug_check_has_widgets_localizations(app, context));
                checked.set(true);
                SizedBox::shrink().into_widget()
            })
        };
        let localized = Localizations::new(
            Locale::new("en"),
            vec![DefaultWidgetsLocalizations::delegate()],
        )
        .child(checking_localizations(Rc::clone(&checked)))
        .into_widget();
        assert_eq!(mount_panic(localized), None);
        assert!(checked.get(), "the check ran under the localizations");

        let message = mount_panic(checking_localizations(Rc::default()).into_widget())
            .expect("no WidgetsLocalizations");
        assert!(
            message.starts_with("No WidgetsLocalizations found.\n"),
            "{message}"
        );
    }

    #[test]
    fn debug_check_has_media_query_passes_under_a_media_query_and_panics_without_one() {
        assert_eq!(
            mount_panic(
                MediaQuery::new(MediaQueryData::new(), checking(debug_check_has_media_query))
                    .into_widget()
            ),
            None
        );
        let message = mount_panic(checking(debug_check_has_media_query)).expect("no MediaQuery");
        assert!(
            message.starts_with("No MediaQuery widget ancestor found.\n"),
            "{message}"
        );
        assert!(
            message.ends_with(
                "\nBuilder widgets require a MediaQuery widget ancestor.\nNo MediaQuery \
                 ancestor could be found starting from the context that was passed to \
                 MediaQuery.of(). This can happen because the context used is not a \
                 descendant of a View widget, which introduces a MediaQuery."
            ),
            "{message}"
        );
    }
}
