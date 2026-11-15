//! Flutter counterpart: material_ui's `divider.dart`.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{Border, BorderRadiusGeometry, BorderSide, BoxDecoration, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, Center, Container, IntoWidget, KeyRef, SizedBox, StatelessWidget, WidgetRef,
};

use crate::divider_theme::{DividerTheme, DividerThemeData, DividerThemeDataGetters};
use crate::generated::divider_defaults_m3::DividerDefaultsM3;
use crate::theme::Theme;

/// A thin horizontal line, with padding on either side.
///
/// In the Material Design language, this represents a divider. Dividers can be
/// used in lists, `Drawer`s, and elsewhere to separate content.
///
/// To create a divider between `ListTile` items, consider using
/// `ListTile::divide_tiles`, which is optimized for this case.
///
/// The box's total height is controlled by [`height`](Self::height). The appropriate
/// padding is automatically computed from the height.
///
/// See also:
///
///  * `PopupMenuDivider`, which is the equivalent but for popup menus.
///  * `ListTile::divide_tiles`, another approach to dividing widgets in a list.
///  * [`VerticalDivider`], which is the vertical analog of this widget.
///  * <https://material.io/design/components/dividers.html>
#[derive(Debug, Default)]
pub struct Divider {
    pub key: Option<KeyRef>,

    /// The divider's height extent.
    ///
    /// The divider itself is always drawn as a horizontal line that is centered
    /// within the height specified by this value.
    ///
    /// If this is null, then the [`DividerThemeData::space`] is used. If that is
    /// also null, then this defaults to 16.0.
    pub height: Option<f64>,

    /// The thickness of the line drawn within the divider.
    ///
    /// A divider with a [`thickness`](Self::thickness) of 0.0 is always drawn as a line with a
    /// height of exactly one device pixel.
    ///
    /// If this is null, then the [`DividerThemeData::thickness`] is used. If
    /// that is also null, then this defaults to 0.0.
    pub thickness: Option<f64>,

    /// The amount of empty space to the leading edge of the divider.
    ///
    /// If this is null, then the [`DividerThemeData::indent`] is used. If that is
    /// also null, then this defaults to 0.0.
    pub indent: Option<f64>,

    /// The amount of empty space to the trailing edge of the divider.
    ///
    /// If this is null, then the [`DividerThemeData::end_indent`] is used. If that is
    /// also null, then this defaults to 0.0.
    pub end_indent: Option<f64>,

    /// The amount of radius for the border of the divider.
    ///
    /// If this is null, then [`DividerThemeData::radius`] is used. If that is
    /// also null, then the default radius of `BoxDecoration` is used.
    pub radius: Option<BorderRadiusGeometry>,

    /// The color to use when painting the line.
    ///
    /// If this is null, then the [`DividerThemeData::color`] is used. If that is
    /// also null, then this defaults to `ColorScheme::outline_variant`, or to
    /// `ThemeData::divider_color` if `ThemeData::use_material3` is false.
    pub color: Option<Color>,
}

impl Divider {
    /// Creates a Material Design divider.
    ///
    /// The [`height`](Self::height), [`thickness`](Self::thickness), [`indent`](Self::indent),
    /// and [`end_indent`](Self::end_indent) must be null or non-negative. Dart's optional named
    /// arguments are the fluent setters.
    pub fn new() -> Divider {
        Divider::default()
    }

    /// Dart `Divider(key:)`.
    pub fn key(mut self, key: KeyRef) -> Divider {
        self.key = Some(key);
        self
    }

    /// Dart `Divider(height:)`.
    pub fn height(mut self, height: f64) -> Divider {
        debug_assert!(height >= 0.0);
        self.height = Some(height);
        self
    }

