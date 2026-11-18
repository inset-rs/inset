//! Flutter counterpart: material_ui's `input_chip.dart` with its
//! `generated/input_chip_defaults_m3.g.dart`.

use std::rc::Rc;

use waft_embedder::{Clip, Color};
use waft_foundation::{App, Listener, ValueChanged};
use waft_painting::{
    AnyBorderSide, AnyColor, CircleBorder, EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorderRef,
    TextStyle,
};
use waft_rendering::BoxConstraints;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, FocusNodeRef, Icon, IconThemeData, IntoWidget, KeyRef, StatelessWidget,
    WidgetRef, WidgetStatePropertyRef,
};

use crate::chip::{
    ChipAnimationStyle, RawChip, chip_into_setters, chip_non_negative_setters, chip_option_setters,
    chip_value_setters, chip_widget_setters, impl_checkmarkable_chip_attributes,
    impl_chip_attributes, impl_chip_debug, impl_deletable_chip_attributes,
    impl_disabled_chip_attributes, impl_selectable_chip_attributes, impl_tappable_chip_attributes,
};
use crate::debug::debug_check_has_material;
use crate::generated::input_chip_defaults_m3::InputChipDefaultsM3;
use crate::icons::Icons;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// A Material Design input chip.
///
/// Input chips represent a complex piece of information, such as an entity
/// (person, place, or thing) or conversational text, in a compact form.
///
/// Input chips can be made selectable by setting [`on_selected`](Self::on_selected), deletable
/// by setting [`on_deleted`](Self::on_deleted), and pressable like a button with
/// [`on_pressed`](Self::on_pressed). They have a [`delete_icon`](Self::delete_icon) that can be
/// set, and customized.
///
/// Requires one of its ancestors to be a `Material` widget. The [`label`](Self::label) and
/// [`is_enabled`](Self::is_enabled) must not be null.
///
/// See also:
///
///  * [`Chip`](crate::Chip), a chip that displays information and can be deleted.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * `ActionChip`, represents an action related to primary content.
///  * `CircleAvatar`, which shows images or initials of entities.
///  * `Wrap`, A widget that displays its children in multiple horizontal or
///    vertical runs.
///  * <https://material.io/design/components/chips.html>
#[derive(Clone)]
pub struct InputChip {
    pub key: Option<KeyRef>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub selected: bool,
    pub is_enabled: bool,
    pub on_selected: Option<ValueChanged<bool>>,
    pub delete_icon: Option<WidgetRef>,
    pub on_deleted: Option<Listener>,
    pub delete_icon_color: Option<AnyColor>,
    pub delete_button_tooltip_message: Option<String>,
    pub on_pressed: Option<Listener>,
    pub press_elevation: Option<f64>,
    pub disabled_color: Option<Color>,
    pub selected_color: Option<Color>,
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
    pub delete_icon_box_constraints: Option<BoxConstraints>,
    pub chip_animation_style: Option<ChipAnimationStyle>,
    pub mouse_cursor: Option<MouseCursorRef>,
}

impl_chip_debug!(InputChip);

impl InputChip {
    /// Creates an input chip.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn new<K>(label: impl IntoWidget<K>) -> InputChip {
        InputChip {
            key: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            label_padding: None,
            selected: false,
            is_enabled: true,
            on_selected: None,
            delete_icon: None,
            on_deleted: None,
            delete_icon_color: None,
            delete_button_tooltip_message: None,
            on_pressed: None,
            press_elevation: None,
            disabled_color: None,
            selected_color: None,
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
            delete_icon_box_constraints: None,
            chip_animation_style: None,
            mouse_cursor: None,
        }
    }

    chip_option_setters!(
        InputChip;
        key: KeyRef,
        label_style: TextStyle,
        on_selected: ValueChanged<bool>,
        on_deleted: Listener,
        delete_button_tooltip_message: String,
        on_pressed: Listener,
        disabled_color: Color,
        selected_color: Color,
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
        delete_icon_box_constraints: BoxConstraints,
        chip_animation_style: ChipAnimationStyle,
        mouse_cursor: MouseCursorRef,
    );
    chip_value_setters!(
        InputChip;
        selected: bool,
        is_enabled: bool,
        clip_behavior: Clip,
        autofocus: bool,
        avatar_border: ShapeBorderRef,
    );
    chip_widget_setters!(InputChip; avatar, delete_icon);
    chip_into_setters!(
        InputChip;
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        delete_icon_color: AnyColor,
        shape: OutlinedBorderRef,
        padding: EdgeInsetsGeometry,
    );
    chip_non_negative_setters!(InputChip; press_elevation, elevation);
}

