//! Flutter counterpart: material_ui's `filled_button.dart`, with its parts
//! `generated/filled_button_defaults_m3.g.dart` and
//! `generated/filled_tonal_button_defaults_m3.g.dart`.
//!
//! Dart's `FilledButton.icon`, `FilledButton.tonalIcon` and `FilledButton.styleFrom` take
//! named arguments the body combines (the icon child reads `style`; `styleFrom` derives its
//! overlay color from two of them), so each is a value collecting them, [`FilledButtonIcon`]
//! and [`FilledButtonStyleFromBuilder`], that converts once every argument is given.
#![allow(deprecated)]

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Clip, Color, Size, clamp_double, lerp_double};
use waft_foundation::{App, Listener};
use waft_painting::{
    AlignmentGeometry, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef, TextStyle,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Flexible, IntoWidget, KeyRef, MainAxisSize, MediaQuery, Row, StatefulKind,
    StatefulWidget, StatelessWidget, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyleGetters;
use crate::button_style::{ButtonLayerBuilder, ButtonStyle};
use crate::button_style_button::{
    ButtonStyleButton, ButtonStyleButtonData, ButtonStyleState, IconAlignment,
};
use crate::filled_button_theme::FilledButtonTheme;
use crate::generated::filled_button_defaults_m3::FilledButtonDefaultsM3;
use crate::generated::filled_tonal_button_defaults_m3::FilledTonalButtonDefaultsM3;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilledButtonVariant {
    Filled,
    Tonal,
}

/// A Material Design filled button.
///
/// Filled buttons have the most visual impact after the `FloatingActionButton`,
/// and should be used for important, final actions that complete a flow,
/// like **Save**, **Join now**, or **Confirm**.
///
/// A filled button is a label `child` displayed on a `Material`
/// widget. The label's `Text` and `Icon` widgets are displayed in
/// `style`'s [`ButtonStyle::foreground_color`] and the button's filled
/// background is the [`ButtonStyle::background_color`].
///
/// The filled button's default style is defined by
/// [`default_style_of`](ButtonStyleButton::default_style_of). The style of this filled
/// button can be overridden with its `style` parameter. The style of all filled
/// buttons in a subtree can be overridden with the
/// [`FilledButtonTheme`], and the style of all of the filled
/// buttons in an app can be overridden with the `Theme`'s
/// `ThemeData::filled_button_theme` property.
///
/// The static [`style_from`](Self::style_from) method is a convenient way to create a
/// filled button [`ButtonStyle`] from simple values.
///
/// If `on_pressed` and `on_long_press` callbacks are null, then the
/// button will be disabled.
///
/// To create a 'filled tonal' button, use [`FilledButton::tonal`].
///
/// ## Visual density effects
///
/// The button's appearance is affected by the [`VisualDensity`] from the enclosing
/// `Theme` or from its `style`. Visual density adjusts the [`ButtonStyle::padding`]
/// and [`ButtonStyle::minimum_size`] to accommodate different UI densities across platforms.
/// See [`VisualDensity`] for more details on how it affects component layout and
/// the platform-specific defaults.
///
/// See also:
///
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * `TextButton`, a button with no outline or fill color.
///  * <https://material.io/design/components/buttons.html>
///  * <https://m3.material.io/components/buttons>
pub struct FilledButton {
    button_style_button: ButtonStyleButtonData,
    variant: FilledButtonVariant,
    add_padding: bool,
}

impl FilledButton {
    /// Create a FilledButton.
    pub fn new<K>(on_pressed: Option<Listener>, child: Option<impl IntoWidget<K>>) -> FilledButton {
        FilledButton::with_child(
            FilledButtonVariant::Filled,
            on_pressed,
            child.map(|child| child.into_widget()),
        )
    }

    /// Create a filled button from [`icon`](FilledButtonIcon::icon) and `label`.
    ///
    /// The icon and label are arranged in a row with padding at the start and end
    /// and a gap between them.
    ///
    /// If [`icon`](FilledButtonIcon::icon) is null, this constructor will create a
    /// [`FilledButton`] that doesn't display an icon.
    ///
    /// The alignment of the icon within the button is set by
    /// [`icon_alignment`](FilledButtonIcon::icon_alignment); see [`IconAlignment`].
    pub fn icon<K>(on_pressed: Option<Listener>, label: impl IntoWidget<K>) -> FilledButtonIcon {
        FilledButtonIcon::new(FilledButtonVariant::Filled, on_pressed, label.into_widget())
    }

    /// Create a tonal variant of FilledButton.
    ///
    /// A filled tonal button is an alternative middle ground between
    /// [`FilledButton`] and `OutlinedButton`. They’re useful in contexts where
    /// a lower-priority button requires slightly more emphasis than an
    /// outline would give, such as "Next" in an onboarding flow.
    pub fn tonal<K>(
        on_pressed: Option<Listener>,
        child: Option<impl IntoWidget<K>>,
    ) -> FilledButton {
        FilledButton::with_child(
            FilledButtonVariant::Tonal,
            on_pressed,
            child.map(|child| child.into_widget()),
        )
    }

    /// Create a filled tonal button from [`icon`](FilledButtonIcon::icon) and `label`.
    ///
    /// The [`icon`](FilledButtonIcon::icon) and `label` are arranged in a row with padding
    /// at the start and end and a gap between them.
    ///
    /// If [`icon`](FilledButtonIcon::icon) is null, this constructor will create a
    /// [`FilledButton`] that doesn't display an icon.
    pub fn tonal_icon<K>(
        on_pressed: Option<Listener>,
        label: impl IntoWidget<K>,
    ) -> FilledButtonIcon {
        FilledButtonIcon::new(FilledButtonVariant::Tonal, on_pressed, label.into_widget())
    }

    /// The initializer list of the constructors without an icon.
    fn with_child(
        variant: FilledButtonVariant,
        on_pressed: Option<Listener>,
        child: Option<WidgetRef>,
    ) -> FilledButton {
        let mut button_style_button = ButtonStyleButtonData::new(on_pressed, child);
        button_style_button.clip_behavior = Some(Clip::None);
        FilledButton {
            button_style_button,
            variant,
            add_padding: false,
        }
    }

    crate::button_style_button_setters!(
        FilledButton;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller,
    );

    /// A static convenience method that constructs a filled button
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
    /// All parameters default to null, by default this method returns
    /// a [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default text and icon colors for a
    /// [`FilledButton`], as well as its overlay color, with all of the
    /// standard opacity adjustments for the pressed, focused, and
    /// hovered states, one could write:
    ///
    /// ```text
    /// FilledButton::new(on_pressed, Text::new("Filled button"))
    ///     .style(FilledButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    /// ```
    ///
    /// or for a Filled tonal variant:
    ///
    /// ```text
    /// FilledButton::tonal(on_pressed, Text::new("Filled tonal button"))
    ///     .style(FilledButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    /// ```
    pub fn style_from() -> FilledButtonStyleFromBuilder {
        FilledButtonStyleFromBuilder::default()
    }
}

impl ButtonStyleButton for FilledButton {
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
    /// * `text_style` - Theme.textTheme.labelLarge
    /// * `background_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.12)
    ///   * others - Theme.colorScheme.secondaryContainer
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others - Theme.colorScheme.onSecondaryContainer
    /// * `overlay_color`
    ///   * hovered - Theme.colorScheme.onSecondaryContainer(0.08)
    ///   * focused or pressed - Theme.colorScheme.onSecondaryContainer(0.12)
    /// * `shadow_color` - Theme.colorScheme.shadow
    /// * `surface_tint_color` - null
    /// * `elevation`
    ///   * disabled - 0
    ///   * default - 0
    ///   * hovered - 1
    ///   * focused or pressed - 0
    /// * `padding`
    ///   * `default font size <= 14` - horizontal(16)
    ///   * `14 < default font size <= 28` - lerp(horizontal(16), horizontal(8))
    ///   * `28 < default font size <= 36` - lerp(horizontal(8), horizontal(4))
    ///   * `36 < default font size` - horizontal(4)
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
    /// The default padding values for the [`FilledButton::icon`] factory are slightly
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
    ///   * others
    ///     - Theme.colorScheme.primary in `FilledButton`
    ///     - Theme.colorScheme.secondaryContainer in `FilledButton.tonal`
    /// * `foreground_color`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others
    ///     - Theme.colorScheme.onPrimary in `FilledButton`
    ///     - Theme.colorScheme.onSecondaryContainer in `FilledButton.tonal`
    /// * `overlay_color`
    ///   * hovered
    ///     - Theme.colorScheme.onPrimary(0.08) in `FilledButton`
    ///     - Theme.colorScheme.onSecondaryContainer(0.08) in `FilledButton.tonal`
    ///   * focused or pressed
    ///     - Theme.colorScheme.onPrimary(0.1) in `FilledButton`
    ///     - Theme.colorScheme.onSecondaryContainer(0.1) in `FilledButton.tonal`
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
    /// For the [`FilledButton::icon`] factory, the start (generally the left) value of
    /// [`ButtonStyle::padding`] is reduced from 24 to 16.
    fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let button_style: Box<dyn ButtonStyleGetters> = match self.variant {
            FilledButtonVariant::Filled => Box::new(FilledButtonDefaultsM3::new(context)),
            FilledButtonVariant::Tonal => Box::new(FilledTonalButtonDefaultsM3::new(context)),
        };

        if self.add_padding {
            let use_material3 = Theme::of(app, context).use_material3;
            let default_font_size = button_style
                .text_style(app)
                .and_then(|text_style| text_style.resolve(&WidgetStates::new()))
                .and_then(|text_style| text_style.font_size)
                .unwrap_or(14.0);
            let effective_text_scale =
                MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;

            let scaled_padding = if use_material3 {
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

    /// Returns the `FilledButtonThemeData::style` of the closest
    /// [`FilledButtonTheme`] ancestor.
    fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        FilledButtonTheme::of(app, context).style
    }
}

impl StatefulWidget for FilledButton {
    type State = ButtonStyleState<FilledButton>;

    fn key(&self) -> Option<&KeyRef> {
        self.button_style_button.key.as_ref()
    }

    fn create_state(&self) -> ButtonStyleState<FilledButton> {
        ButtonStyleState::new()
    }
}

impl Debug for FilledButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.button_style_button, f)
    }
}

