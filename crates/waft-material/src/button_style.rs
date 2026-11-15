//! Flutter counterpart: material_ui's `button_style.dart`.
//!
//! A state-dependent field holds a [`WidgetStatePropertyRef`] of `Option<T>`: Dart's
//! `WidgetStateProperty<T?>`, whose `null` lets the next style decide.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Color, Size, lerp_double};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, BorderSide, EdgeInsetsGeometry, OutlinedBorder, TextStyle};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, WidgetRef, WidgetStateBorderSide, WidgetStateProperty, WidgetStatePropertyRef,
    WidgetStates,
};

use crate::button_style_button::IconAlignment;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Defines size variants for Material 3 Expressive button components.
///
/// Each size variant represents a named visual scale. For example, a Material
/// 3 Expressive `IconButton` uses this value to choose its container size, icon
/// size, and padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonSizeVariant {
    /// The smallest button size.
    XSmall,

    /// A small button size.
    Small,

    /// A medium button size.
    Medium,

    /// A large button size.
    Large,

    /// The largest button size.
    XLarge,
}

/// Defines the width variants for Material 3 Expressive `IconButton`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconButtonWidthVariant {
    /// A narrower icon button.
    Narrow,

    /// The standard icon button width.
    Standard,

    /// A wider icon button.
    Wide,
}

/// Defines shape variants for Material 3 Expressive button components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonShapeVariant {
    /// A rounded button shape.
    ///
    /// For example, a Material 3 Expressive `IconButton` with this variant uses
    /// rounder default container shapes.
    Round,

    /// A squared button shape.
    ///
    /// For example, a Material 3 Expressive `IconButton` with this variant uses
    /// more squared default container shapes.
    Square,
}

/// The type for [`ButtonStyle::background_builder`] and [`ButtonStyle::foreground_builder`].
///
/// The `states` parameter is the button's current pressed/hovered/etc state. The `child` is
/// typically a descendant of the returned widget.
///
/// [`ButtonStyle`]'s `==` compares two builders as Dart compares two function values:
/// [`new`](Self::new) equals only its clones; [`function`](Self::function) equals any other
/// built from the same function, as two tear-offs of one static method are identical (see
/// the porting skill's callback-identity pattern).
#[derive(Clone)]
pub struct ButtonLayerBuilder {
    builder: Rc<ButtonLayerBuilderFn>,
    identity: ButtonLayerBuilderIdentity,
}

/// The function a [`ButtonLayerBuilder`] calls.
type ButtonLayerBuilderFn =
    dyn Fn(&mut App, BuildContext, &WidgetStates, Option<WidgetRef>) -> WidgetRef;

/// Equality key: closures compare by allocation, functions by the `TypeId` of the function
/// item.
#[derive(Clone, Copy, PartialEq)]
enum ButtonLayerBuilderIdentity {
    Closure,
    Function(TypeId),
}

impl ButtonLayerBuilder {
    /// Wraps a closure; it equals only its clones.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, &WidgetStates, Option<WidgetRef>) -> WidgetRef
        + 'static,
    ) -> ButtonLayerBuilder {
        ButtonLayerBuilder {
            builder: Rc::new(builder),
            identity: ButtonLayerBuilderIdentity::Closure,
        }
    }

    /// A function, comparable like a Dart tear-off of a static method.
    ///
    /// Pass the function by name. A fn pointer or capturing closure is rejected: it has no
    /// canonical identity.
    pub fn function<F>(builder: F) -> ButtonLayerBuilder
    where
        F: Fn(&mut App, BuildContext, &WidgetStates, Option<WidgetRef>) -> WidgetRef + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure has no canonical \
                 identity"
            )
        };
        ButtonLayerBuilder {
            builder: Rc::new(builder),
            identity: ButtonLayerBuilderIdentity::Function(TypeId::of::<F>()),
        }
    }

    /// Calls the builder.
    pub fn call(
        &self,
        app: &mut App,
        context: BuildContext,
        states: &WidgetStates,
        child: Option<WidgetRef>,
    ) -> WidgetRef {
        (self.builder)(app, context, states, child)
    }
}

impl PartialEq for ButtonLayerBuilder {
    fn eq(&self, other: &ButtonLayerBuilder) -> bool {
        match (self.identity, other.identity) {
            (ButtonLayerBuilderIdentity::Closure, ButtonLayerBuilderIdentity::Closure) => {
                Rc::ptr_eq(&self.builder, &other.builder)
            }
            (mine, theirs) => mine == theirs,
        }
    }
}

impl Debug for ButtonLayerBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ButtonLayerBuilder")
    }
}