impl StatelessWidget for InputChip {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let use_material3 = Theme::of(app, context).use_material3;
        let mut raw_chip = RawChip::new(self.label.clone());
        if use_material3 {
            raw_chip.default_properties = Some(Rc::new(InputChipDefaultsM3::new(
                context,
                self.is_enabled,
                self.selected,
            )));
        }
        raw_chip.avatar = self.avatar.clone();
        raw_chip.label_style = self.label_style.clone();
        raw_chip.label_padding = self.label_padding;
        if let Some(delete_icon) = self.delete_icon.clone() {
            raw_chip.delete_icon = delete_icon;
        } else if use_material3 {
            raw_chip.delete_icon = Icon::new(Some(Icons::clear())).size(18.0).into_widget();
        }
        raw_chip.on_deleted = self.on_deleted.clone();
        raw_chip.delete_icon_color = self.delete_icon_color.clone();
        raw_chip.delete_button_tooltip_message = self.delete_button_tooltip_message.clone();
        raw_chip.on_selected = self.on_selected.clone();
        raw_chip.on_pressed = self.on_pressed.clone();
        raw_chip.press_elevation = self.press_elevation;
        raw_chip.selected = self.selected;
        raw_chip.disabled_color = self.disabled_color;
        raw_chip.selected_color = self.selected_color;
        raw_chip.tooltip = self.tooltip.clone();
        raw_chip.side = self.side.clone();
        raw_chip.shape = self.shape.clone();
        raw_chip.clip_behavior = self.clip_behavior;
        raw_chip.focus_node = self.focus_node.clone();
        raw_chip.autofocus = self.autofocus;
        raw_chip.color = self.color.clone();
        raw_chip.background_color = self.background_color;
        raw_chip.padding = self.padding;
        raw_chip.visual_density = self.visual_density;
        raw_chip.material_tap_target_size = self.material_tap_target_size;
        raw_chip.elevation = self.elevation;
        raw_chip.shadow_color = self.shadow_color;
        raw_chip.surface_tint_color = self.surface_tint_color;
        raw_chip.selected_shadow_color = self.selected_shadow_color;
        raw_chip.show_checkmark = self.show_checkmark;
        raw_chip.checkmark_color = self.checkmark_color;
        raw_chip.is_enabled = self.is_enabled
            && (self.on_selected.is_some()
                || self.on_deleted.is_some()
                || self.on_pressed.is_some());
        raw_chip.avatar_border = self.avatar_border.clone();
        raw_chip.icon_theme = self.icon_theme.clone();
        raw_chip.avatar_box_constraints = self.avatar_box_constraints;
        raw_chip.delete_icon_box_constraints = self.delete_icon_box_constraints;
        raw_chip.chip_animation_style = self.chip_animation_style.clone();
        raw_chip.mouse_cursor = self.mouse_cursor.clone();
        raw_chip.into_widget()
    }
}

impl_chip_attributes!(InputChip);
impl_deletable_chip_attributes!(InputChip, |chip| chip.delete_icon.as_ref());
impl_selectable_chip_attributes!(InputChip);
impl_checkmarkable_chip_attributes!(InputChip);
impl_disabled_chip_attributes!(InputChip, |chip| chip.is_enabled);
impl_tappable_chip_attributes!(InputChip);

#[cfg(test)]
mod tests {

