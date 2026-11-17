//! Flutter counterpart: material_ui's `button_bar_theme.dart`.
#![allow(
    deprecated,
    reason = "Dart deprecates `ButtonBarThemeData` and `ButtonBarTheme`, which this file implements"
)]

use waft_embedder::lerp_double;
use waft_foundation::App;
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, MainAxisAlignment, MainAxisSize,
    VerticalDirection, WidgetRef,
};

use crate::button_theme::{ButtonBarLayoutBehavior, ButtonTextTheme};
use crate::theme::Theme;

/// Defines the visual properties of `ButtonBar` widgets.
///
/// Used by [`ButtonBarTheme`] to control the visual properties of `ButtonBar` instances in a
/// widget subtree.
///
/// See also:
///
///  * [`ButtonBarTheme`], an `InheritedWidget` that propagates the theme down its subtree.
///  * `ButtonBar`, which uses this to configure itself and its children button widgets.
#[deprecated(note = "Use OverflowBar instead. This feature was deprecated after v3.21.0-10.0.pre.")]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ButtonBarThemeData {
    /// How the children should be placed along the horizontal axis.
    pub alignment: Option<MainAxisAlignment>,
    /// How much horizontal space is available. See `Row::main_axis_size`.
    pub main_axis_size: Option<MainAxisSize>,
    /// Defines a `ButtonBar` button's base colors, and the defaults for the button's minimum
    /// size, internal padding, and shape.
    pub button_text_theme: Option<ButtonTextTheme>,
    /// The minimum width for `ButtonBar` buttons.
    pub button_min_width: Option<f64>,
    /// The minimum height for `ButtonBar` buttons.
    pub button_height: Option<f64>,
    /// Padding for a `ButtonBar` button's child (typically the button's label).
    pub button_padding: Option<EdgeInsetsGeometry>,
    /// If true, then a `DropdownButton` menu's width will match the `ButtonBar` button's
    /// width.
    pub button_aligned_dropdown: Option<bool>,
    /// Defines whether a `ButtonBar` should size itself with a minimum size constraint or with
    /// padding.
    pub layout_behavior: Option<ButtonBarLayoutBehavior>,
    /// Defines the vertical direction of a `ButtonBar`'s children if it overflows.
    pub overflow_direction: Option<VerticalDirection>,
}

impl ButtonBarThemeData {
    /// Creates the set of properties used to configure `ButtonBar` widgets; Dart's optional
    /// named arguments are the `with_*` setters.
    pub fn new() -> ButtonBarThemeData {
        ButtonBarThemeData::default()
    }

