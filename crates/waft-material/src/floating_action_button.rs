//! Flutter counterpart: material_ui's `floating_action_button.dart`.
//!
//! The `scaffold.dart` that Dart's file imports is for its documentation only.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Clip, Color, Offset, Size, TextBaseline};
use waft_foundation::{AnyEq, App, Listener};
use waft_painting::{
    AlignmentGeometry, CircleBorder, EdgeInsetsDirectional, EdgeInsetsGeometry, ShapeBorderRef,
    StadiumBorder, TextDirection, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, PaintingContext, RenderBox,
    RenderBoxData, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderShiftedBox,
};
use waft_rendering::{RenderAligningShiftedBox, RenderAligningShiftedBoxData};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Directionality, FocusNodeRef, Hero, IconTheme, IconThemeData, IntoWidget, KeyRef,
    MainAxisSize, MergeSemantics, Padding, RenderObjectWidget, Row, SingleChildRenderObjectWidget,
    SizedBox, StatelessWidget, WidgetRef, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::button::RawMaterialButton;
use crate::floating_action_button_theme::{
    FloatingActionButtonTheme, FloatingActionButtonThemeData, FloatingActionButtonThemeDataGetters,
};
use crate::generated::fab_defaults_m3::FabDefaultsM3;
use crate::theme::Theme;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data_getters::DefaultsContext;
use crate::tooltip::Tooltip;

/// Dart's `_DefaultHeroTag`.
///
/// Dart only ever makes it as `const _DefaultHeroTag()`, one canonical instance, so the
/// derived equality, which makes every value equal, is Dart's identity.
#[derive(PartialEq, Eq, Hash)]
struct DefaultHeroTag;

impl Debug for DefaultHeroTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<default FloatingActionButton tag>")
    }
}

/// Dart's `const _DefaultHeroTag()`: one shared tag, so every floating action button that
/// keeps the default matches every other.
fn default_hero_tag() -> Rc<dyn AnyEq> {
    thread_local! {
        static TAG: Rc<dyn AnyEq> = Rc::new(DefaultHeroTag);
    }
    TAG.with(Rc::clone)
}

/// Dart's `_FloatingActionButtonType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatingActionButtonType {
    Regular,
    Small,
    Large,
    Extended,
}

/// A Material Design floating action button.
///
/// A floating action button is a circular icon button that hovers over content
/// to promote a primary action in the application. Floating action buttons are
/// most commonly used in the `Scaffold::floating_action_button` field.
///
/// Use at most a single floating action button per screen. Floating action
/// buttons should be used for positive actions such as "create", "share", or
/// "navigate". (If more than one floating action button is used within a
/// `Route`, then make sure that each button has a unique [`hero_tag`](Self::hero_tag),
/// otherwise an exception will be thrown.)
///
/// If the [`on_pressed`](Self::on_pressed) callback is null, then the button will be disabled
/// and will not react to touch. It is highly discouraged to disable a floating
/// action button as there is no indication to the user that the button is
/// disabled. Consider changing the [`background_color`](Self::background_color) if disabling
/// the floating action button.
///
/// See also:
///
///  * `Scaffold`, in which floating action buttons typically live.
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * <https://material.io/design/components/buttons-floating-action-button.html>
///  * <https://m3.material.io/components/floating-action-button>
pub struct FloatingActionButton {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// Typically an `Icon`.
    pub child: Option<WidgetRef>,

    /// Text that describes the action that will occur when the button is pressed.
    ///
    /// This text is displayed when the user long-presses on the button and is
    /// used for accessibility.
    pub tooltip: Option<String>,

    /// The default foreground color for icons and text within the button.
    ///
    /// If this property is null, then the ambient
    /// [`FloatingActionButtonThemeData::foreground_color`] is used. If that property is also
    /// null, then the `ColorScheme::on_primary_container` color of `ThemeData::color_scheme`
    /// is used. If `ThemeData::use_material3` is set to false, then the
    /// `ColorScheme::on_secondary` color of `ThemeData::color_scheme` is used.
    pub foreground_color: Option<Color>,

    /// The button's background color.
    ///
    /// If this property is null, then the ambient
    /// [`FloatingActionButtonThemeData::background_color`] is used. If that property is also
    /// null, then the `ColorScheme::primary_container` color of `ThemeData::color_scheme`
    /// is used. If `ThemeData::use_material3` is set to false, then the
    /// `ColorScheme::secondary` color of `ThemeData::color_scheme` is used.
    pub background_color: Option<Color>,

    /// The color to use for filling the button when the button has input focus.
    ///
    /// In Material3, defaults to `ColorScheme::on_primary_container` with opacity 0.1.
    /// In Material 2, it defaults to `ThemeData::focus_color` for the current theme.
    pub focus_color: Option<Color>,

    /// The color to use for filling the button when the button has a pointer
    /// hovering over it.
    ///
    /// Defaults to `ThemeData::hover_color` for the current theme in Material 2. In
    /// Material 3, defaults to `ColorScheme::on_primary_container` with opacity 0.08.
    pub hover_color: Option<Color>,

    /// The splash color for this [`FloatingActionButton`]'s `InkWell`.
    ///
    /// If null, [`FloatingActionButtonThemeData::splash_color`] is used, if that is
    /// null, `ThemeData::splash_color` is used in Material 2;
    /// `ColorScheme::on_primary_container` with opacity 0.1 is used in Material 3.
    pub splash_color: Option<Color>,

    /// The tag to apply to the button's [`Hero`] widget.
    ///
    /// Defaults to a tag that matches other floating action buttons.
    ///
    /// Set this to null explicitly if you don't want the floating action button to
    /// have a hero tag.
    ///
    /// If this is not explicitly set, then there can only be one
    /// [`FloatingActionButton`] per route (that is, per screen), since otherwise
    /// there would be a tag conflict (multiple heroes on one route can't have the
    /// same tag). The Material Design specification recommends only using one
    /// floating action button per screen.
    pub hero_tag: Option<Rc<dyn AnyEq>>,

    /// The callback that is called when the button is tapped or otherwise activated.
    ///
    /// If this is set to null, the button will be disabled.
    pub on_pressed: Option<Listener>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// button.
    ///
    /// If this property is null, [`FloatingActionButtonThemeData::mouse_cursor`] is used.
    /// If that is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// The z-coordinate at which to place this button relative to its parent.
    ///
    /// This controls the size of the shadow below the floating action button.
    ///
    /// Defaults to 6, the appropriate elevation for floating action buttons. The
    /// value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    pub elevation: Option<f64>,

    /// The z-coordinate at which to place this button relative to its parent when
    /// the button has the input focus.
    ///
    /// This controls the size of the shadow below the floating action button.
    ///
    /// Defaults to 8, the appropriate elevation for floating action buttons
    /// while they have focus. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    pub focus_elevation: Option<f64>,

    /// The z-coordinate at which to place this button relative to its parent when
    /// the button is enabled and has a pointer hovering over it.
    ///
    /// This controls the size of the shadow below the floating action button.
    ///
    /// Defaults to 8, the appropriate elevation for floating action buttons while
    /// they have a pointer hovering over them. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    pub hover_elevation: Option<f64>,

    /// The z-coordinate at which to place this button relative to its parent when
    /// the user is touching the button.
    ///
    /// This controls the size of the shadow below the floating action button.
    ///
    /// Defaults to 12, the appropriate elevation for floating action buttons
    /// while they are being touched. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    pub highlight_elevation: Option<f64>,

    /// The z-coordinate at which to place this button when the button is disabled
    /// ([`on_pressed`](Self::on_pressed) is null).
    ///
    /// This controls the size of the shadow below the floating action button.
    ///
    /// Defaults to the same value as [`elevation`](Self::elevation). Setting this to zero
    /// makes the floating action button work similar to an `ElevatedButton` but the titular
    /// "floating" effect is lost. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    pub disabled_elevation: Option<f64>,

    /// Controls the size of this button.
    ///
    /// By default, floating action buttons are non-mini and have a height and
    /// width of 56.0 logical pixels. Mini floating action buttons have a height
    /// and width of 40.0 logical pixels with a layout width and height of 48.0
    /// logical pixels. (The extra 4 pixels of padding on each side are added as a
    /// result of the floating action button having [`MaterialTapTargetSize::Padded`]
    /// set on the underlying [`RawMaterialButton::material_tap_target_size`].)
    pub mini: bool,

    /// The shape of the button's `Material`.
    ///
    /// The button's highlight and splash are clipped to this shape. If the
    /// button has an elevation, then its drop shadow is defined by this
    /// shape as well.
    pub shape: Option<ShapeBorderRef>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,

    /// True if this is an "extended" floating action button.
    ///
    /// Typically "extended" buttons have a [`StadiumBorder`] [`shape`](Self::shape)
    /// and have been created with the [`FloatingActionButton::extended`]
    /// constructor.
    ///
    /// The `Scaffold` animates the appearance of ordinary floating
    /// action buttons with scale and rotation transitions. Extended
    /// floating action buttons are scaled and faded in.
    pub is_extended: bool,

    /// An optional focus node to use as the focus node for this widget.
    ///
    /// See `Focus::focus_node`.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    ///
    /// See `Focus::autofocus`.
    pub autofocus: bool,

    /// Configures the minimum size of the tap target.
    ///
    /// Defaults to `ThemeData::material_tap_target_size`.
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// If null, [`FloatingActionButtonThemeData::enable_feedback`] is used.
    /// If both are null, then default value is true.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// The spacing between the icon and the label for an extended
    /// [`FloatingActionButton`].
    ///
    /// If null, [`FloatingActionButtonThemeData::extended_icon_label_spacing`] is used.
    /// If that is also null, the default is 8.0.
    pub extended_icon_label_spacing: Option<f64>,

    /// The padding for an extended [`FloatingActionButton`]'s content.
    ///
    /// If null, [`FloatingActionButtonThemeData::extended_padding`] is used. If that
    /// is also null, the default is
    /// `EdgeInsetsDirectional.only(start: 16.0, end: 20.0)` if an icon is
    /// provided, and `EdgeInsetsDirectional.only(start: 20.0, end: 20.0)` if not.
    pub extended_padding: Option<EdgeInsetsGeometry>,

