//! Flutter counterpart: material_ui's `elevated_button.dart`, with its parts
//! `generated/elevated_button_defaults_m3.g.dart` and
//! `generated/elevated_button_defaults_m3e.g.dart`.
//!
//! Dart's `ElevatedButton.icon` and `ElevatedButton.styleFrom` take named arguments the body
//! combines (the icon child reads `style`; `styleFrom` derives its overlay color from two of
//! them), so each is a value collecting them, [`ElevatedButtonIcon`] and
//! [`ElevatedButtonStyleFromBuilder`], that converts once every argument is given.
#![allow(deprecated)]

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Clip, Color, Radius, Size, clamp_double, lerp_double};
use waft_foundation::{App, K_IS_WEB, Listener};
use waft_painting::{
    AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef,
    RoundedRectangleBorder, TextStyle,
};
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    BuildContext, Flexible, IntoWidget, KeyRef, MainAxisSize, MediaQuery, Row, StatefulKind,
    StatefulWidget, StatelessWidget, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyleGetters;
use crate::button_style::{ButtonLayerBuilder, ButtonShapeVariant, ButtonSizeVariant, ButtonStyle};
use crate::button_style_button::{
    ButtonStyleButton, ButtonStyleButtonData, ButtonStyleState, IconAlignment,
};
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::elevated_button_theme::ElevatedButtonTheme;
use crate::generated::elevated_button_defaults_m3::ElevatedButtonDefaultsM3;
use crate::generated::elevated_button_defaults_m3e::{ElevatedButtonDefaultsM3E, icon_label_space};
use crate::ink_ripple::InkRipple;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, StyleVariant, VisualDensity};

/// A Material Design "elevated button".
///
/// Use elevated buttons to add dimension to otherwise mostly flat
/// layouts, e.g. in long busy lists of content, or in wide
/// spaces. Avoid using elevated buttons on already-elevated content
/// such as dialogs or cards.
///
/// An elevated button is a label `child` displayed on a `Material`
/// widget whose `Material::elevation` increases when the button is
/// pressed. The label's `Text` and `Icon` widgets are displayed in
/// `style`'s [`ButtonStyle::foreground_color`] and the button's filled
/// background is the [`ButtonStyle::background_color`].
///
/// The elevated button's default style is defined by
/// [`default_style_of`](ButtonStyleButton::default_style_of). The style of this elevated
/// button can be overridden with its `style` parameter. The style of all elevated
/// buttons in a subtree can be overridden with the
/// [`ElevatedButtonTheme`], and the style of all of the elevated
/// buttons in an app can be overridden with the `Theme`'s
/// `ThemeData::elevated_button_theme` property.
///
/// The static [`style_from`](Self::style_from) method is a convenient way to create a
/// elevated button [`ButtonStyle`] from simple values.
///
/// If `on_pressed` and `on_long_press` callbacks are null, then the
/// button will be disabled.
///
/// To use the Material 3 Expressive style, set
/// `ElevatedButtonThemeData::variant` to [`StyleVariant::Material3Expressive`].
/// The Expressive size and shape variants can then be selected with
/// [`ButtonStyle::size_variant`] and [`ButtonStyle::shape_variant`], and the button
/// becomes a toggle button when `is_selected` is non-null.
///
/// With the Material 3 style, `is_selected` updates [`WidgetState::Selected`] and
/// the button's semantics, but the default style doesn't change the button's
/// appearance when it's selected. To show the selected state, use a `style`
/// that resolves [`WidgetState::Selected`].
///
/// See also:
///
///  * `FilledButton`, a filled button that doesn't elevate when pressed.
///  * `FilledButton::tonal`, a filled button variant that uses a secondary fill color.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * `TextButton`, a button with no outline or fill color.
///  * <https://material.io/design/components/buttons.html>
///  * <https://m3.material.io/components/buttons>
pub struct ElevatedButton {
    button_style_button: ButtonStyleButtonData,
    add_padding: bool,
}

impl ElevatedButton {
    /// Create an ElevatedButton.
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: Option<impl IntoWidget<K>>,
    ) -> ElevatedButton {
        ElevatedButton {
            button_style_button: ButtonStyleButtonData::new(
                on_pressed,
                child.map(|child| child.into_widget()),
            ),
            add_padding: false,
        }
    }

    /// Create an elevated button from a pair of widgets that serve as the button's
    /// [`icon`](ElevatedButtonIcon::icon) and `label`.
    ///
    /// The icon and label are arranged in a row and padded by 12 logical pixels
    /// at the start, and 16 at the end, with an 8 pixel gap in between.
    ///
    /// With the Material 3 Expressive style, the padding and the gap between the
    /// icon and the label depend on [`ButtonStyle::size_variant`]. See
    /// [`default_style_of`](ButtonStyleButton::default_style_of).
    ///
    /// If [`icon`](ElevatedButtonIcon::icon) is null, this constructor will create an
    /// [`ElevatedButton`] that doesn't display an icon.
    ///
    /// The alignment of the icon within the button is set by
    /// [`icon_alignment`](ElevatedButtonIcon::icon_alignment); see [`IconAlignment`].
    pub fn icon<K>(on_pressed: Option<Listener>, label: impl IntoWidget<K>) -> ElevatedButtonIcon {
        let mut button_style_button = ButtonStyleButtonData::new(on_pressed, None);
        button_style_button.clip_behavior = Some(Clip::None);
        ElevatedButtonIcon {
            button_style_button,
            icon: None,
            label: label.into_widget(),
            icon_alignment: None,
        }
    }

    crate::button_style_button_setters!(
        ElevatedButton;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller, is_selected,
    );

    /// A static convenience method that constructs an elevated button
    /// [`ButtonStyle`] given simple values.
    ///
    /// The `foreground_color` and `disabled_foreground_color` colors are used
    /// to create a [`WidgetStateProperty`] [`ButtonStyle::foreground_color`], and
    /// a derived [`ButtonStyle::overlay_color`] if `overlay_color` isn't specified.
    ///
    /// If `overlay_color` is specified and its value is `Colors::TRANSPARENT`
    /// then the pressed/focused/hovered highlights are effectively defeated.
    /// Otherwise a [`WidgetStateProperty`] with the same opacities as the
    /// default is created.
    ///
    /// The `background_color` and `disabled_background_color` colors are
    /// used to create a [`WidgetStateProperty`] [`ButtonStyle::background_color`].
    ///
    /// Similarly, the `enabled_mouse_cursor` and `disabled_mouse_cursor`
    /// parameters are used to construct [`ButtonStyle::mouse_cursor`].
    ///
    /// The `icon_color`, `disabled_icon_color` are used to construct
    /// [`ButtonStyle::icon_color`] and `icon_size` is used to construct
    /// [`ButtonStyle::icon_size`].
    ///
    /// If `icon_color` is null, the button icon will use `foreground_color`. If
    /// `foreground_color` is also null, the button icon will use the default icon color.
    ///
    /// The button's elevations are defined relative to the `elevation`
    /// parameter. The disabled elevation is the same as the parameter
    /// value, `elevation` + 2 is used when the button is hovered
    /// or focused, and elevation + 6 is used when the button is pressed.
    ///
    /// All of the other parameters are either used directly or used to
    /// create a [`WidgetStateProperty`] with a single value for all
    /// states.
    ///
    /// The `size_variant` and `shape_variant` parameters are Material 3 Expressive
    /// options. They provide extra-small through extra-large sizes and round or
    /// square shapes through [`ButtonStyle::size_variant`] and
    /// [`ButtonStyle::shape_variant`].
    ///
    /// When `ElevatedButtonThemeData::variant` is
    /// [`StyleVariant::Material3Expressive`] and these properties are null,
    /// Material 3 Expressive [`ElevatedButton`] defaults use
    /// [`ButtonSizeVariant::Small`] and [`ButtonShapeVariant::Round`].
    ///
    /// All parameters default to null, by default this method returns
    /// a [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default text and icon colors for an
    /// [`ElevatedButton`], as well as its overlay color, with all of the
    /// standard opacity adjustments for the pressed, focused, and
    /// hovered states, one could write:
    ///
    /// ```text
    /// ElevatedButton::new(on_pressed, Text::new("Jump"))
    ///     .style(ElevatedButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    /// ```
    ///
    /// And to change the fill color:
    ///
    /// ```text
    /// ElevatedButton::new(on_pressed, Text::new("Meow"))
    ///     .style(ElevatedButton::style_from().background_color(Colors::GREEN.primary()).build())
    /// ```
    pub fn style_from() -> ElevatedButtonStyleFromBuilder {
        ElevatedButtonStyleFromBuilder::default()
    }
}

impl ButtonStyleButton for ElevatedButton {
    crate::button_style_button_accessors!();

