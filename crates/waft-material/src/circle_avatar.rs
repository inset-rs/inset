//! Flutter counterpart: material_ui's `circle_avatar.dart`.

use waft_embedder::{Brightness, Color};
use waft_foundation::App;
use waft_painting::{BoxDecoration, BoxShape};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    AnimatedContainer, BuildContext, Center, DefaultTextStyle, IconTheme, IntoWidget, KeyRef,
    MediaQuery, StatelessWidget, WidgetRef, debug_check_has_media_query,
};

use crate::constants::K_THEME_CHANGE_DURATION;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// A circle that represents a user.
///
/// Typically used with a user's profile image, or, in the absence of
/// such an image, the user's initials. A given user's initials should
/// always be paired with the same background color, for consistency.
///
/// If the avatar is to just have the user's initials, they are typically
/// provided using a `Text` widget as the [`child`](Self::child) and a
/// [`background_color`](Self::background_color):
///
/// ```text
/// CircleAvatar::new()
///     .background_color(Colors::BROWN.shade800())
///     .child(Text::new("AH"))
/// ```
///
/// Dart's `backgroundImage`, `foregroundImage` and their error callbacks are not here yet:
/// they need `DecorationImage`.
///
/// See also:
///
///  * `Chip`, for representing users or concepts in long form.
///  * `ListTile`, which can combine an icon (such as a [`CircleAvatar`]) with
///    some text for a fixed height list entry.
///  * <https://material.io/design/components/chips.html#input-chips>
#[derive(Debug, Default)]
pub struct CircleAvatar {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: Option<WidgetRef>,

    /// The color with which to fill the circle. Changing the background
    /// color will cause the avatar to animate to the new color.
    ///
    /// If a [`background_color`](Self::background_color) is not specified and
    /// `ThemeData::use_material3` is true, `ColorScheme::primary_container` will be used,
    /// otherwise the theme's `ThemeData::primary_color_light` is used with dark foreground
    /// colors, and `ThemeData::primary_color_dark` with light foreground colors.
    pub background_color: Option<Color>,

    /// The default text color for text in the circle.
    ///
    /// Defaults to the primary text theme color if no
    /// [`background_color`](Self::background_color) is specified.
    ///
    /// If a [`foreground_color`](Self::foreground_color) is not specified and
    /// `ThemeData::use_material3` is true, `ColorScheme::on_primary_container` will be used,
    /// otherwise the theme's `ThemeData::primary_color_light` for dark background colors, and
    /// `ThemeData::primary_color_dark` for light background colors.
    pub foreground_color: Option<Color>,

    /// The size of the avatar, expressed as the radius (half the diameter).
    ///
    /// If [`radius`](Self::radius) is specified, then neither
    /// [`min_radius`](Self::min_radius) nor [`max_radius`](Self::max_radius) may be
    /// specified. Specifying [`radius`](Self::radius) is equivalent to specifying a
    /// [`min_radius`](Self::min_radius) and [`max_radius`](Self::max_radius), both with the
    /// value of [`radius`](Self::radius).
    ///
    /// If neither [`min_radius`](Self::min_radius) nor [`max_radius`](Self::max_radius) are
    /// specified, defaults to 20 logical pixels. This is the appropriate size for use with
    /// `ListTile::leading`.
    ///
    /// Changes to the [`radius`](Self::radius) are animated (including changing from an
    /// explicit [`radius`](Self::radius) to a [`min_radius`](Self::min_radius)/
    /// [`max_radius`](Self::max_radius) pair or vice versa).
    pub radius: Option<f64>,