    /// The text style for an extended [`FloatingActionButton`]'s label.
    ///
    /// If null, [`FloatingActionButtonThemeData::extended_text_style`] is used. If
    /// that is also null, then `TextTheme::label_large` with a letter spacing of 1.2
    /// is used.
    pub extended_text_style: Option<TextStyle>,

    floating_action_button_type: FloatingActionButtonType,

    extended_label: Option<WidgetRef>,

    /// Whether the unnamed constructor made this button, the only one that takes `mini`;
    /// `mini(true)` makes its type `Small` too, so the type cannot tell.
    unnamed_constructor: bool,
}

impl FloatingActionButton {
    /// Creates a circular floating action button.
    ///
    /// The [`elevation`](Self::elevation), [`highlight_elevation`](Self::highlight_elevation),
    /// and [`disabled_elevation`](Self::disabled_elevation) parameters, if specified, must be
    /// non-negative.
    pub fn new(on_pressed: Option<Listener>) -> FloatingActionButton {
        FloatingActionButton {
            key: None,
            child: None,
            tooltip: None,
            foreground_color: None,
            background_color: None,
            focus_color: None,
            hover_color: None,
            splash_color: None,
            hero_tag: Some(default_hero_tag()),
            on_pressed,
            mouse_cursor: None,
            elevation: None,
            focus_elevation: None,
            hover_elevation: None,
            highlight_elevation: None,
            disabled_elevation: None,
            mini: false,
            shape: None,
            clip_behavior: Clip::None,
            is_extended: false,
            focus_node: None,
            autofocus: false,
            material_tap_target_size: None,
            enable_feedback: None,
            extended_icon_label_spacing: None,
            extended_padding: None,
            extended_text_style: None,
            floating_action_button_type: FloatingActionButtonType::Regular,
            extended_label: None,
            unnamed_constructor: true,
        }
    }

    /// Creates a small circular floating action button.
    ///
    /// This constructor overrides the default size constraints of the floating
    /// action button.
    ///
    /// The [`elevation`](Self::elevation), [`focus_elevation`](Self::focus_elevation),
    /// [`hover_elevation`](Self::hover_elevation),
    /// [`highlight_elevation`](Self::highlight_elevation), and
    /// [`disabled_elevation`](Self::disabled_elevation) parameters, if specified, must be
    /// non-negative.
    pub fn small(on_pressed: Option<Listener>) -> FloatingActionButton {
        FloatingActionButton {
            floating_action_button_type: FloatingActionButtonType::Small,
            mini: true,
            unnamed_constructor: false,
            ..FloatingActionButton::new(on_pressed)
        }
    }

    /// Creates a large circular floating action button.
    ///
    /// This constructor overrides the default size constraints of the floating
    /// action button.
    ///
    /// The [`elevation`](Self::elevation), [`focus_elevation`](Self::focus_elevation),
    /// [`hover_elevation`](Self::hover_elevation),
    /// [`highlight_elevation`](Self::highlight_elevation), and
    /// [`disabled_elevation`](Self::disabled_elevation) parameters, if specified, must be
    /// non-negative.
    pub fn large(on_pressed: Option<Listener>) -> FloatingActionButton {
        FloatingActionButton {
            floating_action_button_type: FloatingActionButtonType::Large,
            mini: false,
            unnamed_constructor: false,
            ..FloatingActionButton::new(on_pressed)
        }
    }

    /// Creates a wider [`StadiumBorder`]-shaped floating action button with
    /// an optional [`icon`](Self::icon) and a `label`.
    ///
    /// The [`elevation`](Self::elevation), [`highlight_elevation`](Self::highlight_elevation),
    /// and [`disabled_elevation`](Self::disabled_elevation) parameters, if specified, must be
    /// non-negative.
    ///
    /// See also:
    ///  * <https://m3.material.io/components/extended-fab>
    pub fn extended<K>(
        on_pressed: Option<Listener>,
        label: impl IntoWidget<K>,
    ) -> FloatingActionButton {
        FloatingActionButton {
            is_extended: true,
            mini: false,
            floating_action_button_type: FloatingActionButtonType::Extended,
            extended_label: Some(label.into_widget()),
            unnamed_constructor: false,
            ..FloatingActionButton::new(on_pressed)
        }
    }

    /// Dart `FloatingActionButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> FloatingActionButton {
        self.key = Some(key);
        self
    }

    /// Dart `FloatingActionButton(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> FloatingActionButton {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `FloatingActionButton(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> FloatingActionButton {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `FloatingActionButton(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> FloatingActionButton {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `FloatingActionButton(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> FloatingActionButton {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `FloatingActionButton(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> FloatingActionButton {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `FloatingActionButton(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> FloatingActionButton {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `FloatingActionButton(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> FloatingActionButton {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `FloatingActionButton(heroTag:)`; `None` is Dart's explicit `heroTag: null`.
    pub fn hero_tag(mut self, hero_tag: Option<Rc<dyn AnyEq>>) -> FloatingActionButton {
        self.hero_tag = hero_tag;
        self
    }

    /// Dart `FloatingActionButton(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> FloatingActionButton {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `FloatingActionButton(focusElevation:)`.
    pub fn focus_elevation(mut self, focus_elevation: f64) -> FloatingActionButton {
        debug_assert!(focus_elevation >= 0.0);
        self.focus_elevation = Some(focus_elevation);
        self
    }

    /// Dart `FloatingActionButton(hoverElevation:)`.
    pub fn hover_elevation(mut self, hover_elevation: f64) -> FloatingActionButton {
        debug_assert!(hover_elevation >= 0.0);
        self.hover_elevation = Some(hover_elevation);
        self
    }

    /// Dart `FloatingActionButton(highlightElevation:)`.
    pub fn highlight_elevation(mut self, highlight_elevation: f64) -> FloatingActionButton {
        debug_assert!(highlight_elevation >= 0.0);
        self.highlight_elevation = Some(highlight_elevation);
        self
    }

    /// Dart `FloatingActionButton(disabledElevation:)`.
    pub fn disabled_elevation(mut self, disabled_elevation: f64) -> FloatingActionButton {
        debug_assert!(disabled_elevation >= 0.0);
        self.disabled_elevation = Some(disabled_elevation);
        self
    }

    /// Dart `FloatingActionButton(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> FloatingActionButton {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `FloatingActionButton(mini:)`, an argument of the unnamed constructor only, which
    /// picks the small type when it is true.
    pub fn mini(mut self, mini: bool) -> FloatingActionButton {
        debug_assert!(
            self.unnamed_constructor,
            "`mini` is an argument of the unnamed `FloatingActionButton` constructor only"
        );
        self.mini = mini;
        self.floating_action_button_type = if mini {
            FloatingActionButtonType::Small
        } else {
            FloatingActionButtonType::Regular
        };
        self
    }

    /// Dart `FloatingActionButton(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> FloatingActionButton {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `FloatingActionButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> FloatingActionButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `FloatingActionButton(isExtended:)` and `FloatingActionButton.extended(isExtended:)`.
    pub fn is_extended(mut self, is_extended: bool) -> FloatingActionButton {
        debug_assert!(
            self.unnamed_constructor
                || self.floating_action_button_type == FloatingActionButtonType::Extended,
            "`isExtended` is an argument of the unnamed and the `extended` `FloatingActionButton` \
             constructors only"
        );
        self.is_extended = is_extended;
        self
    }

    /// Dart `FloatingActionButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> FloatingActionButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `FloatingActionButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> FloatingActionButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `FloatingActionButton(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> FloatingActionButton {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `FloatingActionButton(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> FloatingActionButton {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `FloatingActionButton.extended(extendedIconLabelSpacing:)`.
    pub fn extended_icon_label_spacing(
        mut self,
        extended_icon_label_spacing: f64,
    ) -> FloatingActionButton {
        self.debug_assert_extended("extendedIconLabelSpacing");
        self.extended_icon_label_spacing = Some(extended_icon_label_spacing);
        self
    }

    /// Dart `FloatingActionButton.extended(extendedPadding:)`.
    pub fn extended_padding(
        mut self,
        extended_padding: impl Into<EdgeInsetsGeometry>,
    ) -> FloatingActionButton {
        self.debug_assert_extended("extendedPadding");
        self.extended_padding = Some(extended_padding.into());
        self
    }

    /// Dart `FloatingActionButton.extended(extendedTextStyle:)`.
    pub fn extended_text_style(mut self, extended_text_style: TextStyle) -> FloatingActionButton {
        self.debug_assert_extended("extendedTextStyle");
        self.extended_text_style = Some(extended_text_style);
        self
    }

    /// Dart `FloatingActionButton.extended(icon:)`, which the constructor stores as
    /// [`child`](Self::child).
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> FloatingActionButton {
        self.debug_assert_extended("icon");
        self.child = Some(icon.into_widget());
        self
    }

    /// Dart's other constructors set `argument` to null: only `extended` takes it.
    fn debug_assert_extended(&self, argument: &str) {
        debug_assert!(
            self.floating_action_button_type == FloatingActionButtonType::Extended,
            "`{argument}` is an argument of the `FloatingActionButton.extended` constructor only"
        );
    }

    /// The defaults of the theme in `context`: Material 3's, or Material 2's when
    /// `ThemeData::use_material3` is false.
    fn defaults(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn FloatingActionButtonThemeDataGetters> {
        let has_child = self.child.is_some();
        if Theme::of(app, context).use_material3 {
            Box::new(FabDefaultsM3::new(
                context,
                self.floating_action_button_type,
                has_child,
            ))
        } else {
            Box::new(FabDefaultsM2::new(
                app,
                context,
                self.floating_action_button_type,
                has_child,
            ))
        }
    }

    /// The extended button's `Row` children: Dart's `[?child, if (child != null && isExtended)
    /// SizedBox(width: iconLabelSpacing), if (isExtended) _extendedLabel!]`.
    fn extended_children(&self, icon_label_spacing: f64) -> Vec<WidgetRef> {
        let mut children = Vec::new();
        if let Some(child) = &self.child {
            children.push(child.clone());
        }
        if self.child.is_some() && self.is_extended {
            children.push(SizedBox::new().width(icon_label_spacing).into_widget());
        }
        if self.is_extended {
            children.push(
                self.extended_label
                    .clone()
                    .expect("FloatingActionButton.extended has a label"),
            );
        }
        children
    }
}

