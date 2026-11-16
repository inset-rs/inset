//! Flutter counterpart: material_ui's `choice_chip.dart`.

use std::rc::Rc;

use waft_embedder::{Clip, Color, clamp_double};
use waft_foundation::{App, ValueChanged};
use waft_painting::{
    AnyBorderSide, AnyColor, BorderRadiusGeometry, BorderSide, CircleBorder, EdgeInsets,
    EdgeInsetsGeometry, OutlinedBorderRef, RoundedRectangleBorder, ShapeBorderRef, TextStyle,
};
use waft_rendering::BoxConstraints;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, FocusNodeRef, IconThemeData, IntoWidget, KeyRef, MediaQuery, StatelessWidget,
    WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::chip::{
    ChipAnimationStyle, ChipVariant, DisabledChipAttributes, RawChip, chip_into_setters,
    chip_non_negative_setters, chip_option_setters, chip_value_setters, chip_widget_setters,
    impl_checkmarkable_chip_attributes, impl_chip_attributes, impl_chip_debug,
    impl_disabled_chip_attributes, impl_selectable_chip_attributes,
};
use crate::chip_theme::{ChipTheme, ChipThemeData, ChipThemeDataGetters};
use crate::colors::Colors;
use crate::debug::debug_check_has_material;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::theme_data_getters::DefaultsContext;

/// A Material Design choice chip.
///
/// [`ChoiceChip`]s represent a single choice from a set. Choice chips contain
/// related descriptive text or categories.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// ## Material Design 3
///
/// [`ChoiceChip`] can be used for single select Filter chips from
/// Material Design 3. If `ThemeData::use_material3` is true, then [`ChoiceChip`]
/// will be styled to match the Material Design 3 specification for Filter
/// chips. Use `FilterChip` for multiple select Filter chips.
///
/// See also:
///
///  * [`Chip`](crate::Chip), a chip that displays information and can be deleted.
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * `ActionChip`, represents an action related to primary content.
///  * `CircleAvatar`, which shows images or initials of people.
///  * `Wrap`, A widget that displays its children in multiple horizontal or
///    vertical runs.
///  * <https://material.io/design/components/chips.html>
#[derive(Clone)]
pub struct ChoiceChip {
    pub key: Option<KeyRef>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub on_selected: Option<ValueChanged<bool>>,
    pub press_elevation: Option<f64>,
    pub selected: bool,
    pub selected_color: Option<Color>,
    pub disabled_color: Option<Color>,
    pub tooltip: Option<String>,
    pub side: Option<AnyBorderSide>,
    pub shape: Option<OutlinedBorderRef>,
    pub clip_behavior: Clip,
    pub focus_node: Option<FocusNodeRef>,
    pub autofocus: bool,
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub background_color: Option<Color>,
    pub padding: Option<EdgeInsetsGeometry>,
    pub visual_density: Option<VisualDensity>,
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    pub elevation: Option<f64>,
    pub shadow_color: Option<Color>,
    pub surface_tint_color: Option<Color>,
    pub icon_theme: Option<IconThemeData>,
    pub selected_shadow_color: Option<Color>,
    pub show_checkmark: Option<bool>,
    pub checkmark_color: Option<Color>,
    pub avatar_border: ShapeBorderRef,
    pub avatar_box_constraints: Option<BoxConstraints>,
    pub chip_animation_style: Option<ChipAnimationStyle>,
    pub mouse_cursor: Option<MouseCursorRef>,
    chip_variant: ChipVariant,
}

impl_chip_debug!(ChoiceChip);

