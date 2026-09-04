//! Flutter counterpart: `widgets/icon_theme.dart`.
//!
//! [`IconTheme`] is a plain inherited widget: the `InheritedTheme` capture (`wrap`,
//! `captureAll`) waits.

use std::rc::Rc;

use reveal_foundation::App;

use crate::framework::{BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef};
use crate::widgets::basic::Builder;
use crate::widgets::icon_theme_data::IconThemeData;

/// Controls the default properties of icons in a widget subtree.
///
/// The icon theme is honored by `Icon` and `ImageIcon` widgets.
#[derive(Debug)]
pub struct IconTheme {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// The set of properties to use for icons in this subtree.
    pub data: IconThemeData,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl IconTheme {
    /// Creates an icon theme that controls properties of descendant widgets.
    pub fn new(data: IconThemeData, child: WidgetRef) -> IconTheme {
        IconTheme {
            key: None,
            data,
            child,
        }
    }

    /// Dart `IconTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> IconTheme {
        self.key = Some(key);
        self
    }

    /// Creates an icon theme that controls the properties of descendant widgets, and merges
    /// in the current icon theme, if any.
    pub fn merge(key: Option<KeyRef>, data: IconThemeData, child: WidgetRef) -> WidgetRef {
        Builder {
            key: None,
            builder: Rc::new(move |app, context| {
                IconTheme {
                    key: key.clone(),
                    data: Self::get_inherited_icon_theme_data(app, context).merge(Some(&data)),
                    child: child.clone(),
                }
                .into_widget()
            }),
        }
        .into_widget()
    }

    /// The data from the closest instance of this class that encloses the given context, if
    /// any.
    ///
    /// If there is no ambient icon theme, defaults to [`IconThemeData::fallback`]. The
    /// returned [`IconThemeData`] is concrete (all values are non-null; see
    /// [`IconThemeData::is_concrete`]). Any properties on the ambient icon theme that are
    /// null get defaulted to the values specified on [`IconThemeData::fallback`].
    ///
    /// The `Theme` widget from the `material` library introduces an [`IconTheme`] widget set
    /// to the `ThemeData.iconTheme`, so in a Material Design application, this will typically
    /// default to the icon theme from the ambient `Theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = IconTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> IconThemeData {
        let icon_theme_data =
            Self::get_inherited_icon_theme_data(app, context).resolve(app, context);
        if icon_theme_data.is_concrete() {
            return icon_theme_data;
        }
        let fallback = IconThemeData::fallback();
        let mut concrete = icon_theme_data.copy_with();
        concrete.size = icon_theme_data.size.or(fallback.size);
        concrete.fill = icon_theme_data.fill.or(fallback.fill);
        concrete.weight = icon_theme_data.weight.or(fallback.weight);
        concrete.grade = icon_theme_data.grade.or(fallback.grade);
        concrete.optical_size = icon_theme_data.optical_size.or(fallback.optical_size);
        concrete.color = icon_theme_data.color.or(fallback.color);
        concrete.opacity = icon_theme_data.opacity.or(fallback.opacity);
        concrete.shadows = icon_theme_data.shadows.clone().or(fallback.shadows);
        concrete.apply_text_scaling = icon_theme_data
            .apply_text_scaling
            .or(fallback.apply_text_scaling);
        concrete
    }

    fn get_inherited_icon_theme_data(app: &mut App, context: BuildContext) -> IconThemeData {
        let icon_theme = context.depend_on_inherited_widget_of_exact_type::<IconTheme>(app);
        icon_theme.map_or_else(IconThemeData::fallback, |icon_theme| {
            icon_theme.data.clone()
        })
    }
}

impl InheritedWidget for IconTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &IconTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::Color;
    use reveal_foundation::ValueKey;

    use super::*;
    use crate::framework::StatelessWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    /// Reads [`IconTheme::of`] on every build and logs what it saw.
    #[derive(Debug)]
    struct IconThemeReader {
        seen: Rc<RefCell<Vec<IconThemeData>>>,
    }

    impl StatelessWidget for IconThemeReader {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            self.seen.borrow_mut().push(IconTheme::of(app, context));
            SizedBox::shrink(None).into_widget()
        }
    }

    /// One reader instance, reused like a `const` widget, so only the theme decides rebuilds.
    fn reader() -> (WidgetRef, Rc<RefCell<Vec<IconThemeData>>>) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let reader = IconThemeReader {
            seen: Rc::clone(&seen),
        }
        .into_widget();
        (reader, seen)
    }

    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        let harness = Harness::mount(app, child);
        harness.pump(app);
        harness
    }

    #[test]
    fn of_returns_the_fallback_outside_any_icon_theme() {
        let mut app = App::new();
        let (reader, seen) = reader();
        mount(&mut app, reader);
        assert_eq!(seen.borrow().as_slice(), [IconThemeData::fallback()]);
    }

    #[test]
    fn of_fills_a_partial_theme_from_the_fallback() {
        let mut app = App::new();
        let (reader, seen) = reader();
        let partial = IconThemeData::new().size(16.0).opacity(0.5);
        mount(&mut app, IconTheme::new(partial, reader).into_widget());
        let concrete = IconThemeData::fallback().size(16.0).opacity(0.5);
        assert!(concrete.is_concrete());
        assert_eq!(seen.borrow().as_slice(), [concrete]);
    }

    #[test]
    fn a_changed_data_notifies_the_dependent_and_an_equal_one_does_not() {
        let mut app = App::new();
        let (reader, seen) = reader();
        let themed = |size: f64| {
            IconTheme::new(IconThemeData::new().size(size), reader.clone()).into_widget()
        };
        let harness = mount(&mut app, themed(16.0));
        assert_eq!(seen.borrow().len(), 1);
        assert_eq!(seen.borrow()[0].size, Some(16.0));

        harness.set_child(&mut app, themed(16.0));
        harness.pump(&mut app);
        assert_eq!(seen.borrow().len(), 1, "an equal data does not notify");

        harness.set_child(&mut app, themed(20.0));
        harness.pump(&mut app);
        assert_eq!(
            seen.borrow().len(),
            2,
            "a changed data rebuilds the dependent"
        );
        assert_eq!(seen.borrow()[1].size, Some(20.0));
    }

    #[test]
    fn merge_layers_its_data_over_the_ambient_theme() {
        let mut app = App::new();
        let (reader, seen) = reader();
        let red = Color::new(0xFFFF0000);
        let key: KeyRef = Rc::new(ValueKey::new("merged"));
        mount(
            &mut app,
            IconTheme::new(
                IconThemeData::new().size(16.0).color(red),
                IconTheme::merge(Some(key), IconThemeData::new().size(20.0), reader),
            )
            .into_widget(),
        );
        assert_eq!(
            seen.borrow().as_slice(),
            [IconThemeData::fallback().size(20.0).color(red)]
        );
    }
}