/// Dart's `FilledButton.styleFrom`, collecting its arguments; [`build`](Self::build) is the
/// call.
#[derive(Clone, Default)]
pub struct FilledButtonStyleFromBuilder {
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
}

/// The setters of `styleFrom`'s named arguments.
macro_rules! style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `FilledButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> FilledButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl FilledButtonStyleFromBuilder {
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
    );

    /// Dart `FilledButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> FilledButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The [`ButtonStyle`] `FilledButton.styleFrom` returns for the arguments given.
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
            elevation: <dyn ButtonStyleButton>::all_or_null(self.elevation),
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
            size_variant: None,
            icon_button_width: None,
            shape_variant: None,
        }
    }
}

/// Dart's `_scaledPadding`: the default padding for the default font size, scaled by the
/// ambient text scaler.
pub(crate) fn scaled_padding(app: &mut App, context: BuildContext) -> EdgeInsetsGeometry {
    let theme = Theme::of(app, context);
    let default_font_size = theme
        .text_theme
        .label_large
        .as_ref()
        .and_then(|label_large| label_large.font_size)
        .unwrap_or(14.0);
    let effective_text_scale =
        MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;
    let padding1x = if theme.use_material3 { 24.0 } else { 16.0 };
    <dyn ButtonStyleButton>::scaled_padding(
        EdgeInsetsGeometry::symmetric(0.0, padding1x),
        EdgeInsetsGeometry::symmetric(0.0, padding1x / 2.0),
        EdgeInsetsGeometry::symmetric(0.0, padding1x / 2.0 / 2.0),
        effective_text_scale,
    )
}

