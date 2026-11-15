//! Flutter counterpart: material_ui's `bottom_navigation_bar_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::TextStyle;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, IconThemeData, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStatePropertyRef,
};

use crate::bottom_navigation_bar::{BottomNavigationBarLandscapeLayout, BottomNavigationBarType};
use crate::theme::Theme;

/// Defines default property values for descendant `BottomNavigationBar`
/// widgets.
///
/// Descendant widgets obtain the current [`BottomNavigationBarThemeData`] object
/// using [`BottomNavigationBarTheme::of`]. Instances of
/// [`BottomNavigationBarThemeData`] can be customized with
/// [`BottomNavigationBarThemeData::copy_with`].
///
/// Typically a [`BottomNavigationBarThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::bottom_navigation_bar_theme`.
///
/// All [`BottomNavigationBarThemeData`] properties are `None` by default. When
/// null, the `BottomNavigationBar`'s build method provides defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct BottomNavigationBarThemeData {
    /// The color of the `BottomNavigationBar` itself.
    ///
    /// See `BottomNavigationBar::background_color`.
    pub background_color: Option<Color>,

    /// The z-coordinate of the `BottomNavigationBar`.
    ///
    /// See `BottomNavigationBar::elevation`.
    pub elevation: Option<f64>,

    /// The size, opacity, and color of the icon in the currently selected
    /// `BottomNavigationBarItem::icon`.
    ///
    /// If `BottomNavigationBar::selected_icon_theme` is non-null on the widget,
    /// the whole [`IconThemeData`] from the widget will be used over this
    /// [`selected_icon_theme`](Self::selected_icon_theme).
    ///
    /// See `BottomNavigationBar::selected_icon_theme`.
    pub selected_icon_theme: Option<IconThemeData>,

    /// The size, opacity, and color of the icon in the currently unselected
    /// `BottomNavigationBarItem::icon`s.
    ///
    /// If `BottomNavigationBar::unselected_icon_theme` is non-null on the widget,
    /// the whole [`IconThemeData`] from the widget will be used over this
    /// [`unselected_icon_theme`](Self::unselected_icon_theme).
    ///
    /// See `BottomNavigationBar::unselected_icon_theme`.
    pub unselected_icon_theme: Option<IconThemeData>,

    /// The color of the selected `BottomNavigationBarItem::icon` and
    /// `BottomNavigationBarItem::label`.
    ///
    /// See `BottomNavigationBar::selected_item_color`.
    pub selected_item_color: Option<Color>,

    /// The color of the unselected `BottomNavigationBarItem::icon` and
    /// `BottomNavigationBarItem::label`s.
    ///
    /// See `BottomNavigationBar::unselected_item_color`.
    pub unselected_item_color: Option<Color>,

    /// The [`TextStyle`] of the `BottomNavigationBarItem` labels when they are
    /// selected.
    ///
    /// See `BottomNavigationBar::selected_label_style`.
    pub selected_label_style: Option<TextStyle>,

    /// The [`TextStyle`] of the `BottomNavigationBarItem` labels when they are not
    /// selected.
    ///
    /// See `BottomNavigationBar::unselected_label_style`.
    pub unselected_label_style: Option<TextStyle>,

    /// Whether the labels are shown for the selected `BottomNavigationBarItem`.
    ///
    /// See `BottomNavigationBar::show_selected_labels`.
    pub show_selected_labels: Option<bool>,

    /// Whether the labels are shown for the unselected `BottomNavigationBarItem`s.
    ///
    /// See `BottomNavigationBar::show_unselected_labels`.
    pub show_unselected_labels: Option<bool>,

    /// Defines the layout and behavior of a `BottomNavigationBar`.
    ///
    /// See `BottomNavigationBar::type`.
    pub r#type: Option<BottomNavigationBarType>,

    /// If specified, defines the feedback property for `BottomNavigationBar`.
    ///
    /// If `BottomNavigationBar::enable_feedback` is provided,
    /// [`enable_feedback`](Self::enable_feedback) is ignored.
    pub enable_feedback: Option<bool>,

    /// If non-null, overrides the `BottomNavigationBar::landscape_layout` property.
    pub landscape_layout: Option<BottomNavigationBarLandscapeLayout>,

    /// If specified, overrides the default value of `BottomNavigationBar::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
}

