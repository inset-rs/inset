//! Flutter counterpart: material_ui's `text_button.dart`; its Material 3 defaults, a Dart
//! `part`, are `generated/text_button_defaults_m3.rs`.
//!
//! Dart's `TextButton.icon` and `TextButton.styleFrom` take named arguments the body
//! combines (the icon child reads `style`; `styleFrom` derives its overlay color from two of
//! them), so each is a value collecting them, [`TextButtonIcon`] and
//! [`TextButtonStyleFromBuilder`], that converts once every argument is given.
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
    WidgetStatePropertyAll, WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyleGetters;
use crate::button_style::{ButtonLayerBuilder, ButtonStyle};
use crate::button_style_button::{
    ButtonStyleButton, ButtonStyleButtonData, ButtonStyleState, IconAlignment,
};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::generated::text_button_defaults_m3::TextButtonDefaultsM3;
use crate::ink_ripple::InkRipple;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::text_button_theme::TextButtonTheme;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// A Material Design "Text Button".
///
/// Use text buttons on toolbars, in dialogs, or inline with other
/// content but offset from that content with padding so that the
/// button's presence is obvious. Text buttons do not have visible
/// borders and must therefore rely on their position relative to
/// other content for context. In dialogs and cards, they should be
/// grouped together in one of the bottom corners. Avoid using text
/// buttons where they would blend in with other content, for example
/// in the middle of lists.
///
/// A text button is a label `child` displayed on a (zero elevation)
/// `Material` widget. The label's `Text` and `Icon` widgets are
/// displayed in the `style`'s [`ButtonStyle::foreground_color`]. The
/// button reacts to touches by filling with the `style`'s
/// [`ButtonStyle::background_color`].
///
/// The text button's default style is defined by
/// [`default_style_of`](ButtonStyleButton::default_style_of). The style of this text button can
/// be overridden with its `style` parameter. The style of all text buttons in a subtree can be
/// overridden with the [`TextButtonTheme`] and the style of all of the text buttons in an app
/// can be overridden with the `Theme`'s `ThemeData::text_button_theme` property.
///
/// The static [`style_from`](Self::style_from) method is a convenient way to create a
/// text button [`ButtonStyle`] from simple values.
///
/// If the `on_pressed` and `on_long_press` callbacks are null, then this
/// button will be disabled, it will not react to touch.
///
/// See also:
///
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `FilledButton`, a filled button that doesn't elevate when pressed.
///  * `FilledButton::tonal`, a filled button variant that uses a secondary fill color.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * <https://material.io/design/components/buttons.html>
///  * <https://m3.material.io/components/buttons>
pub struct TextButton {
    button_style_button: ButtonStyleButtonData,
    add_padding: bool,
}

impl TextButton {
    /// Create a [`TextButton`].
    pub fn new<K>(on_pressed: Option<Listener>, child: Option<impl IntoWidget<K>>) -> TextButton {
        TextButton {
            button_style_button: ButtonStyleButtonData::new(
                on_pressed,
                child.map(|child| child.into_widget()),
            ),
            add_padding: false,
        }
    }

    crate::button_style_button_setters!(
        TextButton;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller, is_semantic_button,
    );

    /// Create a text button from a pair of widgets that serve as the button's
    /// [`icon`](TextButtonIcon::icon) and `label`.
    ///
    /// The icon and label are arranged in a row and padded by 8 logical pixels
    /// at the ends, with an 8 pixel gap in between.
    ///
    /// If [`icon`](TextButtonIcon::icon) is null, this constructor will create a
    /// [`TextButton`] that doesn't display an icon.
    ///
    /// The alignment of the icon within the button is set by
    /// [`icon_alignment`](TextButtonIcon::icon_alignment); see [`IconAlignment`].
    pub fn icon<K>(on_pressed: Option<Listener>, label: impl IntoWidget<K>) -> TextButtonIcon {
        let mut button_style_button = ButtonStyleButtonData::new(on_pressed, None);
        button_style_button.clip_behavior = Some(Clip::None);
        TextButtonIcon {
            button_style_button,
            icon: None,
            label: label.into_widget(),
            icon_alignment: None,
        }
    }

    /// A static convenience method that constructs a text button
    /// [`ButtonStyle`] given simple values.
    ///
    /// The `foreground_color` and `disabled_foreground_color` colors are used
    /// to create a `WidgetStateProperty` [`ButtonStyle::foreground_color`], and
    /// a derived [`ButtonStyle::overlay_color`] if `overlay_color` isn't specified.
    ///
    /// The `background_color` and `disabled_background_color` colors are
    /// used to create a `WidgetStateProperty` [`ButtonStyle::background_color`].
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
    /// If `overlay_color` is specified and its value is [`Colors::TRANSPARENT`]
    /// then the pressed/focused/hovered highlights are effectively defeated.
    /// Otherwise a `WidgetStateProperty` with the same opacities as the
    /// default is created.
    ///
    /// All of the other parameters are either used directly or used to
    /// create a `WidgetStateProperty` with a single value for all
    /// states.
    ///
    /// All parameters default to null. By default this method returns
    /// a [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default text and icon colors for a
    /// [`TextButton`], as well as its overlay color, with all of the
    /// standard opacity adjustments for the pressed, focused, and
    /// hovered states, one could write:
    ///
    /// ```text
    /// TextButton::new(on_pressed, Text::new("Give Kate a mix tape")).style(
    ///     TextButton::style_from().foreground_color(Colors::GREEN.primary()).build(),
    /// )
    /// ```
    pub fn style_from() -> TextButtonStyleFromBuilder {
        TextButtonStyleFromBuilder::default()
    }
}

impl ButtonStyleButton for TextButton {
    crate::button_style_button_accessors!();

    /// Defines the button's default appearance.
    ///
    /// The button `child`'s `Text` and `Icon` widgets are rendered with
    /// the [`ButtonStyle`]'s foreground color. The button's `InkWell` adds
    /// the style's overlay color when the button is focused, hovered
    /// or pressed. The button's background color becomes its `Material`
    /// color and is transparent by default.
    ///
    /// All of the [`ButtonStyle`]'s defaults appear below.
    ///
    /// In this list "Theme.foo" is shorthand for
    /// `Theme::of(app, context).foo`. Color scheme values like
    /// "onSurface(0.38)" are shorthand for
    /// `on_surface.with_opacity(0.38)`. `WidgetStateProperty` valued
    /// properties that are not followed by a sublist have the same
    /// value for all states, otherwise the values are as specified for
    /// each state and "others" means all other states.
    ///
    /// The "default font size" below refers to the font size specified in the
    /// [`default_style_of`](ButtonStyleButton::default_style_of) method (or 14.0 if
    /// unspecified), scaled by the `MediaQuery::text_scaler_of(app, context).scale` method. And
    /// the names of the EdgeInsets constructors and `EdgeInsetsGeometry::lerp` have been
    /// abbreviated for readability.
    ///
    /// The color of the [`ButtonStyle::text_style`] is not used, the
    /// [`ButtonStyle::foreground_color`] color is used instead.
    ///
    /// ## Material 2 defaults
    ///
    /// * `textStyle` - Theme.textTheme.button
    /// * `backgroundColor` - transparent
    /// * `foregroundColor`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others - Theme.colorScheme.primary
    /// * `overlayColor`
    ///   * hovered - Theme.colorScheme.primary(0.08)
    ///   * focused or pressed - Theme.colorScheme.primary(0.12)
    /// * `shadowColor` - Theme.shadowColor
    /// * `elevation` - 0
    /// * `padding`
    ///   * `default font size <= 14` - (horizontal(12), vertical(8))
    ///   * `14 < default font size <= 28` - lerp(all(8), horizontal(8))
    ///   * `28 < default font size <= 36` - lerp(horizontal(8), horizontal(4))
    ///   * `36 < default font size` - horizontal(4)
    /// * `minimumSize` - Size(64, 36)
    /// * `fixedSize` - null
    /// * `maximumSize` - Size.infinite
    /// * `side` - null
    /// * `shape` - RoundedRectangleBorder(borderRadius: BorderRadius.circular(4))
    /// * `mouseCursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visualDensity` - theme.visualDensity
    /// * `tapTargetSize` - theme.materialTapTargetSize
    /// * `animationDuration` - kThemeChangeDuration
    /// * `enableFeedback` - true
    /// * `alignment` - Alignment.center
    /// * `splashFactory` - InkRipple.splashFactory
    ///
    /// The default padding values for the [`TextButton::icon`] factory are slightly different:
    ///
    /// * `padding`
    ///   * `default font size <= 14` - all(8)
    ///   * `14 < default font size <= 28 `- lerp(all(8), horizontal(4))
    ///   * `28 < default font size` - horizontal(4)
    ///
    /// The default value for `side`, which defines the appearance of the button's
    /// outline, is null. That means that the outline is defined by the button
    /// shape's `OutlinedBorder::side`. Typically the default value of an
    /// `OutlinedBorder`'s side is `BorderSide::NONE`, so an outline is not drawn.
    ///
    /// ## Material 3 defaults
    ///
    /// If `ThemeData::use_material3` is set to true the following defaults will
    /// be used:
    ///
    /// * `textStyle` - Theme.textTheme.labelLarge
    /// * `backgroundColor` - transparent
    /// * `foregroundColor`
    ///   * disabled - Theme.colorScheme.onSurface(0.38)
    ///   * others - Theme.colorScheme.primary
    /// * `overlayColor`
    ///   * hovered - Theme.colorScheme.primary(0.08)
    ///   * focused or pressed - Theme.colorScheme.primary(0.1)
    ///   * others - null
    /// * `shadowColor` - Colors.transparent,
    /// * `surfaceTintColor` - null
    /// * `elevation` - 0
    /// * `padding`
    ///   * `default font size <= 14` - lerp(horizontal(12), horizontal(4))
    ///   * `14 < default font size <= 28` - lerp(all(8), horizontal(8))
    ///   * `28 < default font size <= 36` - lerp(horizontal(8), horizontal(4))
    ///   * `36 < default font size` - horizontal(4)
    /// * `minimumSize` - Size(64, 40)
    /// * `fixedSize` - null
    /// * `maximumSize` - Size.infinite
    /// * `side` - null
    /// * `shape` - StadiumBorder()
    /// * `mouseCursor` - WidgetStateMouseCursor.adaptiveClickable
    /// * `visualDensity` - theme.visualDensity
    /// * `tapTargetSize` - theme.materialTapTargetSize
    /// * `animationDuration` - kThemeChangeDuration
    /// * `enableFeedback` - true
    /// * `alignment` - Alignment.center
    /// * `splashFactory` - Theme.splashFactory
    ///
    /// For the [`TextButton::icon`] factory, the end (generally the right) value of
    /// `padding` is increased from 12 to 16.
    fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let theme = Theme::of(app, context);
        let color_scheme = &theme.color_scheme;
        let button_style: Box<dyn ButtonStyleGetters> = if theme.use_material3 {
            Box::new(TextButtonDefaultsM3::new(context))
        } else {
            Box::new(
                TextButtonStyleFromBuilder {
                    text_style: theme.text_theme.label_large.clone(),
                    ..TextButton::style_from()
                }
                .foreground_color(color_scheme.primary)
                .disabled_foreground_color(color_scheme.on_surface.with_opacity(0.38))
                .background_color(Colors::TRANSPARENT)
                .disabled_background_color(Colors::TRANSPARENT)
                .shadow_color(theme.shadow_color)
                .elevation(0.0)
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
                .build(),
            )
        };

