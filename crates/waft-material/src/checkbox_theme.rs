//! Flutter counterpart: material_ui's `checkbox_theme.dart`.

use std::rc::Rc;

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyBorderSide, BorderSide, EdgeInsets, OutlinedBorder, ShapeBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetStateBorderSide,
    WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Defines default property values for descendant `Checkbox` widgets.
///
/// Descendant widgets obtain the current [`CheckboxThemeData`] object using
/// [`CheckboxTheme::of`]. Instances of [`CheckboxThemeData`] can be
/// customized with [`CheckboxThemeData::copy_with`].
///
/// Typically a [`CheckboxThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::checkbox_theme`.
///
/// All [`CheckboxThemeData`] properties are `None` by default. When null, the
/// `Checkbox` will use the values from `ThemeData` if they exist, otherwise it
/// will provide its own defaults based on the overall `Theme`'s color_scheme.
/// See the individual `Checkbox` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct CheckboxThemeData {
    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If `mouse_cursor` is a `WidgetStateMouseCursor`,
    /// [`WidgetStateProperty::resolve`] is used for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    ///
    /// If specified, overrides the default value of `Checkbox::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// The color that fills the checkbox, in all `WidgetState`s.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    ///
    /// If specified, overrides the default value of `Checkbox::fill_color`.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color to use for the check icon when this checkbox is checked.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    ///
    /// If specified, overrides the default value of `Checkbox::check_color`.
    pub check_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color for the checkbox's `Material`.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Pressed`.
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///
    /// If specified, overrides the default value of `Checkbox::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The splash radius of the circular `Material` ink response.
    ///
    /// If specified, overrides the default value of `Checkbox::splash_radius`.
    pub splash_radius: Option<f64>,

    /// Configures the minimum size of the tap target.
    ///
    /// If specified, overrides the default value of
    /// `Checkbox::material_tap_target_size`.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,

    /// Defines how compact the checkbox's layout will be.
    ///
    /// If specified, overrides the default value of `Checkbox::visual_density`.
    pub visual_density: Option<VisualDensity>,

    /// The shape of the checkbox's `Material`.
    ///
    /// If specified, overrides the default value of `Checkbox::shape`.
    pub shape: Option<Box<dyn OutlinedBorder>>,

    /// The color and width of the checkbox's border.
    ///
    /// This property can be a [`WidgetStateBorderSide`] that can
    /// specify different border color and widths depending on the
    /// checkbox's state.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Pressed`.
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Error`.
    ///
    /// If this property is not a [`WidgetStateBorderSide`] and it is
    /// non-null, then it is only rendered when the checkbox's value is
    /// false. The difference in interpretation is for backwards
    /// compatibility.
    ///
    /// If specified, overrides the default value of `Checkbox::side`.
    pub side: Option<AnyBorderSide>,

    /// The insets applied around the check mark when the checkbox is checked or
    /// in its indeterminate (tristate) state.
    ///
    /// This insets the check mark inward; the size of the checkbox box itself
    /// is not affected. The check mark and its stroke shrink to fit the
    /// remaining space, so the mark stays proportional. Its size is the smaller
    /// of the remaining width and height, and it is centered in the remaining
    /// space, so an inset on one axis also shrinks it on the other. For example,
    /// `EdgeInsets::only(9.0, 0.0, 0.0, 0.0)` draws a half-size mark centered in the right
    /// half of the box.
    ///
    /// On iOS and macOS, when using `Checkbox::adaptive`, this property has no
    /// effect, because the checkbox delegates to `CupertinoCheckbox`, which does
    /// not support insets.
    ///
    /// Must be non-negative. If the horizontal or vertical insets are
    /// greater than or equal to the checkbox size, the check mark or indeterminate
    /// dash is not painted.
    ///
    /// If null, [`CheckboxThemeData::mark_insets`] is used. If that is also null,
    /// it defaults to [`EdgeInsets::ZERO`] and the check mark renders at full size.
    pub mark_insets: Option<EdgeInsets>,
}

impl CheckboxThemeData {
    /// Creates a theme that can be used for `ThemeData::checkbox_theme`; Dart's optional
    /// named arguments are the fluent setters.
    pub fn new() -> CheckboxThemeData {
        CheckboxThemeData::default()
    }

    /// Dart `CheckboxThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> CheckboxThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CheckboxThemeData(fillColor:)`.
    pub fn fill_color(
        mut self,
        fill_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CheckboxThemeData {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `CheckboxThemeData(checkColor:)`.
    pub fn check_color(
        mut self,
        check_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CheckboxThemeData {
        self.check_color = Some(check_color);
        self
    }

    /// Dart `CheckboxThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CheckboxThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `CheckboxThemeData(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> CheckboxThemeData {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `CheckboxThemeData(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> CheckboxThemeData {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `CheckboxThemeData(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> CheckboxThemeData {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `CheckboxThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn OutlinedBorder>>) -> CheckboxThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `CheckboxThemeData(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> CheckboxThemeData {
        self.side = Some(side.into());
        self
    }

    /// Dart `CheckboxThemeData(markInsets:)`.
    pub fn mark_insets(mut self, mark_insets: EdgeInsets) -> CheckboxThemeData {
        self.mark_insets = Some(mark_insets);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().splash_radius(4.0)`).
    pub fn copy_with(&self) -> CheckboxThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`CheckboxThemeData`]s.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&CheckboxThemeData>,
        b: Option<&CheckboxThemeData>,
        t: f64,
    ) -> CheckboxThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        CheckboxThemeData {
            mouse_cursor: if t < 0.5 {
                a.and_then(|a| a.mouse_cursor.clone())
            } else {
                b.and_then(|b| b.mouse_cursor.clone())
            },
            fill_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.fill_color.clone()),
                b.and_then(|b| b.fill_color.clone()),
                t,
                Color::lerp,
            ),
            check_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.check_color.clone()),
                b.and_then(|b| b.check_color.clone()),
                t,
                Color::lerp,
            ),
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            splash_radius: lerp_double(
                a.and_then(|a| a.splash_radius),
                b.and_then(|b| b.splash_radius),
                t,
            ),
            material_tap_target_size: if t < 0.5 {
                a.and_then(|a| a.material_tap_target_size)
            } else {
                b.and_then(|b| b.material_tap_target_size)
            },
            visual_density: if t < 0.5 {
                a.and_then(|a| a.visual_density)
            } else {
                b.and_then(|b| b.visual_density)
            },
            shape: <dyn OutlinedBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
            side: Self::lerp_sides(
                a.and_then(|a| a.side.as_ref()),
                b.and_then(|b| b.side.as_ref()),
                t,
            )
            .map(AnyBorderSide::new),
            mark_insets: EdgeInsets::lerp(
                a.and_then(|a| a.mark_insets),
                b.and_then(|b| b.mark_insets),
                t,
            ),
        }
    }

    // Special case because BorderSide.lerp() doesn't support null arguments
    fn lerp_sides(
        a: Option<&AnyBorderSide>,
        b: Option<&AnyBorderSide>,
        t: f64,
    ) -> Option<BorderSide> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.and_then(Self::resolve_unconditionally);
        let b = b.and_then(Self::resolve_unconditionally);
        let a = a.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: b.expect("b! in Dart").color.with_alpha(0),
            ..BorderSide::default()
        });
        let b = b.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: a.color.with_alpha(0),
            ..BorderSide::default()
        });

        Some(BorderSide::lerp(a, b, t))
    }

    /// Dart's `if (a is WidgetStateBorderSide) { a = a.resolve(const <WidgetState>{}); }`.
    fn resolve_unconditionally(side: &AnyBorderSide) -> Option<BorderSide> {
        match side.extension::<WidgetStateBorderSide>() {
            Some(side) => side.resolve(&WidgetStates::new()),
            None => Some(**side),
        }
    }
}

