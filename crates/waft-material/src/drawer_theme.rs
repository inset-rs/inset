//! Flutter counterpart: material_ui's `drawer_theme.dart`.

use waft_embedder::{Clip, Color, lerp_double};
use waft_foundation::App;
use waft_painting::ShapeBorder;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Defines default property values for descendant `Drawer` widgets.
///
/// Descendant widgets obtain the current [`DrawerThemeData`] object
/// using [`DrawerTheme::of`]. Instances of [`DrawerThemeData`] can be
/// customized with [`DrawerThemeData::copy_with`].
///
/// Typically a [`DrawerThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::drawer_theme`.
///
/// All [`DrawerThemeData`] properties are `null` by default.
///
/// See also:
///
///  * [`DrawerTheme`], an `InheritedWidget` that propagates the theme down its
///    subtree.
///  * `ThemeData`, which describes the overall theme information for the
///    application and can customize a drawer using `ThemeData::drawer_theme`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawerThemeData {
    /// Overrides the default value of `Drawer::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `DrawerController::scrim_color`.
    pub scrim_color: Option<Color>,

    /// Overrides the default value of `Drawer::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value for `Drawer::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value for `Drawer::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of `Drawer::shape`.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value of `Drawer::shape` for an end drawer.
    pub end_shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value of `Drawer::width`.
    pub width: Option<f64>,

    /// Overrides the default value of `Drawer::clip_behavior`.
    pub clip_behavior: Option<Clip>,
}

impl DrawerThemeData {
    /// Creates a theme that can be used for `ThemeData::drawer_theme` and
    /// [`DrawerTheme`]; Dart's optional named arguments are the fluent setters.
    pub fn new() -> DrawerThemeData {
        DrawerThemeData::default()
    }

    /// Dart `DrawerThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> DrawerThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `DrawerThemeData(scrimColor:)`.
    pub fn scrim_color(mut self, scrim_color: Color) -> DrawerThemeData {
        self.scrim_color = Some(scrim_color);
        self
    }

    /// Dart `DrawerThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> DrawerThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `DrawerThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> DrawerThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `DrawerThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> DrawerThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `DrawerThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> DrawerThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `DrawerThemeData(endShape:)`.
    pub fn end_shape(mut self, end_shape: impl Into<Box<dyn ShapeBorder>>) -> DrawerThemeData {
        self.end_shape = Some(end_shape.into());
        self
    }

    /// Dart `DrawerThemeData(width:)`.
    pub fn width(mut self, width: f64) -> DrawerThemeData {
        self.width = Some(width);
        self
    }

    /// Dart `DrawerThemeData(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> DrawerThemeData {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().width(300.0)`).
    pub fn copy_with(&self) -> DrawerThemeData {
        self.clone()
    }

    /// Linearly interpolate between two drawer themes.
    ///
    /// If both arguments are null, then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&DrawerThemeData>,
        b: Option<&DrawerThemeData>,
        t: f64,
    ) -> Option<DrawerThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(DrawerThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            scrim_color: Color::lerp(
                a.and_then(|a| a.scrim_color),
                b.and_then(|b| b.scrim_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
            end_shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.end_shape.as_deref()),
                b.and_then(|b| b.end_shape.as_deref()),
                t,
            ),
            width: lerp_double(a.and_then(|a| a.width), b.and_then(|b| b.width), t),
            clip_behavior: if t < 0.5 {
                a.and_then(|a| a.clip_behavior)
            } else {
                b.and_then(|b| b.clip_behavior)
            },
        })
    }
}

/// An inherited widget that defines visual properties for `Drawer`s in this
/// widget's subtree.
///
/// Values specified here are used for `Drawer` properties that are not
/// given an explicit non-null value.
///
/// Using this would allow you to override the `ThemeData::drawer_theme`.
#[derive(Debug)]
pub struct DrawerTheme {
    pub key: Option<KeyRef>,
    /// Specifies the background color, scrim color, elevation, and shape for
    /// descendant `Drawer` widgets.
    pub data: DrawerThemeData,
    pub child: WidgetRef,
}

impl DrawerTheme {
    /// Creates a theme that defines the [`DrawerThemeData`] properties for a
    /// `Drawer`.
    pub fn new<K>(data: DrawerThemeData, child: impl IntoWidget<K>) -> DrawerTheme {
        DrawerTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `DrawerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DrawerTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`DrawerThemeData`] from the closest ancestor [`DrawerTheme`].
    ///
    /// If there is no enclosing [`DrawerTheme`] widget, then
    /// `ThemeData::drawer_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = DrawerTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> DrawerThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<DrawerTheme>(app) {
            Some(drawer_theme) => drawer_theme.data.clone(),
            None => Theme::of(app, context).drawer_theme.clone(),
        }
    }
}