impl ChoiceChip {
    /// Create a chip.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn new<K>(label: impl IntoWidget<K>, selected: bool) -> ChoiceChip {
        ChoiceChip {
            key: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            label_padding: None,
            on_selected: None,
            press_elevation: None,
            selected,
            selected_color: None,
            disabled_color: None,
            tooltip: None,
            side: None,
            shape: None,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            color: None,
            background_color: None,
            padding: None,
            visual_density: None,
            material_tap_target_size: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            selected_shadow_color: None,
            show_checkmark: None,
            checkmark_color: None,
            avatar_border: ShapeBorderRef::new(CircleBorder::default()),
            avatar_box_constraints: None,
            chip_animation_style: None,
            mouse_cursor: None,
            chip_variant: ChipVariant::Flat,
        }
    }

    /// Create an elevated chip.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn elevated<K>(label: impl IntoWidget<K>, selected: bool) -> ChoiceChip {
        ChoiceChip {
            chip_variant: ChipVariant::Elevated,
            ..ChoiceChip::new(label, selected)
        }
    }

    chip_option_setters!(
        ChoiceChip;
        key: KeyRef,
        label_style: TextStyle,
        on_selected: ValueChanged<bool>,
        selected_color: Color,
        disabled_color: Color,
        tooltip: String,
        focus_node: FocusNodeRef,
        color: WidgetStatePropertyRef<Option<Color>>,
        background_color: Color,
        visual_density: VisualDensity,
        material_tap_target_size: MaterialTapTargetSize,
        shadow_color: Color,
        surface_tint_color: Color,
        icon_theme: IconThemeData,
        selected_shadow_color: Color,
        show_checkmark: bool,
        checkmark_color: Color,
        avatar_box_constraints: BoxConstraints,
        chip_animation_style: ChipAnimationStyle,
        mouse_cursor: MouseCursorRef,
    );
    chip_value_setters!(
        ChoiceChip;
        clip_behavior: Clip,
        autofocus: bool,
        avatar_border: ShapeBorderRef,
    );
    chip_widget_setters!(ChoiceChip; avatar);
    chip_into_setters!(
        ChoiceChip;
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        shape: OutlinedBorderRef,
        padding: EdgeInsetsGeometry,
    );
    chip_non_negative_setters!(ChoiceChip; press_elevation, elevation);
}

impl StatelessWidget for ChoiceChip {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let chip_theme = ChipTheme::of(app, context);
        let use_material3 = Theme::of(app, context).use_material3;
        let is_enabled = DisabledChipAttributes::is_enabled(self);
        let mut raw_chip = RawChip::new(self.label.clone());
        if use_material3 {
            raw_chip.default_properties = Some(Rc::new(ChoiceChipDefaultsM3::new(
                context,
                is_enabled,
                self.selected,
                self.chip_variant,
            )));
        }
        raw_chip.avatar = self.avatar.clone();
        raw_chip.label_style = self.label_style.clone().or_else(|| {
            if self.selected {
                chip_theme.secondary_label_style.clone()
            } else {
                None
            }
        });
        raw_chip.label_padding = self.label_padding;
        raw_chip.on_selected = self.on_selected.clone();
        raw_chip.press_elevation = self.press_elevation;
        raw_chip.selected = self.selected;
        raw_chip.show_checkmark = Some(
            self.show_checkmark
                .or(chip_theme.show_checkmark)
                .unwrap_or(use_material3),
        );
        raw_chip.checkmark_color = self.checkmark_color;
        raw_chip.tooltip = self.tooltip.clone();
        raw_chip.side = self.side.clone();
        raw_chip.shape = self.shape.clone();
        raw_chip.clip_behavior = self.clip_behavior;
        raw_chip.focus_node = self.focus_node.clone();
        raw_chip.autofocus = self.autofocus;
        raw_chip.disabled_color = self.disabled_color;
        raw_chip.selected_color = self.selected_color.or(chip_theme.secondary_selected_color);
        raw_chip.color = self.color.clone();
        raw_chip.background_color = self.background_color;
        raw_chip.padding = self.padding;
        raw_chip.visual_density = self.visual_density;
        raw_chip.is_enabled = is_enabled;
        raw_chip.material_tap_target_size = self.material_tap_target_size;
        raw_chip.elevation = self.elevation;
        raw_chip.shadow_color = self.shadow_color;
        raw_chip.surface_tint_color = self.surface_tint_color;
        raw_chip.selected_shadow_color = self.selected_shadow_color;
        raw_chip.avatar_border = self.avatar_border.clone();
        raw_chip.icon_theme = self.icon_theme.clone();
        raw_chip.avatar_box_constraints = self.avatar_box_constraints;
        raw_chip.chip_animation_style = self.chip_animation_style.clone();
        raw_chip.mouse_cursor = self.mouse_cursor.clone();
        raw_chip.into_widget()
    }
}