    /// Dart `Divider(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> Divider {
        debug_assert!(thickness >= 0.0);
        self.thickness = Some(thickness);
        self
    }

    /// Dart `Divider(indent:)`.
    pub fn indent(mut self, indent: f64) -> Divider {
        debug_assert!(indent >= 0.0);
        self.indent = Some(indent);
        self
    }

    /// Dart `Divider(endIndent:)`.
    pub fn end_indent(mut self, end_indent: f64) -> Divider {
        debug_assert!(end_indent >= 0.0);
        self.end_indent = Some(end_indent);
        self
    }

    /// Dart `Divider(radius:)`.
    pub fn radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> Divider {
        self.radius = Some(radius.into());
        self
    }

    /// Dart `Divider(color:)`.
    pub fn color(mut self, color: Color) -> Divider {
        self.color = Some(color);
        self
    }

    /// Computes the [`BorderSide`] that represents a divider.
    ///
    /// If `color` is null, then [`DividerThemeData::color`] is used. If that is also
    /// null, then if `ThemeData::use_material3` is true then it defaults to
    /// `ThemeData::color_scheme`'s `ColorScheme::outline_variant`. Otherwise
    /// `ThemeData::divider_color` is used.
    ///
    /// If `width` is null, then [`DividerThemeData::thickness`] is used. If that is
    /// also null, then this defaults to 0.0 (a hairline border).
    ///
    /// If `context` is null, the default color of [`BorderSide`] is used and the
    /// default width of 0.0 is used.
    ///
    /// This example uses this method to create a box that has a divider above and
    /// below it. This is sometimes useful with lists, for instance, to separate a
    /// scrollable section from the rest of the interface.
    ///
    /// ```text
    /// DecoratedBox::new(BoxDecoration::new().border(Border {
    ///     top: Divider::create_border_side(app, Some(context), None, None),
    ///     bottom: Divider::create_border_side(app, Some(context), None, None),
    ///     ..Border::default()
    /// }))
    /// ```
    pub fn create_border_side(
        app: &mut App,
        context: Option<BuildContext>,
        color: Option<Color>,
        width: Option<f64>,
    ) -> BorderSide {
        let divider_theme = context.map(|context| DividerTheme::of(app, context));
        let defaults = context.map(|context| divider_defaults(app, context));
        let effective_color = color
            .or(divider_theme.as_ref().and_then(|theme| theme.color))
            .or_else(|| defaults.as_ref().and_then(|defaults| defaults.color(app)));
        let effective_width = width
            .or(divider_theme.as_ref().and_then(|theme| theme.thickness))
            .or_else(|| {
                defaults
                    .as_ref()
                    .and_then(|defaults| defaults.thickness(app))
            })
            .unwrap_or(0.0);

        // Prevent assertion since it is possible that context is null and no color
        // is specified.
        match effective_color {
            None => BorderSide {
                width: effective_width,
                ..BorderSide::default()
            },
            Some(color) => BorderSide {
                color,
                width: effective_width,
                ..BorderSide::default()
            },
        }
    }
}

/// The defaults of the theme in `context`: Material 3's, or Material 2's when
/// `ThemeData::use_material3` is false.
fn divider_defaults(app: &mut App, context: BuildContext) -> Box<dyn DividerThemeDataGetters> {
    if Theme::of(app, context).use_material3 {
        Box::new(DividerDefaultsM3::new(context))
    } else {
        Box::new(DividerDefaultsM2::new(context))
    }
}

impl StatelessWidget for Divider {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let divider_theme = DividerTheme::of(app, context);
        let defaults = divider_defaults(app, context);
        let height = self
            .height
            .or(divider_theme.space)
            .or_else(|| defaults.space(app))
            .expect("the defaults give a space");
        let thickness = self
            .thickness
            .or(divider_theme.thickness)
            .or_else(|| defaults.thickness(app))
            .expect("the defaults give a thickness");
        let indent = self
            .indent
            .or(divider_theme.indent)
            .or_else(|| defaults.indent(app))
            .expect("the defaults give an indent");
        let end_indent = self
            .end_indent
            .or(divider_theme.end_indent)
            .or_else(|| defaults.end_indent(app))
            .expect("the defaults give an end indent");

