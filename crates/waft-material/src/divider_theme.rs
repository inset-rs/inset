//! Flutter counterpart: material_ui's `divider_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::BorderRadiusGeometry;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Defines the visual properties of [`Divider`](crate::Divider), `VerticalDivider`,
/// `Divider::create_border_side`, and `ListTile`'s dividers.
///
/// Descendant widgets obtain the current [`DividerThemeData`] object using
/// [`DividerTheme::of`]. Instances of [`DividerThemeData`] can be customized with
/// [`DividerThemeData::copy_with`].
///
/// Typically a [`DividerThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::divider_theme`.
///
/// All [`DividerThemeData`] properties are `None` by default. When null, the `Divider`
/// will use the values from `ThemeData` if they exist, otherwise it will provide its own
/// defaults based on the overall `Theme`'s textTheme and colorScheme. See the individual
/// `Divider` properties for details.
///
/// See also:
///
///  * [`DividerTheme`], an [`InheritedWidget`] that propagates the theme down its subtree.
///  * `ThemeData::divider_theme`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DividerThemeData {
    /// The color of [`Divider`](crate::Divider)s and `VerticalDivider`s.
    pub color: Option<Color>,

    /// The `Divider`'s height or the `VerticalDivider`'s width.
    ///
    /// This represents the amount of horizontal or vertical space the divider takes up.
    pub space: Option<f64>,

    /// The thickness of the line drawn within the divider.
    pub thickness: Option<f64>,

    /// The amount of empty space at the leading edge of [`Divider`](crate::Divider) or
    /// top edge of `VerticalDivider`.
    pub indent: Option<f64>,

    /// The amount of empty space at the trailing edge of [`Divider`](crate::Divider) or
    /// bottom edge of `VerticalDivider`.
    pub end_indent: Option<f64>,

    /// The border radius of the [`Divider`](crate::Divider) and `VerticalDivider`.
    pub radius: Option<BorderRadiusGeometry>,
}

impl DividerThemeData {
    /// Creates a theme that can be used for `ThemeData::divider_theme`. Dart's optional
    /// named arguments are the fluent setters.
    pub fn new() -> DividerThemeData {
        DividerThemeData::default()
    }

    /// Dart `DividerThemeData(color:)`.
    pub fn color(mut self, color: Color) -> DividerThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `DividerThemeData(space:)`.
    pub fn space(mut self, space: f64) -> DividerThemeData {
        self.space = Some(space);
        self
    }

    /// Dart `DividerThemeData(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> DividerThemeData {
        self.thickness = Some(thickness);
        self
    }

    /// Dart `DividerThemeData(indent:)`.
    pub fn indent(mut self, indent: f64) -> DividerThemeData {
        self.indent = Some(indent);
        self
    }

    /// Dart `DividerThemeData(endIndent:)`.
    pub fn end_indent(mut self, end_indent: f64) -> DividerThemeData {
        self.end_indent = Some(end_indent);
        self
    }

    /// Dart `DividerThemeData(radius:)`.
    pub fn radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> DividerThemeData {
        self.radius = Some(radius.into());
        self
    }

    /// Creates a copy of this object with the given fields replaced with the new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().color(color)`).
    pub fn copy_with(&self) -> DividerThemeData {
        self.clone()
    }

    /// Linearly interpolate between two Divider themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&DividerThemeData>,
        b: Option<&DividerThemeData>,
        t: f64,
    ) -> DividerThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        DividerThemeData {
            color: Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
            space: lerp_double(a.and_then(|a| a.space), b.and_then(|b| b.space), t),
            thickness: lerp_double(a.and_then(|a| a.thickness), b.and_then(|b| b.thickness), t),
            indent: lerp_double(a.and_then(|a| a.indent), b.and_then(|b| b.indent), t),
            end_indent: lerp_double(
                a.and_then(|a| a.end_indent),
                b.and_then(|b| b.end_indent),
                t,
            ),
            radius: BorderRadiusGeometry::lerp(
                a.and_then(|a| a.radius),
                b.and_then(|b| b.radius),
                t,
            ),
        }
    }
}

/// An inherited widget that defines the configuration for
/// [`Divider`](crate::Divider)s, `VerticalDivider`s, dividers between `ListTile`s, and
/// dividers between rows in `DataTable`s in this widget's subtree.
#[derive(Debug)]
pub struct DividerTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `Divider` widgets.
    pub data: DividerThemeData,
    pub child: WidgetRef,
}

