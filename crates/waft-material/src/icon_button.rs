//! Flutter counterpart: material_ui's `icon_button.dart`.
//!
//! The library's eight `part`s, the Material 3 and Material 3 Expressive defaults of each
//! [`IconButton`] variant, live in `generated/`.
#![allow(deprecated)]

use std::fmt::{self, Debug};

use waft_embedder::{Brightness, Clip, Color, Size};
use waft_foundation::{App, Handle, Listener, Ref, ValueChanged};
use waft_painting::{
    Alignment, AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry, OutlinedBorderRef,
};
use waft_rendering::BoxConstraints;
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    Align, BuildContext, ConstrainedBox, FocusNodeRef, IconTheme, IconThemeData, IntoWidget,
    KeyRef, Padding, Semantics, SizedBox, State, StateData, StatefulWidget, StatelessWidget,
    WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyleGetters;
use crate::button_style::{
    ButtonShapeVariant, ButtonSizeVariant, ButtonStyle, IconButtonWidthVariant,
};
use crate::button_style_button::{ButtonStyleButton, ButtonStyleButtonData, ButtonStyleState};
use crate::constants::{
    K_DEFAULT_ICON_DARK_COLOR, K_DEFAULT_ICON_LIGHT_COLOR, K_MIN_INTERACTIVE_DIMENSION,
};
use crate::debug::debug_check_has_material;
use crate::generated::filled_icon_button_defaults_m3::FilledIconButtonDefaultsM3;
use crate::generated::filled_icon_button_defaults_m3e::FilledIconButtonDefaultsM3E;
use crate::generated::filled_tonal_icon_button_defaults_m3::FilledTonalIconButtonDefaultsM3;
use crate::generated::filled_tonal_icon_button_defaults_m3e::FilledTonalIconButtonDefaultsM3E;
use crate::generated::icon_button_defaults_m3::IconButtonDefaultsM3;
use crate::generated::icon_button_defaults_m3e::IconButtonDefaultsM3E;
use crate::generated::outlined_icon_button_defaults_m3::OutlinedIconButtonDefaultsM3;
use crate::generated::outlined_icon_button_defaults_m3e::OutlinedIconButtonDefaultsM3E;
use crate::icon_button_theme::IconButtonTheme;
use crate::ink_well::{InkResponse, InteractiveInkFeatureFactoryRef};
use crate::material::Material;
use crate::material_state::MaterialStatesController;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, StyleVariant, VisualDensity};
use crate::tooltip::Tooltip;

// Minimum logical pixel size of the IconButton.
// See: <https://material.io/design/usability/accessibility.html#layout-typography>.
const K_MIN_BUTTON_SIZE: f64 = K_MIN_INTERACTIVE_DIMENSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IconButtonVariant {
    Standard,
    Filled,
    FilledTonal,
    Outlined,
}

/// A Material Design icon button.
///
/// An icon button is a picture printed on a `Material` widget that reacts to
/// touches by filling with color (ink).
///
/// Icon buttons are commonly used in the `AppBar.actions` field, but they can
/// be used in many other places as well.
///
/// If the [`on_pressed`](Self::on_pressed) callback is null, then the button will be disabled
/// and will not react to touch.
///
/// Requires one of its ancestors to be a `Material` widget. In Material Design 3,
/// this requirement no longer exists because this widget builds a subclass of
/// [`ButtonStyleButton`].
///
/// The hit region of an icon button will, if possible, be at least
/// kMinInteractiveDimension pixels in size, regardless of the actual
/// [`icon_size`](Self::icon_size), to satisfy the [touch target size](https://material.io/design/layout/spacing-methods.html#touch-targets)
/// requirements in the Material Design specification. The [`alignment`](Self::alignment)
/// controls how the icon itself is positioned within the hit region.
///
/// ### Icon sizes
///
/// When creating an icon button with an `Icon`, do not override the
/// icon's size with its `Icon::size` parameter, use the icon button's
/// [`icon_size`](Self::icon_size) parameter instead. For example do this:
///
/// ```text
/// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::favorite()))).icon_size(72.0)
/// ```
///
/// Avoid doing this:
///
/// ```text
/// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::favorite())).size(72.0))
/// ```
///
/// If you do, the button's size will be based on the default icon
/// size, not 72, which may produce unexpected layouts and clipping
/// issues.
///
/// ### Adding a filled background
///
/// Icon buttons don't support specifying a background color or other
/// background decoration because typically the icon is just displayed
/// on top of the parent widget's background. Icon buttons that appear
/// in `AppBar.actions` are an example of this.
///
/// It's easy enough to create an icon button with a filled background
/// using the `Ink` widget. The `Ink` widget renders a decoration on
/// the underlying `Material` along with the splash and highlight
/// [`InkResponse`] contributed by descendant widgets.
///
/// Material Design 3 introduced new types (standard and contained) of [`IconButton`]s.
/// The default [`IconButton`] is the standard type. To create a filled icon button,
/// use [`IconButton::filled`]; to create a filled tonal icon button, use
/// [`IconButton::filled_tonal`]; to create a outlined icon button, use
/// [`IconButton::outlined`].
///
/// Material Design 3 also treats [`IconButton`]s as toggle buttons. In order
/// to not break existing apps, the toggle feature can be optionally controlled
/// by the [`is_selected`](Self::is_selected) property.
///
/// If [`is_selected`](Self::is_selected) is null it will behave as a normal button. If
/// [`is_selected`](Self::is_selected) is not null then it will behave as a toggle button. If
/// [`is_selected`](Self::is_selected) is true then it will show
/// [`selected_icon`](Self::selected_icon), if it false it will show the normal
/// [`icon`](Self::icon).
///
/// In Material Design 3, both `IconTheme` and [`IconButtonTheme`] are used to override the
/// default style of [`IconButton`]. If both themes exist, the [`IconButtonTheme`] will override
/// `IconTheme` no matter which is closer to the [`IconButton`]. Each [`IconButton`]'s property
/// is resolved by the order of precedence: widget property, [`IconButtonTheme`] property,
/// `IconTheme` property and internal default property value.
///
/// In Material Design 3, the [`IconButton::visual_density`] defaults to
/// [`VisualDensity::STANDARD`] for all platforms; otherwise the button will have a rounded
/// rectangle shape if the [`IconButton::visual_density`] is set to
/// [`VisualDensity::COMPACT`]. Users can customize it by using [`IconButtonTheme`],
/// [`IconButton::style`] or [`IconButton::visual_density`].
///
/// To use the Material 3 Expressive style, set `IconButtonThemeData::variant`
/// to [`StyleVariant::Material3Expressive`]. The Expressive size, width, and
/// shape variants can then be selected with [`ButtonStyle::size_variant`],
/// [`ButtonStyle::icon_button_width`], and [`ButtonStyle::shape_variant`].
///
/// See also:
///
///  * `Icons`, the library of Material Icons.
///  * `BackButton`, an icon button for a "back" affordance which adapts to the
///    current platform's conventions.
///  * `CloseButton`, an icon button for closing pages.
///  * `AppBar`, to show a toolbar at the top of an application.
///  * `TextButton`, `ElevatedButton`, `OutlinedButton`, for buttons with text labels and an
///    optional icon.
///  * [`InkResponse`] and `InkWell`, for the ink splash effect itself.
pub struct IconButton {
    pub key: Option<KeyRef>,

    /// The size of the icon inside the button.
    ///
    /// If null, uses `IconThemeData::size`. If it is also null, the default size
    /// is 24.0.
    ///
    /// The size given here is passed down to the widget in the [`icon`](Self::icon) property
    /// via an `IconTheme`. Setting the size here instead of in, for example, the
    /// `Icon::size` property allows the [`IconButton`] to size the splash area to
    /// fit the `Icon`. If you were to set the size of the `Icon` using
    /// `Icon::size` instead, then the [`IconButton`] would default to 24.0 and then
    /// the `Icon` itself would likely get clipped.
    ///
    /// This property is only used when [`icon`](Self::icon) is or contains an `Icon` widget.
    /// It will be ignored if other widgets are used, such as an `Image`.
    ///
    /// If `ThemeData::use_material3` is set to true and this is null, the size of the
    /// [`IconButton`] would default to 24.0. The size given here is passed down to the
    /// [`ButtonStyle::icon_size`] property.
    pub icon_size: Option<f64>,

    /// Defines how compact the icon button's layout will be.
    ///
    /// This property can be null. If null, it defaults to [`VisualDensity::STANDARD`]
    /// in Material Design 3 to make sure the button will be circular on all platforms.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the
    ///    [`visual_density`](Self::visual_density) for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,

    /// The padding around the button's icon. The entire padded icon will react
    /// to input gestures.
    ///
    /// If `ThemeData::use_material3` is set to false, apply padding in the square
    /// proportion to the button's splash/highlight shape. For example, if the
    /// button is a circle, use padding on all sides to center the icon inside the
    /// splash/highlight circle. Otherwise, wrap the [`IconButton`] with a `Padding`
    /// widget to apply padding in the desired direction.
    ///
    /// This property can be null. If null, it defaults to 8.0 padding on all sides.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Defines how the icon is positioned within the IconButton.
    ///
    /// This property can be null. If null, it defaults to `Alignment::CENTER`.
    ///
    /// See also:
    ///
    ///  * `Alignment`, a class with convenient constants typically used to
    ///    specify an [`AlignmentGeometry`].
    ///  * `AlignmentDirectional`, like `Alignment` for specifying alignments
    ///    relative to text direction.
    pub alignment: Option<AlignmentGeometry>,

    /// The splash radius.
    ///
    /// If `ThemeData::use_material3` is set to true, this will not be used.
    ///
    /// If null, default splash radius of [`Material::DEFAULT_SPLASH_RADIUS`] is used.
    pub splash_radius: Option<f64>,

    /// The icon to display inside the button.
    ///
    /// The `Icon::size` and `Icon::color` of the icon is configured automatically
    /// based on the [`icon_size`](Self::icon_size) and [`color`](Self::color) properties of
    /// _this_ widget using an `IconTheme` and therefore should not be explicitly given in the
    /// icon widget.
    ///
    /// See `Icon`, `ImageIcon`.
    pub icon: WidgetRef,

    /// The color for the button when it has the input focus.
    ///
    /// If `ThemeData::use_material3` is set to true, this [`focus_color`](Self::focus_color)
    /// will be mapped to be the [`ButtonStyle::overlay_color`] in focused state, which paints
    /// on top of the button, as an overlay. Therefore, using a color with some transparency
    /// is recommended. For example, one could customize the
    /// [`focus_color`](Self::focus_color) below:
    ///
    /// ```text
    /// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::sunny())))
    ///     .focus_color(Colors::ORANGE.primary().with_values(Some(0.3), None, None, None, None))
    /// ```
    ///
    /// Defaults to `ThemeData::focus_color` of the ambient theme.
    pub focus_color: Option<Color>,

    /// The color for the button when a pointer is hovering over it.
    ///
    /// If `ThemeData::use_material3` is set to true, this [`hover_color`](Self::hover_color)
    /// will be mapped to be the [`ButtonStyle::overlay_color`] in hovered state, which paints
    /// on top of the button, as an overlay. Therefore, using a color with some transparency
    /// is recommended. For example, one could customize the
    /// [`hover_color`](Self::hover_color) below:
    ///
    /// ```text
    /// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::ac_unit())))
    ///     .hover_color(Colors::ORANGE.primary().with_values(Some(0.3), None, None, None, None))
    /// ```
    ///
    /// Defaults to `ThemeData::hover_color` of the ambient theme.
    pub hover_color: Option<Color>,

    /// The color to use for the icon inside the button, if the icon is enabled.
    /// Defaults to leaving this up to the [`icon`](Self::icon) widget.
    ///
    /// The icon is enabled if [`on_pressed`](Self::on_pressed) is not null.
    ///
    /// ```text
    /// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::sunny_snowing())))
    ///     .color(Colors::BLUE.primary())
    /// ```
    pub color: Option<Color>,

    /// The primary color of the button when the button is in the down (pressed) state.
    /// The splash is represented as a circular overlay that appears above the
    /// [`highlight_color`](Self::highlight_color) overlay. The splash overlay has a center
    /// point that matches the hit point of the user touch event. The splash overlay will
    /// expand to fill the button area if the touch is held for long enough time. If the
    /// splash color has transparency then the highlight and button color will show through.
    ///
    /// If `ThemeData::use_material3` is set to true, this will not be used. Use
    /// [`highlight_color`](Self::highlight_color) instead to show the overlay color of the
    /// button when the button is in the pressed state.
    ///
    /// Defaults to the Theme's splash color, `ThemeData::splash_color`.
    pub splash_color: Option<Color>,

    /// The secondary color of the button when the button is in the down (pressed)
    /// state. The highlight color is represented as a solid color that is overlaid over the
    /// button color (if any). If the highlight color has transparency, the button color
    /// will show through. The highlight fades in quickly as the button is held down.
    ///
    /// If `ThemeData::use_material3` is set to true, this
    /// [`highlight_color`](Self::highlight_color) will be mapped to be the
    /// [`ButtonStyle::overlay_color`] in pressed state, which paints on top of the button, as
    /// an overlay. Therefore, using a color with some transparency is recommended. For
    /// example, one could customize the [`highlight_color`](Self::highlight_color) below:
    ///
    /// ```text
    /// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::question_mark())))
    ///     .highlight_color(Colors::ORANGE.primary().with_values(Some(0.3), None, None, None, None))
    /// ```
    ///
    /// Defaults to the Theme's highlight color, `ThemeData::highlight_color`.
    pub highlight_color: Option<Color>,

    /// The color to use for the icon inside the button, if the icon is disabled.
    /// Defaults to the `ThemeData::disabled_color` of the current `Theme`.
    ///
    /// The icon is disabled if [`on_pressed`](Self::on_pressed) is null.
    pub disabled_color: Option<Color>,

    /// The callback that is called when the button is tapped or otherwise activated.
    ///
    /// If this is set to null, the button will be disabled.
    pub on_pressed: Option<Listener>,

    /// The callback that is called when the button is hovered.
    pub on_hover: Option<ValueChanged<bool>>,

    /// The callback that is called when the button is long-pressed.
    ///
    /// If on_pressed is set to null, the on_long_press callback is not called.
    pub on_long_press: Option<Listener>,

    /// The cursor for a mouse pointer when it enters or is hovering over the button.
    ///
    /// If set to null, will default to [`SystemMouseCursors::BASIC`] if
    /// [`on_pressed`](Self::on_pressed) is null, otherwise
    /// [`WidgetStateMouseCursor::adaptive_clickable`].
    pub mouse_cursor: Option<MouseCursorRef>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// Text that describes the action that will occur when the button is pressed.
    ///
    /// This text is displayed when the user long-presses on the button and is
    /// used for accessibility.
    pub tooltip: Option<String>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// Optional size constraints for the button.
    ///
    /// When unspecified, defaults to:
    /// ```text
    /// BoxConstraints::new()
    ///     .min_width(K_MIN_INTERACTIVE_DIMENSION)
    ///     .min_height(K_MIN_INTERACTIVE_DIMENSION)
    /// ```
    /// where [`K_MIN_INTERACTIVE_DIMENSION`] is 48.0, and then with visual density
    /// applied.
    ///
    /// The default constraints ensure that the button is accessible.
    /// Specifying this parameter enables creation of buttons smaller than
    /// the minimum size, but it is not recommended.
    ///
    /// The visual density uses the [`visual_density`](Self::visual_density) parameter if
    /// specified, and `Theme::of(context).visual_density` otherwise.
    pub constraints: Option<BoxConstraints>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding
    /// properties in `IconButtonM3`'s `theme_style_of` and `default_style_of`.
    /// `WidgetStateProperty`s that resolve to non-null values will similarly
    /// override the corresponding `WidgetStateProperty`s in `IconButtonM3`'s
    /// `theme_style_of` and `default_style_of`.
    ///
    /// The [`style`](Self::style) is only used for Material 3 [`IconButton`]. If
    /// `ThemeData::use_material3` is set to true, [`style`](Self::style) is preferred for
    /// icon button customization, and any parameters defined in [`style`](Self::style) will
    /// override the same parameters in [`IconButton`].
    ///
    /// For example, if [`IconButton`]'s [`visual_density`](Self::visual_density) is set to
    /// [`VisualDensity::STANDARD`] and [`style`](Self::style)'s
    /// [`visual_density`](ButtonStyle::visual_density) is set to [`VisualDensity::COMPACT`],
    /// the icon button will have [`VisualDensity::COMPACT`] to define the button's layout.
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// The optional selection state of the icon button.
    ///
    /// If this property is null, the button will behave as a normal push button,
    /// otherwise, the button will toggle between showing [`icon`](Self::icon) and
    /// [`selected_icon`](Self::selected_icon) based on the value of
    /// [`is_selected`](Self::is_selected). If true, it will show
    /// [`selected_icon`](Self::selected_icon), if false it will show [`icon`](Self::icon).
    ///
    /// This property is only used if `ThemeData::use_material3` is true.
    pub is_selected: Option<bool>,