        // Only apply padding when TextButton has an Icon.
        if self.add_padding {
            let default_font_size = default_font_size(app, Some(&*button_style));
            let effective_text_scale =
                MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;
            let scaled_padding = <dyn ButtonStyleButton>::scaled_padding(
                if theme.use_material3 {
                    EdgeInsetsGeometry::from_steb(12.0, 8.0, 16.0, 8.0)
                } else {
                    EdgeInsetsGeometry::all(8.0)
                },
                EdgeInsetsGeometry::symmetric(0.0, 4.0),
                EdgeInsetsGeometry::symmetric(0.0, 4.0),
                effective_text_scale,
            );
            let padding = Rc::new(MaterialStatePropertyAll::new(Some(scaled_padding)));
            return Box::new(button_style.copy_with(app).padding(padding));
        }

        button_style
    }

    /// Returns the [`TextButtonThemeData::style`] of the closest
    /// [`TextButtonTheme`] ancestor.
    fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        TextButtonTheme::of(app, context).style
    }
}

impl StatefulWidget for TextButton {
    type State = ButtonStyleState<TextButton>;

    fn key(&self) -> Option<&KeyRef> {
        self.button_style_button.key.as_ref()
    }

    fn create_state(&self) -> ButtonStyleState<TextButton> {
        ButtonStyleState::new()
    }
}

impl Debug for TextButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.button_style_button, f)
    }
}

/// Dart's `_scaledPadding`, which the Material 3 defaults share.
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
    <dyn ButtonStyleButton>::scaled_padding(
        if theme.use_material3 {
            EdgeInsetsGeometry::symmetric(8.0, 12.0)
        } else {
            EdgeInsetsGeometry::all(8.0)
        },
        EdgeInsetsGeometry::symmetric(0.0, 8.0),
        EdgeInsetsGeometry::symmetric(0.0, 4.0),
        effective_text_scale,
    )
}

/// The font size of `button_style`'s text style with no state, or 14.
fn default_font_size(app: &mut App, button_style: Option<&dyn ButtonStyleGetters>) -> f64 {
    button_style
        .and_then(|style| style.text_style(app))
        .and_then(|text_style| text_style.resolve(&WidgetStates::new()))
        .and_then(|text_style| text_style.font_size)
        .unwrap_or(14.0)
}

/// Dart's `_TextButtonWithIconChild`.
#[derive(Debug)]
struct TextButtonWithIconChild {
    label: WidgetRef,
    icon: WidgetRef,
    button_style: Option<ButtonStyle>,
    icon_alignment: Option<IconAlignment>,
}

impl StatelessWidget for TextButtonWithIconChild {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let default_font_size = default_font_size(
            app,
            self.button_style
                .as_ref()
                .map(|style| style as &dyn ButtonStyleGetters),
        );
        let scale = clamp_double(
            MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0,
            1.0,
            2.0,
        ) - 1.0;

        let text_button_theme = TextButtonTheme::of(app, context);
        let effective_icon_alignment = self
            .icon_alignment
            .or(text_button_theme
                .style
                .and_then(|style| style.icon_alignment))
            .or(self
                .button_style
                .as_ref()
                .and_then(|style| style.icon_alignment))
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

/// Dart's `TextButton.icon(..)` arguments; the value converts to the [`TextButton`].
pub struct TextButtonIcon {
    button_style_button: ButtonStyleButtonData,
    icon: Option<WidgetRef>,
    label: WidgetRef,
    icon_alignment: Option<IconAlignment>,
}

impl TextButtonIcon {
    crate::button_style_button_setters!(
        TextButtonIcon;
        key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
        clip_behavior, states_controller,
    );

    /// Dart `TextButton.icon(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> TextButtonIcon {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `TextButton.icon(iconAlignment:)`.
    pub fn icon_alignment(mut self, icon_alignment: IconAlignment) -> TextButtonIcon {
        self.icon_alignment = Some(icon_alignment);
        self
    }