        SizedBox::new()
            .height(height)
            .child(
                Center::new().child(
                    Container::new()
                        .height(thickness)
                        .margin(EdgeInsetsGeometry::directional(
                            indent, 0.0, end_indent, 0.0,
                        ))
                        .decoration(divider_decoration(
                            self.radius
                                .or(divider_theme.radius)
                                .or_else(|| defaults.radius(app)),
                            Border {
                                bottom: Divider::create_border_side(
                                    app,
                                    Some(context),
                                    self.color,
                                    Some(thickness),
                                ),
                                ..Border::default()
                            },
                        )),
                ),
            )
            .into_widget()
    }
}

/// `BoxDecoration(borderRadius: radius, border: border)`.
fn divider_decoration(radius: Option<BorderRadiusGeometry>, border: Border) -> BoxDecoration {
    let decoration = BoxDecoration::new().border(border);
    match radius {
        Some(radius) => decoration.border_radius(radius),
        None => decoration,
    }
}

/// A thin vertical line, with padding on either side.
///
/// In the Material Design language, this represents a divider. Vertical
/// dividers can be used in horizontally scrolling lists, such as a
/// `ListView` with `ListView::scroll_direction` set to `Axis::Horizontal`.
///
/// The box's total width is controlled by [`width`](Self::width). The appropriate
/// padding is automatically computed from the width.
///
/// See also:
///
///  * `ListView::separated`, which can be used to generate vertical dividers.
///  * [`Divider`], which is the horizontal analog of this widget.
///  * <https://material.io/design/components/dividers.html>
#[derive(Debug, Default)]
pub struct VerticalDivider {
    pub key: Option<KeyRef>,

    /// The divider's width.
    ///
    /// The divider itself is always drawn as a vertical line that is centered
    /// within the width specified by this value.
    ///
    /// If this is null, then the [`DividerThemeData::space`] is used. If that is
    /// also null, then this defaults to 16.0.
    pub width: Option<f64>,

    /// The thickness of the line drawn within the divider.
    ///
    /// A divider with a [`thickness`](Self::thickness) of 0.0 is always drawn as a line with a
    /// width of exactly one device pixel.
    ///
    /// If this is null, then the [`DividerThemeData::thickness`] is used which
    /// defaults to 0.0.
    pub thickness: Option<f64>,

    /// The amount of empty space on top of the divider.
    ///
    /// If this is null, then the [`DividerThemeData::indent`] is used. If that is
    /// also null, then this defaults to 0.0.
    pub indent: Option<f64>,

    /// The amount of empty space under the divider.
    ///
    /// If this is null, then the [`DividerThemeData::end_indent`] is used. If that is
    /// also null, then this defaults to 0.0.
    pub end_indent: Option<f64>,

    /// The color to use when painting the line.
    ///
    /// If this is null, then the [`DividerThemeData::color`] is used. If that is
    /// also null, then this defaults to `ColorScheme::outline_variant`, or to
    /// `ThemeData::divider_color` if `ThemeData::use_material3` is false.
    pub color: Option<Color>,

    /// The amount of radius for the border of the divider.
    ///
    /// If this is null, then the default radius of `BoxDecoration` will be used.
    pub radius: Option<BorderRadiusGeometry>,
}

impl VerticalDivider {
    /// Creates a Material Design vertical divider.
    ///
    /// The [`width`](Self::width), [`thickness`](Self::thickness), [`indent`](Self::indent),
    /// and [`end_indent`](Self::end_indent) must be null or non-negative. Dart's optional named
    /// arguments are the fluent setters.
    pub fn new() -> VerticalDivider {
        VerticalDivider::default()
    }

    /// Dart `VerticalDivider(key:)`.
    pub fn key(mut self, key: KeyRef) -> VerticalDivider {
        self.key = Some(key);
        self
    }