    /// Defines the button's default appearance.
    ///
    /// The button `child`'s `Text` and `Icon` widgets are rendered with
    /// the [`ButtonStyle`]'s foreground color. The button's `InkWell` adds
    /// the style's overlay color when the button is focused, hovered
    /// or pressed. The button's background color becomes its `Material`
    /// color.
    ///
    /// All of the ButtonStyle's defaults appear below. In this list
    /// "Theme.foo" is shorthand for `Theme::of(context).foo`. Color
    /// scheme values like "onSurface(0.38)" are shorthand for
    /// `on_surface.with_opacity(0.38)`. [`WidgetStateProperty`] valued
    /// properties that are not followed by a sublist have the same
    /// value for all states, otherwise the values are as specified for
    /// each state, and "others" means all other states.
    ///
    /// The "default font size" below refers to the font size specified in the
    /// [`default_style_of`](ButtonStyleButton::default_style_of) method (or 14.0 if
    /// unspecified), scaled by the `MediaQuery::text_scaler_of(context).scale` method. The
    /// names of the EdgeInsets constructors and `EdgeInsetsGeometry::lerp` have been
    /// abbreviated for readability.
    ///
    /// The color of the [`ButtonStyle::text_style`] is not used, the
    /// [`ButtonStyle::foreground_color`] color is used instead.
    ///
    /// ## Material 2 defaults
    ///
    /// * `text_style` - Theme.textTheme.button
    /// * `background_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.12)
    ///   * others - Theme.colorScheme.primary
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others - Theme.colorScheme.onPrimary
    /// * `overlay_color`
    ///   * hovered - Theme.colorScheme.onPrimary(0.08)
    ///   * focused or pressed - Theme.colorScheme.onPrimary(0.12)
    /// * `shadow_color` - Theme.shadowColor
    /// * `elevation`
    ///   * disabled - 0
    ///   * default - 2
    ///   * hovered or focused - 4
    ///   * pressed - 8
    /// * `padding`
    ///   * `default font size <= 14` - horizontal(16)
    ///   * `14 < default font size <= 28` - lerp(horizontal(16), horizontal(8))
    ///   * `28 < default font size <= 36` - lerp(horizontal(8), horizontal(4))
    ///   * `36 < default font size` - horizontal(4)
    /// * `minimum_size` - Size(64, 36)
    /// * `fixed_size` - null
    /// * `maximum_size` - Size.infinite
    /// * `side` - null
    /// * `shape` - RoundedRectangleBorder(borderRadius: BorderRadius.circular(4))
    /// * `mouse_cursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visual_density` - theme.visualDensity
    /// * `tap_target_size` - theme.materialTapTargetSize
    /// * `animation_duration` - kThemeChangeDuration
    /// * `enable_feedback` - true
    /// * `alignment` - Alignment.center
    /// * `splash_factory` - InkRipple.splashFactory
    ///
    /// The default padding values for the [`ElevatedButton::icon`] factory are slightly
    /// different:
    ///
    /// * `padding`
    ///   * `default font size <= 14` - start(12) end(16)
    ///   * `14 < default font size <= 28` - lerp(start(12) end(16), horizontal(8))
    ///   * `28 < default font size <= 36` - lerp(horizontal(8), horizontal(4))
    ///   * `36 < default font size` - horizontal(4)
    ///
    /// The default value for `side`, which defines the appearance of the button's
    /// outline, is null. That means that the outline is defined by the button
    /// shape's `OutlinedBorder::side`. Typically the default value of an
    /// [`OutlinedBorder`]'s side is [`BorderSide::NONE`], so an outline is not drawn.
    ///
    /// ## Material 3 defaults
    ///
    /// If `ThemeData::use_material3` is set to true the following defaults will
    /// be used:
    ///
    /// * `text_style` - Theme.textTheme.labelLarge
    /// * `background_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.12)
    ///   * others - Theme.colorScheme.surfaceContainerLow
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others - Theme.colorScheme.primary
    /// * `overlay_color`
    ///   * hovered - Theme.colorScheme.primary(0.08)
    ///   * focused or pressed - Theme.colorScheme.primary(0.1)
    /// * `shadow_color` - Theme.colorScheme.shadow
    /// * `surface_tint_color` - Colors.transparent
    /// * `elevation`
    ///   * disabled - 0
    ///   * default - 1
    ///   * hovered - 3
    ///   * focused or pressed - 1
    /// * `padding`
    ///   * `default font size <= 14` - horizontal(24)
    ///   * `14 < default font size <= 28` - lerp(horizontal(24), horizontal(12))
    ///   * `28 < default font size <= 36` - lerp(horizontal(12), horizontal(6))
    ///   * `36 < default font size` - horizontal(6)
    /// * `minimum_size` - Size(64, 40)
    /// * `fixed_size` - null
    /// * `maximum_size` - Size.infinite
    /// * `side` - null
    /// * `shape` - StadiumBorder()
    /// * `mouse_cursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visual_density` - Theme.visualDensity
    /// * `tap_target_size` - Theme.materialTapTargetSize
    /// * `animation_duration` - kThemeChangeDuration
    /// * `enable_feedback` - true
    /// * `alignment` - Alignment.center
    /// * `splash_factory` - Theme.splashFactory
    ///
    /// For the [`ElevatedButton::icon`] factory, the start (generally the left) value of
    /// [`ButtonStyle::padding`] is reduced from 24 to 16.
    ///
    /// ## Material 3 Expressive defaults
    ///
    /// If `ThemeData::use_material3` is true and `ElevatedButtonThemeData::variant`
    /// is [`StyleVariant::Material3Expressive`], the following defaults will be
    /// used. Values listed per size are for [`ButtonSizeVariant::XSmall`],
    /// [`ButtonSizeVariant::Small`], [`ButtonSizeVariant::Medium`],
    /// [`ButtonSizeVariant::Large`], and [`ButtonSizeVariant::XLarge`] respectively.
    /// The selected values only apply when `is_selected` is non-null.
    ///
    /// * `text_style` - based on [`ButtonStyle::size_variant`]
    ///   * xSmall or small - Theme.textTheme.labelLarge
    ///   * medium - Theme.textTheme.titleMedium
    ///   * large - Theme.textTheme.headlineSmall
    ///   * xLarge - Theme.textTheme.headlineLarge
    /// * `background_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.1)
    ///   * selected - Theme.colorScheme.primary
    ///   * others - Theme.colorScheme.surfaceContainerLow
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * selected - Theme.colorScheme.onPrimary
    ///   * others - Theme.colorScheme.primary
    /// * `overlay_color`
    ///   * selected
    ///     * hovered - Theme.colorScheme.onPrimary(0.08)
    ///     * focused or pressed - Theme.colorScheme.onPrimary(0.1)
    ///   * hovered - Theme.colorScheme.primary(0.08)
    ///   * focused or pressed - Theme.colorScheme.primary(0.1)
    /// * `shadow_color` - Theme.colorScheme.shadow
    /// * `surface_tint_color` - Colors.transparent
    /// * `elevation`
    ///   * disabled - 0
    ///   * others - 1
    /// * `padding` - based on [`ButtonStyle::size_variant`]; horizontal(12),
    ///   horizontal(16), horizontal(24), horizontal(48), horizontal(64)
    /// * `minimum_size` - based on [`ButtonStyle::size_variant`]; Size(64, 32),
    ///   Size(64, 40), Size(64, 56), Size(64, 96), Size(64, 136)
    /// * `fixed_size` - null
    /// * `maximum_size` - Size.infinite
    /// * `icon_size` - based on [`ButtonStyle::size_variant`]; 20, 20, 24, 32, 40
    /// * `icon_color` - same as `foreground_color`
    /// * `side` - null
    /// * `shape` - based on [`ButtonStyle::size_variant`], [`ButtonStyle::shape_variant`],
    ///   and state
    ///   * pressed - RoundedRectangleBorder with radius 8, 8, 12, 16, 16
    ///   * selected
    ///     * round - RoundedRectangleBorder with radius 12, 12, 16, 28, 28
    ///     * square - StadiumBorder()
    ///   * others
    ///     * round - StadiumBorder()
    ///     * square - RoundedRectangleBorder with radius 12, 12, 16, 28, 28
    /// * `mouse_cursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visual_density` - VisualDensity.standard
    /// * `tap_target_size` - MaterialTapTargetSize.padded
    /// * `animation_duration` - kThemeChangeDuration
    /// * `enable_feedback` - true
    /// * `alignment` - Alignment.center
    /// * `splash_factory` - Theme.splashFactory
    /// * `size_variant` - ButtonSizeVariant.small
    /// * `shape_variant` - ButtonShapeVariant.round
    ///
    /// For the [`ElevatedButton::icon`] factory, the padding is the same, and the
    /// gap between the icon and the label is 4, 8, 8, 12, or 16 based on
    /// [`ButtonStyle::size_variant`].
    fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let theme = Theme::of(app, context);
        let color_scheme = &theme.color_scheme;
        let elevated_button_theme = ElevatedButtonTheme::of(app, context);
        let effective_variant = ElevatedButtonTheme::variant_of(app, context);
        let style = self.button_style_button.style.as_ref();
        let effective_size = style.and_then(|style| style.size_variant).or_else(|| {
            elevated_button_theme
                .style
                .as_ref()
                .and_then(|style| style.size_variant)
        });
        let effective_shape = style.and_then(|style| style.shape_variant).or_else(|| {
            elevated_button_theme
                .style
                .as_ref()
                .and_then(|style| style.shape_variant)
        });
        let use_expressive =
            theme.use_material3 && effective_variant == StyleVariant::Material3Expressive;
        let button_style: Box<dyn ButtonStyleGetters> = if theme.use_material3 {
            match effective_variant {
                StyleVariant::Material3 => Box::new(ElevatedButtonDefaultsM3::new(context)),
                StyleVariant::Material3Expressive => Box::new(ElevatedButtonDefaultsM3E::new(
                    context,
                    self.button_style_button.is_selected.is_some(),
                    effective_size,
                    effective_shape,
                )),
            }
        } else {
            let mut style_from = ElevatedButton::style_from()
                .background_color(color_scheme.primary)
                .foreground_color(color_scheme.on_primary)
                .disabled_background_color(color_scheme.on_surface.with_opacity(0.12))
                .disabled_foreground_color(color_scheme.on_surface.with_opacity(0.38))
                .shadow_color(theme.shadow_color)
                .elevation(2.0);
            if let Some(text_style) = theme.text_theme.label_large.clone() {
                style_from = style_from.text_style(text_style);
            }
            let style_from = style_from
                .padding(scaled_padding(app, context))
                .minimum_size(Size::new(64.0, 36.0))
                .maximum_size(Size::INFINITE)
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(4.0)).into(),
                ))
                .enabled_mouse_cursor(if K_IS_WEB {
                    SystemMouseCursors::CLICK.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                })
                .disabled_mouse_cursor(SystemMouseCursors::BASIC.into())
                .visual_density(theme.visual_density)
                .tap_target_size(theme.material_tap_target_size)
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER)
                .splash_factory(InkRipple::splash_factory())
                .build();
            Box::new(style_from)
        };

        // The Material 3 Expressive defaults use the same padding with or without
        // an icon.
        if use_expressive {
            return button_style;
        }

        // Only apply padding when the ElevatedButton has an Icon.
        if self.add_padding {
            let default_font_size = button_style
                .text_style(app)
                .and_then(|text_style| text_style.resolve(&WidgetStates::new()))
                .and_then(|text_style| text_style.font_size)
                .unwrap_or(14.0);
            let effective_text_scale =
                MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;

            let scaled_padding = if theme.use_material3 {
                <dyn ButtonStyleButton>::scaled_padding(
                    EdgeInsetsGeometry::from_steb(16.0, 0.0, 24.0, 0.0),
                    EdgeInsetsGeometry::from_steb(8.0, 0.0, 12.0, 0.0),
                    EdgeInsetsGeometry::from_steb(4.0, 0.0, 6.0, 0.0),
                    effective_text_scale,
                )
            } else {
                <dyn ButtonStyleButton>::scaled_padding(
                    EdgeInsetsGeometry::from_steb(12.0, 0.0, 16.0, 0.0),
                    EdgeInsetsGeometry::symmetric(0.0, 8.0),
                    EdgeInsetsGeometry::from_steb(8.0, 0.0, 4.0, 0.0),
                    effective_text_scale,
                )
            };
            return Box::new(
                button_style
                    .copy_with(app)
                    .padding(Rc::new(MaterialStatePropertyAll::new(Some(scaled_padding)))),
            );
        }

        button_style
    }

    /// Returns the `ElevatedButtonThemeData::style` of the closest
    /// [`ElevatedButtonTheme`] ancestor.
    fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        ElevatedButtonTheme::of(app, context).style
    }
}

impl StatefulWidget for ElevatedButton {
    type State = ButtonStyleState<ElevatedButton>;

    fn key(&self) -> Option<&KeyRef> {
        self.button_style_button.key.as_ref()
    }

    fn create_state(&self) -> ButtonStyleState<ElevatedButton> {
        ButtonStyleState::new()
    }
}

impl Debug for ElevatedButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.button_style_button, f)
    }
}

/// Dart's `ElevatedButton.styleFrom`, collecting its arguments; [`build`](Self::build) is the
/// call.
#[derive(Clone, Default)]
pub struct ElevatedButtonStyleFromBuilder {
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    disabled_foreground_color: Option<Color>,
    disabled_background_color: Option<Color>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    icon_color: Option<Color>,
    icon_size: Option<f64>,
    icon_alignment: Option<IconAlignment>,
    disabled_icon_color: Option<Color>,
    overlay_color: Option<Color>,
    elevation: Option<f64>,
    text_style: Option<TextStyle>,
    padding: Option<EdgeInsetsGeometry>,
    minimum_size: Option<Size>,
    fixed_size: Option<Size>,
    maximum_size: Option<Size>,
    side: Option<BorderSide>,
    shape: Option<OutlinedBorderRef>,
    enabled_mouse_cursor: Option<MouseCursorRef>,
    disabled_mouse_cursor: Option<MouseCursorRef>,
    visual_density: Option<VisualDensity>,
    tap_target_size: Option<MaterialTapTargetSize>,
    animation_duration: Option<Duration>,
    enable_feedback: Option<bool>,
    alignment: Option<AlignmentGeometry>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
    background_builder: Option<ButtonLayerBuilder>,
    foreground_builder: Option<ButtonLayerBuilder>,
    size_variant: Option<ButtonSizeVariant>,
    shape_variant: Option<ButtonShapeVariant>,
}

/// The setters of `styleFrom`'s named arguments.
macro_rules! style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `ElevatedButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> ElevatedButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl ElevatedButtonStyleFromBuilder {
    style_from_setters!(
        foreground_color: Color => "foregroundColor",
        background_color: Color => "backgroundColor",
        disabled_foreground_color: Color => "disabledForegroundColor",
        disabled_background_color: Color => "disabledBackgroundColor",
        shadow_color: Color => "shadowColor",
        surface_tint_color: Color => "surfaceTintColor",
        icon_color: Color => "iconColor",
        icon_size: f64 => "iconSize",
        icon_alignment: IconAlignment => "iconAlignment",
        disabled_icon_color: Color => "disabledIconColor",
        overlay_color: Color => "overlayColor",
        elevation: f64 => "elevation",
        text_style: TextStyle => "textStyle",
        padding: EdgeInsetsGeometry => "padding",
        minimum_size: Size => "minimumSize",
        fixed_size: Size => "fixedSize",
        maximum_size: Size => "maximumSize",
        side: BorderSide => "side",
        enabled_mouse_cursor: MouseCursorRef => "enabledMouseCursor",
        disabled_mouse_cursor: MouseCursorRef => "disabledMouseCursor",
        visual_density: VisualDensity => "visualDensity",
        tap_target_size: MaterialTapTargetSize => "tapTargetSize",
        animation_duration: Duration => "animationDuration",
        enable_feedback: bool => "enableFeedback",
        alignment: AlignmentGeometry => "alignment",
        splash_factory: InteractiveInkFeatureFactoryRef => "splashFactory",
        background_builder: ButtonLayerBuilder => "backgroundBuilder",
        foreground_builder: ButtonLayerBuilder => "foregroundBuilder",
        size_variant: ButtonSizeVariant => "sizeVariant",
        shape_variant: ButtonShapeVariant => "shapeVariant",
    );

    /// Dart `ElevatedButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> ElevatedButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The [`ButtonStyle`] `ElevatedButton.styleFrom` returns for the arguments given.
    pub fn build(self) -> ButtonStyle {
        let overlay_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> =
            match (self.foreground_color, self.overlay_color) {
                (None, None) => None,
                (_, Some(overlay_color)) if overlay_color.a == 0.0 => {
                    Some(Rc::new(MaterialStatePropertyAll::new(Some(overlay_color))))
                }
                (_, Some(color)) | (Some(color), None) => {
                    Some(<dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
                        (WidgetState::Pressed.into(), Some(color.with_opacity(0.1))),
                        (WidgetState::Hovered.into(), Some(color.with_opacity(0.08))),
                        (WidgetState::Focused.into(), Some(color.with_opacity(0.1))),
                        (WidgetState::any(), None),
                    ]))
                }
            };

        let mut elevation_value: Option<WidgetStatePropertyRef<Option<f64>>> = None;
        if let Some(elevation) = self.elevation {
            elevation_value = Some(<dyn WidgetStateProperty<Option<f64>>>::from_map(vec![
                (WidgetState::Disabled.into(), Some(0.0)),
                (WidgetState::Pressed.into(), Some(elevation + 6.0)),
                (WidgetState::Hovered.into(), Some(elevation + 2.0)),
                (WidgetState::Focused.into(), Some(elevation + 2.0)),
                (WidgetState::any(), Some(elevation)),
            ]));
        }

        ButtonStyle {
            text_style: Some(Rc::new(MaterialStatePropertyAll::new(self.text_style))),
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
            icon_color: <dyn ButtonStyleButton>::default_color(
                self.icon_color,
                self.disabled_icon_color,
            ),
            icon_size: <dyn ButtonStyleButton>::all_or_null(self.icon_size),
            icon_alignment: self.icon_alignment,
            elevation: elevation_value,
            padding: <dyn ButtonStyleButton>::all_or_null(self.padding),
            minimum_size: <dyn ButtonStyleButton>::all_or_null(self.minimum_size),
            fixed_size: <dyn ButtonStyleButton>::all_or_null(self.fixed_size),
            maximum_size: <dyn ButtonStyleButton>::all_or_null(self.maximum_size),
            side: <dyn ButtonStyleButton>::all_or_null(self.side),
            shape: <dyn ButtonStyleButton>::all_or_null(self.shape),
            mouse_cursor: Some(<dyn WidgetStateProperty<Option<MouseCursorRef>>>::from_map(
                vec![
                    (WidgetState::Disabled.into(), self.disabled_mouse_cursor),
                    (WidgetState::any(), self.enabled_mouse_cursor),
                ],
            )),
            visual_density: self.visual_density,
            tap_target_size: self.tap_target_size,
            animation_duration: self.animation_duration,
            enable_feedback: self.enable_feedback,
            alignment: self.alignment,
            splash_factory: self.splash_factory,
            background_builder: self.background_builder,
            foreground_builder: self.foreground_builder,
            size_variant: self.size_variant,
            icon_button_width: None,
            shape_variant: self.shape_variant,
        }
    }
}