    /// Dart `ButtonBarThemeData(alignment:)`.
    pub fn with_alignment(mut self, alignment: MainAxisAlignment) -> ButtonBarThemeData {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `ButtonBarThemeData(mainAxisSize:)`.
    pub fn with_main_axis_size(mut self, main_axis_size: MainAxisSize) -> ButtonBarThemeData {
        self.main_axis_size = Some(main_axis_size);
        self
    }

    /// Dart `ButtonBarThemeData(buttonTextTheme:)`.
    pub fn with_button_text_theme(
        mut self,
        button_text_theme: ButtonTextTheme,
    ) -> ButtonBarThemeData {
        self.button_text_theme = Some(button_text_theme);
        self
    }

    /// Dart `ButtonBarThemeData(buttonMinWidth:)`.
    pub fn with_button_min_width(mut self, button_min_width: f64) -> ButtonBarThemeData {
        debug_assert!(button_min_width >= 0.0);
        self.button_min_width = Some(button_min_width);
        self
    }

    /// Dart `ButtonBarThemeData(buttonHeight:)`.
    pub fn with_button_height(mut self, button_height: f64) -> ButtonBarThemeData {
        debug_assert!(button_height >= 0.0);
        self.button_height = Some(button_height);
        self
    }

    /// Dart `ButtonBarThemeData(buttonPadding:)`.
    pub fn with_button_padding(
        mut self,
        button_padding: impl Into<EdgeInsetsGeometry>,
    ) -> ButtonBarThemeData {
        self.button_padding = Some(button_padding.into());
        self
    }

    /// Dart `ButtonBarThemeData(buttonAlignedDropdown:)`.
    pub fn with_button_aligned_dropdown(
        mut self,
        button_aligned_dropdown: bool,
    ) -> ButtonBarThemeData {
        self.button_aligned_dropdown = Some(button_aligned_dropdown);
        self
    }

    /// Dart `ButtonBarThemeData(layoutBehavior:)`.
    pub fn with_layout_behavior(
        mut self,
        layout_behavior: ButtonBarLayoutBehavior,
    ) -> ButtonBarThemeData {
        self.layout_behavior = Some(layout_behavior);
        self
    }

    /// Dart `ButtonBarThemeData(overflowDirection:)`.
    pub fn with_overflow_direction(
        mut self,
        overflow_direction: VerticalDirection,
    ) -> ButtonBarThemeData {
        self.overflow_direction = Some(overflow_direction);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new values.
    pub fn copy_with(&self) -> ButtonBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two button bar themes.
    ///
    /// If both arguments are null, then null is returned.
    pub fn lerp(
        a: Option<&ButtonBarThemeData>,
        b: Option<&ButtonBarThemeData>,
        t: f64,
    ) -> Option<ButtonBarThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        let a = a.cloned().unwrap_or_default();
        let b = b.cloned().unwrap_or_default();
        let pick = |t: f64| t < 0.5;
        Some(ButtonBarThemeData {
            alignment: if pick(t) { a.alignment } else { b.alignment },
            main_axis_size: if pick(t) {
                a.main_axis_size
            } else {
                b.main_axis_size
            },
            button_text_theme: if pick(t) {
                a.button_text_theme
            } else {
                b.button_text_theme
            },
            button_min_width: lerp_double(a.button_min_width, b.button_min_width, t),
            button_height: lerp_double(a.button_height, b.button_height, t),
            button_padding: EdgeInsetsGeometry::lerp(a.button_padding, b.button_padding, t),
            button_aligned_dropdown: if pick(t) {
                a.button_aligned_dropdown
            } else {
                b.button_aligned_dropdown
            },
            layout_behavior: if pick(t) {
                a.layout_behavior
            } else {
                b.layout_behavior
            },
            overflow_direction: if pick(t) {
                a.overflow_direction
            } else {
                b.overflow_direction
            },
        })
    }
}

/// Applies a button bar theme to descendant `ButtonBar` widgets.
///
/// A button bar theme describes the layout and properties for the buttons contained in a
/// `ButtonBar`.
#[deprecated(note = "Use OverflowBar instead. This feature was deprecated after v3.21.0-10.0.pre.")]
pub struct ButtonBarTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `ButtonBar` widgets.
    pub data: ButtonBarThemeData,
    pub child: WidgetRef,
}

impl ButtonBarTheme {
    /// Constructs a button bar theme that configures all descendant `ButtonBar` widgets.
    pub fn new<K>(data: ButtonBarThemeData, child: impl IntoWidget<K>) -> ButtonBarTheme {
        ButtonBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ButtonBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ButtonBarTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration `data` from the closest [`ButtonBarTheme`] ancestor. If there
    /// is no ancestor, it returns `ThemeData::button_bar_theme`.
    pub fn of(app: &mut App, context: BuildContext) -> ButtonBarThemeData {
        if let Some(button_bar_theme) =
            context.depend_on_inherited_widget_of_exact_type::<ButtonBarTheme>(app)
        {
            return button_bar_theme.data.clone();
        }
        Theme::of(app, context).button_bar_theme.clone()
    }
}

impl std::fmt::Debug for ButtonBarTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonBarTheme")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for ButtonBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ButtonBarTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    use waft_widgets::testing::test_widgets;
    use waft_widgets::{Builder, Center, MainAxisAlignment, MainAxisSize, Text, VerticalDirection};