    /// The icon to display inside the button when [`is_selected`](Self::is_selected) is true.
    /// This property can be null. The original [`icon`](Self::icon) will be used for both
    /// selected and unselected status if it is null.
    ///
    /// The `Icon::size` and `Icon::color` of the icon is configured automatically
    /// based on the [`icon_size`](Self::icon_size) and [`color`](Self::color) properties
    /// using an `IconTheme` and therefore should not be explicitly configured in the icon
    /// widget.
    ///
    /// This property is only used if `ThemeData::use_material3` is true.
    ///
    /// See also:
    ///
    /// * `Icon`, for icons based on glyphs from fonts instead of images.
    /// * `ImageIcon`, for showing icons from `AssetImage`s or other `ImageProvider`s.
    pub selected_icon: Option<WidgetRef>,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// [`WidgetState`]s, including [`WidgetState::Pressed`], [`WidgetState::Hovered`] and
    /// [`WidgetState::Focused`].
    pub states_controller: Option<Ref<MaterialStatesController>>,

    variant: IconButtonVariant,
}

/// The fluent setters of Dart's optional `IconButton` arguments.
macro_rules! icon_button_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `IconButton(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> IconButton {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl IconButton {
    /// Creates an icon button.
    ///
    /// Icon buttons are commonly used in the `AppBar.actions` field, but they can
    /// be used in many other places as well.
    ///
    /// Requires one of its ancestors to be a `Material` widget. This requirement
    /// no longer exists if `ThemeData::use_material3` is set to true.
    ///
    /// The [`icon`](Self::icon) argument must be specified, and is typically either an `Icon`
    /// or an `ImageIcon`.
    pub fn new<K>(on_pressed: Option<Listener>, icon: impl IntoWidget<K>) -> IconButton {
        IconButton::with_variant(on_pressed, icon.into_widget(), IconButtonVariant::Standard)
    }

    /// Create a filled variant of IconButton.
    ///
    /// Filled icon buttons have higher visual impact and should be used for
    /// high emphasis actions, such as turning off a microphone or camera.
    pub fn filled<K>(on_pressed: Option<Listener>, icon: impl IntoWidget<K>) -> IconButton {
        IconButton::with_variant(on_pressed, icon.into_widget(), IconButtonVariant::Filled)
    }

    /// Create a filled tonal variant of IconButton.
    ///
    /// Filled tonal icon buttons are a middle ground between filled and outlined
    /// icon buttons. They’re useful in contexts where the button requires slightly
    /// more emphasis than an outline would give, such as a secondary action paired
    /// with a high emphasis action.
    pub fn filled_tonal<K>(on_pressed: Option<Listener>, icon: impl IntoWidget<K>) -> IconButton {
        IconButton::with_variant(
            on_pressed,
            icon.into_widget(),
            IconButtonVariant::FilledTonal,
        )
    }

    /// Create an outlined variant of IconButton.
    ///
    /// Outlined icon buttons are medium-emphasis buttons. They’re useful when an
    /// icon button needs more emphasis than a standard icon button but less than
    /// a filled or filled tonal icon button.
    pub fn outlined<K>(on_pressed: Option<Listener>, icon: impl IntoWidget<K>) -> IconButton {
        IconButton::with_variant(on_pressed, icon.into_widget(), IconButtonVariant::Outlined)
    }

    fn with_variant(
        on_pressed: Option<Listener>,
        icon: WidgetRef,
        variant: IconButtonVariant,
    ) -> IconButton {
        IconButton {
            key: None,
            icon_size: None,
            visual_density: None,
            padding: None,
            alignment: None,
            splash_radius: None,
            icon,
            focus_color: None,
            hover_color: None,
            color: None,
            splash_color: None,
            highlight_color: None,
            disabled_color: None,
            on_pressed,
            on_hover: None,
            on_long_press: None,
            mouse_cursor: None,
            focus_node: None,
            autofocus: false,
            tooltip: None,
            enable_feedback: None,
            constraints: None,
            style: None,
            is_selected: None,
            selected_icon: None,
            states_controller: None,
            variant,
        }
    }

    icon_button_setters!(
        key: KeyRef => "key",
        icon_size: f64 => "iconSize",
        visual_density: VisualDensity => "visualDensity",
        padding: EdgeInsetsGeometry => "padding",
        alignment: AlignmentGeometry => "alignment",
        color: Color => "color",
        focus_color: Color => "focusColor",
        hover_color: Color => "hoverColor",
        highlight_color: Color => "highlightColor",
        splash_color: Color => "splashColor",
        disabled_color: Color => "disabledColor",
        on_hover: ValueChanged<bool> => "onHover",
        on_long_press: Listener => "onLongPress",
        mouse_cursor: MouseCursorRef => "mouseCursor",
        focus_node: FocusNodeRef => "focusNode",
        enable_feedback: bool => "enableFeedback",
        constraints: BoxConstraints => "constraints",
        style: ButtonStyle => "style",
        states_controller: Ref<MaterialStatesController> => "statesController",
    );

    /// Dart `IconButton(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> IconButton {
        debug_assert!(splash_radius > 0.0);
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `IconButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> IconButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `IconButton(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> IconButton {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `IconButton(isSelected:)`.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_selected(mut self, is_selected: bool) -> IconButton {
        self.is_selected = Some(is_selected);
        self
    }

    /// Dart `IconButton(selectedIcon:)`.
    pub fn selected_icon<K>(mut self, selected_icon: impl IntoWidget<K>) -> IconButton {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }

    /// A static convenience method that constructs an icon button
    /// [`ButtonStyle`] given simple values. This method is only used for Material 3.
    ///
    /// The `foreground_color` color is used to create a [`WidgetStateProperty`]
    /// [`ButtonStyle::foreground_color`] value. Specify a value for `foreground_color`
    /// to specify the color of the button's icons. The `hover_color`, `focus_color`
    /// and `highlight_color` colors are used to indicate the hover, focus,
    /// and pressed states if `overlay_color` isn't specified.
    ///
    /// If `overlay_color` is specified and its value is `Colors::TRANSPARENT`
    /// then the pressed/focused/hovered highlights are effectively defeated.
    /// Otherwise a [`WidgetStateProperty`] with the same opacities as the
    /// default is created.
    ///
    /// Use `background_color` for the button's background fill color. Use
    /// `disabled_foreground_color` and `disabled_background_color` to specify the button's
    /// disabled icon and fill color.
    ///
    /// Similarly, the `enabled_mouse_cursor` and `disabled_mouse_cursor`
    /// parameters are used to construct [`ButtonStyle::mouse_cursor`].
    ///
    /// All of the other parameters are either used directly or used to
    /// create a [`WidgetStateProperty`] with a single value for all
    /// states.
    ///
    /// The `size_variant`, `icon_button_width`, and `shape_variant` parameters are
    /// Material 3 Expressive options. They provide extra-small through
    /// extra-large sizes, narrow through wide widths, and round or square shapes
    /// through [`ButtonStyle::size_variant`], [`ButtonStyle::icon_button_width`], and
    /// [`ButtonStyle::shape_variant`].
    ///
    /// When `IconButtonThemeData::variant` is [`StyleVariant::Material3Expressive`],
    /// and these properties are null, Material 3 Expressive [`IconButton`] defaults
    /// use [`ButtonSizeVariant::Small`], [`IconButtonWidthVariant::Standard`], and
    /// [`ButtonShapeVariant::Round`].
    ///
    /// All parameters default to null, by default this method returns
    /// a [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default icon color for a
    /// [`IconButton`], as well as its overlay color, with all of the
    /// standard opacity adjustments for the pressed, focused, and
    /// hovered states, one could write:
    ///
    /// ```text
    /// IconButton::new(Some(on_pressed), Icon::new(Some(Icons::pets())))
    ///     .style(IconButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    /// ```
    pub fn style_from() -> IconButtonStyleFromBuilder {
        IconButtonStyleFromBuilder::default()
    }
}

impl StatelessWidget for IconButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);

        if theme.use_material3 {
            let effective_variant = IconButtonTheme::variant_of(app, context);

            let min_size = self
                .constraints
                .map(|constraints| Size::new(constraints.min_width, constraints.min_height));
            let max_size = self
                .constraints
                .map(|constraints| Size::new(constraints.max_width, constraints.max_height));

            let mut adjusted_style = IconButtonStyleFromBuilder {
                visual_density: self.visual_density,
                foreground_color: self.color,
                disabled_foreground_color: self.disabled_color,
                focus_color: self.focus_color,
                hover_color: self.hover_color,
                highlight_color: self.highlight_color,
                padding: self.padding,
                minimum_size: min_size,
                maximum_size: max_size,
                icon_size: self.icon_size,
                alignment: self.alignment,
                enabled_mouse_cursor: self.mouse_cursor.clone(),
                disabled_mouse_cursor: self.mouse_cursor.clone(),
                enable_feedback: self.enable_feedback,
                ..IconButtonStyleFromBuilder::default()
            }
            .build();
            if let Some(style) = &self.style {
                adjusted_style = style.merge(Some(&adjusted_style));
            }
            if adjusted_style.icon_color.is_none() {
                adjusted_style.icon_color = adjusted_style.foreground_color.clone();
            }
            let mut effective_icon = self.icon.clone();
            if self.is_selected.unwrap_or(false)
                && let Some(selected_icon) = &self.selected_icon
            {
                effective_icon = selected_icon.clone();
            }

            return SelectableIconButton {
                is_selected: self.is_selected,
                style: Some(adjusted_style),
                focus_node: self.focus_node.clone(),
                on_long_press: if self.on_pressed.is_some() {
                    self.on_long_press.clone()
                } else {
                    None
                },
                on_hover: self.on_hover.clone(),
                states_controller: self.states_controller.clone(),
                variant: self.variant,
                style_variant: effective_variant,
                autofocus: self.autofocus,
                on_pressed: self.on_pressed.clone(),
                tooltip: self.tooltip.clone(),
                child: effective_icon,
            }
            .into_widget();
        }

        debug_assert!(debug_check_has_material(app, context));

        let current_color = if self.on_pressed.is_some() {
            self.color
        } else {
            Some(self.disabled_color.unwrap_or(theme.disabled_color))
        };

        let effective_visual_density = self.visual_density.unwrap_or(theme.visual_density);

        let unadjusted_constraints = self.constraints.unwrap_or(
            BoxConstraints::new()
                .min_width(K_MIN_BUTTON_SIZE)
                .min_height(K_MIN_BUTTON_SIZE),
        );
        let adjusted_constraints =
            effective_visual_density.effective_constraints(unadjusted_constraints);
        let effective_icon_size = self
            .icon_size
            .or(IconTheme::of(app, context).size)
            .unwrap_or(24.0);
        let effective_padding = self.padding.unwrap_or(EdgeInsets::all(8.0).into());
        let effective_alignment = self.alignment.unwrap_or(Alignment::CENTER.into());
        let effective_enable_feedback = self.enable_feedback.unwrap_or(true);

        let mut icon_theme_data = IconThemeData::new().size(effective_icon_size);
        if let Some(current_color) = current_color {
            icon_theme_data = icon_theme_data.color(current_color);
        }
        let mut result = ConstrainedBox::new(adjusted_constraints)
            .child(
                Padding::new(effective_padding).child(
                    SizedBox::square(Some(effective_icon_size)).child(
                        Align::new()
                            .alignment(effective_alignment)
                            .child(IconTheme::merge(None, icon_theme_data, self.icon.clone())),
                    ),
                ),
            )
            .into_widget();

        let mut ink_response = InkResponse::new()
            .autofocus(self.autofocus)
            .can_request_focus(self.on_pressed.is_some())
            .mouse_cursor(self.mouse_cursor.clone().unwrap_or_else(|| {
                if self.on_pressed.is_some() {
                    WidgetStateMouseCursor::adaptive_clickable().into()
                } else {
                    SystemMouseCursors::BASIC.into()
                }
            }))
            .enable_feedback(effective_enable_feedback)
            .focus_color(self.focus_color.unwrap_or(theme.focus_color))
            .hover_color(self.hover_color.unwrap_or(theme.hover_color))
            .highlight_color(self.highlight_color.unwrap_or(theme.highlight_color))
            .splash_color(self.splash_color.unwrap_or(theme.splash_color))
            .radius(self.splash_radius.unwrap_or_else(|| {
                Material::DEFAULT_SPLASH_RADIUS.max(
                    (effective_icon_size
                        + effective_padding
                            .horizontal()
                            .min(effective_padding.vertical()))
                        * 0.7,
                    // x 0.5 for diameter -> radius and + 40% overflow derived from other Material apps.
                )
            }))
            .child(result);
        ink_response.focus_node = self.focus_node.clone();
        ink_response.on_tap = self.on_pressed.clone();
        ink_response.on_hover = self.on_hover.clone();
        ink_response.on_long_press = if self.on_pressed.is_some() {
            self.on_long_press.clone()
        } else {
            None
        };
        result = ink_response.into_widget();

        if let Some(tooltip) = &self.tooltip {
            result = Tooltip::new()
                .message(tooltip.clone())
                .child(result)
                .into_widget();
        }

        Semantics::new()
            .button(true)
            .enabled(self.on_pressed.is_some())
            .child(result)
            .into_widget()
    }
}

impl Debug for IconButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("IconButton");
        if let Some(tooltip) = &self.tooltip {
            properties.field("tooltip", tooltip);
        }
        if self.on_pressed.is_none() {
            properties.field("on_pressed", &"disabled");
        }
        if self.on_hover.is_none() {
            properties.field("on_hover", &"disabled");
        }
        if self.on_long_press.is_none() {
            properties.field("on_long_press", &"disabled");
        }
        if let Some(color) = &self.color {
            properties.field("color", color);
        }
        if let Some(disabled_color) = &self.disabled_color {
            properties.field("disabled_color", disabled_color);
        }
        if let Some(focus_color) = &self.focus_color {
            properties.field("focus_color", focus_color);
        }
        if let Some(hover_color) = &self.hover_color {
            properties.field("hover_color", hover_color);
        }
        if let Some(highlight_color) = &self.highlight_color {
            properties.field("highlight_color", highlight_color);
        }
        if let Some(splash_color) = &self.splash_color {
            properties.field("splash_color", splash_color);
        }
        if let Some(padding) = &self.padding {
            properties.field("padding", padding);
        }
        if self.focus_node.is_some() {
            properties.field("focus_node", &"FocusNode");
        }
        properties.finish()
    }
}

/// Dart's `IconButton.styleFrom`, collecting its arguments; [`build`](Self::build) is its
/// body.
#[derive(Clone, Default)]
pub struct IconButtonStyleFromBuilder {
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    disabled_foreground_color: Option<Color>,
    disabled_background_color: Option<Color>,
    focus_color: Option<Color>,
    hover_color: Option<Color>,
    highlight_color: Option<Color>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    overlay_color: Option<Color>,
    elevation: Option<f64>,
    minimum_size: Option<Size>,
    fixed_size: Option<Size>,
    maximum_size: Option<Size>,
    icon_size: Option<f64>,
    side: Option<waft_painting::BorderSide>,
    shape: Option<OutlinedBorderRef>,
    padding: Option<EdgeInsetsGeometry>,
    enabled_mouse_cursor: Option<MouseCursorRef>,
    disabled_mouse_cursor: Option<MouseCursorRef>,
    visual_density: Option<VisualDensity>,
    tap_target_size: Option<MaterialTapTargetSize>,
    animation_duration: Option<std::time::Duration>,
    enable_feedback: Option<bool>,
    alignment: Option<AlignmentGeometry>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
    size_variant: Option<ButtonSizeVariant>,
    icon_button_width: Option<IconButtonWidthVariant>,
    shape_variant: Option<ButtonShapeVariant>,
}