/// Dart's private `_FilledButtonWithIconChild`.
#[derive(Debug)]
struct FilledButtonWithIconChild {
    label: WidgetRef,
    icon: WidgetRef,
    button_style: Option<ButtonStyle>,
    icon_alignment: Option<IconAlignment>,
}

impl StatelessWidget for FilledButtonWithIconChild {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
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

        let filled_button_theme = FilledButtonTheme::of(app, context);
        let effective_icon_alignment = self
            .icon_alignment
            .or_else(|| {
                filled_button_theme
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
            .spacing(lerp_double(Some(8.0), Some(4.0), scale).expect("both ends"))
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

/// Dart's `FilledButton.icon(..)` and `FilledButton.tonalIcon(..)` arguments; the value
/// converts to the [`FilledButton`].
pub struct FilledButtonIcon {
    button_style_button: ButtonStyleButtonData,
    variant: FilledButtonVariant,
    icon: Option<WidgetRef>,
    label: WidgetRef,
    icon_alignment: Option<IconAlignment>,
}

impl FilledButtonIcon {
    /// The arguments of the constructor of `variant`, before its setters.
    fn new(
        variant: FilledButtonVariant,
        on_pressed: Option<Listener>,
        label: WidgetRef,
    ) -> FilledButtonIcon {
        let mut button_style_button = ButtonStyleButtonData::new(on_pressed, None);
        button_style_button.clip_behavior = Some(Clip::None);
        FilledButtonIcon {
            button_style_button,
            variant,
            icon: None,
            label,
            icon_alignment: None,
        }
    }

    crate::button_style_button_setters!(
        FilledButtonIcon;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller,
    );

    /// Dart `FilledButton.icon(icon:)` and `FilledButton.tonalIcon(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> FilledButtonIcon {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `FilledButton.icon(iconAlignment:)` and `FilledButton.tonalIcon(iconAlignment:)`.
    pub fn icon_alignment(mut self, icon_alignment: IconAlignment) -> FilledButtonIcon {
        self.icon_alignment = Some(icon_alignment);
        self
    }

    /// The [`FilledButton`] these arguments describe.
    pub fn build(self) -> FilledButton {
        let mut button_style_button = self.button_style_button;
        let add_padding = self.icon.is_some();
        button_style_button.child = Some(match self.icon {
            Some(icon) => FilledButtonWithIconChild {
                label: self.label,
                icon,
                button_style: button_style_button.style.clone(),
                icon_alignment: self.icon_alignment,
            }
            .into_widget(),
            None => self.label,
        });
        FilledButton {
            button_style_button,
            variant: self.variant,
            add_padding,
        }
    }
}

impl IntoWidget<StatefulKind> for FilledButtonIcon {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{FontWeight, Offset, Rect, TargetPlatform, TextDirection};
    use waft_embedder::{PointerChange, RRect, Radius};
    use waft_foundation::{K_IS_WEB, Listenable};
    use waft_painting::{Alignment, StadiumBorder};
    use waft_painting::{BorderRadius, RoundedRectangleBorder};
    use waft_rendering::{RenderPhysicalModelBase, RenderPhysicalShape};
    use waft_services::{SystemMouseCursor, SystemMouseCursors};
    use waft_widgets::{
        Align, AnyElement, Builder, Center, Column, Directionality, FocusNode, GlobalKey, Icon,
        IconData, IconTheme, MouseRegion, Padding, SizedBox, downcast_widget,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::filled_button_theme::FilledButtonThemeData;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::{Material, MaterialType};
    use crate::material_state::MaterialStatesController;
    use crate::test_support::{
        Drawn, WidgetTester, ahem_text, material_app, with_main_thread_stack,
    };
    use crate::theme_data::ThemeData;
    use waft_widgets::FocusNodeRef;

    /// A tester with the OS fonts, which the debug banner and an `Icon` need to lay out, and
    /// the Material icons.
    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        crate::icons::install_material_icon_font(&mut tester.cell.borrow_mut());
        tester
    }

    fn pressed() -> Listener {
        Listener::new(|_app| {})
    }

    fn text(text: &str) -> WidgetRef {
        ahem_text(text).into_widget()
    }

    fn icon() -> WidgetRef {
        Icon::new(Some(Icons::add())).into_widget()
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    fn is_inside<W: 'static>(tester: &WidgetTester, element: AnyElement) -> bool {
        let ancestors = tester.ancestors(element);
        let app = tester.cell.borrow();
        ancestors
            .into_iter()
            .any(|ancestor| downcast_widget::<W>(&**ancestor.widget(&app)).is_some())
    }

    /// The first `W` below a `FilledButton`.
    fn in_button<W: 'static>(tester: &WidgetTester) -> AnyElement {
        tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| is_inside::<FilledButton>(tester, *element))
            .expect("a descendant of the button")
    }

    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget::<Material, R>(in_button::<Material>(tester), read)
    }

