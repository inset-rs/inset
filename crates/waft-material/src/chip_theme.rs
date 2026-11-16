//! Flutter counterpart: material_ui's `chip_theme.dart`.

use waft_embedder::{Brightness, Color, lerp_double};
use waft_foundation::App;
use waft_painting::{
    AnyBorderSide, AnyColor, BorderSide, EdgeInsets, EdgeInsetsGeometry, OutlinedBorderRef,
    TextStyle,
};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, IconThemeData, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateBorderSide, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::Colors;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// Applies a chip theme to descendant [`Chip`](crate::Chip) widgets.
///
/// Descendant widgets obtain the current theme's [`ChipThemeData`] object using
/// [`ChipTheme::of`]. When a widget uses [`ChipTheme::of`], it is automatically rebuilt if
/// the theme later changes.
///
/// The [`ChipThemeData`] is also used as the default chip theme data in the overall
/// `ThemeData`, as `ThemeData::chip_theme`.
///
/// See also:
///
///  * [`ChipThemeData`], which describes the actual configuration of a chip theme.
///  * `ThemeData`, which describes the overall theme information for the application.
#[derive(Debug)]
pub struct ChipTheme {
    pub key: Option<KeyRef>,
    /// Specifies the color, shape, and text style values for descendant chip widgets.
    pub data: ChipThemeData,
    pub child: WidgetRef,
}

impl ChipTheme {
    /// Applies the given theme [`data`](Self::data) to [`child`](Self::child).
    pub fn new<K>(data: ChipThemeData, child: impl IntoWidget<K>) -> ChipTheme {
        ChipTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ChipTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ChipTheme {
        self.key = Some(key);
        self
    }

    /// Returns the [`data`](Self::data) from the closest [`ChipTheme`] ancestor. If there is
    /// no ancestor, it returns `ThemeData::chip_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ChipTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ChipThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ChipTheme>(app) {
            Some(inherited_theme) => inherited_theme.data.clone(),
            None => Theme::of(app, context).chip_theme.clone(),
        }
    }
}

impl InheritedTheme for ChipTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ChipTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ChipTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ChipTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Holds the color, shape, and text styles for a Material widget subtree.
///
/// [`ChipThemeData`] is used by [`ChipTheme`] to control the visual properties of chip
/// widgets in a widget hierarchy.
///
/// To obtain the current [`ChipThemeData`] for a given [`BuildContext`], use
/// [`ChipTheme::of`]. A widget's build method can use [`ChipTheme::of`] to obtain the
/// current visual properties for a chip.
///
/// The `ChipTheme` constructor is used to apply a [`ChipThemeData`] to a subtree, and
/// `ThemeData::chip_theme` is the default that applies when there is no [`ChipTheme`].
///
/// See also:
///
///  * [`ChipTheme`], a widget that sets the theme data for a subtree.
///  * `ThemeData`, which describes the overall theme information for the application.
#[derive(Clone, Debug, Default)]
pub struct ChipThemeData {
    /// Overrides the default for `ChipAttributes::color`.
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Color to be used for the unselected, enabled chip's background.
    ///
    /// The default is light grey.
    pub background_color: Option<Color>,

    /// The [`Color`] for the delete icon. The default is `Color(0xde000000)` (slightly
    /// transparent black) for light themes, and `Color(0xdeffffff)` (slightly transparent
    /// white) for dark themes.
    ///
    /// May be set to null, in which case the ambient `IconTheme.color` is used.
    pub delete_icon_color: Option<AnyColor>,

    /// Color to be used for the chip's background, indicating that it is disabled.
    ///
    /// The chip is disabled when `DisabledChipAttributes::is_enabled` is false, or all three
    /// of `SelectableChipAttributes::on_selected`, `TappableChipAttributes::on_pressed`, and
    /// `DeletableChipAttributes::on_deleted` are null.
    pub disabled_color: Option<Color>,

    /// Color to be used for the chip's background, indicating that it is selected.
    ///
    /// The chip is selected when `SelectableChipAttributes::selected` is true.
    pub selected_color: Option<Color>,

    /// An alternate color to be used for the chip's background, indicating that it is
    /// selected. For example, this color can be used for a `ChoiceChip` when selected.
    pub secondary_selected_color: Option<Color>,

    /// Color of the chip's shadow when the elevation is greater than 0.
    pub shadow_color: Option<Color>,

    /// Color of the chip's surface tint overlay when its elevation is greater than 0.
    ///
    /// This is not recommended for use. Material 3 spec introduced a set of tone-based
    /// surfaces and surface containers in its `ColorScheme`, which provide more flexibility.
    /// The intention is to eventually remove surface tint color from the framework.
    pub surface_tint_color: Option<Color>,

    /// Color of the chip's shadow when the elevation is greater than 0 and the chip is
    /// selected.
    pub selected_shadow_color: Option<Color>,

    /// Whether or not to show a check mark when
    /// `SelectableChipAttributes::selected` is true.
    pub show_checkmark: Option<bool>,

    /// Color of the chip's check mark when a check mark is visible.
    ///
    /// This will override the color set by the platform's brightness setting.
    pub checkmark_color: Option<Color>,

    /// The padding around the [`label`](crate::ChipAttributes::label) widget.
    ///
    /// By default, this is 4 logical pixels at the beginning and the end of the label, and
    /// zero on top and bottom.
    pub label_padding: Option<EdgeInsetsGeometry>,

    /// The padding between the contents of the chip and the outside
    /// [`shape`](Self::shape).
    ///
    /// Defaults to 4 logical pixels on all sides.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The color and weight of the chip's outline.
    ///
    /// This value is combined with [`shape`](Self::shape) to create a shape decorated with
    /// an outline. To omit the outline entirely, pass [`BorderSide::NONE`] to
    /// [`side`](Self::side).
    pub side: Option<AnyBorderSide>,

    /// The border to draw around the chip.
    ///
    /// This shape is combined with [`side`](Self::side) to create a shape decorated with an
    /// outline.
    pub shape: Option<OutlinedBorderRef>,

    /// The style to be applied to the chip's label.
    ///
    /// This only has an effect on label widgets that respect the `DefaultTextStyle`, such as
    /// `Text`.
    pub label_style: Option<TextStyle>,

    /// An alternate style to be applied to the chip's label, indicating that it is selected.
    ///
    /// This only has an effect on label widgets that respect the `DefaultTextStyle`, such as
    /// `Text`.
    pub secondary_label_style: Option<TextStyle>,

    /// Overrides the default for the brightness of the chip.
    pub brightness: Option<Brightness>,

    /// The elevation to be applied to the chip. Defaults to 0.
    pub elevation: Option<f64>,

    /// The elevation to be applied to the chip during the press motion. Defaults to 8.
    pub press_elevation: Option<f64>,

    /// Theme used for all icons in the chip.
    pub icon_theme: Option<IconThemeData>,

    /// Optional size constraints for the avatar.
    ///
    /// When unspecified, defaults to a minimum size of chip height or label height
    /// (whichever is greater) and a padding of 8.0 pixels on all sides.
    pub avatar_box_constraints: Option<BoxConstraints>,

    /// Optional size constraints for the delete icon.
    ///
    /// When unspecified, defaults to a minimum size of chip height or label height
    /// (whichever is greater) and a padding of 8.0 pixels on all sides.
    pub delete_icon_box_constraints: Option<BoxConstraints>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `ChipThemeData`: the theme data answers with its fields, and a
    /// defaults subclass (Dart's `_ChipDefaultsM3(context, isEnabled)` and its kin) computes
    /// the ones it overrides when they are read.
    pub trait ChipThemeDataGetters for ChipThemeData, fields: chip_theme_data {
        color: Option<WidgetStatePropertyRef<Option<Color>>>,
        background_color: Option<Color>,
        delete_icon_color: Option<AnyColor>,
        disabled_color: Option<Color>,
        selected_color: Option<Color>,
        secondary_selected_color: Option<Color>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        selected_shadow_color: Option<Color>,
        show_checkmark: Option<bool>,
        checkmark_color: Option<Color>,
        label_padding: Option<EdgeInsetsGeometry>,
        padding: Option<EdgeInsetsGeometry>,
        side: Option<AnyBorderSide>,
        shape: Option<OutlinedBorderRef>,
        label_style: Option<TextStyle>,
        secondary_label_style: Option<TextStyle>,
        brightness: Option<Brightness>,
        elevation: Option<f64>,
        press_elevation: Option<f64>,
        icon_theme: Option<IconThemeData>,
        avatar_box_constraints: Option<BoxConstraints>,
        delete_icon_box_constraints: Option<BoxConstraints>,
    }
}

impl ChipThemeData {
    /// Create a [`ChipThemeData`] given a set of exact values. Dart's optional named
    /// arguments are the fluent setters.
    ///
    /// This will rarely be used directly. It is used by [`lerp`](Self::lerp) to create
    /// intermediate themes based on two themes.
    pub fn new() -> ChipThemeData {
        ChipThemeData::default()
    }

