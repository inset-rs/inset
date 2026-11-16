//! Flutter counterpart: material_ui's `filter_chip.dart` with its
//! `generated/filter_chip_defaults_m3.g.dart`.

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
    ChipAnimationStyle, ChipVariant, DisabledChipAttributes, RawChip, chip_into_setters,
    chip_non_negative_setters, chip_option_setters, chip_value_setters, chip_widget_setters,
    impl_checkmarkable_chip_attributes, impl_chip_attributes, impl_chip_debug,
    impl_deletable_chip_attributes, impl_disabled_chip_attributes, impl_selectable_chip_attributes,
};
use crate::debug::debug_check_has_material;
use crate::generated::filter_chip_defaults_m3::FilterChipDefaultsM3;
use crate::icons::Icons;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// A Material Design filter chip.
///
/// Filter chips use tags or descriptive words as a way to filter content.
///
/// Filter chips are a good alternative to [`Checkbox`](crate::Checkbox) or
/// `Switch` widgets. Unlike these alternatives, filter chips allow for clearly
/// delineated and exposed options in a compact area.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// ## Material Design 3
///
/// [`FilterChip`] can be used for multiple select Filter chips from
/// Material Design 3. If `ThemeData::use_material3` is true, then [`FilterChip`]
/// will be styled to match the Material Design 3 specification for Filter
/// chips. Use `ChoiceChip` for single select Filter chips.
///
/// See also:
///
///  * [`Chip`](crate::Chip), a chip that displays information and can be deleted.
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `ActionChip`, represents an action related to primary content.
///  * `CircleAvatar`, which shows images or initials of people.
///  * `Wrap`, A widget that displays its children in multiple horizontal or
///    vertical runs.
///  * <https://material.io/design/components/chips.html>
#[derive(Clone)]
pub struct FilterChip {
    pub key: Option<KeyRef>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub selected: bool,
    pub on_selected: Option<ValueChanged<bool>>,
    pub delete_icon: Option<WidgetRef>,
    pub on_deleted: Option<Listener>,
    pub delete_icon_color: Option<AnyColor>,
    pub delete_button_tooltip_message: Option<String>,
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
    chip_variant: ChipVariant,
}

impl_chip_debug!(FilterChip);

impl FilterChip {
    /// Create a chip that acts like a checkbox.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn new<K>(
        label: impl IntoWidget<K>,
        on_selected: Option<ValueChanged<bool>>,
    ) -> FilterChip {
        FilterChip {
            key: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            label_padding: None,
            selected: false,
            on_selected,
            delete_icon: None,
            on_deleted: None,
            delete_icon_color: None,
            delete_button_tooltip_message: None,
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
            chip_variant: ChipVariant::Flat,
        }
    }

    /// Create an elevated chip that acts like a checkbox.
    ///
    /// The [`elevation`](Self::elevation) and [`press_elevation`](Self::press_elevation) must be null or non-negative. Dart's named arguments are the setters.
    pub fn elevated<K>(
        label: impl IntoWidget<K>,
        on_selected: Option<ValueChanged<bool>>,
    ) -> FilterChip {
        FilterChip {
            chip_variant: ChipVariant::Elevated,
            ..FilterChip::new(label, on_selected)
        }
    }

    chip_option_setters!(
        FilterChip;
        key: KeyRef,
        label_style: TextStyle,
        on_deleted: Listener,
        delete_button_tooltip_message: String,
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
        FilterChip;
        selected: bool,
        clip_behavior: Clip,
        autofocus: bool,
        avatar_border: ShapeBorderRef,
    );
    chip_widget_setters!(FilterChip; avatar, delete_icon);
    chip_into_setters!(
        FilterChip;
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        delete_icon_color: AnyColor,
        shape: OutlinedBorderRef,
        padding: EdgeInsetsGeometry,
    );
    chip_non_negative_setters!(FilterChip; press_elevation, elevation);
}

