//! Flutter counterpart: material_ui's `action_chip.dart` with its
//! `generated/action_chip_defaults_m3.g.dart`.

use std::rc::Rc;

use waft_embedder::{Clip, Color};
use waft_foundation::{App, Listener};
use waft_painting::{AnyBorderSide, EdgeInsetsGeometry, OutlinedBorderRef};
use waft_rendering::BoxConstraints;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, FocusNodeRef, IconThemeData, IntoWidget, KeyRef, StatelessWidget, TextStyle,
    WidgetRef, WidgetStatePropertyRef,
};

use crate::chip::{
    ChipAnimationStyle, ChipVariant, DisabledChipAttributes, RawChip, chip_into_setters,
    chip_non_negative_setters, chip_option_setters, chip_value_setters, chip_widget_setters,
    impl_chip_attributes, impl_disabled_chip_attributes, impl_tappable_chip_attributes,
};
use crate::debug::debug_check_has_material;
use crate::generated::action_chip_defaults_m3::ActionChipDefaultsM3;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// A Material Design action chip.
///
/// Action chips are a set of options which trigger an action related to primary
/// content. Action chips should appear dynamically and contextually in a UI.
///
/// Action chips can be tapped to trigger an action or show progress and
/// confirmation. For Material 3, a disabled state is supported for Action
/// chips and is specified with [`on_pressed`](Self::on_pressed) being null. For previous
/// versions of Material Design, it is recommended to remove the Action chip from
/// the interface entirely rather than display a disabled chip.
///
/// Action chips are displayed after primary content, such as below a card or
/// persistently at the bottom of a screen.
///
/// The material button widgets, `ElevatedButton`, `TextButton`, and
/// `OutlinedButton`, are an alternative to action chips, which should appear
/// statically and consistently in a UI.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// ## Material Design 3
///
/// [`ActionChip`] can be used for both the Assist and Suggestion chips from
/// Material Design 3. If `ThemeData::use_material3` is true, then [`ActionChip`]
/// will be styled to match the Material Design 3 Assist and Suggestion chips.
///
/// See also:
///
///  * [`Chip`], a chip that displays information and can be deleted.
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `CircleAvatar`, which shows images or initials of people.
///  * `Wrap`, A widget that displays its children in multiple horizontal or
///    vertical runs.
///  * <https://material.io/design/components/chips.html>
#[derive(Clone, Debug)]
pub struct ActionChip {
    pub key: Option<KeyRef>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub on_pressed: Option<Listener>,
    pub press_elevation: Option<f64>,
    pub tooltip: Option<String>,
    pub side: Option<AnyBorderSide>,
    pub shape: Option<OutlinedBorderRef>,
    pub clip_behavior: Clip,
    pub focus_node: Option<FocusNodeRef>,
    pub autofocus: bool,
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub background_color: Option<Color>,
    pub disabled_color: Option<Color>,
    pub padding: Option<EdgeInsetsGeometry>,
    pub visual_density: Option<VisualDensity>,
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    pub elevation: Option<f64>,
    pub shadow_color: Option<Color>,
    pub surface_tint_color: Option<Color>,
    pub icon_theme: Option<IconThemeData>,
    pub avatar_box_constraints: Option<BoxConstraints>,
    pub chip_animation_style: Option<ChipAnimationStyle>,
    pub mouse_cursor: Option<MouseCursorRef>,
    chip_variant: ChipVariant,
}