impl BottomNavigationBarThemeData {
    /// Creates a theme that can be used for `ThemeData::bottom_navigation_bar_theme`;
    /// Dart's optional named arguments are the fluent setters.
    pub fn new() -> BottomNavigationBarThemeData {
        BottomNavigationBarThemeData::default()
    }

    /// Dart `BottomNavigationBarThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> BottomNavigationBarThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `BottomNavigationBarThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> BottomNavigationBarThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `BottomNavigationBarThemeData(selectedIconTheme:)`.
    pub fn selected_icon_theme(
        mut self,
        selected_icon_theme: IconThemeData,
    ) -> BottomNavigationBarThemeData {
        self.selected_icon_theme = Some(selected_icon_theme);
        self
    }

    /// Dart `BottomNavigationBarThemeData(unselectedIconTheme:)`.
    pub fn unselected_icon_theme(
        mut self,
        unselected_icon_theme: IconThemeData,
    ) -> BottomNavigationBarThemeData {
        self.unselected_icon_theme = Some(unselected_icon_theme);
        self
    }

    /// Dart `BottomNavigationBarThemeData(selectedItemColor:)`.
    pub fn selected_item_color(
        mut self,
        selected_item_color: Color,
    ) -> BottomNavigationBarThemeData {
        self.selected_item_color = Some(selected_item_color);
        self
    }

    /// Dart `BottomNavigationBarThemeData(unselectedItemColor:)`.
    pub fn unselected_item_color(
        mut self,
        unselected_item_color: Color,
    ) -> BottomNavigationBarThemeData {
        self.unselected_item_color = Some(unselected_item_color);
        self
    }

    /// Dart `BottomNavigationBarThemeData(selectedLabelStyle:)`.
    pub fn selected_label_style(
        mut self,
        selected_label_style: TextStyle,
    ) -> BottomNavigationBarThemeData {
        self.selected_label_style = Some(selected_label_style);
        self
    }

    /// Dart `BottomNavigationBarThemeData(unselectedLabelStyle:)`.
    pub fn unselected_label_style(
        mut self,
        unselected_label_style: TextStyle,
    ) -> BottomNavigationBarThemeData {
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    /// Dart `BottomNavigationBarThemeData(showSelectedLabels:)`.
    pub fn show_selected_labels(
        mut self,
        show_selected_labels: bool,
    ) -> BottomNavigationBarThemeData {
        self.show_selected_labels = Some(show_selected_labels);
        self
    }

    /// Dart `BottomNavigationBarThemeData(showUnselectedLabels:)`.
    pub fn show_unselected_labels(
        mut self,
        show_unselected_labels: bool,
    ) -> BottomNavigationBarThemeData {
        self.show_unselected_labels = Some(show_unselected_labels);
        self
    }

    /// Dart `BottomNavigationBarThemeData(type:)`.
    pub fn r#type(mut self, r#type: BottomNavigationBarType) -> BottomNavigationBarThemeData {
        self.r#type = Some(r#type);
        self
    }

    /// Dart `BottomNavigationBarThemeData(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> BottomNavigationBarThemeData {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `BottomNavigationBarThemeData(landscapeLayout:)`.
    pub fn landscape_layout(
        mut self,
        landscape_layout: BottomNavigationBarLandscapeLayout,
    ) -> BottomNavigationBarThemeData {
        self.landscape_layout = Some(landscape_layout);
        self
    }