impl InheritedTheme for DrawerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DrawerTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for DrawerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DrawerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_foundation::AppCell;
    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
    use waft_widgets::{Builder, SizedBox};

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::{build, probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    /// What `read` sees at a probe that is wrapped, by `<dyn InheritedTheme>::capture`, in the
    /// inherited themes above a first probe, and then built under `shadowed_by`.
    ///
    /// Dart's `InheritedTheme.captureAll` / `CapturedThemes.wrap` on a theme `captured_under`
    /// puts above the first probe: the wrapped probe sees that theme, not the one
    /// `shadowed_by` puts above it.
    pub(crate) fn theme_seen_through_capture<T: 'static>(
        cell: &AppCell,
        captured_under: impl FnOnce(WidgetRef) -> WidgetRef,
        shadowed_by: impl FnOnce(WidgetRef) -> WidgetRef,
        read: impl Fn(&mut App, BuildContext) -> T + 'static,
    ) -> T {
        let captured = Rc::new(RefCell::new(None));
        let capturing_probe = Builder::new({
            let captured = Rc::clone(&captured);
            move |app, context| {
                *captured.borrow_mut() = Some(<dyn waft_widgets::InheritedTheme>::capture(
                    app, context, None,
                ));
                SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        build(cell, captured_under(capturing_probe));
        let captured = captured.borrow_mut().take().expect("the probe built");
        probe(
            cell,
            |_, reading_probe| shadowed_by(captured.wrap(reading_probe)),
            read,
        )
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    #[test]
    fn copy_with_equality_basics() {
        assert_eq!(DrawerThemeData::new(), DrawerThemeData::new().copy_with());

        let theme = DrawerThemeData::new().elevation(4.0).shape(rounded(8.0));
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(theme.copy_with().width(300.0).width, Some(300.0));
        assert_eq!(theme.copy_with().width(300.0).elevation, Some(4.0));
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(theme.copy_with().shape(rounded(9.0)), theme);
    }

    #[test]
    fn drawer_theme_data_lerp_special_cases() {
        assert_eq!(DrawerThemeData::lerp(None, None, 0.0), None);
        let data = DrawerThemeData::new();
        assert_eq!(
            DrawerThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn drawer_theme_data_null_fields_by_default() {
        let theme = DrawerThemeData::new();
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.scrim_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.shadow_color, None);
        assert_eq!(theme.surface_tint_color, None);
        assert!(theme.shape.is_none());
        assert!(theme.end_shape.is_none());
        assert_eq!(theme.width, None);
        assert_eq!(theme.clip_behavior, None);
    }

    #[test]
    fn drawer_theme_data_lerps_each_field() {
        let a = DrawerThemeData::new()
            .background_color(Colors::BLACK)
            .elevation(2.0)
            .shape(rounded(4.0))
            .width(100.0)
            .clip_behavior(Clip::HardEdge);
        let b = DrawerThemeData::new()
            .background_color(Colors::WHITE)
            .scrim_color(Colors::RED.primary())
            .elevation(6.0)
            .shape(rounded(8.0))
            .end_shape(rounded(8.0))
            .width(300.0)
            .clip_behavior(Clip::AntiAlias);

        let halfway = DrawerThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.scrim_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(
            halfway.shape.as_deref(),
            Some(&rounded(6.0) as &dyn ShapeBorder)
        );
        assert_eq!(
            halfway.end_shape.as_deref(),
            <dyn ShapeBorder>::lerp(None, Some(&rounded(8.0)), 0.5).as_deref()
        );
        assert_eq!(halfway.width, Some(200.0));
        assert_eq!(halfway.clip_behavior, Some(Clip::AntiAlias));
        assert_eq!(
            DrawerThemeData::lerp(Some(&a), Some(&b), 0.25)
                .expect("both given")
                .clip_behavior,
            Some(Clip::HardEdge)
        );
    }

    #[test]
    fn of_returns_the_theme_data_drawer_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = DrawerThemeData::new().width(280.0).elevation(3.0);
            let theme_data_drawer = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .drawer_theme(theme_data_drawer.clone())
                        .build(app);
                    crate::theme::Theme::new(data, child).into_widget()
                },
                DrawerTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_drawer_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_drawer = DrawerThemeData::new().width(100.0);
            let expected = widget_drawer.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .drawer_theme(DrawerThemeData::new().width(280.0))
                        .build(app);
                    crate::theme::Theme::new(data, DrawerTheme::new(widget_drawer.clone(), child))
                        .into_widget()
                },
                DrawerTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_drawer_theme_is_kept_under_another_drawer_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let captured = DrawerThemeData::new().width(111.0);
            let shadowing = DrawerThemeData::new().width(222.0);

            let seen = theme_seen_through_capture(
                &cell,
                |probe| DrawerTheme::new(captured.clone(), probe).into_widget(),
                |wrapped| DrawerTheme::new(shadowing.clone(), wrapped).into_widget(),
                DrawerTheme::of,
            );

            assert_eq!(seen.width, Some(111.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: DrawerThemeData| DrawerTheme::new(data, SizedBox::shrink());
        let old = theme(DrawerThemeData::new().width(1.0));

        assert!(!theme(DrawerThemeData::new().width(1.0)).update_should_notify(&old));
        assert!(theme(DrawerThemeData::new().width(2.0)).update_should_notify(&old));
    }

    #[test]
    fn theme_data_holds_the_drawer_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.drawer_theme, DrawerThemeData::new());

            let wide = ThemeData::builder()
                .drawer_theme(DrawerThemeData::new().width(300.0))
                .build(&app);
            assert_eq!(wide.drawer_theme.width, Some(300.0));
            assert_ne!(theme, wide);
            assert_eq!(
                theme
                    .copy_with()
                    .drawer_theme(wide.drawer_theme.clone())
                    .build(),
                wide
            );

            let narrow = ThemeData::builder()
                .drawer_theme(DrawerThemeData::new().width(100.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&narrow, &wide, 0.5).drawer_theme.width,
                Some(200.0)
            );
        });
    }
}