/// Dart's `_scaledPadding`: the default padding for the default font size, scaled by the
/// ambient text scaler.
pub(crate) fn scaled_padding(app: &mut App, context: BuildContext) -> EdgeInsetsGeometry {
    let theme = Theme::of(app, context);
    let padding1x = if theme.use_material3 { 24.0 } else { 16.0 };
    let default_font_size = theme
        .text_theme
        .label_large
        .as_ref()
        .and_then(|label_large| label_large.font_size)
        .unwrap_or(14.0);
    let effective_text_scale =
        MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;

    <dyn ButtonStyleButton>::scaled_padding(
        EdgeInsetsGeometry::symmetric(0.0, padding1x),
        EdgeInsetsGeometry::symmetric(0.0, padding1x / 2.0),
        EdgeInsetsGeometry::symmetric(0.0, padding1x / 2.0 / 2.0),
        effective_text_scale,
    )
}

/// Dart's private `_ElevatedButtonWithIconChild`.
#[derive(Debug)]
struct ElevatedButtonWithIconChild {
    label: WidgetRef,
    icon: WidgetRef,
    button_style: Option<ButtonStyle>,
    icon_alignment: Option<IconAlignment>,
}

impl StatelessWidget for ElevatedButtonWithIconChild {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let elevated_button_theme = ElevatedButtonTheme::of(app, context);
        let use_expressive = Theme::of(app, context).use_material3
            && ElevatedButtonTheme::variant_of(app, context) == StyleVariant::Material3Expressive;
        let spacing = if use_expressive {
            let effective_size = self
                .button_style
                .as_ref()
                .and_then(|style| style.size_variant)
                .or_else(|| {
                    elevated_button_theme
                        .style
                        .as_ref()
                        .and_then(|style| style.size_variant)
                })
                .unwrap_or(ButtonSizeVariant::Small);
            icon_label_space(effective_size)
        } else {
            let default_font_size = self
                .button_style
                .as_ref()
                .and_then(|style| style.text_style.as_ref())
                .and_then(|text_style| text_style.resolve(&WidgetStates::new()))
                .and_then(|text_style| text_style.font_size)
                .unwrap_or(14.0);
            let scale = clamp_double(
                MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0,
                1.0,
                2.0,
            ) - 1.0;
            lerp_double(Some(8.0), Some(4.0), scale).expect("both ends")
        };
        let effective_icon_alignment = self
            .icon_alignment
            .or_else(|| {
                elevated_button_theme
                    .style
                    .as_ref()
                    .and_then(|style| style.icon_alignment)
            })
            .or_else(|| {
                self.button_style
                    .as_ref()
                    .and_then(|style| style.icon_alignment)
            })
            .unwrap_or(IconAlignment::Start);
        Row::new()
            .main_axis_size(MainAxisSize::Min)
            .spacing(spacing)
            .children(if effective_icon_alignment == IconAlignment::Start {
                vec![
                    self.icon.clone(),
                    Flexible::new(self.label.clone()).into_widget(),
                ]
            } else {
                vec![
                    Flexible::new(self.label.clone()).into_widget(),
                    self.icon.clone(),
                ]
            })
            .into_widget()
    }
}

/// Dart's `ElevatedButton.icon(..)` arguments; the value converts to the [`ElevatedButton`].
pub struct ElevatedButtonIcon {
    button_style_button: ButtonStyleButtonData,
    icon: Option<WidgetRef>,
    label: WidgetRef,
    icon_alignment: Option<IconAlignment>,
}

impl ElevatedButtonIcon {
    crate::button_style_button_setters!(
        ElevatedButtonIcon;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller, is_selected,
    );

    /// Dart `ElevatedButton.icon(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> ElevatedButtonIcon {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `ElevatedButton.icon(iconAlignment:)`.
    pub fn icon_alignment(mut self, icon_alignment: IconAlignment) -> ElevatedButtonIcon {
        self.icon_alignment = Some(icon_alignment);
        self
    }

    /// The [`ElevatedButton`] these arguments describe.
    pub fn build(self) -> ElevatedButton {
        let mut button_style_button = self.button_style_button;
        let add_padding = self.icon.is_some();
        button_style_button.child = Some(match self.icon {
            Some(icon) => ElevatedButtonWithIconChild {
                label: self.label,
                icon,
                button_style: button_style_button.style.clone(),
                icon_alignment: self.icon_alignment,
            }
            .into_widget(),
            None => self.label,
        });
        ElevatedButton {
            button_style_button,
            add_padding,
        }
    }
}

impl IntoWidget<StatefulKind> for ElevatedButtonIcon {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{
        FontWeight, Offset, PointerChange, RRect, Rect, TargetPlatform, TextDirection,
    };
    use waft_foundation::Listenable;
    use waft_painting::{Alignment, StadiumBorder};
    use waft_rendering::{
        RenderFlex, RenderFlexMixin, RenderPhysicalModelBase, RenderPhysicalShape,
    };
    use waft_services::{SystemMouseCursor, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Builder, Center, DefaultTextStyle, Directionality, FocusHighlightStrategy,
        FocusManager, FocusNode, GlobalKey, Icon, IconTheme, MouseRegion, Padding, SizedBox, Text,
        UnfocusDisposition, WidgetStatesController, downcast_widget,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::elevated_button_theme::ElevatedButtonThemeData;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::{Material, MaterialType};
    use crate::no_splash::NoSplash;
    use crate::test_support::{
        Drawn, PaintInspection, WidgetTester, ahem_text, material_app, with_main_thread_stack,
    };
    use crate::theme_data::ThemeData;
    use waft_widgets::FocusNodeRef;

    /// A tester with the OS fonts, which the debug banner and an `Icon` need to lay out, and
    /// the Material icons.
    fn tester() -> WidgetTester {
        tester_on(TargetPlatform::Android)
    }

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        crate::icons::install_material_icon_font(&mut tester.cell.borrow_mut());
        tester
    }

    fn pressed() -> Listener {
        Listener::new(|_app| {})
    }

    fn counter(count: &Rc<Cell<usize>>) -> Listener {
        let count = Rc::clone(count);
        Listener::new(move |_app| count.set(count.get() + 1))
    }

    fn text(text: &str) -> WidgetRef {
        ahem_text(text).into_widget()
    }

    fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(MaterialStatePropertyAll::new(Some(value)))
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    fn icon() -> WidgetRef {
        Icon::new(Some(Icons::add())).into_widget()
    }

    fn m3e_theme(tester: &WidgetTester, style: Option<ButtonStyle>) -> ThemeData {
        let mut data = ElevatedButtonThemeData::new().variant(StyleVariant::Material3Expressive);
        data.style = style;
        ThemeData::builder()
            .elevated_button_theme(data)
            .build(&tester.cell.borrow())
    }

