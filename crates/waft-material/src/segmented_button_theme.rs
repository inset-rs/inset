//! Flutter counterpart: material_ui's `segmented_button_theme.dart`.

use std::rc::Rc;

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;

/// Overrides the default values of visual properties for descendant
/// `SegmentedButton` widgets.
///
/// Descendant widgets obtain the current [`SegmentedButtonThemeData`] object with
/// [`SegmentedButtonTheme::of`]. Instances of [`SegmentedButtonThemeData`] can
/// be customized with [`SegmentedButtonThemeData::copy_with`].
///
/// Typically a [`SegmentedButtonThemeData`] is specified as part of the overall
/// `Theme` with `ThemeData::segmented_button_theme`.
///
/// All [`SegmentedButtonThemeData`] properties are null by default. When null,
/// the `SegmentedButton` computes its own default values, typically based on
/// the overall theme's `ThemeData::color_scheme`, `ThemeData::text_theme`, and
/// `ThemeData::icon_theme`.
#[derive(Clone, Debug, Default)]
pub struct SegmentedButtonThemeData {
    /// Overrides the `SegmentedButton`'s default style.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty`
    /// values override the default values used by `SegmentedButton`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    ///
    /// If [`ButtonStyle::side`] is provided, `WidgetStateProperty::resolve` is used
    /// for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Selected`.
    pub style: Option<ButtonStyle>,

    /// Override for `SegmentedButton::selected_icon` property.
    ///
    /// If non-null, then [`selected_icon`](Self::selected_icon) will be used instead of default
    /// value for `SegmentedButton::selected_icon`.
    pub selected_icon: Option<WidgetRef>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `SegmentedButtonThemeData`: the theme data answers with its fields,
    /// and a defaults subclass (Dart's `_SegmentedButtonDefaultsM3(context)`) computes the ones
    /// it overrides when they are read.
    pub trait SegmentedButtonThemeDataGetters for SegmentedButtonThemeData,
    fields: segmented_button_theme_data {
        style: Option<ButtonStyle>,
        selected_icon: Option<WidgetRef>,
    }
}

impl SegmentedButtonThemeData {
    /// Creates a [`SegmentedButtonThemeData`] that can be used to override default properties
    /// in a [`SegmentedButtonTheme`] widget.
    pub fn new() -> SegmentedButtonThemeData {
        SegmentedButtonThemeData::default()
    }

    /// Dart `SegmentedButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> SegmentedButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Dart `SegmentedButtonThemeData(selectedIcon:)`.
    pub fn selected_icon<K>(
        mut self,
        selected_icon: impl IntoWidget<K>,
    ) -> SegmentedButtonThemeData {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().style(style)`).
    pub fn copy_with(&self) -> SegmentedButtonThemeData {
        self.clone()
    }

    /// Linearly interpolates between two segmented button themes.
    pub fn lerp(
        a: Option<&SegmentedButtonThemeData>,
        b: Option<&SegmentedButtonThemeData>,
        t: f64,
    ) -> SegmentedButtonThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        SegmentedButtonThemeData {
            style: ButtonStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
            selected_icon: if t < 0.5 {
                a.and_then(|a| a.selected_icon.clone())
            } else {
                b.and_then(|b| b.selected_icon.clone())
            },
        }
    }
}

/// A widget is equal to the widget it is: `selectedIcon` compares by identity.
impl PartialEq for SegmentedButtonThemeData {
    fn eq(&self, other: &SegmentedButtonThemeData) -> bool {
        self.style == other.style
            && match (&self.selected_icon, &other.selected_icon) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// An inherited widget that defines the visual properties for
/// `SegmentedButton`s in this widget's subtree.
///
/// Values specified here are used for `SegmentedButton` properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct SegmentedButtonTheme {
    pub key: Option<KeyRef>,
    /// Specifies the visual properties used by descendant `SegmentedButton`
    /// widgets.
    pub data: SegmentedButtonThemeData,
    pub child: WidgetRef,
}

impl SegmentedButtonTheme {
    /// Creates a [`SegmentedButtonTheme`] that controls visual parameters for
    /// descendant `SegmentedButton`s.
    pub fn new<K>(
        data: SegmentedButtonThemeData,
        child: impl IntoWidget<K>,
    ) -> SegmentedButtonTheme {
        SegmentedButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SegmentedButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SegmentedButtonTheme {
        self.key = Some(key);
        self
    }

    /// The [`data`](Self::data) from the closest instance of this class that encloses the given
    /// context.
    ///
    /// If there is no [`SegmentedButtonTheme`] in scope, this will return
    /// `ThemeData::segmented_button_theme` from the ambient `Theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = SegmentedButtonTheme::of(app, context);
    /// ```
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which returns null if it doesn't find a
    ///    [`SegmentedButtonTheme`] ancestor.
    pub fn of(app: &mut App, context: BuildContext) -> SegmentedButtonThemeData {
        match SegmentedButtonTheme::maybe_of(app, context) {
            Some(data) => data,
            None => Theme::of(app, context).segmented_button_theme.clone(),
        }
    }

    /// The data from the closest instance of this class that encloses the given
    /// context, if any.
    ///
    /// Use this function if you want to allow situations where no
    /// [`SegmentedButtonTheme`] is in scope. Prefer using [`SegmentedButtonTheme::of`]
    /// in situations where a [`SegmentedButtonThemeData`] is expected to be
    /// non-null.
    ///
    /// If there is no [`SegmentedButtonTheme`] in scope, then this function will
    /// return null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// if let Some(theme) = SegmentedButtonTheme::maybe_of(app, context) {
    ///     // ...
    /// }
    /// ```
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), which will return `ThemeData::segmented_button_theme` if it doesn't
    ///    find a [`SegmentedButtonTheme`] ancestor, instead of returning null.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<SegmentedButtonThemeData> {
        context
            .depend_on_inherited_widget_of_exact_type::<SegmentedButtonTheme>(app)
            .map(|theme| theme.data.clone())
    }
}

