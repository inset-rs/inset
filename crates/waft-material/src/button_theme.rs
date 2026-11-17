//! Flutter counterpart: material_ui's `button_theme.dart`.

use std::fmt::{self, Debug};
use std::time::Duration;

use waft_embedder::{Brightness, Color, Radius};
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_rendering::BoxConstraints;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::material_button::MaterialButton;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, ThemeData};

/// Used with [`ButtonTheme`] and [`ButtonThemeData`] to define a button's base colors, and the
/// defaults for the button's minimum size, internal padding, and shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonTextTheme {
    /// Button text is black or white depending on `ThemeData::brightness`.
    Normal,

    /// Button text is `ColorScheme::secondary`.
    Accent,

    /// Button text is based on `ThemeData::primary_color`.
    Primary,
}

/// Used with [`ButtonTheme`] and [`ButtonThemeData`] to define how the button bar should size
/// itself with either constraints or internal padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonBarLayoutBehavior {
    /// Button bars will be constrained to a minimum height of 52.
    ///
    /// This setting is require to create button bars which conform to the Material Design
    /// specification.
    Constrained,

    /// Button bars will calculate their padding from the button theme padding.
    Padded,
}

/// Used with [`ButtonThemeData`] to configure the color and geometry of buttons.
///
/// This class is planned to be deprecated in a future release. Please use one or more of these
/// buttons and associated themes instead:
///
///  * `ElevatedButton`, `ElevatedButtonTheme`, `ElevatedButtonThemeData`,
///  * `FilledButton`, `FilledButtonTheme`, `FilledButtonThemeData`,
///  * `OutlinedButton`, `OutlinedButtonTheme`, `OutlinedButtonThemeData`
///  * `TextButton`, `TextButtonTheme`, `TextButtonThemeData`,
///
/// A button theme can be specified as part of the overall Material theme using
/// `ThemeData::button_theme`. The Material theme's button theme data can be overridden with
/// [`ButtonTheme`].
///
/// The actual appearance of buttons depends on the button theme, the button's enabled state,
/// its elevation (if any), and the overall [`Theme`].
///
/// See also:
///
///  * `RawMaterialButton`, which can be used to configure a button that doesn't depend on any
///    inherited themes.
#[derive(Debug)]
pub struct ButtonTheme {
    pub key: Option<KeyRef>,
    /// Specifies the color and geometry of buttons.
    pub data: ButtonThemeData,
    pub child: WidgetRef,
}

impl ButtonTheme {
    /// Creates a button theme.
    pub fn new<K>(child: impl IntoWidget<K>) -> ButtonTheme {
        ButtonTheme::from_button_theme_data(ButtonThemeData::new(), child)
    }

    /// Creates a button theme from `data`.
    pub fn from_button_theme_data<K>(
        data: ButtonThemeData,
        child: impl IntoWidget<K>,
    ) -> ButtonTheme {
        ButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ButtonTheme {
        self.key = Some(key);
        self
    }

    /// Dart `ButtonTheme(textTheme:)`.
    pub fn text_theme(mut self, text_theme: ButtonTextTheme) -> ButtonTheme {
        self.data = self.data.with_text_theme(text_theme);
        self
    }

    /// Dart `ButtonTheme(layoutBehavior:)`.
    pub fn layout_behavior(mut self, layout_behavior: ButtonBarLayoutBehavior) -> ButtonTheme {
        self.data = self.data.with_layout_behavior(layout_behavior);
        self
    }

    /// Dart `ButtonTheme(minWidth:)`.
    pub fn min_width(mut self, min_width: f64) -> ButtonTheme {
        self.data = self.data.with_min_width(min_width);
        self
    }

    /// Dart `ButtonTheme(height:)`.
    pub fn height(mut self, height: f64) -> ButtonTheme {
        self.data = self.data.with_height(height);
        self
    }

    /// Dart `ButtonTheme(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> ButtonTheme {
        self.data = self.data.with_padding(padding);
        self
    }

