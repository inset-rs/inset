//! Flutter counterpart: material_ui's `badge_theme.dart`.

use waft_embedder::{Color, Offset, lerp_double};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextStyle};
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Overrides the default properties values for descendant [`Badge`](crate::Badge) widgets.
///
/// Descendant widgets obtain the current [`BadgeThemeData`] object
/// using [`BadgeTheme::of`]. Instances of [`BadgeThemeData`] can
/// be customized with [`BadgeThemeData::copy_with`].
///
/// Typically a [`BadgeThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::badge_theme`.
///
/// All [`BadgeThemeData`] properties are `null` by default.
/// When null, the [`Badge`](crate::Badge) will use the values from `ThemeData`
/// if they exist, otherwise it will provide its own defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BadgeThemeData {
    /// Overrides the default value for `Badge::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value for `Badge::text_color`.
    pub text_color: Option<Color>,

    /// Overrides the default value for `Badge::small_size`.
    pub small_size: Option<f64>,

    /// Overrides the default value for `Badge::large_size`.
    pub large_size: Option<f64>,

    /// Overrides the default value for `Badge::text_style`.
    pub text_style: Option<TextStyle>,

    /// Overrides the default value for `Badge::padding`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Overrides the default value for `Badge::alignment`.
    pub alignment: Option<AlignmentGeometry>,

    /// Overrides the default value for `Badge::offset`.
    pub offset: Option<Offset>,
}

impl BadgeThemeData {
    /// Creates the set of color, style, and size properties used to configure
    /// [`Badge`](crate::Badge); Dart's optional named arguments are the fluent setters.
    pub fn new() -> BadgeThemeData {
        BadgeThemeData::default()
    }

    /// Dart `BadgeThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> BadgeThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `BadgeThemeData(textColor:)`.
    pub fn text_color(mut self, text_color: Color) -> BadgeThemeData {
        self.text_color = Some(text_color);
        self
    }

    /// Dart `BadgeThemeData(smallSize:)`.
    pub fn small_size(mut self, small_size: f64) -> BadgeThemeData {
        self.small_size = Some(small_size);
        self
    }

    /// Dart `BadgeThemeData(largeSize:)`.
    pub fn large_size(mut self, large_size: f64) -> BadgeThemeData {
        self.large_size = Some(large_size);
        self
    }

    /// Dart `BadgeThemeData(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> BadgeThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `BadgeThemeData(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> BadgeThemeData {
        self.padding = Some(padding);
        self
    }

    /// Dart `BadgeThemeData(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> BadgeThemeData {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `BadgeThemeData(offset:)`.
    pub fn offset(mut self, offset: Offset) -> BadgeThemeData {
        self.offset = Some(offset);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().small_size(4.0)`).
    pub fn copy_with(&self) -> BadgeThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`Badge`](crate::Badge) themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: Option<&BadgeThemeData>, b: Option<&BadgeThemeData>, t: f64) -> BadgeThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        BadgeThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            text_color: Color::lerp(
                a.and_then(|a| a.text_color),
                b.and_then(|b| b.text_color),
                t,
            ),
            small_size: lerp_double(
                a.and_then(|a| a.small_size),
                b.and_then(|b| b.small_size),
                t,
            ),
            large_size: lerp_double(
                a.and_then(|a| a.large_size),
                b.and_then(|b| b.large_size),
                t,
            ),
            text_style: TextStyle::lerp(
                a.and_then(|a| a.text_style.as_ref()),
                b.and_then(|b| b.text_style.as_ref()),
                t,
            ),
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.alignment),
                b.and_then(|b| b.alignment),
                t,
            ),
            offset: Offset::lerp(a.and_then(|a| a.offset), b.and_then(|b| b.offset), t),
        }
    }
}

/// An inherited widget that overrides the default color style, and size
/// parameters for [`Badge`](crate::Badge)s in this widget's subtree.
///
/// Values specified here override the defaults for [`Badge`](crate::Badge) properties which
/// are not given an explicit non-null value.
#[derive(Debug)]
pub struct BadgeTheme {
    pub key: Option<KeyRef>,
    /// Specifies the default color and size overrides for descendant
    /// [`Badge`](crate::Badge) widgets.
    pub data: BadgeThemeData,
    pub child: WidgetRef,
}

impl BadgeTheme {
    /// Creates a theme that overrides the default color parameters for
    /// [`Badge`](crate::Badge)s in this widget's subtree.
    pub fn new<K>(data: BadgeThemeData, child: impl IntoWidget<K>) -> BadgeTheme {
        BadgeTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `BadgeTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> BadgeTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`BadgeThemeData`] from the closest ancestor [`BadgeTheme`].
    ///
    /// If there is no enclosing [`BadgeTheme`] widget, then
    /// `ThemeData::badge_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = BadgeTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> BadgeThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<BadgeTheme>(app) {
            Some(badge_theme) => badge_theme.data.clone(),
            None => Theme::of(app, context).badge_theme,
        }
    }
}