impl StatelessWidget for FilterChip {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let use_material3 = Theme::of(app, context).use_material3;
        let is_enabled = DisabledChipAttributes::is_enabled(self);
        let mut raw_chip = RawChip::new(self.label.clone());
        if use_material3 {
            raw_chip.default_properties = Some(Rc::new(FilterChipDefaultsM3::new(
                context,
                is_enabled,
                self.selected,
                self.chip_variant,
            )));
        }
        raw_chip.avatar = self.avatar.clone();
        raw_chip.label_style = self.label_style.clone();
        raw_chip.label_padding = self.label_padding;
        raw_chip.on_selected = self.on_selected.clone();
        if let Some(delete_icon) = self.delete_icon.clone() {
            raw_chip.delete_icon = delete_icon;
        } else if use_material3 {
            raw_chip.delete_icon = Icon::new(Some(Icons::clear())).size(18.0).into_widget();
        }
        raw_chip.on_deleted = self.on_deleted.clone();
        raw_chip.delete_icon_color = self.delete_icon_color.clone();
        raw_chip.delete_button_tooltip_message = self.delete_button_tooltip_message.clone();
        raw_chip.press_elevation = self.press_elevation;
        raw_chip.selected = self.selected;
        raw_chip.tooltip = self.tooltip.clone();
        raw_chip.side = self.side.clone();
        raw_chip.shape = self.shape.clone();
        raw_chip.clip_behavior = self.clip_behavior;
        raw_chip.focus_node = self.focus_node.clone();
        raw_chip.autofocus = self.autofocus;
        raw_chip.color = self.color.clone();
        raw_chip.background_color = self.background_color;
        raw_chip.disabled_color = self.disabled_color;
        raw_chip.selected_color = self.selected_color;
        raw_chip.padding = self.padding;
        raw_chip.visual_density = self.visual_density;
        raw_chip.is_enabled = is_enabled;
        raw_chip.material_tap_target_size = self.material_tap_target_size;
        raw_chip.elevation = self.elevation;
        raw_chip.shadow_color = self.shadow_color;
        raw_chip.surface_tint_color = self.surface_tint_color;
        raw_chip.selected_shadow_color = self.selected_shadow_color;
        raw_chip.show_checkmark = self.show_checkmark;
        raw_chip.checkmark_color = self.checkmark_color;
        raw_chip.avatar_border = self.avatar_border.clone();
        raw_chip.icon_theme = self.icon_theme.clone();
        raw_chip.avatar_box_constraints = self.avatar_box_constraints;
        raw_chip.delete_icon_box_constraints = self.delete_icon_box_constraints;
        raw_chip.chip_animation_style = self.chip_animation_style.clone();
        raw_chip.mouse_cursor = self.mouse_cursor.clone();
        raw_chip.into_widget()
    }
}