/// Dart's `==` on a `WidgetStateProperty`, which no override makes more than identity here.
fn same_property<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Dart's `OutlinedBorder ==`, which is `ShapeBorder`'s.
fn same_shape(a: &Option<Box<dyn OutlinedBorder>>, b: &Option<Box<dyn OutlinedBorder>>) -> bool {
    match (a.as_deref(), b.as_deref()) {
        (None, None) => true,
        (Some(a), Some(b)) => (a as &dyn ShapeBorder) == (b as &dyn ShapeBorder),
        _ => false,
    }
}

impl PartialEq for CheckboxThemeData {
    fn eq(&self, other: &CheckboxThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        same_property(&other.mouse_cursor, &self.mouse_cursor)
            && same_property(&other.fill_color, &self.fill_color)
            && same_property(&other.check_color, &self.check_color)
            && same_property(&other.overlay_color, &self.overlay_color)
            && other.splash_radius == self.splash_radius
            && other.material_tap_target_size == self.material_tap_target_size
            && other.visual_density == self.visual_density
            && same_shape(&other.shape, &self.shape)
            && other.side == self.side
            && other.mark_insets == self.mark_insets
    }
}

/// Applies a checkbox theme to descendant `Checkbox` widgets.
///
/// Descendant widgets obtain the current theme's [`CheckboxTheme`] object using
/// [`CheckboxTheme::of`]. When a widget uses [`CheckboxTheme::of`], it is
/// automatically rebuilt if the theme later changes.
///
/// A checkbox theme can be specified as part of the overall Material theme
/// using `ThemeData::checkbox_theme`.
///
/// See also:
///
///  * [`CheckboxThemeData`], which describes the actual configuration of a
///    checkbox theme.
#[derive(Debug)]
pub struct CheckboxTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `Checkbox` widgets.
    pub data: CheckboxThemeData,
    pub child: WidgetRef,
}