    /// Dart's `ChipThemeData.fromDefaults`: generates a [`ChipThemeData`] from a brightness,
    /// a primary color, a secondary color, and a text style.
    ///
    /// Only one of `primary_color` or `brightness` may be given, and one of them must be:
    /// `brightness` is derived from `primary_color` when that is given, and the primary color
    /// is black or white from `brightness` when it is not.
    ///
    /// The `secondary_color` is used for the selection state. The colors, opacities and
    /// padding of the result are Material Design defaults.
    pub fn from_defaults(
        brightness: Option<Brightness>,
        primary_color: Option<Color>,
        secondary_color: Color,
        label_style: TextStyle,
    ) -> ChipThemeData {
        debug_assert!(
            primary_color.is_some() || brightness.is_some(),
            "One of primaryColor or brightness must be specified"
        );
        debug_assert!(
            primary_color.is_none() || brightness.is_none(),
            "Only one of primaryColor or brightness may be specified"
        );

        let brightness = match primary_color {
            Some(primary_color) => Some(ThemeData::estimate_brightness_for_color(primary_color)),
            None => brightness,
        };

        // These are Material Design defaults, and are used to derive
        // component Colors (with opacity) from base colors.
        const BACKGROUND_ALPHA: i32 = 0x1f; // 12%
        const DELETE_ICON_ALPHA: i32 = 0xde; // 87%
        const DISABLED_ALPHA: i32 = 0x0c; // 38% * 12% = 5%
        const SELECT_ALPHA: i32 = 0x3d; // 12% + 12% = 24%
        const TEXT_LABEL_ALPHA: i32 = 0xde; // 87%
        let padding = EdgeInsetsGeometry::from(EdgeInsets::all(4.0));

        let primary_color = primary_color.unwrap_or(if brightness == Some(Brightness::Light) {
            Colors::BLACK
        } else {
            Colors::WHITE
        });
        let background_color = primary_color.with_alpha(BACKGROUND_ALPHA);
        let delete_icon_color = primary_color.with_alpha(DELETE_ICON_ALPHA);
        let disabled_color = primary_color.with_alpha(DISABLED_ALPHA);
        let selected_color = primary_color.with_alpha(SELECT_ALPHA);
        let secondary_selected_color = secondary_color.with_alpha(SELECT_ALPHA);
        let secondary_label_style = label_style
            .copy_with()
            .color(secondary_color.with_alpha(TEXT_LABEL_ALPHA));
        let label_style = label_style
            .copy_with()
            .color(primary_color.with_alpha(TEXT_LABEL_ALPHA));

        ChipThemeData::new()
            .background_color(background_color)
            .delete_icon_color(delete_icon_color)
            .disabled_color(disabled_color)
            .selected_color(selected_color)
            .secondary_selected_color(secondary_selected_color)
            .shadow_color(Colors::BLACK)
            .selected_shadow_color(Colors::BLACK)
            .show_checkmark(true)
            .padding(padding)
            .label_style(label_style)
            .secondary_label_style(secondary_label_style)
            .brightness_value(brightness)
            .elevation(0.0)
            .press_elevation(8.0)
            .icon_theme(IconThemeData::new().size(18.0))
    }

    /// Dart `ChipThemeData(color:)`.
    pub fn color(mut self, color: WidgetStatePropertyRef<Option<Color>>) -> ChipThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `ChipThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> ChipThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `ChipThemeData(deleteIconColor:)`.
    pub fn delete_icon_color(mut self, delete_icon_color: impl Into<AnyColor>) -> ChipThemeData {
        self.delete_icon_color = Some(delete_icon_color.into());
        self
    }

    /// Dart `ChipThemeData(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ChipThemeData {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ChipThemeData(selectedColor:)`.
    pub fn selected_color(mut self, selected_color: Color) -> ChipThemeData {
        self.selected_color = Some(selected_color);
        self
    }

    /// Dart `ChipThemeData(secondarySelectedColor:)`.
    pub fn secondary_selected_color(mut self, secondary_selected_color: Color) -> ChipThemeData {
        self.secondary_selected_color = Some(secondary_selected_color);
        self
    }

    /// Dart `ChipThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> ChipThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `ChipThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> ChipThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `ChipThemeData(selectedShadowColor:)`.
    pub fn selected_shadow_color(mut self, selected_shadow_color: Color) -> ChipThemeData {
        self.selected_shadow_color = Some(selected_shadow_color);
        self
    }

    /// Dart `ChipThemeData(showCheckmark:)`.
    pub fn show_checkmark(mut self, show_checkmark: bool) -> ChipThemeData {
        self.show_checkmark = Some(show_checkmark);
        self
    }

    /// Dart `ChipThemeData(checkmarkColor:)`.
    pub fn checkmark_color(mut self, checkmark_color: Color) -> ChipThemeData {
        self.checkmark_color = Some(checkmark_color);
        self
    }

    /// Dart `ChipThemeData(labelPadding:)`.
    pub fn label_padding(mut self, label_padding: impl Into<EdgeInsetsGeometry>) -> ChipThemeData {
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Dart `ChipThemeData(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> ChipThemeData {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `ChipThemeData(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> ChipThemeData {
        self.side = Some(side.into());
        self
    }

    /// Dart `ChipThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> ChipThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `ChipThemeData(labelStyle:)`.
    pub fn label_style(mut self, label_style: TextStyle) -> ChipThemeData {
        self.label_style = Some(label_style);
        self
    }

    /// Dart `ChipThemeData(secondaryLabelStyle:)`.
    pub fn secondary_label_style(mut self, secondary_label_style: TextStyle) -> ChipThemeData {
        self.secondary_label_style = Some(secondary_label_style);
        self
    }

    /// Dart `ChipThemeData(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> ChipThemeData {
        self.brightness = Some(brightness);
        self
    }

    /// [`brightness`](Self::brightness) from a value that may be absent, as
    /// [`from_defaults`](Self::from_defaults) derives it.
    fn brightness_value(mut self, brightness: Option<Brightness>) -> ChipThemeData {
        self.brightness = brightness;
        self
    }

    /// Dart `ChipThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> ChipThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `ChipThemeData(pressElevation:)`.
    pub fn press_elevation(mut self, press_elevation: f64) -> ChipThemeData {
        self.press_elevation = Some(press_elevation);
        self
    }

    /// Dart `ChipThemeData(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> ChipThemeData {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `ChipThemeData(avatarBoxConstraints:)`.
    pub fn avatar_box_constraints(
        mut self,
        avatar_box_constraints: BoxConstraints,
    ) -> ChipThemeData {
        self.avatar_box_constraints = Some(avatar_box_constraints);
        self
    }

    /// Dart `ChipThemeData(deleteIconBoxConstraints:)`.
    pub fn delete_icon_box_constraints(
        mut self,
        delete_icon_box_constraints: BoxConstraints,
    ) -> ChipThemeData {
        self.delete_icon_box_constraints = Some(delete_icon_box_constraints);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new
    /// values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(4.0)`).
    pub fn copy_with(&self) -> ChipThemeData {
        self.clone()
    }

    /// Linearly interpolate between two chip themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&ChipThemeData>,
        b: Option<&ChipThemeData>,
        t: f64,
    ) -> Option<ChipThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ChipThemeData {
            color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.color.clone()),
                b.and_then(|b| b.color.clone()),
                t,
                Color::lerp,
            ),
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            delete_icon_color: Color::lerp(
                a.and_then(|a| a.delete_icon_color.as_ref().map(AnyColor::color)),
                b.and_then(|b| b.delete_icon_color.as_ref().map(AnyColor::color)),
                t,
            )
            .map(AnyColor::from),
            disabled_color: Color::lerp(
                a.and_then(|a| a.disabled_color),
                b.and_then(|b| b.disabled_color),
                t,
            ),
            selected_color: Color::lerp(
                a.and_then(|a| a.selected_color),
                b.and_then(|b| b.selected_color),
                t,
            ),
            secondary_selected_color: Color::lerp(
                a.and_then(|a| a.secondary_selected_color),
                b.and_then(|b| b.secondary_selected_color),
                t,
            ),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            selected_shadow_color: Color::lerp(
                a.and_then(|a| a.selected_shadow_color),
                b.and_then(|b| b.selected_shadow_color),
                t,
            ),
            show_checkmark: Some(if t < 0.5 {
                a.and_then(|a| a.show_checkmark).unwrap_or(true)
            } else {
                b.and_then(|b| b.show_checkmark).unwrap_or(true)
            }),
            checkmark_color: Color::lerp(
                a.and_then(|a| a.checkmark_color),
                b.and_then(|b| b.checkmark_color),
                t,
            ),
            label_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.label_padding),
                b.and_then(|b| b.label_padding),
                t,
            ),
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            side: Self::lerp_sides(
                a.and_then(|a| a.side.as_ref()),
                b.and_then(|b| b.side.as_ref()),
                t,
            )
            .map(AnyBorderSide::new),
            shape: OutlinedBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            label_style: TextStyle::lerp(
                a.and_then(|a| a.label_style.as_ref()),
                b.and_then(|b| b.label_style.as_ref()),
                t,
            ),
            secondary_label_style: TextStyle::lerp(
                a.and_then(|a| a.secondary_label_style.as_ref()),
                b.and_then(|b| b.secondary_label_style.as_ref()),
                t,
            ),
            brightness: Some(if t < 0.5 {
                a.and_then(|a| a.brightness).unwrap_or(Brightness::Light)
            } else {
                b.and_then(|b| b.brightness).unwrap_or(Brightness::Light)
            }),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            press_elevation: lerp_double(
                a.and_then(|a| a.press_elevation),
                b.and_then(|b| b.press_elevation),
                t,
            ),
            icon_theme: if a.is_some_and(|a| a.icon_theme.is_some())
                || b.is_some_and(|b| b.icon_theme.is_some())
            {
                Some(IconThemeData::lerp(
                    a.and_then(|a| a.icon_theme.as_ref()),
                    b.and_then(|b| b.icon_theme.as_ref()),
                    t,
                ))
            } else {
                None
            },
            avatar_box_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.avatar_box_constraints),
                b.and_then(|b| b.avatar_box_constraints),
                t,
            ),
            delete_icon_box_constraints: BoxConstraints::lerp(
                a.and_then(|a| a.delete_icon_box_constraints),
                b.and_then(|b| b.delete_icon_box_constraints),
                t,
            ),
        })
    }

    // Special case because BorderSide.lerp() doesn't support null arguments.
    fn lerp_sides(
        a: Option<&AnyBorderSide>,
        b: Option<&AnyBorderSide>,
        t: f64,
    ) -> Option<BorderSide> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.and_then(Self::resolve_unconditionally);
        let b = b.and_then(Self::resolve_unconditionally);
        let a = a.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: b.expect("b! in Dart").color.with_alpha(0),
            ..BorderSide::default()
        });
        let b = b.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: a.color.with_alpha(0),
            ..BorderSide::default()
        });

        Some(BorderSide::lerp(a, b, t))
    }

    /// Dart's `if (a is WidgetStateBorderSide) { a = a.resolve(const <WidgetState>{}); }`.
    fn resolve_unconditionally(side: &AnyBorderSide) -> Option<BorderSide> {
        match side.extension::<WidgetStateBorderSide>() {
            Some(side) => side.resolve(&WidgetStates::new()),
            None => Some(**side),
        }
    }
}