    fn is_inside<W: 'static>(tester: &WidgetTester, element: AnyElement) -> bool {
        let ancestors = tester.ancestors(element);
        let app = tester.cell.borrow();
        ancestors
            .into_iter()
            .any(|ancestor| downcast_widget::<W>(&**ancestor.widget(&app)).is_some())
    }

    /// Dart's `find.descendant(of: find.byType(ElevatedButton), matching: find.byType(W))`,
    /// first match.
    fn in_button<W: 'static>(tester: &WidgetTester) -> AnyElement {
        tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| is_inside::<ElevatedButton>(tester, *element))
            .expect("a descendant of the button")
    }

    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget::<Material, R>(in_button::<Material>(tester), read)
    }

    fn padding(tester: &WidgetTester) -> EdgeInsetsGeometry {
        tester.widget::<Padding, _>(in_button::<Padding>(tester), |padding| padding.padding)
    }

    fn row_spacing(tester: &WidgetTester) -> f64 {
        tester.widget::<Row, _>(in_button::<Row>(tester), |row| row.spacing)
    }

    fn button_size(tester: &WidgetTester) -> Size {
        let button = tester.find_by_type::<ElevatedButton>()[0];
        let render = tester.render_box_of(button);
        render.size(&tester.cell.borrow())
    }

    fn material_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(in_button::<Material>(tester)))
    }

    fn icon_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(tester.find_by_type::<Icon>()[0]))
    }

    fn shape_is(tester: &WidgetTester, expected: &dyn waft_painting::ShapeBorder) -> bool {
        material(tester, |material| {
            material
                .shape
                .as_ref()
                .is_some_and(|shape| shape.eq_shape(expected))
        })
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(radius)).into(),
        )
    }

    fn physical_elevation(tester: &WidgetTester) -> f64 {
        let shape = tester.first::<RenderPhysicalShape>();
        shape.elevation(&tester.cell.borrow())
    }

    fn set_highlight_strategy(tester: &WidgetTester, strategy: FocusHighlightStrategy) {
        let mut app = tester.cell.borrow_mut();
        FocusManager::instance(&mut app).set_highlight_strategy(&mut app, strategy);
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    /// The colours of the rects the button's ink features draw.
    fn ink_rect_colors(tester: &WidgetTester) -> Vec<u32> {
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

    /// The colours and font size a probe in the button's child reads: the ambient icon
    /// theme's colour and size, and the default text style's colour and font size.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct Ambient {
        icon_color: Option<Color>,
        icon_size: Option<f64>,
        text_color: Option<Color>,
        font_size: Option<f64>,
    }

    fn probe(seen: &Rc<RefCell<Ambient>>, size: Size) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let icon_theme = IconTheme::of(app, context);
            let text_style = DefaultTextStyle::of(app, context).style;
            *seen.borrow_mut() = Ambient {
                icon_color: icon_theme.color.map(|color| color.color()),
                icon_size: icon_theme.size,
                text_color: text_style.color.map(|color| color.color()),
                font_size: text_style.font_size,
            };
            SizedBox::new()
                .width(size.width())
                .height(size.height())
                .into_widget()
        })
        .into_widget()
    }

    fn mouse_over(tester: &mut WidgetTester, at: Offset) -> crate::test_support::TestPointer {
        let mut gesture = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, at, false);
        gesture
    }

    fn button_center(tester: &WidgetTester) -> Offset {
        let render = tester.render_box_of(tester.find_by_type::<ElevatedButton>()[0]);
        let app = tester.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, size.center(Offset::ZERO), None)
    }

    // elevated_button_test.dart 'ElevatedButton, ElevatedButton.icon defaults'.
    #[test]
    fn the_material_3_defaults_of_an_elevated_button_and_its_icon_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let color_scheme = ColorScheme::light();
            let theme = ThemeData::from(color_scheme.clone()).build(&tester.cell.borrow());
            assert!(theme.use_material3);

            let check = |tester: &WidgetTester, elevation: f64| {
                material(tester, |material| {
                    assert_eq!(material.animation_duration, Duration::from_millis(200));
                    assert!(!material.border_on_foreground);
                    assert!(material.border_radius.is_none());
                    assert_eq!(material.clip_behavior, Clip::None);
                    assert_eq!(material.color, Some(color_scheme.on_primary));
                    assert_eq!(material.elevation, elevation);
                    assert_eq!(material.shadow_color, Some(Color::new(0xff000000)));
                    assert!(
                        material
                            .shape
                            .as_ref()
                            .expect("a shape")
                            .eq_shape(&StadiumBorder::default())
                    );
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(
                        text_style.color.as_ref().map(|color| color.color()),
                        Some(color_scheme.primary)
                    );
                    assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
                    assert_eq!(text_style.font_size, Some(14.0));
                    assert_eq!(text_style.font_weight, Some(FontWeight::W500));
                    assert_eq!(material.r#type, MaterialType::Button);
                });
            };

            // Enabled ElevatedButton
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(ElevatedButton::new(Some(pressed()), Some(text("button"))))
                    .into_widget(),
            ));
            check(&tester, 1.0);
            let align = in_button::<waft_widgets::Align>(&tester);
            tester.widget::<waft_widgets::Align, _>(align, |align| {
                assert!(align.alignment == AlignmentGeometry::from(Alignment::CENTER));
            });

            let center = button_center(&tester);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            tester.pump(Duration::ZERO); // start the splash animation
            tester.pump(Duration::from_millis(100)); // splash is underway

            // Only elevation changes when enabled and pressed.
            check(&tester, 1.0);
            tester.up(&gesture);
            tester.pump_and_settle();

            // Enabled ElevatedButton.icon
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(ElevatedButton::icon(Some(pressed()), text("label")).icon(icon()))
                    .into_widget(),
            ));
            check(&tester, 1.0);

            // Disabled ElevatedButton
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButton::new(None, Some(text("button"))))
                    .into_widget(),
            ));
            // Finish the elevation animation, final background color change.
            tester.pump_and_settle();
            material(&tester, |material| {
                assert_eq!(
                    material.color,
                    Some(color_scheme.on_surface.with_opacity(0.12))
                );
                assert_eq!(material.elevation, 0.0);
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(
                    text_style.color.as_ref().map(|color| color.color()),
                    Some(color_scheme.on_surface.with_opacity(0.38))
                );
                assert_eq!(material.r#type, MaterialType::Button);
            });
        });
    }

    fn captured_default_style(
        tester: &mut WidgetTester,
        theme: ThemeData,
        button: ElevatedButton,
    ) -> ButtonStyle {
        let style = Rc::new(RefCell::new(None));
        let button = Rc::new(button);
        let home = Builder::new({
            let style = Rc::clone(&style);
            let button = Rc::clone(&button);
            move |app, context| {
                *style.borrow_mut() = Some(button.default_style_of(app, context).copy_with(app));
                SizedBox::shrink().into_widget()
            }
        });
        tester.pump_widget(material_app(theme, home.into_widget()));
        style.borrow_mut().take().expect("built")
    }

    fn assert_complete(style: &ButtonStyle) {
        // Properties that must be non-null.
        assert!(style.text_style.is_some(), "textStyle style");
        assert!(style.background_color.is_some(), "backgroundColor style");
        assert!(style.foreground_color.is_some(), "foregroundColor style");
        assert!(style.overlay_color.is_some(), "overlayColor style");
        assert!(style.shadow_color.is_some(), "shadowColor style");
        assert!(style.surface_tint_color.is_some(), "surfaceTintColor style");
        assert!(style.elevation.is_some(), "elevation style");
        assert!(style.padding.is_some(), "padding style");
        assert!(style.minimum_size.is_some(), "minimumSize style");
        assert!(style.maximum_size.is_some(), "maximumSize style");
        assert!(style.icon_color.is_some(), "iconColor style");
        assert!(style.icon_size.is_some(), "iconSize style");
        assert!(style.shape.is_some(), "shape style");
        assert!(style.mouse_cursor.is_some(), "mouseCursor style");
        assert!(style.visual_density.is_some(), "visualDensity style");
        assert!(style.tap_target_size.is_some(), "tapTargetSize style");
        assert!(
            style.animation_duration.is_some(),
            "animationDuration style"
        );
        assert!(style.enable_feedback.is_some(), "enableFeedback style");
        assert!(style.alignment.is_some(), "alignment style");
        assert!(style.splash_factory.is_some(), "splashFactory style");

        // Properties that are expected to be null.
        assert!(style.fixed_size.is_none(), "fixedSize style");
        assert!(style.side.is_none(), "side style");
        assert!(
            style.background_builder.is_none(),
            "backgroundBuilder style"
        );
        assert!(
            style.foreground_builder.is_none(),
            "foregroundBuilder style"
        );
    }

    // elevated_button_test.dart 'ElevatedButton.defaultStyle produces a ButtonStyle with
    // appropriate non-null values' and its 'with an icon' twin.
    #[test]
    fn the_default_style_fills_every_property_but_the_overriding_ones() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
            let style = captured_default_style(
                &mut tester,
                theme.clone(),
                ElevatedButton::new(Some(pressed()), Some(text("button"))),
            );
            assert_complete(&style);
            let style = captured_default_style(
                &mut tester,
                theme,
                ElevatedButton::icon(Some(pressed()), text("button"))
                    .icon(SizedBox::new())
                    .build(),
            );
            assert_complete(&style);
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon produces the correct widgets if icon is
    // null'.
    #[test]
    fn an_icon_button_without_an_icon_shows_only_its_label() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
            let label_key = GlobalKey::new();
            let label = || {
                SizedBox::new()
                    .width(70.0)
                    .height(14.0)
                    .key(label_key.clone().into())
                    .into_widget()
            };
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(ElevatedButton::icon(Some(pressed()), label()).icon(icon()))
                    .into_widget(),
            ));
            assert_eq!(tester.find_by_type::<Icon>().len(), 1);
            assert!(
                label_key
                    .current_context(&tester.cell.borrow_mut())
                    .is_some()
            );

            tester.pump_widget(material_app(
                theme,
                Center::new()
                    // No icon specified.
                    .child(ElevatedButton::icon(Some(pressed()), label()))
                    .into_widget(),
            ));
            assert!(tester.find_by_type::<Icon>().is_empty());
            assert!(
                label_key
                    .current_context(&tester.cell.borrow_mut())
                    .is_some()
            );
            assert!(tester.find_by_type::<Row>().is_empty());
        });
    }

    // elevated_button_test.dart 'ElevatedButton default overlayColor and elevation resolve
    // pressed state'.
    #[test]
    fn the_default_overlay_color_and_elevation_follow_hover_press_and_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            let primary = theme.color_scheme.primary;
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ElevatedButton::new(Some(pressed()), Some(text("ElevatedButton")))
                            .focus_node(focus_node.clone()),
                    )
                    .into_widget(),
            ));

            // Hovered.
            let center = button_center(&tester);
            let mut gesture = mouse_over(&mut tester, center);
            tester.pump_and_settle();
            assert_eq!(physical_elevation(&tester), 3.0);
            assert_eq!(
                ink_rect_colors(&tester),
                vec![argb(primary.with_opacity(0.08))]
            );

            // Highlighted (pressed).
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump_and_settle();
            assert_eq!(physical_elevation(&tester), 1.0);
            assert_eq!(ink_rect_colors(&tester)[1], argb(primary.with_opacity(0.1)));
            // Remove pressed and hovered states
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Focused.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(physical_elevation(&tester), 1.0);
            assert_eq!(
                ink_rect_colors(&tester),
                vec![argb(primary.with_opacity(0.1))]
            );
        });
    }

    fn state_color(states: &WidgetStates) -> Option<Color> {
        if states.contains(&WidgetState::Pressed) {
            return Some(Color::new(0x00000001));
        }
        if states.contains(&WidgetState::Hovered) {
            return Some(Color::new(0x00000002));
        }
        if states.contains(&WidgetState::Focused) {
            return Some(Color::new(0x00000003));
        }
        Some(Color::new(0x00000004))
    }

    // elevated_button_test.dart 'ElevatedButton uses stateful color for text color in
    // different states' and 'ElevatedButton uses stateful color for icon color in different
    // states'.
    #[test]
    fn a_theme_foreground_and_icon_color_resolve_per_state() {
        with_main_thread_stack(|| {
            for with_icon in [false, true] {
                let mut tester = tester();
                let focus_node = focus_node(&tester);
                let seen = Rc::new(RefCell::new(Ambient::default()));
                let child = probe(&seen, Size::new(196.0, 14.0));
                let button = if with_icon {
                    ElevatedButton::icon(Some(pressed()), text("ElevatedButton"))
                        .icon(child)
                        .build()
                } else {
                    ElevatedButton::new(Some(pressed()), Some(child))
                };
                let theme = ThemeData::new(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(ElevatedButtonTheme::new(
                            ElevatedButtonThemeData::new().style(
                                ButtonStyle::new()
                                    .foreground_color(<dyn WidgetStateProperty<_>>::resolve_with(
                                        state_color,
                                    ))
                                    .icon_color(<dyn WidgetStateProperty<_>>::resolve_with(
                                        state_color,
                                    )),
                            ),
                            button.focus_node(focus_node.clone()),
                        ))
                        .into_widget(),
                ));
                let color = |tester: &WidgetTester| {
                    let ambient = *seen.borrow();
                    let _ = tester;
                    if with_icon {
                        ambient.icon_color
                    } else {
                        ambient.text_color
                    }
                };

                // Default, not disabled.
                assert_eq!(color(&tester), Some(Color::new(0x00000004)));

                // Focused.
                focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump_and_settle();
                assert_eq!(color(&tester), Some(Color::new(0x00000003)));

                // Hovered.
                let center = button_center(&tester);
                let gesture = mouse_over(&mut tester, center);
                tester.pump_and_settle();
                assert_eq!(color(&tester), Some(Color::new(0x00000002)));

                // Highlighted (pressed).
                tester.send(&gesture, waft_embedder::PointerChange::Down);
                tester.pump(Duration::ZERO); // Start the splash and highlight animations.
                // Wait for splash and highlight to be well under way.
                tester.pump(Duration::from_millis(800));
                assert_eq!(color(&tester), Some(Color::new(0x00000001)));
            }
        });
    }

    // elevated_button_test.dart 'ElevatedButton onPressed and onLongPress callbacks are
    // correctly called when non-null'.
    #[test]
    fn either_callback_enables_the_button_and_reaches_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let was_pressed = Rc::new(Cell::new(0));
            let build_frame = |on_pressed: Option<Listener>, on_long_press: Option<Listener>| {
                let mut button = ElevatedButton::new(on_pressed, Some(text("button")));
                if let Some(on_long_press) = on_long_press {
                    button = button.on_long_press(on_long_press);
                }
                Directionality::new(TextDirection::Ltr, button).into_widget()
            };
            let enabled = |tester: &WidgetTester| {
                let button = tester.find_by_type::<ElevatedButton>()[0];
                tester.widget::<ElevatedButton, _>(button, |button| button.enabled())
            };

            // onPressed not null, onLongPress null.
            tester.pump_widget(build_frame(Some(counter(&was_pressed)), None));
            assert!(enabled(&tester));
            tester.tap(button_center(&tester), 1);
            assert_eq!(was_pressed.get(), 1);

            // onPressed null, onLongPress not null.
            tester.pump_widget(build_frame(None, Some(counter(&was_pressed))));
            assert!(enabled(&tester));
            tester.long_press(button_center(&tester), 2);
            assert_eq!(was_pressed.get(), 2);

            // onPressed null, onLongPress null.
            tester.pump_widget(build_frame(None, None));
            assert!(!enabled(&tester));
        });
    }

    // elevated_button_test.dart "ElevatedButton response doesn't hover when disabled".
    #[test]
    fn a_disabled_button_gives_up_its_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTouch);
            let focus_node = focus_node(&tester);
            let hovering = Rc::new(Cell::new(false));
            let child_key = GlobalKey::new();
            let on_hover = || -> waft_foundation::ValueChanged<bool> {
                let hovering = Rc::clone(&hovering);
                Rc::new(move |_app, value| hovering.set(value))
            };
            let frame = |button: ElevatedButton| {
                Directionality::new(
                    TextDirection::Ltr,
                    SizedBox::square(Some(100.0)).child(button.focus_node(focus_node.clone())),
                )
                .into_widget()
            };
            tester.pump_widget(frame(
                ElevatedButton::new(
                    Some(pressed()),
                    Some(SizedBox::new().key(child_key.clone().into())),
                )
                .autofocus(true)
                .on_long_press(pressed())
                .on_hover(on_hover()),
            ));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            let center = tester.center_of(&child_key);
            mouse_over(&mut tester, center);
            tester.pump_and_settle();
            assert!(hovering.get());

            tester.pump_widget(frame(
                ElevatedButton::new(None, Some(SizedBox::new().key(child_key.clone().into())))
                    .on_hover(on_hover()),
            ));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    // elevated_button_test.dart 'disabled and hovered ElevatedButton responds to mouse-exit'.
    #[test]
    fn a_disabled_hovered_button_still_reports_the_mouse_leaving() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let on_hover_count = Rc::new(Cell::new(0));
            let hover = Rc::new(Cell::new(false));
            let build_frame = |enabled: bool| {
                let on_hover_count = Rc::clone(&on_hover_count);
                let hover = Rc::clone(&hover);
                Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(
                        SizedBox::square(Some(100.0)).child(
                            ElevatedButton::new(
                                enabled.then(pressed),
                                Some(text("ElevatedButton")),
                            )
                            .on_hover(Rc::new(move |_app, value| {
                                on_hover_count.set(on_hover_count.get() + 1);
                                hover.set(value);
                            })),
                        ),
                    ),
                )
                .into_widget()
            };

            tester.pump_widget(build_frame(true));
            let center = button_center(&tester);
            let mut gesture = mouse_over(&mut tester, center);
            tester.pump_and_settle();
            assert_eq!(on_hover_count.get(), 1);
            assert!(hover.get());

            tester.pump_widget(build_frame(false));
            tester.pump_and_settle();
            tester.move_to(&mut gesture, Offset::ZERO, false);
            // Even though the ElevatedButton has been disabled, the mouse-exit still
            // causes onHover(false) to be called.
            assert_eq!(on_hover_count.get(), 2);
            assert!(!hover.get());

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            // We no longer see hover events because the ElevatedButton is disabled
            // and it's no longer in the "hovering" state.
            assert_eq!(on_hover_count.get(), 2);
            assert!(!hover.get());

            tester.pump_widget(build_frame(true));
            tester.pump_and_settle();
            // The ElevatedButton was enabled while it contained the mouse, however
            // we do not call onHover() because it may call setState().
            assert_eq!(on_hover_count.get(), 2);
            assert!(!hover.get());

            tester.move_to(&mut gesture, center - Offset::new(1.0, 1.0), false);
            tester.pump_and_settle();
            // Moving the mouse a little within the ElevatedButton doesn't change anything.
            assert_eq!(on_hover_count.get(), 2);
            assert!(!hover.get());
        });
    }

    // elevated_button_test.dart 'Can set ElevatedButton focus and Can set unFocus.' and 'When
    // ElevatedButton disable, Can not set ElevatedButton focus.'.
    #[test]
    fn on_focus_change_reports_focus_of_an_enabled_button_only() {
        with_main_thread_stack(|| {
            for enabled in [true, false] {
                let mut tester = tester();
                let node = focus_node(&tester);
                let got_focus = Rc::new(Cell::new(false));
                let on_focus_change: waft_foundation::ValueChanged<bool> = {
                    let got_focus = Rc::clone(&got_focus);
                    Rc::new(move |_app, focused| got_focus.set(focused))
                };
                tester.pump_widget(
                    Directionality::new(
                        TextDirection::Ltr,
                        ElevatedButton::new(enabled.then(pressed), Some(SizedBox::new()))
                            .focus_node(node.clone())
                            .on_focus_change(on_focus_change),
                    )
                    .into_widget(),
                );

                node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump(Duration::ZERO);
                assert_eq!(got_focus.get(), enabled);
                assert_eq!(node.has_focus(&mut tester.cell.borrow_mut()), enabled);

                if enabled {
                    node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
                    tester.pump(Duration::ZERO);
                    assert!(!got_focus.get());
                    assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
                }
            }
        });
    }

    // elevated_button_test.dart 'Does ElevatedButton work with hover', '... with focus' and
    // '... with autofocus'.
    #[test]
    fn a_style_overlay_color_shows_on_hover_focus_and_autofocus() {
        with_main_thread_stack(|| {
            let color = Color::new(0xff001122);
            let overlay = |state: WidgetState| {
                ButtonStyle::new().overlay_color(<dyn WidgetStateProperty<_>>::resolve_with(
                    move |states| states.contains(&state).then_some(color),
                ))
            };
            let ltr = |button: ElevatedButton| {
                Directionality::new(TextDirection::Ltr, button).into_widget()
            };

            let mut tester = tester();
            tester.pump_widget(ltr(ElevatedButton::new(
                Some(pressed()),
                Some(text("button")),
            )
            .style(overlay(WidgetState::Hovered))));
            let center = button_center(&tester);
            mouse_over(&mut tester, center);
            tester.pump_and_settle();
            assert!(ink_rect_colors(&tester).contains(&argb(color)));

            let mut tester = self::tester();
            let node = focus_node(&tester);
            tester.pump_widget(ltr(ElevatedButton::new(
                Some(pressed()),
                Some(text("button")),
            )
            .style(overlay(WidgetState::Focused))
            .focus_node(node.clone())));
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(ink_rect_colors(&tester).contains(&argb(color)));

            let mut tester = self::tester();
            let node = focus_node(&tester);
            tester.pump_widget(ltr(ElevatedButton::new(
                Some(pressed()),
                Some(text("button")),
            )
            .autofocus(true)
            .style(overlay(WidgetState::Focused))
            .focus_node(node)));
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            tester.pump_and_settle();
            assert!(ink_rect_colors(&tester).contains(&argb(color)));
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon responds to applied padding'.
    #[test]
    fn an_icon_button_puts_its_padding_around_the_icon_and_label() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let label_key = GlobalKey::new();
            tester.pump_widget(
                Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(
                        ElevatedButton::icon(
                            Some(pressed()),
                            // Ahem's 'Hello' at the label style's 14 pixels and 1.43 line height,
                            // whose line Flutter lays out 20 pixels high.
                            SizedBox::new()
                                .width(70.0)
                                .height(20.0)
                                .key(label_key.clone().into()),
                        )
                        .icon(icon())
                        .style(
                            ButtonStyle::new()
                                .padding(all(EdgeInsetsGeometry::from_ltrb(16.0, 5.0, 10.0, 12.0))),
                        ),
                    ),
                )
                .into_widget(),
            );

            let padding_rect =
                tester.global_rect(tester.render_box_of(in_button::<Padding>(&tester)));
            let label = label_key
                .current_context(&tester.cell.borrow_mut())
                .expect("mounted");
            let label_rect = tester.global_rect(tester.render_box_of(label));
            let icon_rect = icon_rect(&tester);

            // The right padding should be applied on the right of the label, whereas the
            // left padding should be applied on the left side of the icon.
            assert_eq!(padding_rect.right, label_rect.right + 10.0);
            assert_eq!(padding_rect.left, icon_rect.left - 16.0);
            // Use the taller widget to check the top and bottom padding.
            let taller_widget = if icon_rect.height() > label_rect.height() {
                icon_rect
            } else {
                label_rect
            };
            assert_eq!(padding_rect.top, taller_widget.top - 6.5);
            assert_eq!(padding_rect.bottom, taller_widget.bottom + 13.5);
        });
    }

    // elevated_button_test.dart 'Default ElevatedButton padding for textScaleFactor,
    // textDirection'.
    #[test]
    fn the_material_2_padding_and_icon_gap_scale_with_the_text() {
        with_main_thread_stack(|| {
            let text_scale_factor_options = [0.5, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0];
            // Expected values for each textScaleFactor.
            let padding_without_icon_start = [16.0, 16.0, 14.0, 12.0, 8.0, 6.0, 4.0, 4.0];
            let padding_without_icon_end = [16.0, 16.0, 14.0, 12.0, 8.0, 6.0, 4.0, 4.0];
            let padding_with_icon_start = [12.0, 12.0, 11.0, 10.0, 8.0, 8.0, 8.0, 8.0];
            let padding_with_icon_end = [16.0, 16.0, 14.0, 12.0, 8.0, 6.0, 4.0, 4.0];
            let padding_with_icon_gap = [8.0, 8.0, 7.0, 6.0, 4.0, 4.0, 4.0, 4.0];
            for (index, text_scale_factor) in text_scale_factor_options.into_iter().enumerate() {
                for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    for with_icon in [false, true] {
                        let mut tester = tester();
                        let button_key = GlobalKey::new();
                        let label_key = GlobalKey::new();
                        let icon_key = GlobalKey::new();
                        let theme = ThemeData::builder()
                            .use_material3(false)
                            .color_scheme(ColorScheme::light())
                            .elevated_button_theme(
                                ElevatedButtonThemeData::new().style(
                                    ElevatedButton::style_from()
                                        .minimum_size(Size::new(64.0, 36.0))
                                        .build(),
                                ),
                            )
                            .build(&tester.cell.borrow());
                        let label = ahem_text("button").key(label_key.clone().into());
                        let button = if with_icon {
                            ElevatedButton::icon(Some(pressed()), label)
                                .key(button_key.clone().into())
                                .icon(
                                    Icon::new(Some(Icons::add()))
                                        .size(18.0)
                                        .key(icon_key.clone().into()),
                                )
                                .build()
                        } else {
                            ElevatedButton::new(Some(pressed()), Some(label))
                                .key(button_key.clone().into())
                        };
                        // Dart's `Scaffold(body:)` is the `Material` the text sits on.
                        tester.pump_widget(material_app(
                            theme,
                            MediaQuery::with_clamped_text_scaling(
                                None,
                                text_scale_factor,
                                text_scale_factor,
                                Directionality::new(
                                    text_direction,
                                    Material::new().child(Center::new().child(button)),
                                ),
                            ),
                        ));

                        let padding_element = in_button::<Padding>(&tester);
                        assert_eq!(
                            Directionality::of(&mut tester.cell.borrow_mut(), padding_element),
                            text_direction
                        );

                        // Compute expected padding, and check.
                        let (expected_start, expected_end) = if with_icon {
                            (padding_with_icon_start[index], padding_with_icon_end[index])
                        } else {
                            (
                                padding_without_icon_start[index],
                                padding_without_icon_end[index],
                            )
                        };
                        let expected_padding =
                            EdgeInsetsGeometry::from_steb(expected_start, 0.0, expected_end, 0.0)
                                .resolve(Some(text_direction));
                        assert_eq!(
                            padding(&tester).resolve(Some(text_direction)),
                            expected_padding,
                            "text scale {text_scale_factor}, icon {with_icon}, {text_direction:?}"
                        );

                        // Measure padding in terms of the difference between the button and
                        // its label child and check that.
                        let label_bounds = tester
                            .global_rect(tester.render_object(&label_key).as_box().expect("a box"));
                        let icon_bounds = with_icon.then(|| {
                            tester.global_rect(
                                tester.render_object(&icon_key).as_box().expect("a box"),
                            )
                        });
                        let child_bounds = match icon_bounds {
                            Some(icon_bounds) => label_bounds.expand_to_include(icon_bounds),
                            None => label_bounds,
                        };

                        // We measure the `InkResponse` descendant of the button element,
                        // because the button has a larger `RenderBox` which accommodates the
                        // minimum tap target with a height of 48.
                        let button_bounds =
                            tester.global_rect(tester.render_box_of(in_button::<InkWell>(&tester)));

                        // Since there is a requirement of a minimum width of 64 and a minimum
                        // height of 36 on material buttons, the visual padding of smaller
                        // buttons may not match their settings. Therefore, we only test
                        // buttons that are large enough.
                        if button_bounds.width() > 64.0 {
                            assert_eq!(
                                child_bounds.left - button_bounds.left,
                                expected_padding.left
                            );
                            assert_eq!(
                                button_bounds.right - child_bounds.right,
                                expected_padding.right
                            );
                        }
                        if button_bounds.height() > 36.0 {
                            assert_eq!(child_bounds.top - button_bounds.top, expected_padding.top);
                            assert_eq!(
                                button_bounds.bottom - child_bounds.bottom,
                                expected_padding.bottom
                            );
                        }

                        // Check the gap between the icon and the label
                        if let Some(icon_bounds) = icon_bounds {
                            let gap_width = if text_direction == TextDirection::Ltr {
                                label_bounds.left - icon_bounds.right
                            } else {
                                icon_bounds.left - label_bounds.right
                            };
                            assert_eq!(gap_width, padding_with_icon_gap[index]);
                        }
                    }
                }
            }
        });
    }

    // elevated_button_test.dart 'Override theme fontSize changes padding', 'Override
    // ElevatedButton default padding', 'M3 ElevatedButton has correct padding' and 'M3
    // ElevatedButton.icon has correct padding'.
    #[test]
    fn the_material_3_padding_follows_the_font_size_the_style_and_the_icon() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mut text_theme = crate::text_theme::TextTheme::new();
            text_theme.label_large = Some(TextStyle::new().font_size(28.0));
            let theme = ThemeData::from(ColorScheme::light())
                .text_theme(text_theme)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButton::new(Some(pressed()), Some(text("text"))))
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::symmetric(0.0, 12.0));

            let mut tester = self::tester();
            let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                MediaQuery::with_clamped_text_scaling(
                    None,
                    2.0,
                    2.0,
                    Center::new().child(
                        ElevatedButton::new(Some(pressed()), Some(text("ElevatedButton"))).style(
                            ElevatedButton::style_from()
                                .padding(EdgeInsetsGeometry::all(22.0))
                                .build(),
                        ),
                    ),
                ),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::all(22.0));

            let mut tester = self::tester();
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(ElevatedButton::new(
                        Some(pressed()),
                        Some(text("ElevatedButton")),
                    ))
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::symmetric(0.0, 24.0));

            let mut tester = self::tester();
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ElevatedButton::icon(Some(pressed()), text("ElevatedButton")).icon(icon()),
                    )
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::from_steb(16.0, 0.0, 24.0, 0.0));
        });
    }

    // elevated_button_test.dart 'ElevatedButton with NoSplash splashFactory paints nothing'.
    #[test]
    fn a_no_splash_factory_draws_no_splash_and_a_ripple_draws_one() {
        with_main_thread_stack(|| {
            for (splash_factory, circles) in [
                (NoSplash::splash_factory(), 0),
                (InkRipple::splash_factory(), 1),
            ] {
                let mut tester = tester();
                let theme = ThemeData::new(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            ElevatedButton::new(Some(pressed()), Some(text("test"))).style(
                                ElevatedButton::style_from()
                                    .splash_factory(splash_factory)
                                    .build(),
                            ),
                        )
                        .into_widget(),
                ));
                let center = button_center(&tester);
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, center);
                // No circle is recorded on the frame the ripple's fade-in starts at alpha 0,
                // where Dart's matcher counts that call, so the splash starts a frame earlier.
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(200));
                let drawn = tester
                    .ink_drawn()
                    .into_iter()
                    .filter(|drawn| matches!(drawn, Drawn::Path(..)))
                    .count();
                assert_eq!(drawn, circles);
                tester.up(&gesture);
                tester.pump_and_settle();
            }
        });
    }

    // elevated_button_test.dart 'ElevatedButton uses InkRipple when useMaterial3 is false',
    // under every platform it names.
    #[test]
    fn a_material_2_button_splashes_with_ripples_on_every_platform() {
        with_main_thread_stack(|| {
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::IOS,
                TargetPlatform::Linux,
                TargetPlatform::MacOS,
                TargetPlatform::Windows,
            ] {
                let mut tester = tester_on(platform);
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(ElevatedButton::new(Some(pressed()), Some(text("button"))))
                        .into_widget(),
                ));
                let splash_factory = tester
                    .widget::<InkWell, _>(in_button::<InkWell>(&tester), |ink_well| {
                        ink_well.ink_response.splash_factory.clone()
                    });
                assert!(Rc::ptr_eq(
                    &splash_factory.expect("a splash factory"),
                    &InkRipple::splash_factory()
                ));
            }
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon icon,label layout'.
    #[test]
    fn an_icon_button_lays_out_icon_gap_and_label_centred() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let icon_key = GlobalKey::new();
            let label_key = GlobalKey::new();
            let style = ElevatedButton::style_from()
                .padding(EdgeInsetsGeometry::ZERO)
                .visual_density(VisualDensity::STANDARD) // dx=0, dy=0
                .build();
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                // Dart's `Scaffold(body:)` lays its body out loosely at the top left.
                waft_widgets::Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(200.0).child(
                            ElevatedButton::icon(
                                Some(pressed()),
                                SizedBox::new()
                                    .width(50.0)
                                    .height(100.0)
                                    .key(label_key.clone().into()),
                            )
                            .icon(
                                SizedBox::new()
                                    .width(50.0)
                                    .height(100.0)
                                    .key(icon_key.clone().into())
                                    .into_widget(),
                            )
                            .key(button_key.clone().into())
                            .style(style),
                        ),
                    )
                    .into_widget(),
            ));

            // The button's label and icon are separated by a gap of 8:
            // 46 [icon 50] 8 [label 50] 46
            // The overall button width is 200. So:
            // icon.x = 46
            // label.x = 46 + 50 + 8 = 104
            let rect = |key: &GlobalKey| {
                let context = key
                    .current_context(&tester.cell.borrow_mut())
                    .expect("mounted");
                tester.global_rect(tester.render_box_of(context))
            };
            assert_eq!(rect(&button_key), Rect::from_ltrb(0.0, 0.0, 200.0, 100.0));
            assert_eq!(rect(&icon_key), Rect::from_ltrb(46.0, 0.0, 96.0, 100.0));
            assert_eq!(rect(&label_key), Rect::from_ltrb(104.0, 0.0, 154.0, 100.0));
        });
    }

    // elevated_button_test.dart 'Default ElevatedButton icon alignment', 'ElevatedButton
    // icon alignment can be customized' and 'ElevatedButton icon alignment respects
    // ButtonStyle.iconAlignment'.
    #[test]
    fn the_icon_sits_at_the_start_or_end_by_its_alignment_and_the_text_direction() {
        with_main_thread_stack(|| {
            let build = |tester: &mut WidgetTester,
                         text_direction: TextDirection,
                         icon_alignment: Option<IconAlignment>,
                         style_alignment: Option<IconAlignment>| {
                let mut button = ElevatedButton::icon(Some(pressed()), text("button")).icon(icon());
                if let Some(icon_alignment) = icon_alignment {
                    button = button.icon_alignment(icon_alignment);
                }
                if let Some(style_alignment) = style_alignment {
                    button = button.style(ButtonStyle::new().icon_alignment(style_alignment));
                }
                let theme = ThemeData::new(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Directionality::new(text_direction, Center::new().child(button)).into_widget(),
                ));
            };
            // 16.0 - padding between icon and button edge at the start, 24.0 at the end.
            let mut tester = tester();
            build(&mut tester, TextDirection::Ltr, None, None);
            assert_eq!(material_rect(&tester).left, icon_rect(&tester).left - 16.0);
            build(&mut tester, TextDirection::Rtl, None, None);
            assert_eq!(
                material_rect(&tester).right,
                icon_rect(&tester).right + 16.0
            );

            build(
                &mut tester,
                TextDirection::Ltr,
                Some(IconAlignment::Start),
                None,
            );
            assert_eq!(material_rect(&tester).left, icon_rect(&tester).left - 16.0);
            build(
                &mut tester,
                TextDirection::Ltr,
                Some(IconAlignment::End),
                None,
            );
            assert_eq!(
                material_rect(&tester).right,
                icon_rect(&tester).right + 24.0
            );
            build(
                &mut tester,
                TextDirection::Rtl,
                Some(IconAlignment::Start),
                None,
            );
            assert_eq!(
                material_rect(&tester).right,
                icon_rect(&tester).right + 16.0
            );
            build(
                &mut tester,
                TextDirection::Rtl,
                Some(IconAlignment::End),
                None,
            );
            assert_eq!(material_rect(&tester).left, icon_rect(&tester).left - 24.0);

            build(
                &mut tester,
                TextDirection::Ltr,
                None,
                Some(IconAlignment::End),
            );
            assert_eq!(
                material_rect(&tester).right,
                icon_rect(&tester).right + 24.0
            );
        });
    }

    // elevated_button_test.dart 'ElevatedButton.styleFrom can be used to set foreground and
    // background colors' and 'ElevatedButton.styleFrom can customize the button icon'.
    #[test]
    fn style_from_turns_each_argument_into_its_property() {
        let red = Color::new(0xffff0000);
        let green = Color::new(0xff00ff00);
        let blue = Color::new(0xff0000ff);
        let none = WidgetStates::new();
        let disabled = states(&[WidgetState::Disabled]);
        let hovered = states(&[WidgetState::Hovered]);
        let focused = states(&[WidgetState::Focused]);
        let pressed = states(&[WidgetState::Pressed]);
        let style = ElevatedButton::style_from()
            .foreground_color(red)
            .disabled_foreground_color(green)
            .background_color(blue)
            .disabled_background_color(red)
            .icon_color(green)
            .disabled_icon_color(blue)
            .icon_size(32.0)
            .icon_alignment(IconAlignment::End)
            .elevation(4.0)
            .shadow_color(green)
            .surface_tint_color(blue)
            .size_variant(ButtonSizeVariant::Large)
            .shape_variant(ButtonShapeVariant::Square)
            .build();
        let resolve = |property: &Option<WidgetStatePropertyRef<Option<Color>>>,
                       states: &WidgetStates| {
            property
                .as_ref()
                .and_then(|property| property.resolve(states))
        };
        assert_eq!(resolve(&style.foreground_color, &none), Some(red));
        assert_eq!(resolve(&style.foreground_color, &disabled), Some(green));
        assert_eq!(resolve(&style.background_color, &none), Some(blue));
        assert_eq!(resolve(&style.background_color, &disabled), Some(red));
        assert_eq!(resolve(&style.icon_color, &none), Some(green));
        assert_eq!(resolve(&style.icon_color, &disabled), Some(blue));
        assert_eq!(resolve(&style.shadow_color, &none), Some(green));
        assert_eq!(resolve(&style.surface_tint_color, &none), Some(blue));
        // The overlay is the foreground color at the default opacities.
        assert_eq!(resolve(&style.overlay_color, &none), None);
        assert_eq!(
            resolve(&style.overlay_color, &hovered),
            Some(red.with_opacity(0.08))
        );
        assert_eq!(
            resolve(&style.overlay_color, &focused),
            Some(red.with_opacity(0.1))
        );
        assert_eq!(
            resolve(&style.overlay_color, &pressed),
            Some(red.with_opacity(0.1))
        );
        let elevation = style.elevation.as_ref().expect("an elevation");
        assert_eq!(elevation.resolve(&none), Some(4.0));
        assert_eq!(elevation.resolve(&disabled), Some(0.0));
        assert_eq!(elevation.resolve(&hovered), Some(6.0));
        assert_eq!(elevation.resolve(&focused), Some(6.0));
        assert_eq!(elevation.resolve(&pressed), Some(10.0));
        assert_eq!(
            style
                .icon_size
                .as_ref()
                .and_then(|size| size.resolve(&none)),
            Some(32.0)
        );
        assert_eq!(style.icon_alignment, Some(IconAlignment::End));
        assert_eq!(style.size_variant, Some(ButtonSizeVariant::Large));
        assert_eq!(style.shape_variant, Some(ButtonShapeVariant::Square));
        // The text style property is always given, null when no style is.
        assert_eq!(
            style
                .text_style
                .as_ref()
                .map(|text_style| text_style.resolve(&none)),
            Some(None)
        );

        // A transparent overlay color defeats the highlights; an opaque one replaces the
        // foreground's.
        let transparent = ElevatedButton::style_from()
            .foreground_color(red)
            .overlay_color(Color::new(0x00000000))
            .build();
        assert_eq!(
            resolve(&transparent.overlay_color, &hovered),
            Some(Color::new(0x00000000))
        );
        assert_eq!(
            resolve(&transparent.overlay_color, &none),
            Some(Color::new(0x00000000))
        );
        let overlay = ElevatedButton::style_from()
            .foreground_color(red)
            .overlay_color(blue)
            .build();
        assert_eq!(
            resolve(&overlay.overlay_color, &hovered),
            Some(blue.with_opacity(0.08))
        );

        // Nothing given overrides nothing but the two always-given properties.
        let empty = ElevatedButton::style_from().build();
        assert!(empty.background_color.is_none());
        assert!(empty.foreground_color.is_none());
        assert!(empty.overlay_color.is_none());
        assert!(empty.elevation.is_none());
        assert!(empty.padding.is_none());
        assert!(empty.shape.is_none());
        assert!(empty.text_style.is_some());
        let cursor = empty.mouse_cursor.as_ref().expect("a mouse cursor");
        assert!(cursor.resolve(&none).is_none());
        assert!(cursor.resolve(&disabled).is_none());
    }

    // elevated_button_test.dart 'ElevatedButton.styleFrom can be used to set foreground and
    // background colors', on a built button.
    #[test]
    fn a_style_from_button_shows_its_colors_and_icon_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let seen = Rc::new(RefCell::new(Ambient::default()));
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ElevatedButton::icon(Some(pressed()), text("button"))
                            .icon(probe(&seen, Size::new(18.0, 18.0)))
                            .style(
                                ElevatedButton::style_from()
                                    .foreground_color(Color::new(0xffffffff))
                                    .background_color(Color::new(0xff000000))
                                    .icon_color(Color::new(0xffff0000))
                                    .icon_size(32.0)
                                    .build(),
                            ),
                    )
                    .into_widget(),
            ));
            material(&tester, |material| {
                assert_eq!(material.color, Some(Color::new(0xff000000)));
                assert_eq!(
                    material
                        .text_style
                        .as_ref()
                        .and_then(|text_style| text_style.color.as_ref())
                        .map(|color| color.color()),
                    Some(Color::new(0xffffffff))
                );
            });
            assert_eq!(seen.borrow().icon_color, Some(Color::new(0xffff0000)));
            assert_eq!(seen.borrow().icon_size, Some(32.0));
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon does not lose focus when icon is
    // nullified'.
    #[test]
    fn an_icon_button_keeps_its_focus_when_its_icon_goes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let build = |tester: &mut WidgetTester, icon: Option<WidgetRef>| {
                let button = ElevatedButton::icon(Some(pressed()), text("button"));
                let button = match icon {
                    Some(icon) => button.icon(icon),
                    None => button,
                };
                let theme = ThemeData::new(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(button.focus_node(focus_node.clone()).autofocus(true))
                        .into_widget(),
                ));
            };
            build(&mut tester, Some(self::icon()));
            tester.pump_and_settle();
            assert!(focus_node.has_focus(&mut tester.cell.borrow_mut()));
            build(&mut tester, None);
            tester.pump_and_settle();
            assert!(focus_node.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    // elevated_button_test.dart 'M3E ElevatedButton resolves selected and unselected colors'.
    #[test]
    fn an_expressive_toggle_button_fills_with_primary_when_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3e_theme(&tester, None);
            let colors = theme.color_scheme.clone();
            for (is_selected, color) in [
                (false, colors.surface_container_low()),
                (true, colors.primary),
            ] {
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new()
                        .child(
                            ElevatedButton::new(Some(pressed()), Some(text("Button")))
                                .is_selected(is_selected),
                        )
                        .into_widget(),
                ));
                tester.pump_and_settle();
                assert_eq!(material(&tester, |material| material.color), Some(color));
            }
        });
    }

    // elevated_button_test.dart 'M3E ElevatedButton uses size variants', 'M3E ElevatedButton
    // keeps size-specific padding when text scales', 'M3E ElevatedButton.icon uses
    // size-specific icon-label spacing' and 'ElevatedButton.styleFrom supports M3E size and
    // shape variants'.
    #[test]
    fn an_expressive_button_takes_its_size_padding_gap_and_shape_from_its_variants() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3e_theme(&tester, None);
            let medium = ButtonStyle::new().size_variant(ButtonSizeVariant::Medium);
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(
                        ElevatedButton::new(Some(pressed()), Some(text("Button")))
                            .style(medium.clone()),
                    )
                    .into_widget(),
            ));
            assert_eq!(button_size(&tester).height(), 56.0);

            for text_scale_factor in [1.0, 2.0, 3.0] {
                for with_icon in [false, true] {
                    let button = if with_icon {
                        ElevatedButton::icon(Some(pressed()), text("Button"))
                            .icon(icon())
                            .build()
                    } else {
                        ElevatedButton::new(Some(pressed()), Some(text("Button")))
                    };
                    tester.pump_widget(material_app(
                        theme.clone(),
                        MediaQuery::with_clamped_text_scaling(
                            None,
                            text_scale_factor,
                            text_scale_factor,
                            Center::new().child(button.style(medium.clone())),
                        ),
                    ));
                    assert!(padding(&tester) == EdgeInsetsGeometry::symmetric(0.0, 24.0));
                }
            }

            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(
                        ElevatedButton::icon(Some(pressed()), text("Button"))
                            .icon(icon())
                            .style(ButtonStyle::new().size_variant(ButtonSizeVariant::Large)),
                    )
                    .into_widget(),
            ));
            assert_eq!(row_spacing(&tester), 12.0);

            let style = ElevatedButton::style_from()
                .size_variant(ButtonSizeVariant::Large)
                .shape_variant(ButtonShapeVariant::Square)
                .build();
            assert_eq!(style.size_variant, Some(ButtonSizeVariant::Large));
            assert_eq!(style.shape_variant, Some(ButtonShapeVariant::Square));
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButton::new(Some(pressed()), Some(text("Button"))).style(style))
                    .into_widget(),
            ));
            tester.pump_and_settle();
            assert_eq!(button_size(&tester).height(), 96.0);
            assert!(shape_is(&tester, &rounded(28.0)));
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'uses size-specific height, icon
    // size, and label font size'.
    #[test]
    fn each_expressive_size_has_its_height_icon_size_and_font_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3e_theme(&tester, None);
            // Font sizes of labelLarge, titleMedium, headlineSmall, and headlineLarge.
            let expectations = [
                (ButtonSizeVariant::XSmall, 32.0, 20.0, 14.0),
                (ButtonSizeVariant::Small, 40.0, 20.0, 14.0),
                (ButtonSizeVariant::Medium, 56.0, 24.0, 16.0),
                (ButtonSizeVariant::Large, 96.0, 32.0, 24.0),
                (ButtonSizeVariant::XLarge, 136.0, 40.0, 32.0),
            ];
            for (size_variant, height, icon_size, font_size) in expectations {
                let icon_seen = Rc::new(RefCell::new(Ambient::default()));
                let label_seen = Rc::new(RefCell::new(Ambient::default()));
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new()
                        .child(
                            ElevatedButton::icon(
                                Some(pressed()),
                                probe(&label_seen, Size::new(84.0, 14.0)),
                            )
                            .icon(probe(&icon_seen, Size::new(18.0, 18.0)))
                            .style(ButtonStyle::new().size_variant(size_variant)),
                        )
                        .into_widget(),
                ));
                // The icon size and text style animate between size variants.
                tester.pump_and_settle();

                assert_eq!(material_rect(&tester).height(), height, "{size_variant:?}");
                assert_eq!(
                    icon_seen.borrow().icon_size,
                    Some(icon_size),
                    "{size_variant:?}"
                );
                assert_eq!(
                    label_seen.borrow().font_size,
                    Some(font_size),
                    "{size_variant:?}"
                );
            }
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'xSmall keeps a 48dp tap target
    // when the theme shrink-wraps'.
    #[test]
    fn an_extra_small_expressive_button_keeps_its_48_tap_target() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let press_count = Rc::new(Cell::new(0));
            let build_app =
                |tester: &mut WidgetTester, tap_target_size: Option<MaterialTapTargetSize>| {
                    let theme = m3e_theme(tester, None)
                        .copy_with()
                        .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                        .build();
                    let mut style = ButtonStyle::new().size_variant(ButtonSizeVariant::XSmall);
                    style.tap_target_size = tap_target_size;
                    tester.pump_widget(material_app(
                        theme,
                        Center::new()
                            .child(
                                ElevatedButton::new(
                                    Some(counter(&press_count)),
                                    Some(text("Button")),
                                )
                                .style(style),
                            )
                            .into_widget(),
                    ));
                };

            build_app(&mut tester, None);
            assert_eq!(material_rect(&tester).height(), 32.0);
            assert_eq!(button_size(&tester).height(), 48.0);

            // Taps just above and below the 32dp button still reach it.
            let center = button_center(&tester);
            for (pointer, offset) in [(1, Offset::new(0.0, 23.0)), (2, Offset::new(0.0, -23.0))] {
                tester.tap(center + offset, pointer);
                tester.pump(Duration::ZERO);
            }
            tester.pump_and_settle();
            assert_eq!(press_count.get(), 2);

            // An explicit tap target size in the style overrides the default.
            build_app(&mut tester, Some(MaterialTapTargetSize::ShrinkWrap));
            assert_eq!(button_size(&tester).height(), 32.0);
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'ignores the theme visual
    // density'.
    #[test]
    fn an_expressive_button_ignores_the_theme_visual_density() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_app = |tester: &mut WidgetTester, visual_density: Option<VisualDensity>| {
                // Compact is the default density on desktop platforms.
                let theme = m3e_theme(tester, None)
                    .copy_with()
                    .visual_density(VisualDensity::COMPACT)
                    .build();
                let mut style = ButtonStyle::new();
                style.visual_density = visual_density;
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            ElevatedButton::new(Some(pressed()), Some(text("Button"))).style(style),
                        )
                        .into_widget(),
                ));
            };

            build_app(&mut tester, None);
            assert_eq!(material_rect(&tester).height(), 40.0);
            assert_eq!(button_size(&tester).height(), 48.0);

            // An explicit visual density in the style overrides the default.
            build_app(&mut tester, Some(VisualDensity::COMPACT));
            assert_eq!(material_rect(&tester).height(), 32.0);
            assert_eq!(button_size(&tester).height(), 40.0);
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'resolves shapes for shape
    // variants and states'.
    #[test]
    fn the_expressive_shapes_follow_the_shape_variant_and_the_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected = states(&[WidgetState::Selected]);
            let pressed_states = states(&[WidgetState::Pressed]);
            let shape = |style: &ButtonStyle, states: &WidgetStates| {
                style
                    .shape
                    .as_ref()
                    .and_then(|shape| shape.resolve(states))
                    .expect("a shape")
            };
            let theme = m3e_theme(&tester, None);

            let style = captured_default_style(
                &mut tester,
                theme.clone(),
                ElevatedButton::new(Some(pressed()), Some(text("Button")))
                    .is_selected(false)
                    .style(ButtonStyle::new().size_variant(ButtonSizeVariant::Medium)),
            );
            assert!(shape(&style, &WidgetStates::new()).eq_shape(&StadiumBorder::default()));
            assert!(shape(&style, &selected).eq_shape(&rounded(16.0)));
            assert!(shape(&style, &pressed_states).eq_shape(&rounded(12.0)));

            let style = captured_default_style(
                &mut tester,
                theme.clone(),
                ElevatedButton::new(Some(pressed()), Some(text("Button")))
                    .is_selected(false)
                    .style(
                        ButtonStyle::new()
                            .size_variant(ButtonSizeVariant::Medium)
                            .shape_variant(ButtonShapeVariant::Square),
                    ),
            );
            assert!(shape(&style, &WidgetStates::new()).eq_shape(&rounded(16.0)));
            assert!(shape(&style, &selected).eq_shape(&StadiumBorder::default()));
            assert!(shape(&style, &pressed_states).eq_shape(&rounded(12.0)));

            // A button that is not toggleable ignores the selected state.
            let style = captured_default_style(
                &mut tester,
                theme,
                ElevatedButton::new(Some(pressed()), Some(text("Button"))),
            );
            assert!(shape(&style, &selected).eq_shape(&StadiumBorder::default()));
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'resolves colors and elevation
    // for each state' and 'resolves selected and unselected colors when toggleable'.
    #[test]
    fn the_expressive_colors_and_elevation_resolve_per_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3e_theme(&tester, None);
            let colors = theme.color_scheme.clone();
            let alpha =
                |color: Color, alpha: f64| color.with_values(Some(alpha), None, None, None, None);
            let resolve = |property: &Option<WidgetStatePropertyRef<Option<Color>>>,
                           states: &WidgetStates| {
                property
                    .as_ref()
                    .and_then(|property| property.resolve(states))
            };
            let none = WidgetStates::new();
            let disabled = states(&[WidgetState::Disabled]);
            let hovered = states(&[WidgetState::Hovered]);
            let focused = states(&[WidgetState::Focused]);
            let pressed_states = states(&[WidgetState::Pressed]);
            let selected = states(&[WidgetState::Selected]);

            let style = captured_default_style(
                &mut tester,
                theme.clone(),
                ElevatedButton::new(Some(pressed()), Some(text("Button"))),
            );
            assert_eq!(
                resolve(&style.background_color, &none),
                Some(colors.surface_container_low())
            );
            assert_eq!(
                resolve(&style.background_color, &disabled),
                Some(alpha(colors.on_surface, 0.1))
            );
            assert_eq!(
                resolve(&style.foreground_color, &none),
                Some(colors.primary)
            );
            assert_eq!(
                resolve(&style.foreground_color, &disabled),
                Some(alpha(colors.on_surface, 0.38))
            );
            assert_eq!(resolve(&style.icon_color, &none), Some(colors.primary));
            assert_eq!(
                resolve(&style.icon_color, &disabled),
                Some(alpha(colors.on_surface, 0.38))
            );
            assert_eq!(resolve(&style.overlay_color, &none), None);
            assert_eq!(
                resolve(&style.overlay_color, &hovered),
                Some(alpha(colors.primary, 0.08))
            );
            assert_eq!(
                resolve(&style.overlay_color, &focused),
                Some(alpha(colors.primary, 0.1))
            );
            assert_eq!(
                resolve(&style.overlay_color, &pressed_states),
                Some(alpha(colors.primary, 0.1))
            );
            let elevation = style.elevation.as_ref().expect("an elevation");
            assert_eq!(elevation.resolve(&none), Some(1.0));
            assert_eq!(elevation.resolve(&hovered), Some(1.0));
            assert_eq!(elevation.resolve(&focused), Some(1.0));
            assert_eq!(elevation.resolve(&pressed_states), Some(1.0));
            assert_eq!(elevation.resolve(&disabled), Some(0.0));

            let style = captured_default_style(
                &mut tester,
                theme,
                ElevatedButton::new(Some(pressed()), Some(text("Button"))).is_selected(true),
            );
            assert_eq!(
                resolve(&style.background_color, &selected),
                Some(colors.primary)
            );
            assert_eq!(
                resolve(&style.foreground_color, &selected),
                Some(colors.on_primary)
            );
            assert_eq!(
                resolve(&style.icon_color, &selected),
                Some(colors.on_primary)
            );
            assert_eq!(
                resolve(
                    &style.overlay_color,
                    &states(&[WidgetState::Selected, WidgetState::Hovered])
                ),
                Some(alpha(colors.on_primary, 0.08))
            );
            assert_eq!(
                resolve(&style.background_color, &none),
                Some(colors.surface_container_low())
            );
            assert_eq!(
                resolve(&style.foreground_color, &none),
                Some(colors.primary)
            );
            assert_eq!(
                resolve(
                    &style.background_color,
                    &states(&[WidgetState::Selected, WidgetState::Disabled])
                ),
                Some(alpha(colors.on_surface, 0.1))
            );
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'uses theme-level size and shape
    // variants' and 'ElevatedButton.icon uses theme-level size for icon-label spacing'.
    #[test]
    fn the_theme_size_and_shape_variants_apply_under_the_widget_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3e_theme(
                &tester,
                Some(
                    ButtonStyle::new()
                        .size_variant(ButtonSizeVariant::Large)
                        .shape_variant(ButtonShapeVariant::Square),
                ),
            );
            let build_app = |tester: &mut WidgetTester, style: Option<ButtonStyle>| {
                let mut button = ElevatedButton::new(Some(pressed()), Some(text("Button")));
                if let Some(style) = style {
                    button = button.style(style);
                }
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new().child(button).into_widget(),
                ));
            };
            build_app(&mut tester, None);
            assert_eq!(material_rect(&tester).height(), 96.0);
            assert!(shape_is(&tester, &rounded(28.0)));

            // The widget style overrides the theme style.
            build_app(
                &mut tester,
                Some(ButtonStyle::new().size_variant(ButtonSizeVariant::XSmall)),
            );
            tester.pump_and_settle();
            assert_eq!(material_rect(&tester).height(), 32.0);
            assert!(shape_is(&tester, &rounded(12.0)));

            let mut tester = self::tester();
            let theme = m3e_theme(
                &tester,
                Some(ButtonStyle::new().size_variant(ButtonSizeVariant::XLarge)),
            );
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButton::icon(Some(pressed()), text("Button")).icon(icon()))
                    .into_widget(),
            ));
            assert_eq!(row_spacing(&tester), 16.0);
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'variant is ignored when
    // useMaterial3 is false'.
    #[test]
    fn a_material_2_theme_ignores_the_expressive_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::builder()
                .use_material3(false)
                .elevated_button_theme(
                    ElevatedButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ElevatedButton::new(Some(pressed()), Some(text("Button")))
                            .style(ButtonStyle::new().size_variant(ButtonSizeVariant::Large)),
                    )
                    .into_widget(),
            ));
            assert_eq!(material_rect(&tester).height(), 36.0);
            assert!(shape_is(&tester, &rounded(4.0)));
        });
    }

    // elevated_button_test.dart group 'M3E ElevatedButton', 'does not crash at zero area'.
    #[test]
    fn an_expressive_button_of_every_size_fits_a_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for size in [
                ButtonSizeVariant::XSmall,
                ButtonSizeVariant::Small,
                ButtonSizeVariant::Medium,
                ButtonSizeVariant::Large,
                ButtonSizeVariant::XLarge,
            ] {
                let theme = m3e_theme(&tester, Some(ButtonStyle::new().size_variant(size)));
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            SizedBox::shrink().child(
                                ElevatedButton::icon(Some(pressed()), text("X"))
                                    .icon(icon())
                                    .is_selected(true),
                            ),
                        )
                        .into_widget(),
                ));
                assert_eq!(button_size(&tester), Size::ZERO);
            }
        });
    }

    // elevated_button_test.dart 'ElevatedButton statesController selected override is
    // visual-only until isSelected changes', without its semantics.
    #[test]
    fn a_controller_selected_state_holds_until_is_selected_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);
            let theme = m3e_theme(&tester, None);
            let colors = theme.color_scheme.clone();
            let build_button = |tester: &mut WidgetTester, is_selected: bool| {
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new()
                        .child(
                            ElevatedButton::new(Some(pressed()), Some(text("Button")))
                                .states_controller(controller.clone())
                                .is_selected(is_selected),
                        )
                        .into_widget(),
                ));
            };
            let background_color =
                |tester: &WidgetTester| material(tester, |material| material.color);

            build_button(&mut tester, false);
            assert_eq!(
                background_color(&tester),
                Some(colors.surface_container_low())
            );

            // Updating the controller changes only the appearance.
            controller.update(&mut tester.cell.borrow_mut(), WidgetState::Selected, true);
            tester.pump(Duration::ZERO);
            assert_eq!(background_color(&tester), Some(colors.primary));

            // Rebuilding with the same isSelected keeps the controller's value.
            build_button(&mut tester, false);
            assert!(
                tester
                    .cell
                    .borrow()
                    .get(&controller)
                    .value()
                    .contains(&WidgetState::Selected)
            );
            assert_eq!(background_color(&tester), Some(colors.primary));

            // Changing isSelected overwrites the controller's value.
            build_button(&mut tester, true);
            build_button(&mut tester, false);
            assert!(
                !tester
                    .cell
                    .borrow()
                    .get(&controller)
                    .value()
                    .contains(&WidgetState::Selected)
            );
            assert_eq!(
                background_color(&tester),
                Some(colors.surface_container_low())
            );
        });
    }

    // elevated_button_test.dart 'ElevatedButton debugFillProperties includes selected'.
    #[test]
    fn the_debug_description_names_a_selected_button() {
        let button = ElevatedButton::new(Some(pressed()), Some(SizedBox::new())).is_selected(true);
        assert!(format!("{button:?}").contains("selected"));
    }

    // An `ElevatedButton::icon` chain gives the same child whatever order its setters run in.
    #[test]
    fn the_icon_child_sees_the_style_and_alignment_in_any_order() {
        let style = ButtonStyle::new().size_variant(ButtonSizeVariant::Large);
        let child_of = |button: &ElevatedButton| {
            let child = button.button_style_button.child.clone().expect("a child");
            let child =
                downcast_widget::<ElevatedButtonWithIconChild>(&*child).expect("the icon child");
            (
                child
                    .button_style
                    .as_ref()
                    .and_then(|style| style.size_variant),
                child.icon_alignment,
            )
        };
        let first = ElevatedButton::icon(Some(pressed()), text("a"))
            .icon(icon())
            .style(style.clone())
            .icon_alignment(IconAlignment::End)
            .build();
        let second = ElevatedButton::icon(Some(pressed()), text("a"))
            .icon_alignment(IconAlignment::End)
            .style(style)
            .icon(icon())
            .build();
        assert_eq!(
            child_of(&first),
            (Some(ButtonSizeVariant::Large), Some(IconAlignment::End))
        );
        assert_eq!(child_of(&first), child_of(&second));
    }

    // elevated_button_test.dart 'By default, ElevatedButton shape outline is defined by
    // shape.side'.
    #[test]
    fn by_default_the_shape_side_is_the_outline() {
        // This is a regression test for https://github.com/flutter/flutter/issues/69544
        with_main_thread_stack(|| {
            let mut tester = tester();
            let border_color = Color::new(0xff4caf50);
            let theme = ThemeData::builder()
                .color_scheme(ColorScheme::light())
                .text_theme(crate::typography::Typography::english_like2014())
                .use_material3(false)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ElevatedButton::new(Some(pressed()), Some(text("button"))).style(
                            ElevatedButton::style_from()
                                .shape(RoundedRectangleBorder::new(
                                    BorderSide {
                                        width: 10.0,
                                        color: border_color,
                                        ..BorderSide::default()
                                    },
                                    BorderRadius::all(Radius::circular(16.0)).into(),
                                ))
                                .minimum_size(Size::new(64.0, 36.0))
                                .build(),
                        ),
                    )
                    .into_widget(),
            ));

            let button = tester.render_box_of(tester.find_by_type::<ElevatedButton>()[0]);
            assert!(tester.paints_drrect(
                button.as_object(),
                border_color,
                // Outer and inner rect that give the outline a width of 10.
                RRect::from_ltrbr(0.0, 0.0, 116.0, 36.0, Radius::circular(16.0)),
                RRect::from_ltrbr(10.0, 10.0, 106.0, 26.0, Radius::circular(16.0 - 10.0)),
            ));
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon does not overflow'. Dart's
    // `tester.takeException()` is the overflow its debug paint reports; Waft paints no overflow
    // indicator, so the case reads the overflow of the button's `Row`.
    #[test]
    fn an_icon_button_with_a_long_label_does_not_overflow() {
        // Regression test for https://github.com/flutter/flutter/issues/77815
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                // Dart's `Scaffold(body:)` lays its body out loosely at the top left.
                waft_widgets::Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(200.0).child(
                            ElevatedButton::icon(
                                Some(pressed()),
                                Text::new(
                                    // Much wider than 200
                                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Ut a euismod nibh. Morbi laoreet purus.",
                                ),
                            )
                            .icon(Icon::new(Some(Icons::add()))),
                        ),
                    )
                    .into_widget(),
            ));
            let flexes = tester.all::<RenderFlex>();
            let app = tester.cell.borrow();
            for flex in flexes {
                assert!(!flex.has_overflow(&app));
            }
        });
    }

    // elevated_button_test.dart 'ElevatedButton maximumSize'. Dart's heights, 224, are 16
    // lines of Ahem's 14 pixel glyphs, which the test platform lacks; the widths need none.
    #[test]
    fn the_maximum_size_bounds_the_width_of_a_button_and_of_an_icon_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key0 = GlobalKey::new();
            let key1 = GlobalKey::new();
            let theme = ThemeData::builder()
                .text_theme(crate::typography::Typography::english_like2014())
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Center::new().child(
                            waft_widgets::Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .children(vec![
                                    ElevatedButton::new(
                                        Some(pressed()),
                                        Some(Text::new("A B C D E F G H I J K L M N O P")),
                                    )
                                    .key(key0.clone().into())
                                    .style(
                                        ElevatedButton::style_from()
                                            .minimum_size(Size::new(24.0, 36.0))
                                            .maximum_size(Size::from_width(64.0))
                                            .build(),
                                    )
                                    .into_widget(),
                                    ElevatedButton::icon(
                                        Some(pressed()),
                                        Text::new("A B C D E F G H I J K L M N O P"),
                                    )
                                    .key(key1.clone().into())
                                    .style(
                                        ElevatedButton::style_from()
                                            .minimum_size(Size::new(24.0, 36.0))
                                            .maximum_size(Size::from_width(104.0))
                                            .build(),
                                    )
                                    .icon(
                                        waft_widgets::Container::new()
                                            .color(Colors::RED.primary())
                                            .width(32.0)
                                            .height(32.0),
                                    )
                                    .into_widget(),
                                ]),
                        ),
                    )
                    .into_widget(),
            ));

            let width = |key: &GlobalKey| {
                let render = tester.render_object(key).as_box().expect("a box");
                render.size(&tester.cell.borrow()).width()
            };
            assert_eq!(width(&key0), 64.0);
            assert_eq!(width(&key1), 104.0);
        });
    }

    // elevated_button_test.dart 'ElevatedButton changes mouse cursor when hovered'.
    #[test]
    fn style_from_cursors_show_while_hovered_and_the_default_is_basic() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = |on_pressed: Option<Listener>, custom: bool| {
                let mut button = ElevatedButton::new(on_pressed, Some(text("button")));
                if custom {
                    button = button.style(
                        ElevatedButton::style_from()
                            .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
                            .disabled_mouse_cursor(SystemMouseCursors::GRAB.into())
                            .build(),
                    );
                }
                Directionality::new(
                    TextDirection::Ltr,
                    MouseRegion::new()
                        .cursor(SystemMouseCursors::FORBIDDEN.into())
                        .child(button),
                )
                .into_widget()
            };
            let is = |tester: &WidgetTester, cursor: SystemMouseCursor| {
                tester.active_cursor(1) == MouseCursorRef::from(cursor)
            };

            tester.pump_widget(frame(Some(pressed()), true));
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump(Duration::ZERO);
            assert!(is(&tester, SystemMouseCursors::TEXT));

            // Test cursor when disabled
            tester.pump_widget(frame(None, true));
            assert!(is(&tester, SystemMouseCursors::GRAB));

            // Test default cursor
            tester.pump_widget(frame(Some(pressed()), false));
            assert!(is(
                &tester,
                if K_IS_WEB {
                    SystemMouseCursors::CLICK
                } else {
                    SystemMouseCursors::BASIC
                }
            ));

            // Test default cursor when disabled
            tester.pump_widget(frame(None, false));
            assert!(is(&tester, SystemMouseCursors::BASIC));
        });
    }

    // elevated_button_test.dart 'Ink Response shape matches Material shape'.
    #[test]
    fn the_ink_response_shape_is_the_material_shape_with_the_style_side() {
        // This is a regression test for https://github.com/flutter/flutter/issues/91844
        with_main_thread_stack(|| {
            let mut tester = tester();
            let shape_side = BorderSide {
                color: Color::new(0xff0000ff),
                width: 0.0,
                ..BorderSide::default()
            };
            let build_frame = |tester: &WidgetTester, side: Option<BorderSide>| {
                let mut style_from = ElevatedButton::style_from().shape(
                    RoundedRectangleBorder::new(shape_side, BorderRadius::ZERO.into()),
                );
                if let Some(side) = side {
                    style_from = style_from.side(side);
                }
                let theme = ThemeData::new(&tester.cell.borrow());
                material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(
                                ElevatedButton::new(Some(pressed()), Some(text("ElevatedButton")))
                                    .style(style_from.build()),
                            ),
                        )
                        .into_widget(),
                )
            };
            let custom_border_is = |tester: &WidgetTester, expected: RoundedRectangleBorder| {
                tester.widget(in_button::<InkWell>(tester), |ink_well: &InkWell| {
                    ink_well
                        .ink_response
                        .custom_border
                        .as_ref()
                        .is_some_and(|border| border.eq_shape(&expected))
                })
            };

            let border_side = BorderSide {
                width: 10.0,
                color: Color::new(0xff00ff00),
                ..BorderSide::default()
            };
            let frame = build_frame(&tester, Some(border_side));
            tester.pump_widget(frame);
            assert!(custom_border_is(
                &tester,
                RoundedRectangleBorder::new(border_side, BorderRadius::ZERO.into())
            ));

            let frame = build_frame(&tester, None);
            tester.pump_widget(frame);
            tester.pump_and_settle();
            assert!(custom_border_is(
                &tester,
                RoundedRectangleBorder::new(shape_side, BorderRadius::ZERO.into())
            ));
        });
    }

    // elevated_button_test.dart 'ElevatedButton.icon statesController'; the plain button's case
    // is button_style_button.rs's.
    #[test]
    fn an_icon_buttons_states_controller_follows_hover_press_and_disable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let count = Rc::new(Cell::new(0));
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.add_listener(&mut tester.cell.borrow_mut(), {
                let count = Rc::clone(&count);
                Listener::new(move |_app| count.set(count.get() + 1))
            });
            let label_key = GlobalKey::new();
            let frame = |tester: &WidgetTester, enabled: bool| {
                let theme = ThemeData::new(&tester.cell.borrow());
                material_app(
                    theme,
                    Center::new()
                        .child(
                            ElevatedButton::icon(
                                enabled.then(pressed),
                                ahem_text("button").key(label_key.clone().into()),
                            )
                            .states_controller(controller.clone())
                            .icon(icon()),
                        )
                        .into_widget(),
                )
            };
            let value =
                |tester: &WidgetTester| tester.cell.borrow().get(&controller).value().clone();

            let first = frame(&tester, true);
            tester.pump_widget(first);
            assert_eq!(value(&tester), states(&[]));
            assert_eq!(count.get(), 0);

            let center = tester.center_of(&label_key);
            let mut gesture = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 1);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[]));
            assert_eq!(count.get(), 2);

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 3);

            tester.send(&gesture, PointerChange::Down);
            tester.pump_and_settle();
            assert_eq!(
                value(&tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 4);

            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 5);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[]));
            assert_eq!(count.get(), 6);

            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert_eq!(
                value(&tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 8); // adds hovered and pressed - two changes

            // If the button is rebuilt disabled, then the pressed state is
            // removed.
            let disabled = frame(&tester, false);
            tester.pump_widget(disabled);
            tester.pump_and_settle();
            assert_eq!(
                value(&tester),
                states(&[WidgetState::Hovered, WidgetState::Disabled])
            );
            assert_eq!(count.get(), 10); // removes pressed and adds disabled - two changes
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.pump_and_settle();
            assert_eq!(value(&tester), states(&[WidgetState::Disabled]));
            assert_eq!(count.get(), 11);
        });
    }
}