    /// Dart `BottomNavigationBarThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> BottomNavigationBarThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(4.0)`).
    pub fn copy_with(&self) -> BottomNavigationBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`BottomNavigationBarThemeData`].
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&BottomNavigationBarThemeData>,
        b: Option<&BottomNavigationBarThemeData>,
        t: f64,
    ) -> BottomNavigationBarThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        BottomNavigationBarThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            selected_icon_theme: Some(IconThemeData::lerp(
                a.and_then(|a| a.selected_icon_theme.as_ref()),
                b.and_then(|b| b.selected_icon_theme.as_ref()),
                t,
            )),
            unselected_icon_theme: Some(IconThemeData::lerp(
                a.and_then(|a| a.unselected_icon_theme.as_ref()),
                b.and_then(|b| b.unselected_icon_theme.as_ref()),
                t,
            )),
            selected_item_color: Color::lerp(
                a.and_then(|a| a.selected_item_color),
                b.and_then(|b| b.selected_item_color),
                t,
            ),
            unselected_item_color: Color::lerp(
                a.and_then(|a| a.unselected_item_color),
                b.and_then(|b| b.unselected_item_color),
                t,
            ),
            selected_label_style: TextStyle::lerp(
                a.and_then(|a| a.selected_label_style.as_ref()),
                b.and_then(|b| b.selected_label_style.as_ref()),
                t,
            ),
            unselected_label_style: TextStyle::lerp(
                a.and_then(|a| a.unselected_label_style.as_ref()),
                b.and_then(|b| b.unselected_label_style.as_ref()),
                t,
            ),
            show_selected_labels: pick(a, b, t, |theme| theme.show_selected_labels),
            show_unselected_labels: pick(a, b, t, |theme| theme.show_unselected_labels),
            r#type: pick(a, b, t, |theme| theme.r#type),
            enable_feedback: pick(a, b, t, |theme| theme.enable_feedback),
            landscape_layout: pick(a, b, t, |theme| theme.landscape_layout),
            mouse_cursor: pick(a, b, t, |theme| theme.mouse_cursor.clone()),
        }
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&BottomNavigationBarThemeData>,
    b: Option<&BottomNavigationBarThemeData>,
    t: f64,
    field: impl Fn(&BottomNavigationBarThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

impl PartialEq for BottomNavigationBarThemeData {
    fn eq(&self, other: &BottomNavigationBarThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.background_color == self.background_color
            && other.elevation == self.elevation
            && other.selected_icon_theme == self.selected_icon_theme
            && other.unselected_icon_theme == self.unselected_icon_theme
            && other.selected_item_color == self.selected_item_color
            && other.unselected_item_color == self.unselected_item_color
            && other.selected_label_style == self.selected_label_style
            && other.unselected_label_style == self.unselected_label_style
            && other.show_selected_labels == self.show_selected_labels
            && other.show_unselected_labels == self.show_unselected_labels
            && other.r#type == self.r#type
            && other.enable_feedback == self.enable_feedback
            && other.landscape_layout == self.landscape_layout
            && other.mouse_cursor == self.mouse_cursor
    }
}

/// Applies a bottom navigation bar theme to descendant `BottomNavigationBar`
/// widgets.
///
/// Descendant widgets obtain the current theme's [`BottomNavigationBarTheme`]
/// object using [`BottomNavigationBarTheme::of`]. When a widget uses
/// [`BottomNavigationBarTheme::of`], it is automatically rebuilt if the theme
/// later changes.
///
/// A bottom navigation theme can be specified as part of the overall Material
/// theme using `ThemeData::bottom_navigation_bar_theme`.
///
/// See also:
///
///  * [`BottomNavigationBarThemeData`], which describes the actual configuration
///    of a bottom navigation bar theme.
#[derive(Debug)]
pub struct BottomNavigationBarTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `BottomNavigationBar` widgets.
    pub data: BottomNavigationBarThemeData,
    pub child: WidgetRef,
}