impl PartialEq for ChipThemeData {
    fn eq(&self, other: &ChipThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.color == self.color
            && other.background_color == self.background_color
            && other.delete_icon_color == self.delete_icon_color
            && other.disabled_color == self.disabled_color
            && other.selected_color == self.selected_color
            && other.secondary_selected_color == self.secondary_selected_color
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.selected_shadow_color == self.selected_shadow_color
            && other.show_checkmark == self.show_checkmark
            && other.checkmark_color == self.checkmark_color
            && other.label_padding == self.label_padding
            && other.padding == self.padding
            && other.side == self.side
            && other.shape == self.shape
            && other.label_style == self.label_style
            && other.secondary_label_style == self.secondary_label_style
            && other.brightness == self.brightness
            && other.elevation == self.elevation
            && other.press_elevation == self.press_elevation
            && other.icon_theme == self.icon_theme
            && other.avatar_box_constraints == self.avatar_box_constraints
            && other.delete_icon_box_constraints == self.delete_icon_box_constraints
    }
}

#[cfg(test)]
mod tests {
    use crate::choice_chip::ChoiceChip;
    use crate::color_scheme::ColorScheme;
    use crate::scaffold::Scaffold;
    use std::cell::Cell;
    use std::time::Duration;
    use waft_embedder::Offset;
    use waft_foundation::Listener;
    use waft_painting::{BeveledRectangleBorder, BorderSide, StadiumBorder};
    use waft_widgets::{
        Focus, FocusNode, WidgetState, WidgetStateColor, WidgetStateOutlinedBorder,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};

    use super::*;
    use crate::test_support::{PaintInspection, is_same_color_as};

    fn fallback_body_large(tester: &mut WidgetTester, color: Color) -> TextStyle {
        let theme_data = ThemeData::fallback(&tester.cell.borrow(), None);
        theme_data
            .text_theme
            .body_large
            .expect("the fallback has bodyLarge")
            .copy_with()
            .color(color)
    }

    fn label_color(data: &ChipThemeData) -> Option<Color> {
        data.label_style
            .as_ref()
            .and_then(|style| style.color.as_ref())
            .map(|color| color.color())
    }

    fn secondary_label_color(data: &ChipThemeData) -> Option<Color> {
        data.secondary_label_style
            .as_ref()
            .and_then(|style| style.color.as_ref())
            .map(|color| color.color())
    }

    fn same(actual: Option<Color>, expected: Color) {
        let actual = actual.expect("a color");
        assert!(
            is_same_color_as(actual, expected),
            "{actual:?} is not the same color as {expected:?}"
        );
    }

    #[test]
    fn chip_theme_data_copy_with_equals_and_hash_code_basics() {
        assert_eq!(ChipThemeData::new(), ChipThemeData::new().copy_with());
    }