    /// The minimum size of the avatar, expressed as the radius (half the
    /// diameter).
    ///
    /// If [`min_radius`](Self::min_radius) is specified, then [`radius`](Self::radius) must
    /// not also be specified.
    ///
    /// Defaults to zero.
    ///
    /// Constraint changes are animated, but size changes due to the environment
    /// itself changing are not. For example, changing the [`min_radius`](Self::min_radius)
    /// from 10 to 20 when the [`CircleAvatar`] is in an unconstrained environment will cause
    /// the avatar to animate from a 20 pixel diameter to a 40 pixel diameter.
    /// However, if the [`min_radius`](Self::min_radius) is 40 and the [`CircleAvatar`] has a
    /// parent `SizedBox` whose size changes instantaneously from 20 pixels to 40 pixels,
    /// the size will snap to 40 pixels instantly.
    pub min_radius: Option<f64>,

    /// The maximum size of the avatar, expressed as the radius (half the
    /// diameter).
    ///
    /// If [`max_radius`](Self::max_radius) is specified, then [`radius`](Self::radius) must
    /// not also be specified.
    ///
    /// Defaults to [`f64::INFINITY`].
    ///
    /// Constraint changes are animated, but size changes due to the environment
    /// itself changing are not. For example, changing the [`max_radius`](Self::max_radius)
    /// from 10 to 20 when the [`CircleAvatar`] is in an unconstrained environment will cause
    /// the avatar to animate from a 20 pixel diameter to a 40 pixel diameter.
    /// However, if the [`max_radius`](Self::max_radius) is 40 and the [`CircleAvatar`] has a
    /// parent `SizedBox` whose size changes instantaneously from 20 pixels to 40 pixels,
    /// the size will snap to 40 pixels instantly.
    pub max_radius: Option<f64>,
}

impl CircleAvatar {
    // The default radius if nothing is specified.
    const DEFAULT_RADIUS: f64 = 20.0;

    // The default min if only the max is specified.
    const DEFAULT_MIN_RADIUS: f64 = 0.0;

    // The default max if only the min is specified.
    const DEFAULT_MAX_RADIUS: f64 = f64::INFINITY;

    /// Creates a circle that represents a user.
    pub fn new() -> CircleAvatar {
        CircleAvatar::default()
    }

    /// Dart `CircleAvatar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CircleAvatar {
        self.key = Some(key);
        self
    }

    /// Dart `CircleAvatar(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CircleAvatar {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `CircleAvatar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> CircleAvatar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `CircleAvatar(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> CircleAvatar {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `CircleAvatar(radius:)`.
    pub fn radius(mut self, radius: f64) -> CircleAvatar {
        debug_assert!(self.min_radius.is_none() && self.max_radius.is_none());
        self.radius = Some(radius);
        self
    }

    /// Dart `CircleAvatar(minRadius:)`.
    pub fn min_radius(mut self, min_radius: f64) -> CircleAvatar {
        debug_assert!(self.radius.is_none());
        self.min_radius = Some(min_radius);
        self
    }

    /// Dart `CircleAvatar(maxRadius:)`.
    pub fn max_radius(mut self, max_radius: f64) -> CircleAvatar {
        debug_assert!(self.radius.is_none());
        self.max_radius = Some(max_radius);
        self
    }

    fn min_diameter(&self) -> f64 {
        if self.radius.is_none() && self.min_radius.is_none() && self.max_radius.is_none() {
            return CircleAvatar::DEFAULT_RADIUS * 2.0;
        }
        2.0 * self
            .radius
            .or(self.min_radius)
            .unwrap_or(CircleAvatar::DEFAULT_MIN_RADIUS)
    }

    fn max_diameter(&self) -> f64 {
        if self.radius.is_none() && self.min_radius.is_none() && self.max_radius.is_none() {
            return CircleAvatar::DEFAULT_RADIUS * 2.0;
        }
        2.0 * self
            .radius
            .or(self.max_radius)
            .unwrap_or(CircleAvatar::DEFAULT_MAX_RADIUS)
    }
}

