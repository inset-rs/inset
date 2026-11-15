//! Flutter counterpart: material_ui's `floating_action_button_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_rendering::BoxConstraints;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `FloatingActionButton`
/// widgets.
///
/// Descendant widgets obtain the current [`FloatingActionButtonThemeData`] object
/// using [`FloatingActionButtonTheme::of`]. Instances of
/// [`FloatingActionButtonThemeData`] can be customized with
/// [`FloatingActionButtonThemeData::copy_with`].
///
/// Typically a [`FloatingActionButtonThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::floating_action_button_theme`.
///
/// All [`FloatingActionButtonThemeData`] properties are `null` by default.
/// When null, the `FloatingActionButton` provides its own defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct FloatingActionButtonThemeData {
    /// Color to be used for the unselected, enabled `FloatingActionButton`'s
    /// foreground.
    pub foreground_color: Option<Color>,

    /// Color to be used for the unselected, enabled `FloatingActionButton`'s
    /// background.
    pub background_color: Option<Color>,

    /// The color to use for filling the button when the button has input focus.
    pub focus_color: Option<Color>,

    /// The color to use for filling the button when the button has a pointer
    /// hovering over it.
    pub hover_color: Option<Color>,

    /// The splash color for this `FloatingActionButton`'s `InkWell`.
    pub splash_color: Option<Color>,

    /// The z-coordinate to be used for the unselected, enabled
    /// `FloatingActionButton`'s elevation foreground.
    pub elevation: Option<f64>,

    /// The z-coordinate at which to place this button relative to its parent when
    /// the button has the input focus.
    ///
    /// This controls the size of the shadow below the floating action button.
    pub focus_elevation: Option<f64>,

    /// The z-coordinate at which to place this button relative to its parent when
    /// the button is enabled and has a pointer hovering over it.
    ///
    /// This controls the size of the shadow below the floating action button.
    pub hover_elevation: Option<f64>,

    /// The z-coordinate to be used for the disabled `FloatingActionButton`'s
    /// elevation foreground.
    pub disabled_elevation: Option<f64>,

    /// The z-coordinate to be used for the selected, enabled
    /// `FloatingActionButton`'s elevation foreground.
    pub highlight_elevation: Option<f64>,

    /// The shape to be used for the floating action button's `Material`.
    pub shape: Option<ShapeBorderRef>,

    /// If specified, defines the feedback property for `FloatingActionButton`.
    ///
    /// If `FloatingActionButton::enable_feedback` is provided, [`enable_feedback`](Self::enable_feedback) is
    /// ignored.
    pub enable_feedback: Option<bool>,

    /// Overrides the default icon size for the `FloatingActionButton`;
    pub icon_size: Option<f64>,

    /// Overrides the default size constraints for the `FloatingActionButton`.
    pub size_constraints: Option<BoxConstraints>,

    /// Overrides the default size constraints for `FloatingActionButton::small`.
    pub small_size_constraints: Option<BoxConstraints>,

    /// Overrides the default size constraints for `FloatingActionButton::large`.
    pub large_size_constraints: Option<BoxConstraints>,

    /// Overrides the default size constraints for `FloatingActionButton::extended`.
    pub extended_size_constraints: Option<BoxConstraints>,

    /// The spacing between the icon and the label for an extended
    /// `FloatingActionButton`.
    pub extended_icon_label_spacing: Option<f64>,

    /// The padding for an extended `FloatingActionButton`'s content.
    pub extended_padding: Option<EdgeInsetsGeometry>,

    /// The text style for an extended `FloatingActionButton`'s label.
    pub extended_text_style: Option<TextStyle>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// button.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Pressed`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    ///
    /// If specified, overrides the default value of `FloatingActionButton::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `FloatingActionButtonThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait FloatingActionButtonThemeDataGetters for FloatingActionButtonThemeData, fields: floating_action_button_theme_data {
        foreground_color: Option<Color>,
        background_color: Option<Color>,
        focus_color: Option<Color>,
        hover_color: Option<Color>,
        splash_color: Option<Color>,
        elevation: Option<f64>,
        focus_elevation: Option<f64>,
        hover_elevation: Option<f64>,
        disabled_elevation: Option<f64>,
        highlight_elevation: Option<f64>,
        shape: Option<ShapeBorderRef>,
        enable_feedback: Option<bool>,
        icon_size: Option<f64>,
        size_constraints: Option<BoxConstraints>,
        small_size_constraints: Option<BoxConstraints>,
        large_size_constraints: Option<BoxConstraints>,
        extended_size_constraints: Option<BoxConstraints>,
        extended_icon_label_spacing: Option<f64>,
        extended_padding: Option<EdgeInsetsGeometry>,
        extended_text_style: Option<TextStyle>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
    }
}