impl DividerTheme {
    /// Creates a divider theme that controls the configurations for
    /// [`Divider`](crate::Divider)s and `VerticalDivider`s in its widget subtree.
    pub fn new<K>(data: DividerThemeData, child: impl IntoWidget<K>) -> DividerTheme {
        DividerTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `DividerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DividerTheme {
        self.key = Some(key);
        self
    }

    /// The data from the closest instance of this class that encloses the given context.
    ///
    /// If there is no enclosing [`DividerTheme`] widget, then `ThemeData::divider_theme`
    /// is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = DividerTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> DividerThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<DividerTheme>(app) {
            Some(divider_theme) => divider_theme.data.clone(),
            None => Theme::of(app, context).divider_theme.clone(),
        }
    }
}

impl InheritedTheme for DividerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DividerTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for DividerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DividerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/divider_theme_test.dart`; the `debugFillProperties` cases wait for
    //! diagnostics, and `hashCode` for `Hash`.

    use waft_embedder::Radius;
    use waft_painting::{BorderRadius, BorderRadiusGeometry};
    use waft_widgets::DecoratedBox;

    use super::*;
    use crate::colors::Colors;
    use crate::divider::tests::{assert_all_corners, border, border_radius, rect_of, tester};
    use crate::divider::{Divider, VerticalDivider};
    use crate::scaffold::Scaffold;
    use crate::test_support::with_main_thread_stack;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app};

    fn divider_theme() -> DividerThemeData {
        DividerThemeData::new()
            .color(Colors::ORANGE.primary())
            .space(12.0)
            .thickness(2.0)
            .indent(7.0)
            .end_indent(5.0)
            .radius(BorderRadiusGeometry::only(
                Radius::circular(1.0),
                Radius::circular(2.0),
                Radius::circular(3.0),
                Radius::circular(4.0),
            ))
    }