/// The visual properties that most buttons have in common.
///
/// Buttons and their themes have a ButtonStyle property which defines the visual
/// properties whose default values are to be overridden. The default values are
/// defined by the individual button widgets and are typically based on overall
/// theme's `ThemeData::color_scheme` and `ThemeData::text_theme`.
///
/// All of the ButtonStyle properties are null by default.
///
/// Many of the ButtonStyle properties are [`WidgetStateProperty`] objects which
/// resolve to different values depending on the button's state. For example
/// the [`Color`] properties are defined with `WidgetStateProperty<Option<Color>>` and
/// can resolve to different colors depending on if the button is pressed,
/// hovered, focused, disabled, etc.
///
/// These properties can override the default value for just one state or all of
/// them. For example to create an `ElevatedButton` whose background color is the
/// color scheme’s primary color with 50% opacity, but only when the button is
/// pressed, one could write:
///
/// ```text
/// ElevatedButton::new(on_pressed, Text::new("Fly me to the moon"))
///     .style(ButtonStyle::new().background_color(
///         <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
///             if states.contains(&WidgetState::Pressed) {
///                 return Some(primary.with_opacity(0.5));
///             }
///             None // Use the component's default.
///         }),
///     ))
/// ```
///
/// In this case the background color for all other button states would fallback
/// to the ElevatedButton’s default values. To unconditionally set the button's
/// [`background_color`](Self::background_color) for all states one could write:
///
/// ```text
/// ElevatedButton::new(on_pressed, Text::new("Let me play among the stars"))
///     .style(ButtonStyle::new().background_color(Rc::new(WidgetStatePropertyAll::new(
///         Some(Colors::GREEN.primary()),
///     ))))
/// ```
///
/// Configuring a ButtonStyle directly makes it possible to very
/// precisely control the button’s visual attributes for all states.
/// This level of control is typically required when a custom
/// “branded” look and feel is desirable. However, in many cases it’s
/// useful to make relatively sweeping changes based on a few initial
/// parameters with simple values. The button `style_from()` methods
/// enable such sweeping changes. See for example:
/// `ElevatedButton::style_from`, `FilledButton::style_from`,
/// `OutlinedButton::style_from`, `TextButton::style_from`.
///
/// For example, to override the default text and icon colors for a
/// `TextButton`, as well as its overlay color, with all of the
/// standard opacity adjustments for the pressed, focused, and
/// hovered states, one could write:
///
/// ```text
/// TextButton::new(on_pressed, Text::new("Let me see what spring is like"))
///     .style(TextButton::style_from().foreground_color(Colors::GREEN.primary()))
/// ```
///
/// To configure all of the application's text buttons in the same
/// way, specify the overall theme's `text_button_theme`.
///
/// ## Material 3 button types
///
/// Material Design 3 specifies five types of common buttons. Flutter provides
/// support for these using the following button classes:
///
/// | Type         | Flutter implementation  |
/// | :----------- | :---------------------- |
/// | Elevated     | `ElevatedButton`        |
/// | Filled       | `FilledButton`          |
/// | Filled Tonal | `FilledButton::tonal`   |
/// | Outlined     | `OutlinedButton`        |
/// | Text         | `TextButton`            |
///
/// See also:
///
///  * `ElevatedButtonTheme`, the theme for `ElevatedButton`s.
///  * `FilledButtonTheme`, the theme for `FilledButton`s.
///  * `OutlinedButtonTheme`, the theme for `OutlinedButton`s.
///  * `TextButtonTheme`, the theme for `TextButton`s.
#[derive(Clone, Default)]
pub struct ButtonStyle {
    /// The style for a button's `Text` widget descendants.
    ///
    /// The color of the [`text_style`](Self::text_style) is typically not used directly, the
    /// [`foreground_color`](Self::foreground_color) is used instead.
    pub text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The button's background fill color.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color for the button's `Text` widget descendants.
    ///
    /// This color is typically used instead of the color of the
    /// [`text_style`](Self::text_style). All of the components that compute defaults from
    /// [`ButtonStyle`] values compute a default [`foreground_color`](Self::foreground_color)
    /// and use that instead of the [`text_style`](Self::text_style)'s color.
    pub foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The highlight color that's typically used to indicate that
    /// the button is focused, hovered, or pressed.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The shadow color of the button's `Material`.
    ///
    /// The material's elevation shadow can be difficult to see for
    /// dark themes, so by default the button classes add a
    /// semi-transparent overlay to indicate elevation. See
    /// `ThemeData::apply_elevation_overlay_color`.
    pub shadow_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The surface tint color of the button's `Material`.
    ///
    /// See `Material::surface_tint_color` for more details.
    pub surface_tint_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The elevation of the button's `Material`.
    pub elevation: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// The padding between the button's boundary and its child.
    ///
    /// The vertical aspect of the default or user-specified padding is adjusted
    /// automatically based on [`visual_density`](Self::visual_density).
    ///
    /// When the visual density is [`VisualDensity::COMPACT`], the top and bottom insets
    /// are reduced by 8 pixels or set to 0 pixels if the result of the reduced padding
    /// is negative. For example: the visual density defaults to [`VisualDensity::COMPACT`]
    /// on desktop and web, so if the provided padding is 16 pixels on the top and bottom,
    /// it will be reduced to 8 pixels on the top and bottom. If the provided padding
    /// is 4 pixels, the result will be no padding on the top and bottom.
    ///
    /// When the visual density is [`VisualDensity::COMFORTABLE`], the top and bottom insets
    /// are reduced by 4 pixels or set to 0 pixels if the result of the reduced padding
    /// is negative.
    ///
    /// When the visual density is [`VisualDensity::STANDARD`] the top and bottom insets
    /// are not changed. The visual density defaults to [`VisualDensity::STANDARD`] on mobile.
    ///
    /// See `ThemeData::visual_density` for more details.
    pub padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,