impl ActionChip {
    /// Create a chip that acts like a button.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn new<K>(label: impl IntoWidget<K>) -> ActionChip {
        ActionChip {
            key: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            label_padding: None,
            on_pressed: None,
            press_elevation: None,
            tooltip: None,
            side: None,
            shape: None,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            color: None,
            background_color: None,
            disabled_color: None,
            padding: None,
            visual_density: None,
            material_tap_target_size: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            avatar_box_constraints: None,
            chip_animation_style: None,
            mouse_cursor: None,
            chip_variant: ChipVariant::Flat,
        }
    }

    /// Create an elevated chip that acts like a button.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn elevated<K>(label: impl IntoWidget<K>) -> ActionChip {
        ActionChip {
            chip_variant: ChipVariant::Elevated,
            ..ActionChip::new(label)
        }
    }

    chip_option_setters!(
        ActionChip;
        key: KeyRef,
        label_style: TextStyle,
        on_pressed: Listener,
        tooltip: String,
        focus_node: FocusNodeRef,
        color: WidgetStatePropertyRef<Option<Color>>,
        background_color: Color,
        disabled_color: Color,
        visual_density: VisualDensity,
        material_tap_target_size: MaterialTapTargetSize,
        shadow_color: Color,
        surface_tint_color: Color,
        icon_theme: IconThemeData,
        avatar_box_constraints: BoxConstraints,
        chip_animation_style: ChipAnimationStyle,
        mouse_cursor: MouseCursorRef,
    );
    chip_value_setters!(
        ActionChip;
        clip_behavior: Clip,
        autofocus: bool,
    );
    chip_widget_setters!(ActionChip; avatar);
    chip_into_setters!(
        ActionChip;
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        shape: OutlinedBorderRef,
        padding: EdgeInsetsGeometry,
    );
    chip_non_negative_setters!(ActionChip; press_elevation, elevation);
}

impl StatelessWidget for ActionChip {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let is_enabled = DisabledChipAttributes::is_enabled(self);
        let mut raw_chip = RawChip::new(self.label.clone());
        if Theme::of(app, context).use_material3 {
            raw_chip.default_properties = Some(Rc::new(ActionChipDefaultsM3::new(
                context,
                is_enabled,
                self.chip_variant,
            )));
        }
        raw_chip.avatar = self.avatar.clone();
        raw_chip.on_pressed = self.on_pressed.clone();
        raw_chip.press_elevation = self.press_elevation;
        raw_chip.tooltip = self.tooltip.clone();
        raw_chip.label_style = self.label_style.clone();
        raw_chip.color = self.color.clone();
        raw_chip.background_color = self.background_color;
        raw_chip.side = self.side.clone();
        raw_chip.shape = self.shape.clone();
        raw_chip.clip_behavior = self.clip_behavior;
        raw_chip.focus_node = self.focus_node.clone();
        raw_chip.autofocus = self.autofocus;
        raw_chip.disabled_color = self.disabled_color;
        raw_chip.padding = self.padding;
        raw_chip.visual_density = self.visual_density;
        raw_chip.is_enabled = is_enabled;
        raw_chip.label_padding = self.label_padding;
        raw_chip.material_tap_target_size = self.material_tap_target_size;
        raw_chip.elevation = self.elevation;
        raw_chip.shadow_color = self.shadow_color;
        raw_chip.surface_tint_color = self.surface_tint_color;
        raw_chip.icon_theme = self.icon_theme.clone();
        raw_chip.avatar_box_constraints = self.avatar_box_constraints;
        raw_chip.chip_animation_style = self.chip_animation_style.clone();
        raw_chip.mouse_cursor = self.mouse_cursor.clone();
        raw_chip.into_widget()
    }
}

impl_chip_attributes!(ActionChip);
impl_tappable_chip_attributes!(ActionChip);
impl_disabled_chip_attributes!(ActionChip, |chip| chip.on_pressed.is_some());

#[cfg(test)]
pub(crate) mod tests {