impl CheckboxTheme {
    /// Constructs a checkbox theme that configures all descendant `Checkbox`
    /// widgets.
    pub fn new<K>(data: CheckboxThemeData, child: impl IntoWidget<K>) -> CheckboxTheme {
        CheckboxTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `CheckboxTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> CheckboxTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`CheckboxTheme`]
    /// ancestor. If there is no ancestor, it returns `ThemeData::checkbox_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = CheckboxTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> CheckboxThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<CheckboxTheme>(app) {
            Some(checkbox_theme) => checkbox_theme.data.clone(),
            None => Theme::of(app, context).checkbox_theme.clone(),
        }
    }
}

impl InheritedWidget for CheckboxTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &CheckboxTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Offset, PointerChange, Radius, Size};
    use waft_painting::{BorderRadiusGeometry, RoundedRectangleBorder};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{FocusHighlightStrategy, SizedBox, WidgetState};

    use super::*;
    use crate::checkbox::Checkbox;
    use crate::checkbox::tests::{
        Paints, assert_paints, checkbox_center, checkbox_size, ignore, press, tester,
    };
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::scaffold::Scaffold;
    use crate::test_support::{
        WidgetTester, is_same_color_as, material_app, with_main_thread_stack,
    };
    use crate::theme_data::ThemeData;

    #[test]
    fn checkbox_theme_data_copy_with_equality_basics() {
        assert_eq!(
            CheckboxThemeData::new(),
            CheckboxThemeData::new().copy_with()
        );
        assert_eq!(
            CheckboxThemeData::new()
                .copy_with()
                .mark_insets(EdgeInsets::all(2.0)),
            CheckboxThemeData::new().mark_insets(EdgeInsets::all(2.0))
        );
        assert_ne!(
            CheckboxThemeData::new().mark_insets(EdgeInsets::all(2.0)),
            CheckboxThemeData::new().mark_insets(EdgeInsets::all(3.0))
        );
    }

    #[test]
    fn checkbox_theme_data_lerp_special_cases() {
        assert_eq!(
            CheckboxThemeData::lerp(None, None, 0.0),
            CheckboxThemeData::new()
        );
        let data = CheckboxThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(CheckboxThemeData::lerp(Some(&data), Some(&data), 0.5), data);
    }

    #[test]
    fn checkbox_theme_data_defaults() {
        let theme_data = CheckboxThemeData::new();
        assert!(theme_data.mouse_cursor.is_none());
        assert!(theme_data.fill_color.is_none());
        assert!(theme_data.check_color.is_none());
        assert!(theme_data.overlay_color.is_none());
        assert_eq!(theme_data.splash_radius, None);
        assert_eq!(theme_data.material_tap_target_size, None);
        assert_eq!(theme_data.visual_density, None);

        let theme = CheckboxTheme::new(CheckboxThemeData::new(), SizedBox::shrink());
        assert!(theme.data.mouse_cursor.is_none());
        assert!(theme.data.fill_color.is_none());
        assert!(theme.data.check_color.is_none());
        assert!(theme.data.overlay_color.is_none());
        assert_eq!(theme.data.splash_radius, None);
        assert_eq!(theme.data.material_tap_target_size, None);
        assert_eq!(theme.data.visual_density, None);
    }

    /// `tester.pumpWidget(MaterialApp(theme: ThemeData(checkboxTheme:), home:
    /// Scaffold(body: checkbox)))`.
    fn pump_themed(tester: &mut WidgetTester, checkbox_theme: CheckboxThemeData, body: WidgetRef) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::builder()
                    .checkbox_theme(checkbox_theme)
                    .build(&app),
                Scaffold::new().body(body).into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// Dart's `_pointGestureToCheckbox`.
    fn point_gesture_to_checkbox(tester: &mut WidgetTester) -> crate::test_support::TestPointer {
        let center = checkbox_center(tester);
        let mut gesture = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, center, false);
        gesture
    }

    fn color_property(
        resolve: impl Fn(&WidgetStates) -> Option<Color> + 'static,
    ) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(resolve)
    }

    fn all_colors(color: Color) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::all(Some(color))
    }

    fn all_cursors(cursor: MouseCursorRef) -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::all(Some(cursor))
    }

    #[test]
    fn checkbox_is_themeable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            const DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const DEFAULT_CHECK_COLOR: Color = Color::new(0xfffffff2);
            const FOCUSED_CHECK_COLOR: Color = Color::new(0xfffffff3);
            const FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff4);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff5);
            const SPLASH_RADIUS: f64 = 1.0;
            const MATERIAL_TAP_TARGET_SIZE: MaterialTapTargetSize =
                MaterialTapTargetSize::ShrinkWrap;
            const VISUAL_DENSITY: VisualDensity = VisualDensity::new(1.0, 1.0);

            let checkbox_theme = CheckboxThemeData::new()
                .mouse_cursor(all_cursors(SystemMouseCursors::TEXT.into()))
                .fill_color(color_property(|states| {
                    if states.contains(&WidgetState::Selected) {
                        return Some(SELECTED_FILL_COLOR);
                    }
                    Some(DEFAULT_FILL_COLOR)
                }))
                .check_color(color_property(|states| {
                    if states.contains(&WidgetState::Focused) {
                        return Some(FOCUSED_CHECK_COLOR);
                    }
                    Some(DEFAULT_CHECK_COLOR)
                }))
                .overlay_color(color_property(|states| {
                    if states.contains(&WidgetState::Focused) {
                        return Some(FOCUS_OVERLAY_COLOR);
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(HOVER_OVERLAY_COLOR);
                    }
                    None
                }))
                .splash_radius(SPLASH_RADIUS)
                .material_tap_target_size(MATERIAL_TAP_TARGET_SIZE)
                .visual_density(VISUAL_DENSITY);
            let frame = |tester: &mut WidgetTester, selected: bool, autofocus: bool| {
                pump_themed(
                    tester,
                    checkbox_theme.clone(),
                    Checkbox::new(Some(selected), ignore())
                        .autofocus(autofocus)
                        .into_widget(),
                );
                tester.pump_and_settle();
            };

            // Checkbox.
            frame(&mut tester, false, false);
            assert_paints(&tester, &[Paints::Rrect(DEFAULT_FILL_COLOR)]);
            // Size from MaterialTapTargetSize.shrinkWrap with added VisualDensity.
            assert_eq!(
                checkbox_size(&tester),
                Size::new(40.0, 40.0) + VISUAL_DENSITY.base_size_adjustment()
            );

            // Selected checkbox.
            frame(&mut tester, true, false);
            assert_paints(&tester, &[Paints::Rrect(SELECTED_FILL_COLOR)]);
            assert_paints(
                &tester,
                &[
                    Paints::Rrect(SELECTED_FILL_COLOR),
                    Paints::Path(DEFAULT_CHECK_COLOR),
                ],
            );

            // Checkbox with hover.
            frame(&mut tester, false, false);
            let gesture = point_gesture_to_checkbox(&mut tester);
            tester.pump_and_settle();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
            assert_paints(&tester, &[Paints::Circle(HOVER_OVERLAY_COLOR)]);

            // Checkbox with focus.
            frame(&mut tester, true, true);
            assert_paints(
                &tester,
                &[Paints::CircleRadius(FOCUS_OVERLAY_COLOR, SPLASH_RADIUS)],
            );
            assert_paints(
                &tester,
                &[
                    Paints::Rrect(SELECTED_FILL_COLOR),
                    Paints::Path(FOCUSED_CHECK_COLOR),
                ],
            );
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_properties_are_taken_over_the_theme_values() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            const THEME_DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const THEME_SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const THEME_CHECK_COLOR: Color = Color::new(0xfffffff2);
            const THEME_FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff3);
            const THEME_HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff4);
            const THEME_SPLASH_RADIUS: f64 = 1.0;

            const DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff5);
            const SELECTED_FILL_COLOR: Color = Color::new(0xfffffff6);
            const CHECK_COLOR: Color = Color::new(0xfffffff7);
            const FOCUS_COLOR: Color = Color::new(0xfffffff8);
            const HOVER_COLOR: Color = Color::new(0xfffffff9);
            const SPLASH_RADIUS: f64 = 2.0;
            const VISUAL_DENSITY: VisualDensity = VisualDensity::STANDARD;

            let checkbox_theme = CheckboxThemeData::new()
                .mouse_cursor(all_cursors(SystemMouseCursors::CLICK.into()))
                .fill_color(color_property(|states| {
                    if states.contains(&WidgetState::Selected) {
                        return Some(THEME_SELECTED_FILL_COLOR);
                    }
                    Some(THEME_DEFAULT_FILL_COLOR)
                }))
                .check_color(all_colors(THEME_CHECK_COLOR))
                .overlay_color(color_property(|states| {
                    if states.contains(&WidgetState::Focused) {
                        return Some(THEME_FOCUS_OVERLAY_COLOR);
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(THEME_HOVER_OVERLAY_COLOR);
                    }
                    None
                }))
                .splash_radius(THEME_SPLASH_RADIUS)
                .material_tap_target_size(MaterialTapTargetSize::Padded)
                .visual_density(VisualDensity::STANDARD);
            let fill_color = color_property(|states| {
                if states.contains(&WidgetState::Selected) {
                    return Some(SELECTED_FILL_COLOR);
                }
                Some(DEFAULT_FILL_COLOR)
            });
            let frame = |tester: &mut WidgetTester, selected: bool, autofocus: bool| {
                pump_themed(
                    tester,
                    checkbox_theme.clone(),
                    Checkbox::new(Some(selected), ignore())
                        .autofocus(autofocus)
                        .mouse_cursor(SystemMouseCursors::TEXT.into())
                        .fill_color(Rc::clone(&fill_color))
                        .check_color(CHECK_COLOR)
                        .focus_color(FOCUS_COLOR)
                        .hover_color(HOVER_COLOR)
                        .splash_radius(SPLASH_RADIUS)
                        .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                        .visual_density(VISUAL_DENSITY)
                        .into_widget(),
                );
                tester.pump_and_settle();
            };

            // Checkbox.
            frame(&mut tester, false, false);
            assert_paints(&tester, &[Paints::Rrect(DEFAULT_FILL_COLOR)]);
            // Size from MaterialTapTargetSize.shrinkWrap with added VisualDensity.
            assert_eq!(
                checkbox_size(&tester),
                Size::new(40.0, 40.0) + VISUAL_DENSITY.base_size_adjustment()
            );

            // Selected checkbox.
            frame(&mut tester, true, false);
            assert_paints(&tester, &[Paints::Rrect(SELECTED_FILL_COLOR)]);
            assert_paints(
                &tester,
                &[
                    Paints::Rrect(SELECTED_FILL_COLOR),
                    Paints::Path(CHECK_COLOR),
                ],
            );

            // Checkbox with hover.
            frame(&mut tester, false, false);
            let gesture = point_gesture_to_checkbox(&mut tester);
            tester.pump_and_settle();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
            assert_paints(&tester, &[Paints::Circle(HOVER_COLOR)]);

            // Checkbox with focus.
            frame(&mut tester, false, true);
            assert_paints(&tester, &[Paints::CircleRadius(FOCUS_COLOR, SPLASH_RADIUS)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_active_color_property_is_taken_over_the_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const THEME_SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const THEME_DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const SELECTED_FILL_COLOR: Color = Color::new(0xfffffff6);
            let checkbox_theme = CheckboxThemeData::new().fill_color(color_property(|states| {
                if states.contains(&WidgetState::Selected) {
                    return Some(THEME_SELECTED_FILL_COLOR);
                }
                Some(THEME_DEFAULT_FILL_COLOR)
            }));
            let frame = |tester: &mut WidgetTester, selected: bool| {
                pump_themed(
                    tester,
                    checkbox_theme.clone(),
                    Checkbox::new(Some(selected), ignore())
                        .active_color(SELECTED_FILL_COLOR)
                        .into_widget(),
                );
                tester.pump_and_settle();
            };

            // Unselected checkbox.
            frame(&mut tester, false);
            assert_paints(&tester, &[Paints::Rrect(THEME_DEFAULT_FILL_COLOR)]);

            // Selected checkbox.
            frame(&mut tester, true);
            assert_paints(&tester, &[Paints::Rrect(SELECTED_FILL_COLOR)]);
        });
    }

    #[test]
    fn checkbox_theme_overlay_color_resolves_in_active_pressed_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const SPLASH_RADIUS: f64 = 24.0;
            let checkbox_theme = CheckboxThemeData::new()
                .overlay_color(color_property(|states| {
                    if states.contains(&WidgetState::Pressed) {
                        if states.contains(&WidgetState::Selected) {
                            return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                        }
                        return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                    }
                    None
                }))
                .splash_radius(SPLASH_RADIUS);

            pump_themed(
                &mut tester,
                checkbox_theme.clone(),
                Checkbox::new(Some(false), ignore()).into_widget(),
            );
            let gesture1 = press(&mut tester, 1);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    INACTIVE_PRESSED_OVERLAY_COLOR,
                    SPLASH_RADIUS,
                )],
            );

            pump_themed(
                &mut tester,
                checkbox_theme,
                Checkbox::new(Some(true), ignore()).into_widget(),
            );
            let gesture2 = press(&mut tester, 2);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    ACTIVE_PRESSED_OVERLAY_COLOR,
                    SPLASH_RADIUS,
                )],
            );

            // Finish gesture to release resources.
            tester.up(&gesture1);
            tester.up(&gesture2);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn local_checkbox_theme_can_override_global_checkbox_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const GLOBAL_THEME_FILL_COLOR: Color = Color::new(0xfffffff1);
            const GLOBAL_THEME_CHECK_COLOR: Color = Color::new(0xff000000);
            const LOCAL_THEME_FILL_COLOR: Color = Color::new(0xffff0000);
            const LOCAL_THEME_CHECK_COLOR: Color = Color::new(0xffffffff);
            pump_themed(
                &mut tester,
                CheckboxThemeData::new()
                    .check_color(all_colors(GLOBAL_THEME_CHECK_COLOR))
                    .fill_color(all_colors(GLOBAL_THEME_FILL_COLOR)),
                CheckboxTheme::new(
                    CheckboxThemeData::new()
                        .fill_color(all_colors(LOCAL_THEME_FILL_COLOR))
                        .check_color(all_colors(LOCAL_THEME_CHECK_COLOR)),
                    Checkbox::new(Some(true), ignore()),
                )
                .into_widget(),
            );
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::Rrect(LOCAL_THEME_FILL_COLOR)]);
            assert_paints(
                &tester,
                &[
                    Paints::Rrect(LOCAL_THEME_FILL_COLOR),
                    Paints::Path(LOCAL_THEME_CHECK_COLOR),
                ],
            );
        });
    }

    #[test]
    fn checkbox_theme_data_lerp_with_null_parameters() {
        let lerped = CheckboxThemeData::lerp(None, None, 0.25);

        assert!(lerped.mouse_cursor.is_none());
        assert!(lerped.fill_color.is_none());
        assert!(lerped.check_color.is_none());
        assert!(lerped.overlay_color.is_none());
        assert_eq!(lerped.splash_radius, None);
        assert_eq!(lerped.material_tap_target_size, None);
        assert_eq!(lerped.visual_density, None);
        assert!(lerped.shape.is_none());
        assert_eq!(lerped.side, None);
        assert_eq!(lerped.mark_insets, None);
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(radius)),
        )
    }

    fn side_of_width(width: f64) -> BorderSide {
        BorderSide {
            width,
            ..BorderSide::default()
        }
    }

    fn populated(
        colors: [u32; 3],
        splash_radius: f64,
        visual_density: f64,
        radius: f64,
        side_width: f64,
        mark_insets: f64,
    ) -> CheckboxThemeData {
        CheckboxThemeData::new()
            .fill_color(all_colors(Color::new(colors[0])))
            .check_color(all_colors(Color::new(colors[1])))
            .overlay_color(all_colors(Color::new(colors[2])))
            .splash_radius(splash_radius)
            .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
            .visual_density(VisualDensity::new(visual_density, visual_density))
            .shape(rounded(radius))
            .side(side_of_width(side_width))
            .mark_insets(EdgeInsets::all(mark_insets))
    }

    fn resolved(property: &Option<WidgetStatePropertyRef<Option<Color>>>) -> Color {
        property
            .as_ref()
            .expect("a property")
            .resolve(&WidgetStates::new())
            .expect("a colour")
    }

    fn assert_shape(lerped: &CheckboxThemeData, radius: f64) {
        let shape: &dyn ShapeBorder = lerped.shape.as_deref().expect("a shape");
        assert!(
            shape == &rounded(radius) as &dyn ShapeBorder,
            "{shape:?} is not of radius {radius}"
        );
    }

    #[test]
    fn checkbox_theme_data_lerp_from_populated_to_null_parameters() {
        let theme = populated(
            [0xfffffff0, 0xfffffff1, 0xfffffff2],
            3.0,
            1.0,
            4.0,
            4.0,
            4.0,
        );
        let lerped = CheckboxThemeData::lerp(Some(&theme), None, 0.5);

        assert!(is_same_color_as(
            resolved(&lerped.fill_color),
            Color::new(0x80fffff0)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.check_color),
            Color::new(0x80fffff1)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.overlay_color),
            Color::new(0x80fffff2)
        ));
        assert_eq!(lerped.splash_radius, Some(1.5));
        assert_eq!(lerped.material_tap_target_size, None);
        assert_eq!(lerped.visual_density, None);
        assert_shape(&lerped, 2.0);
        let side = lerped.side.expect("a side");
        assert_eq!(side.width, 2.0);
        assert!(is_same_color_as(side.color, Color::new(0x80000000)));
        assert_eq!(lerped.mark_insets, Some(EdgeInsets::all(2.0)));
    }

    #[test]
    fn checkbox_theme_data_lerp_from_null_to_populated_parameters() {
        let theme = populated(
            [0xfffffff0, 0xfffffff1, 0xfffffff2],
            4.0,
            1.0,
            4.0,
            4.0,
            4.0,
        );
        let lerped = CheckboxThemeData::lerp(None, Some(&theme), 0.25);

        assert!(is_same_color_as(
            resolved(&lerped.fill_color),
            Color::new(0x40fffff0)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.check_color),
            Color::new(0x40fffff1)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.overlay_color),
            Color::new(0x40fffff2)
        ));
        assert_eq!(lerped.splash_radius, Some(1.0));
        assert_eq!(lerped.material_tap_target_size, None);
        assert_eq!(lerped.visual_density, None);
        assert_shape(&lerped, 1.0);
        let side = lerped.side.expect("a side");
        assert_eq!(side.width, 1.0);
        assert!(is_same_color_as(side.color, Color::new(0x40000000)));
        assert_eq!(lerped.mark_insets, Some(EdgeInsets::all(1.0)));
    }

    #[test]
    fn checkbox_theme_data_lerp_from_populated_parameters() {
        let theme_a = populated(
            [0xfffffff0, 0xfffffff1, 0xfffffff2],
            3.0,
            1.0,
            4.0,
            4.0,
            4.0,
        );
        let theme_b = populated(
            [0xfffffff3, 0xfffffff4, 0xfffffff5],
            9.0,
            2.0,
            1.0,
            3.0,
            2.0,
        );
        let lerped = CheckboxThemeData::lerp(Some(&theme_a), Some(&theme_b), 0.5);

        assert!(is_same_color_as(
            resolved(&lerped.fill_color),
            Color::new(0xfffffff1)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.check_color),
            Color::new(0xfffffff2)
        ));
        assert!(is_same_color_as(
            resolved(&lerped.overlay_color),
            Color::new(0xfffffff3)
        ));
        assert_eq!(lerped.splash_radius, Some(6.0));
        assert_eq!(
            lerped.material_tap_target_size,
            Some(MaterialTapTargetSize::ShrinkWrap)
        );
        assert_eq!(lerped.visual_density, Some(VisualDensity::new(2.0, 2.0)));
        assert_shape(&lerped, 2.5);
        assert_eq!(lerped.side, Some(AnyBorderSide::new(side_of_width(3.5))));
        assert_eq!(lerped.mark_insets, Some(EdgeInsets::all(3.0)));
    }

    #[test]
    fn widget_state_border_side_properly_lerp_in_checkbox_theme_data_side() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Dart's `late ColorScheme colorScheme`, which both themes' side closures read.
            let primary = Rc::new(Cell::new(Colors::TRANSPARENT));
            let frame = |tester: &mut WidgetTester, seed_color: Color| {
                let color_scheme = ColorScheme::from_seed(seed_color).build();
                primary.set(color_scheme.primary);
                let primary = Rc::clone(&primary);
                pump_themed_with_colors(
                    tester,
                    color_scheme,
                    CheckboxThemeData::new().side(WidgetStateBorderSide::resolve_with(
                        move |_states| {
                            Some(BorderSide {
                                color: primary.get(),
                                width: 4.0,
                                ..BorderSide::default()
                            })
                        },
                    )),
                );
            };

            frame(&mut tester, Colors::RED.primary());
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::Ring(primary.get())]);

            frame(&mut tester, Colors::BLUE.primary());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            assert_paints(&tester, &[Paints::Ring(primary.get())]);
        });
    }

    /// The last case's `MaterialApp(theme: ThemeData(colorScheme:, checkboxTheme:))`.
    fn pump_themed_with_colors(
        tester: &mut WidgetTester,
        color_scheme: ColorScheme,
        checkbox_theme: CheckboxThemeData,
    ) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::builder()
                    .color_scheme(color_scheme)
                    .checkbox_theme(checkbox_theme)
                    .build(&app),
                Scaffold::new()
                    .body(Checkbox::new(Some(false), ignore()))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }
}