    #[test]
    fn chip_theme_data_lerp_special_cases() {
        assert_eq!(ChipThemeData::lerp(None, None, 0.0), None);
        let data = ChipThemeData::new();
        assert_eq!(
            ChipThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn chip_theme_data_defaults() {
        let data = ChipThemeData::new();
        assert!(data.color.is_none());
        assert_eq!(data.background_color, None);
        assert_eq!(data.delete_icon_color, None);
        assert_eq!(data.disabled_color, None);
        assert_eq!(data.selected_color, None);
        assert_eq!(data.secondary_selected_color, None);
        assert_eq!(data.shadow_color, None);
        assert_eq!(data.surface_tint_color, None);
        assert_eq!(data.selected_shadow_color, None);
        assert_eq!(data.show_checkmark, None);
        assert_eq!(data.checkmark_color, None);
        assert_eq!(data.label_padding, None);
        assert_eq!(data.padding, None);
        assert!(data.side.is_none());
        assert!(data.shape.is_none());
        assert_eq!(data.label_style, None);
        assert_eq!(data.secondary_label_style, None);
        assert_eq!(data.brightness, None);
        assert_eq!(data.elevation, None);
        assert_eq!(data.press_elevation, None);
        assert_eq!(data.icon_theme, None);
        assert_eq!(data.avatar_box_constraints, None);
        assert_eq!(data.delete_icon_box_constraints, None);
    }

    #[test]
    fn chip_theme_from_defaults() {
        let label_style = TextStyle::new();
        let chip_theme = ChipThemeData::from_defaults(
            Some(Brightness::Light),
            None,
            Colors::RED.primary(),
            label_style.clone(),
        );
        assert_eq!(
            chip_theme.background_color,
            Some(Colors::BLACK.with_alpha(0x1f))
        );
        assert_eq!(
            chip_theme.delete_icon_color,
            Some(Colors::BLACK.with_alpha(0xde).into())
        );
        assert_eq!(
            chip_theme.disabled_color,
            Some(Colors::BLACK.with_alpha(0x0c))
        );
        assert_eq!(
            chip_theme.selected_color,
            Some(Colors::BLACK.with_alpha(0x3d))
        );
        assert_eq!(
            chip_theme.secondary_selected_color,
            Some(Colors::RED.primary().with_alpha(0x3d))
        );
        assert_eq!(chip_theme.shadow_color, Some(Colors::BLACK));
        assert_eq!(chip_theme.surface_tint_color, None);
        assert_eq!(chip_theme.selected_shadow_color, Some(Colors::BLACK));
        assert_eq!(chip_theme.show_checkmark, Some(true));
        assert_eq!(chip_theme.checkmark_color, None);
        assert_eq!(chip_theme.label_padding, None);
        assert_eq!(chip_theme.padding, Some(EdgeInsets::all(4.0).into()));
        assert!(chip_theme.side.is_none());
        assert!(chip_theme.shape.is_none());
        assert_eq!(
            chip_theme.label_style,
            Some(
                label_style
                    .copy_with()
                    .color(Colors::BLACK.with_alpha(0xde))
            )
        );
        assert_eq!(
            chip_theme.secondary_label_style,
            Some(
                label_style
                    .copy_with()
                    .color(Colors::RED.primary().with_alpha(0xde))
            )
        );
        assert_eq!(chip_theme.brightness, Some(Brightness::Light));
        assert_eq!(chip_theme.elevation, Some(0.0));
        assert_eq!(chip_theme.press_elevation, Some(8.0));

        let teal = Colors::TEAL_ACCENT.shade200();
        let chip_theme =
            ChipThemeData::from_defaults(Some(Brightness::Dark), None, teal, TextStyle::new());
        assert_eq!(
            chip_theme.background_color,
            Some(Colors::WHITE.with_alpha(0x1f))
        );
        assert_eq!(
            chip_theme.delete_icon_color,
            Some(Colors::WHITE.with_alpha(0xde).into())
        );
        assert_eq!(
            chip_theme.disabled_color,
            Some(Colors::WHITE.with_alpha(0x0c))
        );
        assert_eq!(
            chip_theme.selected_color,
            Some(Colors::WHITE.with_alpha(0x3d))
        );
        assert_eq!(
            chip_theme.secondary_selected_color,
            Some(teal.with_alpha(0x3d))
        );
        assert_eq!(chip_theme.shadow_color, Some(Colors::BLACK));
        assert_eq!(chip_theme.selected_shadow_color, Some(Colors::BLACK));
        assert_eq!(chip_theme.show_checkmark, Some(true));
        assert_eq!(chip_theme.checkmark_color, None);
        assert_eq!(chip_theme.label_padding, None);
        assert_eq!(chip_theme.padding, Some(EdgeInsets::all(4.0).into()));
        assert!(chip_theme.side.is_none());
        assert!(chip_theme.shape.is_none());
        assert_eq!(
            chip_theme.label_style,
            Some(
                label_style
                    .copy_with()
                    .color(Colors::WHITE.with_alpha(0xde))
            )
        );
        assert_eq!(
            chip_theme.secondary_label_style,
            Some(label_style.copy_with().color(teal.with_alpha(0xde)))
        );
        assert_eq!(chip_theme.brightness, Some(Brightness::Dark));
        assert_eq!(chip_theme.elevation, Some(0.0));
        assert_eq!(chip_theme.press_elevation, Some(8.0));
    }

    #[test]
    fn chip_theme_data_generates_correct_opacities_for_defaults() {
        test_widgets(|tester| {
            let custom_color1 = Color::new(0xcafefeed);
            let custom_color2 = Color::new(0xdeadbeef);
            let custom_style = fallback_body_large(tester, custom_color2);

            let light_theme = ChipThemeData::from_defaults(
                Some(Brightness::Light),
                None,
                custom_color1,
                custom_style.clone(),
            );
            assert_eq!(
                light_theme.background_color,
                Some(Colors::BLACK.with_alpha(0x1f))
            );
            assert_eq!(
                light_theme.delete_icon_color,
                Some(Colors::BLACK.with_alpha(0xde).into())
            );
            assert_eq!(
                light_theme.disabled_color,
                Some(Colors::BLACK.with_alpha(0x0c))
            );
            assert_eq!(
                light_theme.selected_color,
                Some(Colors::BLACK.with_alpha(0x3d))
            );
            assert_eq!(
                light_theme.secondary_selected_color,
                Some(custom_color1.with_alpha(0x3d))
            );
            assert_eq!(light_theme.label_padding, None);
            assert_eq!(light_theme.padding, Some(EdgeInsets::all(4.0).into()));
            assert!(light_theme.side.is_none());
            assert!(light_theme.shape.is_none());
            assert_eq!(
                label_color(&light_theme),
                Some(Colors::BLACK.with_alpha(0xde))
            );
            assert_eq!(
                secondary_label_color(&light_theme),
                Some(custom_color1.with_alpha(0xde))
            );
            assert_eq!(light_theme.brightness, Some(Brightness::Light));

            let dark_theme = ChipThemeData::from_defaults(
                Some(Brightness::Dark),
                None,
                custom_color1,
                custom_style.clone(),
            );
            assert_eq!(
                dark_theme.background_color,
                Some(Colors::WHITE.with_alpha(0x1f))
            );
            assert_eq!(
                dark_theme.delete_icon_color,
                Some(Colors::WHITE.with_alpha(0xde).into())
            );
            assert_eq!(
                dark_theme.disabled_color,
                Some(Colors::WHITE.with_alpha(0x0c))
            );
            assert_eq!(
                dark_theme.selected_color,
                Some(Colors::WHITE.with_alpha(0x3d))
            );
            assert_eq!(
                dark_theme.secondary_selected_color,
                Some(custom_color1.with_alpha(0x3d))
            );
            assert_eq!(dark_theme.label_padding, None);
            assert_eq!(dark_theme.padding, Some(EdgeInsets::all(4.0).into()));
            assert!(dark_theme.side.is_none());
            assert!(dark_theme.shape.is_none());
            assert_eq!(
                label_color(&dark_theme),
                Some(Colors::WHITE.with_alpha(0xde))
            );
            assert_eq!(
                secondary_label_color(&dark_theme),
                Some(custom_color1.with_alpha(0xde))
            );
            assert_eq!(dark_theme.brightness, Some(Brightness::Dark));

            let custom_theme = ChipThemeData::from_defaults(
                None,
                Some(custom_color1),
                custom_color2,
                custom_style,
            );
            assert_eq!(
                custom_theme.delete_icon_color,
                Some(custom_color1.with_alpha(0xde).into())
            );
            assert_eq!(
                custom_theme.disabled_color,
                Some(custom_color1.with_alpha(0x0c))
            );
            assert_eq!(
                custom_theme.selected_color,
                Some(custom_color1.with_alpha(0x3d))
            );
            assert_eq!(
                custom_theme.secondary_selected_color,
                Some(custom_color2.with_alpha(0x3d))
            );
            assert_eq!(custom_theme.label_padding, None);
            assert_eq!(custom_theme.padding, Some(EdgeInsets::all(4.0).into()));
            assert!(custom_theme.side.is_none());
            assert!(custom_theme.shape.is_none());
            assert_eq!(
                label_color(&custom_theme),
                Some(custom_color1.with_alpha(0xde))
            );
            assert_eq!(
                secondary_label_color(&custom_theme),
                Some(custom_color2.with_alpha(0xde))
            );
            assert_eq!(custom_theme.brightness, Some(Brightness::Light));
        });
    }

    fn black_chip_theme(tester: &mut WidgetTester) -> ChipThemeData {
        ChipThemeData::from_defaults(
            Some(Brightness::Dark),
            None,
            Colors::BLACK,
            fallback_body_large(tester, Colors::BLACK),
        )
        .elevation(1.0)
        .label_padding(EdgeInsets::symmetric(0.0, 8.0))
        .shape(StadiumBorder::new(BorderSide::NONE))
        .side(BorderSide::default())
        .press_elevation(4.0)
        .shadow_color(Colors::BLACK)
        .surface_tint_color(Colors::BLACK)
        .selected_shadow_color(Colors::BLACK)
        .show_checkmark(false)
        .checkmark_color(Colors::BLACK)
        .icon_theme(IconThemeData::new().size(26.0))
    }

    fn white_chip_theme(tester: &mut WidgetTester) -> ChipThemeData {
        ChipThemeData::from_defaults(
            Some(Brightness::Light),
            None,
            Colors::WHITE,
            fallback_body_large(tester, Colors::WHITE),
        )
        .padding(EdgeInsets::all(2.0))
        .label_padding(EdgeInsets::only(0.0, 8.0, 0.0, 8.0))
        .shape(BeveledRectangleBorder::default())
        .side(BorderSide {
            color: Colors::WHITE,
            ..BorderSide::default()
        })
        .elevation(5.0)
        .press_elevation(10.0)
        .shadow_color(Colors::WHITE)
        .surface_tint_color(Colors::WHITE)
        .selected_shadow_color(Colors::WHITE)
        .show_checkmark(true)
        .checkmark_color(Colors::WHITE)
        .icon_theme(IconThemeData::new().size(22.0))
    }

    fn is_beveled(data: &ChipThemeData) -> bool {
        data.shape.as_ref().is_some_and(|shape| {
            shape
                .as_any()
                .downcast_ref::<BeveledRectangleBorder>()
                .is_some()
        })
    }

    fn is_stadium(data: &ChipThemeData) -> bool {
        data.shape
            .as_ref()
            .is_some_and(|shape| shape.as_any().downcast_ref::<StadiumBorder>().is_some())
    }

    fn side_color(data: &ChipThemeData) -> Option<Color> {
        data.side.as_ref().map(|side| side.value().color)
    }

    #[test]
    fn chip_theme_data_lerps_correctly() {
        test_widgets(|tester| {
            let black = black_chip_theme(tester);
            let white = white_chip_theme(tester);

            let lerp = ChipThemeData::lerp(Some(&black), Some(&white), 0.5).expect("a theme");
            let middle_grey = Color::new(0xff7f7f7f);
            same(lerp.background_color, middle_grey.with_alpha(0x1f));
            same(
                lerp.delete_icon_color.as_ref().map(AnyColor::color),
                middle_grey.with_alpha(0xde),
            );
            same(lerp.disabled_color, middle_grey.with_alpha(0x0c));
            same(lerp.selected_color, middle_grey.with_alpha(0x3d));
            same(lerp.secondary_selected_color, middle_grey.with_alpha(0x3d));
            same(lerp.shadow_color, middle_grey);
            same(lerp.surface_tint_color, middle_grey);
            same(lerp.selected_shadow_color, middle_grey);
            assert_eq!(lerp.show_checkmark, Some(true));
            assert_eq!(lerp.label_padding, Some(EdgeInsets::all(4.0).into()));
            assert_eq!(lerp.padding, Some(EdgeInsets::all(3.0).into()));
            same(side_color(&lerp), middle_grey);
            assert!(is_beveled(&lerp));
            same(label_color(&lerp), middle_grey.with_alpha(0xde));
            same(secondary_label_color(&lerp), middle_grey.with_alpha(0xde));
            assert_eq!(lerp.brightness, Some(Brightness::Light));
            assert_eq!(lerp.elevation, Some(3.0));
            assert_eq!(lerp.press_elevation, Some(7.0));
            same(lerp.checkmark_color, middle_grey);
            assert_eq!(lerp.icon_theme, Some(IconThemeData::new().size(24.0)));

            assert_eq!(ChipThemeData::lerp(None, None, 0.25), None);

            let a_null_25 = ChipThemeData::lerp(None, Some(&white), 0.25).expect("a theme");
            same(a_null_25.background_color, Colors::BLACK.with_alpha(0x08));
            same(
                a_null_25.delete_icon_color.as_ref().map(AnyColor::color),
                Colors::BLACK.with_alpha(0x38),
            );
            same(a_null_25.disabled_color, Colors::BLACK.with_alpha(0x03));
            same(a_null_25.selected_color, Colors::BLACK.with_alpha(0x0f));
            same(
                a_null_25.secondary_selected_color,
                Colors::WHITE.with_alpha(0x0f),
            );
            same(a_null_25.shadow_color, Colors::WHITE.with_alpha(0x40));
            same(a_null_25.surface_tint_color, Colors::WHITE.with_alpha(0x40));
            same(
                a_null_25.selected_shadow_color,
                Colors::WHITE.with_alpha(0x40),
            );
            assert_eq!(a_null_25.show_checkmark, Some(true));
            assert_eq!(
                a_null_25.label_padding,
                Some(EdgeInsets::only(0.0, 2.0, 0.0, 2.0).into())
            );
            assert_eq!(a_null_25.padding, Some(EdgeInsets::all(0.5).into()));
            same(side_color(&a_null_25), Colors::WHITE.with_alpha(0x3f));
            assert!(is_beveled(&a_null_25));
            same(label_color(&a_null_25), Colors::BLACK.with_alpha(0x38));
            same(
                secondary_label_color(&a_null_25),
                Colors::WHITE.with_alpha(0x38),
            );
            assert_eq!(a_null_25.brightness, Some(Brightness::Light));
            assert_eq!(a_null_25.elevation, Some(1.25));
            assert_eq!(a_null_25.press_elevation, Some(2.5));
            same(a_null_25.checkmark_color, Colors::WHITE.with_alpha(0x40));
            assert_eq!(a_null_25.icon_theme, Some(IconThemeData::new().size(5.5)));

            let a_null_75 = ChipThemeData::lerp(None, Some(&white), 0.75).expect("a theme");
            same(a_null_75.background_color, Colors::BLACK.with_alpha(0x17));
            same(
                a_null_75.delete_icon_color.as_ref().map(AnyColor::color),
                Colors::BLACK.with_alpha(0xa7),
            );
            same(a_null_75.disabled_color, Colors::BLACK.with_alpha(0x09));
            same(a_null_75.selected_color, Colors::BLACK.with_alpha(0x2e));
            same(
                a_null_75.secondary_selected_color,
                Colors::WHITE.with_alpha(0x2e),
            );
            same(a_null_75.shadow_color, Colors::WHITE.with_alpha(0xbf));
            same(a_null_75.surface_tint_color, Colors::WHITE.with_alpha(0xbf));
            same(
                a_null_75.selected_shadow_color,
                Colors::WHITE.with_alpha(0xbf),
            );
            assert_eq!(a_null_75.show_checkmark, Some(true));
            assert_eq!(
                a_null_75.label_padding,
                Some(EdgeInsets::only(0.0, 6.0, 0.0, 6.0).into())
            );
            assert_eq!(a_null_75.padding, Some(EdgeInsets::all(1.5).into()));
            same(side_color(&a_null_75), Colors::WHITE.with_alpha(0xbf));
            assert!(is_beveled(&a_null_75));
            same(label_color(&a_null_75), Colors::BLACK.with_alpha(0xa7));
            same(
                secondary_label_color(&a_null_75),
                Colors::WHITE.with_alpha(0xa7),
            );
            assert_eq!(a_null_75.brightness, Some(Brightness::Light));
            assert_eq!(a_null_75.elevation, Some(3.75));
            assert_eq!(a_null_75.press_elevation, Some(7.5));
            same(a_null_75.checkmark_color, Colors::WHITE.with_alpha(0xbf));
            assert_eq!(a_null_75.icon_theme, Some(IconThemeData::new().size(16.5)));

            let b_null_25 = ChipThemeData::lerp(Some(&black), None, 0.25).expect("a theme");
            same(b_null_25.background_color, Colors::WHITE.with_alpha(0x17));
            same(
                b_null_25.delete_icon_color.as_ref().map(AnyColor::color),
                Colors::WHITE.with_alpha(0xa7),
            );
            same(b_null_25.disabled_color, Colors::WHITE.with_alpha(0x09));
            same(b_null_25.selected_color, Colors::WHITE.with_alpha(0x2e));
            same(
                b_null_25.secondary_selected_color,
                Colors::BLACK.with_alpha(0x2e),
            );
            same(b_null_25.shadow_color, Colors::BLACK.with_alpha(0xbf));
            same(b_null_25.surface_tint_color, Colors::BLACK.with_alpha(0xbf));
            same(
                b_null_25.selected_shadow_color,
                Colors::BLACK.with_alpha(0xbf),
            );
            assert_eq!(b_null_25.show_checkmark, Some(false));
            assert_eq!(
                b_null_25.label_padding,
                Some(EdgeInsets::symmetric(0.0, 6.0).into())
            );
            assert_eq!(b_null_25.padding, Some(EdgeInsets::all(3.0).into()));
            same(side_color(&b_null_25), Colors::BLACK.with_alpha(0xbf));
            assert!(is_stadium(&b_null_25));
            same(label_color(&b_null_25), Colors::WHITE.with_alpha(0xa7));
            same(
                secondary_label_color(&b_null_25),
                Colors::BLACK.with_alpha(0xa7),
            );
            assert_eq!(b_null_25.brightness, Some(Brightness::Dark));
            assert_eq!(b_null_25.elevation, Some(0.75));
            assert_eq!(b_null_25.press_elevation, Some(3.0));
            same(b_null_25.checkmark_color, Colors::BLACK.with_alpha(0xbf));
            assert_eq!(b_null_25.icon_theme, Some(IconThemeData::new().size(19.5)));

            let b_null_75 = ChipThemeData::lerp(Some(&black), None, 0.75).expect("a theme");
            same(b_null_75.background_color, Colors::WHITE.with_alpha(0x08));
            same(
                b_null_75.delete_icon_color.as_ref().map(AnyColor::color),
                Colors::WHITE.with_alpha(0x38),
            );
            same(b_null_75.disabled_color, Colors::WHITE.with_alpha(0x03));
            same(b_null_75.selected_color, Colors::WHITE.with_alpha(0x0f));
            same(
                b_null_75.secondary_selected_color,
                Colors::BLACK.with_alpha(0x0f),
            );
            same(b_null_75.shadow_color, Colors::BLACK.with_alpha(0x40));
            same(b_null_75.surface_tint_color, Colors::BLACK.with_alpha(0x40));
            same(
                b_null_75.selected_shadow_color,
                Colors::BLACK.with_alpha(0x40),
            );
            assert_eq!(b_null_75.show_checkmark, Some(true));
            assert_eq!(
                b_null_75.label_padding,
                Some(EdgeInsets::symmetric(0.0, 2.0).into())
            );
            assert_eq!(b_null_75.padding, Some(EdgeInsets::all(1.0).into()));
            same(side_color(&b_null_75), Colors::BLACK.with_alpha(0x3f));
            assert!(is_stadium(&b_null_75));
            same(label_color(&b_null_75), Colors::WHITE.with_alpha(0x38));
            same(
                secondary_label_color(&b_null_75),
                Colors::BLACK.with_alpha(0x38),
            );
            assert_eq!(b_null_75.brightness, Some(Brightness::Light));
            assert_eq!(b_null_75.elevation, Some(0.25));
            assert_eq!(b_null_75.press_elevation, Some(1.0));
            same(b_null_75.checkmark_color, Colors::BLACK.with_alpha(0x40));
            assert_eq!(b_null_75.icon_theme, Some(IconThemeData::new().size(6.5)));
        });
    }

    // Dart's helpers of chip_theme_test.dart.

    use std::rc::Rc;

    use waft_embedder::Size;
    use waft_painting::{CircleBorder, RoundedRectangleBorder, ShapeBorderRef};
    use waft_widgets::{
        AnyElement, Builder, Center, Container, CustomPaint, DefaultTextStyle, Directionality,
        Icon, IconTheme, SizedBox, Text, TextDirection,
    };

    use crate::chip::{RawChip, tests::only};
    use crate::icons::Icons;
    use crate::material::Material;
    use waft_embedder::valo::Op;

    use crate::test_support::material_app;

    use waft_rendering::testing::{
        CanvasMethod, PaintCall, PaintedStyle, assert_paints, colors_match, paints,
    };
    fn pump_app(tester: &mut WidgetTester, theme: ThemeData, home: WidgetRef) {
        tester.pump_widget(material_app(theme, home));
    }

    fn theme_with(tester: &WidgetTester, chip_theme: ChipThemeData, material3: bool) -> ThemeData {
        ThemeData::builder()
            .use_material3(material3)
            .chip_theme(chip_theme)
            .build(&tester.cell.borrow())
    }

    /// Dart's `Directionality(textDirection: ltr, child: Material(child: Center(child: chip)))`.
    fn centered_material(chip: RawChip) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Material::new()
                .child(Center::new().child(chip))
                .into_widget(),
        )
        .into_widget()
    }