    use waft_animation::AnimationStyle;
    use waft_embedder::{PointerChange, Size};
    use waft_foundation::K_IS_WEB;
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Center, Column, Container, FocusHighlightStrategy, FocusNode, FocusNodeLeaf, FocusScope,
        Icon, SizedBox, Text, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};

    use super::*;
    use crate::action_chip::tests::{
        hover_over_chip, material_of, paints_path_color, paints_rrect_color, pump_wrapped,
    };
    use crate::chip::tests::{icon_data, only, text};
    use crate::circle_avatar::CircleAvatar;
    use crate::colors::Colors;
    use crate::filter_chip::tests::{
        check_chip_material_clip_behavior, expect_checkmark_color, pump_checkmark_chip,
        tooltip_container_count,
    };
    use crate::material::Material;
    use crate::motion::Durations;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;

    fn select_listener() -> ValueChanged<bool> {
        Rc::new(|_app, _selected| {})
    }

    /// Dart's `selectedInputChip`.
    fn selected_input_chip(checkmark_color: Option<Color>, enabled: bool) -> InputChip {
        let mut chip = InputChip::new(Text::new("InputChip"))
            .selected(true)
            .is_enabled(enabled)
            // When `enabled` is true we also need to provide one of the chip callbacks,
            // otherwise the chip would have a 'disabled' `WidgetState`, which is not the
            // intention.
            .show_checkmark(true);
        if enabled {
            chip = chip.on_selected(select_listener());
        }
        if let Some(color) = checkmark_color {
            chip = chip.checkmark_color(color);
        }
        chip
    }

    #[test]
    fn input_chip_color_resolves_material_states() {
        test_widgets(|tester| {
            let disabled_selected_color = Color::new(0xffffff00);
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
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
                let mut chip = InputChip::new(Text::new("InputChip"))
                    .selected(selected)
                    .color(color);
                if enabled {
                    chip = chip.on_selected(select_listener());
                }
                pump_wrapped(tester, chip);
            };
            let paints = |tester: &WidgetTester, color: Color| {
                paints_rrect_color(tester, only::<RawChip>(tester), color)
            };

            // Test enabled chip.
            build_app(tester, true, false);
            // Enabled chip should have the provided backgroundColor.
            assert!(paints(tester, background_color));

            // Test disabled chip.
            build_app(tester, false, false);
            tester.pump_and_settle();
            // Disabled chip should have the provided disabledColor.
            assert!(paints(tester, disabled_color));

            // Test enabled & selected chip.
            build_app(tester, true, true);
            tester.pump_and_settle();
            // Enabled & selected chip should have the provided selectedColor.
            assert!(paints(tester, selected_color));

            // Test disabled & selected chip.
            build_app(tester, false, true);
            tester.pump_and_settle();
            // Disabled & selected chip should have the provided disabledSelectedColor.
            assert!(paints(tester, disabled_selected_color));
        });
    }

    #[test]
    fn input_chip_uses_provided_state_color_properties() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                let mut chip = InputChip::new(Text::new("InputChip"))
                    .selected(selected)
                    .disabled_color(disabled_color)
                    .background_color(background_color)
                    .selected_color(selected_color);
                if enabled {
                    chip = chip.on_selected(select_listener());
                }
                pump_wrapped(tester, chip);
            };
            let paints = |tester: &WidgetTester, color: Color| {
                paints_rrect_color(tester, only::<RawChip>(tester), color)
            };

            // Test enabled chip.
            build_app(tester, true, false);
            assert!(paints(tester, background_color));

            // Test disabled chip.
            build_app(tester, false, false);
            tester.pump_and_settle();
            assert!(paints(tester, disabled_color));

            // Test enabled & selected chip.
            build_app(tester, true, true);
            tester.pump_and_settle();
            assert!(paints(tester, selected_color));
        });
    }

    #[test]
    fn input_chip_can_be_tapped() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(InputChip::new(Text::new("input chip")))
                    .into_widget(),
            ));

            let center = tester.get_center(only::<InputChip>(tester));
            tester.tap_at(center, 1);
        });
    }

    #[test]
    fn input_chip_loses_focus_when_disabled() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            let build_chip = |tester: &mut WidgetTester, enabled: bool| {
                let mut chip = InputChip::new(Text::new("Chip A"))
                    .focus_node(focus_node.erase())
                    .autofocus(true)
                    .shape(waft_painting::RoundedRectangleBorder::default())
                    .avatar(CircleAvatar::new().child(Text::new("A")));
                if enabled {
                    chip = chip.on_pressed(Listener::new(|_| {}));
                }
                pump_wrapped(tester, chip);
            };

            build_chip(tester, true);
            tester.pump();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

            build_chip(tester, false);
            tester.pump();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn input_chip_cannot_be_traversed_to_when_disabled() {
        test_widgets(|tester| {
            let focus_node1: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            let focus_node2: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            pump_wrapped(
                tester,
                FocusScope::new(
                    Column::new().children([
                        InputChip::new(Text::new("Chip A"))
                            .focus_node(focus_node1.clone())
                            .autofocus(true)
                            .on_pressed(Listener::new(|_| {}))
                            .into_widget(),
                        InputChip::new(Text::new("Chip B"))
                            .focus_node(focus_node2.clone())
                            .autofocus(true)
                            .into_widget(),
                    ]),
                ),
            );
            tester.pump();
            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));

            assert!(!focus_node1.next_focus(&mut tester.cell.borrow_mut()));

            tester.pump();
            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));
        });
    }

    fn disabled_checkmark_color_case(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        expected: impl FnOnce(&ThemeData) -> Color,
    ) {
        let theme = theme(&tester.cell.borrow());
        let expected = expected(&theme);
        pump_checkmark_chip(tester, selected_input_chip(None, false), None, Some(theme));
        expect_checkmark_color(tester, only::<InputChip>(tester), expected);
    }

    #[test]
    fn material2_input_chip_disabled_check_mark_color_is_determined_by_platform_brightness_when_light()
     {
        test_widgets(|tester| {
            disabled_checkmark_color_case(
                tester,
                |app| ThemeData::light(app, Some(false)),
                |_| Colors::BLACK.with_alpha(0xde),
            );
        });
    }

    #[test]
    fn material3_input_chip_disabled_check_mark_color_is_determined_by_platform_brightness_when_light()
     {
        test_widgets(|tester| {
            disabled_checkmark_color_case(tester, m3, |theme| theme.color_scheme.on_surface());
        });
    }

    #[test]
    fn material2_input_chip_disabled_check_mark_color_is_determined_by_platform_brightness_when_dark()
     {
        test_widgets(|tester| {
            disabled_checkmark_color_case(
                tester,
                |app| ThemeData::dark(app, Some(false)),
                |_| Colors::WHITE.with_alpha(0xde),
            );
        });
    }

    #[test]
    fn material3_input_chip_disabled_check_mark_color_is_determined_by_platform_brightness_when_dark()
     {
        test_widgets(|tester| {
            disabled_checkmark_color_case(
                tester,
                |app| ThemeData::dark(app, None),
                |theme| theme.color_scheme.on_surface(),
            );
        });
    }

    #[test]
    fn input_chip_check_mark_color_can_be_set_by_the_chip_theme() {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_input_chip(None, false),
                Some(Color::new(0xff00ff00)),
                None,
            );
            expect_checkmark_color(tester, only::<InputChip>(tester), Color::new(0xff00ff00));
        });
    }

    #[test]
    fn input_chip_check_mark_color_can_be_set_by_the_chip_constructor() {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_input_chip(Some(Color::new(0xff00ff00)), false),
                None,
                None,
            );
            expect_checkmark_color(tester, only::<InputChip>(tester), Color::new(0xff00ff00));
        });
    }

    #[test]
    fn input_chip_check_mark_color_is_set_by_chip_constructor_even_when_a_theme_color_is_specified()
    {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_input_chip(Some(Color::new(0xffff0000)), false),
                Some(Color::new(0xff00ff00)),
                None,
            );
            expect_checkmark_color(tester, only::<InputChip>(tester), Color::new(0xffff0000));
        });
    }

    #[test]
    fn input_chip_clip_behavior_properly_passes_through_to_the_material() {
        test_widgets(|tester| {
            pump_wrapped(tester, InputChip::new(Text::new("label")));
            check_chip_material_clip_behavior(tester, Clip::None);

            pump_wrapped(
                tester,
                InputChip::new(Text::new("label")).clip_behavior(Clip::AntiAlias),
            );
            check_chip_material_clip_behavior(tester, Clip::AntiAlias);
        });
    }

    #[test]
    fn material3_input_chip_has_correct_selected_color_when_enabled() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let expected = theme.color_scheme.secondary_container();
            pump_checkmark_chip(tester, selected_input_chip(None, true), None, Some(theme));

            assert!(paints_rrect_color(
                tester,
                only::<InputChip>(tester),
                expected
            ));
        });
    }

    #[test]
    fn material3_input_chip_has_correct_selected_color_when_disabled() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let expected = theme.color_scheme.on_surface();
            pump_checkmark_chip(tester, selected_input_chip(None, false), None, Some(theme));

            assert!(paints_path_color(
                tester,
                only::<InputChip>(tester),
                expected
            ));
        });
    }

    #[test]
    fn input_chip_uses_provided_icon_theme() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let build_chip = |tester: &mut WidgetTester, icon_theme: Option<IconThemeData>| {
                let mut chip =
                    InputChip::new(Text::new("Test")).avatar(Icon::new(Some(Icons::add())));
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
                Some(theme.color_scheme.on_surface_variant())
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
    fn delete_button_tooltip_is_not_shown_on_disabled_input_chip() {
        test_widgets(|tester| {
            let build_chip = |tester: &mut WidgetTester, enabled: bool| {
                pump_wrapped(
                    tester,
                    InputChip::new(Text::new("Label"))
                        .is_enabled(enabled)
                        .on_deleted(Listener::new(|_| {})),
                );
            };

            // Test enabled chip.
            build_chip(tester, true);

            let delete_button_location = tester.get_center(only::<Icon>(tester));
            let mut gesture = TestPointer::mouse(1, 0);
            tester.move_to(&mut gesture, delete_button_location, false);
            tester.pump();

            // Delete button tooltip should be visible.
            assert_eq!(tooltip_container_count(tester, "Delete"), 1);

            // Test disabled chip.
            build_chip(tester, false);
            tester.pump();

            // Delete button tooltip should not be visible.
            assert_eq!(tooltip_container_count(tester, "Delete"), 0);
        });
    }

    #[test]
    fn input_chip_avatar_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = InputChip::new(
                    Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                )
                .avatar(Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| tester.get_size(only::<InputChip>(tester));
            let chip_top_left = |tester: &WidgetTester| {
                tester
                    .get_rect(material_of(tester, only::<RawChip>(tester)))
                    .top_left()
            };
            let label_top_left =
                |tester: &WidgetTester| tester.get_rect(only::<Container>(tester)).top_left();

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

    #[test]
    fn input_chip_delete_icon_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = InputChip::new(
                    Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                )
                .on_deleted(Listener::new(|_| {}));
                if let Some(constraints) = constraints {
                    chip = chip.delete_icon_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| tester.get_size(only::<InputChip>(tester));
            let chip_top_right = |tester: &WidgetTester| {
                tester
                    .get_rect(material_of(tester, only::<RawChip>(tester)))
                    .top_right()
            };
            let label_top_right =
                |tester: &WidgetTester| tester.get_rect(only::<Container>(tester)).top_right();

            // Test default delete icon layout constraints.
            build_chip(tester, None);
            assert_eq!(chip_size(tester).width(), 234.0);
            assert_eq!(chip_size(tester).height(), 118.0);

            // Calculate the distance between delete icon and chip edges.
            let top_right = chip_top_right(tester);
            let delete_icon_center =
                tester.get_center(tester.element(find::by_icon(&Icons::clear()).first()));
            assert_eq!(
                top_right.dx(),
                delete_icon_center.dx() + (label_size.width() / 2.0) + padding + border
            );
            assert_eq!(
                top_right.dy(),
                delete_icon_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between delete icon and label.
            assert_eq!(
                label_top_right(tester).dx(),
                delete_icon_center.dx() - (label_size.width() / 2.0) - label_padding
            );

            // Test custom delete icon layout constraints.
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

            // Calculate the distance between delete icon and chip edges.
            let top_right = chip_top_right(tester);
            assert_eq!(
                top_right.dx(),
                delete_icon_center.dx() + (icon_size / 2.0) + padding + border
            );
            assert_eq!(
                top_right.dy(),
                delete_icon_center.dy() - (label_size.width() / 2.0) - padding - border
            );

            // Calculate the distance between delete icon and label.
            assert_eq!(
                label_top_right(tester).dx(),
                delete_icon_center.dx() - (icon_size / 2.0) - label_padding
            );
        });
    }

    #[test]
    fn input_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            let chip_animation_style = ChipAnimationStyle::new()
                .enable_animation(AnimationStyle::new(
                    None,
                    Some(Durations::SHORT2),
                    None,
                    None,
                ))
                .select_animation(AnimationStyle::NO_ANIMATION);
            pump_wrapped(
                tester,
                Center::new().child(
                    InputChip::new(Text::new("InputChip"))
                        .chip_animation_style(chip_animation_style.clone()),
                ),
            );

            let passed = tester
                .widget(only::<RawChip>(tester), |raw_chip: &RawChip| {
                    raw_chip.chip_animation_style.clone()
                })
                .expect("a chip animation style");
            assert_eq!(
                passed.enable_animation,
                chip_animation_style.enable_animation
            );
            assert_eq!(
                passed.select_animation,
                chip_animation_style.select_animation
            );
            assert_eq!(
                passed.avatar_drawer_animation,
                chip_animation_style.avatar_drawer_animation
            );
            assert_eq!(
                passed.delete_drawer_animation,
                chip_animation_style.delete_drawer_animation
            );
        });
    }

    #[test]
    fn input_chip_has_expected_default_mouse_cursor_on_hover() {
        test_widgets(|tester| {
            pump_wrapped(
                tester,
                Center::new()
                    .child(InputChip::new(Text::new("Chip")).on_pressed(Listener::new(|_| {}))),
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
    fn input_chip_mouse_cursor_behavior() {
        test_widgets(|tester| {
            let custom_cursor: MouseCursorRef = SystemMouseCursors::GRAB.into();
            pump_wrapped(
                tester,
                Center::new()
                    .child(InputChip::new(Text::new("Chip")).mouse_cursor(custom_cursor.clone())),
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
                let mut chip = InputChip::new(Text::new("Chip"))
                    .mouse_cursor(cursor.into())
                    .focus_node(focus_node.erase());
                if enabled {
                    chip = chip.on_selected(select_listener());
                }
                pump_wrapped(tester, Center::new().child(chip));
            };

            // Unfocused case.
            build_chip(tester, true);
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
    fn input_chip_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let mut test_chip = |chip: InputChip| {
                tester.pump_widget(material_app(
                    theme.clone(),
                    Scaffold::new()
                        .body(Center::new().child(SizedBox::shrink().child(chip)))
                        .into_widget(),
                ));
                assert!(tester.get_size(only::<InputChip>(tester)).is_empty());
            };

            test_chip(InputChip::new(Text::new("X")));
            test_chip(
                InputChip::new(Text::new("X")).avatar(CircleAvatar::new().child(Text::new("A"))),
            );
            test_chip(InputChip::new(Text::new("X")).on_deleted(Listener::new(|_| {})));
        });
    }
}