    /// Dart `ButtonTheme(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> ButtonTheme {
        self.data = self.data.with_shape(shape);
        self
    }

    /// Dart `ButtonTheme(alignedDropdown:)`.
    pub fn aligned_dropdown(mut self, aligned_dropdown: bool) -> ButtonTheme {
        self.data = self.data.with_aligned_dropdown(aligned_dropdown);
        self
    }

    /// Dart `ButtonTheme(buttonColor:)`.
    pub fn button_color(mut self, button_color: Color) -> ButtonTheme {
        self.data = self.data.with_button_color(button_color);
        self
    }

    /// Dart `ButtonTheme(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ButtonTheme {
        self.data = self.data.with_disabled_color(disabled_color);
        self
    }

    /// Dart `ButtonTheme(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> ButtonTheme {
        self.data = self.data.with_focus_color(focus_color);
        self
    }

    /// Dart `ButtonTheme(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> ButtonTheme {
        self.data = self.data.with_hover_color(hover_color);
        self
    }

    /// Dart `ButtonTheme(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ButtonTheme {
        self.data = self.data.with_highlight_color(highlight_color);
        self
    }

    /// Dart `ButtonTheme(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ButtonTheme {
        self.data = self.data.with_splash_color(splash_color);
        self
    }

    /// Dart `ButtonTheme(colorScheme:)`.
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> ButtonTheme {
        self.data = self.data.with_color_scheme(color_scheme);
        self
    }

    /// Dart `ButtonTheme(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> ButtonTheme {
        self.data = self
            .data
            .with_material_tap_target_size(material_tap_target_size);
        self
    }

    /// Retrieves the [`ButtonThemeData`] from the closest ancestor [`ButtonTheme`] widget.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ButtonThemeData {
        let inherited_button_theme =
            context.depend_on_inherited_widget_of_exact_type::<ButtonTheme>(app);
        let mut button_theme = inherited_button_theme.map(|theme| theme.data.clone());
        if button_theme
            .as_ref()
            .is_none_or(|button_theme| button_theme.color_scheme.is_none())
        {
            // if buttonTheme or buttonTheme.colorScheme is null
            let theme = Theme::of(app, context);
            let mut theme_button_theme = button_theme.unwrap_or_else(|| theme.button_theme.clone());
            if theme_button_theme.color_scheme.is_none() {
                theme_button_theme = theme_button_theme.copy_with().with_color_scheme(
                    theme
                        .button_theme
                        .color_scheme
                        .clone()
                        .unwrap_or_else(|| theme.color_scheme.clone()),
                );
                debug_assert!(theme_button_theme.color_scheme.is_some());
            }
            button_theme = Some(theme_button_theme);
        }
        button_theme.expect("filled above")
    }
}

