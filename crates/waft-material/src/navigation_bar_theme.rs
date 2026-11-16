//! Flutter counterpart: material_ui's `navigation_bar_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_widgets::{
    BuildContext, IconThemeData, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::navigation_bar::NavigationDestinationLabelBehavior;
use crate::theme::Theme;

/// Defines default property values for descendant `NavigationBar`
/// widgets.
///
/// Descendant widgets obtain the current [`NavigationBarThemeData`] object
/// using [`NavigationBarTheme::of`]. Instances of [`NavigationBarThemeData`] can be
/// customized with [`NavigationBarThemeData::copy_with`].
///
/// Typically a [`NavigationBarThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::navigation_bar_theme`. Alternatively, a
/// [`NavigationBarTheme`] inherited widget can be used to theme `NavigationBar`s
/// in a subtree of widgets.
///
/// All [`NavigationBarThemeData`] properties are `null` by default.
/// When null, the `NavigationBar` will provide its own defaults based on the
/// overall `Theme`'s textTheme and colorScheme. See the individual
/// `NavigationBar` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct NavigationBarThemeData {
    /// Overrides the default value of `NavigationBar::height`.
    pub height: Option<f64>,

    /// Overrides the default value of `NavigationBar::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `NavigationBar::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value of `NavigationBar::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value of `NavigationBar::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of `NavigationBar`'s selection indicator.
    pub indicator_color: Option<Color>,

    /// Overrides the default shape of the `NavigationBar`'s selection indicator.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// The style to merge with the default text style for
    /// `NavigationDestination` labels.
    ///
    /// You can use this to specify a different style when the label is selected.
    pub label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The theme to merge with the default icon theme for
    /// `NavigationDestination` icons.
    ///
    /// You can use this to specify a different icon theme when the icon is
    /// selected.
    pub icon_theme: Option<WidgetStatePropertyRef<Option<IconThemeData>>>,

    /// Overrides the default value of `NavigationBar::label_behavior`.
    pub label_behavior: Option<NavigationDestinationLabelBehavior>,

    /// Overrides the default value of `NavigationBar::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of `NavigationBar::label_padding`.
    pub label_padding: Option<EdgeInsetsGeometry>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `NavigationBarThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait NavigationBarThemeDataGetters for NavigationBarThemeData, fields: navigation_bar_theme_data {
        height: Option<f64>,
        background_color: Option<Color>,
        elevation: Option<f64>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        indicator_color: Option<Color>,
        indicator_shape: Option<ShapeBorderRef>,
        label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
        icon_theme: Option<WidgetStatePropertyRef<Option<IconThemeData>>>,
        label_behavior: Option<NavigationDestinationLabelBehavior>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        label_padding: Option<EdgeInsetsGeometry>,
    }
}

impl NavigationBarThemeData {
    /// Creates a theme that can be used for `ThemeData::navigation_bar_theme` and
    /// [`NavigationBarTheme`]; Dart's optional named arguments are the fluent setters.
    pub fn new() -> NavigationBarThemeData {
        NavigationBarThemeData::default()
    }

    /// Dart `NavigationBarThemeData(height:)`.
    pub fn height(mut self, height: f64) -> NavigationBarThemeData {
        self.height = Some(height);
        self
    }

    /// Dart `NavigationBarThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationBarThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationBarThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationBarThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationBarThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> NavigationBarThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `NavigationBarThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> NavigationBarThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `NavigationBarThemeData(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationBarThemeData {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationBarThemeData(indicatorShape:)`.
    pub fn indicator_shape(
        mut self,
        indicator_shape: impl Into<ShapeBorderRef>,
    ) -> NavigationBarThemeData {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationBarThemeData(labelTextStyle:)`.
    pub fn label_text_style(
        mut self,
        label_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> NavigationBarThemeData {
        self.label_text_style = Some(label_text_style);
        self
    }

    /// Dart `NavigationBarThemeData(iconTheme:)`.
    pub fn icon_theme(
        mut self,
        icon_theme: WidgetStatePropertyRef<Option<IconThemeData>>,
    ) -> NavigationBarThemeData {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `NavigationBarThemeData(labelBehavior:)`.
    pub fn label_behavior(
        mut self,
        label_behavior: NavigationDestinationLabelBehavior,
    ) -> NavigationBarThemeData {
        self.label_behavior = Some(label_behavior);
        self
    }

    /// Dart `NavigationBarThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> NavigationBarThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `NavigationBarThemeData(labelPadding:)`.
    pub fn label_padding(
        mut self,
        label_padding: impl Into<EdgeInsetsGeometry>,
    ) -> NavigationBarThemeData {
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().height(64.0)`).
    pub fn copy_with(&self) -> NavigationBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two navigation rail themes.
    ///
    /// If both arguments are null then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&NavigationBarThemeData>,
        b: Option<&NavigationBarThemeData>,
        t: f64,
    ) -> Option<NavigationBarThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(NavigationBarThemeData {
            height: lerp_double(a.and_then(|a| a.height), b.and_then(|b| b.height), t),
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
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
            indicator_color: Color::lerp(
                a.and_then(|a| a.indicator_color),
                b.and_then(|b| b.indicator_color),
                t,
            ),
            indicator_shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.indicator_shape.as_ref()),
                b.and_then(|b| b.indicator_shape.as_ref()),
                t,
            ),
            label_text_style: <dyn WidgetStateProperty<Option<TextStyle>>>::lerp_optional(
                a.and_then(|a| a.label_text_style.clone()),
                b.and_then(|b| b.label_text_style.clone()),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            ),
            icon_theme: <dyn WidgetStateProperty<Option<IconThemeData>>>::lerp_optional(
                a.and_then(|a| a.icon_theme.clone()),
                b.and_then(|b| b.icon_theme.clone()),
                t,
                |a, b, t| Some(IconThemeData::lerp(a.as_ref(), b.as_ref(), t)),
            ),
            label_behavior: if t < 0.5 {
                a.and_then(|a| a.label_behavior)
            } else {
                b.and_then(|b| b.label_behavior)
            },
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            label_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.label_padding),
                b.and_then(|b| b.label_padding),
                t,
            ),
        })
    }
}

impl PartialEq for NavigationBarThemeData {
    fn eq(&self, other: &NavigationBarThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.height == self.height
            && other.background_color == self.background_color
            && other.elevation == self.elevation
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.indicator_color == self.indicator_color
            && other.indicator_shape == self.indicator_shape
            && other.label_text_style == self.label_text_style
            && other.icon_theme == self.icon_theme
            && other.label_behavior == self.label_behavior
            && other.overlay_color == self.overlay_color
            && other.label_padding == self.label_padding
    }
}

/// An inherited widget that defines visual properties for `NavigationBar`s and
/// `NavigationDestination`s in this widget's subtree.
///
/// Values specified here are used for `NavigationBar` properties that are not
/// given an explicit non-null value.
///
/// See also:
///
///  * `ThemeData::navigation_bar_theme`, which describes the
///    [`NavigationBarThemeData`] in the overall theme for the application.
#[derive(Debug)]
pub struct NavigationBarTheme {
    pub key: Option<KeyRef>,

    /// Specifies the background color, label text style, icon theme, and label
    /// type values for descendant `NavigationBar` widgets.
    pub data: NavigationBarThemeData,

    pub child: WidgetRef,
}

impl NavigationBarTheme {
    /// Creates a navigation rail theme that controls the
    /// [`NavigationBarThemeData`] properties for a `NavigationBar`.
    pub fn new<K>(data: NavigationBarThemeData, child: impl IntoWidget<K>) -> NavigationBarTheme {
        NavigationBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `NavigationBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationBarTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`NavigationBarThemeData`] from the closest ancestor [`NavigationBarTheme`].
    ///
    /// If there is no enclosing [`NavigationBarTheme`] widget, then
    /// `ThemeData::navigation_bar_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = NavigationBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> NavigationBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<NavigationBarTheme>(app) {
            Some(navigation_bar_theme) => navigation_bar_theme.data.clone(),
            None => Theme::of(app, context).navigation_bar_theme.clone(),
        }
    }
}

impl InheritedTheme for NavigationBarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        NavigationBarTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for NavigationBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &NavigationBarTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::ShapeBorder;
    use waft_painting::{BorderSide, CircleBorder, EdgeInsets};
    use waft_widgets::{Icon, IconData, Opacity, WidgetState};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::navigation_bar::NavigationBar;
    use crate::navigation_bar::NavigationDestination;
    use crate::navigation_bar::tests::{
        bar_material, bar_size, indicator_decoration, label_padding, label_style,
        respects_overlay_color, state_overlay_color, text,
    };
    use crate::scaffold::Scaffold;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m3, pump_app};

    #[test]
    fn copy_with_equality_basics() {
        assert_eq!(
            NavigationBarThemeData::new(),
            NavigationBarThemeData::new().copy_with()
        );
    }

    #[test]
    fn navigation_bar_theme_data_lerp_special_cases() {
        assert_eq!(NavigationBarThemeData::lerp(None, None, 0.0), None);
        let data = NavigationBarThemeData::new();
        assert_eq!(
            NavigationBarThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn navigation_bar_theme_data_lerps_each_field() {
        let a = NavigationBarThemeData::new()
            .height(60.0)
            .background_color(Color::new(0xff000000))
            .elevation(2.0)
            .label_behavior(NavigationDestinationLabelBehavior::AlwaysShow)
            .label_padding(EdgeInsets::all(0.0))
            .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::all(Some(
                TextStyle::new().font_size(10.0),
            )));
        let b = NavigationBarThemeData::new()
            .height(100.0)
            .background_color(Color::new(0xffffffff))
            .elevation(6.0)
            .label_behavior(NavigationDestinationLabelBehavior::AlwaysHide)
            .label_padding(EdgeInsets::all(8.0))
            .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::all(Some(
                TextStyle::new().font_size(20.0),
            )));

        let halfway = NavigationBarThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");
        assert_eq!(halfway.height, Some(80.0));
        assert_eq!(
            halfway.background_color,
            Color::lerp(
                Some(Color::new(0xff000000)),
                Some(Color::new(0xffffffff)),
                0.5
            )
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(
            halfway.label_behavior,
            Some(NavigationDestinationLabelBehavior::AlwaysHide)
        );
        assert_eq!(halfway.label_padding, Some(EdgeInsets::all(4.0).into()));
        assert_eq!(
            halfway
                .label_text_style
                .expect("both sides give one")
                .resolve(&Default::default())
                .and_then(|style| style.font_size),
            Some(15.0)
        );
        assert_eq!(
            NavigationBarThemeData::lerp(Some(&a), Some(&b), 0.25)
                .expect("both given")
                .label_behavior,
            Some(NavigationDestinationLabelBehavior::AlwaysShow)
        );
    }

    /// Dart's `_destinations()`.
    fn destinations() -> Vec<WidgetRef> {
        let icon = |icon: IconData| Icon::new(Some(icon));
        vec![
            NavigationDestination::new(icon(Icons::favorite_border()), "Abc")
                .selected_icon(icon(Icons::favorite()))
                .into_widget(),
            NavigationDestination::new(icon(Icons::star_border()), "Def")
                .selected_icon(icon(Icons::star()))
                .into_widget(),
        ]
    }

    /// Dart's `_iconTheme(tester, icon)`: the nearest `IconTheme` above the icon.
    fn icon_theme(tester: &WidgetTester, icon: &IconData) -> IconThemeData {
        let icon_theme =
            tester.ancestor_of_type::<waft_widgets::IconTheme>(tester.find_by_icon(icon));
        tester.widget(icon_theme, |icon_theme: &waft_widgets::IconTheme| {
            icon_theme.data.clone()
        })
    }

    /// Dart's `_labelBehavior(tester)`: `AlwaysShow` unless an `Opacity` is above both labels.
    fn label_behavior(tester: &WidgetTester) -> NavigationDestinationLabelBehavior {
        let opacity_above = |label: &str| {
            let element = text(tester, label);
            tester.ancestors(element).into_iter().find(|ancestor| {
                let app = tester.cell.borrow();
                waft_widgets::downcast_widget::<Opacity>(&**ancestor.widget(&app)).is_some()
            })
        };
        match (opacity_above("Abc"), opacity_above("Def")) {
            (Some(abc), Some(_)) => {
                if tester.widget(abc, |opacity: &Opacity| opacity.opacity) == 1.0 {
                    NavigationDestinationLabelBehavior::OnlyShowSelected
                } else {
                    NavigationDestinationLabelBehavior::AlwaysHide
                }
            }
            _ => NavigationDestinationLabelBehavior::AlwaysShow,
        }
    }

    #[test]
    fn navigation_bar_theme_data_values_are_used_when_no_navigation_bar_properties_are_specified() {
        test_widgets(|tester| {
            let height = 200.0;
            let background_color = Color::new(0x00000001);
            let elevation = 42.0;
            let indicator_color = Color::new(0x00000002);
            let indicator_shape = || CircleBorder::new(BorderSide::NONE, 0.0);
            let selected_icon_size = 25.0;
            let unselected_icon_size = 23.0;
            let selected_icon_color = Color::new(0x00000003);
            let unselected_icon_color = Color::new(0x00000004);
            let selected_icon_opacity = 0.99;
            let unselected_icon_opacity = 0.98;
            let selected_label_font_size = 13.0;
            let unselected_label_font_size = 11.0;
            let behavior = NavigationDestinationLabelBehavior::AlwaysShow;
            let padding = EdgeInsetsGeometry::all(8.0);

            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .bottom_navigation_bar(NavigationBarTheme::new(
                        NavigationBarThemeData::new()
                            .height(height)
                            .background_color(background_color)
                            .elevation(elevation)
                            .indicator_color(indicator_color)
                            .indicator_shape(indicator_shape())
                            .icon_theme(
                                <dyn WidgetStateProperty<Option<IconThemeData>>>::resolve_with(
                                    move |states| {
                                        Some(if states.contains(&WidgetState::Selected) {
                                            IconThemeData::new()
                                                .size(selected_icon_size)
                                                .color(selected_icon_color)
                                                .opacity(selected_icon_opacity)
                                        } else {
                                            IconThemeData::new()
                                                .size(unselected_icon_size)
                                                .color(unselected_icon_color)
                                                .opacity(unselected_icon_opacity)
                                        })
                                    },
                                ),
                            )
                            .label_text_style(
                                <dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
                                    move |states| {
                                        Some(if states.contains(&WidgetState::Selected) {
                                            TextStyle::new().font_size(selected_label_font_size)
                                        } else {
                                            TextStyle::new().font_size(unselected_label_font_size)
                                        })
                                    },
                                ),
                            )
                            .label_behavior(behavior)
                            .label_padding(padding),
                        NavigationBar::new(destinations()),
                    ))
                    .into_widget()
            });

            assert_eq!(bar_size(tester).height(), height);
            assert_eq!(
                bar_material(tester, |material: &Material| material.color),
                Some(background_color)
            );
            assert_eq!(
                bar_material(tester, |material: &Material| material.elevation),
                elevation
            );
            let (color, shape) = indicator_decoration(tester);
            assert_eq!(color, Some(indicator_color));
            assert_eq!(&*shape, &indicator_shape() as &dyn ShapeBorder);
            let selected = icon_theme(tester, &Icons::favorite());
            assert_eq!(selected.size, Some(selected_icon_size));
            assert_eq!(
                selected.color.as_deref().copied(),
                Some(selected_icon_color)
            );
            assert_eq!(selected.opacity, Some(selected_icon_opacity));
            let unselected = icon_theme(tester, &Icons::star_border());
            assert_eq!(unselected.size, Some(unselected_icon_size));
            assert_eq!(
                unselected.color.as_deref().copied(),
                Some(unselected_icon_color)
            );
            assert_eq!(unselected.opacity, Some(unselected_icon_opacity));
            assert_eq!(
                label_style(tester, "Abc").font_size,
                Some(selected_label_font_size)
            );
            assert_eq!(
                label_style(tester, "Def").font_size,
                Some(unselected_label_font_size)
            );
            assert_eq!(label_behavior(tester), behavior);
            assert_eq!(label_padding(tester, "Abc"), padding);
            assert_eq!(label_padding(tester, "Def"), padding);
        });
    }

    #[test]
    fn navigation_bar_values_take_priority_over_navigation_bar_theme_data_values_when_both_properties_are_specified()
     {
        test_widgets(|tester| {
            let height = 200.0;
            let background_color = Color::new(0x00000001);
            let elevation = 42.0;
            let behavior = NavigationDestinationLabelBehavior::AlwaysShow;
            let padding = EdgeInsetsGeometry::symmetric(0.0, 16.0);

            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .bottom_navigation_bar(NavigationBarTheme::new(
                        NavigationBarThemeData::new()
                            .height(100.0)
                            .elevation(18.0)
                            .background_color(Color::new(0x00000099))
                            .label_behavior(NavigationDestinationLabelBehavior::AlwaysHide)
                            .label_padding(EdgeInsets::all(8.0)),
                        NavigationBar::new(destinations())
                            .height(height)
                            .elevation(elevation)
                            .background_color(background_color)
                            .label_behavior(behavior)
                            .label_padding(padding),
                    ))
                    .into_widget()
            });

            assert_eq!(bar_size(tester).height(), height);
            assert_eq!(
                bar_material(tester, |material: &Material| material.color),
                Some(background_color)
            );
            assert_eq!(
                bar_material(tester, |material: &Material| material.elevation),
                elevation
            );
            assert_eq!(label_behavior(tester), behavior);
            assert_eq!(label_padding(tester, "Abc"), padding);
            assert_eq!(label_padding(tester, "Def"), padding);
        });
    }

    #[test]
    fn navigation_bar_respects_navigation_bar_theme_overlay_color_in_active_pressed_hovered_states()
    {
        test_widgets(|tester| {
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .navigation_bar_theme(
                            NavigationBarThemeData::new().overlay_color(state_overlay_color()),
                        )
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .bottom_navigation_bar(
                            waft_widgets::RepaintBoundary::new().child(
                                NavigationBar::new(crate::navigation_bar::tests::ac_alarm())
                                    .on_destination_selected(|_app, _i| {}),
                            ),
                        )
                        .into_widget()
                },
            );

            respects_overlay_color(tester);
        });
    }

    #[test]
    fn of_returns_the_closest_navigation_bar_theme_over_theme_data() {
        test_widgets(|tester| {
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .navigation_bar_theme(NavigationBarThemeData::new().height(10.0))
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .bottom_navigation_bar(NavigationBarTheme::new(
                            NavigationBarThemeData::new().height(64.0),
                            NavigationBar::new(destinations()),
                        ))
                        .into_widget()
                },
            );
            assert_eq!(bar_size(tester).height(), 64.0);
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: NavigationBarThemeData| {
            NavigationBarTheme::new(data, waft_widgets::SizedBox::shrink())
        };
        let old = theme(NavigationBarThemeData::new().height(1.0));

        assert!(!theme(NavigationBarThemeData::new().height(1.0)).update_should_notify(&old));
        assert!(theme(NavigationBarThemeData::new().height(2.0)).update_should_notify(&old));
    }
}