impl FloatingActionButtonThemeData {
    /// Creates a theme that can be used for `ThemeData::floating_action_button_theme` and
    /// [`FloatingActionButtonTheme`]; Dart's optional named arguments are the fluent setters.
    pub fn new() -> FloatingActionButtonThemeData {
        FloatingActionButtonThemeData::default()
    }

    /// Dart `FloatingActionButtonThemeData(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> FloatingActionButtonThemeData {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `FloatingActionButtonThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> FloatingActionButtonThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `FloatingActionButtonThemeData(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> FloatingActionButtonThemeData {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `FloatingActionButtonThemeData(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> FloatingActionButtonThemeData {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `FloatingActionButtonThemeData(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> FloatingActionButtonThemeData {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `FloatingActionButtonThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> FloatingActionButtonThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `FloatingActionButtonThemeData(focusElevation:)`.
    pub fn focus_elevation(mut self, focus_elevation: f64) -> FloatingActionButtonThemeData {
        self.focus_elevation = Some(focus_elevation);
        self
    }

    /// Dart `FloatingActionButtonThemeData(hoverElevation:)`.
    pub fn hover_elevation(mut self, hover_elevation: f64) -> FloatingActionButtonThemeData {
        self.hover_elevation = Some(hover_elevation);
        self
    }

    /// Dart `FloatingActionButtonThemeData(disabledElevation:)`.
    pub fn disabled_elevation(mut self, disabled_elevation: f64) -> FloatingActionButtonThemeData {
        self.disabled_elevation = Some(disabled_elevation);
        self
    }

    /// Dart `FloatingActionButtonThemeData(highlightElevation:)`.
    pub fn highlight_elevation(
        mut self,
        highlight_elevation: f64,
    ) -> FloatingActionButtonThemeData {
        self.highlight_elevation = Some(highlight_elevation);
        self
    }

    /// Dart `FloatingActionButtonThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> FloatingActionButtonThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `FloatingActionButtonThemeData(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> FloatingActionButtonThemeData {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `FloatingActionButtonThemeData(iconSize:)`.
    pub fn icon_size(mut self, icon_size: f64) -> FloatingActionButtonThemeData {
        self.icon_size = Some(icon_size);
        self
    }

    /// Dart `FloatingActionButtonThemeData(sizeConstraints:)`.
    pub fn size_constraints(
        mut self,
        size_constraints: BoxConstraints,
    ) -> FloatingActionButtonThemeData {
        self.size_constraints = Some(size_constraints);
        self
    }

    /// Dart `FloatingActionButtonThemeData(smallSizeConstraints:)`.
    pub fn small_size_constraints(
        mut self,
        small_size_constraints: BoxConstraints,
    ) -> FloatingActionButtonThemeData {
        self.small_size_constraints = Some(small_size_constraints);
        self
    }

    /// Dart `FloatingActionButtonThemeData(largeSizeConstraints:)`.
    pub fn large_size_constraints(
        mut self,
        large_size_constraints: BoxConstraints,
    ) -> FloatingActionButtonThemeData {
        self.large_size_constraints = Some(large_size_constraints);
        self
    }

    /// Dart `FloatingActionButtonThemeData(extendedSizeConstraints:)`.
    pub fn extended_size_constraints(
        mut self,
        extended_size_constraints: BoxConstraints,
    ) -> FloatingActionButtonThemeData {
        self.extended_size_constraints = Some(extended_size_constraints);
        self
    }

    /// Dart `FloatingActionButtonThemeData(extendedIconLabelSpacing:)`.
    pub fn extended_icon_label_spacing(
        mut self,
        extended_icon_label_spacing: f64,
    ) -> FloatingActionButtonThemeData {
        self.extended_icon_label_spacing = Some(extended_icon_label_spacing);
        self
    }

    /// Dart `FloatingActionButtonThemeData(extendedPadding:)`.
    pub fn extended_padding(
        mut self,
        extended_padding: EdgeInsetsGeometry,
    ) -> FloatingActionButtonThemeData {
        self.extended_padding = Some(extended_padding);
        self
    }

    /// Dart `FloatingActionButtonThemeData(extendedTextStyle:)`.
    pub fn extended_text_style(
        mut self,
        extended_text_style: TextStyle,
    ) -> FloatingActionButtonThemeData {
        self.extended_text_style = Some(extended_text_style);
        self
    }