/// The setters of Dart's `IconButton.styleFrom` arguments.
macro_rules! style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `IconButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> IconButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl IconButtonStyleFromBuilder {
    style_from_setters!(
        foreground_color: Color => "foregroundColor",
        background_color: Color => "backgroundColor",
        disabled_foreground_color: Color => "disabledForegroundColor",
        disabled_background_color: Color => "disabledBackgroundColor",
        focus_color: Color => "focusColor",
        hover_color: Color => "hoverColor",
        highlight_color: Color => "highlightColor",
        shadow_color: Color => "shadowColor",
        surface_tint_color: Color => "surfaceTintColor",
        overlay_color: Color => "overlayColor",
        elevation: f64 => "elevation",
        minimum_size: Size => "minimumSize",
        fixed_size: Size => "fixedSize",
        maximum_size: Size => "maximumSize",
        icon_size: f64 => "iconSize",
        side: waft_painting::BorderSide => "side",
        padding: EdgeInsetsGeometry => "padding",
        enabled_mouse_cursor: MouseCursorRef => "enabledMouseCursor",
        disabled_mouse_cursor: MouseCursorRef => "disabledMouseCursor",
        visual_density: VisualDensity => "visualDensity",
        tap_target_size: MaterialTapTargetSize => "tapTargetSize",
        animation_duration: std::time::Duration => "animationDuration",
        enable_feedback: bool => "enableFeedback",
        alignment: AlignmentGeometry => "alignment",
        splash_factory: InteractiveInkFeatureFactoryRef => "splashFactory",
        size_variant: ButtonSizeVariant => "sizeVariant",
        icon_button_width: IconButtonWidthVariant => "iconButtonWidth",
        shape_variant: ButtonShapeVariant => "shapeVariant",
    );

    /// Dart `IconButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> IconButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The style made from the given values.
    pub fn build(self) -> ButtonStyle {
        let overlay_fallback = self.overlay_color.or(self.foreground_color);
        let mut overlay_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> = None;
        if self
            .hover_color
            .or(self.focus_color)
            .or(self.highlight_color)
            .or(overlay_fallback)
            .is_some()
        {
            overlay_color_prop = Some(match self.overlay_color {
                Some(overlay_color) if overlay_color.a == 0.0 => {
                    <dyn ButtonStyleButton>::all_or_null(Some(overlay_color)).expect("a color")
                }
                _ => <dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
                    (
                        WidgetState::Pressed.into(),
                        self.highlight_color
                            .or(overlay_fallback.map(|color| color.with_opacity(0.1))),
                    ),
                    (
                        WidgetState::Hovered.into(),
                        self.hover_color
                            .or(overlay_fallback.map(|color| color.with_opacity(0.08))),
                    ),
                    (
                        WidgetState::Focused.into(),
                        self.focus_color
                            .or(overlay_fallback.map(|color| color.with_opacity(0.1))),
                    ),
                    // Dart's map answers null when no key matches; waft-widgets' needs the key.
                    (WidgetState::any(), None),
                ]),
            });
        }

        let mouse_cursor =
            if self.disabled_mouse_cursor.is_none() && self.enabled_mouse_cursor.is_none() {
                None
            } else {
                Some(<dyn WidgetStateProperty<Option<MouseCursorRef>>>::from_map(
                    vec![
                        (WidgetState::Disabled.into(), self.disabled_mouse_cursor),
                        (WidgetState::any(), self.enabled_mouse_cursor),
                    ],
                ))
            };

        ButtonStyle {
            background_color: <dyn ButtonStyleButton>::default_color(
                self.background_color,
                self.disabled_background_color,
            ),
            foreground_color: <dyn ButtonStyleButton>::default_color(
                self.foreground_color,
                self.disabled_foreground_color,
            ),
            overlay_color: overlay_color_prop,
            shadow_color: <dyn ButtonStyleButton>::all_or_null(self.shadow_color),
            surface_tint_color: <dyn ButtonStyleButton>::all_or_null(self.surface_tint_color),
            elevation: <dyn ButtonStyleButton>::all_or_null(self.elevation),
            padding: <dyn ButtonStyleButton>::all_or_null(self.padding),
            minimum_size: <dyn ButtonStyleButton>::all_or_null(self.minimum_size),
            fixed_size: <dyn ButtonStyleButton>::all_or_null(self.fixed_size),
            maximum_size: <dyn ButtonStyleButton>::all_or_null(self.maximum_size),
            icon_size: <dyn ButtonStyleButton>::all_or_null(self.icon_size),
            side: <dyn ButtonStyleButton>::all_or_null(self.side),
            shape: <dyn ButtonStyleButton>::all_or_null(self.shape),
            mouse_cursor,
            visual_density: self.visual_density,
            tap_target_size: self.tap_target_size,
            animation_duration: self.animation_duration,
            enable_feedback: self.enable_feedback,
            alignment: self.alignment,
            splash_factory: self.splash_factory,
            size_variant: self.size_variant,
            icon_button_width: self.icon_button_width,
            shape_variant: self.shape_variant,
            ..ButtonStyle::new()
        }
    }
}

impl Debug for IconButtonStyleFromBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IconButtonStyleFromBuilder")
    }
}

/// Dart's private `_SelectableIconButton`: the Material 3 [`IconButton`], which keeps the
/// selected state in its states controller.
struct SelectableIconButton {
    is_selected: Option<bool>,
    style: Option<ButtonStyle>,
    focus_node: Option<FocusNodeRef>,
    variant: IconButtonVariant,
    style_variant: StyleVariant,
    autofocus: bool,
    on_pressed: Option<Listener>,
    tooltip: Option<String>,
    child: WidgetRef,
    on_long_press: Option<Listener>,
    on_hover: Option<ValueChanged<bool>>,
    states_controller: Option<Ref<MaterialStatesController>>,
}

impl Debug for SelectableIconButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectableIconButton")
            .field("is_selected", &self.is_selected)
            .field("variant", &self.variant)
            .field("style_variant", &self.style_variant)
            .finish()
    }
}

impl StatefulWidget for SelectableIconButton {
    type State = SelectableIconButtonState;

    fn create_state(&self) -> SelectableIconButtonState {
        SelectableIconButtonState {
            state: StateData::new(),
            internal_states_controller: None,
        }
    }
}

/// Dart's private `_SelectableIconButtonState`.
struct SelectableIconButtonState {
    state: StateData<SelectableIconButton>,
    internal_states_controller: Option<Ref<MaterialStatesController>>,
}

impl SelectableIconButtonState {
    fn states_controller(self: Handle<Self>, app: &App) -> Ref<MaterialStatesController> {
        self.widget(app)
            .states_controller
            .clone()
            .or_else(|| app.get(self).internal_states_controller.clone())
            .expect("created in init_state or init_states_controller")
    }

    fn is_selected(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_selected.unwrap_or(false)
    }

    fn init_states_controller(self: Handle<Self>, app: &mut App) {
        if self.widget(app).states_controller.is_none() {
            let controller = MaterialStatesController::new(app, None);
            app.get_mut(self).internal_states_controller = Some(controller);
        }

        let is_selected = self.is_selected(app);
        self.states_controller(app)
            .update(app, WidgetState::Selected, is_selected);
    }
}

impl State for SelectableIconButtonState {
    type Widget = SelectableIconButton;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.widget(app).states_controller.is_none() {
            let controller = MaterialStatesController::new(app, None);
            app.get_mut(self).internal_states_controller = Some(controller);
        }

        let is_selected = self.is_selected(app);
        self.states_controller(app)
            .update(app, WidgetState::Selected, is_selected);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SelectableIconButton) {
        if self.widget(app).states_controller != old_widget.states_controller {
            if self.widget(app).states_controller.is_some()
                && let Some(internal) = app.get_mut(self).internal_states_controller.take()
            {
                internal.dispose(app);
            }
            self.init_states_controller(app);
        }

        if self.widget(app).is_selected != old_widget.is_selected {
            let is_selected = self.is_selected(app);
            self.states_controller(app)
                .update(app, WidgetState::Selected, is_selected);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let states_controller = self.states_controller(app);
        let widget = self.widget(app);
        let toggleable = widget.is_selected.is_some();

        let mut child = Semantics::new();
        if let Some(is_selected) = widget.is_selected {
            child = child.selected(is_selected);
        }
        let mut button = IconButtonM3::new(
            widget.on_pressed.clone(),
            widget.variant,
            widget.style_variant,
            toggleable,
            child.child(widget.child.clone()).into_widget(),
        );
        let data = &mut button.button_style_button;
        data.states_controller = Some(states_controller);
        data.style = widget.style.clone();
        data.autofocus = widget.autofocus;
        data.focus_node = widget.focus_node.clone();
        data.on_hover = widget.on_hover.clone();
        data.on_long_press = if widget.on_pressed.is_some() {
            widget.on_long_press.clone()
        } else {
            None
        };
        data.tooltip = widget.tooltip.clone();
        button.into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(internal) = app.get_mut(self).internal_states_controller.take() {
            internal.dispose(app);
        }
    }
}

/// Dart's private `_IconButtonM3`: the [`ButtonStyleButton`] a Material 3 [`IconButton`]
/// builds.
struct IconButtonM3 {
    button_style_button: ButtonStyleButtonData,
    icon_button_variant: IconButtonVariant,
    style_variant: StyleVariant,
    toggleable: bool,
}

impl IconButtonM3 {
    fn new(
        on_pressed: Option<Listener>,
        icon_button_variant: IconButtonVariant,
        style_variant: StyleVariant,
        toggleable: bool,
        child: WidgetRef,
    ) -> IconButtonM3 {
        let mut button_style_button = ButtonStyleButtonData::new(on_pressed, Some(child));
        button_style_button.autofocus = false;
        button_style_button.on_focus_change = None;
        button_style_button.clip_behavior = Some(Clip::None);
        IconButtonM3 {
            button_style_button,
            icon_button_variant,
            style_variant,
            toggleable,
        }
    }
}

impl ButtonStyleButton for IconButtonM3 {
    crate::button_style_button_accessors!();

    /// ## Material 3 defaults
    ///
    /// If `ThemeData::use_material3` is true and `IconButtonThemeData::variant` is
    /// [`StyleVariant::Material3`], the following defaults will be
    /// used:
    ///
    /// * `text_style` - null
    /// * `background_color` - transparent
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * selected - Theme.colorScheme.primary
    ///   * others - Theme.colorScheme.onSurfaceVariant
    /// * `overlay_color`
    ///   * selected
    ///      * hovered - Theme.colorScheme.primary(0.08)
    ///      * focused or pressed - Theme.colorScheme.primary(0.1)
    ///   * hovered - Theme.colorScheme.onSurfaceVariant(0.08)
    ///   * pressed or focused - Theme.colorScheme.onSurfaceVariant(0.1)
    ///   * others - null
    /// * `shadow_color` - null
    /// * `surface_tint_color` - null
    /// * `elevation` - 0
    /// * `padding` - all(8)
    /// * `minimum_size` - Size(40, 40)
    /// * `fixed_size` - null
    /// * `maximum_size` - Size.infinite
    /// * `icon_size` - 24
    /// * `side` - null
    /// * `shape` - StadiumBorder()
    /// * `mouse_cursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visual_density` - VisualDensity.standard
    /// * `tap_target_size` - theme.materialTapTargetSize
    /// * `animation_duration` - kThemeChangeDuration
    /// * `enable_feedback` - true
    /// * `alignment` - Alignment.center
    /// * `splash_factory` - Theme.splashFactory
    ///
    /// ## Material 3 Expressive defaults
    ///
    /// If `ThemeData::use_material3` is true and `IconButtonThemeData::variant` is
    /// [`StyleVariant::Material3Expressive`], Material 3 Expressive defaults are
    /// used:
    ///
    /// * `text_style` - null
    /// * `background_color` - transparent
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * selected - Theme.colorScheme.primary
    ///   * others - Theme.colorScheme.onSurfaceVariant
    /// * `overlay_color`
    ///   * selected
    ///      * hovered - Theme.colorScheme.primary(0.08)
    ///      * focused or pressed - Theme.colorScheme.primary(0.1)
    ///   * hovered - Theme.colorScheme.onSurfaceVariant(0.08)
    ///   * pressed or focused - Theme.colorScheme.onSurfaceVariant(0.1)
    ///   * others - transparent
    /// * `shadow_color` - null
    /// * `surface_tint_color` - null
    /// * `elevation` - 0
    /// * `padding` - based on [`ButtonStyle::size_variant`] and
    ///   [`ButtonStyle::icon_button_width`]; defaults to all(8)
    /// * `minimum_size` - based on [`ButtonStyle::size_variant`] and
    ///   [`ButtonStyle::icon_button_width`]; defaults to Size(40, 40)
    /// * `fixed_size` - null
    /// * `maximum_size` - Size.infinite
    /// * `icon_size` - based on [`ButtonStyle::size_variant`]; defaults to 24
    /// * `side` - null
    /// * `shape` - based on [`ButtonStyle::size_variant`],
    ///   [`ButtonStyle::shape_variant`], and state; defaults to StadiumBorder()
    /// * `mouse_cursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visual_density` - VisualDensity.standard
    /// * `tap_target_size` - MaterialTapTargetSize.padded
    /// * `animation_duration` - kThemeChangeDuration
    /// * `enable_feedback` - true
    /// * `alignment` - Alignment.center
    /// * `splash_factory` - Theme.splashFactory
    /// * `size_variant` - ButtonSizeVariant.small
    /// * `icon_button_width` - IconButtonWidthVariant.standard
    /// * `shape_variant` - ButtonShapeVariant.round
    fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let icon_button_theme_style = IconButtonTheme::of(app, context).style;
        let style = self.button_style_button.style.as_ref();
        let effective_size = style
            .and_then(|style| style.size_variant)
            .or(icon_button_theme_style
                .as_ref()
                .and_then(|style| style.size_variant));
        let effective_width =
            style
                .and_then(|style| style.icon_button_width)
                .or(icon_button_theme_style
                    .as_ref()
                    .and_then(|style| style.icon_button_width));
        let effective_shape =
            style
                .and_then(|style| style.shape_variant)
                .or(icon_button_theme_style
                    .as_ref()
                    .and_then(|style| style.shape_variant));

        let toggleable = self.toggleable;
        match self.style_variant {
            StyleVariant::Material3 => match self.icon_button_variant {
                IconButtonVariant::Filled => {
                    Box::new(FilledIconButtonDefaultsM3::new(context, toggleable))
                }
                IconButtonVariant::FilledTonal => {
                    Box::new(FilledTonalIconButtonDefaultsM3::new(context, toggleable))
                }
                IconButtonVariant::Outlined => {
                    Box::new(OutlinedIconButtonDefaultsM3::new(context, toggleable))
                }
                IconButtonVariant::Standard => {
                    Box::new(IconButtonDefaultsM3::new(context, toggleable))
                }
            },
            StyleVariant::Material3Expressive => match self.icon_button_variant {
                IconButtonVariant::Filled => Box::new(FilledIconButtonDefaultsM3E::new(
                    context,
                    toggleable,
                    effective_size,
                    effective_width,
                    effective_shape,
                )),
                IconButtonVariant::FilledTonal => Box::new(FilledTonalIconButtonDefaultsM3E::new(
                    context,
                    toggleable,
                    effective_size,
                    effective_width,
                    effective_shape,
                )),
                IconButtonVariant::Outlined => Box::new(OutlinedIconButtonDefaultsM3E::new(
                    context,
                    toggleable,
                    effective_size,
                    effective_width,
                    effective_shape,
                )),
                IconButtonVariant::Standard => Box::new(IconButtonDefaultsM3E::new(
                    context,
                    toggleable,
                    effective_size,
                    effective_width,
                    effective_shape,
                )),
            },
        }
    }

    /// Returns the `IconButtonThemeData::style` of the closest [`IconButtonTheme`] ancestor.
    /// The color and icon size can also be configured by the `IconTheme` if the same property
    /// has a null value in [`IconButtonTheme`]. However, if any of the properties exist
    /// in both [`IconButtonTheme`] and `IconTheme`, `IconTheme` will be overridden.
    fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        let icon_theme = IconTheme::of(app, context);
        let is_default_size = icon_theme.size == IconThemeData::fallback().size;
        let default_color = match Theme::brightness_of(app, context) {
            Brightness::Light => K_DEFAULT_ICON_DARK_COLOR,
            Brightness::Dark => K_DEFAULT_ICON_LIGHT_COLOR,
        };
        let is_default_color = icon_theme
            .color
            .as_ref()
            .is_some_and(|color| *color == default_color);

        let mut icon_theme_style = IconButton::style_from();
        if !is_default_color && let Some(color) = &icon_theme.color {
            icon_theme_style = icon_theme_style.foreground_color(color.color());
        }
        if !is_default_size && let Some(size) = icon_theme.size {
            icon_theme_style = icon_theme_style.icon_size(size);
        }
        let icon_theme_style = icon_theme_style.build();

        Some(match IconButtonTheme::of(app, context).style {
            Some(style) => style.merge(Some(&icon_theme_style)),
            None => icon_theme_style,
        })
    }
}

impl StatefulWidget for IconButtonM3 {
    type State = ButtonStyleState<IconButtonM3>;

    fn key(&self) -> Option<&KeyRef> {
        self.button_style_button.key.as_ref()
    }

    fn create_state(&self) -> ButtonStyleState<IconButtonM3> {
        ButtonStyleState::new()
    }
}