impl_chip_attributes!(ChoiceChip);
impl_selectable_chip_attributes!(ChoiceChip);
impl_checkmarkable_chip_attributes!(ChoiceChip);
impl_disabled_chip_attributes!(ChoiceChip, |chip| chip.on_selected.is_some());

// BEGIN GENERATED TOKEN PROPERTIES - ChoiceChip

// Do not edit by hand. The code between the "BEGIN GENERATED" and
// "END GENERATED" comments are generated from data in the Material
// Design token database by the script:
//   dev/tools/gen_defaults/bin/gen_defaults.dart.

/// Dart's `_ChoiceChipDefaultsM3`.
pub(crate) struct ChoiceChipDefaultsM3 {
    /// Dart's `super(..)` arguments.
    chip_theme_data: ChipThemeData,
    context: DefaultsContext,
    is_enabled: bool,
    is_selected: bool,
    chip_variant: ChipVariant,
}

impl ChoiceChipDefaultsM3 {
    pub(crate) fn new(
        context: BuildContext,
        is_enabled: bool,
        is_selected: bool,
        chip_variant: ChipVariant,
    ) -> ChoiceChipDefaultsM3 {
        ChoiceChipDefaultsM3 {
            chip_theme_data: ChipThemeData::new()
                .shape(OutlinedBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(8.0),
                )))
                .show_checkmark(true),
            context: DefaultsContext::new(context),
            is_enabled,
            is_selected,
            chip_variant,
        }
    }
}

#[allow(deprecated)]
impl ChipThemeDataGetters for ChoiceChipDefaultsM3 {
    fn chip_theme_data(&self) -> &ChipThemeData {
        &self.chip_theme_data
    }

    fn elevation(&self, _app: &mut App) -> Option<f64> {
        Some(if self.chip_variant == ChipVariant::Flat {
            0.0
        } else if self.is_enabled {
            1.0
        } else {
            0.0
        })
    }

    fn press_elevation(&self, _app: &mut App) -> Option<f64> {
        Some(1.0)
    }

    fn label_style(&self, app: &mut App) -> Option<TextStyle> {
        let colors = self.context.colors(app);
        let color = if self.is_enabled {
            if self.is_selected {
                colors.on_secondary_container()
            } else {
                colors.on_surface_variant()
            }
        } else {
            colors.on_surface
        };
        self.context
            .text_theme(app)
            .label_large
            .as_ref()
            .map(|label_large| label_large.copy_with().color(color))
    }

    fn color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_surface, secondary_container, surface_container_low) = (
            colors.on_surface,
            colors.secondary_container(),
            colors.surface_container_low(),
        );
        let chip_variant = self.chip_variant;
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected)
                    && states.contains(&WidgetState::Disabled)
                {
                    return Some(on_surface.with_opacity(0.12));
                }
                if states.contains(&WidgetState::Disabled) {
                    return if chip_variant == ChipVariant::Flat {
                        None
                    } else {
                        Some(on_surface.with_opacity(0.12))
                    };
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(secondary_container);
                }
                if chip_variant == ChipVariant::Flat {
                    None
                } else {
                    Some(surface_container_low)
                }
            },
        ))
    }

    fn shadow_color(&self, app: &mut App) -> Option<Color> {
        Some(if self.chip_variant == ChipVariant::Flat {
            Colors::TRANSPARENT
        } else {
            self.context.colors(app).shadow()
        })
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn checkmark_color(&self, app: &mut App) -> Option<Color> {
        let colors = self.context.colors(app);
        Some(if self.is_enabled {
            if self.is_selected {
                colors.on_secondary_container()
            } else {
                colors.primary
            }
        } else {
            colors.on_surface
        })
    }

    fn delete_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let colors = self.context.colors(app);
        Some(AnyColor::from(if self.is_enabled {
            if self.is_selected {
                colors.on_secondary_container()
            } else {
                colors.on_surface_variant()
            }
        } else {
            colors.on_surface
        }))
    }

    fn side(&self, app: &mut App) -> Option<AnyBorderSide> {
        let colors = self.context.colors(app);
        Some(AnyBorderSide::new(
            if self.chip_variant == ChipVariant::Flat && !self.is_selected {
                if self.is_enabled {
                    BorderSide {
                        color: colors.outline_variant(),
                        ..BorderSide::default()
                    }
                } else {
                    BorderSide {
                        color: colors.on_surface.with_opacity(0.12),
                        ..BorderSide::default()
                    }
                }
            } else {
                BorderSide {
                    color: Colors::TRANSPARENT,
                    ..BorderSide::default()
                }
            },
        ))
    }

    fn icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let colors = self.context.colors(app);
        Some(
            IconThemeData::new()
                .color(if self.is_enabled {
                    if self.is_selected {
                        colors.on_secondary_container()
                    } else {
                        colors.primary
                    }
                } else {
                    colors.on_surface
                })
                .size(18.0),
        )
    }

    fn padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::all(8.0).into())
    }

    /// The label padding of the chip scales with the font size specified in the
    /// [`label_style`](ChipThemeDataGetters::label_style), and the system font size settings
    /// that scale font sizes globally.
    ///
    /// The chip at effective font size 14.0 starts with 8px on each side and as
    /// the font size scales up to closer to 28.0, the label padding is linearly
    /// interpolated from 8px to 4px. Once the label has a font size of 2 or
    /// higher, label padding remains 4px.
    fn label_padding(&self, app: &mut App) -> Option<EdgeInsetsGeometry> {
        let font_size = self
            .label_style(app)
            .and_then(|label_style| label_style.font_size)
            .unwrap_or(14.0);
        let font_size_ratio =
            MediaQuery::text_scaler_of(app, self.context.context()).scale(font_size) / 14.0;
        Some(
            EdgeInsets::lerp(
                Some(EdgeInsets::symmetric(0.0, 8.0)),
                Some(EdgeInsets::symmetric(0.0, 4.0)),
                clamp_double(font_size_ratio - 1.0, 0.0, 1.0),
            )
            .expect("both insets are given")
            .into(),
        )
    }
}