    fn avatar_chip() -> RawChip {
        RawChip::new(SizedBox::new().width(100.0).height(100.0))
            .avatar(Icon::new(Some(Icons::add())))
            .on_selected(Rc::new(|_app, _value| {}))
    }

    /// Dart's `getMaterialBox`: the first `CustomPaint` below the `RawChip`, with what it and
    /// everything under it draws, as Dart's `paints` records it.
    fn material_box_ops(tester: &WidgetTester) -> Vec<Op> {
        let custom_paint = tester.descendants_of_type::<CustomPaint>(only::<RawChip>(tester))[0];
        tester.painted_ops_deep(tester.render_box_of(custom_paint).as_object())
    }

    /// What the `RawChip` and everything under it draws.
    fn raw_chip_ops(tester: &WidgetTester) -> Vec<Op> {
        tester.painted_ops_deep(tester.render_box_of(only::<RawChip>(tester)).as_object())
    }

    /// Dart's `paints..rect(color:)`: the first `drawRect` is of `color`.
    fn paints_rect(ops: &[Op], color: Color) -> bool {
        ops.iter()
            .find(|op| CanvasMethod::of(op) == CanvasMethod::DrawRect)
            .is_some_and(
                |op| matches!(op, Op::DrawRect { paint, .. } if colors_match(paint.color, color)),
            )
    }