    fn text_color(material: &Material) -> Option<Color> {
        material
            .text_style
            .as_ref()
            .and_then(|text_style| text_style.color.as_ref())
            .map(|color| color.color())
    }

    fn padding(tester: &WidgetTester) -> EdgeInsetsGeometry {
        tester.widget::<Padding, _>(in_button::<Padding>(tester), |padding| padding.padding)
    }

    fn material_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(in_button::<Material>(tester)))
    }

    fn icon_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(tester.find_by_type::<Icon>()[0]))
    }

    fn button_center(tester: &WidgetTester) -> Offset {
        let render = tester.render_box_of(tester.find_by_type::<FilledButton>()[0]);
        let app = tester.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, size.center(Offset::ZERO), None)
    }

    fn physical_elevation(tester: &WidgetTester) -> f64 {
        let shape = tester.first::<RenderPhysicalShape>();
        shape.elevation(&tester.cell.borrow())
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

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

    fn check_defaults(tester: &WidgetTester, color: Color, foreground: Color, elevation: f64) {
        material(tester, |material| {
            assert_eq!(material.animation_duration, Duration::from_millis(200));
            assert!(!material.border_on_foreground);
            assert!(material.border_radius.is_none());
            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.color, Some(color));
            assert_eq!(material.elevation, elevation);
            assert_eq!(material.shadow_color, Some(Color::new(0xff000000)));
            assert!(
                material
                    .shape
                    .as_ref()
                    .expect("a shape")
                    .eq_shape(&StadiumBorder::default())
            );
            assert_eq!(text_color(material), Some(foreground));
            let text_style = material.text_style.as_ref().expect("a text style");
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.font_size, Some(14.0));
            assert_eq!(text_style.font_weight, Some(FontWeight::W500));
            assert_eq!(material.r#type, MaterialType::Button);
        });
    }

    // filled_button_test.dart 'FilledButton, FilledButton.icon defaults' and
    // 'FilledButton.tonal, FilledButton.tonalIcon defaults'.
    #[test]
    fn the_defaults_of_a_filled_and_a_tonal_button_and_their_icon_variants() {
        with_main_thread_stack(|| {
            let color_scheme = ColorScheme::light();
            for tonal in [false, true] {
                let mut tester = tester();
                let theme = ThemeData::from(color_scheme.clone())
                    .use_material3(tonal)
                    .build(&tester.cell.borrow());
                let (color, foreground) = if tonal {
                    (
                        color_scheme.secondary_container(),
                        color_scheme.on_secondary_container(),
                    )
                } else {
                    (color_scheme.primary, color_scheme.on_primary)
                };
                let plain = |child: WidgetRef, on_pressed: Option<Listener>| {
                    if tonal {
                        FilledButton::tonal(on_pressed, Some(child))
                    } else {
                        FilledButton::new(on_pressed, Some(child))
                    }
                };

                // Enabled FilledButton
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new()
                        .child(plain(text("button"), Some(pressed())))
                        .into_widget(),
                ));
                check_defaults(&tester, color, foreground, 0.0);
                tester.widget::<Align, _>(in_button::<Align>(&tester), |align| {
                    assert!(align.alignment == AlignmentGeometry::from(Alignment::CENTER));
                });

                let center = button_center(&tester);
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, center);
                tester.pump(Duration::ZERO); // start the splash animation
                tester.pump(Duration::from_millis(100)); // splash is underway
                check_defaults(&tester, color, foreground, 0.0);
                tester.up(&gesture);
                tester.pump_and_settle();

                // Enabled FilledButton.icon
                let with_icon = if tonal {
                    FilledButton::tonal_icon(Some(pressed()), text("label")).icon(icon())
                } else {
                    FilledButton::icon(Some(pressed()), text("label")).icon(icon())
                };
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new().child(with_icon).into_widget(),
                ));
                check_defaults(&tester, color, foreground, 0.0);

                // Disabled FilledButton
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(plain(text("button"), None))
                        .into_widget(),
                ));
                // Finish the elevation animation, final background color change.
                tester.pump_and_settle();
                check_defaults(
                    &tester,
                    color_scheme.on_surface.with_opacity(0.12),
                    color_scheme.on_surface.with_opacity(0.38),
                    0.0,
                );
            }
        });
    }

    fn captured_default_style(
        tester: &mut WidgetTester,
        theme: ThemeData,
        button: FilledButton,
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

    // filled_button_test.dart 'FilledButton.defaultStyle produces a ButtonStyle with
    // appropriate non-null values' and its 'with an icon' twin.
    #[test]
    fn the_default_style_fills_every_property_but_the_overriding_ones() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
            for button in [
                FilledButton::new(Some(pressed()), Some(text("button"))),
                FilledButton::icon(Some(pressed()), text("button"))
                    .icon(SizedBox::new())
                    .build(),
            ] {
                let style = captured_default_style(&mut tester, theme.clone(), button);
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
        });
    }

    // filled_button_test.dart 'FilledButton.icon produces the correct widgets if icon is null'
    // and 'FilledButton.tonalIcon produces the correct widgets if icon is null'.
    #[test]
    fn an_icon_button_without_an_icon_shows_only_its_label() {
        with_main_thread_stack(|| {
            for tonal in [false, true] {
                let mut tester = tester();
                let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
                let label_key = GlobalKey::new();
                let build = |icon: Option<WidgetRef>| {
                    let label = SizedBox::new()
                        .width(70.0)
                        .height(14.0)
                        .key(label_key.clone().into());
                    let button = if tonal {
                        FilledButton::tonal_icon(Some(pressed()), label)
                    } else {
                        FilledButton::icon(Some(pressed()), label)
                    };
                    match icon {
                        Some(icon) => button.icon(icon),
                        None => button,
                    }
                };
                tester.pump_widget(material_app(
                    theme.clone(),
                    Center::new().child(build(Some(icon()))).into_widget(),
                ));
                assert_eq!(tester.find_by_type::<Icon>().len(), 1);
                assert!(
                    label_key
                        .current_context(&tester.cell.borrow_mut())
                        .is_some()
                );

                tester.pump_widget(material_app(
                    theme,
                    // No icon specified.
                    Center::new().child(build(None)).into_widget(),
                ));
                assert!(tester.find_by_type::<Icon>().is_empty());
                assert!(
                    label_key
                        .current_context(&tester.cell.borrow_mut())
                        .is_some()
                );
            }
        });
    }

    // filled_button_test.dart 'FilledButton default overlayColor and elevation resolve pressed
    // state' and 'FilledButton.tonal default overlayColor and elevation resolve pressed state'.
    #[test]
    fn the_default_overlay_color_and_elevation_follow_hover_press_and_focus() {
        with_main_thread_stack(|| {
            for tonal in [false, true] {
                let mut tester = tester();
                let focus_node = focus_node(&tester);
                let theme = ThemeData::new(&tester.cell.borrow());
                let overlay = if tonal {
                    theme.color_scheme.on_secondary_container()
                } else {
                    theme.color_scheme.on_primary
                };
                let button = if tonal {
                    FilledButton::tonal(Some(pressed()), Some(text("FilledButton")))
                } else {
                    FilledButton::new(Some(pressed()), Some(text("FilledButton")))
                };
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(button.focus_node(focus_node.clone()))
                        .into_widget(),
                ));

                // Hovered.
                let center = button_center(&tester);
                let mut gesture = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();
                assert_eq!(physical_elevation(&tester), 1.0);
                assert_eq!(
                    ink_rect_colors(&tester),
                    vec![overlay.with_opacity(0.08).value()]
                );

                // Highlighted (pressed).
                tester.send(&gesture, waft_embedder::PointerChange::Down);
                tester.pump_and_settle();
                assert_eq!(physical_elevation(&tester), 0.0);
                assert_eq!(
                    ink_rect_colors(&tester)[1],
                    overlay.with_opacity(0.1).value()
                );
                // Remove pressed and hovered states
                tester.up(&gesture);
                tester.pump_and_settle();
                tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
                tester.pump_and_settle();

                // Focused.
                focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump_and_settle();
                assert_eq!(physical_elevation(&tester), 0.0);
                assert_eq!(
                    ink_rect_colors(&tester),
                    vec![overlay.with_opacity(0.1).value()]
                );
            }
        });
    }

    // filled_button_test.dart 'FilledButton onPressed and onLongPress callbacks are correctly
    // called when non-null'.
    #[test]
    fn either_callback_enables_the_button_and_reaches_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let was_pressed = Rc::new(Cell::new(0));
            let count = || {
                let was_pressed = Rc::clone(&was_pressed);
                Listener::new(move |_app| was_pressed.set(was_pressed.get() + 1))
            };
            let build_frame = |on_pressed: Option<Listener>, on_long_press: Option<Listener>| {
                let mut button = FilledButton::new(on_pressed, Some(text("button")));
                if let Some(on_long_press) = on_long_press {
                    button = button.on_long_press(on_long_press);
                }
                Directionality::new(TextDirection::Ltr, button).into_widget()
            };
            let enabled = |tester: &WidgetTester| {
                let button = tester.find_by_type::<FilledButton>()[0];
                tester.widget::<FilledButton, _>(button, |button| button.enabled())
            };

            // onPressed not null, onLongPress null.
            tester.pump_widget(build_frame(Some(count()), None));
            assert!(enabled(&tester));
            tester.tap(button_center(&tester), 1);
            assert_eq!(was_pressed.get(), 1);

            // onPressed null, onLongPress not null.
            tester.pump_widget(build_frame(None, Some(count())));
            assert!(enabled(&tester));
            tester.long_press(button_center(&tester), 2);
            assert_eq!(was_pressed.get(), 2);

            // onPressed null, onLongPress null.
            tester.pump_widget(build_frame(None, None));
            assert!(!enabled(&tester));
        });
    }

    // filled_button_test.dart 'Default FilledButton padding for textScaleFactor,
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
                            .filled_button_theme(
                                FilledButtonThemeData::new().style(
                                    FilledButton::style_from()
                                        .minimum_size(Size::new(64.0, 36.0))
                                        .build(),
                                ),
                            )
                            .build(&tester.cell.borrow());
                        let label = ahem_text("button").key(label_key.clone().into());
                        let button = if with_icon {
                            FilledButton::icon(Some(pressed()), label)
                                .key(button_key.clone().into())
                                .icon(
                                    Icon::new(Some(Icons::add()))
                                        .size(18.0)
                                        .key(icon_key.clone().into()),
                                )
                                .build()
                        } else {
                            FilledButton::new(Some(pressed()), Some(label))
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

    // filled_button_test.dart 'Override FilledButton default padding', 'Override theme
    // fontSize changes padding', 'M3 FilledButton has correct padding' and 'M3
    // FilledButton.icon has correct padding'.
    #[test]
    fn the_material_3_padding_follows_the_style_the_font_size_and_the_icon() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                MediaQuery::with_clamped_text_scaling(
                    None,
                    2.0,
                    2.0,
                    Center::new().child(
                        FilledButton::new(Some(pressed()), Some(text("FilledButton"))).style(
                            FilledButton::style_from()
                                .padding(EdgeInsetsGeometry::all(22.0))
                                .build(),
                        ),
                    ),
                ),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::all(22.0));

            let mut tester = self::tester();
            let mut text_theme = crate::text_theme::TextTheme::new();
            text_theme.label_large = Some(TextStyle::new().font_size(28.0));
            let large = ThemeData::from(ColorScheme::light())
                .text_theme(text_theme)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                large,
                Center::new()
                    .child(FilledButton::new(Some(pressed()), Some(text("text"))))
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::symmetric(0.0, 12.0));

            let mut tester = self::tester();
            tester.pump_widget(material_app(
                theme.clone(),
                Center::new()
                    .child(FilledButton::new(
                        Some(pressed()),
                        Some(text("FilledButton")),
                    ))
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::symmetric(0.0, 24.0));

            let mut tester = self::tester();
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(FilledButton::icon(Some(pressed()), text("FilledButton")).icon(icon()))
                    .into_widget(),
            ));
            assert!(padding(&tester) == EdgeInsetsGeometry::from_steb(16.0, 0.0, 24.0, 0.0));
        });
    }

    // filled_button_test.dart 'FilledButton.icon icon,label layout'.
    #[test]
    fn an_icon_button_lays_out_icon_gap_and_label_centred() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let icon_key = GlobalKey::new();
            let label_key = GlobalKey::new();
            let style = FilledButton::style_from()
                .padding(EdgeInsetsGeometry::ZERO)
                .visual_density(VisualDensity::STANDARD) // dx=0, dy=0
                .build();
            let theme = ThemeData::new(&tester.cell.borrow());
            let sized = |key: &GlobalKey| {
                SizedBox::new()
                    .width(50.0)
                    .height(100.0)
                    .key(key.clone().into())
                    .into_widget()
            };
            tester.pump_widget(material_app(
                theme,
                // Dart's `Scaffold(body:)` lays its body out loosely at the top left.
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(200.0).child(
                            FilledButton::icon(Some(pressed()), sized(&label_key))
                                .icon(sized(&icon_key))
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

    // filled_button_test.dart 'Default FilledButton icon alignment', 'FilledButton icon
    // alignment can be customized', 'FilledButton icon alignment respects
    // ButtonStyle.iconAlignment', 'FilledButton tonal button icon alignment respects
    // ButtonStyle.iconAlignment', 'Tonal icon default iconAlignment' and 'Tonal icon
    // iconAlignment can be customized'.
    #[test]
    fn the_icon_sits_at_the_start_or_end_by_its_alignment_and_the_text_direction() {
        with_main_thread_stack(|| {
            for tonal in [false, true] {
                let build = |tester: &mut WidgetTester,
                             text_direction: TextDirection,
                             icon_alignment: Option<IconAlignment>,
                             style_alignment: Option<IconAlignment>| {
                    let mut button = if tonal {
                        FilledButton::tonal_icon(Some(pressed()), text("button")).icon(icon())
                    } else {
                        FilledButton::icon(Some(pressed()), text("button")).icon(icon())
                    };
                    if let Some(icon_alignment) = icon_alignment {
                        button = button.icon_alignment(icon_alignment);
                    }
                    if let Some(style_alignment) = style_alignment {
                        button = button.style(ButtonStyle::new().icon_alignment(style_alignment));
                    }
                    let theme = ThemeData::new(&tester.cell.borrow());
                    tester.pump_widget(material_app(
                        theme,
                        Directionality::new(text_direction, Center::new().child(button))
                            .into_widget(),
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
            }
        });
    }

    // filled_button_test.dart 'FilledButton.styleFrom can be used to set foreground and
    // background colors'.
    #[test]
    fn style_from_turns_each_argument_into_its_property() {
        with_main_thread_stack(|| {
            let red = Color::new(0xffff0000);
            let blue = Color::new(0xff0000ff);
            let none = WidgetStates::new();
            let disabled = states(&[WidgetState::Disabled]);
            let hovered = states(&[WidgetState::Hovered]);
            let style = FilledButton::style_from()
                .foreground_color(red)
                .background_color(blue)
                .icon_color(blue)
                .icon_size(32.0)
                .elevation(4.0)
                .build();
            // The elevation is the same in every state, unlike ElevatedButton's.
            let elevation = style.elevation.as_ref().expect("an elevation");
            assert_eq!(elevation.resolve(&none), Some(4.0));
            assert_eq!(elevation.resolve(&disabled), Some(4.0));
            assert_eq!(elevation.resolve(&hovered), Some(4.0));
            assert_eq!(
                style
                    .overlay_color
                    .as_ref()
                    .and_then(|overlay| overlay.resolve(&hovered)),
                Some(red.with_opacity(0.08))
            );
            assert_eq!(style.size_variant, None);
            assert_eq!(style.shape_variant, None);

            let mut tester = tester();
            let icon_seen = Rc::new(Cell::new((None, None)));
            let probe = Builder::new({
                let icon_seen = Rc::clone(&icon_seen);
                move |app, context| {
                    let icon_theme = IconTheme::of(app, context);
                    icon_seen.set((icon_theme.color.map(|color| color.color()), icon_theme.size));
                    SizedBox::new().width(18.0).height(18.0).into_widget()
                }
            });
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        FilledButton::icon(Some(pressed()), text("button"))
                            .icon(probe)
                            .style(style),
                    )
                    .into_widget(),
            ));
            material(&tester, |material| {
                assert_eq!(material.color, Some(blue));
                assert_eq!(text_color(material), Some(red));
            });
            assert_eq!(icon_seen.get(), (Some(blue), Some(32.0)));
        });
    }

    // filled_button_test.dart 'FilledButton.icon does not lose focus when icon is nullified'
    // and 'FilledButton.tonalIcon does not lose focus when icon is nullified'.
    #[test]
    fn an_icon_button_keeps_its_focus_when_its_icon_goes() {
        with_main_thread_stack(|| {
            for tonal in [false, true] {
                let mut tester = tester();
                let focus_node = focus_node(&tester);
                let build = |tester: &mut WidgetTester, icon: Option<WidgetRef>| {
                    let button = if tonal {
                        FilledButton::tonal_icon(Some(pressed()), text("button"))
                    } else {
                        FilledButton::icon(Some(pressed()), text("button"))
                    };
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
            }
        });
    }

    // Dart's `super.clipBehavior = Clip.none` on every FilledButton constructor: with a layer
    // builder, which makes a button without a clip behavior clip, the button still does not.
    #[test]
    fn every_filled_constructor_defaults_to_no_clip() {
        let builder =
            crate::button_style::ButtonLayerBuilder::new(|_app, _context, _states, child| {
                child.expect("a child")
            });
        for button in [
            FilledButton::new(Some(pressed()), Some(text("a"))),
            FilledButton::icon(Some(pressed()), text("a"))
                .icon(icon())
                .build(),
            FilledButton::tonal(Some(pressed()), Some(text("a"))),
            FilledButton::tonal_icon(Some(pressed()), text("a"))
                .icon(icon())
                .build(),
        ] {
            let button = button.style(ButtonStyle::new().background_builder(builder.clone()));
            assert_eq!(button.button_style_button.clip_behavior, Some(Clip::None));
        }
    }

    /// The colour of the glyph of the `Icon` showing `icon`.
    fn glyph_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
        tester
            .glyph_style(tester.find_by_icon(icon))
            .color
            .map(|color| color.color())
    }

    // filled_button_test.dart 'FilledButton.icon responds to applied padding'.
    #[test]
    fn an_icon_button_puts_its_padding_around_the_icon_and_label() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let label_key = GlobalKey::new();
            tester.pump_widget(
                // When textDirection is set to TextDirection.ltr, the label appears on the
                // right side of the icon. This is important in determining whether the
                // horizontal padding is applied correctly later on
                Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(
                        FilledButton::icon(
                            Some(pressed()),
                            // Ahem's 'Hello' at the label style's 14 pixels and 1.43 line height,
                            // whose line Flutter lays out 20 pixels high.
                            SizedBox::new()
                                .width(70.0)
                                .height(20.0)
                                .key(label_key.clone().into()),
                        )
                        .style(
                            ButtonStyle::new().padding(Rc::new(MaterialStatePropertyAll::new(
                                Some(EdgeInsetsGeometry::from_ltrb(16.0, 5.0, 10.0, 12.0)),
                            ))),
                        )
                        .icon(icon()),
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

    // filled_button_test.dart 'By default, FilledButton shape outline is defined by
    // shape.side'.
    #[test]
    fn by_default_the_shape_side_is_the_outline() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let border_color = Color::new(0xff4caf50);
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        FilledButton::new(Some(pressed()), Some(text("button"))).style(
                            FilledButton::style_from()
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

            let button = tester.render_box_of(tester.find_by_type::<FilledButton>()[0]);
            assert!(tester.paints_drrect(
                button.as_object(),
                border_color,
                // Outer and inner rect that give the outline a width of 10.
                RRect::from_ltrbr(0.0, 0.0, 116.0, 36.0, Radius::circular(16.0)),
                RRect::from_ltrbr(10.0, 10.0, 106.0, 26.0, Radius::circular(16.0 - 10.0)),
            ));
        });
    }

    // filled_button_test.dart 'FilledButton changes mouse cursor when hovered'.
    #[test]
    fn style_from_cursors_show_while_hovered_and_the_default_is_basic() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = |on_pressed: Option<Listener>, custom: bool| {
                let mut button = FilledButton::new(on_pressed, Some(text("button")));
                if custom {
                    button = button.style(
                        FilledButton::style_from()
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

    // filled_button_test.dart 'FilledButton.icon statesController'; the plain button's case is
    // button_style_button.rs's.
    #[test]
    fn an_icon_buttons_states_controller_follows_hover_press_and_disable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let count = Rc::new(Cell::new(0));
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
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
                            FilledButton::icon(
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

    // filled_button_test.dart 'FilledButton.styleFrom can customize the button icon'.
    #[test]
    fn style_from_can_customize_the_button_icon() {
        with_main_thread_stack(|| {
            let icon_color = Color::new(0xFFF000FF);
            let icon_size = 32.0;
            let disabled_icon_color = Color::new(0xFFFFF000);
            let build_button = |tester: &WidgetTester, enabled: bool| {
                let theme = ThemeData::new(&tester.cell.borrow());
                material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(
                                FilledButton::icon(enabled.then(pressed), text("Button"))
                                    .style(
                                        FilledButton::style_from()
                                            .icon_color(icon_color)
                                            .icon_size(icon_size)
                                            .icon_alignment(IconAlignment::End)
                                            .disabled_icon_color(disabled_icon_color)
                                            .build(),
                                    )
                                    .icon(icon()),
                            ),
                        )
                        .into_widget(),
                )
            };

            // Test enabled button.
            let mut tester = tester();
            let enabled = build_button(&tester, true);
            tester.pump_widget(enabled);
            assert_eq!(icon_rect(&tester).size(), Size::new(icon_size, icon_size));
            assert_eq!(glyph_color(&tester, &Icons::add()), Some(icon_color));

            // Test disabled button.
            let disabled = build_button(&tester, false);
            tester.pump_widget(disabled);
            tester.pump_and_settle();
            assert_eq!(
                glyph_color(&tester, &Icons::add()),
                Some(disabled_icon_color)
            );

            let button_top_right = material_rect(&tester).top_right();
            let icon_top_right = icon_rect(&tester).top_right();
            assert_eq!(button_top_right.dx(), icon_top_right.dx() + 24.0);
        });
    }

    // filled_button_test.dart 'FilledButton icon uses provided foregroundColor over default icon
    // color'.
    #[test]
    fn an_icon_takes_the_style_foreground_color_over_the_default_icon_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let foreground_color = Color::new(0xFFFF1234);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(Column::new().children(vec![
                            FilledButton::icon(Some(pressed()), text("Button"))
                                .style(
                                    FilledButton::style_from()
                                        .foreground_color(foreground_color)
                                        .build(),
                                )
                                .icon(Icon::new(Some(Icons::add())))
                                .into_widget(),
                            FilledButton::tonal_icon(Some(pressed()), text("Button"))
                                .style(
                                    FilledButton::style_from()
                                        .foreground_color(foreground_color)
                                        .build(),
                                )
                                .icon(Icon::new(Some(Icons::mail())))
                                .into_widget(),
                        ])))
                    .into_widget(),
            ));
            assert_eq!(glyph_color(&tester, &Icons::add()), Some(foreground_color));
            assert_eq!(glyph_color(&tester, &Icons::mail()), Some(foreground_color));
        });
    }
}