impl StatelessWidget for CircleAvatar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_media_query(app, context));
        let theme = Theme::of(app, context);
        let effective_foreground_color = self.foreground_color.or(if theme.use_material3 {
            Some(theme.color_scheme.on_primary_container())
        } else {
            None
        });
        let effective_text_style = if theme.use_material3 {
            theme
                .text_theme
                .title_medium
                .clone()
                .expect("the text theme has a title medium style")
        } else {
            theme
                .primary_text_theme
                .title_medium
                .clone()
                .expect("the primary text theme has a title medium style")
        };
        let mut text_style = match effective_foreground_color {
            Some(color) => effective_text_style.copy_with().color(color),
            None => effective_text_style,
        };
        let mut effective_background_color = self.background_color.or(if theme.use_material3 {
            Some(theme.color_scheme.primary_container())
        } else {
            None
        });
        match effective_background_color {
            None => {
                let text_color = text_style
                    .color
                    .as_ref()
                    .expect("the title style has a color")
                    .color();
                effective_background_color =
                    Some(match ThemeData::estimate_brightness_for_color(text_color) {
                        Brightness::Dark => theme.primary_color_light,
                        Brightness::Light => theme.primary_color_dark,
                    });
            }
            Some(_) if effective_foreground_color.is_none() => {
                text_style = match ThemeData::estimate_brightness_for_color(
                    self.background_color
                        .expect("only the Material 2 colors leave the foreground out"),
                ) {
                    Brightness::Dark => text_style.copy_with().color(theme.primary_color_light),
                    Brightness::Light => text_style.copy_with().color(theme.primary_color_dark),
                };
            }
            Some(_) => {}
        }
        let min_diameter = self.min_diameter();
        let max_diameter = self.max_diameter();
        let mut container = AnimatedContainer::new(K_THEME_CHANGE_DURATION)
            .constraints(
                BoxConstraints::new()
                    .min_height(min_diameter)
                    .min_width(min_diameter)
                    .max_width(max_diameter)
                    .max_height(max_diameter),
            )
            .decoration(
                BoxDecoration::new()
                    .color(effective_background_color.expect("set above"))
                    .shape(BoxShape::Circle),
            );
        if let Some(child) = &self.child {
            let icon_theme = match &text_style.color {
                Some(color) => theme.icon_theme.copy_with().color(color.color()),
                None => theme.icon_theme.copy_with(),
            };
            container = container.child(Center::new().child(
                // Need to disable text scaling here so that the text doesn't
                // escape the avatar when the textScaleFactor is large.
                MediaQuery::with_no_text_scaling(
                    None,
                    IconTheme::new(icon_theme, DefaultTextStyle::new(text_style, child.clone())),
                ),
            ));
        }
        container.into_widget()
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/circle_avatar_test.dart`. A case that reads the style of the avatar's
    //! `Text('Z')` reads its `RenderParagraph`, as Dart does. The cases on `backgroundImage` and `foregroundImage` ('with
    //! image background', 'with image foreground', and 'backgroundImage is used as a fallback
    //! for foregroundImage') wait for `DecorationImage`.

    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::Size;
    use waft_painting::{EdgeInsets, TextScaler};
    use waft_widgets::{Builder, DecoratedBox, MediaQueryData, SizedBox, Text, UnconstrainedBox};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::bottom_navigation_bar::tests::paragraph_style;
    use crate::colors::Colors;
    use crate::tooltip::tests::{m2, m3, pump_app};

    /// Dart's `wrap(child:)`: the child centered in a Material 2 `MaterialApp`.
    fn pump_wrapped(tester: &mut WidgetTester, child: WidgetRef) {
        pump_app(tester, m2, move |_app| {
            Center::new().child(child).into_widget()
        });
    }

    fn avatar_rect(tester: &WidgetTester) -> waft_embedder::Rect {
        let avatar = tester.find_by_type::<CircleAvatar>()[0];
        tester.global_rect(tester.render_box_of(avatar))
    }

    /// Dart's `box.child! as RenderDecoratedBox).decoration`.
    fn avatar_decoration(tester: &WidgetTester) -> BoxDecoration {
        let avatar = tester.find_by_type::<CircleAvatar>()[0];
        let boxes = tester.descendants_of_type::<DecoratedBox>(avatar);
        tester.widget(boxes[0], |decorated_box: &DecoratedBox| {
            decorated_box
                .decoration
                .as_any()
                .downcast_ref::<BoxDecoration>()
                .expect("a BoxDecoration")
                .clone()
        })
    }

    fn avatar_color(tester: &WidgetTester) -> Option<Color> {
        avatar_decoration(tester)
            .color
            .as_ref()
            .map(|color| color.color())
    }

    fn text_color(tester: &WidgetTester) -> Option<Color> {
        paragraph_style(tester, "Z")
            .color
            .as_ref()
            .map(|color| color.color())
    }

    fn fallback(tester: &WidgetTester) -> ThemeData {
        ThemeData::fallback(&tester.cell.borrow(), None)
    }

    #[test]
    fn circle_avatar_with_dark_background_color() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.shade900();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .background_color(background_color)
                    .radius(50.0)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(100.0, 100.0));
            assert_eq!(avatar_color(tester), Some(background_color));
            assert_eq!(
                text_color(tester),
                Some(fallback(tester).primary_color_light)
            );
        });
    }

    #[test]
    fn circle_avatar_with_light_background_color() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.shade100();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .background_color(background_color)
                    .radius(50.0)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(100.0, 100.0));
            assert_eq!(avatar_color(tester), Some(background_color));
            assert_eq!(
                text_color(tester),
                Some(fallback(tester).primary_color_dark)
            );
        });
    }

    #[test]
    fn circle_avatar_with_foreground_color() {
        test_widgets(|tester| {
            let foreground_color = Colors::RED.shade100();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .foreground_color(foreground_color)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(40.0, 40.0));
            assert_eq!(
                avatar_color(tester),
                Some(fallback(tester).primary_color_dark)
            );
            assert_eq!(text_color(tester), Some(foreground_color));
        });
    }

    #[test]
    fn material3_circle_avatar_default_colors() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_wrapped(
                tester,
                Theme::new(theme.clone(), CircleAvatar::new().child(Text::new("Z"))).into_widget(),
            );

            assert_eq!(
                avatar_color(tester),
                Some(theme.color_scheme.primary_container())
            );
            assert_eq!(
                text_color(tester),
                Some(theme.color_scheme.on_primary_container())
            );
        });
    }

    #[test]
    fn circle_avatar_text_does_not_expand_with_text_scaler() {
        test_widgets(|tester| {
            let foreground_color = Colors::RED.shade100();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .foreground_color(foreground_color)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(
                tester
                    .global_rect(tester.render_box_of(tester.find_text("Z")[0]))
                    .size(),
                Size::new(16.0, 16.0)
            );

            let seen_data: Rc<RefCell<Option<MediaQueryData>>> = Rc::new(RefCell::new(None));
            let seen = Rc::clone(&seen_data);
            pump_wrapped(
                tester,
                MediaQuery::new(
                    MediaQueryData::new()
                        .text_scaler(TextScaler::linear(2.0))
                        .size(Size::new(111.0, 111.0))
                        .device_pixel_ratio(1.1)
                        .padding(EdgeInsets::all(11.0)),
                    CircleAvatar::new().child(Builder::new(move |app, context| {
                        *seen.borrow_mut() = Some(MediaQuery::of(app, context));
                        Text::new("Z").into_widget()
                    })),
                )
                .into_widget(),
            );

            let data = seen_data.borrow().clone().expect("the child was built");
            // These should not change.
            assert_eq!(data.size, Size::new(111.0, 111.0));
            assert_eq!(data.device_pixel_ratio, 1.1);
            assert_eq!(data.padding, EdgeInsets::all(11.0));

            // This should be overridden to 1.0.
            assert_eq!(data.text_scaler, TextScaler::NO_SCALING);
            assert_eq!(
                tester
                    .global_rect(tester.render_box_of(tester.find_text("Z")[0]))
                    .size(),
                Size::new(16.0, 16.0)
            );
        });
    }

    #[test]
    fn circle_avatar_respects_min_radius() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.shade900();
            pump_wrapped(
                tester,
                UnconstrainedBox::new()
                    .child(
                        CircleAvatar::new()
                            .background_color(background_color)
                            .min_radius(50.0)
                            .child(Text::new("Z")),
                    )
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(100.0, 100.0));
            assert_eq!(avatar_color(tester), Some(background_color));
            assert_eq!(
                text_color(tester),
                Some(fallback(tester).primary_color_light)
            );
        });
    }

    #[test]
    fn circle_avatar_respects_max_radius() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.shade900();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .background_color(background_color)
                    .max_radius(50.0)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(100.0, 100.0));
            assert_eq!(avatar_color(tester), Some(background_color));
            assert_eq!(
                text_color(tester),
                Some(fallback(tester).primary_color_light)
            );
        });
    }

    #[test]
    fn circle_avatar_respects_setting_both_min_radius_and_max_radius() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.shade900();
            pump_wrapped(
                tester,
                CircleAvatar::new()
                    .background_color(background_color)
                    .max_radius(50.0)
                    .min_radius(50.0)
                    .child(Text::new("Z"))
                    .into_widget(),
            );

            assert_eq!(avatar_rect(tester).size(), Size::new(100.0, 100.0));
            assert_eq!(avatar_color(tester), Some(background_color));
            assert_eq!(
                text_color(tester),
                Some(fallback(tester).primary_color_light)
            );
        });
    }

    #[test]
    fn material2_circle_avatar_default_colors_with_light_theme() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .primary_color(Colors::GREY.shade100())
                .build(&tester.cell.borrow());
            pump_wrapped(
                tester,
                Theme::new(theme.clone(), CircleAvatar::new().child(Text::new("Z"))).into_widget(),
            );

            assert_eq!(avatar_color(tester), Some(theme.primary_color_light));
            assert_eq!(
                paragraph_style(tester, "Z").color,
                theme
                    .primary_text_theme
                    .title_large
                    .clone()
                    .expect("a title large style")
                    .color
            );
        });
    }

    #[test]
    fn material2_circle_avatar_default_colors_with_dark_theme() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .primary_color(Colors::GREY.shade800())
                .build(&tester.cell.borrow());
            pump_wrapped(
                tester,
                Theme::new(theme.clone(), CircleAvatar::new().child(Text::new("Z"))).into_widget(),
            );

            assert_eq!(avatar_color(tester), Some(theme.primary_color_dark));
            assert_eq!(
                paragraph_style(tester, "Z").color,
                theme
                    .primary_text_theme
                    .title_large
                    .clone()
                    .expect("a title large style")
                    .color
            );
        });
    }

    #[test]
    fn circle_avatar_renders_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                SizedBox::shrink()
                    .child(CircleAvatar::new().child(Text::new("X")))
                    .into_widget()
            });
            assert_eq!(tester.find_by_type::<CircleAvatar>().len(), 1);
        });
    }

    #[test]
    #[should_panic]
    fn a_radius_with_a_min_radius_asserts() {
        let _ = CircleAvatar::new().radius(10.0).min_radius(5.0);
    }

    #[test]
    fn circle_avatar_without_a_child_is_a_plain_circle() {
        test_widgets(|tester| {
            pump_wrapped(tester, CircleAvatar::new().radius(10.0).into_widget());
            assert_eq!(avatar_rect(tester).size(), Size::new(20.0, 20.0));
            assert!(
                tester
                    .descendants_of_type::<Center>(tester.find_by_type::<CircleAvatar>()[0])
                    .is_empty()
            );
        });
    }
}