impl InheritedTheme for SegmentedButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        SegmentedButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for SegmentedButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SegmentedButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
#[allow(deprecated, reason = "Dart's tests call `withOpacity`")]
mod tests {
    //! Ported from `test/segmented_button_theme_test.dart`; the `debugFillProperties` case waits
    //! for diagnostics, and `hashCode` for `Hash`.

    use std::time::Duration;

    use waft_embedder::{Color, FontWeight};
    use waft_painting::{RoundedRectangleBorder, TextStyle};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        Center, Icon, IconData, WidgetState, WidgetStateProperty, WidgetStatePropertyRef,
    };

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::segmented_button::tests::{
        argb, ink_rect_colors, label, mouse_at, numbered, text_center,
    };
    use crate::segmented_button::{ButtonSegment, SegmentedButton};
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    #[test]
    fn segmented_button_theme_data_copy_with_and_equality_basics() {
        assert_eq!(
            SegmentedButtonThemeData::new(),
            SegmentedButtonThemeData::new().copy_with()
        );

        let custom = SegmentedButtonThemeData::new()
            .style(ButtonStyle::new().background_color(Rc::new(
                waft_widgets::WidgetStatePropertyAll::new(Some(Colors::GREEN.primary())),
            )))
            .selected_icon(Icon::new(Some(Icons::error())));
        let copy = SegmentedButtonThemeData::new().copy_with();
        let mut copy = copy;
        copy.style = custom.style.clone();
        copy.selected_icon = custom.selected_icon.clone();
        assert_eq!(copy, custom);
    }

    #[test]
    fn segmented_button_theme_data_lerp_special_cases() {
        assert_eq!(
            SegmentedButtonThemeData::lerp(None, None, 0.0),
            SegmentedButtonThemeData::new()
        );
        let theme = SegmentedButtonThemeData::new();
        assert_eq!(
            SegmentedButtonThemeData::lerp(Some(&theme), Some(&theme), 0.5),
            theme
        );
    }

    /// What Dart's `ButtonStyle(backgroundColor: ..resolveWith, foregroundColor: ..resolveWith)`
    /// of these tests gives: the colors for the disabled, the selected and (the foreground only)
    /// every other state.
    fn colored_style(
        disabled: (Color, Color),
        selected: (Color, Color),
        other_foreground: Color,
    ) -> ButtonStyle {
        let background: WidgetStatePropertyRef<Option<Color>> =
            <dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(disabled.0);
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(selected.0);
                }
                None
            });
        let foreground: WidgetStatePropertyRef<Option<Color>> =
            <dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(disabled.1);
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(selected.1);
                }
                Some(other_foreground)
            });
        ButtonStyle::new()
            .background_color(background)
            .foreground_color(foreground)
    }

    fn three_segments() -> Vec<ButtonSegment<i32>> {
        let mut segments = numbered(&[1, 2, 3]);
        segments[2].enabled = false;
        segments
    }

    /// What a segment's `Material` and selected icon were expected to be.
    struct Expected {
        animation_duration: bool,
        color: Color,
        text_color: Color,
        selected_icon: Option<IconData>,
    }

    fn check_segment(tester: &WidgetTester, text: &str, expected: Expected) {
        let text_element = tester.find_text(text)[0];
        let parent = tester.ancestor_of_type::<Material>(text_element);
        let icons = tester
            .descendants_of_type::<Icon>(parent)
            .into_iter()
            .filter(|icon| {
                tester.widget(*icon, |icon: &Icon| {
                    expected.selected_icon.is_some() && icon.icon == expected.selected_icon
                })
            })
            .count();
        tester.widget(parent, |material: &Material| {
            if expected.animation_duration {
                assert_eq!(material.animation_duration, Duration::from_millis(200));
                assert_eq!(material.border_radius, None);
            }
            assert_eq!(material.color, Some(expected.color));
            assert_eq!(
                material.shape,
                Some(waft_painting::ShapeBorderRef::new(
                    RoundedRectangleBorder::default()
                ))
            );
            let text_style: &TextStyle = material.text_style.as_ref().expect("a text style");
            assert_eq!(
                text_style.color.as_deref().copied(),
                Some(expected.text_color)
            );
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.font_size, Some(14.0));
            assert_eq!(text_style.font_weight, Some(FontWeight::W500));
        });
        assert_eq!(icons, usize::from(expected.selected_icon.is_some()));
    }

    fn pump_three(
        tester: &mut WidgetTester,
        theme: ThemeData,
        home: impl FnOnce(WidgetRef) -> WidgetRef,
        button: SegmentedButton<i32>,
    ) {
        let home = home(
            Scaffold::new()
                .body(Center::new().child(button))
                .into_widget(),
        );
        tester.pump_widget(material_app(theme, home));
    }

    // 'With no other configuration, defaults are used'
    #[test]
    fn with_no_other_configuration_the_defaults_are_used() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            let colors = theme.color_scheme.clone();
            pump_three(
                tester,
                theme,
                |home| home,
                SegmentedButton::new(three_segments(), vec![2])
                    .on_selection_changed(|_app, _selected| {}),
            );

            // Test first segment, should be enabled
            check_segment(
                tester,
                "1",
                Expected {
                    animation_duration: false,
                    color: Colors::TRANSPARENT,
                    text_color: colors.on_surface,
                    selected_icon: None,
                },
            );
            // Test second segment, should be enabled and selected
            check_segment(
                tester,
                "2",
                Expected {
                    animation_duration: false,
                    color: colors.secondary_container(),
                    text_color: colors.on_secondary_container(),
                    selected_icon: Some(Icons::check()),
                },
            );
            // Test last segment, should be disabled
            check_segment(
                tester,
                "3",
                Expected {
                    animation_duration: false,
                    color: Colors::TRANSPARENT,
                    text_color: colors.on_surface.with_opacity(0.38),
                    selected_icon: None,
                },
            );
        });
    }

    fn global_theme_data() -> SegmentedButtonThemeData {
        SegmentedButtonThemeData::new()
            .style(colored_style(
                (Colors::BLUE.primary(), Colors::YELLOW.primary()),
                (Colors::PURPLE.primary(), Colors::BROWN.primary()),
                Colors::CYAN.primary(),
            ))
            .selected_icon(Icon::new(Some(Icons::error())))
    }

    fn segmented_theme_data() -> SegmentedButtonThemeData {
        SegmentedButtonThemeData::new()
            .style(colored_style(
                (Colors::LIGHT_BLUE.primary(), Colors::LIME.primary()),
                (Colors::LIGHT_GREEN.primary(), Colors::AMBER.primary()),
                Colors::DEEP_PURPLE.primary(),
            ))
            .selected_icon(Icon::new(Some(Icons::plus_one())))
    }

    // 'ThemeData.segmentedButtonTheme overrides defaults'
    #[test]
    fn theme_data_segmented_button_theme_overrides_the_defaults() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .segmented_button_theme(global_theme_data())
                .build(&tester.cell.borrow());
            pump_three(
                tester,
                theme,
                |home| home,
                SegmentedButton::new(three_segments(), vec![2])
                    .on_selection_changed(|_app, _selected| {}),
            );

            check_segment(
                tester,
                "1",
                Expected {
                    animation_duration: false,
                    color: Colors::TRANSPARENT,
                    text_color: Colors::CYAN.primary(),
                    selected_icon: None,
                },
            );
            check_segment(
                tester,
                "2",
                Expected {
                    animation_duration: false,
                    color: Colors::PURPLE.primary(),
                    text_color: Colors::BROWN.primary(),
                    selected_icon: Some(Icons::error()),
                },
            );
            check_segment(
                tester,
                "3",
                Expected {
                    animation_duration: false,
                    color: Colors::BLUE.primary(),
                    text_color: Colors::YELLOW.primary(),
                    selected_icon: None,
                },
            );
        });
    }

    // 'SegmentedButtonTheme overrides ThemeData and defaults'
    #[test]
    fn segmented_button_theme_overrides_theme_data_and_the_defaults() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .segmented_button_theme(global_theme_data())
                .build(&tester.cell.borrow());
            pump_three(
                tester,
                theme,
                |home| SegmentedButtonTheme::new(segmented_theme_data(), home).into_widget(),
                SegmentedButton::new(three_segments(), vec![2])
                    .on_selection_changed(|_app, _selected| {}),
            );

            check_segment(
                tester,
                "1",
                Expected {
                    animation_duration: true,
                    color: Colors::TRANSPARENT,
                    text_color: Colors::DEEP_PURPLE.primary(),
                    selected_icon: None,
                },
            );
            check_segment(
                tester,
                "2",
                Expected {
                    animation_duration: true,
                    color: Colors::LIGHT_GREEN.primary(),
                    text_color: Colors::AMBER.primary(),
                    selected_icon: Some(Icons::plus_one()),
                },
            );
            check_segment(
                tester,
                "3",
                Expected {
                    animation_duration: true,
                    color: Colors::LIGHT_BLUE.primary(),
                    text_color: Colors::LIME.primary(),
                    selected_icon: None,
                },
            );
        });
    }

    // 'Widget parameters overrides SegmentedTheme, ThemeData and defaults'
    #[test]
    fn widget_parameters_override_the_segmented_theme_theme_data_and_the_defaults() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .segmented_button_theme(global_theme_data())
                .build(&tester.cell.borrow());
            pump_three(
                tester,
                theme,
                |home| SegmentedButtonTheme::new(segmented_theme_data(), home).into_widget(),
                SegmentedButton::new(three_segments(), vec![2])
                    .on_selection_changed(|_app, _selected| {})
                    .style(colored_style(
                        (Colors::BLACK12, Colors::AMBER_ACCENT.primary()),
                        (Colors::GREY.primary(), Colors::DEEP_ORANGE.primary()),
                        Colors::DEEP_PURPLE_ACCENT.primary(),
                    ))
                    .selected_icon(Icon::new(Some(Icons::alarm()))),
            );

            check_segment(
                tester,
                "1",
                Expected {
                    animation_duration: true,
                    color: Colors::TRANSPARENT,
                    text_color: Colors::DEEP_PURPLE_ACCENT.primary(),
                    selected_icon: None,
                },
            );
            check_segment(
                tester,
                "2",
                Expected {
                    animation_duration: true,
                    color: Colors::GREY.primary(),
                    text_color: Colors::DEEP_ORANGE.primary(),
                    selected_icon: Some(Icons::alarm()),
                },
            );
            check_segment(
                tester,
                "3",
                Expected {
                    animation_duration: true,
                    color: Colors::BLACK12,
                    text_color: Colors::AMBER_ACCENT.primary(),
                    selected_icon: None,
                },
            );
        });
    }

    // 'SegmentedButtonTheme SegmentedButton.styleFrom overlayColor overrides default overlay
    // color'
    #[test]
    fn the_theme_style_from_overlay_color_overrides_the_default_overlay_color() {
        test_widgets(|tester| {
            let overlay_color = Color::new(0xffff0000);
            let theme = ThemeData::builder()
                .segmented_button_theme(
                    SegmentedButtonThemeData::new().style(
                        SegmentedButton::<i32>::style_from()
                            .overlay_color(overlay_color)
                            .build(),
                    ),
                )
                .build(&tester.cell.borrow());
            let segments = vec![
                ButtonSegment::new(0).label(label("Option 1")),
                ButtonSegment::new(1).label(label("Option 2")),
            ];
            pump_three(
                tester,
                theme,
                |home| home,
                SegmentedButton::new(segments, vec![1]).on_selection_changed(|_app, _selected| {}),
            );
            let hover = argb(overlay_color.with_opacity(0.08));
            let press = argb(overlay_color.with_opacity(0.1));

            // Hovered selected segment,
            let center = text_center(tester, "Option 1");
            let mut gesture = mouse_at(tester, center);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&hover));

            // Hovered unselected segment,
            let center = text_center(tester, "Option 2");
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&hover));

            // Highlighted unselected segment (pressed).
            let center = text_center(tester, "Option 1");
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(colors.contains(&hover) && colors.contains(&press));
            // Remove pressed and hovered states,
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, waft_embedder::Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Highlighted selected segment (pressed)
            let center = text_center(tester, "Option 2");
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(colors.contains(&hover) && colors.contains(&press));
            // Remove pressed and hovered states,
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, waft_embedder::Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Focused unselected segment.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&press));

            // Focused selected segment.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&press));
        });
    }
}