    use waft_animation::AnimationStyle;
    use waft_embedder::{Offset, PointerChange, Size};
    use waft_foundation::K_IS_WEB;
    use waft_painting::{
        BorderRadiusGeometry, BorderSide, RoundedRectangleBorder, ShapeBorderRef, StadiumBorder,
    };
    use waft_rendering::BoxConstraints;
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Center, Column, CustomPaint, FocusHighlightStrategy, FocusNode, FocusNodeLeaf,
        SizedBox, Text, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
        WidgetStatePropertyRef,
    };

    use waft_widgets::testing::{
        TestPointer, WidgetTester, assert_more_or_less_equals_within, find, test_widgets,
    };

    use super::*;
    use crate::chip::tests::{
        chip_material_of, icon_data, ink_decoration_color, label_style_color, only, text, wrap_ltr,
    };
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::motion::Durations;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};

    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3};
    use waft_rendering::testing::{PaintCall, paints};

    const LABEL: &str = "action chip";

    fn pump_chip(tester: &mut WidgetTester, theme: &ThemeData, chip: ActionChip, centered: bool) {
        let home = if centered {
            Material::new()
                .child(Center::new().child(chip))
                .into_widget()
        } else {
            Material::new().child(chip).into_widget()
        };
        tester.pump_widget(material_app(theme.clone(), home));
    }

    fn rounded_rectangle(side: BorderSide) -> Option<waft_painting::ShapeBorderRef> {
        Some(waft_painting::ShapeBorderRef::new(
            RoundedRectangleBorder::new(side, BorderRadiusGeometry::circular(8.0)),
        ))
    }

    #[test]
    fn material3_action_chip_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());

            // Test enabled ActionChip defaults.
            let enabled = ActionChip::new(Text::new(LABEL)).on_pressed(Listener::new(|_| {}));
            pump_chip(tester, &theme, enabled, true);

            // Test default chip size.
            let size = tester.get_size(only::<ActionChip>(tester));
            assert_more_or_less_equals_within(size.width(), 189.1, 0.01);
            assert_more_or_less_equals_within(size.height(), 48.0, 0.01);
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(theme.color_scheme.on_surface())
            );

            let chip = only::<RawChip>(tester);
            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, chip);
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                shape,
                rounded_rectangle(BorderSide {
                    color: theme.color_scheme.outline_variant(),
                    ..BorderSide::default()
                })
            );
            assert_eq!(ink_decoration_color(tester), None);

            // Test disabled ActionChip defaults.
            pump_chip(tester, &theme, ActionChip::new(Text::new(LABEL)), false);
            tester.pump_and_settle();

            let chip = only::<RawChip>(tester);
            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, chip);
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                shape,
                rounded_rectangle(BorderSide {
                    color: theme.color_scheme.on_surface().with_alpha(0x1f),
                    ..BorderSide::default()
                })
            );
            assert_eq!(ink_decoration_color(tester), None);
        });
    }

    #[test]
    fn material3_action_chip_elevated_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let transparent_side = BorderSide {
                color: Colors::TRANSPARENT,
                ..BorderSide::default()
            };

            // Test enabled ActionChip.elevated defaults.
            let enabled = ActionChip::elevated(Text::new(LABEL)).on_pressed(Listener::new(|_| {}));
            pump_chip(tester, &theme, enabled, true);

            let size = tester.get_size(only::<ActionChip>(tester));
            assert_more_or_less_equals_within(size.width(), 189.1, 0.01);
            assert_more_or_less_equals_within(size.height(), 48.0, 0.01);
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(theme.color_scheme.on_surface())
            );

            let chip = only::<RawChip>(tester);
            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, chip);
            assert_eq!(elevation, 1.0);
            assert_eq!(shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(transparent_side));
            assert_eq!(
                ink_decoration_color(tester),
                Some(theme.color_scheme.surface_container_low())
            );

            // Test disabled ActionChip.elevated defaults.
            pump_chip(
                tester,
                &theme,
                ActionChip::elevated(Text::new(LABEL)),
                false,
            );
            tester.pump_and_settle();

            let chip = only::<RawChip>(tester);
            let (elevation, shadow_color, surface_tint_color, shape) =
                chip_material_of(tester, chip);
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shape, rounded_rectangle(transparent_side));
            assert_eq!(
                ink_decoration_color(tester),
                Some(theme.color_scheme.on_surface().with_alpha(0x1f))
            );
        });
    }

    #[test]
    fn action_chip_can_be_tapped() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let pressed = std::rc::Rc::new(std::cell::Cell::new(0));
            let chip = ActionChip::new(Text::new(LABEL)).on_pressed({
                let pressed = std::rc::Rc::clone(&pressed);
                Listener::new(move |_| pressed.set(pressed.get() + 1))
            });
            pump_chip(tester, &theme, chip, false);

            let center = crate::chip::tests::center_of(tester, only::<ActionChip>(tester));
            tester.tap_at(center, 1);
            assert_eq!(pressed.get(), 1);
        });
    }

    pub(crate) fn pump_wrapped<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) {
        let root = wrap_ltr(tester, child);
        tester.pump_widget(root);
    }

    pub(crate) fn material_of(tester: &WidgetTester, chip: AnyElement) -> AnyElement {
        tester.descendants_of_type::<Material>(chip)[0]
    }

    /// Dart's `getMaterialBox(tester, chip)`: the first `CustomPaint` below the chip.
    pub(crate) fn material_box(tester: &WidgetTester, chip: AnyElement) -> AnyElement {
        tester.descendants_of_type::<CustomPaint>(chip)[0]
    }

    /// Dart's `paints..rrect(color: color)` on `getMaterialBox(tester, chip)`: the first
    /// rounded rect the box and everything under it draw is of `color`.
    pub(crate) fn paints_rrect_color(
        tester: &WidgetTester,
        chip: AnyElement,
        color: Color,
    ) -> bool {
        let render = tester.render_box_of(material_box(tester, chip)).as_object();
        let ops = tester.painted_ops_deep(render);
        paints(&ops, &[PaintCall::rrect(Some(color))]).is_ok()
    }

    /// Dart's `paints..path(color: color)` on `getMaterialBox(tester, chip)`.
    pub(crate) fn paints_path_color(tester: &WidgetTester, chip: AnyElement, color: Color) -> bool {
        let render = tester.render_box_of(material_box(tester, chip)).as_object();
        paints(
            &tester.painted_ops_deep(render),
            &[PaintCall::path(Some(color))],
        )
        .is_ok()
    }

    pub(crate) fn raw_chips(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<RawChip>().evaluate(tester)
    }

    #[test]
    fn material2_action_chip_defaults() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());

            // Test enabled ActionChip defaults.
            pump_chip(
                tester,
                &theme,
                ActionChip::new(Text::new(LABEL)).on_pressed(Listener::new(|_| {})),
                true,
            );

            // Test default chip size.
            assert_eq!(
                tester.get_size(only::<ActionChip>(tester)),
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
            assert_eq!(
                shape,
                Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(Colors::BLACK.with_alpha(0x1f))
            );

            // Test disabled ActionChip defaults.
            pump_chip(tester, &theme, ActionChip::new(Text::new(LABEL)), false);
            tester.pump_and_settle();

            let (elevation, shadow_color, _, shape) =
                chip_material_of(tester, only::<RawChip>(tester));
            assert_eq!(elevation, 0.0);
            assert_eq!(shadow_color, Some(Colors::BLACK));
            assert_eq!(
                shape,
                Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
            );
            assert_eq!(ink_decoration_color(tester), Some(Colors::BLACK38));
        });
    }

    fn two_chips(
        color: Option<WidgetStatePropertyRef<Option<Color>>>,
        colors: Option<(Color, Color)>,
        enabled: bool,
    ) -> Column {
        let build = |mut chip: ActionChip| {
            if enabled {
                chip = chip.on_pressed(Listener::new(|_| {}));
            }
            if let Some(color) = &color {
                chip = chip.color(color.clone());
            }
            if let Some((disabled, background)) = colors {
                chip = chip.disabled_color(disabled).background_color(background);
            }
            chip.into_widget()
        };
        Column::new().children([
            build(ActionChip::new(Text::new("ActionChip"))),
            build(ActionChip::elevated(Text::new("ActionChip.elevated"))),
        ])
    }

    fn check_enabled_and_disabled_colors(
        tester: &mut WidgetTester,
        build: impl Fn(bool) -> Column,
        disabled_color: Color,
        background_color: Color,
    ) {
        // Test enabled state.
        pump_wrapped(tester, build(true));
        let chips = raw_chips(tester);
        // Enabled ActionChip should have the provided backgroundColor.
        assert!(paints_rrect_color(tester, chips[0], background_color));
        // Enabled elevated ActionChip should have the provided backgroundColor.
        assert!(paints_rrect_color(tester, chips[1], background_color));

        // Test disabled state.
        pump_wrapped(tester, build(false));
        tester.pump_and_settle();
        let chips = raw_chips(tester);
        // Disabled ActionChip should have the provided disabledColor.
        assert!(paints_rrect_color(tester, chips[0], disabled_color));
        // Disabled elevated ActionChip should have the provided disabledColor.
        assert!(paints_rrect_color(tester, chips[1], disabled_color));
    }

    #[test]
    fn action_chip_color_resolves_material_states() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(disabled_color);
                }
                Some(background_color)
            });
            check_enabled_and_disabled_colors(
                tester,
                |enabled| two_chips(Some(color.clone()), None, enabled),
                disabled_color,
                background_color,
            );
        });
    }

    #[test]
    fn action_chip_uses_provided_state_color_properties() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            check_enabled_and_disabled_colors(
                tester,
                |enabled| two_chips(None, Some((disabled_color, background_color)), enabled),
                disabled_color,
                background_color,
            );
        });
    }

    #[test]
    fn action_chip_clip_behavior_properly_passes_through_to_the_material() {
        test_widgets(|tester| {
            let check = |tester: &WidgetTester, clip_behavior: Clip| {
                let materials = find::by_type::<Material>().evaluate(tester);
                // There should be two Material widgets, first Material is from the
                // "wrapForChip" and last Material is from the "RawChip".
                assert_eq!(materials.len(), 2);
                let last = *materials.last().expect("a Material");
                assert_eq!(
                    tester.widget(last, |material: &Material| material.clip_behavior),
                    clip_behavior
                );
            };
            pump_wrapped(
                tester,
                ActionChip::new(Text::new("label")).on_pressed(Listener::new(|_| {})),
            );
            check(tester, Clip::None);

            pump_wrapped(
                tester,
                ActionChip::new(Text::new("label"))
                    .clip_behavior(Clip::AntiAlias)
                    .on_pressed(Listener::new(|_| {})),
            );
            check(tester, Clip::AntiAlias);
        });
    }

    #[test]
    fn action_chip_uses_provided_icon_theme() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let build_chip = |tester: &mut WidgetTester, icon_theme: Option<IconThemeData>| {
                let mut chip = ActionChip::new(Text::new("action chip"))
                    .avatar(waft_widgets::Icon::new(Some(Icons::add())))
                    .on_pressed(Listener::new(|_| {}));
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
                Some(theme.color_scheme.primary())
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
    fn action_chip_avatar_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = ActionChip::new(
                    waft_widgets::Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                )
                .avatar(waft_widgets::Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| tester.get_size(only::<ActionChip>(tester));
            let chip_top_left = |tester: &WidgetTester| {
                tester
                    .get_rect(material_of(tester, only::<RawChip>(tester)))
                    .top_left()
            };
            let label_top_left = |tester: &WidgetTester| {
                tester
                    .get_rect(only::<waft_widgets::Container>(tester))
                    .top_left()
            };

            // Test default avatar layout constraints.
            build_chip(tester, None);
            assert_eq!(chip_size(tester).width(), 234.0);
            assert_eq!(chip_size(tester).height(), 118.0);

            // Calculate the distance between avatar and chip edges.
            let top_left = chip_top_left(tester);
            let avatar_center =
                tester.get_center(tester.element(find::by_icon(&Icons::favorite()).first()));
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

    pub(crate) fn animation_style() -> ChipAnimationStyle {
        ChipAnimationStyle::new()
            .enable_animation(AnimationStyle::new(
                None,
                Some(Durations::EXTRALONG4),
                None,
                None,
            ))
            .select_animation(AnimationStyle::NO_ANIMATION)
    }

    pub(crate) fn assert_animation_style_passed_to_raw_chip<K>(
        tester: &mut WidgetTester,
        chip: impl IntoWidget<K>,
    ) {
        pump_wrapped(tester, Center::new().child(chip));
        let passed = tester.widget(only::<RawChip>(tester), |raw_chip: &RawChip| {
            raw_chip.chip_animation_style.clone()
        });
        let expected = animation_style();
        let passed = passed.expect("a chip animation style");
        assert_eq!(passed.enable_animation, expected.enable_animation);
        assert_eq!(passed.select_animation, expected.select_animation);
        assert_eq!(
            passed.avatar_drawer_animation,
            expected.avatar_drawer_animation
        );
        assert_eq!(
            passed.delete_drawer_animation,
            expected.delete_drawer_animation
        );
    }

    #[test]
    fn action_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                ActionChip::new(Text::new("ActionChip")).chip_animation_style(animation_style()),
            );
        });
    }

    #[test]
    fn elevated_action_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                ActionChip::elevated(Text::new("ActionChip"))
                    .chip_animation_style(animation_style()),
            );
        });
    }

    pub(crate) fn hover_over_chip(tester: &mut WidgetTester, mouse: &mut TestPointer) {
        tester.add_pointer(mouse, Offset::new(10.0, 10.0));
        tester.pump();
        assert!(*tester.active_cursor(1) == *MouseCursorRef::from(SystemMouseCursors::BASIC));
        let chip = tester.get_center(text(tester, "Chip"));
        tester.move_to(mouse, chip, false);
        tester.pump();
    }

    #[test]
    fn action_chip_has_expected_default_mouse_cursor_on_hover() {
        test_widgets(|tester| {
            pump_wrapped(
                tester,
                Center::new()
                    .child(ActionChip::new(Text::new("Chip")).on_pressed(Listener::new(|_| {}))),
            );
            let mut mouse = TestPointer::mouse(1, 0);
            hover_over_chip(tester, &mut mouse);
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert!(*tester.active_cursor(1) == *expected);
        });
    }

    #[test]
    fn action_chip_mouse_cursor_behavior() {
        test_widgets(|tester| {
            let custom_cursor: MouseCursorRef = SystemMouseCursors::GRAB.into();
            pump_wrapped(
                tester,
                Center::new()
                    .child(ActionChip::new(Text::new("Chip")).mouse_cursor(custom_cursor.clone())),
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
                    (WidgetState::any(), SystemMouseCursors::BASIC.into()),
                ]);
                let mut chip = ActionChip::new(Text::new("Chip"))
                    .mouse_cursor(cursor.into())
                    .focus_node(focus_node.erase());
                if enabled {
                    chip = chip.on_pressed(Listener::new(|_| {}));
                }
                pump_wrapped(tester, Center::new().child(chip));
            };

            build_chip(tester, true);

            // Unfocused case.
            let mut mouse = TestPointer::mouse(1, 0);
            let chip = tester.get_center(text(tester, "Chip"));
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
    fn action_chip_renders_at_zero_area() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        SizedBox::shrink()
                            .child(Scaffold::new().body(ActionChip::new(Text::new("X")))),
                    )
                    .into_widget(),
            ));
            let x_text = text(tester, "X");
            assert!(tester.get_size(x_text).is_empty());
        });
    }
}