impl_chip_attributes!(FilterChip);
impl_deletable_chip_attributes!(FilterChip, |chip| chip.delete_icon.as_ref());
impl_selectable_chip_attributes!(FilterChip);
impl_checkmarkable_chip_attributes!(FilterChip);
impl_disabled_chip_attributes!(FilterChip, |chip| chip.on_selected.is_some());

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_animation::AnimationStyle;
    use waft_embedder::{PointerChange, Size};
    use waft_foundation::K_IS_WEB;
    use waft_painting::{
        BorderRadiusGeometry, BorderSide, RoundedRectangleBorder, ShapeBorderRef, StadiumBorder,
    };
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Builder, Center, Column, Container, FocusHighlightStrategy, FocusNode,
        FocusNodeLeaf, Icon, IconData, SizedBox, Text, WidgetState, WidgetStateMouseCursor,
        WidgetStateProperty,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::action_chip::tests::{
        hover_over_chip, material_of, paints_rrect_color, pump_wrapped, raw_chips,
    };
    use crate::chip::tests::{
        center_of, chip_material_of, close_to, icon_data, ink_decoration_color, label_style_color,
        only, rect_of, size_of, text,
    };
    use crate::chip_theme::ChipTheme;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::motion::Durations;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};

    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3};
    use waft_rendering::testing::{PaintCall, paints};

    const LABEL: &str = "filter chip";

    fn select_listener() -> ValueChanged<bool> {
        Rc::new(|_app, _selected| {})
    }

    fn filter_chip(label: &str, enabled: bool) -> FilterChip {
        FilterChip::new(Text::new(label.to_owned()), enabled.then(select_listener))
    }

    fn pump_chip(tester: &mut WidgetTester, theme: &ThemeData, chip: FilterChip, centered: bool) {
        let home = if centered {
            Material::new()
                .child(Center::new().child(chip))
                .into_widget()
        } else {
            Material::new().child(chip).into_widget()
        };
        tester.pump_widget(material_app(theme.clone(), home));
    }

    /// Dart's `expectCheckmarkColor(finder, color)`: the selection overlay's rrect and then the
    /// check mark's path of `color`. Dart's first `..path()` is the physical model's, a rounded
    /// rect here too (Waft tells `drawRRect` from `drawPath` by the path), so the pattern starts
    /// at the first rounded rect: the check mark is the next path after it.
    pub(crate) fn expect_checkmark_color(tester: &WidgetTester, chip: AnyElement, color: Color) {
        let ops = tester.painted_ops_deep(tester.render_box_of(chip).as_object());
        assert!(
            paints(
                &ops,
                &[PaintCall::rrect(None), PaintCall::path(Some(color))]
            )
            .is_ok(),
            "no check mark path of {color:?} after a rounded rect"
        );
    }

    /// Dart's `pumpCheckmarkChip`: the chip under a `ChipTheme` with `theme_color` as its
    /// checkmark color when one is given.
    pub(crate) fn pump_checkmark_chip<C: StatelessWidget + Clone + 'static>(
        tester: &mut WidgetTester,
        chip: C,
        theme_color: Option<Color>,
        theme: Option<ThemeData>,
    ) {
        let child = Builder::new(move |app, context| {
            let chip_theme = ChipTheme::of(app, context);
            let data = match theme_color {
                None => chip_theme,
                Some(color) => chip_theme.copy_with().checkmark_color(color),
            };
            ChipTheme::new(data, chip.clone()).into_widget()
        });
        let root = crate::chip::tests::wrap_for_chip(
            tester,
            child.into_widget(),
            waft_widgets::TextDirection::Ltr,
            waft_painting::TextScaler::NO_SCALING,
            theme,
        );
        tester.pump_widget(root);
    }

    fn selected_filter_chip(checkmark_color: Option<Color>) -> FilterChip {
        let mut chip = FilterChip::new(Text::new("InputChip"), Some(select_listener()))
            .selected(true)
            .show_checkmark(true);
        if let Some(color) = checkmark_color {
            chip = chip.checkmark_color(color);
        }
        chip
    }

    /// Dart's `checkChipMaterialClipBehavior`.
    pub(crate) fn check_chip_material_clip_behavior(tester: &WidgetTester, clip_behavior: Clip) {
        let materials = tester.find_by_type::<Material>();
        // There should be two Material widgets, first Material is from the "_wrapForChip" and
        // last Material is from the "RawChip".
        assert_eq!(materials.len(), 2);
        let last = *materials.last().expect("a Material");
        assert_eq!(
            tester.widget(last, |material: &Material| material.clip_behavior),
            clip_behavior
        );
    }

    /// The `Container`s above the text `tooltip_text`: Dart's `findTooltipContainer`.
    pub(crate) fn tooltip_container_count(tester: &WidgetTester, tooltip_text: &str) -> usize {
        tester
            .find_text(tooltip_text)
            .into_iter()
            .flat_map(|text| tester.ancestors(text))
            .filter(|ancestor| is_widget_of_type::<Container>(tester, *ancestor))
            .count()
    }

    /// Dart's `find.byIcon(icon)`, counted.
    pub(crate) fn icons_found(tester: &WidgetTester, icon: &IconData) -> usize {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    fn is_widget_of_type<W: 'static>(tester: &WidgetTester, element: AnyElement) -> bool {
        let app = tester.cell.borrow();
        waft_widgets::downcast_widget::<W>(&**element.widget(&app)).is_some()
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

    fn stadium() -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
    }

    fn body_large_color(theme: &ThemeData) -> Option<Color> {
        theme
            .text_theme
            .body_large
            .as_ref()
            .and_then(|style| style.color.as_ref())
            .map(|color| color.color())
    }

    fn assert_material(
        tester: &WidgetTester,
        elevation: f64,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Option<Color>>,
        shape: Option<ShapeBorderRef>,
    ) {
        let (actual_elevation, actual_shadow, actual_tint, actual_shape) =
            chip_material_of(tester, only::<RawChip>(tester));
        assert_eq!(actual_elevation, elevation);
        assert_eq!(actual_shadow, shadow_color);
        if let Some(surface_tint_color) = surface_tint_color {
            assert_eq!(actual_tint, surface_tint_color);
        }
        assert_eq!(actual_shape, shape);
    }

    #[test]
    fn material2_filter_chip_defaults() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());

            // Test enabled FilterChip defaults.
            pump_chip(tester, &theme, filter_chip(LABEL, true), true);

            // Test default chip size.
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)),
                Size::new(178.0, 48.0)
            );
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                body_large_color(&theme).map(|color| color.with_alpha(0xde))
            );
            assert_material(tester, 0.0, Some(Colors::BLACK), None, stadium());
            assert_eq!(
                ink_decoration_color(tester),
                Some(Colors::BLACK.with_alpha(0x1f))
            );

            // Test disabled FilterChip defaults.
            pump_chip(tester, &theme, filter_chip(LABEL, false), false);
            tester.pump_and_settle();
            assert_material(tester, 0.0, Some(Colors::BLACK), None, stadium());
            assert_eq!(ink_decoration_color(tester), Some(Colors::BLACK38));

            // Test selected enabled FilterChip defaults.
            pump_chip(
                tester,
                &theme,
                filter_chip(LABEL, true).selected(true),
                false,
            );
            tester.pump_and_settle();
            assert_material(tester, 0.0, Some(Colors::BLACK), None, stadium());
            assert_eq!(
                ink_decoration_color(tester),
                Some(Colors::BLACK.with_alpha(0x3d))
            );

            // Test selected disabled FilterChip defaults.
            pump_chip(
                tester,
                &theme,
                filter_chip(LABEL, false).selected(true),
                false,
            );
            tester.pump_and_settle();
            assert_material(tester, 0.0, Some(Colors::BLACK), None, stadium());
            assert_eq!(
                ink_decoration_color(tester),
                Some(Colors::BLACK.with_alpha(0x3d))
            );
        });
    }

    #[test]
    fn material3_filter_chip_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let scheme = &theme.color_scheme;
            let transparent = Some(Some(Colors::TRANSPARENT));

            // Test enabled FilterChip defaults.
            pump_chip(tester, &theme, filter_chip(LABEL, true), true);

            // Test default chip size.
            let size = size_of(tester, only::<FilterChip>(tester));
            close_to(size.width(), 189.1, 0.001);
            close_to(size.height(), 48.0, 0.001);
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(scheme.on_surface_variant())
            );
            assert_material(
                tester,
                0.0,
                Some(Colors::TRANSPARENT),
                transparent,
                rounded_rectangle(scheme.outline_variant()),
            );
            assert_eq!(ink_decoration_color(tester), None);

            // Test disabled FilterChip defaults.
            pump_chip(tester, &theme, filter_chip(LABEL, false), false);
            tester.pump_and_settle();
            assert_material(
                tester,
                0.0,
                Some(Colors::TRANSPARENT),
                transparent,
                rounded_rectangle(scheme.on_surface.with_alpha(0x1f)),
            );
            assert_eq!(ink_decoration_color(tester), None);

            // Test selected enabled FilterChip defaults.
            pump_chip(
                tester,
                &theme,
                filter_chip(LABEL, true).selected(true),
                false,
            );
            tester.pump_and_settle();
            assert_material(
                tester,
                0.0,
                None,
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.secondary_container())
            );

            // Test selected disabled FilterChip defaults.
            pump_chip(
                tester,
                &theme,
                filter_chip(LABEL, false).selected(true),
                false,
            );
            tester.pump_and_settle();
            assert_material(
                tester,
                0.0,
                None,
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.on_surface.with_alpha(0x1f))
            );
        });
    }

    fn elevated(label: &str, enabled: bool) -> FilterChip {
        FilterChip::elevated(Text::new(label.to_owned()), enabled.then(select_listener))
    }

    #[test]
    fn material3_filter_chip_elevated_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let scheme = &theme.color_scheme;
            let transparent = Some(Some(Colors::TRANSPARENT));

            // Test enabled FilterChip.elevated defaults.
            pump_chip(tester, &theme, elevated(LABEL, true), true);

            // Test default chip size.
            let size = size_of(tester, only::<FilterChip>(tester));
            close_to(size.width(), 189.1, 0.001);
            close_to(size.height(), 48.0, 0.001);
            // Test default label style.
            assert_eq!(
                label_style_color(tester, LABEL),
                Some(scheme.on_surface_variant())
            );
            assert_material(
                tester,
                1.0,
                Some(scheme.shadow()),
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.surface_container_low())
            );

            // Test disabled FilterChip.elevated defaults.
            pump_chip(tester, &theme, elevated(LABEL, false), false);
            tester.pump_and_settle();
            assert_material(
                tester,
                0.0,
                Some(scheme.shadow()),
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.on_surface.with_alpha(0x1f))
            );

            // Test selected enabled FilterChip.elevated defaults.
            pump_chip(tester, &theme, elevated(LABEL, true).selected(true), false);
            tester.pump_and_settle();
            assert_material(
                tester,
                1.0,
                None,
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.secondary_container())
            );

            // Test selected disabled FilterChip.elevated defaults.
            pump_chip(tester, &theme, elevated(LABEL, false).selected(true), false);
            tester.pump_and_settle();
            assert_material(
                tester,
                0.0,
                None,
                transparent,
                rounded_rectangle(Colors::TRANSPARENT),
            );
            assert_eq!(
                ink_decoration_color(tester),
                Some(scheme.on_surface.with_alpha(0x1f))
            );
        });
    }

    fn two_chips(
        color: Option<WidgetStatePropertyRef<Option<Color>>>,
        colors: Option<(Color, Color, Color)>,
        enabled: bool,
        selected: bool,
    ) -> Column {
        let build = |mut chip: FilterChip| {
            chip = chip.selected(selected);
            if let Some(color) = &color {
                chip = chip.color(color.clone());
            }
            if let Some((disabled, background, selected_color)) = colors {
                chip = chip
                    .disabled_color(disabled)
                    .background_color(background)
                    .selected_color(selected_color);
            }
            chip.into_widget()
        };
        Column::new().children([
            build(filter_chip("FilterChip", enabled)),
            build(elevated("FilterChip.elevated", enabled)),
        ])
    }

    /// Whether both chips of `two_chips` paint `color`.
    fn both_chips_paint(tester: &WidgetTester, color: Color) -> bool {
        let chips = raw_chips(tester);
        paints_rrect_color(tester, chips[0], color) && paints_rrect_color(tester, chips[1], color)
    }

    #[test]
    fn filter_chip_color_resolves_material_states() {
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
            let build = |enabled: bool, selected: bool| {
                two_chips(Some(color.clone()), None, enabled, selected)
            };

            // Test enabled state.
            pump_wrapped(tester, build(true, false));
            // Enabled FilterChip and elevated FilterChip should have the provided
            // backgroundColor.
            assert!(both_chips_paint(tester, background_color));

            // Test disabled state.
            pump_wrapped(tester, build(false, false));
            tester.pump_and_settle();
            assert!(both_chips_paint(tester, disabled_color));

            // Test enabled & selected state.
            pump_wrapped(tester, build(true, true));
            tester.pump_and_settle();
            assert!(both_chips_paint(tester, selected_color));

            // Test disabled & selected state.
            pump_wrapped(tester, build(false, true));
            tester.pump_and_settle();
            assert!(both_chips_paint(tester, disabled_selected_color));
        });
    }

    #[test]
    fn filter_chip_uses_provided_state_color_properties() {
        test_widgets(|tester| {
            let disabled_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xff0000ff);
            let selected_color = Color::new(0xffff0000);
            let build = |enabled: bool, selected: bool| {
                two_chips(
                    None,
                    Some((disabled_color, background_color, selected_color)),
                    enabled,
                    selected,
                )
            };

            // Test enabled state.
            pump_wrapped(tester, build(true, false));
            assert!(both_chips_paint(tester, background_color));

            // Test disabled state.
            pump_wrapped(tester, build(false, false));
            tester.pump_and_settle();
            assert!(both_chips_paint(tester, disabled_color));

            // Test enabled & selected state.
            pump_wrapped(tester, build(true, true));
            tester.pump_and_settle();
            assert!(both_chips_paint(tester, selected_color));
        });
    }

    #[test]
    fn filter_chip_can_be_tapped() {
        test_widgets(|tester| {
            let selections = Rc::new(Cell::new(0));
            let on_selected: ValueChanged<bool> = {
                let selections = Rc::clone(&selections);
                Rc::new(move |_app, _selected| selections.set(selections.get() + 1))
            };
            let theme = m3(&tester.cell.borrow());
            pump_chip(
                tester,
                &theme,
                FilterChip::new(Text::new("filter chip"), Some(on_selected)),
                false,
            );

            let center = center_of(tester, only::<FilterChip>(tester));
            tester.tap(center, 1);
            assert_eq!(selections.get(), 1);
        });
    }

    fn checkmark_color_case(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        expected: impl FnOnce(&ThemeData) -> Color,
    ) {
        let theme = theme(&tester.cell.borrow());
        let expected = expected(&theme);
        pump_checkmark_chip(tester, selected_filter_chip(None), None, Some(theme));
        expect_checkmark_color(tester, only::<FilterChip>(tester), expected);
    }

    #[test]
    fn material2_filter_chip_check_mark_color_is_determined_by_platform_brightness_when_light() {
        test_widgets(|tester| {
            checkmark_color_case(
                tester,
                |app| ThemeData::light(app, Some(false)),
                |_| Colors::BLACK.with_alpha(0xde),
            );
        });
    }

    #[test]
    fn material3_filter_chip_check_mark_color_is_determined_by_platform_brightness_when_light() {
        test_widgets(|tester| {
            checkmark_color_case(tester, m3, |theme| {
                theme.color_scheme.on_secondary_container()
            });
        });
    }

    #[test]
    fn material2_filter_chip_check_mark_color_is_determined_by_platform_brightness_when_dark() {
        test_widgets(|tester| {
            checkmark_color_case(
                tester,
                |app| ThemeData::dark(app, Some(false)),
                |_| Colors::WHITE.with_alpha(0xde),
            );
        });
    }

    #[test]
    fn material3_filter_chip_check_mark_color_is_determined_by_platform_brightness_when_dark() {
        test_widgets(|tester| {
            checkmark_color_case(
                tester,
                |app| {
                    ThemeData::builder()
                        .brightness(waft_embedder::Brightness::Dark)
                        .build(app)
                },
                |theme| theme.color_scheme.on_secondary_container(),
            );
        });
    }

    #[test]
    fn filter_chip_check_mark_color_can_be_set_by_the_chip_theme() {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_filter_chip(None),
                Some(Color::new(0xff00ff00)),
                None,
            );
            expect_checkmark_color(tester, only::<FilterChip>(tester), Color::new(0xff00ff00));
        });
    }

    #[test]
    fn filter_chip_check_mark_color_can_be_set_by_the_chip_constructor() {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_filter_chip(Some(Color::new(0xff00ff00))),
                None,
                None,
            );
            expect_checkmark_color(tester, only::<FilterChip>(tester), Color::new(0xff00ff00));
        });
    }

    #[test]
    fn filter_chip_check_mark_color_is_set_by_chip_constructor_even_when_a_theme_color_is_specified()
     {
        test_widgets(|tester| {
            pump_checkmark_chip(
                tester,
                selected_filter_chip(Some(Color::new(0xffff0000))),
                Some(Color::new(0xff00ff00)),
                None,
            );
            expect_checkmark_color(tester, only::<FilterChip>(tester), Color::new(0xffff0000));
        });
    }

    #[test]
    fn filter_chip_clip_behavior_properly_passes_through_to_the_material() {
        test_widgets(|tester| {
            pump_wrapped(tester, filter_chip("label", true));
            check_chip_material_clip_behavior(tester, Clip::None);

            pump_wrapped(
                tester,
                filter_chip("label", true).clip_behavior(Clip::AntiAlias),
            );
            check_chip_material_clip_behavior(tester, Clip::AntiAlias);
        });
    }

    #[test]
    fn material3_width_should_not_change_with_selection() {
        test_widgets(|tester| {
            // For the text "FilterChip" the chip should default to 175 regardless of selection.
            let expected_width = 175.0;
            let theme = m3(&tester.cell.borrow());

            // Unselected
            pump_chip(
                tester,
                &theme,
                filter_chip("FilterChip", true).show_checkmark(false),
                true,
            );
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)).width(),
                expected_width
            );

            // Selected
            pump_chip(
                tester,
                &theme,
                filter_chip("FilterChip", true)
                    .show_checkmark(false)
                    .selected(true),
                true,
            );
            tester.pump_and_settle();
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)).width(),
                expected_width
            );
        });
    }

    #[test]
    fn filter_chip_uses_provided_icon_theme() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let build_chip = |tester: &mut WidgetTester, icon_theme: Option<IconThemeData>| {
                let mut chip =
                    filter_chip("FilterChip", true).avatar(Icon::new(Some(Icons::add())));
                if let Some(icon_theme) = icon_theme {
                    chip = chip.icon_theme(icon_theme);
                }
                pump_chip(tester, &theme, chip, false);
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

    fn with_delete(chip: FilterChip) -> FilterChip {
        chip.on_deleted(Listener::new(|_| {}))
    }

    #[test]
    fn material3_filter_chip_supports_delete_button() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let on_surface_variant = Some(theme.color_scheme.on_surface_variant());
            let delete_icon_color =
                |tester: &WidgetTester| icon_data(tester).color.map(|color| color.color());

            pump_chip(
                tester,
                &theme,
                with_delete(filter_chip("FilterChip", true)),
                true,
            );

            // Test the chip size with delete button.
            assert_eq!(tester.find_text("FilterChip").len(), 1);
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)),
                Size::new(195.0, 48.0)
            );
            // Test the delete button icon.
            assert_eq!(
                size_of(tester, tester.find_by_icon(&Icons::clear())),
                Size::new(18.0, 18.0)
            );
            assert_eq!(delete_icon_color(tester), on_surface_variant);

            pump_chip(
                tester,
                &theme,
                with_delete(elevated("Elevated FilterChip", true)),
                true,
            );

            // Test the elevated chip size with delete button.
            assert_eq!(tester.find_text("Elevated FilterChip").len(), 1);
            let size = size_of(tester, only::<FilterChip>(tester));
            close_to(size.width(), 321.9, 0.001);
            close_to(size.height(), 48.0, 0.001);
            // Test the delete button icon.
            assert_eq!(
                size_of(tester, tester.find_by_icon(&Icons::clear())),
                Size::new(18.0, 18.0)
            );
            assert_eq!(delete_icon_color(tester), on_surface_variant);
        });
    }

    #[test]
    fn material2_filter_chip_supports_delete_button() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let expected = theme
                .icon_theme
                .color
                .as_ref()
                .map(|color| color.color().with_alpha(0xde));
            let delete_icon_color =
                |tester: &WidgetTester| icon_data(tester).color.map(|color| color.color());

            pump_chip(
                tester,
                &theme,
                with_delete(filter_chip("FilterChip", true)),
                true,
            );

            // Test the chip size with delete button.
            assert_eq!(tester.find_text("FilterChip").len(), 1);
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)),
                Size::new(188.0, 48.0)
            );
            // Test the delete button icon.
            assert_eq!(
                size_of(tester, tester.find_by_icon(&Icons::cancel())),
                Size::new(18.0, 18.0)
            );
            assert_eq!(delete_icon_color(tester), expected);

            pump_chip(
                tester,
                &theme,
                with_delete(elevated("Elevated FilterChip", true)),
                true,
            );

            // Test the elevated chip size with delete button.
            assert_eq!(tester.find_text("Elevated FilterChip").len(), 1);
            assert_eq!(
                size_of(tester, only::<FilterChip>(tester)),
                Size::new(314.0, 48.0)
            );
            // Test the delete button icon.
            assert_eq!(
                size_of(tester, tester.find_by_icon(&Icons::cancel())),
                Size::new(18.0, 18.0)
            );
            assert_eq!(delete_icon_color(tester), expected);
        });
    }

    #[test]
    fn customize_filter_chip_delete_button() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let build_chip = |tester: &mut WidgetTester,
                              delete_icon: Option<Icon>,
                              delete_icon_color: Option<Color>,
                              tooltip_message: Option<&str>| {
                let mut chip = with_delete(filter_chip("FilterChip", true));
                if let Some(delete_icon) = delete_icon {
                    chip = chip.delete_icon(delete_icon);
                }
                if let Some(color) = delete_icon_color {
                    chip = chip.delete_icon_color(color);
                }
                if let Some(message) = tooltip_message {
                    chip = chip.delete_button_tooltip_message(message.to_owned());
                }
                pump_chip(tester, &theme, chip, true);
            };

            // Test the custom delete icon.
            build_chip(tester, Some(Icon::new(Some(Icons::delete()))), None, None);
            assert!(icons_found(tester, &Icons::clear()) == 0);
            assert_eq!(icons_found(tester, &Icons::delete()), 1);

            // Test the custom delete icon color.
            build_chip(
                tester,
                Some(Icon::new(Some(Icons::delete()))),
                Some(Color::new(0xff00ff00)),
                None,
            );
            tester.pump_and_settle();
            assert!(icons_found(tester, &Icons::clear()) == 0);
            assert_eq!(icons_found(tester, &Icons::delete()), 1);
            assert_eq!(
                icon_data(tester).color.map(|color| color.color()),
                Some(Color::new(0xff00ff00))
            );

            // Test the custom delete button tooltip message.
            build_chip(tester, None, None, Some("Delete FilterChip"));
            tester.pump_and_settle();

            // Press the delete icon of the chip.
            let mut gesture = TestPointer::touch(1);
            let at = center_of(tester, tester.find_by_icon(&Icons::clear()));
            tester.down(&mut gesture, at);
            tester.pump_and_settle();

            // Verify the tooltip message is set.
            let tooltips = tester
                .find_text("Delete FilterChip")
                .into_iter()
                .filter(|text| {
                    tester.ancestors(*text).into_iter().any(|ancestor| {
                        is_widget_of_type::<crate::tooltip::Tooltip>(tester, ancestor)
                    })
                })
                .count();
            assert_eq!(tooltips, 1);
            tester.up(&gesture);
        });
    }

    #[test]
    fn filter_chip_delete_button_control_test() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let deleted = Rc::new(RefCell::new(Vec::<String>::new()));
            let chip = filter_chip("FilterChip", true).on_deleted({
                let deleted = Rc::clone(&deleted);
                Listener::new(move |_app| deleted.borrow_mut().push("A".to_owned()))
            });
            pump_chip(tester, &theme, chip, true);

            assert!(deleted.borrow().is_empty());

            let at = center_of(tester, tester.find_by_icon(&Icons::clear()));
            tester.tap(at, 1);
            assert_eq!(*deleted.borrow(), vec!["A"]);
            tester.pump_and_settle_by(Duration::from_secs(1));

            tester.tap(at, 1);
            assert_eq!(*deleted.borrow(), vec!["A", "A"]);
            tester.pump_and_settle_by(Duration::from_secs(1));
        });
    }

    #[test]
    fn delete_button_tooltip_is_not_shown_on_disabled_filter_chip() {
        test_widgets(|tester| {
            let build_chip = |tester: &mut WidgetTester, enabled: bool| {
                pump_wrapped(tester, with_delete(filter_chip("Label", enabled)));
            };

            // Test enabled chip.
            build_chip(tester, true);

            let delete_button_location = center_of(tester, only::<Icon>(tester));
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

    /// The color of the nearest `IconTheme` above `icon`.
    pub(crate) fn icon_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
        let icon = tester.find_by_icon(icon);
        let theme = tester
            .ancestors(icon)
            .into_iter()
            .find(|ancestor| is_widget_of_type::<waft_widgets::IconTheme>(tester, *ancestor))
            .expect("an IconTheme above the icon");
        tester.widget(theme, |theme: &waft_widgets::IconTheme| {
            theme.data.color.as_ref().map(|color| color.color())
        })
    }

    #[test]
    fn material3_default_filter_chip_icon_colors() {
        test_widgets(|tester| {
            let flat_avatar = Icons::favorite();
            let flat_delete_icon = Icons::delete();
            let elevated_avatar = Icons::house();
            let elevated_delete_icon = Icons::clear_all();
            let theme = m3(&tester.cell.borrow());
            let scheme = &theme.color_scheme;
            let build_chips = |tester: &mut WidgetTester, selected: bool| {
                let chips = Column::new().children([
                    with_delete(filter_chip("FilterChip", true))
                        .avatar(Icon::new(Some(flat_avatar.clone())))
                        .delete_icon(Icon::new(Some(flat_delete_icon.clone())))
                        .selected(selected)
                        .into_widget(),
                    with_delete(elevated("Elevated FilterChip", true))
                        .avatar(Icon::new(Some(elevated_avatar.clone())))
                        .delete_icon(Icon::new(Some(elevated_delete_icon.clone())))
                        .selected(selected)
                        .into_widget(),
                ]);
                tester.pump_widget(material_app(
                    theme.clone(),
                    Material::new().child(chips).into_widget(),
                ));
            };

            // Test unselected state.
            build_chips(tester, false);

            // Check the flat chip icon colors.
            assert_eq!(icon_color(tester, &flat_avatar), Some(scheme.primary));
            assert_eq!(
                icon_color(tester, &flat_delete_icon),
                Some(scheme.on_surface_variant())
            );
            // Check the elevated chip icon colors.
            assert_eq!(icon_color(tester, &elevated_avatar), Some(scheme.primary));
            assert_eq!(
                icon_color(tester, &elevated_delete_icon),
                Some(scheme.on_surface_variant())
            );

            // Test selected state.
            build_chips(tester, true);

            // Check the flat chip icon colors.
            let on_secondary_container = Some(scheme.on_secondary_container());
            assert_eq!(icon_color(tester, &flat_avatar), on_secondary_container);
            assert_eq!(
                icon_color(tester, &flat_delete_icon),
                on_secondary_container
            );
            // Check the elevated chip icon colors.
            assert_eq!(icon_color(tester, &elevated_avatar), on_secondary_container);
            assert_eq!(
                icon_color(tester, &elevated_delete_icon),
                on_secondary_container
            );
        });
    }

    #[test]
    fn filter_chip_avatar_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = FilterChip::new(
                    Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                    Some(select_listener()),
                )
                .avatar(Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| size_of(tester, only::<FilterChip>(tester));
            let chip_top_left = |tester: &WidgetTester| {
                rect_of(tester, material_of(tester, only::<RawChip>(tester))).top_left()
            };
            let label_top_left =
                |tester: &WidgetTester| rect_of(tester, only::<Container>(tester)).top_left();

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
    fn filter_chip_delete_icon_layout_constraints_can_be_customized() {
        test_widgets(|tester| {
            let border = 1.0;
            let icon_size = 18.0;
            let label_padding = 8.0;
            let padding = 8.0;
            let label_size = Size::new(100.0, 100.0);

            let build_chip = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                let mut chip = FilterChip::new(
                    Container::new()
                        .width(label_size.width())
                        .height(label_size.width())
                        .color(Color::new(0xFFFF0000)),
                    Some(select_listener()),
                )
                .on_deleted(Listener::new(|_| {}));
                if let Some(constraints) = constraints {
                    chip = chip.delete_icon_box_constraints(constraints);
                }
                pump_wrapped(tester, Center::new().child(chip));
            };
            let chip_size = |tester: &WidgetTester| size_of(tester, only::<FilterChip>(tester));
            let chip_top_right = |tester: &WidgetTester| {
                rect_of(tester, material_of(tester, only::<RawChip>(tester))).top_right()
            };
            let label_top_right =
                |tester: &WidgetTester| rect_of(tester, only::<Container>(tester)).top_right();

            // Test default delete icon layout constraints.
            build_chip(tester, None);
            assert_eq!(chip_size(tester).width(), 234.0);
            assert_eq!(chip_size(tester).height(), 118.0);

            // Calculate the distance between delete icon and chip edges.
            let top_right = chip_top_right(tester);
            let delete_icon_center = center_of(tester, tester.find_by_icon(&Icons::clear()));
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

    fn animation_style() -> ChipAnimationStyle {
        ChipAnimationStyle::new()
            .enable_animation(AnimationStyle::NO_ANIMATION)
            .select_animation(AnimationStyle::new(
                None,
                Some(Durations::EXTRALONG4),
                None,
                None,
            ))
    }

    fn assert_animation_style_passed_to_raw_chip(tester: &mut WidgetTester, chip: FilterChip) {
        pump_wrapped(tester, Center::new().child(chip));
        let passed = tester
            .widget(only::<RawChip>(tester), |raw_chip: &RawChip| {
                raw_chip.chip_animation_style.clone()
            })
            .expect("a chip animation style");
        let expected = animation_style();
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
    fn filter_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                filter_chip("FilterChip", true).chip_animation_style(animation_style()),
            );
        });
    }

    #[test]
    fn elevated_filter_chip_chip_animation_style_is_passed_to_raw_chip() {
        test_widgets(|tester| {
            assert_animation_style_passed_to_raw_chip(
                tester,
                elevated("FilterChip", true).chip_animation_style(animation_style()),
            );
        });
    }

    #[test]
    fn filter_chip_has_expected_default_mouse_cursor_on_hover() {
        test_widgets(|tester| {
            pump_wrapped(tester, Center::new().child(filter_chip("Chip", true)));
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
    fn filter_chip_mouse_cursor_behavior() {
        test_widgets(|tester| {
            let custom_cursor: MouseCursorRef = SystemMouseCursors::GRAB.into();
            pump_wrapped(
                tester,
                Center::new().child(filter_chip("Chip", true).mouse_cursor(custom_cursor.clone())),
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
                let chip = filter_chip("Chip", enabled)
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
    fn filter_chip_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(Center::new().child(SizedBox::shrink().child(filter_chip("X", true))))
                    .into_widget(),
            ));
            assert!(size_of(tester, only::<FilterChip>(tester)).is_empty());
        });
    }
}