    fn material_of_chip(tester: &WidgetTester) -> AnyElement {
        tester.descendants_of_type::<Material>(only::<RawChip>(tester))[0]
    }

    fn material_elevation(tester: &WidgetTester) -> f64 {
        tester.widget(material_of_chip(tester), |material: &Material| {
            material.elevation
        })
    }

    fn material_shape(tester: &WidgetTester) -> Option<ShapeBorderRef> {
        tester.widget(material_of_chip(tester), |material: &Material| {
            material.shape.clone()
        })
    }

    fn icon_theme_data(tester: &WidgetTester) -> IconThemeData {
        let icon_theme = tester.descendants_of_type::<IconTheme>(only::<RawChip>(tester))[0];
        tester.widget(icon_theme, |icon_theme: &IconTheme| icon_theme.data.clone())
    }

    fn icon_color(tester: &WidgetTester) -> Option<Color> {
        icon_theme_data(tester).color.map(|color| color.color())
    }

    /// Dart's `getLabelStyle`: the last `DefaultTextStyle` below the `RawChip`.
    fn label_font_size(tester: &WidgetTester) -> Option<f64> {
        let styles = tester.descendants_of_type::<DefaultTextStyle>(only::<RawChip>(tester));
        let last = *styles.last().expect("a DefaultTextStyle");
        tester.widget(last, |style: &DefaultTextStyle| style.style.font_size)
    }