    /// Dart `VerticalDivider(width:)`.
    pub fn width(mut self, width: f64) -> VerticalDivider {
        debug_assert!(width >= 0.0);
        self.width = Some(width);
        self
    }

    /// Dart `VerticalDivider(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> VerticalDivider {
        debug_assert!(thickness >= 0.0);
        self.thickness = Some(thickness);
        self
    }

    /// Dart `VerticalDivider(indent:)`.
    pub fn indent(mut self, indent: f64) -> VerticalDivider {
        debug_assert!(indent >= 0.0);
        self.indent = Some(indent);
        self
    }

    /// Dart `VerticalDivider(endIndent:)`.
    pub fn end_indent(mut self, end_indent: f64) -> VerticalDivider {
        debug_assert!(end_indent >= 0.0);
        self.end_indent = Some(end_indent);
        self
    }

    /// Dart `VerticalDivider(color:)`.
    pub fn color(mut self, color: Color) -> VerticalDivider {
        self.color = Some(color);
        self
    }

    /// Dart `VerticalDivider(radius:)`.
    pub fn radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> VerticalDivider {
        self.radius = Some(radius.into());
        self
    }
}

impl StatelessWidget for VerticalDivider {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let divider_theme = DividerTheme::of(app, context);
        let defaults = divider_defaults(app, context);
        let width = self
            .width
            .or(divider_theme.space)
            .or_else(|| defaults.space(app))
            .expect("the defaults give a space");
        let thickness = self
            .thickness
            .or(divider_theme.thickness)
            .or_else(|| defaults.thickness(app))
            .expect("the defaults give a thickness");
        let indent = self
            .indent
            .or(divider_theme.indent)
            .or_else(|| defaults.indent(app))
            .expect("the defaults give an indent");
        let end_indent = self
            .end_indent
            .or(divider_theme.end_indent)
            .or_else(|| defaults.end_indent(app))
            .expect("the defaults give an end indent");

        SizedBox::new()
            .width(width)
            .child(
                Center::new().child(
                    Container::new()
                        .width(thickness)
                        .margin(EdgeInsetsGeometry::directional(
                            0.0, indent, 0.0, end_indent,
                        ))
                        .decoration(divider_decoration(
                            self.radius
                                .or(divider_theme.radius)
                                .or_else(|| defaults.radius(app)),
                            Border {
                                left: Divider::create_border_side(
                                    app,
                                    Some(context),
                                    self.color,
                                    Some(thickness),
                                ),
                                ..Border::default()
                            },
                        )),
                ),
            )
            .into_widget()
    }
}

/// Dart's `_DividerDefaultsM2`.
struct DividerDefaultsM2 {
    /// Dart's `super(..)` arguments.
    divider_theme_data: DividerThemeData,
    context: BuildContext,
}

impl DividerDefaultsM2 {
    fn new(context: BuildContext) -> DividerDefaultsM2 {
        DividerDefaultsM2 {
            divider_theme_data: DividerThemeData::new()
                .space(16.0)
                .thickness(0.0)
                .indent(0.0)
                .end_indent(0.0),
            context,
        }
    }
}