    /// Dart `FloatingActionButtonThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> FloatingActionButtonThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> FloatingActionButtonThemeData {
        self.clone()
    }

    /// Linearly interpolate between two floating action button themes.
    ///
    /// If both arguments are null, then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&FloatingActionButtonThemeData>,
        b: Option<&FloatingActionButtonThemeData>,
        t: f64,
    ) -> Option<FloatingActionButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(FloatingActionButtonThemeData {
            foreground_color: Color::lerp(
                a.and_then(|a| a.foreground_color),
                b.and_then(|b| b.foreground_color),
                t,
            ),
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            focus_color: Color::lerp(
                a.and_then(|a| a.focus_color),
                b.and_then(|b| b.focus_color),
                t,
            ),
            hover_color: Color::lerp(
                a.and_then(|a| a.hover_color),
                b.and_then(|b| b.hover_color),
                t,
            ),
            splash_color: Color::lerp(
                a.and_then(|a| a.splash_color),
                b.and_then(|b| b.splash_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            focus_elevation: lerp_double(
                a.and_then(|a| a.focus_elevation),
                b.and_then(|b| b.focus_elevation),
                t,
            ),
            hover_elevation: lerp_double(
                a.and_then(|a| a.hover_elevation),
                b.and_then(|b| b.hover_elevation),
                t,
            ),
            disabled_elevation: lerp_double(
                a.and_then(|a| a.disabled_elevation),
                b.and_then(|b| b.disabled_elevation),
                t,
            ),
            highlight_elevation: lerp_double(
                a.and_then(|a| a.highlight_elevation),
                b.and_then(|b| b.highlight_elevation),
                t,
            ),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            enable_feedback: if t < 0.5 {
                a.and_then(|a| a.enable_feedback)
            } else {
                b.and_then(|b| b.enable_feedback)
            },
            icon_size: lerp_double(a.and_then(|a| a.icon_size), b.and_then(|b| b.icon_size), t),
            size_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.size_constraints),
                b.and_then(|b| b.size_constraints),
                t,
            ),
            small_size_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.small_size_constraints),
                b.and_then(|b| b.small_size_constraints),
                t,
            ),
            large_size_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.large_size_constraints),
                b.and_then(|b| b.large_size_constraints),
                t,
            ),
            extended_size_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.extended_size_constraints),
                b.and_then(|b| b.extended_size_constraints),
                t,
            ),
            extended_icon_label_spacing: lerp_double(
                a.and_then(|a| a.extended_icon_label_spacing),
                b.and_then(|b| b.extended_icon_label_spacing),
                t,
            ),
            extended_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.extended_padding),
                b.and_then(|b| b.extended_padding),
                t,
            ),
            extended_text_style: TextStyle::lerp(
                a.and_then(|a| a.extended_text_style.as_ref()),
                b.and_then(|b| b.extended_text_style.as_ref()),
                t,
            ),
            mouse_cursor: if t < 0.5 {
                a.and_then(|a| a.mouse_cursor.clone())
            } else {
                b.and_then(|b| b.mouse_cursor.clone())
            },
        })
    }
}

impl PartialEq for FloatingActionButtonThemeData {
    fn eq(&self, other: &FloatingActionButtonThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.foreground_color == self.foreground_color
            && other.background_color == self.background_color
            && other.focus_color == self.focus_color
            && other.hover_color == self.hover_color
            && other.splash_color == self.splash_color
            && other.elevation == self.elevation
            && other.focus_elevation == self.focus_elevation
            && other.hover_elevation == self.hover_elevation
            && other.disabled_elevation == self.disabled_elevation
            && other.highlight_elevation == self.highlight_elevation
            && other.shape == self.shape
            && other.enable_feedback == self.enable_feedback
            && other.icon_size == self.icon_size
            && other.size_constraints == self.size_constraints
            && other.small_size_constraints == self.small_size_constraints
            && other.large_size_constraints == self.large_size_constraints
            && other.extended_size_constraints == self.extended_size_constraints
            && other.extended_icon_label_spacing == self.extended_icon_label_spacing
            && other.extended_padding == self.extended_padding
            && other.extended_text_style == self.extended_text_style
            && other.mouse_cursor == self.mouse_cursor
    }
}

/// An inherited widget that defines the configuration for
/// `FloatingActionButton`s in this widget's subtree.
///
/// Values specified here are used for `FloatingActionButton` properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct FloatingActionButtonTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `FloatingActionButton` widgets.
    pub data: FloatingActionButtonThemeData,
    pub child: WidgetRef,
}

