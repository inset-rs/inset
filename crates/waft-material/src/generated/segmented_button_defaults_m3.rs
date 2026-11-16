//! Flutter counterpart: material_ui's `generated/segmented_button_defaults_m3.g.dart`, a part of
//! `segmented_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{BorderSide, OutlinedBorderRef, StadiumBorder};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyAll, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::icons::Icons;
use crate::segmented_button_theme::{SegmentedButtonThemeData, SegmentedButtonThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_SegmentedButtonDefaultsM3`.
pub(crate) struct SegmentedButtonDefaultsM3 {
    /// Dart's `super(..)` arguments.
    segmented_button_theme_data: SegmentedButtonThemeData,
    context: DefaultsContext,
}

impl SegmentedButtonDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> SegmentedButtonDefaultsM3 {
        SegmentedButtonDefaultsM3 {
            segmented_button_theme_data: SegmentedButtonThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }

    pub(crate) fn resolve_state_color(
        unselected_color: Option<Color>,
        selected_color: Option<Color>,
        overlay_color: Option<Color>,
    ) -> WidgetStatePropertyRef<Option<Color>> {
        let selected = overlay_color.or(selected_color);
        let unselected = overlay_color.or(unselected_color);
        <dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
            (
                WidgetState::Selected & WidgetState::Pressed,
                selected.map(|color| color.with_opacity(0.1)),
            ),
            (
                WidgetState::Selected & WidgetState::Hovered,
                selected.map(|color| color.with_opacity(0.08)),
            ),
            (
                WidgetState::Selected & WidgetState::Focused,
                selected.map(|color| color.with_opacity(0.1)),
            ),
            (
                WidgetState::Pressed.into(),
                unselected.map(|color| color.with_opacity(0.1)),
            ),
            (
                WidgetState::Hovered.into(),
                unselected.map(|color| color.with_opacity(0.08)),
            ),
            (
                WidgetState::Focused.into(),
                unselected.map(|color| color.with_opacity(0.1)),
            ),
            (WidgetState::any(), Some(Colors::TRANSPARENT)),
        ])
    }
}

impl SegmentedButtonThemeDataGetters for SegmentedButtonDefaultsM3 {
    fn segmented_button_theme_data(&self) -> &SegmentedButtonThemeData {
        &self.segmented_button_theme_data
    }

    fn style(&self, app: &mut App) -> Option<ButtonStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let secondary_container = colors.secondary_container();
        let on_secondary_container = colors.on_secondary_container();
        let on_surface = colors.on_surface;
        let outline = colors.outline();
        Some(
            ButtonStyle::new()
                .text_style(Rc::new(WidgetStatePropertyAll::new(
                    theme.text_theme.label_large.clone(),
                )))
                .background_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Disabled) {
                        return None;
                    }
                    if states.contains(&WidgetState::Selected) {
                        return Some(secondary_container);
                    }
                    None
                }))
                .foreground_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Disabled) {
                        return Some(on_surface.with_opacity(0.38));
                    }
                    if states.contains(&WidgetState::Selected) {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(on_secondary_container);
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(on_secondary_container);
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(on_secondary_container);
                        }
                        Some(on_secondary_container)
                    } else {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(on_surface);
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(on_surface);
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(on_surface);
                        }
                        Some(on_surface)
                    }
                }))
                .overlay_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Selected) {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(on_secondary_container.with_opacity(0.1));
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(on_secondary_container.with_opacity(0.08));
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(on_secondary_container.with_opacity(0.1));
                        }
                    } else {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(on_surface.with_opacity(0.1));
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(on_surface.with_opacity(0.08));
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(on_surface.with_opacity(0.1));
                        }
                    }
                    None
                }))
                .surface_tint_color(all(Colors::TRANSPARENT))
                .elevation(all(0.0))
                .icon_size(all(18.0))
                .side(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Disabled) {
                        return Some(BorderSide {
                            color: on_surface.with_opacity(0.12),
                            ..BorderSide::default()
                        });
                    }
                    Some(BorderSide {
                        color: outline,
                        ..BorderSide::default()
                    })
                }))
                .shape(all(OutlinedBorderRef::new(StadiumBorder::new(
                    BorderSide::NONE,
                ))))
                .minimum_size(all(Size::from_height(40.0))),
        )
    }

    fn selected_icon(&self, _app: &mut App) -> Option<WidgetRef> {
        Some(Icon::new(Some(Icons::check())).into_widget())
    }
}

/// Dart's `const WidgetStatePropertyAll<T>(value)`.
fn all<T: Clone + std::fmt::Debug + PartialEq + 'static>(
    value: T,
) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(WidgetStatePropertyAll::new(Some(value)))
}