    /// The minimum size of the button itself before applying
    /// [`visual_density`](Self::visual_density).
    ///
    /// The size of the rectangle the button lies within may be larger
    /// per [`tap_target_size`](Self::tap_target_size).
    ///
    /// This value must be less than or equal to [`maximum_size`](Self::maximum_size).
    ///
    /// The minimum size is adjusted automatically based on
    /// [`visual_density`](Self::visual_density).
    ///
    /// When visual density is [`VisualDensity::COMPACT`], the minimum size is
    /// reduced by 8 pixels on both dimensions.
    ///
    /// When visual density is [`VisualDensity::COMFORTABLE`], the minimum size is
    /// [`minimum_size`](Self::minimum_size) reduced by 4 pixels on both dimensions.
    ///
    /// When visual density is [`VisualDensity::STANDARD`], the minimum size is
    /// [`minimum_size`](Self::minimum_size).
    pub minimum_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The button's size.
    ///
    /// This size is still constrained by the style's [`minimum_size`](Self::minimum_size)
    /// and [`maximum_size`](Self::maximum_size). Fixed size dimensions whose value is
    /// [`f64::INFINITY`] are ignored.
    ///
    /// The size of the rectangle the button lies within may be larger
    /// per [`tap_target_size`](Self::tap_target_size).
    ///
    /// To specify buttons with a fixed width and the default height use
    /// `Size::from_width(320.0)`. Similarly, to specify a fixed
    /// height and the default width use `Size::from_height(100.0)`.
    pub fixed_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The maximum size of the button itself.
    ///
    /// A `Size::INFINITE` or null value for this property means that
    /// the button's maximum size is not constrained.
    ///
    /// This value must be greater than or equal to [`minimum_size`](Self::minimum_size).
    pub maximum_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The icon's color inside of the button.
    pub icon_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The icon's size inside of the button.
    pub icon_size: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// The alignment of the button's icon.
    ///
    /// This property is supported for the following button types:
    ///
    ///  * `ElevatedButton::icon`.
    ///  * `FilledButton::icon`.
    ///  * `FilledButton::tonal_icon`.
    ///  * `OutlinedButton::icon`.
    ///  * `TextButton::icon`.
    ///
    /// See also:
    ///
    ///  * [`IconAlignment`], for more information about the different icon
    ///    alignments.
    pub icon_alignment: Option<IconAlignment>,

    /// The color and weight of the button's outline.
    ///
    /// This value is combined with [`shape`](Self::shape) to create a shape decorated
    /// with an outline.
    pub side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,

    /// The shape of the button's underlying `Material`.
    ///
    /// This shape is combined with [`side`](Self::side) to create a shape decorated
    /// with an outline.
    pub shape: Option<WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>>>,

    /// The cursor for a mouse pointer when it enters or is hovering over
    /// this button's `InkWell`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Defines how compact the button's layout will be.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the
    ///    [`visual_density`](Self::visual_density) for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,

    /// Configures the minimum size of the area within which the button may be pressed.
    ///
    /// If the [`tap_target_size`](Self::tap_target_size) is larger than
    /// [`minimum_size`](Self::minimum_size), the button will include a transparent margin
    /// that responds to taps.
    ///
    /// Always defaults to `ThemeData::material_tap_target_size`.
    pub tap_target_size: Option<MaterialTapTargetSize>,

    /// Defines the duration of animated changes for [`shape`](Self::shape) and
    /// [`elevation`](Self::elevation).
    ///
    /// Typically the component default value is `K_THEME_CHANGE_DURATION`.
    pub animation_duration: Option<Duration>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// Typically the component default value is true.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// The alignment of the button's child.
    ///
    /// Typically buttons are sized to be just big enough to contain the child and its
    /// padding. If the button's size is constrained to a fixed size, for example by
    /// enclosing it with a `SizedBox`, this property defines how the child is aligned
    /// within the available space.
    ///
    /// Always defaults to `Alignment::CENTER`.
    pub alignment: Option<AlignmentGeometry>,

    /// Creates the `InkWell` splash factory, which defines the appearance of
    /// "ink" splashes that occur in response to taps.
    ///
    /// Use `NoSplash::splash_factory()` to defeat ink splash rendering.
    pub splash_factory: Option<InteractiveInkFeatureFactoryRef>,

    /// Creates a widget that becomes the child of the button's `Material`
    /// and whose child is the rest of the button, including the button's
    /// `child` parameter.
    ///
    /// The widget created by [`background_builder`](Self::background_builder) is
    /// constrained to be the same size as the overall button and will appear behind the
    /// button's child. The widget created by [`foreground_builder`](Self::foreground_builder)
    /// is constrained to be the same size as the button's child, i.e. it's
    /// inset by [`ButtonStyle::padding`] and aligned by the button's
    /// [`ButtonStyle::alignment`].
    ///
    /// By default the returned widget is clipped to the Material's [`ButtonStyle::shape`].
    ///
    /// See also:
    ///
    ///  * [`foreground_builder`](Self::foreground_builder), to create a widget that's as big
    ///    as the button's child and is layered behind the child.
    ///  * `ButtonStyleButtonData::clip_behavior`, for more information about
    ///    configuring clipping.
    pub background_builder: Option<ButtonLayerBuilder>,

    /// Creates a Widget that contains the button's child parameter which is used
    /// instead of the button's child.
    ///
    /// The returned widget is clipped by the button's
    /// [`ButtonStyle::shape`], inset by the button's [`ButtonStyle::padding`]
    /// and aligned by the button's [`ButtonStyle::alignment`].
    ///
    /// See also:
    ///
    ///  * [`background_builder`](Self::background_builder), to create a widget that's as big
    ///    as the button and is layered behind the button's child.
    ///  * `ButtonStyleButtonData::clip_behavior`, for more information about
    ///    configuring clipping.
    pub foreground_builder: Option<ButtonLayerBuilder>,

    /// The Material 3 Expressive size variant for this button.
    ///
    /// A size variant selects a component-defined token set rather than a single
    /// dimension. For example, an `IconButton` uses this value to choose its
    /// default container size, padding, and icon size when it is using
    /// `StyleVariant::Material3Expressive`.
    ///
    /// Use [`minimum_size`](Self::minimum_size), [`fixed_size`](Self::fixed_size),
    /// [`maximum_size`](Self::maximum_size), [`padding`](Self::padding), or
    /// [`icon_size`](Self::icon_size) to directly override those individual style
    /// properties.
    pub size_variant: Option<ButtonSizeVariant>,