impl Debug for IconButtonM3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.button_style_button, f)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Offset, Radius, TextDirection};
    use waft_foundation::Listenable;
    use waft_painting::{
        BorderRadius, BorderSide, OutlinedBorderRef, RoundedRectangleBorder, ShapeBorder,
        StadiumBorder,
    };
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        AnyElement, Builder, Center, Column, CrossAxisAlignment, Directionality, FocusNode,
        FocusTraversalGroup, GlobalKey, Icon, IconData, ReadingOrderTraversalPolicy, RichText, Row,
        StatefulBuilder, WidgetStates,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::icon_button_theme::IconButtonThemeData;
    use crate::icons::Icons;
    use crate::material::{Material, MaterialType};
    use crate::test_support::{Drawn, PaintInspection, material_app};
    use crate::theme_data::ThemeData;

    /// `IconButton::new` or one of its named constructors.
    type Constructor = fn(Option<Listener>, Icon) -> IconButton;

    /// The colour a case expects of a scheme.
    type ContainerColor = fn(&ColorScheme) -> Color;

    /// Dart's `ThemeData.from(colorScheme: const ColorScheme.light())`.
    fn m3_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow())
    }

    /// Dart's `ThemeData()`.
    fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder().build(&tester.cell.borrow())
    }

    fn m2_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    /// Dart's `wrap(child:, useMaterial3:)` at the bottom of `icon_button_test.dart`.
    fn wrap<K>(tester: &WidgetTester, child: impl IntoWidget<K>, use_material3: bool) -> WidgetRef {
        let policy = ReadingOrderTraversalPolicy::new(&mut tester.cell.borrow_mut())
            .into_erased::<waft_widgets::AnyFocusTraversalPolicy>();
        if use_material3 {
            material_app(
                m3_theme(tester),
                FocusTraversalGroup::new(Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(child),
                ))
                .policy(policy.clone())
                .into_widget(),
            )
        } else {
            FocusTraversalGroup::new(Directionality::new(
                TextDirection::Ltr,
                Material::new().child(Center::new().child(child)),
            ))
            .policy(policy.clone())
            .into_widget()
        }
    }

    fn noop() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn counter(count: &Rc<Cell<usize>>) -> Option<Listener> {
        let count = Rc::clone(count);
        Some(Listener::new(move |_app| count.set(count.get() + 1)))
    }

    fn icon(data: IconData) -> Icon {
        Icon::new(Some(data))
    }

    fn icon_button_element(tester: &WidgetTester) -> AnyElement {
        tester.find_by_type::<IconButton>()[0]
    }

    /// Dart's `tester.getSize(find.byType(IconButton))`.
    fn button_size(tester: &WidgetTester) -> Size {
        let render = tester.render_box_of(icon_button_element(tester));
        render.size(&tester.cell.borrow())
    }

    fn button_center(tester: &WidgetTester) -> Offset {
        let render = tester.render_box_of(icon_button_element(tester));
        tester.global_rect(render).center()
    }

    /// The elements of `find.byIcon(data)`.
    fn find_icon(tester: &WidgetTester, data: &IconData) -> Vec<AnyElement> {
        let found = tester.find_by_type::<Icon>();
        found
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |icon: &Icon| icon.icon.as_ref() == Some(data))
            })
            .collect()
    }

    fn icon_size(tester: &WidgetTester) -> Size {
        let render = tester.render_box_of(tester.find_by_type::<Icon>()[0]);
        render.size(&tester.cell.borrow())
    }

    /// Dart's `_iconStyle(tester, icon)?.color`.
    fn icon_color(tester: &WidgetTester) -> Option<Color> {
        let rich_text = tester.find_by_type::<RichText>()[0];
        tester.widget(rich_text, |rich_text: &RichText| {
            rich_text
                .text
                .style()
                .and_then(|style| style.color.as_ref().map(|color| color.color()))
        })
    }

    /// Dart's `IconTheme.of(tester.element(find.byIcon(icon))).color`.
    fn icon_theme_color(tester: &WidgetTester) -> Option<Color> {
        let element = tester.find_by_type::<Icon>()[0];
        let mut app = tester.cell.borrow_mut();
        IconTheme::of(&mut app, element)
            .color
            .map(|color| color.color())
    }

    /// The `Material` below the `IconButton`.
    fn button_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let button = icon_button_element(tester);
        let material = tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&button))
            .expect("a Material below the IconButton");
        tester.widget(material, read)
    }

    fn button_material_size(tester: &WidgetTester) -> Size {
        let button = icon_button_element(tester);
        let material = tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&button))
            .expect("a Material below the IconButton");
        let render = tester.render_box_of(material);
        render.size(&tester.cell.borrow())
    }

    fn material_color(tester: &WidgetTester) -> Option<Color> {
        button_material(tester, |material| material.color)
    }

    fn material_shape_is(tester: &WidgetTester, shape: &dyn ShapeBorder) -> bool {
        button_material(tester, |material| material.shape.as_deref() == Some(shape))
    }

    fn stadium() -> StadiumBorder {
        StadiumBorder::default()
    }

    fn stadium_with(color: Color) -> StadiumBorder {
        StadiumBorder::new(BorderSide {
            color,
            ..BorderSide::default()
        })
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(radius)).into(),
        )
    }

    /// The `Material` checks every `IconButton defaults - M3` case repeats.
    fn check_material(tester: &WidgetTester, color: Color, shape: &dyn ShapeBorder) {
        button_material(tester, |material| {
            assert_eq!(material.animation_duration, Duration::from_millis(200));
            assert!(!material.border_on_foreground);
            assert!(material.border_radius.is_none());
            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.color, Some(color));
            assert_eq!(material.elevation, 0.0);
            assert_eq!(material.shadow_color, Some(Colors::TRANSPARENT));
            assert!(material.shape.as_deref() == Some(shape));
            assert!(material.text_style.is_none());
            assert_eq!(material.r#type, MaterialType::Button);
        });
    }

    fn check_icon_alignment_and_size(tester: &WidgetTester) {
        let icon = tester.find_by_type::<Icon>()[0];
        let align = tester.ancestor_of_type::<Align>(icon);
        tester.widget(align, |align: &Align| {
            assert!(align.alignment == AlignmentGeometry::CENTER);
        });
        assert_eq!(icon_size(tester), Size::new(24.0, 24.0));
    }

    /// A tap that a frame starts and 100ms carries, then lifted and settled.
    fn press_and_release(tester: &mut WidgetTester) {
        let center = button_center(tester);
        let mut touch = TestPointer::touch(1);
        tester.down(&mut touch, center);
        tester.pump();
        tester.pump_by(Duration::from_millis(100));
        tester.up(&touch);
        tester.pump_and_settle();
    }

    /// The colours of the rectangles the first material's ink draws: Dart's
    /// `paints..rect(color:)` on `_RenderInkFeatures`.
    fn ink_rects(tester: &WidgetTester) -> Vec<u32> {
        tester
            .ink_drawn()
            .into_iter()
            .filter_map(|drawn| match drawn {
                Drawn::Rect(color, _) => Some(color),
                _ => None,
            })
            .collect()
    }

    fn argb(color: Color) -> u32 {
        color.value()
    }

    fn active_cursor(tester: &WidgetTester) -> MouseCursorRef {
        let mut app = tester.cell.borrow_mut();
        let tracker = waft_rendering::RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, 1)
            .expect("the mouse is over the view")
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    fn states_value(
        tester: &WidgetTester,
        controller: Handle<MaterialStatesController>,
    ) -> WidgetStates {
        tester.cell.borrow().get(controller).value().clone()
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    // 'test icon is findable by key'
    #[test]
    fn an_icon_button_is_findable_by_key() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::link())).key(key.clone().into()),
                true,
            );
            tester.pump_widget(widget);
            assert!(key.current_context(&tester.cell.borrow_mut()).is_some());
        });
    }

    // 'test default icon buttons are sized up to 48'
    #[test]
    fn a_default_icon_button_is_sized_up_to_48() {
        test_widgets(|tester| {
            let pressed = Rc::new(Cell::new(0));
            let widget = wrap(
                tester,
                IconButton::new(counter(&pressed), icon(Icons::link())),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            let center = button_center(tester);
            tester.tap(center, 1);
            assert_eq!(pressed.get(), 1);
        });
    }

    // 'test small icons are sized up to 48dp'
    #[test]
    fn a_small_icon_is_sized_up_to_48() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::link())).icon_size(10.0),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));
        });
    }

    // 'test icons can be small when total size is >48dp'
    #[test]
    fn a_small_icon_with_large_padding_is_larger_than_48() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::link()))
                    .icon_size(10.0)
                    .padding(EdgeInsets::all(30.0).into()),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(70.0, 70.0));
        });
    }

    // 'when both iconSize and IconTheme.of(context).size are null, size falls back to 24.0'
    #[test]
    fn the_icon_size_falls_back_to_24() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let widget = wrap(
                tester,
                IconTheme::new(
                    IconThemeData::new(),
                    IconButton::new(noop(), icon(Icons::link())).focus_node(focus_node),
                ),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(24.0, 24.0));
        });
    }

    // 'when null, iconSize is overridden by closest IconTheme'
    #[test]
    fn without_an_icon_size_the_closest_icon_theme_sizes_the_icon() {
        test_widgets(|tester| {
            let button = || IconButton::new(noop(), icon(Icons::link()));
            let themed = |tester: &WidgetTester, size: f64, child: WidgetRef| {
                let theme = ThemeData::builder()
                    .icon_theme(IconThemeData::new().size(size))
                    .build(&tester.cell.borrow());
                Theme::new(theme, child).into_widget()
            };

            let widget = wrap(
                tester,
                IconTheme::new(IconThemeData::new().size(10.0), button()),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(10.0, 10.0));

            let child = themed(tester, 10.0, button().into_widget());
            let widget = wrap(tester, child, true);
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(10.0, 10.0));

            let child = themed(
                tester,
                20.0,
                IconTheme::new(IconThemeData::new().size(10.0), button()).into_widget(),
            );
            let widget = wrap(tester, child, true);
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(10.0, 10.0));

            let child = IconTheme::new(
                IconThemeData::new().size(20.0),
                themed(tester, 10.0, button().into_widget()),
            );
            let widget = wrap(tester, child, true);
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(10.0, 10.0));
        });
    }

    // 'when non-null, iconSize precedes IconTheme.of(context).size'
    #[test]
    fn an_icon_size_precedes_the_icon_theme_size() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconTheme::new(
                    IconThemeData::new().size(30.0),
                    IconButton::new(noop(), icon(Icons::link())).icon_size(10.0),
                ),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(icon_size(tester), Size::new(10.0, 10.0));
        });
    }

    // 'Small icons with non-null constraints can be <48dp for M2, but =48dp for M3'
    #[test]
    fn small_icons_with_constraints_are_smaller_than_48_in_m2_only() {
        test_widgets(|tester| {
            for (material3, expected) in [(true, 48.0), (false, 26.0)] {
                let button = IconButton::new(noop(), icon(Icons::link()))
                    .icon_size(10.0)
                    .constraints(BoxConstraints::new());
                let widget = if material3 {
                    wrap(tester, button, true)
                } else {
                    // Dart's `wrap` gives no theme, whose fallback is Material 3 now; the
                    // Material 2 sizes need a Material 2 theme.
                    let theme = m2_theme(tester);
                    wrap(tester, Theme::new(theme, button), false)
                };
                tester.pump_widget(widget);
                // By default IconButton has a padding of 8.0 on all sides, so both
                // width and height are 10.0 + 2 * 8.0 = 26.0
                // M3 IconButton is a subclass of ButtonStyleButton which has a minimum
                // Size(48.0, 48.0).
                assert_eq!(button_size(tester), Size::new(expected, expected));
                assert_eq!(icon_size(tester), Size::new(10.0, 10.0));
            }
        });
    }

    // 'Small icons with non-null constraints and custom padding can be <48dp'
    #[test]
    fn small_icons_with_constraints_and_padding_are_smaller_than_48_in_m2_only() {
        test_widgets(|tester| {
            for (material3, expected) in [(true, 48.0), (false, 16.0)] {
                let button = IconButton::new(noop(), icon(Icons::link()))
                    .icon_size(10.0)
                    .padding(EdgeInsets::all(3.0).into())
                    .constraints(BoxConstraints::new());
                let widget = if material3 {
                    wrap(tester, button, true)
                } else {
                    // Dart's `wrap` gives no theme, whose fallback is Material 3 now; the
                    // Material 2 sizes need a Material 2 theme.
                    let theme = m2_theme(tester);
                    wrap(tester, Theme::new(theme, button), false)
                };
                tester.pump_widget(widget);
                assert_eq!(button_size(tester), Size::new(expected, expected));
                assert_eq!(icon_size(tester), Size::new(10.0, 10.0));
            }
        });
    }

    // 'Small icons comply with VisualDensity requirements'
    #[test]
    fn small_icons_follow_the_visual_density() {
        test_widgets(|tester| {
            for (material3, expected) in [
                (true, Size::new(52.0, 44.0)),
                (false, Size::new(36.0, 28.0)),
            ] {
                let density = VisualDensity::new(1.0, -1.0);
                let theme = if material3 {
                    ThemeData::builder()
                        .icon_button_theme(
                            IconButtonThemeData::new()
                                .style(IconButton::style_from().visual_density(density).build()),
                        )
                        .build(&tester.cell.borrow())
                } else {
                    ThemeData::builder()
                        .use_material3(false)
                        .visual_density(density)
                        .build(&tester.cell.borrow())
                };
                let widget = wrap(
                    tester,
                    Theme::new(
                        theme,
                        IconButton::new(noop(), icon(Icons::link()))
                            .icon_size(10.0)
                            .constraints(BoxConstraints::new().min_width(32.0).min_height(32.0)),
                    ),
                    material3,
                );
                tester.pump_widget(widget);
                // VisualDensity(horizontal: 1, vertical: -1) increases the icon's
                // width by 4 pixels and decreases its height by 4 pixels, giving
                // final width 32.0 + 4.0 = 36.0 and
                // final height 32.0 - 4.0 = 28.0
                assert_eq!(button_size(tester), expected);
            }
        });
    }

    // 'test default icon buttons are constrained'
    #[test]
    fn a_default_icon_button_is_as_large_as_its_icon() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit()))
                    .padding(EdgeInsets::ZERO.into())
                    .icon_size(80.0),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(80.0, 80.0));
        });
    }

    // 'test default icon buttons can be stretched if specified'
    #[test]
    fn a_default_icon_button_stretches_in_a_stretching_row() {
        test_widgets(|tester| {
            let row = || {
                Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children([IconButton::new(noop(), icon(Icons::ac_unit())).into_widget()])
            };
            tester.pump_widget(
                Directionality::new(TextDirection::Ltr, Material::new().child(row())).into_widget(),
            );
            assert_eq!(button_size(tester), Size::new(48.0, 600.0));

            // Test for Material 3
            let widget = material_app(
                m3_theme(tester),
                Directionality::new(TextDirection::Ltr, row()).into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(48.0, 600.0));
        });
    }

    // 'test default padding'
    #[test]
    fn the_default_padding_is_8() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit())).icon_size(80.0),
                true,
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(96.0, 96.0));
        });
    }

    // 'test default alignment'
    #[test]
    fn the_default_alignment_is_center() {
        test_widgets(|tester| {
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit())).icon_size(80.0),
                true,
            );
            tester.pump_widget(widget);
            let icon = tester.find_by_type::<Icon>()[0];
            let align = tester.ancestor_of_type::<Align>(icon);
            tester.widget(align, |align: &Align| {
                assert!(align.alignment == AlignmentGeometry::CENTER);
            });
        });
    }

    // 'test tooltip'
    #[test]
    fn a_tooltip_is_shown_on_hover_and_the_button_still_taps() {
        test_widgets(|tester| {
            let pressed = Rc::new(Cell::new(0));
            let build = |tester: &WidgetTester, tooltip: Option<&str>| {
                let mut button = IconButton::new(counter(&pressed), icon(Icons::ac_unit()));
                if let Some(tooltip) = tooltip {
                    button = button.tooltip(tooltip);
                }
                material_app(
                    m3_theme(tester),
                    Material::new()
                        .child(Center::new().child(button))
                        .into_widget(),
                )
            };
            let widget = build(tester, None);
            tester.pump_widget(widget);
            assert!(tester.find_by_type::<Tooltip>().is_empty());

            // Clear the widget tree.
            tester.pump_widget(
                SizedBox::shrink()
                    .key(waft_foundation::UniqueKey::new().into())
                    .into_widget(),
            );

            let widget = build(tester, Some("Test tooltip"));
            tester.pump_widget(widget);
            assert_eq!(tester.find_by_type::<Tooltip>().len(), 1);

            let center = button_center(tester);
            tester.tap(center, 1);
            assert_eq!(pressed.get(), 1);

            // Hovering over the button should show the tooltip.
            let mut mouse = TestPointer::mouse(2, 0);
            tester.add_pointer(&mut mouse, center);
            tester.pump();
            assert_eq!(tester.find_text("Test tooltip").len(), 1);
        });
    }

    // 'IconButton AppBar size'
    #[test]
    fn an_icon_button_in_an_app_bar_is_sized_and_centred_in_the_toolbar() {
        test_widgets(|tester| {
            let theme = m3_theme(tester);
            let material3 = theme.use_material3;
            let widget = material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new().actions(vec![
                            IconButton::new(noop(), icon(Icons::ac_unit()))
                                .padding(EdgeInsets::ZERO.into())
                                .into_widget(),
                        ]),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);

            let bar = tester.find_by_type::<crate::app_bar::AppBar>()[0];
            let bar_size = tester.render_box_of(bar).size(&tester.cell.borrow());
            let icon_size = button_size(tester);
            assert_eq!(
                icon_size.height(),
                if material3 { 48.0 } else { bar_size.height() }
            );
            assert_eq!(button_center(tester).dy(), 28.0);
        });
    }

    // 'IconButton with explicit splashColor and highlightColor - M2'
    #[test]
    fn m2_splash_and_highlight_colors_come_from_the_button_then_the_theme() {
        test_widgets(|tester| {
            let direct_splash_color = Color::new(0xFF00000F);
            let direct_highlight_color = Color::new(0xFF0000F0);
            let circles = |tester: &WidgetTester| -> Vec<u32> {
                tester
                    .ink_drawn()
                    .into_iter()
                    .filter_map(|drawn| match drawn {
                        Drawn::Path(color, _) => Some(color),
                        _ => None,
                    })
                    .collect()
            };
            let themed = |tester: &WidgetTester,
                          highlight: Option<Color>,
                          splash: Option<Color>,
                          child: WidgetRef| {
                let mut builder = ThemeData::builder().use_material3(false);
                if let Some(highlight) = highlight {
                    builder = builder.highlight_color(highlight);
                }
                if let Some(splash) = splash {
                    builder = builder.splash_color(splash);
                }
                Theme::new(builder.build(&tester.cell.borrow()), child).into_widget()
            };

            let button_widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::android()))
                    .splash_color(direct_splash_color)
                    .highlight_color(direct_highlight_color),
                false,
            );
            let widget = themed(tester, None, None, button_widget);
            tester.pump_widget(widget);

            let center = button_center(tester);
            let mut touch = TestPointer::touch(1);
            tester.down(&mut touch, center);
            tester.pump(); // start gesture
            tester.pump_by(Duration::from_millis(200)); // wait for splash to be well under way
            assert_eq!(
                circles(tester),
                vec![argb(direct_splash_color), argb(direct_highlight_color)]
            );

            let theme_splash_color1 = Color::new(0xFF000F00);
            let theme_highlight_color1 = Color::new(0xFF00FF00);
            let button_widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::android())),
                false,
            );
            let widget = themed(
                tester,
                Some(theme_highlight_color1),
                Some(theme_splash_color1),
                button_widget.clone(),
            );
            tester.pump_widget(widget);
            assert_eq!(
                circles(tester),
                vec![argb(theme_splash_color1), argb(theme_highlight_color1)]
            );

            let theme_splash_color2 = Color::new(0xFF002200);
            let theme_highlight_color2 = Color::new(0xFF001100);
            // same widget, so does not get updated because of us
            let widget = themed(
                tester,
                Some(theme_highlight_color2),
                Some(theme_splash_color2),
                button_widget,
            );
            tester.pump_widget(widget);
            assert_eq!(
                circles(tester),
                vec![argb(theme_splash_color2), argb(theme_highlight_color2)]
            );

            tester.up(&touch);
        });
    }

    // 'IconButton with explicit splash radius - M2'
    #[test]
    fn m2_an_explicit_splash_radius_sizes_the_splash() {
        test_widgets(|tester| {
            let splash_radius = 30.0;
            let widget = material_app(
                m2_theme(tester),
                Material::new()
                    .child(
                        Center::new().child(
                            IconButton::new(noop(), icon(Icons::android()))
                                .splash_radius(splash_radius),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);

            let center = button_center(tester);
            let mut touch = TestPointer::touch(1);
            tester.down(&mut touch, center);
            tester.pump(); // Start gesture.
            tester.pump_by(Duration::from_millis(1000)); // Wait for splash to be well under way.

            let splash = tester
                .ink_drawn()
                .into_iter()
                .find_map(|drawn| match drawn {
                    Drawn::Path(_, bounds) => Some(bounds),
                    _ => None,
                });
            let splash = splash.expect("a splash circle");
            assert_eq!(splash.width(), splash_radius * 2.0);
            assert_eq!(splash.height(), splash_radius * 2.0);

            tester.up(&touch);
        });
    }

    // 'IconButton loses focus when disabled.'
    #[test]
    fn a_button_loses_focus_when_disabled() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let widget = wrap(
                tester,
                IconButton::new(noop(), icon(Icons::link()))
                    .focus_node(focus_node.clone())
                    .autofocus(true),
                true,
            );
            tester.pump_widget(widget);
            tester.pump();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

            let widget = wrap(
                tester,
                IconButton::new(None, icon(Icons::link()))
                    .focus_node(focus_node.clone())
                    .autofocus(true),
                true,
            );
            tester.pump_widget(widget);
            tester.pump();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    // 'IconButton keeps focus when disabled in directional navigation mode.'
    #[test]
    fn a_button_keeps_focus_when_disabled_in_directional_navigation() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let frame = |tester: &WidgetTester, on_pressed: Option<Listener>| {
                wrap(
                    tester,
                    waft_widgets::MediaQuery::new(
                        waft_widgets::MediaQueryData::default()
                            .navigation_mode(waft_widgets::NavigationMode::Directional),
                        IconButton::new(on_pressed, icon(Icons::link()))
                            .focus_node(focus_node.clone())
                            .autofocus(true),
                    )
                    .into_widget(),
                    true,
                )
            };
            let widget = frame(tester, noop());
            tester.pump_widget(widget);
            tester.pump();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

            let widget = frame(tester, None);
            tester.pump_widget(widget);
            tester.pump();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    // "Disabled IconButton can't be traversed to when disabled."
    #[test]
    fn a_disabled_button_cannot_be_traversed_to() {
        test_widgets(|tester| {
            let focus_node1 = focus_node(tester);
            let focus_node2 = focus_node(tester);
            let widget = wrap(
                tester,
                Column::new().children([
                    IconButton::new(noop(), icon(Icons::link()))
                        .focus_node(focus_node1.clone())
                        .autofocus(true)
                        .into_widget(),
                    IconButton::new(None, icon(Icons::link()))
                        .focus_node(focus_node2.clone())
                        .into_widget(),
                ]),
                true,
            );
            tester.pump_widget(widget);
            tester.pump();

            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));

            assert!(!focus_node1.next_focus(&mut tester.cell.borrow_mut()));
            tester.pump();

            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));
        });
    }

    // 'IconButton responds to density changes.'
    #[test]
    fn the_button_responds_to_density_changes() {
        test_widgets(|tester| {
            let build = |tester: &mut WidgetTester, visual_density: VisualDensity| {
                let widget = material_app(
                    m3_theme(tester),
                    Material::new()
                        .child(
                            Center::new().child(
                                IconButton::new(noop(), icon(Icons::play_arrow()))
                                    .visual_density(visual_density),
                            ),
                        )
                        .into_widget(),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle();
            };

            build(tester, VisualDensity::STANDARD);
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            build(tester, VisualDensity::new(3.0, 3.0));
            assert_eq!(button_size(tester), Size::new(64.0, 64.0));

            build(tester, VisualDensity::new(-3.0, -3.0));
            // IconButton is a subclass of ButtonStyleButton in Material 3, so the negative
            // visualDensity cannot be applied to horizontal padding.
            // The size of the Button with padding is (24 + 8 + 8, 24) -> (40, 24)
            // minSize of M3 IconButton is (48 - 12, 48 - 12) -> (36, 36)
            // So, the button size in Material 3 is (40, 36)
            assert_eq!(button_size(tester), Size::new(40.0, 36.0));

            build(tester, VisualDensity::new(3.0, -3.0));
            assert_eq!(button_size(tester), Size::new(64.0, 36.0));
        });
    }

    fn cursor_frame(tester: &WidgetTester, button: IconButton) -> WidgetRef {
        material_app(
            m3_theme(tester),
            Material::new()
                .child(Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(button),
                ))
                .into_widget(),
        )
    }

    // 'IconButton.mouseCursor changes cursor on hover'
    #[test]
    fn the_mouse_cursor_changes_on_hover() {
        test_widgets(|tester| {
            // Test argument works
            let widget = cursor_frame(
                tester,
                IconButton::new(noop(), icon(Icons::play_arrow()))
                    .mouse_cursor(SystemMouseCursors::FORBIDDEN.into()),
            );
            tester.pump_widget(widget);
            let mut mouse = TestPointer::mouse(1, 0);
            let center = button_center(tester);
            tester.add_pointer(&mut mouse, center);
            tester.pump();
            let forbidden: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();
            assert!(*active_cursor(tester) == *forbidden);

            // Test default is click on web, basic on non-web
            let widget = cursor_frame(tester, IconButton::new(noop(), icon(Icons::play_arrow())));
            tester.pump_widget(widget);
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            assert!(*active_cursor(tester) == *basic);
        });
    }

    // 'disabled IconButton has basic mouse cursor'
    #[test]
    fn a_disabled_button_has_the_basic_mouse_cursor() {
        test_widgets(|tester| {
            let widget = cursor_frame(tester, IconButton::new(None, icon(Icons::play_arrow())));
            tester.pump_widget(widget);
            let mut mouse = TestPointer::mouse(1, 0);
            let center = button_center(tester);
            tester.add_pointer(&mut mouse, center);
            tester.pump();
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            assert!(*active_cursor(tester) == *basic);
        });
    }

    // 'IconButton.mouseCursor overrides implicit setting of mouse cursor'
    #[test]
    fn the_mouse_cursor_overrides_the_implicit_one() {
        test_widgets(|tester| {
            let none: MouseCursorRef = SystemMouseCursors::NONE.into();
            let widget = cursor_frame(
                tester,
                IconButton::new(None, icon(Icons::play_arrow())).mouse_cursor(none.clone()),
            );
            tester.pump_widget(widget);
            let mut mouse = TestPointer::mouse(1, 0);
            let center = button_center(tester);
            tester.add_pointer(&mut mouse, center);
            tester.pump();
            assert!(*active_cursor(tester) == *none);

            let widget = cursor_frame(
                tester,
                IconButton::new(noop(), icon(Icons::play_arrow())).mouse_cursor(none.clone()),
            );
            tester.pump_widget(widget);
            assert!(*active_cursor(tester) == *none);
        });
    }

    // 'IconTheme opacity test'; Dart's `Scaffold` is the `Material` it provides.
    #[test]
    fn m2_the_icon_color_takes_the_icon_theme_opacity() {
        test_widgets(|tester| {
            let theme = ThemeData::from(ColorScheme::light())
                .use_material3(false)
                .build(&tester.cell.borrow());
            let purple = Colors::PURPLE.primary();
            let button = || IconButton::new(noop(), icon(Icons::add())).color(purple);

            let widget = material_app(
                theme.clone(),
                Material::new()
                    .child(Center::new().child(button()))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(purple));

            let widget = material_app(
                theme,
                Material::new()
                    .child(Center::new().child(IconTheme::merge(
                        None,
                        IconThemeData::new().opacity(0.5),
                        button(),
                    )))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(purple.with_opacity(0.5)));
        });
    }

    // 'IconButton defaults - M3'
    #[test]
    fn m3_standard_defaults() {
        test_widgets(|tester| {
            let widget = material_app(
                m3_theme(tester),
                Center::new()
                    .child(IconButton::new(noop(), icon(Icons::ac_unit())))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            check_material(tester, Colors::TRANSPARENT, &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            // No change vs enabled and not pressed.
            check_material(tester, Colors::TRANSPARENT, &stadium());

            // Disabled IconButton
            let widget = material_app(
                m3_theme(tester),
                Center::new()
                    .child(IconButton::new(None, icon(Icons::ac_unit())))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            check_material(tester, Colors::TRANSPARENT, &stadium());
        });
    }

    /// Dart's `... default overlayColor resolves pressed state` cases: hovered, then pressed,
    /// then focused, each the overlay `colors` gives for the state.
    fn check_overlay_by_state(
        tester: &mut WidgetTester,
        button: Constructor,
        colors: fn(&ColorScheme) -> (Color, Color, Color),
    ) {
        let focus_node = focus_node(tester);
        let theme = default_theme(tester);
        let (hovered, pressed, focused) = colors(&theme.color_scheme);
        let widget = material_app(
            theme,
            Center::new()
                .child(Builder::new({
                    let focus_node = focus_node.clone();
                    move |_app, _context| {
                        button(noop(), icon(Icons::add()))
                            .focus_node(focus_node.clone())
                            .into_widget()
                    }
                }))
                .into_widget(),
        );
        tester.pump_widget(widget);

        // Hovered.
        let center = button_center(tester);
        let mut mouse = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();
        assert_eq!(ink_rects(tester), vec![argb(hovered)]);

        // Highlighted (pressed).
        tester.send(&mouse, waft_embedder::PointerChange::Down);
        tester.pump_and_settle();
        assert_eq!(ink_rects(tester)[1], argb(pressed));
        // Remove pressed and hovered states
        tester.up(&mouse);
        tester.pump_and_settle();
        tester.move_to(&mut mouse, Offset::new(0.0, 50.0), false);
        tester.pump_and_settle();

        // Focused.
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        assert_eq!(ink_rects(tester), vec![argb(focused)]);
    }

    // 'IconButton default overlayColor resolves pressed state'
    #[test]
    fn m3_standard_overlay_resolves_by_state() {
        test_widgets(|tester| {
            check_overlay_by_state(tester, IconButton::new, |colors| {
                (
                    colors.on_surface_variant().with_opacity(0.08),
                    colors.on_surface_variant().with_opacity(0.1),
                    colors.on_surface_variant().with_opacity(0.1),
                )
            });
        });
    }

    // 'IconButton.fill default overlayColor resolves pressed state'
    #[test]
    fn m3_filled_overlay_resolves_by_state() {
        test_widgets(|tester| {
            check_overlay_by_state(tester, IconButton::filled, |colors| {
                (
                    colors.on_primary.with_opacity(0.08),
                    colors.on_primary.with_opacity(0.1),
                    colors.on_primary.with_opacity(0.1),
                )
            });
        });
    }

    // 'IconButton.filledTonal default overlayColor resolves pressed state'
    #[test]
    fn m3_filled_tonal_overlay_resolves_by_state() {
        test_widgets(|tester| {
            check_overlay_by_state(tester, IconButton::filled_tonal, |colors| {
                (
                    colors.on_secondary_container().with_opacity(0.08),
                    colors.on_secondary_container().with_opacity(0.1),
                    colors.on_secondary_container().with_opacity(0.1),
                )
            });
        });
    }

    // 'IconButton.outlined default overlayColor resolves pressed state'
    #[test]
    fn m3_outlined_overlay_resolves_by_state() {
        test_widgets(|tester| {
            check_overlay_by_state(tester, IconButton::outlined, |colors| {
                (
                    colors.on_surface_variant().with_opacity(0.08),
                    colors.on_surface.with_opacity(0.1),
                    colors.on_surface_variant().with_opacity(0.08),
                )
            });
        });
    }

    fn pump_m3(tester: &mut WidgetTester, button: IconButton) {
        let widget = material_app(m3_theme(tester), Center::new().child(button).into_widget());
        tester.pump_widget(widget);
        tester.pump_and_settle();
    }

    // 'IconButton.fill defaults - M3'
    #[test]
    fn m3_filled_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(tester, IconButton::filled(noop(), icon(Icons::ac_unit())));
            assert_eq!(icon_color(tester), Some(colors.on_primary));
            check_material(tester, colors.primary, &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            // No change vs enabled and not pressed.
            check_material(tester, colors.primary, &stadium());

            // Disabled IconButton
            pump_m3(tester, IconButton::filled(None, icon(Icons::ac_unit())));
            check_material(tester, colors.on_surface.with_opacity(0.12), &stadium());
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'Toggleable IconButton.fill defaults - M3'
    #[test]
    fn m3_toggleable_filled_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(
                tester,
                IconButton::filled(noop(), icon(Icons::ac_unit())).is_selected(true),
            );
            assert_eq!(icon_color(tester), Some(colors.on_primary));
            check_material(tester, colors.primary, &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            // No change vs enabled and not pressed.
            check_material(tester, colors.primary, &stadium());

            // Enabled unselected IconButton
            pump_m3(
                tester,
                IconButton::filled(noop(), icon(Icons::ac_unit())).is_selected(false),
            );
            check_material(tester, colors.surface_variant(), &stadium());
            assert_eq!(icon_color(tester), Some(colors.primary));

            // Disabled IconButton
            pump_m3(
                tester,
                IconButton::filled(None, icon(Icons::ac_unit())).is_selected(true),
            );
            check_material(tester, colors.on_surface.with_opacity(0.12), &stadium());
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'IconButton.filledTonal defaults - M3'
    #[test]
    fn m3_filled_tonal_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(
                tester,
                IconButton::filled_tonal(noop(), icon(Icons::ac_unit())),
            );
            assert_eq!(icon_color(tester), Some(colors.on_secondary_container()));
            check_material(tester, colors.secondary_container(), &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            check_material(tester, colors.secondary_container(), &stadium());

            pump_m3(
                tester,
                IconButton::filled_tonal(None, icon(Icons::ac_unit())),
            );
            check_material(tester, colors.on_surface.with_opacity(0.12), &stadium());
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'Toggleable IconButton.filledTonal defaults - M3'
    #[test]
    fn m3_toggleable_filled_tonal_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(
                tester,
                IconButton::filled_tonal(noop(), icon(Icons::ac_unit())).is_selected(true),
            );
            assert_eq!(icon_color(tester), Some(colors.on_secondary_container()));
            check_material(tester, colors.secondary_container(), &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            check_material(tester, colors.secondary_container(), &stadium());

            pump_m3(
                tester,
                IconButton::filled_tonal(noop(), icon(Icons::ac_unit())).is_selected(false),
            );
            check_material(tester, colors.surface_variant(), &stadium());
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));

            pump_m3(
                tester,
                IconButton::filled_tonal(None, icon(Icons::ac_unit())).is_selected(true),
            );
            check_material(tester, colors.on_surface.with_opacity(0.12), &stadium());
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'IconButton.outlined defaults - M3'
    #[test]
    fn m3_outlined_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(tester, IconButton::outlined(noop(), icon(Icons::ac_unit())));
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            check_material(tester, Colors::TRANSPARENT, &stadium_with(colors.outline()));
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            check_material(tester, Colors::TRANSPARENT, &stadium_with(colors.outline()));

            pump_m3(tester, IconButton::outlined(None, icon(Icons::ac_unit())));
            check_material(
                tester,
                Colors::TRANSPARENT,
                &stadium_with(colors.on_surface.with_opacity(0.12)),
            );
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'Toggleable IconButton.outlined defaults - M3'
    #[test]
    fn m3_toggleable_outlined_defaults() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            pump_m3(
                tester,
                IconButton::outlined(noop(), icon(Icons::ac_unit())).is_selected(true),
            );
            assert_eq!(icon_color(tester), Some(colors.on_inverse_surface()));
            check_material(tester, colors.inverse_surface(), &stadium());
            check_icon_alignment_and_size(tester);

            press_and_release(tester);
            check_material(tester, colors.inverse_surface(), &stadium());

            pump_m3(
                tester,
                IconButton::outlined(noop(), icon(Icons::ac_unit())).is_selected(false),
            );
            check_material(tester, Colors::TRANSPARENT, &stadium_with(colors.outline()));
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));

            pump_m3(
                tester,
                IconButton::outlined(None, icon(Icons::ac_unit())).is_selected(true),
            );
            check_material(tester, colors.on_surface.with_opacity(0.12), &stadium());
            assert_eq!(
                icon_color(tester),
                Some(colors.on_surface.with_opacity(0.38))
            );
        });
    }

    // 'IconButton uses stateful color for icon color in different states - M3'
    #[test]
    fn m3_the_icon_color_follows_a_stateful_foreground_color() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let pressed_color = Color::new(0x00000001);
            let hover_color = Color::new(0x00000002);
            let focused_color = Color::new(0x00000003);
            let default_color = Color::new(0x00000004);
            let selected_color = Color::new(0x00000005);
            let is_selected = Rc::new(Cell::new(false));

            let foreground =
                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(pressed_color);
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(hover_color);
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(focused_color);
                    }
                    if states.contains(&WidgetState::Selected) {
                        return Some(selected_color);
                    }
                    Some(default_color)
                });
            let widget = material_app(
                m3_theme(tester),
                StatefulBuilder::new({
                    let focus_node = focus_node.clone();
                    move |_app, _context, set_state| {
                        let is_selected_now = Rc::clone(&is_selected);
                        Center::new()
                            .child(
                                IconButton::new(
                                    Some(Listener::new(move |app| {
                                        let is_selected = Rc::clone(&is_selected_now);
                                        set_state(app, &mut |_app| {
                                            is_selected.set(!is_selected.get())
                                        });
                                    })),
                                    icon(Icons::ac_unit()),
                                )
                                .style(ButtonStyle::new().foreground_color(Rc::clone(&foreground)))
                                .is_selected(is_selected.get())
                                .focus_node(focus_node.clone()),
                            )
                            .into_widget()
                    }
                })
                .into_widget(),
            );
            tester.pump_widget(widget);

            // Default, not disabled.
            assert_eq!(icon_color(tester), Some(default_color));

            // Selected
            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();
            assert_eq!(icon_color(tester), Some(selected_color));

            // Focused.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(icon_color(tester), Some(focused_color));

            // Hovered.
            let mut mouse = TestPointer::mouse(2, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(&mut mouse, center, false);
            tester.pump_and_settle();
            assert_eq!(icon_color(tester), Some(hover_color));

            // Highlighted (pressed).
            tester.send(&mouse, waft_embedder::PointerChange::Down);
            tester.pump(); // Start the splash and highlight animations.
            tester.pump_by(Duration::from_millis(800)); // Wait for splash and highlight to be well under way.
            assert_eq!(icon_color(tester), Some(pressed_color));
        });
    }

    // 'IconButton size is configurable by ThemeData.materialTapTargetSize - M3'
    #[test]
    fn m3_the_theme_tap_target_size_pads_the_button() {
        test_widgets(|tester| {
            let build = |tester: &WidgetTester, tap_target_size: MaterialTapTargetSize| {
                let theme = ThemeData::builder()
                    .material_tap_target_size(tap_target_size)
                    .build(&tester.cell.borrow());
                Theme::new(
                    theme,
                    Directionality::new(
                        TextDirection::Ltr,
                        Center::new().child(
                            IconButton::new(noop(), icon(Icons::ac_unit())).style(
                                IconButton::style_from()
                                    .minimum_size(Size::new(40.0, 40.0))
                                    .build(),
                            ),
                        ),
                    ),
                )
                .into_widget()
            };
            let widget = build(tester, MaterialTapTargetSize::Padded);
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            let widget = build(tester, MaterialTapTargetSize::ShrinkWrap);
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::new(40.0, 40.0));
        });
    }

    // 'Override IconButton default padding - M3'
    #[test]
    fn m3_the_padding_is_overridden_by_the_button_then_its_style() {
        test_widgets(|tester| {
            let padding_of = |tester: &WidgetTester| {
                let button = icon_button_element(tester);
                let padding = tester
                    .find_by_type::<Padding>()
                    .into_iter()
                    .find(|padding| tester.ancestors(*padding).contains(&button))
                    .expect("a Padding below the IconButton");
                tester.widget(padding, |padding: &Padding| padding.padding)
            };

            // Use [IconButton]'s padding property to override default value.
            pump_m3(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit()))
                    .padding(EdgeInsets::all(20.0).into()),
            );
            assert!(padding_of(tester) == EdgeInsets::all(20.0).into());

            // Use [IconButton.style]'s padding property to override default value.
            pump_m3(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit())).style(
                    IconButton::style_from()
                        .padding(EdgeInsets::all(20.0).into())
                        .build(),
                ),
            );
            assert!(padding_of(tester) == EdgeInsets::all(20.0).into());

            // [IconButton.style]'s padding will override [IconButton]'s padding if both
            // values are not null.
            pump_m3(
                tester,
                IconButton::new(noop(), icon(Icons::ac_unit()))
                    .padding(EdgeInsets::all(15.0).into())
                    .style(
                        IconButton::style_from()
                            .padding(EdgeInsets::all(22.0).into())
                            .build(),
                    ),
            );
            assert!(padding_of(tester) == EdgeInsets::all(22.0).into());
        });
    }

    /// A toggle `IconButton` under `StatefulBuilder`, whose press flips `is_selected`.
    fn toggle_frame(
        tester: &WidgetTester,
        is_selected: Option<Rc<Cell<bool>>>,
        build: fn(Option<Listener>, Option<bool>) -> IconButton,
    ) -> WidgetRef {
        material_app(
            m3_theme(tester),
            StatefulBuilder::new(move |_app, _context, set_state| {
                let toggle = is_selected.clone();
                let selected = is_selected.as_ref().map(|is_selected| is_selected.get());
                build(
                    Some(Listener::new(move |app| {
                        if let Some(toggle) = toggle.clone() {
                            set_state(app, &mut |_app| toggle.set(!toggle.get()));
                        }
                    })),
                    selected,
                )
                .into_widget()
            })
            .into_widget(),
        )
    }

    fn selected_of(tester: &WidgetTester) -> Option<bool> {
        tester.widget(icon_button_element(tester), |button: &IconButton| {
            button.is_selected
        })
    }

    fn ac_unit_button(on_pressed: Option<Listener>, is_selected: Option<bool>) -> IconButton {
        let button = IconButton::new(on_pressed, icon(Icons::ac_unit()));
        match is_selected {
            Some(is_selected) => button.is_selected(is_selected),
            None => button,
        }
    }

    // 'Default IconButton is not selectable - M3'
    #[test]
    fn m3_a_default_button_is_not_selectable() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            let widget = toggle_frame(tester, None, ac_unit_button);
            tester.pump_widget(widget);

            assert_eq!(selected_of(tester), None);
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));

            // The non-toggle IconButton should not change appearance after clicking
            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            assert_eq!(selected_of(tester), None);
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));
        });
    }

    // 'Icon button is selectable when isSelected is not null - M3'
    #[test]
    fn m3_a_button_with_is_selected_toggles() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            let widget = toggle_frame(tester, Some(Rc::new(Cell::new(false))), ac_unit_button);
            tester.pump_widget(widget);

            assert_eq!(selected_of(tester), Some(false));
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));

            // The toggle IconButton should change appearance after clicking
            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            assert_eq!(selected_of(tester), Some(true));
            assert_eq!(icon_color(tester), Some(colors.primary));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));

            // The IconButton should be unselected if it's clicked again
            tester.tap(center, 2);
            tester.pump_and_settle();

            assert_eq!(selected_of(tester), Some(false));
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));
        });
    }

    // 'The IconButton is in selected status if isSelected is true by default - M3'
    #[test]
    fn m3_a_button_selected_from_the_start_shows_selected() {
        test_widgets(|tester| {
            let colors = ColorScheme::light();
            let widget = toggle_frame(tester, Some(Rc::new(Cell::new(true))), ac_unit_button);
            tester.pump_widget(widget);

            assert_eq!(selected_of(tester), Some(true));
            assert_eq!(icon_color(tester), Some(colors.primary));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));

            // The IconButton becomes unselected if it's clicked
            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            assert_eq!(selected_of(tester), Some(false));
            assert_eq!(icon_color(tester), Some(colors.on_surface_variant()));
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));
        });
    }

    fn account_box_button(on_pressed: Option<Listener>, is_selected: Option<bool>) -> IconButton {
        IconButton::new(on_pressed, icon(Icons::account_box_outlined()))
            .selected_icon(icon(Icons::account_box()))
            .is_selected(is_selected.expect("a toggle"))
    }

    // "The selectedIcon is used if it's not null and the button is clicked"
    #[test]
    fn the_selected_icon_shows_while_selected() {
        test_widgets(|tester| {
            let widget = toggle_frame(tester, Some(Rc::new(Cell::new(false))), account_box_button);
            tester.pump_widget(widget);

            assert_eq!(find_icon(tester, &Icons::account_box_outlined()).len(), 1);
            assert!(find_icon(tester, &Icons::account_box()).is_empty());

            // The icon becomes to selectedIcon
            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            assert_eq!(find_icon(tester, &Icons::account_box()).len(), 1);
            assert!(find_icon(tester, &Icons::account_box_outlined()).is_empty());

            // The icon becomes the original icon when it's clicked again
            tester.tap(center, 2);
            tester.pump_and_settle();

            assert_eq!(find_icon(tester, &Icons::account_box_outlined()).len(), 1);
            assert!(find_icon(tester, &Icons::account_box()).is_empty());
        });
    }

    fn plain_account_box_button(
        on_pressed: Option<Listener>,
        is_selected: Option<bool>,
    ) -> IconButton {
        IconButton::new(on_pressed, icon(Icons::account_box()))
            .is_selected(is_selected.expect("a toggle"))
    }

    // 'The original icon is used for selected and unselected status when selectedIcon is null'
    #[test]
    fn without_a_selected_icon_the_icon_shows_in_both_states() {
        test_widgets(|tester| {
            let widget = toggle_frame(
                tester,
                Some(Rc::new(Cell::new(false))),
                plain_account_box_button,
            );
            tester.pump_widget(widget);

            assert_eq!(selected_of(tester), Some(false));
            assert!(
                tester.widget(icon_button_element(tester), |button: &IconButton| button
                    .selected_icon
                    .is_none())
            );
            assert_eq!(find_icon(tester, &Icons::account_box()).len(), 1);

            let center = button_center(tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            assert_eq!(selected_of(tester), Some(true));
            assert!(
                tester.widget(icon_button_element(tester), |button: &IconButton| button
                    .selected_icon
                    .is_none())
            );
            assert_eq!(find_icon(tester, &Icons::account_box()).len(), 1);
        });
    }

    // 'The selectedIcon is used for disabled button if isSelected is true - M3'
    #[test]
    fn m3_a_disabled_selected_button_shows_the_selected_icon() {
        test_widgets(|tester| {
            pump_m3(
                tester,
                IconButton::new(None, icon(Icons::account_box()))
                    .is_selected(true)
                    .selected_icon(icon(Icons::ac_unit())),
            );
            assert_eq!(selected_of(tester), Some(true));
            assert!(find_icon(tester, &Icons::account_box()).is_empty());
            assert_eq!(find_icon(tester, &Icons::ac_unit()).len(), 1);
        });
    }

    // 'The visualDensity of M3 IconButton can be configured by IconButtonTheme, but cannot be
    // configured by ThemeData - M3'
    #[test]
    fn m3_the_visual_density_comes_from_the_icon_button_theme_not_the_theme() {
        test_widgets(|tester| {
            let build = |tester: &mut WidgetTester,
                         icon_button_theme_density: Option<VisualDensity>,
                         theme_density: Option<VisualDensity>| {
                let mut style = IconButton::style_from();
                if let Some(density) = icon_button_theme_density {
                    style = style.visual_density(density);
                }
                let base = m3_theme(tester);
                let mut copy = base
                    .copy_with()
                    .icon_button_theme(IconButtonThemeData::new().style(style.build()));
                if let Some(density) = theme_density {
                    copy = copy.visual_density(density);
                }
                let widget = material_app(
                    copy.build(),
                    Material::new()
                        .child(
                            Center::new().child(IconButton::new(noop(), icon(Icons::play_arrow()))),
                        )
                        .into_widget(),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle();
            };

            build(tester, Some(VisualDensity::STANDARD), None);
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            build(tester, Some(VisualDensity::COMPACT), None);
            assert_eq!(button_size(tester), Size::new(40.0, 40.0));

            build(tester, Some(VisualDensity::new(3.0, 3.0)), None);
            assert_eq!(button_size(tester), Size::new(64.0, 64.0));

            // ThemeData.visualDensity will be ignored because useMaterial3 is true
            build(tester, None, Some(VisualDensity::STANDARD));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            build(tester, None, Some(VisualDensity::COMPACT));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            build(tester, None, Some(VisualDensity::new(3.0, 3.0)));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));
        });
    }

    /// Hovers, presses, and then focuses by Tab the button `style` styles: the overlay colours
    /// the ink paints in each state.
    fn hovered_pressed_and_focused_overlays(
        tester: &mut WidgetTester,
        style: ButtonStyle,
    ) -> [Vec<u32>; 3] {
        let widget = material_app(
            default_theme(tester),
            Center::new()
                .child(IconButton::new(noop(), icon(Icons::add())).style(style))
                .into_widget(),
        );
        tester.pump_widget(widget);

        // Hovered.
        let center = button_center(tester);
        let mut mouse = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();
        let hovered = ink_rects(tester);

        // Highlighted (pressed).
        tester.send(&mouse, waft_embedder::PointerChange::Down);
        tester.pump_and_settle();
        let pressed = ink_rects(tester);
        // Remove pressed and hovered states,
        tester.up(&mouse);
        tester.pump_and_settle();
        tester.move_to(&mut mouse, Offset::new(0.0, 50.0), false);
        tester.pump_and_settle();

        // Focused.
        tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
        tester.pump_and_settle();
        [hovered, pressed, ink_rects(tester)]
    }

    // 'IconButton.styleFrom overlayColor overrides default overlay color'
    #[test]
    fn style_from_overlay_color_overrides_the_default_overlay() {
        test_widgets(|tester| {
            let overlay_color = Color::new(0xffff0000);
            let [hovered, pressed, focused] = hovered_pressed_and_focused_overlays(
                tester,
                IconButton::style_from()
                    .overlay_color(overlay_color)
                    .build(),
            );
            assert_eq!(hovered, vec![argb(overlay_color.with_opacity(0.08))]);
            assert_eq!(
                pressed,
                vec![
                    argb(overlay_color.with_opacity(0.08)),
                    argb(overlay_color.with_opacity(0.1))
                ]
            );
            assert_eq!(focused, vec![argb(overlay_color.with_opacity(0.1))]);
        });
    }

    // 'IconButton.styleFrom highlight, hover, focus colors overrides overlayColor'
    #[test]
    fn style_from_state_colors_override_the_overlay_color() {
        test_widgets(|tester| {
            let hover_color = Color::new(0xff0000f2);
            let highlight_color = Color::new(0xff0000f1);
            let focus_color = Color::new(0xff0000f3);
            let overlay_color = Color::new(0xffff0000);
            let style = IconButton::style_from()
                .hover_color(hover_color)
                .highlight_color(highlight_color)
                .focus_color(focus_color)
                .overlay_color(overlay_color)
                .build();
            let focused = style
                .overlay_color
                .as_ref()
                .expect("an overlay")
                .resolve(&states(&[WidgetState::Focused]));
            assert_eq!(focused, Some(focus_color));
            let [hovered, pressed, focused] = hovered_pressed_and_focused_overlays(tester, style);
            assert_eq!(hovered, vec![argb(hover_color)]);
            assert_eq!(pressed, vec![argb(hover_color), argb(highlight_color)]);
            assert_eq!(focused, vec![argb(focus_color)]);
        });
    }

    // 'IconButton.styleFrom with transparent overlayColor'
    #[test]
    fn style_from_a_transparent_overlay_color_defeats_the_highlights() {
        test_widgets(|tester| {
            let overlay_color = Colors::TRANSPARENT;
            let style = IconButton::style_from()
                .overlay_color(overlay_color)
                .build();
            let focused = style
                .overlay_color
                .as_ref()
                .expect("an overlay")
                .resolve(&states(&[WidgetState::Focused]));
            assert_eq!(focused, Some(overlay_color));
            // Dart paints transparent rectangles; Valo records no draw that shows nothing.
            let [hovered, pressed, focused] = hovered_pressed_and_focused_overlays(tester, style);
            assert!(hovered.is_empty());
            assert!(pressed.is_empty());
            assert!(focused.is_empty());
        });
    }

    // 'IconButton.color takes precedence over ambient IconButtonThemeData.iconColor'
    #[test]
    fn the_button_color_precedes_the_theme_icon_color() {
        test_widgets(|tester| {
            let icon_button_color = Color::new(0xFFFF1234);
            let purple = Colors::PURPLE.primary();
            let theme = ThemeData::builder()
                .icon_button_theme(
                    IconButtonThemeData::new().style(ButtonStyle::new().icon_color(
                        <dyn WidgetStateProperty<Option<Color>>>::from_map(vec![(
                            WidgetState::any(),
                            Some(purple),
                        )]),
                    )),
                )
                .build(&tester.cell.borrow());
            let widget = material_app(
                theme,
                Material::new()
                    .child(
                        Center::new().child(
                            IconButton::new(noop(), icon(Icons::add()).size(64.0))
                                .color(icon_button_color),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(icon_button_color));
        });
    }

    // 'IconTheme overrides default values in M3'
    #[test]
    fn m3_the_icon_theme_overrides_the_defaults() {
        test_widgets(|tester| {
            let red = Colors::RED.primary();
            let pink = Colors::PINK.primary();
            // Theme's IconTheme
            let theme = m3_theme(tester)
                .copy_with()
                .icon_theme(IconThemeData::new().color(red).size(37.0))
                .build();
            let widget = material_app(
                theme,
                IconButton::new(noop(), icon(Icons::account_box())).into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(red));
            assert_eq!(icon_size(tester), Size::new(37.0, 37.0));

            // custom IconTheme outside of IconButton
            let widget = material_app(
                m3_theme(tester),
                IconTheme::merge(
                    None,
                    IconThemeData::new().color(pink).size(35.0),
                    IconButton::new(noop(), icon(Icons::account_box())),
                ),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(pink));
            assert_eq!(icon_size(tester), Size::new(35.0, 35.0));
        });
    }

    // 'Theme IconButtonTheme overrides IconTheme in Material3'
    #[test]
    fn m3_the_theme_icon_button_theme_overrides_the_icon_theme() {
        test_widgets(|tester| {
            let green = Colors::GREEN.primary();
            let theme = m3_theme(tester)
                .copy_with()
                .icon_theme(IconThemeData::new().color(Colors::RED.primary()).size(25.0))
                .icon_button_theme(
                    IconButtonThemeData::new().style(
                        IconButton::style_from()
                            .foreground_color(green)
                            .icon_size(27.0)
                            .build(),
                    ),
                )
                .build();
            let widget = material_app(
                theme,
                IconButton::new(noop(), icon(Icons::account_box())).into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(green));
            assert_eq!(icon_size(tester), Size::new(27.0, 27.0));
        });
    }

    // 'Button IconButtonTheme always overrides IconTheme in Material3'
    #[test]
    fn m3_an_icon_button_theme_always_overrides_the_icon_theme() {
        test_widgets(|tester| {
            let orange = Colors::ORANGE.primary();
            let blue = Colors::BLUE.primary();
            let build = |tester: &WidgetTester, outer: (Color, f64), inner: (Color, f64)| {
                material_app(
                    m3_theme(tester),
                    IconTheme::merge(
                        None,
                        IconThemeData::new().color(outer.0).size(outer.1),
                        IconButtonTheme::new(
                            IconButtonThemeData::new().style(
                                IconButton::style_from()
                                    .foreground_color(inner.0)
                                    .icon_size(inner.1)
                                    .build(),
                            ),
                            IconButton::new(noop(), icon(Icons::account_box())),
                        ),
                    ),
                )
            };

            // When IconButtonTheme is closer to IconButton, IconButtonTheme overrides IconTheme
            let widget = build(tester, (orange, 36.0), (blue, 35.0));
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(blue));
            assert_eq!(icon_size(tester), Size::new(35.0, 35.0));

            // When IconTheme is closer to IconButton, IconButtonTheme still overrides IconTheme
            let widget = build(tester, (blue, 35.0), (orange, 36.0));
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert_eq!(icon_color(tester), Some(orange));
            assert_eq!(icon_size(tester), Size::new(36.0, 36.0));
        });
    }

    // 'White icon color defined by users shows correctly in Material3'
    #[test]
    fn m3_a_white_icon_theme_color_shows_in_a_dark_theme() {
        test_widgets(|tester| {
            let theme = ThemeData::from(ColorScheme::dark())
                .build(&tester.cell.borrow())
                .copy_with()
                .icon_theme(IconThemeData::new().color(Colors::WHITE))
                .build();
            let widget = material_app(
                theme,
                IconButton::new(noop(), icon(Icons::account_box())).into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(icon_color(tester), Some(Colors::WHITE));
        });
    }

    // 'In light mode, icon color is M3 default color instead of IconTheme.of(context).color, if
    // only setting color in IconTheme' and its dark-mode twin; Dart's `Scaffold` adds nothing
    // the icon reads.
    #[test]
    fn m3_an_icon_theme_size_alone_keeps_the_default_icon_color() {
        test_widgets(|tester| {
            for scheme in [ColorScheme::dark(), ColorScheme::light()] {
                let scheme = scheme
                    .copy_with()
                    .with_on_surface_variant(Color::new(0xffe91e60));
                let theme = ThemeData::builder()
                    .color_scheme(scheme.clone())
                    .build(&tester.cell.borrow());
                let widget = material_app(
                    theme,
                    IconTheme::merge(
                        None,
                        IconThemeData::new().size(26.0),
                        IconButton::new(noop(), icon(Icons::account_box())),
                    ),
                );
                tester.pump_widget(widget);
                assert_eq!(icon_color(tester), Some(scheme.on_surface_variant()));
            }
        });
    }

    // "IconButton.styleFrom doesn't throw exception on passing only one cursor"
    #[test]
    fn a_style_with_only_an_enabled_cursor_builds() {
        test_widgets(|tester| {
            tester.pump_widget(
                Directionality::new(
                    TextDirection::Ltr,
                    Material::new().child(
                        IconButton::new(noop(), icon(Icons::add())).style(
                            crate::outlined_button::OutlinedButton::style_from()
                                .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
                                .build(),
                        ),
                    ),
                )
                .into_widget(),
            );
        });
    }

    // 'Material3 - IconButton memory leak'
    #[test]
    fn m3_removing_the_button_disposes_its_controllers() {
        test_widgets(|tester| {
            let widget = material_app(
                default_theme(tester),
                IconButton::new(noop(), icon(Icons::search())).into_widget(),
            );
            tester.pump_widget(widget);
            tester.pump_widget(SizedBox::new().into_widget());
            // No panic.
        });
    }

    // 'Tooltip is drawn when hovering within IconButton area but outside the Icon itself'
    #[test]
    fn the_tooltip_shows_when_hovering_the_button_outside_its_icon() {
        test_widgets(|tester| {
            let widget = material_app(
                default_theme(tester),
                Material::new()
                    .child(Center::new().child(
                        IconButton::new(noop(), icon(Icons::favorite())).tooltip("Test tooltip"),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);

            // Verify that the tooltip is not shown initially.
            assert!(tester.find_text("Test tooltip").is_empty());

            // Start hovering within IconButton area to show the tooltip.
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            let icon = tester.render_box_of(tester.find_by_type::<Icon>()[0]);
            let top_left = tester.global_rect(icon).top_left();
            // Move the cursor just outside the Icon.
            tester.move_to(
                &mut mouse,
                Offset::new(top_left.dx() - 1.0, top_left.dy() - 1.0),
                false,
            );
            tester.pump();

            // Verify that the tooltip is shown.
            assert_eq!(tester.find_text("Test tooltip").len(), 1);
        });
    }

    /// Dart's `buildAllVariants`.
    fn all_variants(
        tester: &WidgetTester,
        enabled: bool,
        use_material3: bool,
        on_hover: ValueChanged<bool>,
        on_long_press: Listener,
    ) -> WidgetRef {
        let on_pressed = || if enabled { noop() } else { None };
        let theme = ThemeData::builder()
            .use_material3(use_material3)
            .build(&tester.cell.borrow());
        let variant = |button: IconButton| {
            button
                .on_hover(on_hover.clone())
                .on_long_press(on_long_press.clone())
                .into_widget()
        };
        material_app(
            theme,
            Material::new()
                .child(Directionality::new(
                    TextDirection::Ltr,
                    Column::new().children([
                        variant(IconButton::new(on_pressed(), icon(Icons::favorite()))),
                        variant(IconButton::filled(on_pressed(), icon(Icons::add()))),
                        variant(IconButton::filled_tonal(
                            on_pressed(),
                            icon(Icons::settings()),
                        )),
                        variant(IconButton::outlined(on_pressed(), icon(Icons::home()))),
                    ]),
                ))
                .into_widget(),
        )
    }

    /// The centre of the `IconButton` around the icon `data`: Dart's
    /// `find.widgetWithIcon(IconButton, data)`.
    fn center_of_button_with(tester: &WidgetTester, data: &IconData) -> Offset {
        let icon = find_icon(tester, data)[0];
        let button = tester.ancestor_of_type::<IconButton>(icon);
        tester.global_rect(tester.render_box_of(button)).center()
    }

    /// Dart's `IconButton variants hovered & onLongPressed` cases; as in Dart, the filled,
    /// tonal and outlined rounds all aim at the button with the `add` icon.
    fn check_variants_hover_and_long_press(tester: &mut WidgetTester, use_material3: bool) {
        let hovered = Rc::new(Cell::new(false));
        let long_pressed = Rc::new(Cell::new(false));
        let on_hover: ValueChanged<bool> = {
            let hovered = Rc::clone(&hovered);
            Rc::new(move |_app, hover| hovered.set(hover))
        };
        let on_long_press = {
            let long_pressed = Rc::clone(&long_pressed);
            Listener::new(move |_app| long_pressed.set(true))
        };
        let mut mouse = TestPointer::mouse(1, 0);
        let mut pointer = 10;
        for data in [Icons::favorite(), Icons::add(), Icons::add(), Icons::add()] {
            let widget = all_variants(
                tester,
                true,
                use_material3,
                on_hover.clone(),
                on_long_press.clone(),
            );
            tester.pump_widget(widget);
            let offset = center_of_button_with(tester, &data);
            tester.add_pointer(&mut mouse, Offset::ZERO);

            tester.move_to(&mut mouse, offset, false);
            tester.pump();
            assert!(hovered.get());

            pointer += 1;
            tester.long_press(offset, pointer);
            tester.pump();
            assert!(long_pressed.get());

            hovered.set(false);
            long_pressed.set(false);

            let widget = all_variants(
                tester,
                false,
                use_material3,
                on_hover.clone(),
                on_long_press.clone(),
            );
            tester.pump_widget(widget);
            tester.move_to(&mut mouse, offset, false);
            tester.pump();
            assert!(!hovered.get());

            pointer += 1;
            tester.long_press(offset, pointer);
            tester.pump();
            assert!(!long_pressed.get());

            tester.send(&mouse, waft_embedder::PointerChange::Remove);
        }
    }

    // 'Material3 - IconButton variants hovered & onLongPressed'
    #[test]
    fn m3_every_variant_reports_hover_and_long_press_only_when_enabled() {
        test_widgets(|tester| check_variants_hover_and_long_press(tester, true));
    }

    // 'Material2 - IconButton variants hovered & onLongPressed'
    #[test]
    fn m2_every_variant_reports_hover_and_long_press_only_when_enabled() {
        test_widgets(|tester| check_variants_hover_and_long_press(tester, false));
    }

    /// Dart's `testStatesController`.
    fn check_states_controller(tester: &mut WidgetTester, button: Constructor) {
        let count = Rc::new(Cell::new(0));
        let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
        {
            let count = Rc::clone(&count);
            controller.add_listener(
                &mut tester.cell.borrow_mut(),
                Listener::new(move |_app| count.set(count.get() + 1)),
            );
        }
        let widget = material_app(
            default_theme(tester),
            Center::new()
                .child(button(noop(), icon(Icons::add())).states_controller(controller.clone()))
                .into_widget(),
        );
        tester.pump_widget(widget);

        assert_eq!(states_value(tester, controller.handle()), states(&[]));
        assert_eq!(count.get(), 0);

        let icon = tester.render_box_of(tester.find_by_type::<Icon>()[0]);
        let center = tester.global_rect(icon).center();
        let mut mouse = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();

        assert_eq!(
            states_value(tester, controller.handle()),
            states(&[WidgetState::Hovered])
        );
        assert_eq!(count.get(), 1);

        tester.move_to(&mut mouse, Offset::ZERO, false);
        tester.pump_and_settle();

        assert_eq!(states_value(tester, controller.handle()), states(&[]));
        assert_eq!(count.get(), 2);

        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();

        assert_eq!(
            states_value(tester, controller.handle()),
            states(&[WidgetState::Hovered])
        );
        assert_eq!(count.get(), 3);

        tester.send(&mouse, waft_embedder::PointerChange::Down);
        tester.pump_and_settle();

        assert_eq!(
            states_value(tester, controller.handle()),
            states(&[WidgetState::Hovered, WidgetState::Pressed])
        );
        assert_eq!(count.get(), 4);

        tester.up(&mouse);
        tester.pump_and_settle();

        assert_eq!(
            states_value(tester, controller.handle()),
            states(&[WidgetState::Hovered])
        );
        assert_eq!(count.get(), 5);

        tester.move_to(&mut mouse, Offset::ZERO, false);
        tester.pump_and_settle();

        assert_eq!(states_value(tester, controller.handle()), states(&[]));
        assert_eq!(count.get(), 6);

        tester.down(&mut mouse, center);
        tester.pump_and_settle();
        assert_eq!(
            states_value(tester, controller.handle()),
            states(&[WidgetState::Hovered, WidgetState::Pressed])
        );
        assert_eq!(count.get(), 8); // adds hovered and pressed - two changes
    }

    // 'IconButton statesController'
    #[test]
    fn the_states_controller_follows_hover_and_press() {
        test_widgets(|tester| check_states_controller(tester, IconButton::new));
    }

    // 'IconButton.filled statesController'
    #[test]
    fn the_filled_states_controller_follows_hover_and_press() {
        test_widgets(|tester| check_states_controller(tester, IconButton::filled));
    }

    // 'IconButton.filledTonal statesController'
    #[test]
    fn the_filled_tonal_states_controller_follows_hover_and_press() {
        test_widgets(|tester| check_states_controller(tester, IconButton::filled_tonal));
    }

    // 'IconButton.outlined statesController'
    #[test]
    fn the_outlined_states_controller_follows_hover_and_press() {
        test_widgets(|tester| check_states_controller(tester, IconButton::outlined));
    }

    // 'IconButton does not crash at zero area'
    #[test]
    fn a_button_lays_out_at_zero_area() {
        test_widgets(|tester| {
            let widget = material_app(
                default_theme(tester),
                Center::new()
                    .child(SizedBox::shrink().child(IconButton::new(noop(), icon(Icons::add()))))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(button_size(tester), Size::ZERO);
        });
    }

    /// Dart's `buildM3EApp`; its `Scaffold` adds nothing these cases read.
    fn m3e_app<K>(
        tester: &WidgetTester,
        theme: Option<ThemeData>,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        let theme = theme.unwrap_or_else(|| {
            ThemeData::builder()
                .icon_button_theme(
                    IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                )
                .build(&tester.cell.borrow())
        });
        material_app(theme, Center::new().child(child).into_widget())
    }

    fn sized(size_variant: ButtonSizeVariant) -> IconButton {
        IconButton::new(noop(), icon(Icons::add()))
            .style(ButtonStyle::new().size_variant(size_variant))
    }

    // 'M3E IconButton size variants': 'default size is small (40x40)'
    #[test]
    fn m3e_the_default_size_is_small() {
        test_widgets(|tester| {
            let widget = m3e_app(tester, None, IconButton::new(noop(), icon(Icons::add())));
            tester.pump_widget(widget);
            // ButtonStyleButton renders with minimum size 40x40, but tap target
            // padding brings it to 48x48.
            assert_eq!(button_material_size(tester), Size::new(40.0, 40.0));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));
        });
    }

    // 'xSmall size renders at 32dp minimum with 48dp tap target'
    #[test]
    fn m3e_extra_small_is_32_in_a_48_tap_target() {
        test_widgets(|tester| {
            let press_count = Rc::new(Cell::new(0));
            let widget = m3e_app(
                tester,
                None,
                IconButton::new(counter(&press_count), icon(Icons::add()))
                    .style(ButtonStyle::new().size_variant(ButtonSizeVariant::XSmall)),
            );
            tester.pump_widget(widget);

            assert_eq!(button_material_size(tester), Size::new(32.0, 32.0));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));

            let center = button_center(tester);
            for (pointer, offset) in [
                Offset::new(23.0, 0.0),
                Offset::new(-23.0, 0.0),
                Offset::new(0.0, 23.0),
                Offset::new(0.0, -23.0),
            ]
            .into_iter()
            .enumerate()
            {
                tester.tap(center + offset, pointer as i64 + 1);
                tester.pump();
            }
            assert_eq!(press_count.get(), 4);
        });
    }

    // 'styleFrom sets the size variant'
    #[test]
    fn m3e_style_from_sets_the_size_variant() {
        test_widgets(|tester| {
            let widget = m3e_app(
                tester,
                None,
                IconButton::new(noop(), icon(Icons::add())).style(
                    IconButton::style_from()
                        .size_variant(ButtonSizeVariant::Medium)
                        .build(),
                ),
            );
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(56.0, 56.0));
            assert_eq!(button_size(tester), Size::new(56.0, 56.0));
        });
    }

    // 'medium size renders at 56dp minimum', 'large size renders at 96dp minimum',
    // 'xLarge size renders at 136dp minimum'
    #[test]
    fn m3e_the_larger_sizes_render_at_their_minimum() {
        test_widgets(|tester| {
            for (size_variant, side) in [
                (ButtonSizeVariant::Medium, 56.0),
                (ButtonSizeVariant::Large, 96.0),
                (ButtonSizeVariant::XLarge, 136.0),
            ] {
                let widget = m3e_app(tester, None, sized(size_variant));
                tester.pump_widget(widget);
                assert_eq!(button_material_size(tester), Size::new(side, side));
                assert_eq!(button_size(tester), Size::new(side, side));
            }
        });
    }

    // 'M3E IconButton width variants': 'small IconButton supports narrow, standard, and wide
    // widths'
    #[test]
    fn m3e_small_supports_three_widths() {
        test_widgets(|tester| {
            let mut material_size_for = |width: IconButtonWidthVariant| {
                let widget = m3e_app(
                    tester,
                    None,
                    IconButton::new(noop(), icon(Icons::add()))
                        .style(ButtonStyle::new().icon_button_width(width)),
                );
                tester.pump_widget(widget);
                button_material_size(tester)
            };
            assert_eq!(
                material_size_for(IconButtonWidthVariant::Narrow),
                Size::new(32.0, 40.0)
            );
            assert_eq!(
                material_size_for(IconButtonWidthVariant::Standard),
                Size::new(40.0, 40.0)
            );
            assert_eq!(
                material_size_for(IconButtonWidthVariant::Wide),
                Size::new(52.0, 40.0)
            );

            assert_eq!(
                button_material(tester, |material| material.animation_duration),
                crate::constants::K_THEME_CHANGE_DURATION
            );
        });
    }

    // 'IconButtonThemeData style width sets default width'
    #[test]
    fn m3e_the_theme_style_width_sets_the_default_width() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .icon_button_theme(
                    IconButtonThemeData::new()
                        .style(ButtonStyle::new().icon_button_width(IconButtonWidthVariant::Wide))
                        .variant(StyleVariant::Material3Expressive),
                )
                .build(&tester.cell.borrow());
            let widget = m3e_app(
                tester,
                Some(theme),
                IconButton::new(noop(), icon(Icons::add())),
            );
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(52.0, 40.0));
        });
    }

    // 'M3E IconButton shape': 'default shape resolves M3E token shapes by state'
    #[test]
    fn m3e_the_default_shape_resolves_by_state() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let widget = m3e_app(
                tester,
                None,
                IconButton::new(noop(), icon(Icons::add()))
                    .states_controller(controller.clone())
                    .is_selected(true),
            );
            tester.pump_widget(widget);
            assert!(material_shape_is(tester, &rounded(12.0)));

            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Pressed, true);
            tester.pump_and_settle();
            assert!(material_shape_is(tester, &rounded(8.0)));
        });
    }

    // 'square shape resolves M3E token shapes by state'
    #[test]
    fn m3e_the_square_shape_resolves_by_state() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let build = |tester: &WidgetTester, is_selected: bool| {
                m3e_app(
                    tester,
                    None,
                    IconButton::new(noop(), icon(Icons::add()))
                        .states_controller(controller.clone())
                        .is_selected(is_selected)
                        .style(
                            ButtonStyle::new()
                                .size_variant(ButtonSizeVariant::Medium)
                                .shape_variant(ButtonShapeVariant::Square),
                        ),
                )
            };
            let widget = build(tester, false);
            tester.pump_widget(widget);
            assert!(material_shape_is(tester, &rounded(16.0)));

            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Pressed, true);
            tester.pump_and_settle();
            assert!(material_shape_is(tester, &rounded(12.0)));

            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Pressed, false);
            let widget = build(tester, true);
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert!(material_shape_is(tester, &stadium()));
        });
    }

    // 'ButtonStyle.shape remains the stateful shape override API'
    #[test]
    fn m3e_a_style_shape_overrides_the_token_shapes() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let shape =
                <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(|states| {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(OutlinedBorderRef::new(rounded(4.0)));
                    }
                    Some(OutlinedBorderRef::new(stadium()))
                });
            let widget = m3e_app(
                tester,
                None,
                IconButton::new(noop(), icon(Icons::add()))
                    .states_controller(controller.clone())
                    .style(ButtonStyle::new().shape(shape)),
            );
            tester.pump_widget(widget);
            assert!(material_shape_is(tester, &stadium()));

            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Pressed, true);
            tester.pump_and_settle();
            assert!(material_shape_is(tester, &rounded(4.0)));
        });
    }

    fn scheme_of(tester: &WidgetTester) -> ColorScheme {
        let element = icon_button_element(tester);
        let mut app = tester.cell.borrow_mut();
        Theme::of(&mut app, element).color_scheme.clone()
    }

    // 'M3E IconButton variants': 'standard variant has transparent background', 'filled
    // variant resolves default container color', 'filledTonal variant resolves default
    // container color'
    #[test]
    fn m3e_each_variant_has_its_container_color() {
        test_widgets(|tester| {
            let cases: [(Constructor, ContainerColor); 3] = [
                (IconButton::new, |_| Colors::TRANSPARENT),
                (IconButton::filled, |scheme| scheme.primary),
                (IconButton::filled_tonal, |scheme| {
                    scheme.secondary_container()
                }),
            ];
            for (button, color) in cases {
                let widget = m3e_app(tester, None, button(noop(), icon(Icons::add())));
                tester.pump_widget(widget);
                let scheme = scheme_of(tester);
                assert_eq!(material_color(tester), Some(color(&scheme)));
                assert!(material_shape_is(tester, &stadium()));
            }
        });
    }

    // 'outlined variant resolves default side and transparent background'
    #[test]
    fn m3e_outlined_has_a_side_and_a_transparent_background() {
        test_widgets(|tester| {
            let widget = m3e_app(
                tester,
                None,
                IconButton::outlined(noop(), icon(Icons::add())),
            );
            tester.pump_widget(widget);
            let scheme = scheme_of(tester);
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));
            assert!(material_shape_is(
                tester,
                &stadium_with(scheme.outline_variant())
            ));
        });
    }

    // 'filled variant with style size', 'outlined variant with style size'
    #[test]
    fn m3e_filled_and_outlined_take_a_style_size() {
        test_widgets(|tester| {
            let cases: [(Constructor, ButtonSizeVariant, f64); 2] = [
                (IconButton::filled, ButtonSizeVariant::Large, 96.0),
                (IconButton::outlined, ButtonSizeVariant::Medium, 56.0),
            ];
            for (button, size_variant, side) in cases {
                let widget = m3e_app(
                    tester,
                    None,
                    button(noop(), icon(Icons::add()))
                        .style(ButtonStyle::new().size_variant(size_variant)),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(button_material_size(tester), Size::new(side, side));
                assert_eq!(button_size(tester), Size::new(side, side));
            }
        });
    }

    fn m3e_theme_with_size(tester: &WidgetTester, size_variant: ButtonSizeVariant) -> ThemeData {
        ThemeData::builder()
            .icon_button_theme(
                IconButtonThemeData::new()
                    .style(ButtonStyle::new().size_variant(size_variant))
                    .variant(StyleVariant::Material3Expressive),
            )
            .build(&tester.cell.borrow())
    }

    // 'M3E IconButton theme integration': 'IconButtonThemeData style size sets default size'
    #[test]
    fn m3e_the_theme_style_size_sets_the_default_size() {
        test_widgets(|tester| {
            let theme = m3e_theme_with_size(tester, ButtonSizeVariant::Large);
            let widget = m3e_app(
                tester,
                Some(theme),
                IconButton::new(noop(), icon(Icons::add())),
            );
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(96.0, 96.0));
            assert_eq!(button_size(tester), Size::new(96.0, 96.0));
        });
    }

    // 'widget size overrides theme size'
    #[test]
    fn m3e_the_button_size_overrides_the_theme_size() {
        test_widgets(|tester| {
            let theme = m3e_theme_with_size(tester, ButtonSizeVariant::Large);
            let widget = m3e_app(tester, Some(theme), sized(ButtonSizeVariant::XSmall));
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(32.0, 32.0));
            assert_eq!(button_size(tester), Size::new(48.0, 48.0));
        });
    }

    // 'IconButtonTheme wrapping sets size'
    #[test]
    fn m3e_an_icon_button_theme_sets_the_size() {
        test_widgets(|tester| {
            let widget = m3e_app(
                tester,
                None,
                IconButtonTheme::new(
                    IconButtonThemeData::new()
                        .style(ButtonStyle::new().size_variant(ButtonSizeVariant::Medium))
                        .variant(StyleVariant::Material3Expressive),
                    IconButton::new(noop(), icon(Icons::add())),
                ),
            );
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(56.0, 56.0));
            assert_eq!(button_size(tester), Size::new(56.0, 56.0));
        });
    }

    fn favorite_button(is_selected: Option<bool>) -> IconButton {
        let button = IconButton::new(noop(), icon(Icons::favorite_border()))
            .selected_icon(icon(Icons::favorite()));
        match is_selected {
            Some(is_selected) => button.is_selected(is_selected),
            None => button,
        }
    }

    // 'M3E IconButton selection': 'isSelected shows selectedIcon', 'isSelected false shows
    // regular icon'
    #[test]
    fn m3e_is_selected_picks_the_icon() {
        test_widgets(|tester| {
            for (is_selected, shown, hidden) in [
                (true, Icons::favorite(), Icons::favorite_border()),
                (false, Icons::favorite_border(), Icons::favorite()),
            ] {
                let widget = m3e_app(tester, None, favorite_button(Some(is_selected)));
                tester.pump_widget(widget);
                assert_eq!(find_icon(tester, &shown).len(), 1);
                assert!(find_icon(tester, &hidden).is_empty());
            }
        });
    }

    // 'external selected state does not affect non-toggleable visual state'
    #[test]
    fn m3e_an_external_selected_state_leaves_a_plain_button_as_it_is() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Selected, true);
            let widget = m3e_app(
                tester,
                None,
                favorite_button(None).states_controller(controller.clone()),
            );
            tester.pump_widget(widget);
            assert!(material_shape_is(tester, &stadium()));
            assert_eq!(find_icon(tester, &Icons::favorite_border()).len(), 1);
            assert!(find_icon(tester, &Icons::favorite()).is_empty());
        });
    }

    // 'isSelected updates selected widget state when toggled through null'
    #[test]
    fn m3e_is_selected_updates_the_selected_state_through_null() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let selected = |tester: &WidgetTester| {
                states_value(tester, controller.handle()).contains(&WidgetState::Selected)
            };
            for (is_selected, expected) in [
                (Some(true), true),
                (None, false),
                (Some(false), false),
                (Some(true), true),
            ] {
                let widget = m3e_app(
                    tester,
                    None,
                    favorite_button(is_selected).states_controller(controller.clone()),
                );
                tester.pump_widget(widget);
                assert_eq!(selected(tester), expected);
            }
        });
    }

    // 'M3E IconButton disabled state': 'disabled button has reduced opacity colors'
    #[test]
    fn m3e_a_disabled_button_has_reduced_opacity_colors() {
        test_widgets(|tester| {
            let widget = m3e_app(tester, None, IconButton::new(None, icon(Icons::add())));
            tester.pump_widget(widget);
            let scheme = scheme_of(tester);
            assert_eq!(material_color(tester), Some(Colors::TRANSPARENT));
            assert_eq!(
                icon_theme_color(tester),
                Some(
                    scheme
                        .on_surface
                        .with_values(Some(0.38), None, None, None, None)
                )
            );
        });
    }

    // 'onLongPress without onPressed keeps button disabled', without its semantics
    #[test]
    fn m3e_a_long_press_without_on_pressed_does_nothing() {
        test_widgets(|tester| {
            let long_pressed = Rc::new(Cell::new(0));
            let on_long_press = {
                let long_pressed = Rc::clone(&long_pressed);
                Listener::new(move |_app| long_pressed.set(long_pressed.get() + 1))
            };
            let widget = m3e_app(
                tester,
                None,
                IconButton::new(None, icon(Icons::add())).on_long_press(on_long_press),
            );
            tester.pump_widget(widget);
            let center = button_center(tester);
            tester.long_press(center, 1);
            assert_eq!(long_pressed.get(), 0);
        });
    }

    // 'disabled filled button has reduced background'
    #[test]
    fn m3e_a_disabled_filled_button_has_a_reduced_background() {
        test_widgets(|tester| {
            let widget = m3e_app(tester, None, IconButton::filled(None, icon(Icons::add())));
            tester.pump_widget(widget);
            let scheme = scheme_of(tester);
            assert_eq!(
                material_color(tester),
                Some(
                    scheme
                        .on_surface
                        .with_values(Some(0.1), None, None, None, None)
                )
            );
            assert_eq!(
                icon_theme_color(tester),
                Some(
                    scheme
                        .on_surface
                        .with_values(Some(0.38), None, None, None, None)
                )
            );
        });
    }

    // 'M3E IconButton variant opt in': 'IconButtonThemeData variant enables M3E defaults'
    #[test]
    fn m3e_the_theme_variant_opts_into_the_expressive_defaults() {
        test_widgets(|tester| {
            let widget = m3e_app(tester, None, sized(ButtonSizeVariant::Medium));
            tester.pump_widget(widget);
            assert_eq!(button_material_size(tester), Size::new(56.0, 56.0));
            assert_eq!(button_size(tester), Size::new(56.0, 56.0));
        });
    }

    // 'Trigger Ink splash when hovering within layout bounds with tooltip'
    #[test]
    fn a_tooltip_passes_a_press_at_the_corner_of_the_layout_to_the_ink() {
        // This is a regression test for https://github.com/flutter/flutter/issues/153544.
        test_widgets(|tester| {
            let box_key = GlobalKey::new();
            let widget = material_app(
                default_theme(tester),
                Material::new()
                    .child(
                        Center::new().child(
                            waft_widgets::ColoredBox::new(Color::new(0xFFFF0000))
                                .key(box_key.clone().into())
                                .child(
                                    IconButton::new(noop(), icon(Icons::favorite()))
                                        .tooltip("Test tooltip")
                                        .style(
                                            ButtonStyle::new()
                                                .overlay_color(Rc::new(
                                                    waft_widgets::WidgetStatePropertyAll::new(
                                                        Some(Color::new(0xFF00FF00)),
                                                    ),
                                                ))
                                                .padding(Rc::new(
                                                    waft_widgets::WidgetStatePropertyAll::new(
                                                        Some(EdgeInsetsGeometry::all(20.0)),
                                                    ),
                                                )),
                                        ),
                                ),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);

            let top_left = {
                let render = tester.render_object(&box_key).as_box().expect("a box");
                tester.global_rect(render).top_left()
            };
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, top_left, false);
            tester.down(&mut gesture, top_left);
            tester.pump_and_settle();
            // Dart's `paints..rect(red)..rect(green)` on the outer `Material`: the colored box,
            // then the button's overlay, in order among what it paints.
            let ink = tester.first::<crate::material::RenderInkFeatures>();
            let rects: Vec<u32> = tester
                .painted(waft_rendering::RenderBox::as_object(ink))
                .into_iter()
                .filter_map(|drawn| match drawn {
                    Drawn::Rect(color, _) => Some(color),
                    _ => None,
                })
                .collect();
            let colored_box = rects
                .iter()
                .position(|color| *color == 0xFFFF0000)
                .expect("the colored box"); // ColoredBox.
            assert!(rects[colored_box..].contains(&0xFF00FF00)); // IconButton overlay.
        });
    }
}