impl StatelessWidget for FloatingActionButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let floating_action_button_theme = FloatingActionButtonTheme::of(app, context);
        let defaults = self.defaults(app, context);

        let foreground_color = self
            .foreground_color
            .or(floating_action_button_theme.foreground_color)
            .or_else(|| defaults.foreground_color(app))
            .expect("the defaults give a foreground color");
        let background_color = self
            .background_color
            .or(floating_action_button_theme.background_color)
            .or_else(|| defaults.background_color(app))
            .expect("the defaults give a background color");
        let focus_color = self
            .focus_color
            .or(floating_action_button_theme.focus_color)
            .or_else(|| defaults.focus_color(app))
            .expect("the defaults give a focus color");
        let hover_color = self
            .hover_color
            .or(floating_action_button_theme.hover_color)
            .or_else(|| defaults.hover_color(app))
            .expect("the defaults give a hover color");
        let splash_color = self
            .splash_color
            .or(floating_action_button_theme.splash_color)
            .or_else(|| defaults.splash_color(app))
            .expect("the defaults give a splash color");
        let elevation = self
            .elevation
            .or(floating_action_button_theme.elevation)
            .or_else(|| defaults.elevation(app))
            .expect("the defaults give an elevation");
        let focus_elevation = self
            .focus_elevation
            .or(floating_action_button_theme.focus_elevation)
            .or_else(|| defaults.focus_elevation(app))
            .expect("the defaults give a focus elevation");
        let hover_elevation = self
            .hover_elevation
            .or(floating_action_button_theme.hover_elevation)
            .or_else(|| defaults.hover_elevation(app))
            .expect("the defaults give a hover elevation");
        let disabled_elevation = self
            .disabled_elevation
            .or(floating_action_button_theme.disabled_elevation)
            .or_else(|| defaults.disabled_elevation(app))
            .unwrap_or(elevation);
        let highlight_elevation = self
            .highlight_elevation
            .or(floating_action_button_theme.highlight_elevation)
            .or_else(|| defaults.highlight_elevation(app))
            .expect("the defaults give a highlight elevation");
        let material_tap_target_size = self
            .material_tap_target_size
            .unwrap_or(theme.material_tap_target_size);
        let enable_feedback = self
            .enable_feedback
            .or(floating_action_button_theme.enable_feedback)
            .or_else(|| defaults.enable_feedback(app))
            .expect("the defaults give enable feedback");
        let icon_size = floating_action_button_theme
            .icon_size
            .or_else(|| defaults.icon_size(app))
            .expect("the defaults give an icon size");
        let extended_text_style = self
            .extended_text_style
            .clone()
            .or_else(|| floating_action_button_theme.extended_text_style.clone())
            .or_else(|| defaults.extended_text_style(app))
            .expect("the defaults give an extended text style")
            .copy_with()
            .color(foreground_color);
        let shape = self
            .shape
            .clone()
            .or_else(|| floating_action_button_theme.shape.clone())
            .or_else(|| defaults.shape(app))
            .expect("the defaults give a shape");

        let size_constraints;
        let mut resolved_child = self.child.as_ref().map(|child| {
            IconTheme::merge(None, IconThemeData::new().size(icon_size), child.clone())
        });
        match self.floating_action_button_type {
            FloatingActionButtonType::Regular => {
                size_constraints = floating_action_button_theme
                    .size_constraints
                    .or_else(|| defaults.size_constraints(app))
                    .expect("the defaults give size constraints");
            }
            FloatingActionButtonType::Small => {
                size_constraints = floating_action_button_theme
                    .small_size_constraints
                    .or_else(|| defaults.small_size_constraints(app))
                    .expect("the defaults give small size constraints");
            }
            FloatingActionButtonType::Large => {
                size_constraints = floating_action_button_theme
                    .large_size_constraints
                    .or_else(|| defaults.large_size_constraints(app))
                    .expect("the defaults give large size constraints");
            }
            FloatingActionButtonType::Extended => {
                size_constraints = floating_action_button_theme
                    .extended_size_constraints
                    .or_else(|| defaults.extended_size_constraints(app))
                    .expect("the defaults give extended size constraints");
                let icon_label_spacing = self
                    .extended_icon_label_spacing
                    .or(floating_action_button_theme.extended_icon_label_spacing)
                    .unwrap_or(8.0);
                let padding = self
                    .extended_padding
                    .or(floating_action_button_theme.extended_padding)
                    .or_else(|| defaults.extended_padding(app))
                    .expect("the defaults give an extended padding");
                resolved_child = Some(
                    ChildOverflowBox {
                        child: Some(
                            Padding::new(padding)
                                .child(
                                    Row::new()
                                        .main_axis_size(MainAxisSize::Min)
                                        .children(self.extended_children(icon_label_spacing)),
                                )
                                .into_widget(),
                        ),
                    }
                    .into_widget(),
                );
            }
        }

        let mut button = RawMaterialButton::new(self.on_pressed.clone())
            .mouse_cursor(
                effective_mouse_cursor(
                    self.mouse_cursor.clone(),
                    floating_action_button_theme.mouse_cursor.clone(),
                )
                .into(),
            )
            .elevation(elevation)
            .focus_elevation(focus_elevation)
            .hover_elevation(hover_elevation)
            .highlight_elevation(highlight_elevation)
            .disabled_elevation(disabled_elevation)
            .constraints(size_constraints)
            .material_tap_target_size(material_tap_target_size)
            .fill_color(background_color)
            .focus_color(focus_color)
            .hover_color(hover_color)
            .splash_color(splash_color)
            .text_style(extended_text_style)
            .shape(shape)
            .clip_behavior(self.clip_behavior)
            .autofocus(self.autofocus)
            .enable_feedback(enable_feedback);
        if let Some(focus_node) = self.focus_node.clone() {
            button = button.focus_node(focus_node);
        }
        if let Some(resolved_child) = resolved_child {
            button = button.child(resolved_child);
        }
        let mut result = button.into_widget();

        if let Some(tooltip) = &self.tooltip {
            result = Tooltip::new()
                .message(tooltip.clone())
                .child(result)
                .into_widget();
        }

        if let Some(hero_tag) = &self.hero_tag {
            result = Hero::new(Rc::clone(hero_tag), result).into_widget();
        }

        MergeSemantics::new().child(result).into_widget()
    }
}

impl Debug for FloatingActionButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("FloatingActionButton");
        if self.on_pressed.is_none() {
            properties.field("on_pressed", &"disabled");
        }
        if let Some(tooltip) = &self.tooltip {
            properties.field("tooltip", tooltip);
        }
        if let Some(foreground_color) = &self.foreground_color {
            properties.field("foreground_color", foreground_color);
        }
        if let Some(background_color) = &self.background_color {
            properties.field("background_color", background_color);
        }
        if let Some(focus_color) = &self.focus_color {
            properties.field("focus_color", focus_color);
        }
        if let Some(hover_color) = &self.hover_color {
            properties.field("hover_color", hover_color);
        }
        if let Some(splash_color) = &self.splash_color {
            properties.field("splash_color", splash_color);
        }
        if self.hero_tag.is_some() {
            properties.field("hero_tag", &"hero");
        }
        if let Some(elevation) = &self.elevation {
            properties.field("elevation", elevation);
        }
        if let Some(focus_elevation) = &self.focus_elevation {
            properties.field("focus_elevation", focus_elevation);
        }
        if let Some(hover_elevation) = &self.hover_elevation {
            properties.field("hover_elevation", hover_elevation);
        }
        if let Some(highlight_elevation) = &self.highlight_elevation {
            properties.field("highlight_elevation", highlight_elevation);
        }
        if let Some(disabled_elevation) = &self.disabled_elevation {
            properties.field("disabled_elevation", disabled_elevation);
        }
        if let Some(shape) = &self.shape {
            properties.field("shape", shape);
        }
        if let Some(focus_node) = &self.focus_node {
            properties.field("focus_node", focus_node);
        }
        if self.is_extended {
            properties.field("is_extended", &"extended");
        }
        if let Some(material_tap_target_size) = &self.material_tap_target_size {
            properties.field("material_tap_target_size", material_tap_target_size);
        }
        properties.finish_non_exhaustive()
    }
}

/// Dart's `_EffectiveMouseCursor`: a [`WidgetStateMouseCursor`] that [`RawMaterialButton`]
/// resolves against `WidgetState::Pressed`, `WidgetState::Hovered`, `WidgetState::Focused`
/// and `WidgetState::Disabled`.
fn effective_mouse_cursor(
    widget_cursor: Option<MouseCursorRef>,
    theme_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
) -> WidgetStateMouseCursor {
    WidgetStateMouseCursor::resolve_with(
        move |states| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget_cursor, states)
                .or_else(|| {
                    theme_cursor
                        .as_ref()
                        .and_then(|theme_cursor| theme_cursor.resolve(states))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::adaptive_clickable().resolve(states))
        },
        Some("WidgetStateMouseCursor(FloatActionButton)"),
    )
}

/// This widget's size matches its child's size unless its constraints
/// force it to be larger or smaller. The child is centered.
///
/// Used to encapsulate extended FABs whose size is fixed, using `Row`
/// and `MainAxisSize::Min`, to be as wide as their label and icon.
///
/// Dart's `_ChildOverflowBox`.
#[derive(Debug)]
struct ChildOverflowBox {
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for ChildOverflowBox {
    type RenderObject = RenderChildOverflowBox;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderChildOverflowBox::new(app, Some(text_direction)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderChildOverflowBox>,
    ) {
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, Some(text_direction));
    }
}