    fn rounded_rectangle_with_side(color: Color) -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::default().copy_with(
            Some(BorderSide {
                color,
                ..BorderSide::default()
            }),
            None,
        ))
    }

    fn chip_theme_for_overrides() -> ChipThemeData {
        ChipThemeData::new()
            .background_color(Color::new(0xff112233))
            .elevation(4.0)
            .padding(EdgeInsets::all(50.0))
            .label_padding(EdgeInsets::all(25.0))
            .shape(RoundedRectangleBorder::default())
            .label_style(TextStyle::new().font_size(32.0))
            .icon_theme(IconThemeData::new().color(Color::new(0xff332211)))
    }

    fn assert_chip_uses_theme(tester: &WidgetTester, size: Size, shape: ShapeBorderRef) {
        let chip_theme = chip_theme_for_overrides();
        assert!(paints_rect(
            &material_box_ops(tester),
            chip_theme.background_color.expect("a color")
        ));
        assert_eq!(material_elevation(tester), 4.0);
        assert_eq!(tester.get_size(only::<RawChip>(tester)), size); // label + padding + labelPadding
        assert_eq!(material_shape(tester), Some(shape));
        assert_eq!(label_font_size(tester), Some(32.0));
        assert_eq!(icon_color(tester), Some(Color::new(0xff332211)));
    }

    #[test]
    fn material3_chip_uses_theme_data_chip_theme() {
        test_widgets(|tester| {
            let theme = theme_with(tester, chip_theme_for_overrides(), true);
            let outline_variant = theme.color_scheme.outline_variant();
            pump_app(tester, theme, centered_material(avatar_chip()));
            assert_chip_uses_theme(
                tester,
                Size::new(402.0, 252.0),
                rounded_rectangle_with_side(outline_variant),
            );
        });
    }

    #[test]
    fn material2_chip_uses_theme_data_chip_theme() {
        test_widgets(|tester| {
            let theme = theme_with(tester, chip_theme_for_overrides(), false);
            pump_app(tester, theme, centered_material(avatar_chip()));
            assert_chip_uses_theme(
                tester,
                Size::new(400.0, 250.0),
                ShapeBorderRef::new(RoundedRectangleBorder::default()),
            );
        });
    }

    fn local_chip_theme_home(chip_theme: ChipThemeData) -> WidgetRef {
        ChipTheme::new(
            chip_theme,
            Builder::new(|_app, _context| centered_material(avatar_chip())),
        )
        .into_widget()
    }

    #[test]
    fn material3_chip_uses_local_chip_theme() {
        test_widgets(|tester| {
            let theme = theme_with(tester, ChipThemeData::new(), true);
            let outline_variant = theme.color_scheme.outline_variant();
            pump_app(
                tester,
                theme,
                local_chip_theme_home(chip_theme_for_overrides()),
            );
            assert_chip_uses_theme(
                tester,
                Size::new(402.0, 252.0),
                rounded_rectangle_with_side(outline_variant),
            );
        });
    }

    #[test]
    fn material2_chip_uses_local_chip_theme() {
        test_widgets(|tester| {
            let theme = theme_with(tester, ChipThemeData::new(), false);
            pump_app(
                tester,
                theme,
                local_chip_theme_home(chip_theme_for_overrides()),
            );
            assert_chip_uses_theme(
                tester,
                Size::new(400.0, 250.0),
                ShapeBorderRef::new(RoundedRectangleBorder::default()),
            );
        });
    }

    #[test]
    fn chip_properties_overrides_chip_theme() {
        test_widgets(|tester| {
            let background_color = Color::new(0xff000000);
            let elevation = 6.0;
            let shape = RoundedRectangleBorder::new(
                BorderSide {
                    color: Color::new(0xff0000ff),
                    ..BorderSide::default()
                },
                waft_painting::BorderRadius::ZERO.into(),
            );
            let icon_theme = IconThemeData::new().color(Color::new(0xff00ff00));
            let theme = theme_with(tester, chip_theme_for_overrides(), true);
            let chip = avatar_chip()
                .background_color(background_color)
                .elevation(elevation)
                .padding(EdgeInsets::all(10.0))
                .label_padding(EdgeInsets::all(5.0))
                .label_style(TextStyle::new().font_size(20.0))
                .shape(shape)
                .icon_theme(icon_theme.clone());
            pump_app(tester, theme, centered_material(chip));

            assert!(paints_rect(&material_box_ops(tester), background_color));
            assert_eq!(
                tester.get_size(only::<RawChip>(tester)),
                Size::new(242.0, 132.0)
            ); // label + padding + labelPadding
            assert_eq!(material_elevation(tester), elevation);
            assert_eq!(material_shape(tester), Some(ShapeBorderRef::new(shape)));
            assert_eq!(label_font_size(tester), Some(20.0));
            assert_eq!(icon_color(tester), Some(Color::new(0xff00ff00)));
        });
    }

    fn assert_chip_uses_constructor_parameters(
        tester: &WidgetTester,
        background_color: Color,
        size: Size,
        shape: CircleBorder,
        icon_color_expected: Color,
    ) {
        assert_paints(
            &material_box_ops(tester),
            &[PaintCall::circle(Some(background_color))],
        );
        assert_eq!(tester.get_size(only::<RawChip>(tester)), size); // label + padding + labelPadding
        assert_eq!(material_elevation(tester), 3.0);
        assert_eq!(material_shape(tester), Some(ShapeBorderRef::new(shape)));
        assert_eq!(label_font_size(tester), Some(32.0));
        assert_eq!(icon_color(tester), Some(icon_color_expected));
    }

    fn constructor_parameters_chip(background_color: Color, shape: CircleBorder) -> RawChip {
        avatar_chip()
            .background_color(background_color)
            .elevation(3.0)
            .padding(EdgeInsets::all(50.0))
            .label_padding(EdgeInsets::all(25.0))
            .label_style(TextStyle::new().font_size(32.0))
            .shape(shape)
            .icon_theme(IconThemeData::new().color(Color::new(0xff443322)))
    }

    #[test]
    fn material3_chip_uses_constructor_parameters() {
        test_widgets(|tester| {
            let background_color = Color::new(0xff332211);
            let shape = CircleBorder::new(
                BorderSide {
                    color: Color::new(0xff0000ff),
                    ..BorderSide::default()
                },
                0.0,
            );
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_app(
                tester,
                theme,
                centered_material(constructor_parameters_chip(background_color, shape)),
            );
            assert_chip_uses_constructor_parameters(
                tester,
                background_color,
                Size::new(402.0, 252.0),
                shape,
                Color::new(0xff443322),
            );
        });
    }

    #[test]
    fn material2_chip_uses_constructor_parameters() {
        test_widgets(|tester| {
            let background_color = Color::new(0xff332211);
            let shape = CircleBorder::default();
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            pump_app(
                tester,
                theme,
                centered_material(constructor_parameters_chip(background_color, shape)),
            );
            assert_chip_uses_constructor_parameters(
                tester,
                background_color,
                Size::new(400.0, 250.0),
                shape,
                Color::new(0xff443322),
            );
        });
    }

    fn selectable_choice_chip(selected: bool, enabled: bool) -> ChoiceChip {
        let mut chip = ChoiceChip::new(Text::new("Chip"), selected);
        if enabled {
            chip.on_selected = Some(Rc::new(|_app, _value| {}));
        }
        chip
    }

    fn scaffold_with<K>(chip: impl IntoWidget<K>) -> WidgetRef {
        Scaffold::new().body(chip).into_widget()
    }

    #[test]
    fn chip_uses_stateful_color_from_chip_theme() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyFocusNode>();

            let pressed_color = Color::new(0x00000001);
            let hover_color = Color::new(0x00000002);
            let focused_color = Color::new(0x00000003);
            let default_color = Color::new(0x00000004);
            let selected_color = Color::new(0x00000005);
            let disabled_color = Color::new(0x00000006);

            let text_color_of = move |states: &WidgetStates| {
                if states.contains(&WidgetState::Disabled) {
                    return disabled_color;
                }
                if states.contains(&WidgetState::Pressed) {
                    return pressed_color;
                }
                if states.contains(&WidgetState::Hovered) {
                    return hover_color;
                }
                if states.contains(&WidgetState::Focused) {
                    return focused_color;
                }
                if states.contains(&WidgetState::Selected) {
                    return selected_color;
                }
                default_color
            };
            let label_style = TextStyle::new().color(WidgetStateColor::resolve_with(text_color_of));
            let chip_widget = |tester: &WidgetTester, enabled: bool, selected: bool| {
                let chip_theme = ThemeData::new(&tester.cell.borrow())
                    .chip_theme
                    .copy_with()
                    .label_style(label_style.clone())
                    .secondary_label_style(label_style.clone());
                let theme = ThemeData::builder()
                    .chip_theme(chip_theme)
                    .build(&tester.cell.borrow());
                material_app(
                    theme,
                    scaffold_with(
                        Focus::new(selectable_choice_chip(selected, enabled))
                            .focus_node(focus_node.clone()),
                    ),
                )
            };

            // Default, not disabled.
            let root = chip_widget(tester, true, false);
            tester.pump_widget(root);
            assert_eq!(tester.text_color("Chip"), default_color);

            // Selected.
            let root = chip_widget(tester, true, true);
            tester.pump_widget(root);
            assert_eq!(tester.text_color("Chip"), selected_color);

            // Focused.
            let chip_focus_node = focus_node.handle().children(&tester.cell.borrow())[0];
            chip_focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(tester.text_color("Chip"), focused_color);

            // Hovered.
            let center = tester.get_center(only::<ChoiceChip>(tester));
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(tester.text_color("Chip"), hover_color);

            // Pressed.
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert_eq!(tester.text_color("Chip"), pressed_color);

            // Disabled.
            let root = chip_widget(tester, false, false);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_eq!(tester.text_color("Chip"), disabled_color);
        });
    }

    /// Dart's `chipWidget({selected})` of the shape cases.
    fn pump_choice_chip_with_theme(tester: &mut WidgetTester, theme: ThemeData, selected: bool) {
        pump_app(
            tester,
            theme,
            scaffold_with(selectable_choice_chip(selected, true)),
        );
    }

    fn is_stadium_shape(shape: &Option<ShapeBorderRef>) -> bool {
        shape
            .as_ref()
            .is_some_and(|shape| shape.as_any().downcast_ref::<StadiumBorder>().is_some())
    }

    fn is_rounded_rectangle_shape(shape: &Option<ShapeBorderRef>) -> bool {
        shape.as_ref().is_some_and(|shape| {
            shape
                .as_any()
                .downcast_ref::<RoundedRectangleBorder>()
                .is_some()
        })
    }

    #[test]
    fn material2_chip_uses_stateful_shape_from_chip_theme() {
        test_widgets(|tester| {
            let chip_theme = || {
                ChipThemeData::from_defaults(
                    Some(Brightness::Light),
                    None,
                    Colors::BLUE.primary(),
                    TextStyle::new(),
                )
                .shape(WidgetStateOutlinedBorder::resolve_with(|states| {
                    states
                        .contains(&WidgetState::Selected)
                        .then(|| OutlinedBorderRef::new(RoundedRectangleBorder::default()))
                }))
            };

            // Default.
            let theme = theme_with(tester, chip_theme(), false);
            pump_choice_chip_with_theme(tester, theme, false);
            assert!(is_stadium_shape(&material_shape(tester)));

            // Selected.
            let theme = theme_with(tester, chip_theme(), false);
            pump_choice_chip_with_theme(tester, theme, true);
            assert!(is_rounded_rectangle_shape(&material_shape(tester)));
        });
    }

    #[test]
    fn material3_chip_uses_stateful_shape_from_chip_theme() {
        test_widgets(|tester| {
            let chip_theme = || {
                ChipThemeData::new().shape(WidgetStateOutlinedBorder::resolve_with(|states| {
                    states
                        .contains(&WidgetState::Selected)
                        .then(|| OutlinedBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
                }))
            };

            // Default.
            let theme = theme_with(tester, chip_theme(), true);
            pump_choice_chip_with_theme(tester, theme, false);
            assert!(is_rounded_rectangle_shape(&material_shape(tester)));

            // Selected.
            let theme = theme_with(tester, chip_theme(), true);
            pump_choice_chip_with_theme(tester, theme, true);
            assert!(is_stadium_shape(&material_shape(tester)));
        });
    }

    fn pump_raw_chip_in_material(tester: &mut WidgetTester, theme: ThemeData, chip: RawChip) {
        pump_app(tester, theme, Material::new().child(chip).into_widget());
    }

    #[test]
    fn raw_chip_uses_material_state_color_from_chip_theme() {
        test_widgets(|tester| {
            let disabled_selected_color = Color::new(0xffffff00);
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                let chip_theme = ChipThemeData::new().color(
                    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                        let disabled = states.contains(&WidgetState::Disabled);
                        let selected = states.contains(&WidgetState::Selected);
                        Some(match (disabled, selected) {
                            (true, true) => disabled_selected_color,
                            (true, false) => disabled_color,
                            (false, true) => selected_color,
                            (false, false) => background_color,
                        })
                    }),
                );
                let theme = theme_with(tester, chip_theme, true);
                pump_raw_chip_in_material(
                    tester,
                    theme,
                    RawChip::new(Text::new("RawChip"))
                        .is_enabled(enabled)
                        .selected(selected),
                );
                tester.pump_and_settle();
            };

            // Enabled chip should have the provided backgroundColor.
            build_app(tester, true, false);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(background_color))],
            );

            // Disabled chip should have the provided disabledColor.
            build_app(tester, false, false);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(disabled_color))],
            );

            // Enabled & selected chip should have the provided selectedColor.
            build_app(tester, true, true);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(selected_color))],
            );

            // Disabled & selected chip should have the provided disabledSelectedColor.
            build_app(tester, false, true);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(disabled_selected_color))],
            );
        });
    }

    #[test]
    fn raw_chip_uses_state_colors_from_chip_theme() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xadfefafe);
            let background_color = Color::new(0xcafefeed);
            let selected_color = Color::new(0xbeefcafe);
            let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                let chip_theme = ChipThemeData::new()
                    .disabled_color(disabled_color)
                    .background_color(background_color)
                    .selected_color(selected_color);
                let theme = theme_with(tester, chip_theme, true);
                pump_raw_chip_in_material(
                    tester,
                    theme,
                    RawChip::new(Text::new("RawChip"))
                        .is_enabled(enabled)
                        .selected(selected),
                );
                tester.pump_and_settle();
            };

            // Enabled chip should have the provided backgroundColor.
            build_app(tester, true, false);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(background_color))],
            );

            // Disabled chip should have the provided disabledColor.
            build_app(tester, false, false);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(disabled_color))],
            );

            // Enabled & selected chip should have the provided selectedColor.
            build_app(tester, true, true);
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::rrect(Some(selected_color))],
            );
        });
    }

    #[test]
    fn raw_chip_respects_checkmark_properties_from_chip_theme() {
        test_widgets(|tester| {
            let red = Color::new(0xffff0000);
            let build_raw_chip = |tester: &mut WidgetTester, chip_theme: ChipThemeData| {
                let theme = theme_with(tester, chip_theme, true);
                let chip = RawChip::new(SizedBox::new().width(100.0).height(100.0))
                    .selected(true)
                    .on_selected(Rc::new(|_app, _value| {}));
                pump_app(tester, theme, centered_material(chip));
            };

            // Test that the checkmark is painted.
            build_raw_chip(tester, ChipThemeData::new().checkmark_color(red));
            assert_paints(
                &material_box_ops(tester),
                &[PaintCall::Path {
                    color: Some(red),
                    style: Some(PaintedStyle::Stroke(None)),
                    includes: Vec::new(),
                    excludes: Vec::new(),
                }],
            );

            // Test that the checkmark is not painted when ChipThemeData.showCheckmark is false.
            build_raw_chip(
                tester,
                ChipThemeData::new()
                    .show_checkmark(false)
                    .checkmark_color(red),
            );
            tester.pump_and_settle();
            assert!(
                paints(
                    &material_box_ops(tester),
                    &[PaintCall::Path {
                        color: Some(red),
                        style: Some(PaintedStyle::Stroke(None)),
                        includes: Vec::new(),
                        excludes: Vec::new(),
                    }]
                )
                .is_err()
            );
        });
    }

    #[test]
    fn material3_raw_chip_shapes_side_is_used_when_provided() {
        test_widgets(|tester| {
            let shape_with_side = |color: Color| {
                RoundedRectangleBorder::new(
                    BorderSide {
                        color,
                        ..BorderSide::default()
                    },
                    waft_painting::BorderRadiusGeometry::circular(7.0),
                )
            };
            let build_chip = |tester: &mut WidgetTester, side: Option<BorderSide>| {
                let mut chip_theme =
                    ChipThemeData::new().shape(shape_with_side(Color::new(0xffff00ff)));
                if let Some(side) = side {
                    chip_theme = chip_theme.side(side);
                }
                let theme = theme_with(tester, chip_theme, true);
                pump_app(
                    tester,
                    theme,
                    Material::new()
                        .child(Center::new().child(RawChip::new(Text::new("RawChip"))))
                        .into_widget(),
                );
            };

            // Test [RawChip.shape] with a side.
            build_chip(tester, None);

            // Chip should have the provided shape and the side from [RawChip.shape].
            assert_eq!(
                material_shape(tester),
                Some(ShapeBorderRef::new(shape_with_side(Color::new(0xffff00ff))))
            );

            // Test [RawChip.shape] with a side and [RawChip.side].
            build_chip(
                tester,
                Some(BorderSide {
                    color: Color::new(0xfffff000),
                    ..BorderSide::default()
                }),
            );
            tester.pump_and_settle();

            // Chip use shape from [RawChip.shape] and the side from [RawChip.side].
            // [RawChip.shape]'s side should be ignored.
            assert_eq!(
                material_shape(tester),
                Some(ShapeBorderRef::new(shape_with_side(Color::new(0xfffff000))))
            );
        });
    }

    #[test]
    fn material3_chip_theme_data_icon_theme_respects_default_icon_theme_size() {
        test_widgets(|tester| {
            let build_chip = |tester: &mut WidgetTester, icon_theme: IconThemeData| {
                let theme = theme_with(tester, ChipThemeData::new().icon_theme(icon_theme), true);
                pump_app(tester, theme, centered_material(avatar_chip()));
            };

            build_chip(tester, IconThemeData::new().color(Color::new(0xff332211)));

            // Icon should have the default chip iconSize.
            assert_eq!(icon_theme_data(tester).size, Some(18.0));
            assert_eq!(icon_color(tester), Some(Color::new(0xff332211)));

            // Icon should have the provided iconSize.
            build_chip(
                tester,
                IconThemeData::new()
                    .color(Color::new(0xff112233))
                    .size(23.0),
            );
            tester.pump_and_settle();

            assert_eq!(icon_theme_data(tester).size, Some(23.0));
            assert_eq!(icon_color(tester), Some(Color::new(0xff112233)));
        });
    }

    fn label_container() -> Container {
        Container::new()
            .width(75.0)
            .height(75.0)
            .color(Color::new(0xFFFF0000))
    }

    #[test]
    fn chip_theme_data_avatar_box_constraints_updates_avatar_size_constraints() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(75.0, 75.0);

            // Test default avatar layout constraints.
            let chip_theme = ChipThemeData::new().avatar_box_constraints(
                BoxConstraints::tight_for_finite(f64::INFINITY, f64::INFINITY),
            );
            let theme = theme_with(tester, chip_theme, true);
            pump_app(
                tester,
                theme,
                Material::new()
                    .child(Center::new().child(
                        RawChip::new(label_container()).avatar(Icon::new(Some(Icons::favorite()))),
                    ))
                    .into_widget(),
            );

            let chip_size = tester.get_size(only::<RawChip>(tester));
            assert_eq!(chip_size.width(), 127.0);
            assert_eq!(chip_size.height(), 93.0);

            // Calculate the distance between avatar and chip edges.
            let chip_top_left = tester.get_rect(material_of_chip(tester)).top_left();
            let avatar_center =
                tester.get_center(tester.element(find::by_icon(&Icons::favorite()).first()));
            assert_eq!(
                chip_top_left.dx(),
                avatar_center.dx() - (icon_size / 2.0) - padding - border
            );
            assert_eq!(
                chip_top_left.dy(),
                avatar_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between avatar and label.
            let label_top_left = tester.get_rect(only::<Container>(tester)).top_left();
            assert_eq!(
                label_top_left.dx(),
                avatar_center.dx() + (icon_size / 2.0) + label_padding
            );
        });
    }

    #[test]
    fn chip_theme_data_delete_icon_box_constraints_updates_delete_icon_size_constraints() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(75.0, 75.0);

            // Test custom delete layout constraints.
            let chip_theme = ChipThemeData::new().delete_icon_box_constraints(
                BoxConstraints::tight_for_finite(f64::INFINITY, f64::INFINITY),
            );
            let theme = theme_with(tester, chip_theme, true);
            pump_app(
                tester,
                theme,
                Material::new()
                    .child(Center::new().child(
                        RawChip::new(label_container()).on_deleted(Listener::new(|_app| {})),
                    ))
                    .into_widget(),
            );

            let chip_size = tester.get_size(only::<RawChip>(tester));
            assert_eq!(chip_size.width(), 127.0);
            assert_eq!(chip_size.height(), 93.0);

            // Calculate the distance between delete icon and chip edges.
            let chip_top_right = tester.get_rect(material_of_chip(tester)).top_right();
            let delete_icon_center =
                tester.get_center(tester.element(find::by_icon(&Icons::cancel()).first()));
            assert_eq!(
                chip_top_right.dx(),
                delete_icon_center.dx() + (icon_size / 2.0) + padding + border
            );
            assert_eq!(
                chip_top_right.dy(),
                delete_icon_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between delete icon and label.
            let label_top_right = tester.get_rect(only::<Container>(tester)).top_right();
            assert_eq!(
                label_top_right.dx(),
                delete_icon_center.dx() - (icon_size / 2.0) - label_padding
            );
        });
    }

    #[test]
    fn chip_theme_data_icon_theme_updates_avatar_and_delete_icons() {
        test_widgets(|tester| {
            let icon_color = Color::new(0xffff0000);
            let icon_size = 32.0;
            let chip_theme = ChipThemeData::new()
                .icon_theme(IconThemeData::new().color(icon_color).size(icon_size));
            let theme = theme_with(tester, chip_theme, true);
            pump_app(
                tester,
                theme,
                Material::new()
                    .child(
                        Center::new().child(
                            RawChip::new(SizedBox::new().height(100.0))
                                .avatar(Icon::new(Some(Icons::favorite())))
                                .delete_icon(Icon::new(Some(Icons::delete())))
                                .on_deleted(Listener::new(|_app| {})),
                        ),
                    )
                    .into_widget(),
            );

            // Test rendered icon size.
            let avatar_icon = tester.element(find::by_icon(&Icons::favorite()).first());
            let delete_icon = tester.element(find::by_icon(&Icons::delete()).first());
            assert_eq!(tester.get_size(avatar_icon).width(), icon_size);
            assert_eq!(tester.get_size(delete_icon).width(), icon_size);

            // Test rendered icon color.
            assert_eq!(
                tester
                    .glyph_style(avatar_icon)
                    .color
                    .map(|color| color.color()),
                Some(icon_color)
            );
            assert_eq!(
                tester
                    .glyph_style(delete_icon)
                    .color
                    .map(|color| color.color()),
                Some(icon_color)
            );
        });
    }

    #[test]
    fn chip_theme_data_delete_icon_color_overrides_chip_theme_data_icon_theme_color() {
        test_widgets(|tester| {
            let icon_color = Color::new(0xffff00ff);
            let delete_icon_color = Color::new(0xffff00ff);
            let build_chip = |tester: &mut WidgetTester, delete_icon_color: Option<Color>| {
                let mut chip_theme =
                    ChipThemeData::new().icon_theme(IconThemeData::new().color(icon_color));
                if let Some(delete_icon_color) = delete_icon_color {
                    chip_theme = chip_theme.delete_icon_color(delete_icon_color);
                }
                let theme = theme_with(tester, chip_theme, true);
                pump_app(
                    tester,
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(
                                RawChip::new(SizedBox::new().height(100.0))
                                    .delete_icon(Icon::new(Some(Icons::delete())))
                                    .on_deleted(Listener::new(|_app| {})),
                            ),
                        )
                        .into_widget(),
                );
            };
            let delete_glyph_color = |tester: &WidgetTester| {
                tester
                    .glyph_style(tester.element(find::by_icon(&Icons::delete()).first()))
                    .color
                    .map(|color| color.color())
            };

            build_chip(tester, None);

            // Test rendered icon color.
            assert_eq!(delete_glyph_color(tester), Some(icon_color));

            build_chip(tester, Some(delete_icon_color));

            // Test rendered icon color.
            assert_eq!(delete_glyph_color(tester), Some(delete_icon_color));
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/135136.
    #[test]
    fn widget_state_border_side_properly_lerp_in_chip_theme_data_side() {
        test_widgets(|tester| {
            let primary = Rc::new(Cell::new(Color::new(0)));
            let build_chip = |tester: &mut WidgetTester, seed_color: Color| {
                let color_scheme = ColorScheme::from_seed(seed_color).build();
                primary.set(color_scheme.primary);
                let side_primary = Rc::clone(&primary);
                let theme = ThemeData::builder()
                    .color_scheme(color_scheme)
                    .chip_theme(
                        ChipThemeData::new().side(WidgetStateBorderSide::resolve_with(
                            move |_states| {
                                Some(BorderSide {
                                    color: side_primary.get(),
                                    width: 4.0,
                                    ..BorderSide::default()
                                })
                            },
                        )),
                    )
                    .build(&tester.cell.borrow());
                pump_app(
                    tester,
                    theme,
                    Scaffold::new()
                        .body(RawChip::new(Text::new("Chip")))
                        .into_widget(),
                );
            };

            build_chip(tester, Colors::RED.primary());
            tester.pump_and_settle();
            assert_paints(
                &raw_chip_ops(tester),
                &[PaintCall::drrect(Some(primary.get()))],
            );

            build_chip(tester, Colors::BLUE.primary());
            tester.pump_by(Duration::from_millis(100));
            assert_paints(
                &raw_chip_ops(tester),
                &[PaintCall::drrect(Some(primary.get()))],
            );
        });
    }
}