impl InheritedTheme for BadgeTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        BadgeTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for BadgeTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &BadgeTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/badge_theme_test.dart`; the `debugFillProperties` cases wait for
    //! diagnostics, and `hashCode` for `Hash`. The two cases that measure a badge's label give it
    //! flutter_test's Ahem as [`crate::badge::tests`] does.

    use waft_embedder::{RRect, Radius, Size};
    use waft_painting::Alignment;
    use waft_widgets::SizedBox;

    use super::*;
    use crate::badge::Badge;
    use crate::badge::tests::{
        Seen, add_icon, ahem_label, badge_paints_rrect, badge_rect, rect_of_ahem,
    };
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::scaffold::Scaffold;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m3, pump_app, tester};

    const GREEN: Color = Color::new(0xff00ff00);
    const BLACK: Color = Color::new(0xff000000);

    /// The `badgeTheme` of 'Badge uses ThemeData badge theme'.
    fn badge_theme_data() -> BadgeThemeData {
        BadgeThemeData::new()
            .background_color(GREEN)
            .text_color(BLACK)
            .small_size(5.0)
            .large_size(20.0)
            .text_style(TextStyle::new().font_size(12.0))
            .padding(EdgeInsetsGeometry::symmetric(0.0, 5.0))
            .alignment(Alignment::TOP_RIGHT.into())
            .offset(Offset::new(24.0, 0.0))
    }

    fn badge_in_scaffold(seen: &Seen) -> WidgetRef {
        Scaffold::new()
            .body(
                Badge::new()
                    .label(ahem_label("1234", seen))
                    .child(add_icon()),
            )
            .into_widget()
    }

    fn assert_badge_uses_the_theme(tester: &crate::test_support::WidgetTester, seen: &Seen) {
        // text width = 48 = fontSize * 4, text height = fontSize
        let text_rect = rect_of_ahem(tester, "1234");
        assert_eq!(text_rect.size(), Size::new(48.0, 12.0));

        assert_eq!(text_rect.top_left(), Offset::new(33.0, 2.0));

        // default Icon size
        assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
        assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

        let text_style = seen.label_style();
        assert_eq!(text_style.font_size, Some(12.0));
        assert_eq!(
            text_style.color.as_ref().map(|color| color.color()),
            Some(BLACK)
        );

        assert!(badge_paints_rrect(
            tester,
            RRect::from_ltrbr(28.0, -2.0, 86.0, 18.0, Radius::circular(10.0)),
            Some(GREEN),
        ));
    }

    #[test]
    fn badge_theme_data_copy_with_equality_basics() {
        assert_eq!(BadgeThemeData::new(), BadgeThemeData::new().copy_with());

        let theme = badge_theme_data();
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(theme.copy_with().small_size(9.0).small_size, Some(9.0));
        assert_eq!(theme.copy_with().small_size(9.0).large_size, Some(20.0));
        assert_ne!(theme.copy_with().small_size(9.0), theme);
        assert_ne!(theme.copy_with().offset(Offset::ZERO), theme);
    }

    #[test]
    fn badge_theme_data_lerp_special_cases() {
        assert_eq!(BadgeThemeData::lerp(None, None, 0.0), BadgeThemeData::new());
        let data = BadgeThemeData::new();
        assert_eq!(BadgeThemeData::lerp(Some(&data), Some(&data), 0.5), data);
    }

    #[test]
    fn badge_theme_data_lerps_each_field() {
        let a = BadgeThemeData::new()
            .background_color(Colors::BLACK)
            .small_size(2.0)
            .text_style(TextStyle::new().font_size(10.0))
            .alignment(Alignment::TOP_LEFT.into())
            .offset(Offset::ZERO);
        let b = BadgeThemeData::new()
            .background_color(Colors::WHITE)
            .text_color(Colors::RED.primary())
            .small_size(6.0)
            .large_size(20.0)
            .text_style(TextStyle::new().font_size(20.0))
            .padding(EdgeInsetsGeometry::all(10.0))
            .alignment(Alignment::BOTTOM_RIGHT.into())
            .offset(Offset::new(10.0, 20.0));

        let halfway = BadgeThemeData::lerp(Some(&a), Some(&b), 0.5);

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.text_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.small_size, Some(4.0));
        assert_eq!(halfway.large_size, lerp_double(None, Some(20.0), 0.5));
        assert_eq!(
            halfway.text_style.and_then(|style| style.font_size),
            Some(15.0)
        );
        assert_eq!(
            halfway.padding,
            EdgeInsetsGeometry::lerp(None, Some(EdgeInsetsGeometry::all(10.0)), 0.5)
        );
        assert_eq!(halfway.alignment, Some(Alignment::CENTER.into()));
        assert_eq!(halfway.offset, Some(Offset::new(5.0, 10.0)));
    }

    #[test]
    fn badge_theme_data_defaults() {
        let theme_data = BadgeThemeData::new();
        assert_eq!(theme_data.background_color, None);
        assert_eq!(theme_data.text_color, None);
        assert_eq!(theme_data.small_size, None);
        assert_eq!(theme_data.large_size, None);
        assert_eq!(theme_data.text_style, None);
        assert_eq!(theme_data.padding, None);
        assert_eq!(theme_data.alignment, None);
        assert_eq!(theme_data.offset, None);
    }

    #[test]
    fn badge_uses_theme_data_badge_theme() {
        with_main_thread_stack(|| {
            let seen = Seen::default();
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .badge_theme(badge_theme_data())
                        .build(app)
                },
                |_app| badge_in_scaffold(&seen),
            );

            assert_badge_uses_the_theme(&tester, &seen);
        });
    }

    // This test is essentially the same as 'Badge uses ThemeData badge theme'. In
    // this case the theme is introduced with the BadgeTheme widget instead of
    // ThemeData.badgeTheme.
    #[test]
    fn badge_uses_badge_theme() {
        with_main_thread_stack(|| {
            let seen = Seen::default();
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                BadgeTheme::new(badge_theme_data(), badge_in_scaffold(&seen)).into_widget()
            });

            assert_badge_uses_the_theme(&tester, &seen);
        });
    }

    #[test]
    fn of_returns_the_theme_data_badge_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = badge_theme_data();
            let theme_data_badge = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .badge_theme(theme_data_badge.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                BadgeTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_badge_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_badge = BadgeThemeData::new().small_size(7.0);
            let expected = widget_badge.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .badge_theme(BadgeThemeData::new().small_size(3.0))
                        .build(app);
                    Theme::new(data, BadgeTheme::new(widget_badge.clone(), child)).into_widget()
                },
                BadgeTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_badge_theme_is_kept_under_another_badge_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let captured = BadgeThemeData::new().small_size(1.0);
            let shadowing = BadgeThemeData::new().small_size(2.0);

            let seen = theme_seen_through_capture(
                &cell,
                |probe| BadgeTheme::new(captured.clone(), probe).into_widget(),
                |wrapped| BadgeTheme::new(shadowing.clone(), wrapped).into_widget(),
                BadgeTheme::of,
            );

            assert_eq!(seen.small_size, Some(1.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: BadgeThemeData| BadgeTheme::new(data, SizedBox::shrink());
        let old = theme(BadgeThemeData::new().small_size(1.0));

        assert!(!theme(BadgeThemeData::new().small_size(1.0)).update_should_notify(&old));
        assert!(theme(BadgeThemeData::new().small_size(2.0)).update_should_notify(&old));
    }

    #[test]
    fn theme_data_holds_the_badge_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.badge_theme, BadgeThemeData::new());

            let big = ThemeData::builder()
                .badge_theme(BadgeThemeData::new().small_size(8.0))
                .build(&app);
            assert_eq!(big.badge_theme.small_size, Some(8.0));
            assert_ne!(theme, big);
            assert_eq!(
                theme
                    .copy_with()
                    .badge_theme(big.badge_theme.clone())
                    .build(),
                big
            );

            let small = ThemeData::builder()
                .badge_theme(BadgeThemeData::new().small_size(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&small, &big, 0.5).badge_theme.small_size,
                Some(4.0)
            );
        });
    }
}