    /// The Material 3 Expressive width variant for an `IconButton`.
    ///
    /// A width variant selects the leading and trailing space tokens used to
    /// compute an icon button's default horizontal padding and minimum width when
    /// it is using `StyleVariant::Material3Expressive`. It does not change the
    /// button's height or icon size.
    ///
    /// This property is only used by icon buttons. Use [`padding`](Self::padding),
    /// [`minimum_size`](Self::minimum_size), [`fixed_size`](Self::fixed_size), or
    /// [`maximum_size`](Self::maximum_size) to directly override the resulting layout.
    pub icon_button_width: Option<IconButtonWidthVariant>,

    /// The Material 3 Expressive shape variant for this button.
    ///
    /// A shape variant selects between component-defined token families, such as
    /// round and square shapes. For `IconButton`, this affects the default
    /// container and selected-state shapes when it is using
    /// `StyleVariant::Material3Expressive`; state-specific tokens, such as the
    /// pressed shape, are still resolved by the component defaults.
    ///
    /// Use [`shape`](Self::shape) to provide a specific [`OutlinedBorder`] or stateful shape
    /// override instead of selecting one of the Material 3 Expressive shape
    /// variants.
    pub shape_variant: Option<ButtonShapeVariant>,
}

/// The fluent setters of Dart's named `ButtonStyle` arguments, which `copyWith` shares.
macro_rules! button_style_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `ButtonStyle(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> ButtonStyle {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl ButtonStyle {
    /// Create a [`ButtonStyle`].
    pub fn new() -> ButtonStyle {
        ButtonStyle::default()
    }

    button_style_setters!(
        text_style: WidgetStatePropertyRef<Option<TextStyle>> => "textStyle",
        background_color: WidgetStatePropertyRef<Option<Color>> => "backgroundColor",
        foreground_color: WidgetStatePropertyRef<Option<Color>> => "foregroundColor",
        overlay_color: WidgetStatePropertyRef<Option<Color>> => "overlayColor",
        shadow_color: WidgetStatePropertyRef<Option<Color>> => "shadowColor",
        surface_tint_color: WidgetStatePropertyRef<Option<Color>> => "surfaceTintColor",
        elevation: WidgetStatePropertyRef<Option<f64>> => "elevation",
        padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>> => "padding",
        minimum_size: WidgetStatePropertyRef<Option<Size>> => "minimumSize",
        fixed_size: WidgetStatePropertyRef<Option<Size>> => "fixedSize",
        maximum_size: WidgetStatePropertyRef<Option<Size>> => "maximumSize",
        icon_color: WidgetStatePropertyRef<Option<Color>> => "iconColor",
        icon_size: WidgetStatePropertyRef<Option<f64>> => "iconSize",
        icon_alignment: IconAlignment => "iconAlignment",
        side: WidgetStatePropertyRef<Option<BorderSide>> => "side",
        shape: WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>> => "shape",
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> => "mouseCursor",
        visual_density: VisualDensity => "visualDensity",
        tap_target_size: MaterialTapTargetSize => "tapTargetSize",
        animation_duration: Duration => "animationDuration",
        enable_feedback: bool => "enableFeedback",
        alignment: AlignmentGeometry => "alignment",
        splash_factory: InteractiveInkFeatureFactoryRef => "splashFactory",
        background_builder: ButtonLayerBuilder => "backgroundBuilder",
        foreground_builder: ButtonLayerBuilder => "foregroundBuilder",
        size_variant: ButtonSizeVariant => "sizeVariant",
        icon_button_width: IconButtonWidthVariant => "iconButtonWidth",
        shape_variant: ButtonShapeVariant => "shapeVariant",
    );

    /// Returns a copy of this ButtonStyle with the given fields replaced with
    /// the new values.
    ///
    /// Chain setters: `style.copy_with().elevation(e)`.
    pub fn copy_with(&self) -> ButtonStyle {
        self.clone()
    }

    /// Returns a copy of this ButtonStyle where the non-null fields in `style`
    /// have replaced the corresponding null fields in this ButtonStyle.
    ///
    /// In other words, `style` is used to fill in unspecified (null) fields
    /// this ButtonStyle.
    pub fn merge(&self, style: Option<&ButtonStyle>) -> ButtonStyle {
        let Some(style) = style else {
            return self.clone();
        };
        ButtonStyle {
            text_style: self.text_style.clone().or_else(|| style.text_style.clone()),
            background_color: self
                .background_color
                .clone()
                .or_else(|| style.background_color.clone()),
            foreground_color: self
                .foreground_color
                .clone()
                .or_else(|| style.foreground_color.clone()),
            overlay_color: self
                .overlay_color
                .clone()
                .or_else(|| style.overlay_color.clone()),
            shadow_color: self
                .shadow_color
                .clone()
                .or_else(|| style.shadow_color.clone()),
            surface_tint_color: self
                .surface_tint_color
                .clone()
                .or_else(|| style.surface_tint_color.clone()),
            elevation: self.elevation.clone().or_else(|| style.elevation.clone()),
            padding: self.padding.clone().or_else(|| style.padding.clone()),
            minimum_size: self
                .minimum_size
                .clone()
                .or_else(|| style.minimum_size.clone()),
            fixed_size: self.fixed_size.clone().or_else(|| style.fixed_size.clone()),
            maximum_size: self
                .maximum_size
                .clone()
                .or_else(|| style.maximum_size.clone()),
            icon_color: self.icon_color.clone().or_else(|| style.icon_color.clone()),
            icon_size: self.icon_size.clone().or_else(|| style.icon_size.clone()),
            icon_alignment: self.icon_alignment.or(style.icon_alignment),
            side: self.side.clone().or_else(|| style.side.clone()),
            shape: self.shape.clone().or_else(|| style.shape.clone()),
            mouse_cursor: self
                .mouse_cursor
                .clone()
                .or_else(|| style.mouse_cursor.clone()),
            visual_density: self.visual_density.or(style.visual_density),
            tap_target_size: self.tap_target_size.or(style.tap_target_size),
            animation_duration: self.animation_duration.or(style.animation_duration),
            enable_feedback: self.enable_feedback.or(style.enable_feedback),
            alignment: self.alignment.or(style.alignment),
            splash_factory: self
                .splash_factory
                .clone()
                .or_else(|| style.splash_factory.clone()),
            background_builder: self
                .background_builder
                .clone()
                .or_else(|| style.background_builder.clone()),
            foreground_builder: self
                .foreground_builder
                .clone()
                .or_else(|| style.foreground_builder.clone()),
            size_variant: self.size_variant.or(style.size_variant),
            icon_button_width: self.icon_button_width.or(style.icon_button_width),
            shape_variant: self.shape_variant.or(style.shape_variant),
        }
    }

    /// Linearly interpolate between two [`ButtonStyle`]s.
    pub fn lerp(a: Option<&ButtonStyle>, b: Option<&ButtonStyle>, t: f64) -> Option<ButtonStyle> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ButtonStyle {
            text_style: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.text_style.clone()),
                b.and_then(|b| b.text_style.clone()),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            ),
            background_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.background_color.clone()),
                b.and_then(|b| b.background_color.clone()),
                t,
                Color::lerp,
            ),
            foreground_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.foreground_color.clone()),
                b.and_then(|b| b.foreground_color.clone()),
                t,
                Color::lerp,
            ),
            overlay_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            shadow_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.shadow_color.clone()),
                b.and_then(|b| b.shadow_color.clone()),
                t,
                Color::lerp,
            ),
            surface_tint_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.surface_tint_color.clone()),
                b.and_then(|b| b.surface_tint_color.clone()),
                t,
                Color::lerp,
            ),
            elevation: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.elevation.clone()),
                b.and_then(|b| b.elevation.clone()),
                t,
                lerp_double,
            ),
            padding: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.padding.clone()),
                b.and_then(|b| b.padding.clone()),
                t,
                EdgeInsetsGeometry::lerp,
            ),
            minimum_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.minimum_size.clone()),
                b.and_then(|b| b.minimum_size.clone()),
                t,
                Size::lerp,
            ),
            fixed_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.fixed_size.clone()),
                b.and_then(|b| b.fixed_size.clone()),
                t,
                Size::lerp,
            ),
            maximum_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.maximum_size.clone()),
                b.and_then(|b| b.maximum_size.clone()),
                t,
                Size::lerp,
            ),
            icon_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.icon_color.clone()),
                b.and_then(|b| b.icon_color.clone()),
                t,
                Color::lerp,
            ),
            icon_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.icon_size.clone()),
                b.and_then(|b| b.icon_size.clone()),
                t,
                lerp_double,
            ),
            icon_alignment: if t < 0.5 {
                a.and_then(|a| a.icon_alignment)
            } else {
                b.and_then(|b| b.icon_alignment)
            },
            side: WidgetStateBorderSide::lerp(
                a.and_then(|a| a.side.clone()),
                b.and_then(|b| b.side.clone()),
                t,
            ),
            shape: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.shape.clone()),
                b.and_then(|b| b.shape.clone()),
                t,
                |a, b, t| <dyn OutlinedBorder>::lerp(a.as_deref(), b.as_deref(), t),
            ),
            mouse_cursor: if t < 0.5 {
                a.and_then(|a| a.mouse_cursor.clone())
            } else {
                b.and_then(|b| b.mouse_cursor.clone())
            },
            visual_density: if t < 0.5 {
                a.and_then(|a| a.visual_density)
            } else {
                b.and_then(|b| b.visual_density)
            },
            tap_target_size: if t < 0.5 {
                a.and_then(|a| a.tap_target_size)
            } else {
                b.and_then(|b| b.tap_target_size)
            },
            animation_duration: if t < 0.5 {
                a.and_then(|a| a.animation_duration)
            } else {
                b.and_then(|b| b.animation_duration)
            },
            enable_feedback: if t < 0.5 {
                a.and_then(|a| a.enable_feedback)
            } else {
                b.and_then(|b| b.enable_feedback)
            },
            alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.alignment),
                b.and_then(|b| b.alignment),
                t,
            ),
            splash_factory: if t < 0.5 {
                a.and_then(|a| a.splash_factory.clone())
            } else {
                b.and_then(|b| b.splash_factory.clone())
            },
            background_builder: if t < 0.5 {
                a.and_then(|a| a.background_builder.clone())
            } else {
                b.and_then(|b| b.background_builder.clone())
            },
            foreground_builder: if t < 0.5 {
                a.and_then(|a| a.foreground_builder.clone())
            } else {
                b.and_then(|b| b.foreground_builder.clone())
            },
            size_variant: if t < 0.5 {
                a.and_then(|a| a.size_variant)
            } else {
                b.and_then(|b| b.size_variant)
            },
            icon_button_width: if t < 0.5 {
                a.and_then(|a| a.icon_button_width)
            } else {
                b.and_then(|b| b.icon_button_width)
            },
            shape_variant: if t < 0.5 {
                a.and_then(|a| a.shape_variant)
            } else {
                b.and_then(|b| b.shape_variant)
            },
        })
    }
}

impl PartialEq for ButtonStyle {
    fn eq(&self, other: &ButtonStyle) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.text_style == self.text_style
            && other.background_color == self.background_color
            && other.foreground_color == self.foreground_color
            && other.overlay_color == self.overlay_color
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.elevation == self.elevation
            && other.padding == self.padding
            && other.minimum_size == self.minimum_size
            && other.fixed_size == self.fixed_size
            && other.maximum_size == self.maximum_size
            && other.icon_color == self.icon_color
            && other.icon_size == self.icon_size
            && other.icon_alignment == self.icon_alignment
            && other.side == self.side
            && other.shape == self.shape
            && other.mouse_cursor == self.mouse_cursor
            && other.visual_density == self.visual_density
            && other.tap_target_size == self.tap_target_size
            && other.animation_duration == self.animation_duration
            && other.enable_feedback == self.enable_feedback
            && other.alignment == self.alignment
            && other.splash_factory == self.splash_factory
            && other.background_builder == self.background_builder
            && other.foreground_builder == self.foreground_builder
            && other.size_variant == self.size_variant
            && other.icon_button_width == self.icon_button_width
            && other.shape_variant == self.shape_variant
    }
}

impl Debug for ButtonStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("ButtonStyle");
        macro_rules! property {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = &self.$field {
                        properties.field(stringify!($field), value);
                    }
                )*
            };
        }
        property!(
            text_style,
            background_color,
            foreground_color,
            overlay_color,
            shadow_color,
            surface_tint_color,
            elevation,
            padding,
            minimum_size,
            fixed_size,
            maximum_size,
            icon_color,
            icon_size,
            icon_alignment,
            side,
            shape,
            mouse_cursor,
            visual_density,
            tap_target_size,
            animation_duration,
            enable_feedback,
            alignment,
            background_builder,
            foreground_builder,
            size_variant,
            icon_button_width,
            shape_variant,
        );
        properties.finish()
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::{BorderStyle, StadiumBorder};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{WidgetState, WidgetStatePropertyAll};

    use super::*;

    fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    // button_style_test.dart 'ButtonStyle copyWith, merge, ==, hashCode basics'.
    #[test]
    fn an_empty_style_equals_its_copy_and_its_merge_with_an_empty_style() {
        assert_eq!(ButtonStyle::new(), ButtonStyle::new().copy_with());
        assert_eq!(
            ButtonStyle::new().merge(Some(&ButtonStyle::new())),
            ButtonStyle::new()
        );
    }

    // button_style_test.dart 'ButtonStyle lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_style_with_itself_is_that_style() {
        assert!(ButtonStyle::lerp(None, None, 0.0).is_none());
        let data = ButtonStyle::new().elevation(all(1.0));
        assert_eq!(ButtonStyle::lerp(Some(&data), Some(&data), 0.5), Some(data));
    }

    // button_style_test.dart 'ButtonStyle defaults'.
    #[test]
    fn every_property_of_a_new_style_is_null() {
        let style = ButtonStyle::new();
        assert!(style.text_style.is_none());
        assert!(style.background_color.is_none());
        assert!(style.foreground_color.is_none());
        assert!(style.overlay_color.is_none());
        assert!(style.shadow_color.is_none());
        assert!(style.surface_tint_color.is_none());
        assert!(style.elevation.is_none());
        assert!(style.padding.is_none());
        assert!(style.minimum_size.is_none());
        assert!(style.fixed_size.is_none());
        assert!(style.maximum_size.is_none());
        assert!(style.icon_color.is_none());
        assert!(style.icon_size.is_none());
        assert!(style.side.is_none());
        assert!(style.shape.is_none());
        assert!(style.mouse_cursor.is_none());
        assert!(style.visual_density.is_none());
        assert!(style.tap_target_size.is_none());
        assert!(style.animation_duration.is_none());
        assert!(style.enable_feedback.is_none());
        assert!(style.alignment.is_none());
        assert!(style.splash_factory.is_none());
        assert!(style.background_builder.is_none());
        assert!(style.foreground_builder.is_none());
        assert!(style.size_variant.is_none());
        assert!(style.icon_button_width.is_none());
        assert!(style.shape_variant.is_none());
    }

    // button_style_test.dart 'ButtonStyle copyWith, merge'.
    #[test]
    fn a_style_equals_an_empty_copy_given_every_field_and_an_empty_style_merged_with_it() {
        let text_style = all(TextStyle::new().font_size(10.0));
        let background_color = all(Color::new(0xfffffff1));
        let foreground_color = all(Color::new(0xfffffff2));
        let overlay_color = all(Color::new(0xfffffff3));
        let shadow_color = all(Color::new(0xfffffff4));
        let surface_tint_color = all(Color::new(0xfffffff5));
        let elevation = all(1.0);
        let padding = all(EdgeInsetsGeometry::all(1.0));
        let minimum_size = all(Size::new(1.0, 2.0));
        let fixed_size = all(Size::new(3.0, 4.0));
        let maximum_size = all(Size::new(5.0, 6.0));
        let icon_color = all(Color::new(0xfffffff6));
        let icon_size = all(48.0);
        let side = all(BorderSide::default());
        let shape: WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>> =
            Rc::new(WidgetStatePropertyAll::new(Some(
                Box::new(StadiumBorder::default()) as Box<dyn OutlinedBorder>,
            )));
        let mouse_cursor = all::<MouseCursorRef>(SystemMouseCursors::FORBIDDEN.into());
        let visual_density = VisualDensity::COMPACT;
        let tap_target_size = MaterialTapTargetSize::ShrinkWrap;
        let animation_duration = Duration::from_secs(1);
        let enable_feedback = true;
        let size_variant = ButtonSizeVariant::Small;
        let icon_button_width = IconButtonWidthVariant::Standard;
        let shape_variant = ButtonShapeVariant::Round;

        let style = ButtonStyle::new()
            .text_style(Rc::clone(&text_style))
            .background_color(Rc::clone(&background_color))
            .foreground_color(Rc::clone(&foreground_color))
            .overlay_color(Rc::clone(&overlay_color))
            .shadow_color(Rc::clone(&shadow_color))
            .surface_tint_color(Rc::clone(&surface_tint_color))
            .elevation(Rc::clone(&elevation))
            .padding(Rc::clone(&padding))
            .minimum_size(Rc::clone(&minimum_size))
            .fixed_size(Rc::clone(&fixed_size))
            .maximum_size(Rc::clone(&maximum_size))
            .icon_color(Rc::clone(&icon_color))
            .icon_size(Rc::clone(&icon_size))
            .side(Rc::clone(&side))
            .shape(Rc::clone(&shape))
            .mouse_cursor(Rc::clone(&mouse_cursor))
            .visual_density(visual_density)
            .tap_target_size(tap_target_size)
            .animation_duration(animation_duration)
            .enable_feedback(enable_feedback)
            .size_variant(size_variant)
            .icon_button_width(icon_button_width)
            .shape_variant(shape_variant);

        assert_eq!(
            style,
            ButtonStyle::new()
                .copy_with()
                .text_style(text_style)
                .background_color(background_color)
                .foreground_color(foreground_color)
                .overlay_color(overlay_color)
                .shadow_color(shadow_color)
                .surface_tint_color(surface_tint_color)
                .elevation(elevation)
                .padding(padding)
                .minimum_size(minimum_size)
                .fixed_size(fixed_size)
                .maximum_size(maximum_size)
                .icon_color(icon_color)
                .icon_size(icon_size)
                .side(side)
                .shape(shape)
                .mouse_cursor(mouse_cursor)
                .visual_density(visual_density)
                .tap_target_size(tap_target_size)
                .animation_duration(animation_duration)
                .enable_feedback(enable_feedback)
                .size_variant(size_variant)
                .icon_button_width(icon_button_width)
                .shape_variant(shape_variant)
        );

        assert_eq!(style, ButtonStyle::new().merge(Some(&style)));

        assert_eq!(style.copy_with(), style.merge(Some(&ButtonStyle::new())));
    }

    // button_style_test.dart 'ButtonStyle.lerp selects discrete variants at midpoint'.
    #[test]
    fn lerp_switches_the_discrete_variants_at_the_midpoint() {
        let a = ButtonStyle::new()
            .size_variant(ButtonSizeVariant::XSmall)
            .icon_button_width(IconButtonWidthVariant::Narrow)
            .shape_variant(ButtonShapeVariant::Round);
        let b = ButtonStyle::new()
            .size_variant(ButtonSizeVariant::XLarge)
            .icon_button_width(IconButtonWidthVariant::Wide)
            .shape_variant(ButtonShapeVariant::Square);

        let before = ButtonStyle::lerp(Some(&a), Some(&b), 0.49).expect("two styles");
        assert_eq!(before.size_variant, Some(ButtonSizeVariant::XSmall));
        assert_eq!(
            before.icon_button_width,
            Some(IconButtonWidthVariant::Narrow)
        );
        assert_eq!(before.shape_variant, Some(ButtonShapeVariant::Round));
        let at = ButtonStyle::lerp(Some(&a), Some(&b), 0.5).expect("two styles");
        assert_eq!(at.size_variant, Some(ButtonSizeVariant::XLarge));
        assert_eq!(at.icon_button_width, Some(IconButtonWidthVariant::Wide));
        assert_eq!(at.shape_variant, Some(ButtonShapeVariant::Square));
    }

    // button_style_test.dart 'ButtonStyle.lerp BorderSide'.
    #[test]
    fn lerp_fades_a_missing_side_from_or_to_an_empty_one() {
        assert!(ButtonStyle::lerp(None, None, 0.0).is_none());
        assert!(ButtonStyle::lerp(None, None, 0.5).is_none());
        assert!(ButtonStyle::lerp(None, None, 1.0).is_none());

        let black_side = BorderSide::default();
        let white_side = BorderSide {
            color: Color::new(0xFFFFFFFF),
            ..BorderSide::default()
        };
        let empty_black_side = BorderSide {
            width: 0.0,
            color: Color::new(0x00000000),
            ..BorderSide::default()
        };

        let black_style = ButtonStyle::new().side(all(black_side));
        let white_style = ButtonStyle::new().side(all(white_side));

        // WidgetStatePropertyAll(value) properties resolve to value
        // for any set of states.
        let states = WidgetStates::new();
        let side = |a: Option<&ButtonStyle>, b: Option<&ButtonStyle>, t: f64| {
            ButtonStyle::lerp(a, b, t)
                .and_then(|style| style.side)
                .and_then(|side| side.resolve(&states))
        };

        assert_eq!(
            side(Some(&black_style), Some(&black_style), 0.0),
            Some(black_side)
        );
        assert_eq!(
            side(Some(&black_style), Some(&black_style), 0.5),
            Some(black_side)
        );
        assert_eq!(
            side(Some(&black_style), Some(&black_style), 1.0),
            Some(black_side)
        );

        assert_eq!(side(Some(&black_style), None, 0.0), Some(black_side));
        assert_eq!(
            side(Some(&black_style), None, 0.5),
            Some(BorderSide::lerp(black_side, empty_black_side, 0.5))
        );
        assert_eq!(side(Some(&black_style), None, 1.0), Some(empty_black_side));

        assert_eq!(side(None, Some(&black_style), 0.0), Some(empty_black_side));
        assert_eq!(
            side(None, Some(&black_style), 0.5),
            Some(BorderSide::lerp(empty_black_side, black_side, 0.5))
        );
        assert_eq!(side(None, Some(&black_style), 1.0), Some(black_side));

        assert_eq!(
            side(Some(&black_style), Some(&white_style), 0.0),
            Some(black_side)
        );
        assert_eq!(
            side(Some(&black_style), Some(&white_style), 0.5),
            Some(BorderSide::lerp(black_side, white_side, 0.5))
        );
        assert_eq!(
            side(Some(&black_style), Some(&white_style), 1.0),
            Some(white_side)
        );
    }

    #[test]
    fn lerp_of_nullable_properties_resolves_to_the_lerped_values() {
        let a = ButtonStyle::new()
            .elevation(all(2.0))
            .shape(Rc::new(WidgetStatePropertyAll::new(Some(
                Box::new(StadiumBorder::default()) as Box<dyn OutlinedBorder>,
            ))));
        let b = ButtonStyle::new().elevation(<dyn WidgetStateProperty<Option<f64>>>::resolve_with(
            |states| states.contains(&WidgetState::Pressed).then_some(6.0),
        ));
        let lerped = ButtonStyle::lerp(Some(&a), Some(&b), 0.5).expect("two styles");
        let elevation = lerped.elevation.expect("both sides");
        assert_eq!(elevation.resolve(&WidgetStates::new()), Some(1.0));
        assert_eq!(
            elevation.resolve(&WidgetStates::from([WidgetState::Pressed])),
            Some(4.0)
        );
        let shape = lerped
            .shape
            .expect("one side")
            .resolve(&WidgetStates::new())
            .expect("a shape scaled from the stadium");
        assert!(shape.as_any().downcast_ref::<StadiumBorder>().is_some());
    }

    // button_style_test.dart 'Default ButtonStyle debugFillProperties'.
    #[test]
    fn a_new_style_describes_no_property() {
        assert_eq!(format!("{:?}", ButtonStyle::new()), "ButtonStyle");
    }

    // button_style_test.dart 'ButtonStyle debugFillProperties'.
    #[test]
    fn a_style_describes_the_properties_it_sets_in_dart_order() {
        let text_style = all(TextStyle::new().font_size(10.0));
        let background_color = all(Color::new(0xfffffff1));
        let foreground_color = all(Color::new(0xfffffff2));
        let overlay_color = all(Color::new(0xfffffff3));
        let shadow_color = all(Color::new(0xfffffff4));
        let surface_tint_color = all(Color::new(0xfffffff5));
        let elevation = all(1.5);
        let padding = all(EdgeInsetsGeometry::all(1.0));
        let minimum_size = all(Size::new(1.0, 2.0));
        let side = all(BorderSide::new(
            Color::new(0xfffffff6),
            4.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        ));
        let maximum_size = all(Size::new(100.0, 200.0));
        let icon_color = all(Color::new(0xfffffff6));
        let icon_size = all(48.1);
        let shape: WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>> =
            Rc::new(WidgetStatePropertyAll::new(Some(
                Box::new(StadiumBorder::default()) as Box<dyn OutlinedBorder>,
            )));
        let mouse_cursor = all::<MouseCursorRef>(SystemMouseCursors::FORBIDDEN.into());
        let style = ButtonStyle::new()
            .text_style(text_style.clone())
            .background_color(background_color.clone())
            .foreground_color(foreground_color.clone())
            .overlay_color(overlay_color.clone())
            .shadow_color(shadow_color.clone())
            .surface_tint_color(surface_tint_color.clone())
            .elevation(elevation.clone())
            .padding(padding.clone())
            .minimum_size(minimum_size.clone())
            .side(side.clone())
            .maximum_size(maximum_size.clone())
            .icon_color(icon_color.clone())
            .icon_size(icon_size.clone())
            .shape(shape.clone())
            .mouse_cursor(mouse_cursor.clone())
            .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
            .animation_duration(Duration::from_secs(1))
            .enable_feedback(true)
            .size_variant(ButtonSizeVariant::Small)
            .icon_button_width(IconButtonWidthVariant::Standard)
            .shape_variant(ButtonShapeVariant::Round);

        let description = format!("{style:?}");
        let expected = [
            format!("text_style: {text_style:?}"),
            format!("background_color: {background_color:?}"),
            format!("foreground_color: {foreground_color:?}"),
            format!("overlay_color: {overlay_color:?}"),
            format!("shadow_color: {shadow_color:?}"),
            format!("surface_tint_color: {surface_tint_color:?}"),
            format!("elevation: {elevation:?}"),
            format!("padding: {padding:?}"),
            format!("minimum_size: {minimum_size:?}"),
            format!("maximum_size: {maximum_size:?}"),
            format!("icon_color: {icon_color:?}"),
            format!("icon_size: {icon_size:?}"),
            format!("side: {side:?}"),
            format!("shape: {shape:?}"),
            format!("mouse_cursor: {mouse_cursor:?}"),
            "tap_target_size: ShrinkWrap".to_owned(),
            "animation_duration: 1s".to_owned(),
            "enable_feedback: true".to_owned(),
            "size_variant: Small".to_owned(),
            "icon_button_width: Standard".to_owned(),
            "shape_variant: Round".to_owned(),
        ];
        assert_eq!(
            description,
            format!("ButtonStyle {{ {} }}", expected.join(", "))
        );
    }
}