impl SingleChildRenderObjectWidget for ChildOverflowBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderChildOverflowBox`.
struct RenderChildOverflowBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
}

impl RenderChildOverflowBox {
    fn new(
        app: &mut App,
        text_direction: Option<TextDirection>,
    ) -> RenderHandle<RenderChildOverflowBox> {
        RenderHandle::new_box(
            app,
            RenderChildOverflowBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(
                    AlignmentGeometry::CENTER,
                    text_direction,
                ),
            },
        )
    }
}

impl RenderObjectWithChildMixin for RenderChildOverflowBox {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderChildOverflowBox {}

impl RenderAligningShiftedBox for RenderChildOverflowBox {
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData {
        &self.get(app).aligning
    }

    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData {
        &mut self.get_mut(app).aligning
    }
}

impl RenderObject for RenderChildOverflowBox {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        if let Some(child) = self.child(app) {
            child.layout(app, BoxConstraints::new(), true);
            let child_size = child.size(app);
            self.set_size(
                app,
                Size::new(
                    constraints
                        .min_width
                        .max(constraints.max_width.min(child_size.width())),
                    constraints
                        .min_height
                        .max(constraints.max_height.min(child_size.height())),
                ),
            );
            self.align_child(app);
        } else {
            self.set_size(app, constraints.biggest());
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderChildOverflowBox {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if let Some(child) = self.child(app) {
            let child_size = child.get_dry_layout(app, BoxConstraints::new());
            Size::new(
                constraints
                    .min_width
                    .max(constraints.max_width.min(child_size.width())),
                constraints
                    .min_height
                    .max(constraints.max_height.min(child_size.height())),
            )
        } else {
            constraints.biggest()
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

/// Hand coded defaults based on Material Design 2.
///
/// Dart's `_FABDefaultsM2`.
struct FabDefaultsM2 {
    /// Dart's `super(..)` arguments.
    floating_action_button_theme_data: FloatingActionButtonThemeData,
    context: DefaultsContext,
    r#type: FloatingActionButtonType,
    has_child: bool,
}

impl FabDefaultsM2 {
    fn new(
        app: &mut App,
        context: BuildContext,
        r#type: FloatingActionButtonType,
        has_child: bool,
    ) -> FabDefaultsM2 {
        FabDefaultsM2 {
            floating_action_button_theme_data: FloatingActionButtonThemeData::new()
                .elevation(6.0)
                .focus_elevation(6.0)
                .hover_elevation(8.0)
                .highlight_elevation(12.0)
                .enable_feedback(true)
                .size_constraints(BoxConstraints::tight_for(Some(56.0), Some(56.0)))
                .small_size_constraints(BoxConstraints::tight_for(Some(40.0), Some(40.0)))
                .large_size_constraints(BoxConstraints::tight_for(Some(96.0), Some(96.0)))
                .extended_size_constraints(BoxConstraints::tight_for(None, Some(48.0)))
                .extended_icon_label_spacing(8.0),
            context: DefaultsContext::with_theme(app, context),
            r#type,
            has_child,
        }
    }
}

impl FloatingActionButtonThemeDataGetters for FabDefaultsM2 {
    fn floating_action_button_theme_data(&self) -> &FloatingActionButtonThemeData {
        &self.floating_action_button_theme_data
    }

    fn foreground_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_secondary())
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.secondary())
    }

    fn focus_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        Some(theme.focus_color)
    }

    fn hover_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        Some(theme.hover_color)
    }

    fn splash_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        Some(theme.splash_color)
    }

    fn shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        let is_extended = self.r#type == FloatingActionButtonType::Extended;
        Some(if is_extended {
            ShapeBorderRef::new(StadiumBorder::default())
        } else {
            ShapeBorderRef::new(CircleBorder::default())
        })
    }

    fn icon_size(&self, _app: &mut App) -> Option<f64> {
        let is_large = self.r#type == FloatingActionButtonType::Large;
        Some(if is_large { 36.0 } else { 24.0 })
    }

    fn extended_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        let is_extended = self.r#type == FloatingActionButtonType::Extended;
        Some(
            EdgeInsetsDirectional::only(
                if self.has_child && is_extended {
                    16.0
                } else {
                    20.0
                },
                0.0,
                20.0,
                0.0,
            )
            .into(),
        )
    }

    fn extended_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        Some(
            theme
                .text_theme
                .label_large
                .as_ref()
                .expect("the text theme has a labelLarge style")
                .copy_with()
                .letter_spacing(1.2),
        )
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/floating_action_button_test.dart` and
    //! `test/floating_action_button_theme_test.dart`.

    use std::cell::{Cell, RefCell};
    use std::time::Duration;
    use waft_painting::ShapeBorder;
    use waft_rendering::testing::{CanvasMethod, PaintCall, assert_paints, count_calls};

    use waft_embedder::{Rect, TargetPlatform};
    use waft_painting::{BeveledRectangleBorder, BorderRadius, RoundedRectangleBorder};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Align, AnyElement, Center, FocusNode, GlobalKey, Icon, IconData, PhysicalShape, Text,
    };

    use waft_widgets::testing::{
        TestPointer, WidgetTester, expect, find, finds_nothing, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme_data::ThemeData;

    fn noop() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn m3(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn m2(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// `MaterialApp(theme: theme, home: Scaffold(floatingActionButton: fab))`.
    fn app_with(theme: ThemeData, fab: FloatingActionButton) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new().floating_action_button(fab).into_widget(),
        )
    }

    fn icon(data: IconData) -> Icon {
        Icon::new(Some(data))
    }

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    fn fab(tester: &WidgetTester) -> AnyElement {
        only(find::by_type::<FloatingActionButton>().evaluate(tester))
    }

    fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        tester.get_rect(render)
    }

    fn fab_rect(tester: &WidgetTester) -> Rect {
        tester.get_rect(fab(tester))
    }

    fn icon_rect(tester: &WidgetTester) -> Rect {
        tester.get_rect(only(find::by_type::<Icon>().evaluate(tester)))
    }

    /// Dart's `tester.widget<RawMaterialButton>(find.byType(RawMaterialButton))`.
    fn raw_button<R>(tester: &WidgetTester, read: impl FnOnce(&RawMaterialButton) -> R) -> R {
        tester.widget(
            only(find::by_type::<RawMaterialButton>().evaluate(tester)),
            read,
        )
    }

    /// Dart's `tester.widget<PhysicalShape>(find.byType(PhysicalShape)).elevation`.
    fn elevation(tester: &WidgetTester) -> f64 {
        tester.widget(
            only(find::by_type::<PhysicalShape>().evaluate(tester)),
            |shape: &PhysicalShape| shape.elevation,
        )
    }

    /// Dart's `tester.press(find.byType(PhysicalShape))`.
    fn press_physical_shape(tester: &mut WidgetTester) -> TestPointer {
        let center = tester
            .get_rect(only(find::by_type::<PhysicalShape>().evaluate(tester)))
            .center();
        let mut touch = TestPointer::touch(1);
        tester.down(&mut touch, center);
        touch
    }

    /// The colour of the icon's glyph: Dart's `_getRichText(tester).text.style!.color`.
    fn icon_color(tester: &WidgetTester) -> Option<Color> {
        tester.icon_style().color.as_deref().copied()
    }

    /// Dart's `_getIconSize(tester)`: the `SizedBox` the `Icon` builds.
    fn icon_size(tester: &WidgetTester) -> (Option<f64>, Option<f64>) {
        let icon = only(find::by_type::<Icon>().evaluate(tester));
        let sized_box = find::by_type::<SizedBox>()
            .evaluate(tester)
            .into_iter()
            .find(|sized_box| tester.ancestors(*sized_box).contains(&icon))
            .expect("the Icon's SizedBox");
        tester.widget(sized_box, |sized_box: &SizedBox| {
            (sized_box.width, sized_box.height)
        })
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            waft_painting::BorderSide::NONE,
            BorderRadius::all(waft_embedder::Radius::circular(radius)).into(),
        )
    }

    fn assert_shape(tester: &WidgetTester, expected: &dyn ShapeBorder) {
        raw_button(tester, |button| {
            assert!(
                button.shape.eq_shape(expected),
                "{:?} is not {expected:?}",
                button.shape
            );
        });
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<waft_widgets::AnyFocusNode>()
    }

    #[test]
    fn floating_action_button_control_test() {
        test_widgets(|tester| {
            let did_press_button = Rc::new(Cell::new(false));
            tester.pump_widget(ltr(Center::new().child(
                FloatingActionButton::new(Some(Listener::new({
                    let did_press_button = Rc::clone(&did_press_button);
                    move |_app| did_press_button.set(true)
                })))
                .child(icon(Icons::add())),
            )));

            assert!(!did_press_button.get());
            let at = icon_rect(tester).center();
            tester.tap_at(at, 1);
            assert!(did_press_button.get());
        });
    }

    #[test]
    fn floating_action_button_tooltip() {
        test_widgets(|tester| {
            let theme = m3(tester);
            tester.pump_widget(app_with(
                theme,
                FloatingActionButton::new(noop())
                    .tooltip("Add")
                    .child(icon(Icons::add())),
            ));

            let at = icon_rect(tester).center();
            tester.tap_at(at, 1);
            let tooltips = find::by_type::<Tooltip>().evaluate(tester);
            let found = tooltips
                .into_iter()
                .filter(|tooltip| {
                    tester.widget(*tooltip, |tooltip: &Tooltip| {
                        tooltip.message.as_deref() == Some("Add")
                    })
                })
                .count();
            assert_eq!(found, 1);
        });
    }

    /// Dart's `_rightEdgeOfFab(tester)`.
    fn right_edge_of_fab(tester: &WidgetTester) -> Offset {
        fab_rect(tester).center_right() - Offset::new(1.0, 0.0)
    }

    fn long_press_at_right_edge_shows_tooltip(
        tester: &mut WidgetTester,
        fab: FloatingActionButton,
    ) {
        let theme = m3(tester);
        tester.pump_widget(app_with(theme, fab));

        expect(tester, &find::text("Add"), finds_nothing());
        let at = right_edge_of_fab(tester);
        tester.long_press_at(at, 1);
        tester.pump_and_settle();

        expect(tester, &find::text("Add"), finds_one_widget());
    }

    // Regression test for: https://github.com/flutter/flutter/pull/21084
    #[test]
    fn floating_action_button_tooltip_long_press_button_edge() {
        test_widgets(|tester| {
            long_press_at_right_edge_shows_tooltip(
                tester,
                FloatingActionButton::new(noop())
                    .tooltip("Add")
                    .child(icon(Icons::add())),
            );
        });
    }

    // Regression test for: https://github.com/flutter/flutter/pull/21084
    #[test]
    fn floating_action_button_tooltip_long_press_button_edge_no_child() {
        test_widgets(|tester| {
            long_press_at_right_edge_shows_tooltip(
                tester,
                FloatingActionButton::new(noop()).tooltip("Add"),
            );
        });
    }

    fn hover_and_long_press_show_tooltip(tester: &mut WidgetTester, on_pressed: Option<Listener>) {
        let theme = m3(tester);
        tester.pump_widget(app_with(
            theme,
            FloatingActionButton::new(on_pressed).tooltip("Add"),
        ));

        expect(tester, &find::text("Add"), finds_nothing());

        // Test hover for tooltip.
        let mut mouse = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.pump_and_settle();
        let center = fab_rect(tester).center();
        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();

        expect(tester, &find::text("Add"), finds_one_widget());

        tester.move_to(&mut mouse, Offset::ZERO, false);
        tester.pump_and_settle();

        expect(tester, &find::text("Add"), finds_nothing());

        // Test long press for tooltip.
        let center = fab_rect(tester).center();
        tester.long_press_at(center, 2);
        tester.pump_and_settle();

        expect(tester, &find::text("Add"), finds_one_widget());
    }

    #[test]
    fn floating_action_button_tooltip_no_child() {
        test_widgets(|tester| hover_and_long_press_show_tooltip(tester, noop()));
    }

    #[test]
    fn floating_action_button_tooltip_reacts_when_disabled() {
        test_widgets(|tester| hover_and_long_press_show_tooltip(tester, None));
    }

    /// Dart's 'Floating Action Button elevation when highlighted - effect', for Material 3
    /// (`highlight` 6) and in its Material 2 group (12).
    fn elevation_when_highlighted(
        tester: &mut WidgetTester,
        theme: fn(&WidgetTester) -> ThemeData,
        highlight: f64,
    ) {
        tester.pump_widget(app_with(theme(tester), FloatingActionButton::new(noop())));
        assert_eq!(elevation(tester), 6.0);
        let gesture = press_physical_shape(tester);
        tester.pump();
        assert_eq!(elevation(tester), 6.0);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), highlight);
        tester.pump_widget(app_with(
            theme(tester),
            FloatingActionButton::new(noop()).highlight_elevation(20.0),
        ));
        tester.pump();
        assert_eq!(elevation(tester), highlight);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), 20.0);
        tester.up(&gesture);
        tester.pump();
        assert_eq!(elevation(tester), 20.0);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), 6.0);
    }

    #[test]
    fn floating_action_button_elevation_when_highlighted_effect() {
        test_widgets(|tester| elevation_when_highlighted(tester, m3, 6.0));
    }

    #[test]
    fn material2_floating_action_button_elevation_when_highlighted_effect() {
        test_widgets(|tester| elevation_when_highlighted(tester, m2, 12.0));
    }

    #[test]
    fn floating_action_button_elevation_when_disabled_defaults() {
        test_widgets(|tester| {
            let theme = m3(tester);
            tester.pump_widget(app_with(theme, FloatingActionButton::new(None)));

            // Disabled elevation defaults to regular default elevation.
            assert_eq!(elevation(tester), 6.0);
        });
    }

    #[test]
    fn floating_action_button_elevation_when_disabled_override() {
        test_widgets(|tester| {
            let theme = m3(tester);
            tester.pump_widget(app_with(
                theme,
                FloatingActionButton::new(None).disabled_elevation(0.0),
            ));

            assert_eq!(elevation(tester), 0.0);
        });
    }

    #[test]
    fn floating_action_button_elevation_when_disabled_effect() {
        test_widgets(|tester| {
            tester.pump_widget(app_with(m3(tester), FloatingActionButton::new(None)));
            assert_eq!(elevation(tester), 6.0);
            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::new(None).disabled_elevation(3.0),
            ));
            assert_eq!(elevation(tester), 6.0);
            tester.pump_by(Duration::from_secs(1));
            assert_eq!(elevation(tester), 3.0);
            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::new(noop()).disabled_elevation(3.0),
            ));
            assert_eq!(elevation(tester), 3.0);
            tester.pump_by(Duration::from_secs(1));
            assert_eq!(elevation(tester), 6.0);
        });
    }

    /// Dart's 'Floating Action Button elevation when disabled while highlighted - effect', for
    /// Material 3 (`highlight` 6) and in its Material 2 group (12).
    fn elevation_when_disabled_while_highlighted(
        tester: &mut WidgetTester,
        theme: fn(&WidgetTester) -> ThemeData,
        highlight: f64,
    ) {
        tester.pump_widget(app_with(theme(tester), FloatingActionButton::new(noop())));
        assert_eq!(elevation(tester), 6.0);
        let _gesture = press_physical_shape(tester);
        tester.pump();
        assert_eq!(elevation(tester), 6.0);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), highlight);
        tester.pump_widget(app_with(theme(tester), FloatingActionButton::new(None)));
        tester.pump();
        assert_eq!(elevation(tester), highlight);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), 6.0);
        tester.pump_widget(app_with(theme(tester), FloatingActionButton::new(noop())));
        tester.pump();
        assert_eq!(elevation(tester), 6.0);
        tester.pump_by(Duration::from_secs(1));
        assert_eq!(elevation(tester), 6.0);
    }

    #[test]
    fn floating_action_button_elevation_when_disabled_while_highlighted_effect() {
        test_widgets(|tester| elevation_when_disabled_while_highlighted(tester, m3, 6.0));
    }

    #[test]
    fn material2_floating_action_button_elevation_when_disabled_while_highlighted_effect() {
        test_widgets(|tester| elevation_when_disabled_while_highlighted(tester, m2, 12.0));
    }

    /// Dart's 'Floating Action Button states elevation', for Material 3 (`highlight` 6) and
    /// in its Material 2 group (12).
    fn states_elevation(
        tester: &mut WidgetTester,
        theme: fn(&WidgetTester) -> ThemeData,
        highlight: f64,
    ) {
        let focus_node = focus_node(tester);
        tester.pump_widget(material_app(
            theme(tester),
            Scaffold::new()
                .body(
                    FloatingActionButton::extended(noop(), Text::new("tooltip"))
                        .focus_node(focus_node.clone()),
                )
                .into_widget(),
        ));

        // Default, not disabled.
        assert_eq!(elevation(tester), 6.0);

        // Focused.
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        assert_eq!(elevation(tester), 6.0);

        // Hovered.
        let center = tester
            .get_rect(only(find::by_type::<PhysicalShape>().evaluate(tester)))
            .center();
        let mut mouse = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, center, false);
        tester.pump_and_settle();
        assert_eq!(elevation(tester), 8.0);

        // Highlighted (pressed).
        tester.down(&mut mouse, center);
        tester.pump(); // Start the splash and highlight animations.
        tester.pump_by(Duration::from_millis(800)); // Wait for splash and highlight to be well under way.
        assert_eq!(elevation(tester), highlight);
    }

    #[test]
    fn floating_action_button_states_elevation() {
        test_widgets(|tester| states_elevation(tester, m3, 6.0));
    }

    #[test]
    fn material2_floating_action_button_states_elevation() {
        test_widgets(|tester| states_elevation(tester, m2, 12.0));
    }

    #[test]
    fn flat_action_button_mini_size_is_configurable_by_theme_data_material_tap_target_size() {
        test_widgets(|tester| {
            let mini_size = |tester: &mut WidgetTester, size: MaterialTapTargetSize| {
                let theme = ThemeData::builder()
                    .material_tap_target_size(size)
                    .build(&tester.cell.borrow());
                let key = GlobalKey::new();
                tester.pump_widget(material_app(
                    m3(tester),
                    Theme::new(
                        theme,
                        Scaffold::new().floating_action_button(
                            FloatingActionButton::new(None)
                                .key(key.clone().into())
                                .mini(true),
                        ),
                    )
                    .into_widget(),
                ));
                tester.get_size(&key)
            };

            assert_eq!(
                mini_size(tester, MaterialTapTargetSize::Padded),
                Size::new(48.0, 48.0)
            );
            assert_eq!(
                mini_size(tester, MaterialTapTargetSize::ShrinkWrap),
                Size::new(40.0, 40.0)
            );
        });
    }

    /// Dart's 'FloatingActionButton.isExtended' after its first frame: an extended button with
    /// an icon and a 100 wide label, `height` high and laid out as 16 icon 8 label 20.
    fn assert_extended_with_icon_layout(tester: &WidgetTester, height: f64) {
        assert!(tester.widget(fab(tester), |fab: &FloatingActionButton| fab.is_extended));
        expect(tester, &find::text("label"), finds_one_widget());
        expect(tester, &find::by_type::<Icon>(), finds_one_widget());

        // Verify that the widget's height is `height` and that its internal
        // horizontal layout is: 16 icon 8 label 20
        let fab = fab_rect(tester);
        let icon = icon_rect(tester);
        let label = tester.get_rect(find::text("label"));
        assert_eq!(fab.height(), height);
        assert_eq!(icon.left - fab.left, 16.0);
        assert_eq!(label.left - icon.right, 8.0);
        assert_eq!(fab.right - label.right, 20.0);

        // The overall width of the button is:
        // 168 = 16 + 24(icon) + 8 + 100(label) + 20
        assert_eq!(icon.width(), 24.0);
        assert_eq!(label.width(), 100.0);
        assert_eq!(fab.width(), 168.0);
    }

    /// Dart's 'FloatingActionButton.isExtended (without icon)': a 100 wide label, `height`
    /// high and laid out as 20 label 20.
    fn assert_extended_without_icon_layout(tester: &WidgetTester, height: f64) {
        assert!(tester.widget(fab(tester), |fab: &FloatingActionButton| fab.is_extended));
        expect(tester, &find::text("label"), finds_one_widget());
        expect(tester, &find::by_type::<Icon>(), finds_nothing());

        // Verify that the widget's height is `height` and that its internal
        // horizontal layout is: 20 label 20
        let fab = fab_rect(tester);
        let label = tester.get_rect(find::text("label"));
        assert_eq!(fab.height(), height);
        assert_eq!(label.left - fab.left, 20.0);
        assert_eq!(fab.right - label.right, 20.0);

        // The overall width of the button is:
        // 140 = 20 + 100(label) + 20
        assert_eq!(label.width(), 100.0);
        assert_eq!(fab.width(), 140.0);
    }

    fn sized_label() -> SizedBox {
        SizedBox::new().width(100.0).child(Text::new("label"))
    }

    #[test]
    fn floating_action_button_is_extended() {
        test_widgets(|tester| {
            tester.pump_widget(app_with(m3(tester), FloatingActionButton::new(None)));

            assert!(!tester.widget(fab(tester), |fab: &FloatingActionButton| fab.is_extended));
            assert_shape(tester, &rounded(16.0));

            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::extended(None, sized_label()).icon(icon(Icons::android())),
            ));

            assert_shape(tester, &rounded(16.0));
            assert_extended_with_icon_layout(tester, 56.0);
        });
    }

    #[test]
    fn floating_action_button_is_extended_without_icon() {
        test_widgets(|tester| {
            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::extended(None, sized_label()),
            ));

            assert_shape(tester, &rounded(16.0));
            assert_extended_without_icon_layout(tester, 56.0);
        });
    }

    #[test]
    fn material2_floating_action_button_is_extended() {
        test_widgets(|tester| {
            tester.pump_widget(app_with(m2(tester), FloatingActionButton::new(None)));

            assert!(!tester.widget(fab(tester), |fab: &FloatingActionButton| fab.is_extended));
            assert_shape(tester, &CircleBorder::default());

            tester.pump_widget(app_with(
                m2(tester),
                FloatingActionButton::extended(None, sized_label()).icon(icon(Icons::android())),
            ));

            assert_shape(tester, &StadiumBorder::default());
            assert_extended_with_icon_layout(tester, 48.0);
        });
    }

    #[test]
    fn material2_floating_action_button_is_extended_without_icon() {
        test_widgets(|tester| {
            tester.pump_widget(app_with(
                m2(tester),
                FloatingActionButton::extended(None, sized_label()),
            ));

            assert_shape(tester, &StadiumBorder::default());
            assert_extended_without_icon_layout(tester, 48.0);
        });
    }

    /// `MaterialApp(home: Scaffold(body: Builder(..), floatingActionButton: ..))` with the
    /// hero controller `MaterialApp` gives its navigator. Returns the body's context.
    fn two_fabs(
        tester: &mut WidgetTester,
        body: FloatingActionButton,
        fab: FloatingActionButton,
    ) -> BuildContext {
        let context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let body = Rc::new(RefCell::new(Some(body)));
        let theme = m3(tester);
        let controller = waft_widgets::HeroController::new(&mut tester.cell.borrow_mut());
        tester.pump_widget(
            waft_widgets::HeroControllerScope::new(
                controller,
                material_app(
                    theme,
                    Scaffold::new()
                        .body(waft_widgets::Builder::new({
                            let context = Rc::clone(&context);
                            move |_app, built| {
                                context.set(Some(built));
                                body.borrow_mut().take().expect("built once").into_widget()
                            }
                        }))
                        .floating_action_button(fab)
                        .into_widget(),
                ),
            )
            .into_widget(),
        );
        context.get().expect("the body built")
    }

    /// Dart's `Navigator.push(context, PageRouteBuilder(pageBuilder: (..) => Placeholder()))`.
    fn push_page(tester: &WidgetTester, context: BuildContext) {
        let mut app = tester.cell.borrow_mut();
        let page_builder: waft_widgets::RoutePageBuilder =
            Rc::new(|_app, _context, _, _| SizedBox::expand().into_widget());
        let route = waft_widgets::PageRouteBuilder::new(&mut app, page_builder);
        drop(waft_widgets::Navigator::push(
            &mut app,
            context,
            route.erase(),
        ));
    }

    #[test]
    fn floating_action_button_hero_tag() {
        test_widgets(|tester| {
            let context = two_fabs(
                tester,
                FloatingActionButton::new(None).hero_tag(Some(Rc::new(1))),
                FloatingActionButton::new(None).hero_tag(Some(Rc::new(2))),
            );
            push_page(tester, context);
            // this would fail if heroTag was the same on both FloatingActionButtons (see below).
            tester.pump();
        });
    }

    /// Dart pushes a `MaterialPageRoute` from the scaffold's button; a `PageRouteBuilder`,
    /// whose page has no transition of its own, carries the hero flight here.
    #[test]
    fn extended_fab_hero_transitions_succeed() {
        // Regression test for https://github.com/flutter/flutter/issues/18782
        test_widgets(|tester| {
            let theme = m3(tester);
            let controller = waft_widgets::HeroController::new(&mut tester.cell.borrow_mut());
            let second_page: waft_widgets::RoutePageBuilder = Rc::new(|_app, context, _, _| {
                Scaffold::new()
                    .floating_action_button(
                        FloatingActionButton::extended(noop(), Text::new("X"))
                            .icon(icon(Icons::add())),
                    )
                    .body(
                        Center::new().child(crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(move |app| {
                                waft_widgets::Navigator::pop(app, context, None);
                            })),
                            Some(Text::new("POP").into_widget()),
                        )),
                    )
                    .into_widget()
            });
            let first_page = Scaffold::new()
                .floating_action_button(waft_widgets::Builder::new(move |_app, context| {
                    // define context of Navigator.push()
                    let second_page = Rc::clone(&second_page);
                    FloatingActionButton::extended(
                        Some(Listener::new(move |app| {
                            let route =
                                waft_widgets::PageRouteBuilder::new(app, Rc::clone(&second_page));
                            drop(waft_widgets::Navigator::push(app, context, route.erase()));
                        })),
                        Text::new("A long FAB label"),
                    )
                    .icon(icon(Icons::add()))
                    .into_widget()
                }))
                .body(Center::new().child(Text::new("Hello World")));
            tester.pump_widget(
                waft_widgets::HeroControllerScope::new(
                    controller,
                    material_app(theme, first_page.into_widget()),
                )
                .into_widget(),
            );

            let long_fab = "A long FAB label";
            let short_fab = "X";
            let hello_world = "Hello World";

            expect(tester, &find::text(long_fab), finds_one_widget());
            expect(tester, &find::text(short_fab), finds_nothing());
            expect(tester, &find::text(hello_world), finds_one_widget());

            let at = tester.get_rect(find::text(long_fab)).center();
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            expect(tester, &find::text(short_fab), finds_one_widget());
            expect(tester, &find::text(long_fab), finds_nothing());

            // Trigger a hero transition from shortFAB to longFAB.
            let at = tester.get_rect(find::text("POP")).center();
            tester.tap_at(at, 2);
            tester.pump_and_settle();

            expect(tester, &find::text(long_fab), finds_one_widget());
            expect(tester, &find::text(short_fab), finds_nothing());
            expect(tester, &find::text(hello_world), finds_one_widget());
        });
    }

    /// Not in Flutter's file: the duplicate-tag cases assert the message names the default
    /// tag, which waft-widgets' assert does not carry; this pins the tag they share.
    #[test]
    fn every_floating_action_button_shares_one_default_hero_tag() {
        let a = FloatingActionButton::new(None)
            .hero_tag
            .expect("a default tag");
        let b = FloatingActionButton::extended(None, SizedBox::new())
            .hero_tag
            .expect("a default tag");
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(format!("{a:?}"), "<default FloatingActionButton tag>");
        assert!(
            FloatingActionButton::new(None)
                .hero_tag(None)
                .hero_tag
                .is_none()
        );
    }

    #[test]
    fn floating_action_button_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            let top_left = |fab: FloatingActionButton| {
                Scaffold::new()
                    .body(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(fab),
                    )
                    .into_widget()
            };
            tester.pump_widget(material_app(
                m3(tester),
                top_left(
                    FloatingActionButton::extended(noop(), Text::new("label"))
                        .mouse_cursor(SystemMouseCursors::TEXT)
                        .icon(icon(Icons::android())),
                ),
            ));

            let mut mouse = TestPointer::mouse(1, 0);
            let center = fab_rect(tester).center();
            tester.add_pointer(&mut mouse, center);

            tester.pump();

            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            assert!(*tester.active_cursor(1) == *text);

            tester.pump_widget(material_app(
                m3(tester),
                top_left(
                    FloatingActionButton::new(noop())
                        .mouse_cursor(SystemMouseCursors::TEXT)
                        .child(icon(Icons::add())),
                ),
            ));

            let center = fab_rect(tester).center();
            tester.move_to(&mut mouse, center, false);
            assert!(*tester.active_cursor(1) == *text);

            // Test default cursor
            tester.pump_widget(material_app(
                m3(tester),
                top_left(FloatingActionButton::new(noop()).child(icon(Icons::add()))),
            ));

            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            assert!(*tester.active_cursor(1) == *basic);

            // Test default cursor when disabled
            tester.pump_widget(material_app(
                m3(tester),
                top_left(FloatingActionButton::new(None).child(icon(Icons::add()))),
            ));

            assert!(*tester.active_cursor(1) == *basic);
        });
    }

    #[test]
    fn floating_action_button_has_no_clip_by_default() {
        test_widgets(|tester| {
            let focus_node = focus_node(tester);
            tester.pump_widget(ltr(FloatingActionButton::new(Some(Listener::new(
                |_app| {
                    // to make sure the button is enabled
                },
            )))
            .focus_node(focus_node.clone())));

            focus_node.unfocus(&mut tester.cell.borrow_mut(), Default::default());
            tester.pump();

            let render = tester.render_box_of(fab(tester)).as_object();
            let clips = count_calls(&tester.painted_ops(render), CanvasMethod::Clip);
            assert_eq!(clips, 0);
        });
    }

    #[test]
    fn foreground_color_applies_to_icon_on_fab() {
        test_widgets(|tester| {
            let foreground_color = Color::new(0xcafefeed);

            tester.pump_widget(material_app(
                m3(tester),
                FloatingActionButton::new(noop())
                    .foreground_color(foreground_color)
                    .child(icon(Icons::access_alarm()))
                    .into_widget(),
            ));

            assert_eq!(icon_color(tester), Some(foreground_color));
        });
    }

    #[test]
    fn floating_action_button_uses_custom_splash_color() {
        test_widgets(|tester| {
            let splash_color = Color::new(0xcafefeed);

            tester.pump_widget(material_app(
                m2(tester),
                FloatingActionButton::new(noop())
                    .splash_color(splash_color)
                    .child(icon(Icons::access_alarm()))
                    .into_widget(),
            ));

            let center = fab_rect(tester).center();
            let mut touch = TestPointer::touch(1);
            tester.down(&mut touch, center);
            tester.pump_and_settle();

            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::circle(Some(splash_color))],
            );
        });
    }

    #[test]
    fn extended_fab_does_not_show_label_when_is_extended_is_false() {
        test_widgets(|tester| {
            let icon_key = GlobalKey::new();
            let label_key = GlobalKey::new();

            tester.pump_widget(ltr(FloatingActionButton::extended(
                noop(),
                Text::new("").key(label_key.clone().into()),
            )
            .is_extended(false)
            .icon(icon(Icons::add()).key(icon_key.clone().into()))));

            // Verify that Icon is present and label is not.
            let app = tester.cell.borrow();
            assert!(icon_key.current_context(&app).is_some());
            assert!(label_key.current_context(&app).is_none());
        });
    }

    #[test]
    fn floating_action_button_small_configures_correct_size() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::small(None)
                    .key(key.clone().into())
                    .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap),
            ));

            assert_eq!(tester.get_size(&key), Size::new(40.0, 40.0));
        });
    }

    #[test]
    fn floating_action_button_large_configures_correct_size() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::large(None).key(key.clone().into()),
            ));

            assert_eq!(tester.get_size(&key), Size::new(96.0, 96.0));
        });
    }

    /// The spacing and padding cases: the label's left minus the icon's right, the icon's
    /// left minus the button's, and the button's right minus the label's.
    fn extended_gaps(
        tester: &WidgetTester,
        icon_key: &GlobalKey,
        label_key: &GlobalKey,
    ) -> [f64; 3] {
        let fab = fab_rect(tester);
        let icon = key_rect(tester, icon_key);
        let label = key_rect(tester, label_key);
        [
            label.left - icon.right,
            icon.left - fab.left,
            fab.right - label.right,
        ]
    }

    #[test]
    fn floating_action_button_extended_can_customize_spacing() {
        test_widgets(|tester| {
            let icon_key = GlobalKey::new();
            let label_key = GlobalKey::new();
            let spacing = 33.0;
            let padding = EdgeInsetsDirectional::only(5.0, 0.0, 6.0, 0.0);

            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::extended(noop(), Text::new("").key(label_key.clone().into()))
                    .icon(icon(Icons::add()).key(icon_key.clone().into()))
                    .extended_icon_label_spacing(spacing)
                    .extended_padding(padding),
            ));

            assert_eq!(
                extended_gaps(tester, &icon_key, &label_key),
                [spacing, padding.start, padding.end]
            );
        });
    }

    #[test]
    fn floating_action_button_extended_can_customize_text_style() {
        test_widgets(|tester| {
            let style = TextStyle::new().letter_spacing(2.0);

            tester.pump_widget(app_with(
                m2(tester),
                FloatingActionButton::extended(noop(), Text::new(""))
                    .icon(icon(Icons::add()))
                    .extended_text_style(style.clone()),
            ));

            // The color comes from the default color scheme's onSecondary value.
            raw_button(tester, |button| {
                assert_eq!(
                    button.text_style,
                    Some(style.copy_with().color(Color::new(0xffffffff)))
                );
            });
        });
    }

    /// Records the haptic feedback the platform is asked for.
    #[derive(Default)]
    struct FeedbackTester {
        haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
    }

    impl waft_embedder::Haptics for FeedbackTester {
        fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
            self.haptics.borrow_mut().push(kind);
        }
    }

    /// Dart's `feedback` group: taps `fab` under `theme` and returns whether its `InkWell`
    /// asks for feedback, which is what the click sound count stands for while the click
    /// system sound waits with `SystemSound`, and checks that no haptic was asked for.
    fn tap_feedback(
        theme: Option<FloatingActionButtonThemeData>,
        fab: FloatingActionButton,
    ) -> bool {
        let feedback = Rc::new(FeedbackTester::default());
        let platform = waft_test::TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
        let mut tester = WidgetTester::with_platform(platform);
        let tester = &mut tester;
        let fab = fab.child(icon(Icons::access_alarm()));
        let home = match theme {
            Some(fab_theme) => {
                let theme = ThemeData::builder()
                    .floating_action_button_theme(fab_theme)
                    .build(&tester.cell.borrow());
                Theme::new(theme, fab).into_widget()
            }
            None => fab.into_widget(),
        };
        tester.pump_widget(material_app(m3(tester), home));

        let at = tester
            .get_rect(only(find::by_type::<RawMaterialButton>().evaluate(tester)))
            .center();
        tester.tap_at(at, 1);
        tester.pump_by(Duration::from_secs(1));
        assert!(feedback.haptics.borrow().is_empty());
        tester.widget(
            only(find::by_type::<InkWell>().evaluate(tester)),
            |ink_well: &InkWell| ink_well.ink_response.enable_feedback,
        )
    }

    #[test]
    fn floating_action_button_with_enabled_feedback() {
        test_widgets(|_| {
            assert!(tap_feedback(
                None,
                FloatingActionButton::new(noop()).enable_feedback(true)
            ));
        });
    }

    #[test]
    fn floating_action_button_with_disabled_feedback() {
        test_widgets(|_| {
            assert!(!tap_feedback(
                None,
                FloatingActionButton::new(noop()).enable_feedback(false)
            ));
        });
    }

    #[test]
    fn floating_action_button_with_enabled_feedback_by_default() {
        test_widgets(|_| {
            assert!(tap_feedback(None, FloatingActionButton::new(noop())));
        });
    }

    #[test]
    fn floating_action_button_with_disabled_feedback_using_floating_action_button_theme() {
        test_widgets(|_| {
            assert!(!tap_feedback(
                Some(FloatingActionButtonThemeData::new().enable_feedback(false)),
                FloatingActionButton::new(noop()),
            ));
        });
    }

    #[test]
    fn floating_action_button_enable_feedback_is_overridden_by_floating_action_button_theme_data_enable_feedback()
     {
        test_widgets(|_| {
            assert!(tap_feedback(
                Some(FloatingActionButtonThemeData::new().enable_feedback(false)),
                FloatingActionButton::new(noop()).enable_feedback(true),
            ));
        });
    }

    #[test]
    fn floating_action_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(material_app(
                m3(tester),
                Scaffold::new()
                    .floating_action_button(
                        Center::new()
                            .child(SizedBox::shrink().child(FloatingActionButton::new(noop()))),
                    )
                    .into_widget(),
            ));
            assert_eq!(fab_rect(tester).size(), Size::ZERO);
        });
    }

    // floating_action_button_theme_test.dart

    /// `MaterialApp(theme: ThemeData(floatingActionButtonTheme: fab_theme), home: Scaffold(
    /// floatingActionButton: fab))`, with `Center` in the scaffold's place.
    fn app_with_fab_theme(
        tester: &mut WidgetTester,
        fab_theme: FloatingActionButtonThemeData,
        fab: FloatingActionButton,
    ) {
        let theme = ThemeData::builder()
            .floating_action_button_theme(fab_theme)
            .build(&tester.cell.borrow());
        tester.pump_widget(app_with(theme, fab));
    }

    #[test]
    fn material3_default_values_are_used_when_no_floating_action_button_or_floating_action_button_theme_data_properties_are_specified()
     {
        test_widgets(|tester| {
            let color_scheme = ColorScheme::light();
            let theme = ThemeData::from(color_scheme.clone()).build(&tester.cell.borrow());
            tester.pump_widget(app_with(
                theme,
                FloatingActionButton::new(noop()).child(icon(Icons::add())),
            ));

            raw_button(tester, |button| {
                assert_eq!(button.fill_color, Some(color_scheme.primary_container()));
            });
            assert_eq!(
                icon_color(tester),
                Some(color_scheme.on_primary_container())
            );

            // These defaults come directly from the [FloatingActionButton].
            raw_button(tester, |button| {
                assert_eq!(button.elevation, 6.0);
                assert_eq!(button.highlight_elevation, 6.0);
                assert!(button.shape.eq_shape(&rounded(16.0)));
                #[allow(deprecated)]
                let splash = color_scheme.on_primary_container().with_opacity(0.1);
                assert_eq!(button.splash_color, Some(splash));
                assert_eq!(
                    button.constraints,
                    BoxConstraints::tight_for(Some(56.0), Some(56.0))
                );
            });
            assert_eq!(icon_size(tester), (Some(24.0), Some(24.0)));
        });
    }

    #[test]
    fn material2_default_values_are_used_when_no_floating_action_button_or_floating_action_button_theme_data_properties_are_specified()
     {
        test_widgets(|tester| {
            let color_scheme = ColorScheme::light();
            let theme = ThemeData::from(color_scheme.clone())
                .use_material3(false)
                .build(&tester.cell.borrow());
            let default_splash_color = m3(tester).splash_color;
            tester.pump_widget(app_with(
                theme,
                FloatingActionButton::new(noop()).child(icon(Icons::add())),
            ));

            raw_button(tester, |button| {
                assert_eq!(button.fill_color, Some(color_scheme.secondary()));
            });
            assert_eq!(icon_color(tester), Some(color_scheme.on_secondary()));

            // These defaults come directly from the [FloatingActionButton].
            raw_button(tester, |button| {
                assert_eq!(button.elevation, 6.0);
                assert_eq!(button.highlight_elevation, 12.0);
                assert!(button.shape.eq_shape(&CircleBorder::default()));
                assert_eq!(button.splash_color, Some(default_splash_color));
                assert_eq!(
                    button.constraints,
                    BoxConstraints::tight_for(Some(56.0), Some(56.0))
                );
            });
            assert_eq!(icon_size(tester), (Some(24.0), Some(24.0)));
        });
    }

    /// The fields a theme or widget case sets, as the button resolves them.
    struct Resolved {
        background_color: Color,
        foreground_color: Color,
        splash_color: Color,
        elevation: f64,
        disabled_elevation: f64,
        highlight_elevation: f64,
        shape: ShapeBorderRef,
    }

    impl Resolved {
        fn local() -> Resolved {
            Resolved {
                background_color: Color::new(0x00000001),
                foreground_color: Color::new(0x00000002),
                splash_color: Color::new(0x00000003),
                elevation: 7.0,
                disabled_elevation: 1.0,
                highlight_elevation: 13.0,
                shape: ShapeBorderRef::new(StadiumBorder::default()),
            }
        }

        fn theme_data(&self) -> FloatingActionButtonThemeData {
            FloatingActionButtonThemeData::new()
                .background_color(self.background_color)
                .foreground_color(self.foreground_color)
                .splash_color(self.splash_color)
                .elevation(self.elevation)
                .disabled_elevation(self.disabled_elevation)
                .highlight_elevation(self.highlight_elevation)
                .shape(self.shape.clone())
        }

        /// The ambient theme the priority cases override.
        fn overridden() -> FloatingActionButtonThemeData {
            FloatingActionButtonThemeData::new()
                .background_color(Color::new(0x00000004))
                .foreground_color(Color::new(0x00000005))
                .splash_color(Color::new(0x00000006))
                .elevation(23.0)
                .disabled_elevation(11.0)
                .highlight_elevation(43.0)
                .shape(BeveledRectangleBorder::default())
        }

        fn assert_shown(&self, tester: &WidgetTester) {
            raw_button(tester, |button| {
                assert_eq!(button.fill_color, Some(self.background_color));
            });
            assert_eq!(icon_color(tester), Some(self.foreground_color));
            raw_button(tester, |button| {
                assert_eq!(button.elevation, self.elevation);
                assert_eq!(button.disabled_elevation, self.disabled_elevation);
                assert_eq!(button.highlight_elevation, self.highlight_elevation);
                assert!(button.shape.eq_shape(&*self.shape));
                assert_eq!(button.splash_color, Some(self.splash_color));
            });
        }
    }

    fn add_button() -> FloatingActionButton {
        FloatingActionButton::new(noop()).child(icon(Icons::add()))
    }

    #[test]
    fn floating_action_button_theme_data_values_are_used_when_no_floating_action_button_properties_are_specified()
     {
        test_widgets(|tester| {
            let resolved = Resolved {
                background_color: Color::new(0xBEEFBEEF),
                foreground_color: Color::new(0xFACEFACE),
                splash_color: Color::new(0xCAFEFEED),
                ..Resolved::local()
            };
            let constraints = BoxConstraints::tight_for(Some(100.0), Some(100.0));

            app_with_fab_theme(
                tester,
                resolved.theme_data().size_constraints(constraints),
                add_button(),
            );

            resolved.assert_shown(tester);
            raw_button(tester, |button| assert_eq!(button.constraints, constraints));
        });
    }

    #[test]
    fn floating_action_button_values_take_priority_over_floating_action_button_theme_data_values_when_both_properties_are_specified()
     {
        test_widgets(|tester| {
            let resolved = Resolved::local();
            app_with_fab_theme(
                tester,
                Resolved::overridden(),
                add_button()
                    .background_color(resolved.background_color)
                    .foreground_color(resolved.foreground_color)
                    .splash_color(resolved.splash_color)
                    .elevation(resolved.elevation)
                    .disabled_elevation(resolved.disabled_elevation)
                    .highlight_elevation(resolved.highlight_elevation)
                    .shape(resolved.shape.clone()),
            );

            resolved.assert_shown(tester);
        });
    }

    #[test]
    fn local_floating_action_button_theme_properties_are_used() {
        test_widgets(|tester| {
            let resolved = Resolved::local();
            tester.pump_widget(material_app(
                m3(tester),
                FloatingActionButtonTheme::new(
                    resolved.theme_data(),
                    Scaffold::new().floating_action_button(add_button()),
                )
                .into_widget(),
            ));

            resolved.assert_shown(tester);
        });
    }

    #[test]
    fn local_floating_action_button_theme_takes_priority_over_theme_data_floating_action_button_theme()
     {
        test_widgets(|tester| {
            let resolved = Resolved::local();
            let theme = ThemeData::builder()
                .floating_action_button_theme(Resolved::overridden())
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                FloatingActionButtonTheme::new(
                    resolved.theme_data(),
                    Scaffold::new().floating_action_button(add_button()),
                )
                .into_widget(),
            ));

            resolved.assert_shown(tester);
        });
    }

    #[test]
    fn floating_action_button_uses_a_custom_shape_when_specified_in_the_theme() {
        test_widgets(|tester| {
            let custom_shape = BeveledRectangleBorder::default();

            tester.pump_widget(app_with(
                m3(tester),
                FloatingActionButton::new(noop()).shape(custom_shape),
            ));

            assert_shape(tester, &custom_shape);
        });
    }

    #[test]
    fn floating_action_button_small_uses_custom_constraints_when_specified_in_the_theme() {
        test_widgets(|tester| {
            let constraints = BoxConstraints::tight_for(Some(100.0), Some(100.0));
            let icon_size_value = 24.0;

            app_with_fab_theme(
                tester,
                FloatingActionButtonThemeData::new().small_size_constraints(constraints),
                FloatingActionButton::small(noop()).child(icon(Icons::add())),
            );

            raw_button(tester, |button| assert_eq!(button.constraints, constraints));
            assert_eq!(
                icon_size(tester),
                (Some(icon_size_value), Some(icon_size_value))
            );
        });
    }

    #[test]
    fn floating_action_button_large_uses_custom_constraints_when_specified_in_the_theme() {
        test_widgets(|tester| {
            let constraints = BoxConstraints::tight_for(Some(100.0), Some(100.0));
            let icon_size_value = 36.0;

            app_with_fab_theme(
                tester,
                FloatingActionButtonThemeData::new().large_size_constraints(constraints),
                FloatingActionButton::large(noop()).child(icon(Icons::add())),
            );

            raw_button(tester, |button| assert_eq!(button.constraints, constraints));
            assert_eq!(
                icon_size(tester),
                (Some(icon_size_value), Some(icon_size_value))
            );
        });
    }

    /// The extended theme cases: a `theme` (Material 3 on `ColorScheme.light()`, or Material 2)
    /// carrying `fab_theme`, and an extended button labelled "Extended" with `customize`
    /// applied. Returns the gaps of [`extended_gaps`].
    fn extended_case(
        tester: &mut WidgetTester,
        use_material3: bool,
        fab_theme: FloatingActionButtonThemeData,
        customize: impl FnOnce(FloatingActionButton) -> FloatingActionButton,
    ) -> [f64; 3] {
        let icon_key = GlobalKey::new();
        let label_key = GlobalKey::new();
        let builder = ThemeData::builder()
            .use_material3(use_material3)
            .floating_action_button_theme(fab_theme);
        let builder = if use_material3 {
            builder.color_scheme(ColorScheme::light())
        } else {
            builder
        };
        let theme = builder.build(&tester.cell.borrow());
        tester.pump_widget(app_with(
            theme,
            customize(
                FloatingActionButton::extended(
                    noop(),
                    Text::new("Extended").key(label_key.clone().into()),
                )
                .icon(icon(Icons::add()).key(icon_key.clone().into())),
            ),
        ));
        extended_gaps(tester, &icon_key, &label_key)
    }

    /// The text colour the extended cases expect: `ColorScheme.light().onPrimaryContainer` in
    /// Material 3, and the default colour scheme's `onSecondary`, white, in Material 2.
    fn extended_text_color(use_material3: bool) -> Color {
        if use_material3 {
            ColorScheme::light().on_primary_container()
        } else {
            Color::new(0xffffffff)
        }
    }

    fn extended_uses_custom_properties_when_specified_in_the_theme(
        tester: &mut WidgetTester,
        use_material3: bool,
    ) {
        let constraints = BoxConstraints::tight_for(None, Some(100.0));
        let icon_label_spacing = 33.0;
        let padding = EdgeInsetsDirectional::only(5.0, 0.0, 6.0, 0.0);
        let text_style = TextStyle::new().letter_spacing(2.0);

        let gaps = extended_case(
            tester,
            use_material3,
            FloatingActionButtonThemeData::new()
                .extended_size_constraints(constraints)
                .extended_icon_label_spacing(icon_label_spacing)
                .extended_padding(padding)
                .extended_text_style(text_style.clone()),
            |fab| fab,
        );

        raw_button(tester, |button| assert_eq!(button.constraints, constraints));
        assert_eq!(gaps, [icon_label_spacing, padding.start, padding.end]);
        raw_button(tester, |button| {
            assert_eq!(
                button.text_style,
                Some(
                    text_style
                        .copy_with()
                        .color(extended_text_color(use_material3))
                )
            );
        });
    }

    #[test]
    fn material3_floating_action_button_extended_uses_custom_properties_when_specified_in_the_theme()
     {
        test_widgets(|tester| {
            extended_uses_custom_properties_when_specified_in_the_theme(tester, true)
        });
    }

    #[test]
    fn material2_floating_action_button_extended_uses_custom_properties_when_specified_in_the_theme()
     {
        test_widgets(|tester| {
            extended_uses_custom_properties_when_specified_in_the_theme(tester, false);
        });
    }

    fn extended_custom_properties_take_priority_over_theme(
        tester: &mut WidgetTester,
        use_material3: bool,
    ) {
        let icon_label_spacing = 33.0;
        let padding = EdgeInsetsDirectional::only(5.0, 0.0, 6.0, 0.0);
        let text_style = TextStyle::new().letter_spacing(2.0);

        let gaps = extended_case(
            tester,
            use_material3,
            FloatingActionButtonThemeData::new()
                .extended_icon_label_spacing(25.0)
                .extended_padding(EdgeInsetsDirectional::only(7.0, 0.0, 8.0, 0.0))
                .extended_text_style(TextStyle::new().letter_spacing(3.0)),
            |fab| {
                fab.extended_icon_label_spacing(icon_label_spacing)
                    .extended_padding(padding)
                    .extended_text_style(text_style.clone())
            },
        );

        assert_eq!(gaps, [icon_label_spacing, padding.start, padding.end]);
        raw_button(tester, |button| {
            assert_eq!(
                button.text_style,
                Some(
                    text_style
                        .copy_with()
                        .color(extended_text_color(use_material3))
                )
            );
        });
    }

    #[test]
    fn material3_floating_action_button_extended_custom_properties_takes_priority_over_floating_action_button_theme_data_spacing()
     {
        test_widgets(|tester| extended_custom_properties_take_priority_over_theme(tester, true));
    }

    #[test]
    fn material2_floating_action_button_extended_custom_properties_takes_priority_over_floating_action_button_theme_data_spacing()
     {
        test_widgets(|tester| extended_custom_properties_take_priority_over_theme(tester, false));
    }

    #[test]
    fn floating_action_button_mouse_cursor_uses_floating_action_button_theme_data_mouse_cursor_when_specified()
     {
        test_widgets(|tester| {
            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            app_with_fab_theme(
                tester,
                FloatingActionButtonThemeData::new().mouse_cursor(<dyn WidgetStateProperty<
                    Option<MouseCursorRef>,
                >>::all(Some(
                    text.clone(),
                ))),
                add_button(),
            );

            tester.pump_and_settle();
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            let center = fab_rect(tester).center();
            tester.move_to(&mut mouse, center, false);
            tester.pump_and_settle();
            assert!(*tester.active_cursor(1) == *text);
        });
    }
}