    use super::*;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    #[test]
    fn button_bar_theme_data_lerp_special_cases() {
        assert_eq!(ButtonBarThemeData::lerp(None, None, 0.0), None);
        let data = ButtonBarThemeData::new();
        assert_eq!(
            ButtonBarThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn every_field_is_null_by_default() {
        let theme = ButtonBarThemeData::new();
        assert_eq!(theme.alignment, None);
        assert_eq!(theme.main_axis_size, None);
        assert_eq!(theme.button_text_theme, None);
        assert_eq!(theme.button_min_width, None);
        assert_eq!(theme.button_height, None);
        assert_eq!(theme.button_padding, None);
        assert_eq!(theme.button_aligned_dropdown, None);
        assert_eq!(theme.layout_behavior, None);
        assert_eq!(theme.overflow_direction, None);
    }

    #[test]
    fn theme_data_uses_the_default_button_bar_theme_data() {
        let cell = waft_foundation::AppCell::new();
        let theme = ThemeData::builder().build(&cell.borrow());
        assert_eq!(theme.button_bar_theme, ButtonBarThemeData::default());
    }

    #[test]
    fn copy_with_and_equality_basics() {
        assert_eq!(
            ButtonBarThemeData::new(),
            ButtonBarThemeData::new().copy_with()
        );
    }

    #[test]
    fn the_fields_lerp_correctly() {
        let primary = ButtonBarThemeData::new()
            .with_alignment(MainAxisAlignment::End)
            .with_main_axis_size(MainAxisSize::Min)
            .with_button_text_theme(ButtonTextTheme::Primary)
            .with_button_min_width(20.0)
            .with_button_height(20.0)
            .with_button_padding(waft_painting::EdgeInsets::symmetric(5.0, 0.0))
            .with_button_aligned_dropdown(false)
            .with_layout_behavior(ButtonBarLayoutBehavior::Padded)
            .with_overflow_direction(VerticalDirection::Down);
        let accent = ButtonBarThemeData::new()
            .with_alignment(MainAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max)
            .with_button_text_theme(ButtonTextTheme::Accent)
            .with_button_min_width(10.0)
            .with_button_height(40.0)
            .with_button_padding(waft_painting::EdgeInsets::symmetric(0.0, 10.0))
            .with_button_aligned_dropdown(true)
            .with_layout_behavior(ButtonBarLayoutBehavior::Constrained)
            .with_overflow_direction(VerticalDirection::Up);

        let lerp = ButtonBarThemeData::lerp(Some(&primary), Some(&accent), 0.5)
            .expect("two themes lerp to a theme");
        assert_eq!(lerp.alignment, Some(MainAxisAlignment::Center));
        assert_eq!(lerp.main_axis_size, Some(MainAxisSize::Max));
        assert_eq!(lerp.button_text_theme, Some(ButtonTextTheme::Accent));
        assert_eq!(lerp.button_min_width, Some(15.0));
        assert_eq!(lerp.button_height, Some(30.0));
        assert_eq!(
            lerp.button_padding,
            Some(waft_painting::EdgeInsets::from_ltrb(5.0, 2.5, 5.0, 2.5).into())
        );
        assert_eq!(lerp.button_aligned_dropdown, Some(true));
        assert_eq!(
            lerp.layout_behavior,
            Some(ButtonBarLayoutBehavior::Constrained)
        );
        assert_eq!(lerp.overflow_direction, Some(VerticalDirection::Up));
    }

    #[test]
    fn of_falls_back_to_the_theme_data_field() {
        test_widgets(|tester| {
            let data = ButtonBarThemeData::new().with_button_min_width(42.0);
            let theme = ThemeData::builder()
                .button_bar_theme(data.clone())
                .build(&tester.cell.borrow());
            let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
            let captured = std::rc::Rc::clone(&seen);
            tester.pump_widget(material_app(
                theme,
                Builder::new(move |app, context| {
                    *captured.borrow_mut() = Some(ButtonBarTheme::of(app, context));
                    Center::new().child(Text::new("x")).into_widget()
                })
                .into_widget(),
            ));
            assert_eq!(seen.borrow().clone(), Some(data));
            assert_eq!(
                seen.borrow()
                    .as_ref()
                    .and_then(|theme| theme.button_min_width),
                Some(42.0)
            );
        });
    }

    #[test]
    fn an_ancestor_button_bar_theme_overrides_the_theme_data_field() {
        test_widgets(|tester| {
            let default_bar_theme = ButtonBarThemeData::new().with_button_min_width(42.0);
            let override_theme = ButtonBarThemeData::new().with_button_min_width(84.0);
            let theme = ThemeData::builder()
                .button_bar_theme(default_bar_theme)
                .build(&tester.cell.borrow());
            let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
            let captured = std::rc::Rc::clone(&seen);
            let override_clone = override_theme.clone();
            tester.pump_widget(material_app(
                theme,
                ButtonBarTheme::new(
                    override_clone,
                    Builder::new(move |app, context| {
                        *captured.borrow_mut() = Some(ButtonBarTheme::of(app, context));
                        Center::new().child(Text::new("x")).into_widget()
                    }),
                )
                .into_widget(),
            ));
            assert_eq!(seen.borrow().clone(), Some(override_theme));
            assert_eq!(
                seen.borrow()
                    .as_ref()
                    .and_then(|theme| theme.button_min_width),
                Some(84.0)
            );
        });
    }
}