impl FloatingActionButtonTheme {
    /// Creates a floating action button theme that controls the configurations for
    /// `FloatingActionButton`s in its widget subtree.
    pub fn new<K>(
        data: FloatingActionButtonThemeData,
        child: impl IntoWidget<K>,
    ) -> FloatingActionButtonTheme {
        FloatingActionButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `FloatingActionButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> FloatingActionButtonTheme {
        self.key = Some(key);
        self
    }

    /// The closest instance of this class's [`data`](Self::data) value that encloses the given
    /// context.
    ///
    /// If there is no ancestor, it returns `ThemeData::floating_action_button_theme`. Applications
    /// can assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = FloatingActionButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> FloatingActionButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<FloatingActionButtonTheme>(app) {
            Some(fab_theme) => fab_theme.data.clone(),
            None => Theme::of(app, context).floating_action_button_theme.clone(),
        }
    }
}

impl InheritedTheme for FloatingActionButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        FloatingActionButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for FloatingActionButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &FloatingActionButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use waft_painting::ShapeBorder;

    use waft_embedder::Size;
    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{SizedBox, WidgetStatePropertyAll};

    use super::*;
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    fn cursor(cursor: impl Into<MouseCursorRef>) -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        Rc::new(WidgetStatePropertyAll::new(Some(cursor.into())))
    }

    #[test]
    fn floating_action_button_theme_data_copy_with_equality_basics() {
        assert_eq!(
            FloatingActionButtonThemeData::new(),
            FloatingActionButtonThemeData::new().copy_with()
        );

        let theme = FloatingActionButtonThemeData::new()
            .elevation(4.0)
            .shape(rounded(8.0))
            .mouse_cursor(cursor(SystemMouseCursors::FORBIDDEN));
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(theme.copy_with().icon_size(30.0).icon_size, Some(30.0));
        assert_eq!(theme.copy_with().icon_size(30.0).elevation, Some(4.0));
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(theme.copy_with().shape(rounded(9.0)), theme);
    }

    #[test]
    fn floating_action_button_theme_data_equality_compares_the_mouse_cursor_by_value() {
        let forbidden = cursor(SystemMouseCursors::FORBIDDEN);
        let with = |mouse_cursor| FloatingActionButtonThemeData::new().mouse_cursor(mouse_cursor);

        assert_eq!(with(Rc::clone(&forbidden)), with(Rc::clone(&forbidden)));
        assert_eq!(
            with(Rc::clone(&forbidden)),
            with(cursor(SystemMouseCursors::FORBIDDEN))
        );
        assert_ne!(
            with(Rc::clone(&forbidden)),
            with(cursor(SystemMouseCursors::CLICK))
        );
        assert_ne!(with(forbidden), FloatingActionButtonThemeData::new());
    }

    #[test]
    fn floating_action_button_theme_data_lerp_special_cases() {
        assert_eq!(FloatingActionButtonThemeData::lerp(None, None, 0.0), None);
        let data = FloatingActionButtonThemeData::new();
        assert_eq!(
            FloatingActionButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn floating_action_button_theme_data_null_fields_by_default() {
        let theme = FloatingActionButtonThemeData::new();
        assert_eq!(theme.foreground_color, None);
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.focus_color, None);
        assert_eq!(theme.hover_color, None);
        assert_eq!(theme.splash_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.focus_elevation, None);
        assert_eq!(theme.hover_elevation, None);
        assert_eq!(theme.disabled_elevation, None);
        assert_eq!(theme.highlight_elevation, None);
        assert!(theme.shape.is_none());
        assert_eq!(theme.enable_feedback, None);
        assert_eq!(theme.icon_size, None);
        assert_eq!(theme.size_constraints, None);
        assert_eq!(theme.small_size_constraints, None);
        assert_eq!(theme.large_size_constraints, None);
        assert_eq!(theme.extended_size_constraints, None);
        assert_eq!(theme.extended_icon_label_spacing, None);
        assert_eq!(theme.extended_padding, None);
        assert_eq!(theme.extended_text_style, None);
        assert!(theme.mouse_cursor.is_none());
    }

    #[test]
    fn floating_action_button_theme_data_lerps_each_field() {
        let forbidden = cursor(SystemMouseCursors::FORBIDDEN);
        let click = cursor(SystemMouseCursors::CLICK);
        let a = FloatingActionButtonThemeData::new()
            .foreground_color(Colors::BLACK)
            .elevation(2.0)
            .highlight_elevation(10.0)
            .shape(rounded(4.0))
            .enable_feedback(true)
            .icon_size(20.0)
            .size_constraints(BoxConstraints::tight(Size::new(100.0, 100.0)))
            .extended_icon_label_spacing(8.0)
            .extended_padding(EdgeInsetsGeometry::all(10.0))
            .extended_text_style(TextStyle::new().font_size(10.0))
            .mouse_cursor(Rc::clone(&forbidden));
        let b = FloatingActionButtonThemeData::new()
            .foreground_color(Colors::WHITE)
            .splash_color(Colors::RED.primary())
            .elevation(6.0)
            .highlight_elevation(20.0)
            .shape(rounded(8.0))
            .enable_feedback(false)
            .icon_size(40.0)
            .size_constraints(BoxConstraints::tight(Size::new(300.0, 300.0)))
            .small_size_constraints(BoxConstraints::tight(Size::new(30.0, 30.0)))
            .extended_icon_label_spacing(16.0)
            .extended_padding(EdgeInsetsGeometry::all(30.0))
            .extended_text_style(TextStyle::new().font_size(20.0))
            .mouse_cursor(Rc::clone(&click));

        let halfway =
            FloatingActionButtonThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");

        assert_eq!(
            halfway.foreground_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.splash_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.highlight_elevation, Some(15.0));
        assert_eq!(
            halfway.shape.as_deref(),
            Some(&rounded(6.0) as &dyn ShapeBorder)
        );
        assert_eq!(halfway.enable_feedback, Some(false));
        assert_eq!(halfway.icon_size, Some(30.0));
        assert_eq!(
            halfway.size_constraints,
            Some(BoxConstraints::tight(Size::new(200.0, 200.0)))
        );
        assert_eq!(
            halfway.small_size_constraints,
            BoxConstraints::lerp(
                None,
                Some(BoxConstraints::tight(Size::new(30.0, 30.0))),
                0.5
            )
        );
        assert_eq!(halfway.extended_icon_label_spacing, Some(12.0));
        assert_eq!(
            halfway.extended_padding,
            Some(EdgeInsetsGeometry::all(20.0))
        );
        assert_eq!(
            halfway
                .extended_text_style
                .and_then(|style| style.font_size),
            Some(15.0)
        );
        assert!(Rc::ptr_eq(
            halfway.mouse_cursor.as_ref().expect("a cursor"),
            &click
        ));

        let early =
            FloatingActionButtonThemeData::lerp(Some(&a), Some(&b), 0.25).expect("both given");
        assert_eq!(early.enable_feedback, Some(true));
        assert!(Rc::ptr_eq(
            early.mouse_cursor.as_ref().expect("a cursor"),
            &forbidden
        ));
    }

    #[test]
    fn of_returns_the_theme_data_floating_action_button_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = FloatingActionButtonThemeData::new()
                .elevation(3.0)
                .background_color(Colors::RED.primary());
            let theme_data_fab = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .floating_action_button_theme(theme_data_fab.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                FloatingActionButtonTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_floating_action_button_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_fab = FloatingActionButtonThemeData::new().elevation(7.0);
            let expected = widget_fab.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .floating_action_button_theme(
                            FloatingActionButtonThemeData::new().elevation(3.0),
                        )
                        .build(app);
                    Theme::new(
                        data,
                        FloatingActionButtonTheme::new(widget_fab.clone(), child),
                    )
                    .into_widget()
                },
                FloatingActionButtonTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_floating_action_button_theme_is_kept_under_another_one() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let captured = FloatingActionButtonThemeData::new().elevation(1.0);
            let shadowing = FloatingActionButtonThemeData::new().elevation(2.0);

            let seen = theme_seen_through_capture(
                &cell,
                |probe| FloatingActionButtonTheme::new(captured.clone(), probe).into_widget(),
                |wrapped| FloatingActionButtonTheme::new(shadowing.clone(), wrapped).into_widget(),
                FloatingActionButtonTheme::of,
            );

            assert_eq!(seen.elevation, Some(1.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: FloatingActionButtonThemeData| {
            FloatingActionButtonTheme::new(data, SizedBox::shrink())
        };
        let old = theme(FloatingActionButtonThemeData::new().elevation(1.0));

        assert!(
            !theme(FloatingActionButtonThemeData::new().elevation(1.0)).update_should_notify(&old)
        );
        assert!(
            theme(FloatingActionButtonThemeData::new().elevation(2.0)).update_should_notify(&old)
        );
    }

    #[test]
    fn theme_data_holds_the_floating_action_button_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(
                theme.floating_action_button_theme,
                FloatingActionButtonThemeData::new()
            );

            let raised = ThemeData::builder()
                .floating_action_button_theme(FloatingActionButtonThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.floating_action_button_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .floating_action_button_theme(raised.floating_action_button_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .floating_action_button_theme(FloatingActionButtonThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5)
                    .floating_action_button_theme
                    .elevation,
                Some(4.0)
            );
        });
    }
}
