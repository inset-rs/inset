//! Flutter counterpart: material_ui's `debug.dart`.

use waft_foundation::App;
use waft_widgets::{BuildContext, Localizations, LookupBoundary, downcast_widget};

use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::scaffold::{Scaffold, ScaffoldMessenger};

/// Asserts that the given context has a [`Material`] ancestor within the closest
/// [`LookupBoundary`].
///
/// Used by many Material Design widgets to make sure that they are
/// only used in contexts where they can print ink onto some material.
///
/// To call this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_material(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// This method can be expensive (it walks the element tree).
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_material(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && LookupBoundary::find_ancestor_widget_of_exact_type::<Material>(app, context).is_none()
    {
        let hidden_by_boundary =
            LookupBoundary::debug_is_hiding_ancestor_widget_of_exact_type::<Material>(app, context);
        panic!(
            "No Material widget found{}.{}\n{} widgets require a Material widget ancestor \
             within the closest LookupBoundary.\nIn Material Design, most widgets are \
             conceptually \"printed\" on a sheet of material. In Flutter's material library, \
             that material is represented by the Material widget. It is the Material widget \
             that renders ink splashes, for instance. Because of this, many material library \
             widgets require that there be a Material widget in the tree above them.\nTo \
             introduce a Material widget, you can either directly include one, or use a widget \
             that contains Material itself, such as a Card, Dialog, Drawer, or Scaffold.",
            if hidden_by_boundary {
                " within the closest LookupBoundary"
            } else {
                ""
            },
            if hidden_by_boundary {
                "\nThere is an ancestor Material widget, but it is hidden by a LookupBoundary."
            } else {
                ""
            },
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has a [`Localizations`] ancestor that contains a
/// [`MaterialLocalizations`] delegate.
///
/// Used by many Material Design widgets to make sure that they are
/// only used in contexts where they have access to localizations.
///
/// To call this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_material_localizations(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// This function has the side-effect of establishing an inheritance
/// relationship with the nearest [`Localizations`] widget (see
/// `BuildContext::depend_on_inherited_widget_of_exact_type`). This is ok if the caller
/// always also calls [`Localizations::of`] or [`Localizations::of`]-like
/// methods such as `<dyn MaterialLocalizations>::of`.
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_material_localizations(app: &mut App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && Localizations::of::<dyn MaterialLocalizations>(app, context).is_none()
    {
        panic!(
            "No MaterialLocalizations found.\n{} widgets require MaterialLocalizations to be \
             provided by a Localizations widget ancestor.\nThe material library uses \
             Localizations to generate messages, labels, and abbreviations.\nTo introduce a \
             MaterialLocalizations, either use a MaterialApp at the root of your application to \
             include them automatically, or add a Localization widget with a \
             MaterialLocalizations delegate.",
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has a [`Scaffold`] ancestor.
///
/// Used by various widgets to make sure that they are only used in an
/// appropriate context.
///
/// To invoke this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_scaffold(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// This method can be expensive (it walks the element tree).
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_scaffold(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && downcast_widget::<Scaffold>(&**context.widget(app)).is_none()
        && context
            .find_ancestor_widget_of_exact_type::<Scaffold>(app)
            .is_none()
    {
        panic!(
            "No Scaffold widget found.\n{} widgets require a Scaffold widget ancestor.\n\
             Typically, the Scaffold widget is introduced by the MaterialApp or WidgetsApp \
             widget at the top of your application widget tree.",
            context.widget(app).type_name(),
        );
    }
    true
}

/// Asserts that the given context has a [`ScaffoldMessenger`] ancestor.
///
/// Used by various widgets to make sure that they are only used in an
/// appropriate context.
///
/// To invoke this function, use the following pattern, typically in the
/// relevant Widget's build method:
///
/// ```text
/// debug_assert!(debug_check_has_scaffold_messenger(app, context));
/// ```
///
/// Always place this before any early returns, so that the invariant is checked
/// in all cases. This prevents bugs from hiding until a particular codepath is
/// hit.
///
/// This method can be expensive (it walks the element tree).
///
/// Does nothing if asserts are disabled. Always returns true.
pub fn debug_check_has_scaffold_messenger(app: &App, context: BuildContext) -> bool {
    if cfg!(debug_assertions)
        && context
            .find_ancestor_widget_of_exact_type::<ScaffoldMessenger>(app)
            .is_none()
    {
        panic!(
            "No ScaffoldMessenger widget found.\n{} widgets require a ScaffoldMessenger \
             widget ancestor.\nTypically, the ScaffoldMessenger widget is introduced by the \
             MaterialApp at the top of your application widget tree.",
            context.widget(app).type_name(),
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use waft_embedder::TextDirection;
    use waft_widgets::{Directionality, IntoWidget, LookupBoundary, WidgetRef};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::drawer_theme::tests::probe;
    use crate::material_localizations::tests::us_english;

    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default()
    }

    fn checked(
        tester: &mut WidgetTester,
        wrap: impl FnOnce(WidgetRef) -> WidgetRef + 'static,
    ) -> Result<bool, String> {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            probe(
                tester,
                move |_app, probe| {
                    Directionality::new(TextDirection::Ltr, wrap(probe)).into_widget()
                },
                |app, context| debug_check_has_material(app, context),
            )
        }));
        outcome.map_err(panic_message)
    }

    #[test]
    fn debug_check_has_material_passes_under_a_material() {
        test_widgets(|tester| {
            let found = checked(tester, |probe| Material::new().child(probe).into_widget());
            assert_eq!(found, Ok(true));
        });
    }

    #[test]
    fn debug_check_has_material_reports_a_missing_material() {
        test_widgets(|tester| {
            let message = checked(tester, |probe| probe).expect_err("there is no Material");
            assert!(
                message.starts_with("No Material widget found."),
                "{message}"
            );
            assert!(
                message.contains(
                    "widgets require a Material widget ancestor within the closest LookupBoundary."
                ),
                "{message}"
            );
            assert!(
                message.contains(
                    "To introduce a Material widget, you can either directly include one, or use a \
             widget that contains Material itself, such as a Card, Dialog, Drawer, or Scaffold."
                ),
                "{message}"
            );
        });
    }

    #[test]
    fn debug_check_has_material_reports_a_material_hidden_by_a_lookup_boundary() {
        test_widgets(|tester| {
            let message = checked(tester, |probe| {
                Material::new()
                    .child(LookupBoundary::new(probe))
                    .into_widget()
            })
            .expect_err("the boundary hides the Material");
            assert!(
                message.starts_with("No Material widget found within the closest LookupBoundary."),
                "{message}"
            );
            assert!(
                message.contains(
                    "There is an ancestor Material widget, but it is hidden by a LookupBoundary."
                ),
                "{message}"
            );
        });
    }

    #[test]
    fn debug_check_has_material_localizations_reports_missing_localizations() {
        test_widgets(|tester| {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                probe(
                    tester,
                    |_app, probe| probe,
                    debug_check_has_material_localizations,
                )
            }));
            let message = panic_message(outcome.expect_err("there are no localizations"));
            assert!(
                message.starts_with("No MaterialLocalizations found."),
                "{message}"
            );
            assert!(
            message.contains(
            "widgets require MaterialLocalizations to be provided by a Localizations widget \
             ancestor."
            ),
            "{message}"
        );
            assert!(
                message.contains(
                    "The material library uses Localizations to generate messages, labels, and \
             abbreviations."
                ),
                "{message}"
            );
        });
    }

    // 'debugCheckHasScaffold control test', without the diagnostics' structure (a panic carries
    // a message) and the ancestor chain (`describeMissingAncestor`, deferred).
    #[test]
    fn debug_check_has_scaffold_reports_a_missing_scaffold() {
        test_widgets(|_| {
            let mut tester = WidgetTester::on(waft_embedder::TargetPlatform::Android);
            let home = waft_widgets::Builder::new(|app, context| {
                crate::bottom_sheet::show_bottom_sheet(
                    app,
                    context,
                    std::rc::Rc::new(|_app, _context| waft_widgets::Container::new().into_widget()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                waft_widgets::Container::new().into_widget()
            })
            .into_widget();
            let theme = crate::theme_data::ThemeData::new(&tester.cell.borrow());
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tester.pump_widget(crate::test_support::material_app(theme, home));
            }));
            let message = panic_message(outcome.expect_err("there is no Scaffold"));
            assert!(
                message.starts_with(
                    "No Scaffold widget found.\nBuilder widgets require a Scaffold widget \
                     ancestor.\n"
                ),
                "{message}"
            );
            assert!(
                message.ends_with(
                    "Typically, the Scaffold widget is introduced by the MaterialApp or \
                     WidgetsApp widget at the top of your application widget tree."
                ),
                "{message}"
            );
        });
    }

    #[test]
    fn debug_check_has_material_localizations_passes_under_material_localizations() {
        test_widgets(|tester| {
            let found = probe(
                tester,
                |_app, probe| us_english(probe),
                debug_check_has_material_localizations,
            );
            assert!(found);
        });
    }
}