    /// `Scaffold(body: child)`.
    fn body<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Scaffold::new().body(child).into_widget()
    }

    fn themed(divider_theme: DividerThemeData) -> impl Fn(&App) -> ThemeData {
        move |app| {
            ThemeData::builder()
                .divider_theme(divider_theme.clone())
                .build(app)
        }
    }

    fn pump_with_theme<K>(
        tester: &mut crate::test_support::WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        child: impl IntoWidget<K>,
    ) {
        pump_app(tester, theme, move |_app| body(child));
    }

    fn assert_theme_radii(radius: BorderRadius) {
        assert_eq!(radius.top_left, Radius::circular(1.0));
        assert_eq!(radius.top_right, Radius::circular(2.0));
        assert_eq!(radius.bottom_left, Radius::circular(3.0));
        assert_eq!(radius.bottom_right, Radius::circular(4.0));
    }

    #[test]
    fn divider_theme_data_copy_with_equality_basics() {
        assert_eq!(DividerThemeData::new(), DividerThemeData::new().copy_with());
    }

    #[test]
    fn divider_theme_data_null_fields_by_default() {
        let divider_theme = DividerThemeData::new();
        assert_eq!(divider_theme.color, None);
        assert_eq!(divider_theme.space, None);
        assert_eq!(divider_theme.thickness, None);
        assert_eq!(divider_theme.indent, None);
        assert_eq!(divider_theme.end_indent, None);
        assert_eq!(divider_theme.radius, None);
    }

    #[test]
    fn material3_horizontal_divider_passing_no_divider_theme_data_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_app(&mut tester, m3, |_app| body(Divider::new()));

            assert_eq!(rect_of::<Divider>(&tester).height(), 16.0);
            let border = border(&tester);
            assert_eq!(border.bottom.width, 1.0);
            assert_eq!(border.bottom.color, theme.color_scheme.outline_variant());

            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left);
            assert_eq!(line_rect.right, divider_rect.right);
        });
    }

    #[test]
    fn material3_horizontal_divider_uses_values_from_divider_theme_data() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let divider_theme = divider_theme();
            pump_with_theme(&mut tester, themed(divider_theme.clone()), Divider::new());

            assert_eq!(
                rect_of::<Divider>(&tester).height(),
                divider_theme.space.expect("a space")
            );
            let border = border(&tester);
            assert_eq!(
                border.bottom.width,
                divider_theme.thickness.expect("a thickness")
            );
            assert_eq!(border.bottom.color, divider_theme.color.expect("a color"));

            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(
                line_rect.left,
                divider_rect.left + divider_theme.indent.expect("an indent")
            );
            assert_eq!(
                line_rect.right,
                divider_rect.right - divider_theme.end_indent.expect("an end indent")
            );
            assert_theme_radii(border_radius(&tester));
        });
    }

    #[test]
    fn material3_horizontal_divider_divider_theme_overrides_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let divider_theme = divider_theme();
            pump_app(&mut tester, m3, |_app| {
                body(DividerTheme::new(divider_theme.clone(), Divider::new()))
            });

            let border = border(&tester);
            assert_eq!(
                border.bottom.width,
                divider_theme.thickness.expect("a thickness")
            );
            assert_eq!(border.bottom.color, divider_theme.color.expect("a color"));
        });
    }

    #[test]
    fn material3_horizontal_divider_widget_properties_take_priority_over_theme() {
        with_main_thread_stack(|| {
            let color = Colors::PURPLE.primary();
            let (height, thickness, indent, end_indent) = (10.0, 5.0, 8.0, 9.0);
            let mut tester = tester();
            pump_with_theme(
                &mut tester,
                themed(divider_theme()),
                Divider::new()
                    .color(color)
                    .height(height)
                    .thickness(thickness)
                    .indent(indent)
                    .end_indent(end_indent)
                    .radius(BorderRadiusGeometry::all(Radius::circular(5.0))),
            );

            assert_eq!(rect_of::<Divider>(&tester).height(), height);
            let border = border(&tester);
            assert_eq!(border.bottom.width, thickness);
            assert_eq!(border.bottom.color, color);

            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left + indent);
            assert_eq!(line_rect.right, divider_rect.right - end_indent);
            assert_all_corners(border_radius(&tester), 5.0);
        });
    }

    #[test]
    fn material3_vertical_divider_passing_no_divider_theme_data_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_app(&mut tester, m3, |_app| body(VerticalDivider::new()));

            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), 16.0);
            let border = border(&tester);
            assert_eq!(border.left.width, 1.0);
            assert_eq!(border.left.color, theme.color_scheme.outline_variant());

            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top);
            assert_eq!(line_rect.bottom, divider_rect.bottom);
        });
    }

    #[test]
    fn material3_vertical_divider_uses_values_from_divider_theme_data() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let divider_theme = divider_theme();
            pump_with_theme(
                &mut tester,
                themed(divider_theme.clone()),
                VerticalDivider::new(),
            );

            assert_eq!(
                rect_of::<VerticalDivider>(&tester).width(),
                divider_theme.space.expect("a space")
            );
            let border = border(&tester);
            assert_eq!(
                border.left.width,
                divider_theme.thickness.expect("a thickness")
            );
            assert_eq!(border.left.color, divider_theme.color.expect("a color"));

            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(
                line_rect.top,
                divider_rect.top + divider_theme.indent.expect("an indent")
            );
            assert_eq!(
                line_rect.bottom,
                divider_rect.bottom - divider_theme.end_indent.expect("an end indent")
            );
            assert_theme_radii(border_radius(&tester));
        });
    }

    #[test]
    fn material3_vertical_divider_divider_theme_overrides_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let divider_theme = divider_theme();
            pump_app(&mut tester, m3, |_app| {
                body(DividerTheme::new(
                    divider_theme.clone(),
                    VerticalDivider::new(),
                ))
            });

            let border = border(&tester);
            assert_eq!(
                border.left.width,
                divider_theme.thickness.expect("a thickness")
            );
            assert_eq!(border.left.color, divider_theme.color.expect("a color"));
        });
    }

    #[test]
    fn material3_vertical_divider_widget_properties_take_priority_over_theme() {
        with_main_thread_stack(|| {
            let color = Colors::PURPLE.primary();
            let (width, thickness, indent, end_indent) = (10.0, 5.0, 8.0, 9.0);
            let mut tester = tester();
            pump_with_theme(
                &mut tester,
                themed(divider_theme()),
                VerticalDivider::new()
                    .color(color)
                    .width(width)
                    .thickness(thickness)
                    .indent(indent)
                    .end_indent(end_indent)
                    .radius(BorderRadiusGeometry::all(Radius::circular(5.0))),
            );

            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), width);
            let border = border(&tester);
            assert_eq!(border.left.width, thickness);
            assert_eq!(border.left.color, color);

            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top + indent);
            assert_eq!(line_rect.bottom, divider_rect.bottom - end_indent);
            assert_all_corners(border_radius(&tester), 5.0);
        });
    }

    #[test]
    fn material2_horizontal_divider_passing_no_divider_theme_data_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            pump_app(&mut tester, m2, |_app| body(Divider::new()));

            assert_eq!(rect_of::<Divider>(&tester).height(), 16.0);
            let border = border(&tester);
            assert_eq!(border.bottom.width, 0.0);
            assert_eq!(border.bottom.color, theme.divider_color);

            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left);
            assert_eq!(line_rect.right, divider_rect.right);
        });
    }

    #[test]
    fn material2_horizontal_divider_divider_theme_overrides_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = divider_theme();
            pump_app(&mut tester, m3, |_app| {
                body(DividerTheme::new(theme.clone(), Divider::new()))
            });

            let border = border(&tester);
            assert_eq!(border.bottom.width, theme.thickness.expect("a thickness"));
            assert_eq!(border.bottom.color, theme.color.expect("a color"));
            assert_theme_radii(border_radius(&tester));
        });
    }

    #[test]
    fn material2_vertical_divider_passing_no_divider_theme_data_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            pump_app(&mut tester, m2, |_app| body(VerticalDivider::new()));

            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), 16.0);
            let border = border(&tester);
            assert_eq!(border.left.width, 0.0);
            assert_eq!(border.left.color, theme.divider_color);

            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top);
            assert_eq!(line_rect.bottom, divider_rect.bottom);
        });
    }
}