impl DividerThemeDataGetters for DividerDefaultsM2 {
    fn divider_theme_data(&self) -> &DividerThemeData {
        &self.divider_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(Theme::of(app, self.context).divider_color)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/divider_test.dart`.

    use waft_embedder::{Radius, Rect, Size, TextDirection};
    use waft_foundation::AppCell;
    use waft_painting::BorderRadius;
    use waft_widgets::{AnyElement, DecoratedBox, Directionality, Row, Text};

    use super::*;
    use crate::material::Material;
    use crate::test_support::{Drawn, WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    pub(crate) use crate::tooltip::tests::tester;
    use crate::tooltip::tests::{m2, m3, pump_app};

    /// Dart's `tester.widget<Container>(find.byType(Container))`, which expects one `Container`.
    pub(crate) fn only_container(tester: &WidgetTester) -> AnyElement {
        let mut containers = tester.find_by_type::<Container>();
        assert_eq!(containers.len(), 1, "one Container");
        containers.remove(0)
    }

    /// The `BoxDecoration` of [`only_container`].
    pub(crate) fn decoration(tester: &WidgetTester) -> BoxDecoration {
        let container = only_container(tester);
        tester.widget(container, |container: &Container| {
            container
                .decoration
                .as_ref()
                .expect("the container has a decoration")
                .as_any()
                .downcast_ref::<BoxDecoration>()
                .expect("a BoxDecoration")
                .clone()
        })
    }

    /// The `Border` of [`decoration`].
    pub(crate) fn border(tester: &WidgetTester) -> Border {
        *decoration(tester)
            .border
            .expect("the decoration has a border")
            .as_any()
            .downcast_ref::<Border>()
            .expect("a Border")
    }

    /// Dart's `tester.getRect(find.byType(W))`.
    pub(crate) fn rect_of<W: 'static>(tester: &WidgetTester) -> Rect {
        let element = *tester
            .find_by_type::<W>()
            .first()
            .expect("a widget of that type");
        tester.global_rect(tester.render_box_of(element))
    }

    /// The border radius of [`decoration`], which is a `BorderRadius`.
    pub(crate) fn border_radius(tester: &WidgetTester) -> BorderRadius {
        match decoration(tester).border_radius.expect("a border radius") {
            BorderRadiusGeometry::BorderRadius(radius) => radius,
            _ => panic!("a BorderRadius"),
        }
    }

    pub(crate) fn assert_all_corners(radius: BorderRadius, expected: f64) {
        assert_eq!(radius.bottom_left, Radius::circular(expected));
        assert_eq!(radius.bottom_right, Radius::circular(expected));
        assert_eq!(radius.top_left, Radius::circular(expected));
        assert_eq!(radius.top_right, Radius::circular(expected));
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, Center::new().child(child)).into_widget()
    }

    fn pump_centered(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData, child: WidgetRef) {
        pump_app(tester, theme, move |_app| {
            Center::new().child(child).into_widget()
        });
    }

    #[test]
    fn material3_divider_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_centered(&mut tester, m3, Divider::new().into_widget());
            assert_eq!(rect_of::<Divider>(&tester).height(), 16.0);
            assert_eq!(border(&tester).bottom.width, 1.0);
        });
    }

    #[test]
    fn material2_divider_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_centered(&mut tester, m2, Divider::new().into_widget());
            assert_eq!(rect_of::<Divider>(&tester).height(), 16.0);
            assert_eq!(border(&tester).bottom.width, 0.0);
        });
    }

    #[test]
    fn divider_custom_thickness() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ltr(Divider::new().thickness(5.0)));
            assert_eq!(border(&tester).bottom.width, 5.0);
        });
    }

    #[test]
    fn divider_custom_radius() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ltr(Divider::new().radius(BorderRadius::circular(5.0))));
            assert_all_corners(border_radius(&tester), 5.0);
        });
    }

    #[test]
    fn horizontal_divider_custom_indentation() {
        with_main_thread_stack(|| {
            let custom_indent = 10.0;
            let mut tester = tester();

            tester.pump_widget(ltr(Divider::new().indent(custom_indent)));
            // The divider line is drawn with a DecoratedBox with a border
            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left + custom_indent);
            assert_eq!(line_rect.right, divider_rect.right);

            tester.pump_widget(ltr(Divider::new().end_indent(custom_indent)));
            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left);
            assert_eq!(line_rect.right, divider_rect.right - custom_indent);

            tester.pump_widget(ltr(Divider::new()
                .indent(custom_indent)
                .end_indent(custom_indent)));
            let divider_rect = rect_of::<Divider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.left, divider_rect.left + custom_indent);
            assert_eq!(line_rect.right, divider_rect.right - custom_indent);
        });
    }

    #[test]
    fn material3_vertical_divider_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_centered(&mut tester, m3, VerticalDivider::new().into_widget());
            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), 16.0);
            assert_eq!(border(&tester).left.width, 1.0);
        });
    }

    #[test]
    fn material2_vertical_divider_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_centered(&mut tester, m2, VerticalDivider::new().into_widget());
            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), 16.0);
            assert_eq!(border(&tester).left.width, 0.0);
        });
    }

    #[test]
    fn vertical_divider_custom_thickness() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ltr(VerticalDivider::new().thickness(5.0)));
            assert_eq!(border(&tester).left.width, 5.0);
        });
    }

    #[test]
    fn vertical_divider_test_2() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m2, |_app| {
                Material::new()
                    .child(SizedBox::new().height(24.0).child(Row::new().children([
                        Text::new("Hey.").into_widget(),
                        VerticalDivider::new().into_widget(),
                    ])))
                    .into_widget()
            });
            let container_box = *tester
                .find_by_type::<Container>()
                .last()
                .expect("a Container");
            let container_size: Size = tester
                .render_box_of(container_box)
                .size(&tester.cell.borrow());

            assert_eq!(rect_of::<VerticalDivider>(&tester).width(), 16.0);
            assert_eq!(container_size.height(), 600.0);
            // Dart: `paints..path(strokeWidth: 0.0)`; `Drawn` does not carry the stroke width.
            let divider = *tester
                .find_by_type::<VerticalDivider>()
                .first()
                .expect("a VerticalDivider");
            let drawn = tester.painted(tester.render_box_of(divider).as_object());
            assert!(matches!(drawn.as_slice(), [Drawn::Path(..)]), "{drawn:?}");
        });
    }

    #[test]
    fn vertical_divider_custom_indentation() {
        with_main_thread_stack(|| {
            let custom_indent = 10.0;
            let mut tester = tester();

            tester.pump_widget(ltr(VerticalDivider::new().indent(custom_indent)));
            // The divider line is drawn with a DecoratedBox with a border
            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top + custom_indent);
            assert_eq!(line_rect.bottom, divider_rect.bottom);

            tester.pump_widget(ltr(VerticalDivider::new().end_indent(custom_indent)));
            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top);
            assert_eq!(line_rect.bottom, divider_rect.bottom - custom_indent);

            tester.pump_widget(ltr(VerticalDivider::new()
                .indent(custom_indent)
                .end_indent(custom_indent)));
            let divider_rect = rect_of::<VerticalDivider>(&tester);
            let line_rect = rect_of::<DecoratedBox>(&tester);
            assert_eq!(line_rect.top, divider_rect.top + custom_indent);
            assert_eq!(line_rect.bottom, divider_rect.bottom - custom_indent);
        });
    }

    #[test]
    fn vertical_divider_custom_radius() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ltr(
                VerticalDivider::new().radius(BorderRadius::circular(5.0))
            ));
            assert_all_corners(border_radius(&tester), 5.0);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/39533
    #[test]
    fn create_border_side_does_not_throw_exception_with_null_context() {
        let cell = AppCell::new();
        let side = Divider::create_border_side(&mut cell.borrow_mut(), None, None, None);
        assert_eq!(
            side,
            BorderSide {
                width: 0.0,
                ..BorderSide::default()
            }
        );
    }

    #[test]
    fn divider_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(Divider::new()))
                    .into_widget()
            });
            assert_eq!(rect_of::<Divider>(&tester).size(), Size::ZERO);
        });
    }

    #[test]
    fn vertical_divider_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(VerticalDivider::new()))
                    .into_widget()
            });
            assert_eq!(rect_of::<VerticalDivider>(&tester).size(), Size::ZERO);
        });
    }
}