    /// The [`TextButton`] these arguments describe.
    pub fn build(self) -> TextButton {
        let mut button_style_button = self.button_style_button;
        let add_padding = self.icon.is_some();
        button_style_button.child = Some(match self.icon {
            Some(icon) => TextButtonWithIconChild {
                label: self.label,
                icon,
                button_style: button_style_button.style.clone(),
                icon_alignment: self.icon_alignment,
            }
            .into_widget(),
            None => self.label,
        });
        TextButton {
            button_style_button,
            add_padding,
        }
    }
}

impl IntoWidget<StatefulKind> for TextButtonIcon {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `TextButton.styleFrom(..)` arguments; [`build`](Self::build) makes the
/// [`ButtonStyle`].
#[derive(Clone, Default)]
pub struct TextButtonStyleFromBuilder {
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

/// The fluent setters of Dart's named `styleFrom` arguments.
macro_rules! style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `TextButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> TextButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl TextButtonStyleFromBuilder {
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

    /// Dart `TextButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> TextButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The [`ButtonStyle`] made from these values.
    pub fn build(self) -> ButtonStyle {
        let background_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> =
            match (self.background_color, self.disabled_background_color) {
                (Some(_), None) => Some(Rc::new(MaterialStatePropertyAll::new(
                    self.background_color,
                ))),
                (_, _) => <dyn ButtonStyleButton>::default_color(
                    self.background_color,
                    self.disabled_background_color,
                ),
            };
        let icon_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> =
            match (self.icon_color, self.disabled_icon_color) {
                (Some(_), None) => Some(Rc::new(MaterialStatePropertyAll::new(self.icon_color))),
                (_, _) => <dyn ButtonStyleButton>::default_color(
                    self.icon_color,
                    self.disabled_icon_color,
                ),
            };
        let overlay_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> =
            match (self.foreground_color, self.overlay_color) {
                (None, None) => None,
                (_, Some(overlay_color)) if overlay_color.a == 0.0 => {
                    Some(Rc::new(WidgetStatePropertyAll::new(self.overlay_color)))
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
            text_style: <dyn ButtonStyleButton>::all_or_null(self.text_style),
            foreground_color: <dyn ButtonStyleButton>::default_color(
                self.foreground_color,
                self.disabled_foreground_color,
            ),
            background_color: background_color_prop,
            overlay_color: overlay_color_prop,
            shadow_color: <dyn ButtonStyleButton>::all_or_null(self.shadow_color),
            surface_tint_color: <dyn ButtonStyleButton>::all_or_null(self.surface_tint_color),
            icon_color: icon_color_prop,
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
            ..ButtonStyle::new()
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::drawer_theme::tests::assert_near;
    use std::cell::{Cell, RefCell};

    use waft_embedder::{FontWeight, Offset, PointerChange, Rect, TargetPlatform, TextDirection};
    use waft_painting::{ShapeBorder, ShapeBorderRef, StadiumBorder};
    use waft_widgets::{
        Align, Builder, Center, Directionality, FocusHighlightStrategy, FocusNode, GlobalKey, Icon,
        MouseRegion, Padding, SizedBox, Text, UnfocusDisposition,
    };

    use waft_foundation::Listenable;

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::icons::Icons;
    use crate::ink_sparkle::InkSparkle;
    use crate::ink_well::InkWell;
    use crate::material::{Material, MaterialType};
    use crate::material_state::MaterialStatesController;
    use crate::no_splash::NoSplash;
    use crate::test_support::{Drawn, PaintInspection, material_app};
    use crate::text_button_theme::TextButtonThemeData;
    use crate::text_theme::TextTheme;
    use crate::theme_data::ThemeData;
    use waft_widgets::FocusNodeRef;

    fn pressed() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// Dart's `ThemeData.from(colorScheme: const ColorScheme.light(), useMaterial3: ..)`.
    fn light(tester: &WidgetTester, use_material3: bool) -> ThemeData {
        ThemeData::from(ColorScheme::light())
            .use_material3(use_material3)
            .build(&tester.cell.borrow())
    }

    /// Dart's `ThemeData()`.
    fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    /// `MaterialApp(theme: theme, home: Center(child: child))`.
    fn app_centered<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(theme, Center::new().child(child).into_widget())
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    /// Reads the button's `Material`, the last one in the tree: Dart's
    /// `tester.widget<Material>(find.descendant(of: find.byType(TextButton), matching:
    /// find.byType(Material)))`.
    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let element = *tester
            .find_by_type::<Material>()
            .last()
            .expect("the button's Material");
        tester.widget(element, read)
    }

    fn material_shape_is(tester: &WidgetTester, expected: &dyn ShapeBorder) -> bool {
        material(tester, |material| {
            material
                .shape
                .as_deref()
                .expect("a button gives its Material a shape")
                .eq_shape(expected)
        })
    }

    fn ink_well<R>(tester: &WidgetTester, read: impl FnOnce(&InkWell) -> R) -> R {
        let element = *tester.find_by_type::<InkWell>().last().expect("an InkWell");
        tester.widget(element, read)
    }

    fn button_enabled(tester: &WidgetTester) -> bool {
        let element = *tester
            .find_by_type::<TextButton>()
            .first()
            .expect("an TextButton");
        tester.widget(element, |button: &TextButton| button.enabled())
    }

    /// The center of the first `TextButton`: Dart's
    /// `tester.getCenter(find.byType(TextButton))`.
    fn button_center(tester: &WidgetTester) -> Offset {
        let element = *tester
            .find_by_type::<TextButton>()
            .first()
            .expect("an TextButton");
        tester.global_rect(tester.render_box_of(element)).center()
    }

    fn button_rect(tester: &WidgetTester) -> Rect {
        let element = *tester
            .find_by_type::<Material>()
            .last()
            .expect("the button's Material");
        tester.global_rect(tester.render_box_of(element))
    }

    fn icon_rect(tester: &WidgetTester) -> Rect {
        let element = *tester.find_by_type::<Icon>().first().expect("an Icon");
        tester.global_rect(tester.render_box_of(element))
    }

    /// The colors of the rects the button's ink features draw.
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

    fn ink_circle_count(tester: &WidgetTester) -> usize {
        tester
            .ink_drawn()
            .into_iter()
            .filter(|drawn| matches!(drawn, Drawn::Path(..)))
            .count()
    }

    fn padding_of_button(tester: &WidgetTester) -> EdgeInsetsGeometry {
        let button = *tester
            .find_by_type::<TextButton>()
            .first()
            .expect("an TextButton");
        let padding = tester
            .find_by_type::<Padding>()
            .into_iter()
            .find(|padding| tester.ancestors(*padding).contains(&button))
            .expect("the button's Padding");
        tester.widget(padding, |padding: &Padding| padding.padding)
    }

    fn mouse() -> TestPointer {
        TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON)
    }

    fn argb(color: Color) -> u32 {
        color.value()
    }

    /// Checks the `Material` of a default button; `disabled` picks the disabled colors.
    fn check_default_material(
        tester: &WidgetTester,
        color_scheme: &ColorScheme,
        material3: bool,
        disabled: bool,
    ) {
        material(tester, |material| {
            assert_eq!(material.animation_duration, Duration::from_millis(200));
            assert!(!material.border_on_foreground);
            assert!(material.border_radius.is_none());
            assert_eq!(material.clip_behavior, waft_embedder::Clip::None);
            assert_eq!(material.color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 0.0);
            assert_eq!(
                material.shadow_color,
                Some(if material3 {
                    Colors::TRANSPARENT
                } else {
                    Color::new(0xff000000)
                })
            );
            let text_style = material.text_style.as_ref().expect("a text style");
            assert_eq!(
                text_style.color.as_deref().copied(),
                Some(if disabled {
                    color_scheme.on_surface.with_opacity(0.38)
                } else {
                    color_scheme.primary
                })
            );
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.font_size, Some(14.0));
            assert_eq!(text_style.font_weight, Some(FontWeight::W500));
            assert_eq!(material.r#type, MaterialType::Button);
        });
        let expected: ShapeBorderRef = if material3 {
            ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE))
        } else {
            ShapeBorderRef::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadius::all(Radius::circular(4.0)).into(),
            ))
        };
        assert!(material_shape_is(tester, &*expected));
    }

    // 'TextButton, TextButton.icon defaults', which Flutter runs in Material 3; the Material 2
    // values are its other branch.
    fn defaults_case(tester: &mut WidgetTester, material3: bool) {
        let color_scheme = ColorScheme::light();
        let theme = light(tester, material3);

        // Enabled TextButton
        tester.pump_widget(app_centered(
            theme.clone(),
            TextButton::new(pressed(), Some(Text::new("button"))),
        ));
        check_default_material(tester, &color_scheme, material3, false);
        let label = *tester.find_text("button").first().expect("the label");
        let align = tester.ancestor_of_type::<Align>(label);
        tester.widget(align, |align: &Align| {
            assert!(align.alignment == AlignmentGeometry::CENTER);
        });

        let mut touch = TestPointer::touch(1);
        tester.down(&mut touch, button_center(tester));
        tester.pump(); // start the splash animation
        tester.pump_by(Duration::from_millis(100)); // splash is underway
        tester.up(&touch);
        tester.pump_and_settle();
        // No change vs enabled and not pressed.
        check_default_material(tester, &color_scheme, material3, false);

        // Enabled TextButton.icon
        let icon_button_key = GlobalKey::new();
        tester.pump_widget(app_centered(
            theme.clone(),
            TextButton::icon(pressed(), Text::new("label"))
                .key(icon_button_key.into())
                .icon(Icon::new(Some(Icons::add()))),
        ));
        check_default_material(tester, &color_scheme, material3, false);

        // Disabled TextButton
        tester.pump_widget(app_centered(
            theme,
            TextButton::new(None, Some(Text::new("button"))),
        ));
        check_default_material(tester, &color_scheme, material3, true);
    }

    #[test]
    fn material_3_defaults_of_text_button_and_its_icon_variant_per_state() {
        test_widgets(|tester| defaults_case(tester, true));
    }

    #[test]
    fn material_2_defaults_of_text_button_and_its_icon_variant_per_state() {
        test_widgets(|tester| defaults_case(tester, false));
    }

    /// Which of a default style's properties are set, read where `button` builds.
    fn default_style_fields(
        tester: &mut WidgetTester,
        button: TextButton,
    ) -> (Vec<bool>, Vec<bool>) {
        let seen = Rc::new(RefCell::new(None));
        let button = Rc::new(button);
        let theme = light(tester, true);
        tester.pump_widget(app_centered(
            theme,
            Builder::new({
                let seen = Rc::clone(&seen);
                move |app, context| {
                    let style = button.default_style_of(app, context).copy_with(app);
                    *seen.borrow_mut() = Some((
                        vec![
                            style.text_style.is_some(),
                            style.background_color.is_some(),
                            style.foreground_color.is_some(),
                            style.overlay_color.is_some(),
                            style.shadow_color.is_some(),
                            style.surface_tint_color.is_some(),
                            style.elevation.is_some(),
                            style.padding.is_some(),
                            style.minimum_size.is_some(),
                            style.maximum_size.is_some(),
                            style.icon_color.is_some(),
                            style.icon_size.is_some(),
                            style.shape.is_some(),
                            style.mouse_cursor.is_some(),
                            style.visual_density.is_some(),
                            style.tap_target_size.is_some(),
                            style.animation_duration.is_some(),
                            style.enable_feedback.is_some(),
                            style.alignment.is_some(),
                            style.splash_factory.is_some(),
                        ],
                        vec![
                            style.fixed_size.is_some(),
                            style.side.is_some(),
                            style.background_builder.is_some(),
                            style.foreground_builder.is_some(),
                        ],
                    ));
                    SizedBox::shrink().into_widget()
                }
            }),
        ));
        seen.borrow_mut().take().expect("the builder ran")
    }

    #[test]
    fn the_default_style_sets_every_property_but_the_fixed_size_the_side_and_the_builders() {
        test_widgets(|tester| {
            let (set, unset) = default_style_fields(
                tester,
                TextButton::new(pressed(), Some(Text::new("button"))),
            );
            assert!(set.iter().all(|set| *set), "{set:?}");
            assert!(unset.iter().all(|set| !*set), "{unset:?}");

            let (set, unset) = default_style_fields(
                tester,
                TextButton::icon(pressed(), Text::new("button"))
                    .icon(SizedBox::new())
                    .build(),
            );
            assert!(set.iter().all(|set| *set), "{set:?}");
            assert!(unset.iter().all(|set| !*set), "{unset:?}");
        });
    }

    #[test]
    fn text_button_icon_without_an_icon_shows_only_the_label() {
        test_widgets(|tester| {
            let theme = light(tester, true);
            tester.pump_widget(app_centered(
                theme.clone(),
                TextButton::icon(pressed(), Text::new("label")).icon(Icon::new(Some(Icons::add()))),
            ));
            assert_eq!(tester.find_by_type::<Icon>().len(), 1);
            assert_eq!(tester.find_text("label").len(), 1);

            tester.pump_widget(app_centered(
                theme,
                // No icon specified.
                TextButton::icon(pressed(), Text::new("label")),
            ));
            assert!(tester.find_by_type::<Icon>().is_empty());
            assert_eq!(tester.find_text("label").len(), 1);
        });
    }

    #[test]
    fn the_default_overlay_color_resolves_hovered_pressed_and_focused() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let theme = default_theme(tester);
            let primary = theme.color_scheme.primary;
            tester.pump_widget(app_centered(
                theme,
                TextButton::new(pressed(), Some(Text::new("TextButton")))
                    .focus_node(focus_node.clone()),
            ));

            // Hovered.
            let center = button_center(tester);
            let mut gesture = mouse();
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(primary.with_opacity(0.08))));

            // Highlighted (pressed).
            tester.send(&gesture, PointerChange::Down);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(
                colors[1..].contains(&argb(primary.with_opacity(0.1))),
                "{colors:x?}"
            );
            // Remove pressed and hovered states
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Focused.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(primary.with_opacity(0.1))));
        });
    }

    #[test]
    fn a_style_overlay_color_shows_while_hovered() {
        test_widgets(|tester| {
            let hover_color = Color::new(0xff001122);
            tester.pump_widget(ltr(TextButton::new(pressed(), Some(Text::new("button")))
                .style(ButtonStyle::new().overlay_color(<dyn WidgetStateProperty<
                    Option<Color>,
                >>::resolve_with(
                    move |states| {
                        states
                            .contains(&WidgetState::Hovered)
                            .then_some(hover_color)
                    },
                )))));

            let mut gesture = mouse();
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, button_center(tester), false);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(hover_color)));
        });
    }

    #[test]
    fn a_style_overlay_color_shows_while_focused() {
        test_widgets(|tester| {
            let focus_color = Color::new(0xff001122);
            let focus_node = focus_node(tester);
            tester.pump_widget(ltr(TextButton::new(pressed(), Some(Text::new("button")))
                .style(ButtonStyle::new().overlay_color(
                    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                        states
                            .contains(&WidgetState::Focused)
                            .then_some(focus_color)
                    }),
                ))
                .focus_node(focus_node.clone())));

            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            assert!(ink_rect_colors(tester).contains(&argb(focus_color)));
        });
    }

    const PRESSED_COLOR: Color = Color::new(0x00000001);
    const HOVER_COLOR: Color = Color::new(0x00000002);
    const FOCUSED_COLOR: Color = Color::new(0x00000003);
    const DEFAULT_COLOR: Color = Color::new(0x00000004);

    fn color_for(states: &WidgetStates) -> Color {
        if states.contains(&WidgetState::Pressed) {
            return PRESSED_COLOR;
        }
        if states.contains(&WidgetState::Hovered) {
            return HOVER_COLOR;
        }
        if states.contains(&WidgetState::Focused) {
            return FOCUSED_COLOR;
        }
        DEFAULT_COLOR
    }

    /// Runs default, focused, hovered and pressed, reading `read` at each.
    fn stateful_color_case(
        tester: &mut WidgetTester,
        button: impl FnOnce(waft_widgets::FocusNodeRef) -> WidgetRef,
        read: impl Fn(&WidgetTester) -> Color,
        settle_pressed: bool,
    ) {
        let focus_node = focus_node(tester);
        let theme = default_theme(tester);
        tester.pump_widget(app_centered(theme, button(focus_node.clone())));

        // Default, not disabled.
        assert_eq!(read(tester), DEFAULT_COLOR);

        // Focused.
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        assert_eq!(read(tester), FOCUSED_COLOR);

        // Hovered.
        let center = button_center(tester);
        let mut gesture = mouse();
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, center, false);
        tester.pump_and_settle();
        assert_eq!(read(tester), HOVER_COLOR);

        // Highlighted (pressed).
        tester.send(&gesture, PointerChange::Down);
        if settle_pressed {
            tester.pump_and_settle();
        } else {
            tester.pump(); // Start the splash and highlight animations.
            tester.pump_by(Duration::from_millis(800)); // Wait for splash and highlight to be well under way.
        }
        assert_eq!(read(tester), PRESSED_COLOR);
    }

    #[test]
    fn a_stateful_foreground_color_colors_the_text_in_each_state() {
        test_widgets(|tester| {
            stateful_color_case(
                tester,
                |focus_node| {
                    TextButton::new(pressed(), Some(Text::new("TextButton")))
                        .style(
                            ButtonStyle::new().foreground_color(<dyn WidgetStateProperty<
                                Option<Color>,
                            >>::resolve_with(
                                |states| Some(color_for(states))
                            )),
                        )
                        .focus_node(focus_node.clone())
                        .into_widget()
                },
                |tester| tester.text_color("TextButton"),
                false,
            );
        });
    }

    #[test]
    fn a_stateful_icon_color_colors_the_icon_in_each_state() {
        test_widgets(|tester| {
            stateful_color_case(
                tester,
                |focus_node| {
                    TextButton::icon(pressed(), Text::new("TextButton"))
                        .style(ButtonStyle::new().icon_color(<dyn WidgetStateProperty<
                            Option<Color>,
                        >>::resolve_with(
                            |states| Some(color_for(states))
                        )))
                        .icon(Icon::new(Some(Icons::add())))
                        .focus_node(focus_node.clone())
                        .into_widget()
                },
                |tester| *tester.icon_style().color.expect("an icon color"),
                false,
            );
        });
    }

    #[test]
    fn on_pressed_or_on_long_press_enables_the_button_and_is_called() {
        test_widgets(|tester| {
            let was_pressed = Rc::new(Cell::new(false));
            let setter = || {
                let was_pressed = Rc::clone(&was_pressed);
                Listener::new(move |_app| was_pressed.set(true))
            };

            // onPressed not null, onLongPress null.
            tester.pump_widget(ltr(TextButton::new(
                Some(setter()),
                Some(Text::new("button")),
            )));
            assert!(button_enabled(tester));
            tester.tap(button_center(tester), 1);
            assert!(was_pressed.get());

            // onPressed null, onLongPress not null.
            was_pressed.set(false);
            tester.pump_widget(ltr(
                TextButton::new(None, Some(Text::new("button"))).on_long_press(setter())
            ));
            assert!(button_enabled(tester));
            tester.long_press(button_center(tester), 2);
            assert!(was_pressed.get());

            // onPressed null, onLongPress null.
            tester.pump_widget(ltr(TextButton::new(None, Some(Text::new("button")))));
            assert!(!button_enabled(tester));
        });
    }

    fn hover_setter(hovering: &Rc<Cell<bool>>) -> waft_foundation::ValueChanged<bool> {
        let hovering = Rc::clone(hovering);
        Rc::new(move |_app, value| hovering.set(value))
    }

    // "TextButton response doesn't hover when disabled".
    #[test]
    fn a_button_disabled_while_focused_and_hovered_loses_focus() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTouch);
            let focus_node = focus_node(tester);
            let child_key = GlobalKey::new();
            let hovering = Rc::new(Cell::new(false));
            let frame = |enabled: bool| {
                let child = SizedBox::new().key(child_key.clone().into());
                let mut button =
                    TextButton::new(if enabled { pressed() } else { None }, Some(child))
                        .on_hover(hover_setter(&hovering))
                        .focus_node(focus_node.clone());
                if enabled {
                    button = button
                        .autofocus(true)
                        .on_long_press(Listener::new(|_app| {}));
                }
                ltr(SizedBox::square(Some(100.0)).child(button))
            };
            tester.pump_widget(frame(true));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            let mut gesture = mouse();
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, tester.center_of(&child_key), false);
            tester.pump_and_settle();
            assert!(hovering.get());

            tester.pump_widget(frame(false));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    fn mouse_exit_case(tester: &mut WidgetTester) {
        let on_hover_count = Rc::new(Cell::new(0));
        let hover = Rc::new(Cell::new(false));
        let on_hover = || -> waft_foundation::ValueChanged<bool> {
            let on_hover_count = Rc::clone(&on_hover_count);
            let hover = Rc::clone(&hover);
            Rc::new(move |_app, value| {
                on_hover_count.set(on_hover_count.get() + 1);
                hover.set(value);
            })
        };
        let frame = |enabled: bool| {
            let on_pressed = if enabled { pressed() } else { None };
            let button =
                TextButton::new(on_pressed, Some(Text::new("TextButton"))).on_hover(on_hover());
            ltr(Center::new().child(SizedBox::square(Some(100.0)).child(button)))
        };

        tester.pump_widget(frame(true));
        let mut gesture = mouse();
        tester.add_pointer(&mut gesture, Offset::ZERO);

        tester.move_to(&mut gesture, button_center(tester), false);
        tester.pump_and_settle();
        assert_eq!(on_hover_count.get(), 1);
        assert!(hover.get());

        tester.pump_widget(frame(false));
        tester.pump_and_settle();
        tester.move_to(&mut gesture, Offset::ZERO, false);
        // Even though the TextButton has been disabled, the mouse-exit still
        // causes onHover(false) to be called.
        assert_eq!(on_hover_count.get(), 2);
        assert!(!hover.get());

        tester.move_to(&mut gesture, button_center(tester), false);
        tester.pump_and_settle();
        // We no longer see hover events because the TextButton is disabled
        // and it's no longer in the "hovering" state.
        assert_eq!(on_hover_count.get(), 2);
        assert!(!hover.get());

        tester.pump_widget(frame(true));
        tester.pump_and_settle();
        // The TextButton was enabled while it contained the mouse, however
        // we do not call onHover() because it may call setState().
        assert_eq!(on_hover_count.get(), 2);
        assert!(!hover.get());

        tester.move_to(
            &mut gesture,
            button_center(tester) - Offset::new(1.0, 1.0),
            false,
        );
        tester.pump_and_settle();
        // Moving the mouse a little within the TextButton doesn't change anything.
        assert_eq!(on_hover_count.get(), 2);
        assert!(!hover.get());
    }

    #[test]
    fn a_disabled_and_hovered_button_reports_the_mouse_exit() {
        test_widgets(mouse_exit_case);
    }

    fn focus_change_case(tester: &mut WidgetTester, enabled: bool) {
        let node = focus_node(tester);
        let got_focus = Rc::new(Cell::new(false));
        let on_focus_change: waft_foundation::ValueChanged<bool> = {
            let got_focus = Rc::clone(&got_focus);
            Rc::new(move |_app, focused| got_focus.set(focused))
        };
        let on_pressed = if enabled { pressed() } else { None };
        let button = TextButton::new(on_pressed, Some(SizedBox::new()))
            .focus_node(node.clone())
            .on_focus_change(on_focus_change);
        tester.pump_widget(ltr(button));

        node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump();
        assert_eq!(got_focus.get(), enabled);
        assert_eq!(node.has_focus(&mut tester.cell.borrow_mut()), enabled);
        if !enabled {
            return;
        }

        node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
        tester.pump();
        assert!(!got_focus.get());
        assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
    }

    #[test]
    fn a_button_can_be_focused_and_unfocused() {
        test_widgets(|tester| focus_change_case(tester, true));
    }

    #[test]
    fn a_disabled_button_cannot_be_focused() {
        test_widgets(|tester| focus_change_case(tester, false));
    }

    #[test]
    fn a_button_disabled_during_a_gesture_does_not_crash() {
        test_widgets(|tester| {
            let frame = |tester: &WidgetTester, on_pressed: Option<Listener>| {
                let theme = default_theme(tester);
                ltr(Theme::new(
                    theme,
                    Center::new().child(TextButton::new(on_pressed, Some(Text::new("button")))),
                ))
            };
            let first = frame(tester, pressed());
            tester.pump_widget(first);
            let mut touch = TestPointer::touch(1);
            tester.down(&mut touch, button_center(tester));
            tester.pump_and_settle();
            let second = frame(tester, None);
            tester.pump_widget(second);
            tester.pump_and_settle();
        });
    }

    // 'Does TextButton scale with font scale changes'.
    #[test]
    fn the_text_scale_grows_the_text_and_then_the_button() {
        test_widgets(|tester| {
            let label_key = GlobalKey::new();
            let button_key = GlobalKey::new();
            let frame = |tester: &WidgetTester, scale: Option<f64>| {
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                let button = TextButton::new(
                    pressed(),
                    Some(Text::new("ABC").key(label_key.clone().into())),
                )
                .key(button_key.clone().into());
                let center = Center::new().child(button).into_widget();
                let scaled = match scale {
                    Some(scale) => {
                        MediaQuery::with_clamped_text_scaling(None, scale, scale, center)
                    }
                    None => {
                        MediaQuery::new(waft_widgets::MediaQueryData::new(), center).into_widget()
                    }
                };
                Theme::new(theme, ltr(scaled)).into_widget()
            };
            let size_of = |tester: &WidgetTester, key: &GlobalKey| {
                tester
                    .render_object(key)
                    .as_box()
                    .expect("a box")
                    .size(&tester.cell.borrow())
            };

            let frame1 = frame(tester, None);
            tester.pump_widget(frame1);
            assert_eq!(size_of(tester, &button_key), Size::new(64.0, 48.0));
            assert_eq!(size_of(tester, &label_key), Size::new(42.0, 14.0));

            // textScaleFactor expands text, but not button.
            let frame2 = frame(tester, Some(1.25));
            tester.pump_widget(frame2);
            assert_eq!(size_of(tester, &button_key), Size::new(68.5, 48.0));
            assert_eq!(size_of(tester, &label_key), Size::new(52.5, 18.0));

            // Set text scale large enough to expand text and button.
            let frame3 = frame(tester, Some(3.0));
            tester.pump_widget(frame3);
            assert_eq!(size_of(tester, &button_key), Size::new(134.0, 48.0));
            assert_eq!(size_of(tester, &label_key), Size::new(126.0, 42.0));
        });
    }

    #[test]
    fn the_theme_tap_target_size_pads_the_button_or_shrink_wraps_it() {
        test_widgets(|tester| {
            let frame = |tester: &WidgetTester, tap_target_size, key: &GlobalKey| {
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .material_tap_target_size(tap_target_size)
                    .build(&tester.cell.borrow());
                Theme::new(
                    theme,
                    ltr(Center::new().child(
                        TextButton::new(pressed(), Some(SizedBox::new().width(50.0).height(8.0)))
                            .key(key.clone().into())
                            .style(
                                TextButton::style_from()
                                    .minimum_size(Size::new(64.0, 36.0))
                                    .build(),
                            ),
                    )),
                )
                .into_widget()
            };
            let size_of = |tester: &WidgetTester, key: &GlobalKey| {
                tester
                    .render_object(key)
                    .as_box()
                    .expect("a box")
                    .size(&tester.cell.borrow())
            };

            let key1 = GlobalKey::new();
            let padded = frame(tester, MaterialTapTargetSize::Padded, &key1);
            tester.pump_widget(padded);
            assert_eq!(size_of(tester, &key1), Size::new(66.0, 48.0));

            let key2 = GlobalKey::new();
            let shrink_wrapped = frame(tester, MaterialTapTargetSize::ShrinkWrap, &key2);
            tester.pump_widget(shrink_wrapped);
            assert_eq!(size_of(tester, &key2), Size::new(66.0, 36.0));
        });
    }

    // 'TextButton responds to density changes.'; the `Text` child is the 56x14 box FlutterTest
    // lays 'Text' out in.
    #[test]
    fn visual_density_grows_and_shrinks_the_button_around_its_child() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let child_key = GlobalKey::new();
            let mut build_test = |visual_density: VisualDensity, use_text: bool| {
                let child = if use_text {
                    SizedBox::new()
                        .width(56.0)
                        .height(14.0)
                        .key(child_key.clone().into())
                } else {
                    SizedBox::new()
                        .width(100.0)
                        .height(100.0)
                        .key(child_key.clone().into())
                };
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Directionality::new(
                        TextDirection::Rtl,
                        Center::new().child(
                            TextButton::new(pressed(), Some(child))
                                .style(ButtonStyle::new().visual_density(visual_density))
                                .key(key.clone().into()),
                        ),
                    )
                    .into_widget(),
                ));
                tester.pump_and_settle();
                let size = tester
                    .render_object(&key)
                    .as_box()
                    .expect("a box")
                    .size(&tester.cell.borrow());
                let child_rect =
                    tester.global_rect(tester.render_object(&child_key).as_box().expect("a box"));
                (size, child_rect)
            };

            let cases = [
                (
                    VisualDensity::STANDARD,
                    false,
                    Size::new(116.0, 116.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::new(3.0, 3.0),
                    false,
                    Size::new(140.0, 140.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::new(-3.0, -3.0),
                    false,
                    Size::new(116.0, 100.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::STANDARD,
                    true,
                    Size::new(72.0, 48.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
                (
                    VisualDensity::new(3.0, 3.0),
                    true,
                    Size::new(96.0, 60.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
                (
                    VisualDensity::new(-3.0, -3.0),
                    true,
                    Size::new(72.0, 36.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
            ];
            for (visual_density, use_text, size, child_rect) in cases {
                assert_eq!(build_test(visual_density, use_text), (size, child_rect));
            }
        });
    }

    // 'Default TextButton padding for textScaleFactor, textDirection': each text scale,
    // direction and icon, in Material 2.
    #[test]
    fn the_default_padding_follows_the_text_scale_in_both_directions() {
        test_widgets(|tester| {
            const TEXT_SCALE_FACTOR_OPTIONS: [f64; 8] = [0.5, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0];
            // Expected values for each textScaleFactor.
            let padding_vertical = |scale: f64| match scale {
                0.5 | 1.0 => 8.0,
                1.25 => 6.0,
                1.5 => 4.0,
                _ => 0.0,
            };
            let padding_with_icon_gap = |scale: f64| match scale {
                0.5 | 1.0 => 8.0,
                1.25 => 7.0,
                1.5 => 6.0,
                _ => 4.0,
            };
            let text_padding_without_icon_horizontal = |scale: f64| match scale {
                0.5 | 1.0 | 1.25 | 1.5 | 2.0 => 8.0,
                2.5 => 6.0,
                _ => 4.0,
            };
            let text_padding_with_icon_horizontal = |scale: f64| match scale {
                0.5 | 1.0 => 8.0,
                1.25 => 7.0,
                1.5 => 6.0,
                _ => 4.0,
            };

            for text_scale_factor in TEXT_SCALE_FACTOR_OPTIONS {
                for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    for with_icon in [false, true] {
                        let button_key = GlobalKey::new();
                        let label_key = GlobalKey::new();
                        let icon_key = GlobalKey::new();
                        let theme = ThemeData::builder()
                            .use_material3(false)
                            .color_scheme(ColorScheme::light())
                            .text_button_theme(
                                TextButtonThemeData::new().style(
                                    TextButton::style_from()
                                        .minimum_size(Size::new(64.0, 36.0))
                                        .build(),
                                ),
                            )
                            .build(&tester.cell.borrow());
                        let label = Text::new("button").key(label_key.clone().into());
                        let button = if with_icon {
                            TextButton::icon(pressed(), label)
                                .key(button_key.clone().into())
                                .icon(
                                    Icon::new(Some(Icons::add()))
                                        .size(18.0)
                                        .key(icon_key.clone().into()),
                                )
                                .build()
                        } else {
                            TextButton::new(pressed(), Some(label)).key(button_key.clone().into())
                        };
                        tester.pump_widget(material_app(
                            theme,
                            MediaQuery::with_clamped_text_scaling(
                                None,
                                text_scale_factor,
                                text_scale_factor,
                                Directionality::new(
                                    text_direction,
                                    crate::material::Material::new()
                                        .child(Center::new().child(button)),
                                ),
                            ),
                        ));

                        // Compute expected padding, and check.
                        let expected_padding_start = if with_icon {
                            text_padding_with_icon_horizontal(text_scale_factor)
                        } else {
                            text_padding_without_icon_horizontal(text_scale_factor)
                        };
                        let expected_padding = EdgeInsetsGeometry::from_steb(
                            expected_padding_start,
                            padding_vertical(text_scale_factor),
                            expected_padding_start,
                            padding_vertical(text_scale_factor),
                        )
                        .resolve(Some(text_direction));
                        assert_eq!(
                            padding_of_button(tester).resolve(Some(text_direction)),
                            expected_padding,
                            "scale {text_scale_factor}, {text_direction:?}, icon {with_icon}"
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
                        let ink_well =
                            *tester.find_by_type::<InkWell>().last().expect("an InkWell");
                        let button_bounds = tester.global_rect(tester.render_box_of(ink_well));

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
                            assert_eq!(gap_width, padding_with_icon_gap(text_scale_factor));
                        }
                    }
                }
            }
        });
    }

    #[test]
    fn a_style_padding_overrides_the_scaled_default() {
        test_widgets(|tester| {
            let theme = light(tester, true);
            tester.pump_widget(material_app(
                theme,
                MediaQuery::with_clamped_text_scaling(
                    None,
                    2.0,
                    2.0,
                    Center::new().child(
                        TextButton::new(pressed(), Some(Text::new("TextButton"))).style(
                            TextButton::style_from()
                                .padding(EdgeInsetsGeometry::all(22.0))
                                .build(),
                        ),
                    ),
                ),
            ));
            assert!(padding_of_button(tester) == EdgeInsetsGeometry::all(22.0));
        });
    }

    #[test]
    fn a_larger_theme_label_font_size_scales_the_padding_down() {
        test_widgets(|tester| {
            let theme = ThemeData::from(ColorScheme::light())
                .text_theme(TextTheme::new().label_large(TextStyle::new().font_size(28.0)))
                .build(&tester.cell.borrow());
            tester.pump_widget(app_centered(
                theme,
                TextButton::new(pressed(), Some(Text::new("text"))),
            ));
            assert!(padding_of_button(tester) == EdgeInsetsGeometry::symmetric(0.0, 8.0));
        });
    }

    #[test]
    fn material_3_padding_of_the_button_and_its_icon_variant() {
        test_widgets(|tester| {
            let theme = light(tester, true);
            tester.pump_widget(app_centered(
                theme.clone(),
                TextButton::new(pressed(), Some(Text::new("TextButton"))),
            ));
            assert!(padding_of_button(tester) == EdgeInsetsGeometry::symmetric(8.0, 12.0));

            tester.pump_widget(app_centered(
                theme,
                TextButton::icon(pressed(), Text::new("TextButton"))
                    .icon(Icon::new(Some(Icons::add()))),
            ));
            assert!(
                padding_of_button(tester) == EdgeInsetsGeometry::from_steb(12.0, 8.0, 16.0, 8.0)
            );
        });
    }

    #[test]
    fn no_splash_paints_no_splash_and_ink_ripple_one() {
        test_widgets(|tester| {
            for (splash_factory, circles) in [
                (NoSplash::splash_factory(), 0),
                (InkRipple::splash_factory(), 1),
            ] {
                let theme = default_theme(tester);
                tester.pump_widget(app_centered(
                    theme,
                    TextButton::new(pressed(), Some(Text::new("test"))).style(
                        TextButton::style_from()
                            .splash_factory(splash_factory)
                            .build(),
                    ),
                ));
                let mut touch = TestPointer::touch(1);
                tester.down(&mut touch, button_center(tester));
                // Valo's canvas drops a draw whose paint is transparent, and the splash fades
                // in from transparent on the frame after the press: let that frame pass first.
                tester.pump();
                tester.pump_by(Duration::from_millis(200));
                assert_eq!(ink_circle_count(tester), circles);
                tester.up(&touch);
                tester.pump_and_settle();
            }
        });
    }

    // 'TextButton uses InkSparkle only for Android non-web when useMaterial3 is true' and
    // 'TextButton uses InkRipple when useMaterial3 is false', on every platform.
    #[test]
    fn the_splash_factory_is_ink_sparkle_on_android_in_material_3_and_ink_ripple_in_material_2() {
        test_widgets(|_| {
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::IOS,
                TargetPlatform::Linux,
                TargetPlatform::MacOS,
                TargetPlatform::Windows,
            ] {
                for material3 in [true, false] {
                    test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                        let theme = ThemeData::builder()
                            .use_material3(material3)
                            .build(&tester.cell.borrow());
                        let sparkles =
                            material3 && platform == TargetPlatform::Android && !K_IS_WEB;
                        let expected = if sparkles {
                            InkSparkle::splash_factory()
                        } else {
                            InkRipple::splash_factory()
                        };
                        tester.pump_widget(app_centered(
                            theme,
                            TextButton::new(pressed(), Some(Text::new("button"))),
                        ));
                        let splash_factory = ink_well(tester, |ink_well| {
                            ink_well.ink_response.splash_factory.clone()
                        })
                        .expect("the button gives its InkWell a splash factory");
                        assert!(Rc::ptr_eq(&splash_factory, &expected), "{platform:?}");
                    });
                }
            }
        });
    }

    #[test]
    fn an_icon_button_with_a_long_label_in_a_narrow_box_lays_out() {
        // Regression test for https://github.com/flutter/flutter/issues/77815
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(material_app(
                theme,
                SizedBox::new()
                    .width(200.0)
                    .child(
                        TextButton::icon(
                            pressed(),
                            // Much wider than 200
                            Text::new(
                                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Ut a \
                                 euismod nibh. Morbi laoreet purus.",
                            ),
                        )
                        .icon(Icon::new(Some(Icons::add()))),
                    )
                    .into_widget(),
            ));
        });
    }

    #[test]
    fn the_icon_and_label_sit_in_a_row_with_an_eight_pixel_gap() {
        test_widgets(|tester| {
            let button_key = GlobalKey::new();
            let icon_key = GlobalKey::new();
            let label_key = GlobalKey::new();
            let style = TextButton::style_from()
                .padding(EdgeInsetsGeometry::ZERO)
                .visual_density(VisualDensity::STANDARD) // dx=0, dy=0
                .build();
            let theme = default_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(200.0).child(
                            TextButton::icon(
                                pressed(),
                                SizedBox::new()
                                    .key(label_key.clone().into())
                                    .width(50.0)
                                    .height(100.0),
                            )
                            .key(button_key.clone().into())
                            .style(style)
                            .icon(
                                SizedBox::new()
                                    .key(icon_key.clone().into())
                                    .width(50.0)
                                    .height(100.0),
                            ),
                        ),
                    )
                    .into_widget(),
            ));

            // The button's label and icon are separated by a gap of 8:
            // 46 [icon 50] 8 [label 50] 46
            // The overall button width is 200. So:
            // icon.x = 46
            // label.x = 46 + 50 + 8 = 104
            let rect_of = |key: &GlobalKey| {
                tester.global_rect(tester.render_object(key).as_box().expect("a box"))
            };
            assert_eq!(
                rect_of(&button_key),
                Rect::from_ltrb(0.0, 0.0, 200.0, 100.0)
            );
            assert_eq!(rect_of(&icon_key), Rect::from_ltrb(46.0, 0.0, 96.0, 100.0));
            assert_eq!(
                rect_of(&label_key),
                Rect::from_ltrb(104.0, 0.0, 154.0, 100.0)
            );
        });
    }

    #[test]
    fn style_from_cursors_show_while_hovered_and_the_default_is_basic() {
        test_widgets(|tester| {
            let frame = |on_pressed: Option<Listener>, custom: bool| {
                let mut button = TextButton::new(on_pressed, Some(Text::new("button")));
                if custom {
                    button = button.style(
                        TextButton::style_from()
                            .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
                            .disabled_mouse_cursor(SystemMouseCursors::GRAB.into())
                            .build(),
                    );
                }
                ltr(MouseRegion::new()
                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                    .child(button))
            };
            let is = |tester: &WidgetTester, cursor: waft_services::SystemMouseCursor| {
                tester.active_cursor(1) == MouseCursorRef::from(cursor)
            };

            tester.pump_widget(frame(pressed(), true));
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump();
            assert!(is(tester, SystemMouseCursors::TEXT));

            // Test cursor when disabled
            tester.pump_widget(frame(None, true));
            assert!(is(tester, SystemMouseCursors::GRAB));

            // Test default cursor
            tester.pump_widget(frame(pressed(), false));
            assert!(is(
                tester,
                if K_IS_WEB {
                    SystemMouseCursors::CLICK
                } else {
                    SystemMouseCursors::BASIC
                }
            ));

            // Test default cursor when disabled
            tester.pump_widget(frame(None, false));
            assert!(is(tester, SystemMouseCursors::BASIC));
        });
    }

    #[test]
    fn style_from_sets_the_foreground_and_background_colors() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let purple = Colors::PURPLE.primary();
            tester.pump_widget(material_app(
                theme,
                TextButton::new(pressed(), Some(Text::new("button")))
                    .style(
                        TextButton::style_from()
                            .foreground_color(Colors::WHITE)
                            .background_color(purple)
                            .build(),
                    )
                    .into_widget(),
            ));
            material(tester, |material| {
                assert_eq!(material.color, Some(purple));
                assert_eq!(
                    material
                        .text_style
                        .as_ref()
                        .and_then(|style| style.color.as_deref().copied()),
                    Some(Colors::WHITE)
                );
            });
        });
    }

    // 'TextButton.icon statesController'; the plain button's case is button_style_button.rs's.
    #[test]
    fn an_icon_buttons_states_controller_follows_hover_press_and_disable() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.add_listener(&mut tester.cell.borrow_mut(), {
                let count = Rc::clone(&count);
                Listener::new(move |_app| count.set(count.get() + 1))
            });
            let frame = |tester: &WidgetTester, enabled: bool| {
                let theme = default_theme(tester);
                app_centered(
                    theme,
                    TextButton::icon(if enabled { pressed() } else { None }, Text::new("button"))
                        .states_controller(controller.clone())
                        .icon(Icon::new(Some(Icons::add()))),
                )
            };
            let value =
                |tester: &WidgetTester| tester.cell.borrow().get(&controller).value().clone();

            let first = frame(tester, true);
            tester.pump_widget(first);
            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 0);

            let label = *tester.find_text("button").first().expect("the label");
            let center = tester.global_rect(tester.render_box_of(label)).center();
            let mut gesture = mouse();
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 1);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 2);

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 3);

            tester.send(&gesture, PointerChange::Down);
            tester.pump_and_settle();
            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 4);

            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 5);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 6);

            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 8); // adds hovered and pressed - two changes

            // If the button is rebuilt disabled, then the pressed state is
            // removed.
            let disabled = frame(tester, false);
            tester.pump_widget(disabled);
            tester.pump_and_settle();
            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Disabled])
            );
            assert_eq!(count.get(), 10); // removes pressed and adds disabled - two changes
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[WidgetState::Disabled]));
            assert_eq!(count.get(), 11);
        });
    }

    #[test]
    fn style_from_with_only_an_enabled_cursor_builds() {
        // This is a regression test for https://github.com/flutter/flutter/issues/118071.
        test_widgets(|tester| {
            tester.pump_widget(ltr(TextButton::new(pressed(), Some(Text::new("button")))
                .style(
                    TextButton::style_from()
                        .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
                        .build(),
                )));
        });
    }

    #[test]
    fn style_from_a_lone_background_color_applies_when_disabled_too() {
        // Regression test for an internal Google issue: b/323399158
        test_widgets(|tester| {
            let background_color = Color::new(0xFF000022);
            let frame = |on_pressed: Option<Listener>| {
                ltr(
                    TextButton::new(on_pressed, Some(Text::new("button"))).style(
                        TextButton::style_from()
                            .background_color(background_color)
                            .build(),
                    ),
                )
            };

            tester.pump_widget(frame(pressed())); // enabled
            material(tester, |material| {
                assert_eq!(material.color, Some(background_color))
            });

            tester.pump_widget(frame(None)); // onPressed: null - disabled
            material(tester, |material| {
                assert_eq!(material.color, Some(background_color))
            });
        });
    }

    /// `MaterialApp(home: Directionality(textDirection, Center(button)))`.
    fn icon_alignment_frame(
        tester: &WidgetTester,
        text_direction: TextDirection,
        button: TextButtonIcon,
    ) -> WidgetRef {
        let theme = default_theme(tester);
        material_app(
            theme,
            Directionality::new(
                text_direction,
                Center::new().child(button.icon(Icon::new(Some(Icons::add())))),
            )
            .into_widget(),
        )
    }

    #[test]
    fn the_icon_sits_at_the_start_by_default_in_both_directions() {
        test_widgets(|tester| {
            // Test default iconAlignment when textDirection is ltr.
            let frame = icon_alignment_frame(
                tester,
                TextDirection::Ltr,
                TextButton::icon(pressed(), Text::new("button")),
            );
            tester.pump_widget(frame);
            // The icon is aligned to the left of the button.
            // 12.0 - padding between icon and button edge.
            assert_eq!(button_rect(tester).left, icon_rect(tester).left - 12.0);

            // Test default iconAlignment when textDirection is rtl.
            let frame = icon_alignment_frame(
                tester,
                TextDirection::Rtl,
                TextButton::icon(pressed(), Text::new("button")),
            );
            tester.pump_widget(frame);
            // The icon is aligned to the right of the button.
            // 12.0 - padding between icon and button edge.
            assert_near(button_rect(tester).right, icon_rect(tester).right + 12.0);
        });
    }

    #[test]
    fn icon_alignment_places_the_icon_at_either_end_in_both_directions() {
        test_widgets(|tester| {
            let mut case = |text_direction, icon_alignment| {
                let frame = icon_alignment_frame(
                    tester,
                    text_direction,
                    TextButton::icon(pressed(), Text::new("button")).icon_alignment(icon_alignment),
                );
                tester.pump_widget(frame);
                (button_rect(tester), icon_rect(tester))
            };

            // The icon is aligned to the left of the button.
            let (button, icon) = case(TextDirection::Ltr, IconAlignment::Start);
            assert_eq!(button.left, icon.left - 12.0);

            // The icon is aligned to the right of the button.
            let (button, icon) = case(TextDirection::Ltr, IconAlignment::End);
            assert_near(button.right, icon.right + 16.0);

            // The icon is aligned to the right of the button.
            let (button, icon) = case(TextDirection::Rtl, IconAlignment::Start);
            assert_near(button.right, icon.right + 12.0);

            // The icon is aligned to the left of the button.
            let (button, icon) = case(TextDirection::Rtl, IconAlignment::End);
            assert_eq!(button.left, icon.left - 16.0);
        });
    }

    #[test]
    fn the_icon_follows_the_style_icon_alignment() {
        test_widgets(|tester| {
            let mut case = |style: ButtonStyle| {
                let theme = default_theme(tester);
                tester.pump_widget(app_centered(
                    theme,
                    TextButton::icon(pressed(), Text::new("button"))
                        .style(style)
                        .icon(Icon::new(Some(Icons::add()))),
                ));
                (button_rect(tester), icon_rect(tester))
            };

            let (button, icon) = case(ButtonStyle::new());
            assert_eq!(button.left, icon.left - 12.0);

            let (button, icon) = case(ButtonStyle::new().icon_alignment(IconAlignment::End));
            assert_near(button.right, icon.right + 16.0);
        });
    }

    #[test]
    fn style_from_sets_the_icon_size_colors_and_alignment() {
        // Regression test for https://github.com/flutter/flutter/issues/154798.
        test_widgets(|tester| {
            let icon_color = Color::new(0xFFF000FF);
            let icon_size = 32.0;
            let disabled_icon_color = Color::new(0xFFFFF000);
            let frame = |tester: &WidgetTester, enabled: bool| {
                let theme = default_theme(tester);
                material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(
                                TextButton::icon(
                                    if enabled { pressed() } else { None },
                                    Text::new("Button"),
                                )
                                .style(
                                    TextButton::style_from()
                                        .icon_color(icon_color)
                                        .icon_size(icon_size)
                                        .icon_alignment(IconAlignment::End)
                                        .disabled_icon_color(disabled_icon_color)
                                        .build(),
                                )
                                .icon(Icon::new(Some(Icons::add()))),
                            ),
                        )
                        .into_widget(),
                )
            };

            // Test enabled button.
            let enabled = frame(tester, true);
            tester.pump_widget(enabled);
            assert_eq!(icon_rect(tester).size(), Size::new(icon_size, icon_size));
            assert_eq!(
                tester.icon_style().color.as_deref().copied(),
                Some(icon_color)
            );

            // Test disabled button.
            let disabled = frame(tester, false);
            tester.pump_widget(disabled);
            tester.pump_and_settle();
            assert_eq!(
                tester.icon_style().color.as_deref().copied(),
                Some(disabled_icon_color)
            );

            assert_near(button_rect(tester).right, icon_rect(tester).right + 16.0);
        });
    }

    #[test]
    fn the_icon_takes_the_style_from_foreground_color_over_the_default_icon_color() {
        // Regression test for https://github.com/flutter/flutter/issues/162839.
        test_widgets(|tester| {
            let foreground_color = Color::new(0xFFFF1234);
            let theme = default_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Center::new().child(
                            TextButton::icon(pressed(), Text::new("Button"))
                                .style(
                                    TextButton::style_from()
                                        .foreground_color(foreground_color)
                                        .build(),
                                )
                                .icon(Icon::new(Some(Icons::add()))),
                        ),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                tester.icon_style().color.as_deref().copied(),
                Some(foreground_color)
            );
        });
    }

    #[test]
    fn an_icon_button_keeps_its_focus_when_its_icon_is_removed() {
        test_widgets(|tester| {
            let frame = |tester: &WidgetTester, icon: bool| {
                let theme = default_theme(tester);
                let mut button = TextButton::icon(pressed(), Text::new("button"));
                if icon {
                    button = button.icon(Icon::new(Some(Icons::abc())));
                }
                app_centered(theme, button)
            };
            let button_focus_node = |tester: &WidgetTester| {
                let label = *tester.find_text("button").first().expect("the label");
                let mut app = tester.cell.borrow_mut();
                waft_widgets::Focus::of(&mut app, label, false, true)
            };

            // Build once with an icon.
            let with_icon = frame(tester, true);
            tester.pump_widget(with_icon);
            let node = button_focus_node(tester);
            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(button_focus_node(tester).has_focus(&mut tester.cell.borrow_mut()));

            // Rebuild without icon.
            let without_icon = frame(tester, false);
            tester.pump_widget(without_icon);

            // The button should still be focused.
            assert!(button_focus_node(tester).has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    #[test]
    fn the_icon_color_can_differ_from_the_text_color() {
        test_widgets(|tester| {
            let color_scheme = ColorScheme::light();
            let theme = light(tester, true);
            let red = Colors::RED.primary();
            let blue = Colors::BLUE.primary();
            tester.pump_widget(app_centered(
                theme.clone(),
                TextButton::icon(pressed(), Text::new("button"))
                    .style(TextButton::style_from().icon_color(red).build())
                    .icon(Icon::new(Some(Icons::add()))),
            ));
            material(tester, |material| {
                assert_eq!(
                    material
                        .text_style
                        .as_ref()
                        .and_then(|style| style.color.as_deref().copied()),
                    Some(color_scheme.primary)
                );
            });
            assert_eq!(tester.icon_style().color.as_deref().copied(), Some(red));

            // disabled button
            tester.pump_widget(app_centered(
                theme,
                TextButton::icon(None, Text::new("button"))
                    .style(
                        TextButton::style_from()
                            .icon_color(red)
                            .disabled_icon_color(blue)
                            .build(),
                    )
                    .icon(Icon::new(Some(Icons::add()))),
            ));
            tester.pump_and_settle();
            material(tester, |material| {
                assert_eq!(
                    material
                        .text_style
                        .as_ref()
                        .and_then(|style| style.color.as_deref().copied()),
                    Some(color_scheme.on_surface.with_opacity(0.38))
                );
            });
            assert_eq!(tester.icon_style().color.as_deref().copied(), Some(blue));
        });
    }

    #[test]
    fn a_hovering_stylus_hovers_like_a_mouse() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            let theme = default_theme(tester);
            let primary = theme.color_scheme.primary;
            let has_been_hovered = Rc::new(Cell::new(false));
            let on_hover: waft_foundation::ValueChanged<bool> = {
                let has_been_hovered = Rc::clone(&has_been_hovered);
                Rc::new(move |_app, _entered| has_been_hovered.set(true))
            };
            tester.pump_widget(app_centered(
                theme,
                TextButton::new(pressed(), Some(Text::new("TextButton")))
                    .on_hover(on_hover)
                    .focus_node(focus_node),
            ));

            let center = button_center(tester);
            let mut gesture = TestPointer {
                kind: waft_embedder::PointerDeviceKind::Stylus,
                ..TestPointer::mouse(1, 0)
            };
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump_and_settle();
            assert!(!has_been_hovered.get());

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(primary.with_opacity(0.08))));
            assert!(has_been_hovered.get());
        });
    }
}