// END GENERATED TOKEN PROPERTIES - ChoiceChip

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use waft_embedder::{Offset, PointerChange, Size};
    use waft_foundation::K_IS_WEB;
    use waft_painting::{
        BorderRadiusGeometry, BorderSide, RoundedRectangleBorder, ShapeBorderRef, StadiumBorder,
    };
    use waft_rendering::BoxConstraints;
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        Center, Column, FocusHighlightStrategy, FocusNode, FocusNodeLeaf, SizedBox, Text,
        WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::action_chip::tests::{
        animation_style, assert_animation_style_passed_to_raw_chip, hover_over_chip, material_of,
        paints_rrect_color, pump_wrapped, raw_chips,
    };
    use crate::chip::tests::{
        center_of, chip_material_of, icon_data, ink_decoration_color, label_style_color, only,
        rect_of, size_of, text,
    };
    use crate::chip_theme::{ChipTheme, ChipThemeData};
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3};

    const LABEL: &str = "choice chip";

    fn pump_chip(tester: &mut WidgetTester, theme: &ThemeData, chip: ChoiceChip, centered: bool) {
        let home = if centered {
            Material::new()
                .child(Center::new().child(chip))
                .into_widget()
        } else {
            Material::new().child(chip).into_widget()
        };
        tester.pump_widget(material_app(theme.clone(), home));
    }

    fn enabled(selected: bool) -> ChoiceChip {
        let mut chip = ChoiceChip::new(Text::new(LABEL), selected);
        chip.on_selected = Some(Rc::new(|_app, _selected| {}));
        chip
    }

    fn rounded_rectangle(side_color: Color) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide {
                color: side_color,
                ..BorderSide::default()
            },
            BorderRadiusGeometry::circular(8.0),
        )))
    }

    #[test]
    fn material3_choice_chip_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let disabled_side = theme.color_scheme.on_surface.with_alpha(0x1f);

            // Test enabled ChoiceChip defaults.
            pump_chip(tester, &theme, enabled(false), true);

            // Test default chip size.
            let size = size_of(tester, only::<ChoiceChip>(tester));
            assert!((size.width() - 189.1).abs() <= 0.01, "{size:?}");
            assert!((size.height() - 48.0).abs() <= 0.01, "{size:?}");
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(theme.color_scheme.on_surface_variant())
            );

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                shape,
                rounded_rectangle(theme.color_scheme.outline_variant())
            );
            assert_eq!(ink_decoration_color(tester), None);

            // Test disabled ChoiceChip defaults.
            pump_chip(
                tester,
                &theme,
                ChoiceChip::new(Text::new(LABEL), false),
                false,
            );
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(disabled_side));
            assert_eq!(ink_decoration_color(tester), None);

            // Test selected enabled ChoiceChip defaults.
            pump_chip(tester, &theme, enabled(true), false);
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, None);
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(
                ink_decoration_color(tester),
                Some(theme.color_scheme.secondary_container())
            );

            // Test selected disabled ChoiceChip defaults.
            pump_chip(
                tester,
                &theme,
                ChoiceChip::new(Text::new(LABEL), true),
                false,
            );
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, None);
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(ink_decoration_color(tester), Some(disabled_side));
        });
    }

    #[test]
    fn choice_chip_can_be_tapped() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            pump_chip(
                tester,
                &theme,
                ChoiceChip::new(Text::new(LABEL), false),
                false,
            );

            let center = center_of(tester, only::<ChoiceChip>(tester));
            tester.tap(center, 1);
        });
    }

    fn raw_chip_of<R>(tester: &WidgetTester, read: impl FnOnce(&RawChip) -> R) -> R {
        tester.widget(only::<RawChip>(tester), |raw_chip: &RawChip| read(raw_chip))
    }

    fn choice_chip(label: &str, selected: bool) -> ChoiceChip {
        ChoiceChip::new(Text::new(label), selected)
    }

    fn with_on_selected(mut chip: ChoiceChip, enabled: bool) -> ChoiceChip {
        if enabled {
            chip.on_selected = Some(Rc::new(|_app, _selected| {}));
        }
        chip
    }

    #[test]
    fn material2_choice_chip_defaults() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let stadium = Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)));

            // Test enabled ChoiceChip defaults.
            pump_chip(tester, &theme, enabled(false), true);

            // Test default chip size.
            assert_eq!(
                size_of(tester, only::<ChoiceChip>(tester)),
                Size::new(178.0, 48.0)
            );
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                theme
                    .text_theme
                    .body_large
                    .as_ref()
                    .and_then(|style| style.color.as_ref())
                    .map(|color| color.color().with_alpha(0xde))
            );

            let (elevation, shadow_color, _, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::BLACK));
            assert_eq!(shape, stadium);
            assert_eq!(
                ink_decoration_color(tester),
                Some(Colors::BLACK.with_alpha(0x1f))
            );

            let cases = [
                (choice_chip(LABEL, false), Colors::BLACK38),
                (enabled(true), Colors::BLACK.with_alpha(0x3d)),
                (choice_chip(LABEL, true), Colors::BLACK.with_alpha(0x3d)),
            ];
            // Test disabled, selected enabled and selected disabled ChoiceChip defaults.
            for (chip, ink_color) in cases {
                pump_chip(tester, &theme, chip, false);
                tester.pump_and_settle();

                let (elevation, shadow_color, _, shape) =
                    chip_material_of(tester, only::<RawChip>(tester));
                assert_eq!(elevation, 0.0);
                assert_eq!(shadow_color, Some(Colors::BLACK));
                assert_eq!(shape, stadium);
                assert_eq!(ink_decoration_color(tester), Some(ink_color));
            }
        });
    }

    #[test]
    fn material3_choice_chip_elevated_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let disabled_fill = theme.color_scheme.on_surface.with_alpha(0x1f);
            let elevated = |selected: bool, enabled: bool| {
                with_on_selected(ChoiceChip::elevated(Text::new(LABEL), selected), enabled)
            };

            // Test enabled ChoiceChip.elevated defaults.
            pump_chip(tester, &theme, elevated(false, true), true);

            // Test default chip size.
            let size = size_of(tester, only::<ChoiceChip>(tester));
            assert!((size.width() - 189.1).abs() <= 0.01, "{size:?}");
            assert!((size.height() - 48.0).abs() <= 0.01, "{size:?}");
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(theme.color_scheme.on_surface_variant())
            );

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 1.0);
            assert_eq!(shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(
                ink_decoration_color(tester),
                Some(theme.color_scheme.surface_container_low())
            );

            // Test disabled ChoiceChip.elevated defaults.
            pump_chip(tester, &theme, elevated(false, false), false);
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(ink_decoration_color(tester), Some(disabled_fill));

            // Test selected enabled ChoiceChip.elevated defaults.
            pump_chip(tester, &theme, elevated(true, true), false);
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 1.0);
            assert_eq!(shadow_color, None);
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(
                ink_decoration_color(tester),
                Some(theme.color_scheme.secondary_container())
            );

            // Test selected disabled ChoiceChip.elevated defaults. The Dart case builds
            // `selected: false` here, and so does this one.
            pump_chip(tester, &theme, elevated(false, false), false);
            tester.pump_and_settle();

            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(Colors::TRANSPARENT));
            assert_eq!(ink_decoration_color(tester), Some(disabled_fill));
        });
    }

    fn two_chips(
        color: Option<WidgetStatePropertyRef<Option<Color>>>,
        colors: Option<[Color; 3]>,
        enabled: bool,
        selected: bool,
    ) -> Column {
        let build = |mut chip: ChoiceChip| {
            chip = with_on_selected(chip, enabled);
            if let Some(color) = &color {
                chip = chip.color(color.clone());
            }
            if let Some([disabled, background, selected_color]) = colors {
                chip = chip.disabled_color(disabled).background_color(background);
                chip.selected_color = Some(selected_color);
            }
            chip.into_widget()
        };
        Column::new().children([
            build(ChoiceChip::new(Text::new("ChoiceChip"), selected)),
            build(ChoiceChip::elevated(
                Text::new("ChoiceChip.elevated"),
                selected,
            )),
        ])
    }

    /// Pumps `build(enabled, selected)` for each of `steps` and checks the colour both chips
    /// paint, as Dart's `paints..rrect(color:)` on each chip's material box does.
    fn check_painted_colors(
        tester: &mut WidgetTester,
        build: impl Fn(bool, bool) -> Column,
        steps: &[(bool, bool, Color)],
    ) {
        for (index, (enabled, selected, expected)) in steps.iter().enumerate() {
            pump_wrapped(tester, build(*enabled, *selected));
            if index > 0 {
                tester.pump_and_settle();
            }
            let chips = raw_chips(tester);
            assert!(
                paints_rrect_color(tester, chips[0], *expected),
                "ChoiceChip {enabled} {selected}"
            );
            assert!(
                paints_rrect_color(tester, chips[1], *expected),
                "ChoiceChip.elevated {enabled} {selected}"
            );
        }
    }

    #[test]
    fn choice_chip_color_resolves_material_states() {
        test_widgets(|tester| {
            let disabled_selected_color = Color::new(0xffffff00);
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled)
                    && states.contains(&WidgetState::Selected)
                {
                    return Some(disabled_selected_color);
                }
                if states.contains(&WidgetState::Disabled) {
                    return Some(disabled_color);
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(selected_color);
                }
                Some(background_color)
            });
            check_painted_colors(
                tester,
                |enabled, selected| two_chips(Some(color.clone()), None, enabled, selected),
                &[
                    (true, false, background_color),
                    (false, false, disabled_color),
                    (true, true, selected_color),
                    (false, true, disabled_selected_color),
                ],
            );
        });
    }

    #[test]
    fn choice_chip_uses_provided_state_color_properties() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            check_painted_colors(
                tester,
                |enabled, selected| {
                    two_chips(
                        None,
                        Some([disabled_color, background_color, selected_color]),
                        enabled,
                        selected,
                    )
                },
                &[
                    (true, false, background_color),
                    (false, false, disabled_color),
                    (true, true, selected_color),
                ],
            );
        });
    }

    #[test]
    fn choice_chip_clip_behavior_properly_passes_through_to_the_material() {
        test_widgets(|tester| {
            let check = |tester: &WidgetTester, clip_behavior: Clip| {
                let materials = tester.find_by_type::<Material>();
                // There should be two Material widgets, first Material is from the
                // "wrapForChip" and last Material is from the "RawChip".
                assert_eq!(materials.len(), 2);
                let last = *materials.last().expect("a Material");
                assert_eq!(
                    tester.widget(last, |material: &Material| material.clip_behavior),
                    clip_behavior
                );
            };
            pump_wrapped(tester, choice_chip("label", false));
            check(tester, Clip::None);

            pump_wrapped(
                tester,
                choice_chip("label", false).clip_behavior(Clip::AntiAlias),
            );
            check(tester, Clip::AntiAlias);
        });
    }

    #[test]
    fn choice_chip_passes_icon_theme_property_to_raw_chip() {
        test_widgets(|tester| {
            let icon_theme = IconThemeData::new().color(Colors::RED.primary());
            pump_wrapped(
                tester,
                choice_chip("Test", true).icon_theme(icon_theme.clone()),
            );
            assert_eq!(
                raw_chip_of(tester, |raw| raw.icon_theme.clone()),
                Some(icon_theme)
            );
        });
    }

    #[test]
    fn choice_chip_passes_show_checkmark_from_chip_theme_to_raw_chip() {
        test_widgets(|tester| {
            let show_checkmark = false;
            pump_wrapped(
                tester,
                ChipTheme::new(
                    ChipThemeData::new().show_checkmark(show_checkmark),
                    choice_chip("Test", true),
                ),
            );
            assert_eq!(
                raw_chip_of(tester, |raw| raw.show_checkmark),
                Some(show_checkmark)
            );
        });
    }

    #[test]
    fn choice_chip_passes_checkmark_properties_to_raw_chip() {
        test_widgets(|tester| {
            let show_checkmark = false;
            let checkmark_color = Color::new(0xff0000ff);
            let mut chip = choice_chip("Test", true);
            chip.show_checkmark = Some(show_checkmark);
            chip.checkmark_color = Some(checkmark_color);
            pump_wrapped(tester, chip);
            assert_eq!(
                raw_chip_of(tester, |raw| raw.show_checkmark),
                Some(show_checkmark)
            );
            assert_eq!(
                raw_chip_of(tester, |raw| raw.checkmark_color),
                Some(checkmark_color)
            );
        });
    }

    #[test]
    fn choice_chip_uses_provided_icon_theme() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let build_chip = |tester: &mut WidgetTester, icon_theme: Option<IconThemeData>| {
                let mut chip = enabled(false).avatar(waft_widgets::Icon::new(Some(Icons::add())));
                if let Some(icon_theme) = icon_theme {
                    chip = chip.icon_theme(icon_theme);
                }
                tester.pump_widget(material_app(
                    theme.clone(),
                    Material::new().child(chip).into_widget(),
                ));
            };

            // Test default icon theme.
            build_chip(tester, None);
            assert_eq!(
                icon_data(tester).color.map(|color| color.color()),
                Some(theme.color_scheme.primary)
            );

            // Test provided icon theme.
            build_chip(
                tester,
                Some(IconThemeData::new().color(Color::new(0xff00ff00))),
            );
            assert_eq!(
                icon_data(tester).color.map(|color| color.color()),
                Some(Color::new(0xff00ff00))
            );
        });
    }

    #[test]
    fn choice_chip_avatar_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = ChoiceChip::new(
                    waft_widgets::Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                    false,
                )
                .avatar(waft_widgets::Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| size_of(tester, only::<ChoiceChip>(tester));
            let chip_top_left = |tester: &WidgetTester| {
                rect_of(tester, material_of(tester, only::<RawChip>(tester))).top_left()
            };
            let label_top_left = |tester: &WidgetTester| {
                rect_of(tester, only::<waft_widgets::Container>(tester)).top_left()
            };

            // Test default avatar layout constraints.
            build_chip(tester, None);
            assert_eq!(chip_size(tester).width(), 234.0);
            assert_eq!(chip_size(tester).height(), 118.0);

            // Calculate the distance between avatar and chip edges.
            let top_left = chip_top_left(tester);
            let avatar_center = center_of(tester, tester.find_by_icon(&Icons::favorite()));
            assert_eq!(
                top_left.dx(),
                avatar_center.dx() - (label_size.width() / 2.0) - padding - border
            );
            assert_eq!(
                top_left.dy(),
                avatar_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between avatar and label.
            assert_eq!(
                label_top_left(tester).dx(),
                avatar_center.dx() + (label_size.width() / 2.0) + label_padding
            );

            // Test custom avatar layout constraints.
            build_chip(
                tester,
                Some(BoxConstraints::tight_for_finite(
                    f64::INFINITY,
                    f64::INFINITY,
                )),
            );
            tester.pump();

            assert_eq!(chip_size(tester).width(), 152.0);
            assert_eq!(chip_size(tester).height(), 118.0);

            // Calculate the distance between avatar and chip edges.
            let top_left = chip_top_left(tester);
            assert_eq!(
                top_left.dx(),
                avatar_center.dx() - (icon_size / 2.0) - padding - border
            );
            assert_eq!(
                top_left.dy(),
                avatar_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between avatar and label.
            assert_eq!(
                label_top_left(tester).dx(),
                avatar_center.dx() + (icon_size / 2.0) + label_padding
            );
        });
    }

    #[test]
    fn choice_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                Center::new()
                    .child(choice_chip("ChoiceChip", true).chip_animation_style(animation_style())),
            );
        });
    }

    #[test]
    fn elevated_choice_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                Center::new().child(
                    ChoiceChip::elevated(Text::new("ChoiceChip"), true)
                        .chip_animation_style(animation_style()),
                ),
            );
        });
    }

    #[test]
    fn choice_chip_has_expected_default_mouse_cursor_on_hover() {
        test_widgets(|tester| {
            let mut chip = ChoiceChip::new(Text::new("Chip"), false);
            chip.on_selected = Some(Rc::new(|_app, _selected| {}));
            pump_wrapped(tester, Center::new().child(chip));
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::new(10.0, 10.0));
            assert!(*tester.active_cursor(1) == *MouseCursorRef::from(SystemMouseCursors::BASIC));
            let center = center_of(tester, only::<ChoiceChip>(tester));
            tester.move_to(&mut mouse, center, false);
            tester.pump();
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert!(*tester.active_cursor(1) == *expected);
        });
    }

    #[test]
    fn choice_chip_mouse_cursor_behavior() {
        test_widgets(|tester| {
            let custom_cursor: MouseCursorRef = SystemMouseCursors::GRAB.into();
            pump_wrapped(
                tester,
                Center::new().child(
                    ChoiceChip::new(Text::new("Chip"), false).mouse_cursor(custom_cursor.clone()),
                ),
            );
            let mut mouse = TestPointer::mouse(1, 0);
            hover_over_chip(tester, &mut mouse);
            assert!(*tester.active_cursor(1) == *custom_cursor);
        });
    }

    #[test]
    fn mouse_cursor_resolves_in_focused_unfocused_disabled_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());

            let build_chip = |tester: &mut WidgetTester, enabled: bool| {
                let cursor = WidgetStateMouseCursor::from_map(vec![
                    (
                        WidgetState::Disabled.into(),
                        SystemMouseCursors::FORBIDDEN.into(),
                    ),
                    (WidgetState::Focused.into(), SystemMouseCursors::GRAB.into()),
                    (
                        WidgetState::Selected.into(),
                        SystemMouseCursors::CLICK.into(),
                    ),
                    (WidgetState::any(), SystemMouseCursors::BASIC.into()),
                ]);
                let chip = with_on_selected(ChoiceChip::new(Text::new("Chip"), false), enabled)
                    .mouse_cursor(cursor.into())
                    .focus_node(focus_node.erase());
                pump_wrapped(tester, Center::new().child(chip));
            };

            // Unfocused case.
            build_chip(tester, true);
            let mut mouse = TestPointer::mouse(1, 0);
            let chip = center_of(tester, text(tester, "Chip"));
            tester.add_pointer(&mut mouse, chip);
            tester.pump();
            tester.move_to(&mut mouse, chip, false);
            assert!(*tester.active_cursor(1) == *MouseCursorRef::from(SystemMouseCursors::BASIC));

            // Focused case.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(*tester.active_cursor(1) == *MouseCursorRef::from(SystemMouseCursors::GRAB));

            // Disabled case.
            build_chip(tester, false);
            assert!(
                *tester.active_cursor(1) == *MouseCursorRef::from(SystemMouseCursors::FORBIDDEN)
            );
            tester.send(&mouse, PointerChange::Remove);
        });
    }

    #[test]
    fn choice_chip_renders_at_zero_area() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        SizedBox::shrink()
                            .child(Scaffold::new().body(ChoiceChip::new(Text::new("X"), true))),
                    )
                    .into_widget(),
            ));
            let x_text = text(tester, "X");
            assert!(size_of(tester, x_text).is_empty());
        });
    }
}