impl BottomNavigationBarTheme {
    /// Constructs a bottom navigation bar theme that configures all descendant
    /// `BottomNavigationBar` widgets.
    pub fn new<K>(
        data: BottomNavigationBarThemeData,
        child: impl IntoWidget<K>,
    ) -> BottomNavigationBarTheme {
        BottomNavigationBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `BottomNavigationBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> BottomNavigationBarTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest
    /// [`BottomNavigationBarTheme`] ancestor. If there is no ancestor, it returns
    /// `ThemeData::bottom_navigation_bar_theme`. Applications can assume that the
    /// returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = BottomNavigationBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> BottomNavigationBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<BottomNavigationBarTheme>(app) {
            Some(bottom_nav_theme) => bottom_nav_theme.data.clone(),
            None => Theme::of(app, context).bottom_navigation_bar_theme.clone(),
        }
    }
}

impl InheritedWidget for BottomNavigationBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &BottomNavigationBarTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/bottom_navigation_bar_theme_test.dart`.

    use waft_embedder::{Color, Offset};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        FadeTransition, Opacity, SizedBox, Visibility, WidgetState, WidgetStateMouseCursor,
        WidgetStateProperty,
    };

    use super::*;
    use crate::bottom_navigation_bar::BottomNavigationBar;
    use crate::bottom_navigation_bar::tests::{
        ac_alarm, color_of, descendants_of_bar, diagonal3, first_text, icon_style, label_transform,
        material_of, paragraph_style, pump_bar,
    };
    use crate::icons::Icons;
    use crate::test_support::{WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::tester;

    #[test]
    fn bottom_navigation_bar_theme_data_copy_with_equality_basics() {
        assert_eq!(
            BottomNavigationBarThemeData::new(),
            BottomNavigationBarThemeData::new().copy_with()
        );
    }

    #[test]
    fn bottom_navigation_bar_theme_data_lerp_special_cases() {
        let data = BottomNavigationBarThemeData::new();
        assert_eq!(
            BottomNavigationBarThemeData::lerp(Some(&data), Some(&data), 0.5),
            data
        );
    }

    fn assert_all_none(theme_data: &BottomNavigationBarThemeData) {
        assert_eq!(theme_data.background_color, None);
        assert_eq!(theme_data.elevation, None);
        assert_eq!(theme_data.selected_icon_theme, None);
        assert_eq!(theme_data.unselected_icon_theme, None);
        assert_eq!(theme_data.selected_item_color, None);
        assert_eq!(theme_data.unselected_item_color, None);
        assert_eq!(theme_data.selected_label_style, None);
        assert_eq!(theme_data.unselected_label_style, None);
        assert_eq!(theme_data.show_selected_labels, None);
        assert_eq!(theme_data.show_unselected_labels, None);
        assert_eq!(theme_data.r#type, None);
        assert_eq!(theme_data.landscape_layout, None);
        assert!(theme_data.mouse_cursor.is_none());
    }

    #[test]
    fn bottom_navigation_bar_theme_data_defaults() {
        let theme_data = BottomNavigationBarThemeData::new();
        assert_all_none(&theme_data);

        let theme =
            BottomNavigationBarTheme::new(BottomNavigationBarThemeData::new(), SizedBox::new());
        assert_all_none(&theme.data);
    }

    /// Dart's `WidgetStateMouseCursor` given to a theme's `mouseCursor`.
    fn as_theme_cursor(
        cursor: WidgetStateMouseCursor,
    ) -> waft_widgets::WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
            Some(cursor.resolve(states))
        })
    }

    fn transform_center(tester: &WidgetTester, text: &str) -> Offset {
        let transform =
            tester.ancestor_of_type::<waft_widgets::Transform>(first_text(tester, text));
        tester.global_rect(tester.render_box_of(transform)).center()
    }

    #[test]
    fn bottom_navigation_bar_is_themeable() {
        with_main_thread_stack(|| {
            let background_color = Color::new(0xFF000001);
            let selected_item_color = Color::new(0xFF000002);
            let unselected_item_color = Color::new(0xFF000003);
            let selected_icon_theme = IconThemeData::new().size(10.0);
            let unselected_icon_theme = IconThemeData::new().size(11.0);
            let selected_text_style = TextStyle::new().font_size(22.0);
            let unselected_text_style = TextStyle::new().font_size(21.0);
            let elevation = 9.0;

            let mut tester = tester();
            let theme = BottomNavigationBarThemeData::new()
                .background_color(background_color)
                .selected_item_color(selected_item_color)
                .unselected_item_color(unselected_item_color)
                .selected_icon_theme(selected_icon_theme.clone())
                .unselected_icon_theme(unselected_icon_theme.clone())
                .elevation(elevation)
                .show_unselected_labels(true)
                .show_selected_labels(true)
                .r#type(BottomNavigationBarType::Fixed)
                .selected_label_style(selected_text_style.clone())
                .unselected_label_style(unselected_text_style.clone())
                .mouse_cursor(
                    <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
                        if states.contains(&WidgetState::Selected) {
                            return Some(SystemMouseCursors::GRAB.into());
                        }
                        Some(SystemMouseCursors::MOVE.into())
                    }),
                );
            pump_bar(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .bottom_navigation_bar_theme(theme)
                        .build(app)
                },
                || BottomNavigationBar::new(ac_alarm()),
            );

            let selected_font_style = paragraph_style(&tester, "AC");
            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());
            assert_eq!(selected_font_style.font_size, selected_font_style.font_size);
            // Unselected label has a font size of 22 but is scaled down to be font size 21.
            assert_eq!(
                label_transform(&tester, "Alarm"),
                diagonal3(
                    unselected_text_style.font_size.unwrap()
                        / selected_text_style.font_size.unwrap()
                )
            );
            assert_eq!(color_of(&selected_icon), Some(selected_item_color));
            assert_eq!(selected_icon.font_size, selected_icon_theme.size);
            assert_eq!(color_of(&unselected_icon), Some(unselected_item_color));
            assert_eq!(unselected_icon.font_size, unselected_icon_theme.size);
            // There should not be any [Opacity] or [FadeTransition] widgets
            // since showUnselectedLabels and showSelectedLabels are true.
            assert_eq!(descendants_of_bar::<Opacity>(&tester), 0);
            assert_eq!(descendants_of_bar::<FadeTransition>(&tester), 0);
            assert_eq!(
                material_of(&tester, |material| material.elevation),
                elevation
            );
            assert_eq!(
                material_of(&tester, |material| material.color),
                Some(background_color)
            );

            let selected_bar_item = transform_center(&tester, "AC");
            let unselected_bar_item = transform_center(&tester, "Alarm");
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, selected_bar_item, false);
            tester.pump_and_settle();
            let grab: MouseCursorRef = SystemMouseCursors::GRAB.into();
            assert!(*tester.active_cursor(1) == *grab);
            tester.move_to(&mut gesture, unselected_bar_item, false);
            tester.pump_and_settle();
            let move_cursor: MouseCursorRef = SystemMouseCursors::MOVE.into();
            assert!(*tester.active_cursor(1) == *move_cursor);
        });
    }

    #[test]
    fn bottom_navigation_bar_properties_are_taken_over_the_theme_values() {
        with_main_thread_stack(|| {
            let theme = BottomNavigationBarThemeData::new()
                .background_color(Color::new(0xFF000001))
                .selected_item_color(Color::new(0xFF000002))
                .unselected_item_color(Color::new(0xFF000003))
                .selected_icon_theme(IconThemeData::new().size(10.0))
                .unselected_icon_theme(IconThemeData::new().size(11.0))
                .elevation(9.0)
                .show_unselected_labels(false)
                .show_selected_labels(false)
                .r#type(BottomNavigationBarType::Shifting)
                .selected_label_style(TextStyle::new().font_size(22.0))
                .unselected_label_style(TextStyle::new().font_size(21.0))
                .landscape_layout(BottomNavigationBarLandscapeLayout::Centered)
                .mouse_cursor(as_theme_cursor(WidgetStateMouseCursor::clickable()));

            let background_color = Color::new(0xFF000004);
            let selected_item_color = Color::new(0xFF000005);
            let unselected_item_color = Color::new(0xFF000006);
            let selected_icon_theme = IconThemeData::new().size(15.0);
            let unselected_icon_theme = IconThemeData::new().size(16.0);
            let selected_text_style = TextStyle::new().font_size(25.0);
            let unselected_text_style = TextStyle::new().font_size(26.0);
            let elevation = 7.0;
            let landscape_layout = BottomNavigationBarLandscapeLayout::Spread;
            let cursor = WidgetStateMouseCursor::textable();

            let mut tester = tester();
            let (selected_icon_given, unselected_icon_given) =
                (selected_icon_theme.clone(), unselected_icon_theme.clone());
            let (selected_text_given, unselected_text_given) =
                (selected_text_style.clone(), unselected_text_style.clone());
            pump_bar(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .bottom_navigation_bar_theme(theme)
                        .build(app)
                },
                move || {
                    BottomNavigationBar::new(ac_alarm())
                        .background_color(background_color)
                        .selected_item_color(selected_item_color)
                        .unselected_item_color(unselected_item_color)
                        .selected_icon_theme(selected_icon_given)
                        .unselected_icon_theme(unselected_icon_given)
                        .elevation(elevation)
                        .show_unselected_labels(true)
                        .show_selected_labels(true)
                        .r#type(BottomNavigationBarType::Fixed)
                        .selected_label_style(selected_text_given)
                        .unselected_label_style(unselected_text_given)
                        .landscape_layout(landscape_layout)
                        .mouse_cursor(cursor)
                },
            );

            let selected_font_style = paragraph_style(&tester, "AC");
            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());
            assert_eq!(selected_font_style.font_size, selected_font_style.font_size);
            // Unselected label has a font size of 22 but is scaled down to be font size 21.
            assert_eq!(
                label_transform(&tester, "Alarm"),
                diagonal3(
                    unselected_text_style.font_size.unwrap()
                        / selected_text_style.font_size.unwrap()
                )
            );
            assert_eq!(color_of(&selected_icon), Some(selected_item_color));
            assert_eq!(selected_icon.font_size, selected_icon_theme.size);
            assert_eq!(color_of(&unselected_icon), Some(unselected_item_color));
            assert_eq!(unselected_icon.font_size, unselected_icon_theme.size);
            // There should not be any [Opacity] or [FadeTransition] widgets
            // since showUnselectedLabels and showSelectedLabels are true.
            assert_eq!(descendants_of_bar::<Opacity>(&tester), 0);
            assert_eq!(descendants_of_bar::<FadeTransition>(&tester), 0);
            assert_eq!(
                material_of(&tester, |material| material.elevation),
                elevation
            );
            assert_eq!(
                material_of(&tester, |material| material.color),
                Some(background_color)
            );

            let bar_item = transform_center(&tester, "AC");
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, bar_item, false);
            tester.pump_and_settle();
            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            assert!(*tester.active_cursor(1) == *text);
        });
    }

    fn pump_with_labels(tester: &mut WidgetTester, show_selected: bool, show_unselected: bool) {
        pump_bar(
            tester,
            move |app| {
                ThemeData::builder()
                    .bottom_navigation_bar_theme(
                        BottomNavigationBarThemeData::new()
                            .show_selected_labels(show_selected)
                            .show_unselected_labels(show_unselected),
                    )
                    .build(app)
            },
            || BottomNavigationBar::new(ac_alarm()),
        );
    }

    fn bar_descendants<W: 'static>(tester: &WidgetTester) -> Vec<waft_widgets::AnyElement> {
        let bar = tester.find_by_type::<BottomNavigationBar>()[0];
        tester.descendants_of_type::<W>(bar)
    }

    fn fade_opacity(tester: &WidgetTester, element: waft_widgets::AnyElement) -> f64 {
        let opacity = tester.widget(element, |fade: &FadeTransition| fade.opacity);
        opacity.value(&tester.cell.borrow())
    }

    #[test]
    fn bottom_navigation_bar_theme_can_be_used_to_hide_all_labels() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/66738.
            let mut tester = tester();
            pump_with_labels(&mut tester, false, false);

            let find_visibility = bar_descendants::<Visibility>(&tester);
            assert_eq!(find_visibility.len(), 2);
            for element in find_visibility {
                assert!(!tester.widget(element, |visibility: &Visibility| visibility.visible));
            }
        });
    }

    #[test]
    fn bottom_navigation_bar_theme_can_be_used_to_hide_selected_labels() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/66738.
            let mut tester = tester();
            pump_with_labels(&mut tester, false, true);

            let find_fade_transition = bar_descendants::<FadeTransition>(&tester);
            assert_eq!(find_fade_transition.len(), 2);
            assert_eq!(fade_opacity(&tester, find_fade_transition[0]), 0.0);
            assert_eq!(fade_opacity(&tester, find_fade_transition[1]), 1.0);
        });
    }

    #[test]
    fn bottom_navigation_bar_theme_can_be_used_to_hide_unselected_labels() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_labels(&mut tester, true, false);

            let find_fade_transition = bar_descendants::<FadeTransition>(&tester);
            assert_eq!(find_fade_transition.len(), 2);
            assert_eq!(fade_opacity(&tester, find_fade_transition[0]), 1.0);
            assert_eq!(fade_opacity(&tester, find_fade_transition[1]), 0.0);
        });
    }

    #[test]
    fn theme_data_holds_the_bottom_navigation_bar_theme() {
        with_main_thread_stack(|| {
            let cell = crate::test_support::test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(
                theme.bottom_navigation_bar_theme,
                BottomNavigationBarThemeData::new()
            );

            let raised = ThemeData::builder()
                .bottom_navigation_bar_theme(BottomNavigationBarThemeData::new().elevation(10.0))
                .build(&app);
            assert_eq!(raised.bottom_navigation_bar_theme.elevation, Some(10.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .bottom_navigation_bar_theme(raised.bottom_navigation_bar_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .bottom_navigation_bar_theme(BottomNavigationBarThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5)
                    .bottom_navigation_bar_theme
                    .elevation,
                Some(5.0)
            );
        });
    }

    #[test]
    fn bottom_navigation_bar_theme_data_lerps_each_field() {
        let a = BottomNavigationBarThemeData::new()
            .background_color(Color::new(0xFF000000))
            .elevation(2.0)
            .show_selected_labels(true)
            .r#type(BottomNavigationBarType::Fixed);
        let b = BottomNavigationBarThemeData::new()
            .background_color(Color::new(0xFFFFFFFF))
            .elevation(6.0)
            .show_selected_labels(false)
            .r#type(BottomNavigationBarType::Shifting)
            .selected_label_style(TextStyle::new().font_size(20.0));

        let quarter = BottomNavigationBarThemeData::lerp(Some(&a), Some(&b), 0.25);
        assert_eq!(quarter.elevation, Some(3.0));
        assert_eq!(quarter.show_selected_labels, Some(true));
        assert_eq!(quarter.r#type, Some(BottomNavigationBarType::Fixed));
        let halfway = BottomNavigationBarThemeData::lerp(Some(&a), Some(&b), 0.5);
        assert_eq!(
            halfway.background_color,
            Color::lerp(
                Some(Color::new(0xFF000000)),
                Some(Color::new(0xFFFFFFFF)),
                0.5
            )
        );
        assert_eq!(halfway.show_selected_labels, Some(false));
        assert_eq!(halfway.r#type, Some(BottomNavigationBarType::Shifting));
        assert_eq!(
            halfway.selected_label_style,
            TextStyle::lerp(None, Some(&TextStyle::new().font_size(20.0)), 0.5)
        );
        assert_eq!(
            halfway.selected_icon_theme,
            Some(IconThemeData::lerp(None, None, 0.5))
        );
    }
}