impl InheritedTheme for ButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ButtonTheme::from_button_theme_data(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Used with [`ButtonTheme`] to configure the color and geometry of buttons.
///
/// This class is planned to be deprecated in a future release. Please use one or more of these
/// buttons and associated themes instead:
///
///  * `TextButton`, `TextButtonTheme`, `TextButtonThemeData`,
///  * `ElevatedButton`, `ElevatedButtonTheme`, `ElevatedButtonThemeData`,
///  * `OutlinedButton`, `OutlinedButtonTheme`, `OutlinedButtonThemeData`
///
/// A button theme can be specified as part of the overall Material theme using
/// `ThemeData::button_theme`. The Material theme's button theme data can be overridden with
/// [`ButtonTheme`].
///
/// [`padding`](Self::padding) and [`shape`](Self::shape) are getters that fall back to a
/// default for the [`text_theme`](Self::text_theme), so the fields hold only what was given;
/// [`copy_with`](Self::copy_with), as Dart's `copyWith`, keeps the worked-out values.
#[derive(Clone)]
pub struct ButtonThemeData {
    /// The minimum width for buttons.
    ///
    /// The actual horizontal space allocated for a button's child is at least this value less
    /// the theme's horizontal [`padding`](Self::padding).
    ///
    /// Defaults to 88.0 logical pixels.
    pub min_width: f64,

    /// The minimum height for buttons.
    ///
    /// Defaults to 36.0 logical pixels.
    pub height: f64,

    /// Defines a button's base colors, and the defaults for the button's minimum size, internal
    /// padding, and shape.
    ///
    /// Despite the name, this property is not a `TextTheme`, its value is not a collection of
    /// `TextStyle`s.
    pub text_theme: ButtonTextTheme,

    /// Defines whether a `ButtonBar` should size itself with a minimum size constraint or with
    /// padding.
    ///
    /// Defaults to [`ButtonBarLayoutBehavior::Padded`].
    pub layout_behavior: ButtonBarLayoutBehavior,

    padding: Option<EdgeInsetsGeometry>,

    shape: Option<ShapeBorderRef>,

    /// If true, then a `DropdownButton` menu's width will match the button's width.
    ///
    /// If false (the default), then the dropdown's menu will be wider than its button. In
    /// either case the dropdown button will line up the leading edge of the menu's value with
    /// the leading edge of the values displayed by the menu items.
    ///
    /// This property only affects `DropdownButton` and its menu.
    pub aligned_dropdown: bool,

    /// The background fill color.
    ///
    /// This property is null by default.
    ///
    /// If the button is in the focused, hovering, or highlighted state, then the `focus_color`,
    /// `hover_color`, or `highlight_color` will take precedence over the `button_color`.
    button_color: Option<Color>,

    /// The background fill color when disabled.
    ///
    /// This property is null by default.
    disabled_color: Option<Color>,

    /// The fill color of the button when it has the input focus.
    ///
    /// This property is null by default.
    ///
    /// If the button is in the hovering or highlighted state, then the `hover_color` or
    /// `highlight_color` will take precedence over the `focus_color`.
    focus_color: Option<Color>,

    /// The fill color of the button when a pointer is hovering over it.
    ///
    /// This property is null by default.
    ///
    /// If the button is in the highlighted state, then the `highlight_color` will take
    /// precedence over the `hover_color`.
    hover_color: Option<Color>,

    /// The color of the overlay that appears when a button is pressed.
    ///
    /// This property is null by default.
    highlight_color: Option<Color>,

    /// The color of the ink "splash" overlay that appears when a button is tapped.
    ///
    /// This property is null by default.
    splash_color: Option<Color>,

    /// A set of thirteen colors that can be used to derive the button theme's colors.
    ///
    /// This property was added much later than the theme's set of highly specific colors, like
    /// `ThemeData::highlight_color` and `ThemeData::splash_color` etc.
    ///
    /// The colors for new button classes can be defined exclusively in terms of
    /// [`color_scheme`](Self::color_scheme). When it's possible, the existing buttons will
    /// (continue to) gradually migrate to it.
    pub color_scheme: Option<ColorScheme>,

    // The minimum size of a button's tap target.
    //
    // This property is null by default.
    material_tap_target_size: Option<MaterialTapTargetSize>,
}

impl ButtonThemeData {
    /// Create a button theme object that can be used with [`ButtonTheme`] or `ThemeData`.
    ///
    /// The [`min_width`](Self::min_width) and [`height`](Self::height) parameters must greater
    /// than or equal to zero.
    pub fn new() -> ButtonThemeData {
        ButtonThemeData {
            min_width: 88.0,
            height: 36.0,
            text_theme: ButtonTextTheme::Normal,
            layout_behavior: ButtonBarLayoutBehavior::Padded,
            padding: None,
            shape: None,
            aligned_dropdown: false,
            button_color: None,
            disabled_color: None,
            focus_color: None,
            hover_color: None,
            highlight_color: None,
            splash_color: None,
            color_scheme: None,
            material_tap_target_size: None,
        }
    }

    /// Dart `ButtonThemeData(textTheme:)`.
    pub fn with_text_theme(mut self, text_theme: ButtonTextTheme) -> ButtonThemeData {
        self.text_theme = text_theme;
        self
    }

    /// Dart `ButtonThemeData(minWidth:)`.
    pub fn with_min_width(mut self, min_width: f64) -> ButtonThemeData {
        debug_assert!(min_width >= 0.0);
        self.min_width = min_width;
        self
    }

    /// Dart `ButtonThemeData(height:)`.
    pub fn with_height(mut self, height: f64) -> ButtonThemeData {
        debug_assert!(height >= 0.0);
        self.height = height;
        self
    }

    /// Dart `ButtonThemeData(padding:)`.
    pub fn with_padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> ButtonThemeData {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `ButtonThemeData(shape:)`.
    pub fn with_shape(mut self, shape: impl Into<ShapeBorderRef>) -> ButtonThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `ButtonThemeData(layoutBehavior:)`.
    pub fn with_layout_behavior(
        mut self,
        layout_behavior: ButtonBarLayoutBehavior,
    ) -> ButtonThemeData {
        self.layout_behavior = layout_behavior;
        self
    }

    /// Dart `ButtonThemeData(alignedDropdown:)`.
    pub fn with_aligned_dropdown(mut self, aligned_dropdown: bool) -> ButtonThemeData {
        self.aligned_dropdown = aligned_dropdown;
        self
    }

    /// Dart `ButtonThemeData(buttonColor:)`.
    pub fn with_button_color(mut self, button_color: Color) -> ButtonThemeData {
        self.button_color = Some(button_color);
        self
    }

    /// Dart `ButtonThemeData(disabledColor:)`.
    pub fn with_disabled_color(mut self, disabled_color: Color) -> ButtonThemeData {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ButtonThemeData(focusColor:)`.
    pub fn with_focus_color(mut self, focus_color: Color) -> ButtonThemeData {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `ButtonThemeData(hoverColor:)`.
    pub fn with_hover_color(mut self, hover_color: Color) -> ButtonThemeData {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `ButtonThemeData(highlightColor:)`.
    pub fn with_highlight_color(mut self, highlight_color: Color) -> ButtonThemeData {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `ButtonThemeData(splashColor:)`.
    pub fn with_splash_color(mut self, splash_color: Color) -> ButtonThemeData {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ButtonThemeData(colorScheme:)`.
    pub fn with_color_scheme(mut self, color_scheme: ColorScheme) -> ButtonThemeData {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Dart `ButtonThemeData(materialTapTargetSize:)`.
    pub fn with_material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> ButtonThemeData {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Convenience that returns [`min_width`](Self::min_width) and [`height`](Self::height) as
    /// a [`BoxConstraints`] object.
    pub fn constraints(&self) -> BoxConstraints {
        BoxConstraints::new()
            .min_width(self.min_width)
            .min_height(self.height)
    }

    /// Padding for a button's child (typically the button's label).
    ///
    /// Defaults to 24.0 on the left and right if [`text_theme`](Self::text_theme) is
    /// [`ButtonTextTheme::Primary`], 16.0 on the left and right otherwise.
    pub fn padding(&self) -> EdgeInsetsGeometry {
        self.padding.unwrap_or(match self.text_theme {
            ButtonTextTheme::Normal => EdgeInsetsGeometry::symmetric(0.0, 16.0),
            ButtonTextTheme::Accent => EdgeInsetsGeometry::symmetric(0.0, 16.0),
            ButtonTextTheme::Primary => EdgeInsetsGeometry::symmetric(0.0, 24.0),
        })
    }

    /// The shape of a button's material.
    ///
    /// The button's highlight and splash are clipped to this shape. If the button has an
    /// elevation, then its drop shadow is defined by this shape as well.
    ///
    /// Defaults to a rounded rectangle with circular corner radii of 4.0 if
    /// [`text_theme`](Self::text_theme) is [`ButtonTextTheme::Primary`], a rounded rectangle
    /// with circular corner radii of 2.0 otherwise.
    pub fn shape(&self) -> ShapeBorderRef {
        match &self.shape {
            Some(shape) => shape.clone(),
            None => match self.text_theme {
                ButtonTextTheme::Normal | ButtonTextTheme::Accent => {
                    ShapeBorderRef::new(RoundedRectangleBorder::new(
                        BorderSide::NONE,
                        BorderRadiusGeometry::all(Radius::circular(2.0)),
                    ))
                }
                ButtonTextTheme::Primary => ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(4.0)),
                )),
            },
        }
    }

    fn color_scheme(&self) -> &ColorScheme {
        self.color_scheme
            .as_ref()
            .expect("a button theme has a color scheme")
    }

    /// The [button]'s overall brightness.
    ///
    /// Returns the button's `colorBrightness` if it is non-null, otherwise the color scheme's
    /// `brightness`.
    pub fn get_brightness(&self, button: &MaterialButton) -> Brightness {
        button
            .color_brightness
            .unwrap_or_else(|| self.color_scheme().brightness)
    }

    /// Defines the [button]'s base colors, and the defaults for the button's minimum size,
    /// internal padding, and shape.
    pub fn get_text_theme(&self, button: &MaterialButton) -> ButtonTextTheme {
        button.text_theme.unwrap_or(self.text_theme)
    }

    /// The foreground color of the [button]'s text and icon when `on_pressed` is null.
    pub fn get_disabled_text_color(&self, button: &MaterialButton) -> Color {
        button
            .text_color
            .or(button.disabled_text_color)
            .unwrap_or_else(|| {
                self.color_scheme()
                    .on_surface
                    .with_values(Some(0.38), None, None, None, None)
            })
    }

    /// The [button]'s background color when `on_pressed` is null.
    pub fn get_disabled_fill_color(&self, button: &MaterialButton) -> Color {
        button
            .disabled_color
            .or(self.disabled_color)
            .unwrap_or_else(|| {
                self.color_scheme()
                    .on_surface
                    .with_values(Some(0.38), None, None, None, None)
            })
    }

    /// The button's background fill color, or `None` for buttons that don't have a background
    /// color.
    pub fn get_fill_color(&self, button: &MaterialButton) -> Option<Color> {
        let fill_color = if button.enabled() {
            button.color
        } else {
            button.disabled_color
        };
        if fill_color.is_some() {
            return fill_color;
        }
        // A plain `MaterialButton` has no fill; only a subclass would, and this port has none.
        None
    }

    /// The foreground color of the [button]'s text and icon.
    pub fn get_text_color(&self, button: &MaterialButton) -> Color {
        if !button.enabled() {
            return self.get_disabled_text_color(button);
        }
        if let Some(text_color) = button.text_color {
            return text_color;
        }
        match self.get_text_theme(button) {
            ButtonTextTheme::Normal => {
                if self.get_brightness(button) == Brightness::Dark {
                    Colors::WHITE
                } else {
                    Colors::BLACK87
                }
            }
            ButtonTextTheme::Accent => self.color_scheme().secondary,
            ButtonTextTheme::Primary => {
                let fill_is_dark = match self.get_fill_color(button) {
                    Some(fill_color) => {
                        ThemeData::estimate_brightness_for_color(fill_color) == Brightness::Dark
                    }
                    None => self.get_brightness(button) == Brightness::Dark,
                };
                if fill_is_dark {
                    Colors::WHITE
                } else {
                    Colors::BLACK
                }
            }
        }
    }

    /// The color of the ink "splash" overlay that appears when the (enabled) [button] is
    /// tapped.
    pub fn get_splash_color(&self, button: &MaterialButton) -> Color {
        if let Some(splash_color) = button.splash_color {
            return splash_color;
        }
        if let Some(splash_color) = self.splash_color
            && matches!(
                self.get_text_theme(button),
                ButtonTextTheme::Normal | ButtonTextTheme::Accent
            )
        {
            return splash_color;
        }
        self.get_text_color(button)
            .with_values(Some(0.12), None, None, None, None)
    }

    /// The fill color of the button when it has input focus.
    pub fn get_focus_color(&self, button: &MaterialButton) -> Color {
        button.focus_color.or(self.focus_color).unwrap_or_else(|| {
            self.get_text_color(button)
                .with_values(Some(0.12), None, None, None, None)
        })
    }

    /// The fill color of the button when a pointer is hovering over it.
    pub fn get_hover_color(&self, button: &MaterialButton) -> Color {
        button.hover_color.or(self.hover_color).unwrap_or_else(|| {
            self.get_text_color(button)
                .with_values(Some(0.04), None, None, None, None)
        })
    }

    /// The color of the overlay that appears when the [button] is pressed.
    pub fn get_highlight_color(&self, button: &MaterialButton) -> Color {
        if let Some(highlight_color) = button.highlight_color {
            return highlight_color;
        }
        match self.get_text_theme(button) {
            ButtonTextTheme::Normal | ButtonTextTheme::Accent => {
                self.highlight_color.unwrap_or_else(|| {
                    self.get_text_color(button)
                        .with_values(Some(0.16), None, None, None, None)
                })
            }
            ButtonTextTheme::Primary => Colors::TRANSPARENT,
        }
    }

    /// The [button]'s elevation when it is enabled and has not been pressed.
    pub fn get_elevation(&self, button: &MaterialButton) -> f64 {
        button.elevation.unwrap_or(2.0)
    }

    /// The [button]'s elevation when it is enabled and has focus.
    pub fn get_focus_elevation(&self, button: &MaterialButton) -> f64 {
        button.focus_elevation.unwrap_or(4.0)
    }

    /// The [button]'s elevation when it is enabled and hovered.
    pub fn get_hover_elevation(&self, button: &MaterialButton) -> f64 {
        button.hover_elevation.unwrap_or(4.0)
    }

    /// The [button]'s elevation when it is enabled and pressed.
    pub fn get_highlight_elevation(&self, button: &MaterialButton) -> f64 {
        button.highlight_elevation.unwrap_or(8.0)
    }

    /// The [button]'s elevation when `on_pressed` is null.
    pub fn get_disabled_elevation(&self, button: &MaterialButton) -> f64 {
        button.disabled_elevation.unwrap_or(0.0)
    }

    /// Padding for the [button]'s child (typically the button's label).
    pub fn get_padding(&self, button: &MaterialButton) -> EdgeInsetsGeometry {
        button
            .padding
            .or(self.padding)
            .unwrap_or_else(|| match self.get_text_theme(button) {
                ButtonTextTheme::Normal | ButtonTextTheme::Accent => {
                    EdgeInsetsGeometry::symmetric(0.0, 16.0)
                }
                ButtonTextTheme::Primary => EdgeInsetsGeometry::symmetric(0.0, 24.0),
            })
    }

    /// The shape of the [button]'s `Material`.
    pub fn get_shape(&self, button: &MaterialButton) -> ShapeBorderRef {
        button.shape.clone().unwrap_or_else(|| self.shape())
    }

    /// The duration of the [button]'s highlight animation.
    pub fn get_animation_duration(&self, button: &MaterialButton) -> Duration {
        button.animation_duration.unwrap_or(K_THEME_CHANGE_DURATION)
    }

    /// The `BoxConstraints` that define the [button]'s size.
    pub fn get_constraints(&self, _button: &MaterialButton) -> BoxConstraints {
        self.constraints()
    }

    /// The minimum size of the [button]'s tap target.
    pub fn get_material_tap_target_size(&self, button: &MaterialButton) -> MaterialTapTargetSize {
        button
            .material_tap_target_size
            .or(self.material_tap_target_size)
            .unwrap_or(MaterialTapTargetSize::Padded)
    }

    /// Creates a copy of this button theme data object with the matching fields replaced with
    /// the non-null parameter values.
    ///
    /// The copy holds this theme's [`padding`](Self::padding) and [`shape`](Self::shape) as
    /// given values, their defaults included, as Dart's `copyWith` passes `this.padding` and
    /// `this.shape` on.
    pub fn copy_with(&self) -> ButtonThemeData {
        let mut copy = self.clone();
        copy.padding = Some(self.padding());
        copy.shape = Some(self.shape());
        copy
    }
}

impl Default for ButtonThemeData {
    fn default() -> ButtonThemeData {
        ButtonThemeData::new()
    }
}

impl PartialEq for ButtonThemeData {
    fn eq(&self, other: &ButtonThemeData) -> bool {
        other.text_theme == self.text_theme
            && other.min_width == self.min_width
            && other.height == self.height
            && other.padding() == self.padding()
            && other.shape() == self.shape()
            && other.aligned_dropdown == self.aligned_dropdown
            && other.button_color == self.button_color
            && other.disabled_color == self.disabled_color
            && other.focus_color == self.focus_color
            && other.hover_color == self.hover_color
            && other.highlight_color == self.highlight_color
            && other.splash_color == self.splash_color
            && other.color_scheme == self.color_scheme
            && other.material_tap_target_size == self.material_tap_target_size
    }
}

impl Debug for ButtonThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let default_theme = ButtonThemeData::new();
        let mut properties = f.debug_struct("ButtonThemeData");
        if self.text_theme != default_theme.text_theme {
            properties.field("textTheme", &self.text_theme);
        }
        if self.min_width != default_theme.min_width {
            properties.field("minWidth", &self.min_width);
        }
        if self.height != default_theme.height {
            properties.field("height", &self.height);
        }
        if self.padding() != default_theme.padding() {
            properties.field("padding", &self.padding());
        }
        if self.shape() != default_theme.shape() {
            properties.field("shape", &self.shape());
        }
        if self.aligned_dropdown != default_theme.aligned_dropdown {
            properties.field("alignedDropdown", &"dropdown width matches button");
        }
        if let Some(button_color) = &self.button_color {
            properties.field("buttonColor", button_color);
        }
        if let Some(disabled_color) = &self.disabled_color {
            properties.field("disabledColor", disabled_color);
        }
        if let Some(focus_color) = &self.focus_color {
            properties.field("focusColor", focus_color);
        }
        if let Some(hover_color) = &self.hover_color {
            properties.field("hoverColor", hover_color);
        }
        if let Some(highlight_color) = &self.highlight_color {
            properties.field("highlightColor", highlight_color);
        }
        if let Some(splash_color) = &self.splash_color {
            properties.field("splashColor", splash_color);
        }
        if self.color_scheme.is_some() {
            properties.field("colorScheme", &"ColorScheme");
        }
        if let Some(material_tap_target_size) = &self.material_tap_target_size {
            properties.field("materialTapTargetSize", material_tap_target_size);
        }
        properties.finish()
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::StadiumBorder;

    use super::*;

    fn rounded(radius: f64) -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(radius)),
        ))
    }

    fn rounded_rectangle_border() -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::ZERO,
        ))
    }

    // button_theme_test.dart 'ButtonThemeData defaults'.
    #[test]
    fn a_new_button_theme_data_has_dart_defaults() {
        let theme = ButtonThemeData::new();
        assert_eq!(theme.text_theme, ButtonTextTheme::Normal);
        assert_eq!(
            theme.constraints(),
            BoxConstraints::new().min_width(88.0).min_height(36.0)
        );
        assert_eq!(theme.padding(), EdgeInsetsGeometry::symmetric(0.0, 16.0));
        assert_eq!(Some(theme.shape()), Some(rounded(2.0)));
        assert!(!theme.aligned_dropdown);
        assert_eq!(theme.layout_behavior, ButtonBarLayoutBehavior::Padded);
    }

    // button_theme_test.dart 'ButtonThemeData default overrides'.
    #[test]
    fn the_given_values_override_the_defaults() {
        let theme = ButtonThemeData::new()
            .with_text_theme(ButtonTextTheme::Primary)
            .with_min_width(100.0)
            .with_height(200.0)
            .with_padding(EdgeInsetsGeometry::ZERO)
            .with_shape(rounded_rectangle_border())
            .with_aligned_dropdown(true);
        assert_eq!(theme.text_theme, ButtonTextTheme::Primary);
        assert_eq!(
            theme.constraints(),
            BoxConstraints::new().min_width(100.0).min_height(200.0)
        );
        assert_eq!(theme.padding(), EdgeInsetsGeometry::ZERO);
        assert_eq!(Some(theme.shape()), Some(rounded_rectangle_border()));
        assert!(theme.aligned_dropdown);
    }

    // button_theme_test.dart 'ButtonThemeData.copyWith'.
    #[test]
    fn copy_with_keeps_every_value_and_replaces_the_given_ones() {
        let theme = ButtonThemeData::new().copy_with();
        assert_eq!(theme.text_theme, ButtonTextTheme::Normal);
        assert_eq!(theme.layout_behavior, ButtonBarLayoutBehavior::Padded);
        assert_eq!(
            theme.constraints(),
            BoxConstraints::new().min_width(88.0).min_height(36.0)
        );
        assert_eq!(theme.padding(), EdgeInsetsGeometry::symmetric(0.0, 16.0));
        assert_eq!(Some(theme.shape()), Some(rounded(2.0)));
        assert!(!theme.aligned_dropdown);
        assert!(theme.color_scheme.is_none());

        let theme = ButtonThemeData::new()
            .copy_with()
            .with_text_theme(ButtonTextTheme::Primary)
            .with_layout_behavior(ButtonBarLayoutBehavior::Constrained)
            .with_min_width(100.0)
            .with_height(200.0)
            .with_padding(EdgeInsetsGeometry::ZERO)
            .with_shape(StadiumBorder::new(BorderSide::NONE))
            .with_aligned_dropdown(true)
            .with_color_scheme(ColorScheme::dark())
            .with_material_tap_target_size(MaterialTapTargetSize::ShrinkWrap);
        assert_eq!(theme.text_theme, ButtonTextTheme::Primary);
        assert_eq!(theme.layout_behavior, ButtonBarLayoutBehavior::Constrained);
        assert_eq!(
            theme.constraints(),
            BoxConstraints::new().min_width(100.0).min_height(200.0)
        );
        assert_eq!(theme.padding(), EdgeInsetsGeometry::ZERO);
        let stadium: ShapeBorderRef = ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE));
        assert_eq!(Some(theme.shape()), Some(stadium));
        assert!(theme.aligned_dropdown);
        assert_eq!(theme.color_scheme, Some(ColorScheme::dark()));
    }

    // Dart's `copyWith` passes the `padding` and `shape` getters on, so a copy keeps the
    // defaults of the text theme it was made from.
    #[test]
    fn a_copy_keeps_the_padding_and_shape_of_the_original_text_theme() {
        let theme = ButtonThemeData::new()
            .copy_with()
            .with_text_theme(ButtonTextTheme::Primary);
        assert_eq!(theme.padding(), EdgeInsetsGeometry::symmetric(0.0, 16.0));
        assert_eq!(Some(theme.shape()), Some(rounded(2.0)));
        assert_ne!(
            theme,
            ButtonThemeData::new().with_text_theme(ButtonTextTheme::Primary)
        );
    }
}
