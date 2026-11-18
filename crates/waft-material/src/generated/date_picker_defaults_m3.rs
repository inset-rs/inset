//! Flutter counterpart: material_ui's `generated/date_picker_defaults_m3.g.dart`, a part of
//! `date_picker_theme.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, CircleBorder, OutlinedBorderRef, RoundedRectangleBorder,
    StadiumBorder, TextStyle,
};
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
    WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::date_picker_theme::{DatePickerThemeData, DatePickerThemeDataGetters};
use crate::text_button::TextButton;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_DatePickerDefaultsM3`.
pub(crate) struct DatePickerDefaultsM3 {
    /// Dart's `super(..)` arguments.
    date_picker_theme_data: DatePickerThemeData,
    context: DefaultsContext,
}

impl DatePickerDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> DatePickerDefaultsM3 {
        DatePickerDefaultsM3 {
            date_picker_theme_data: DatePickerThemeData::new()
                .elevation(6.0)
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(28.0),
                ))
                .day_shape(Rc::new(WidgetStatePropertyAll::new(Some(
                    OutlinedBorderRef::new(CircleBorder::default()),
                ))))
                .year_shape(Rc::new(WidgetStatePropertyAll::new(Some(
                    OutlinedBorderRef::new(StadiumBorder::default()),
                ))))
                .range_picker_elevation(0.0)
                .range_picker_shape(RoundedRectangleBorder::default()),
            context: DefaultsContext::new(context),
        }
    }
}

impl DatePickerThemeDataGetters for DatePickerDefaultsM3 {
    fn date_picker_theme_data(&self) -> &DatePickerThemeData {
        &self.date_picker_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_high())
    }

    fn sub_header_foreground_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface().with_opacity(0.6))
    }

    fn toggle_button_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let color = self.sub_header_foreground_color(app);
        self.context
            .text_theme(app)
            .title_small
            .as_ref()
            .map(|style| style.apply().color(color.expect("a color")).into_style())
    }

    fn cancel_button_style(&self, _app: &mut App) -> Option<ButtonStyle> {
        Some(TextButton::style_from().build())
    }

    fn confirm_button_style(&self, _app: &mut App) -> Option<ButtonStyle> {
        Some(TextButton::style_from().build())
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn header_background_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn header_foreground_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface_variant())
    }

    fn header_headline_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).headline_large.clone()
    }

    fn header_help_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).label_large.clone()
    }

    fn weekday_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface = self.context.colors(app).on_surface();
        self.context
            .text_theme(app)
            .body_large
            .as_ref()
            .map(|style| style.apply().color(on_surface).into_style())
    }

    fn day_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_large.clone()
    }

    fn day_foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, base) = (colors.on_primary(), colors.on_surface());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(on_primary)
                } else if states.contains(&WidgetState::Disabled) {
                    Some(base.with_opacity(0.38))
                } else {
                    Some(base)
                }
            },
        ))
    }

    fn day_background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let primary = self.context.colors(app).primary();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(primary)
                } else {
                    None
                }
            },
        ))
    }

    fn day_overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, on_surface_variant) = (colors.on_primary(), colors.on_surface_variant());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_primary.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_primary.with_opacity(0.1));
                    }
                } else {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_surface_variant.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_surface_variant.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_surface_variant.with_opacity(0.1));
                    }
                }
                None
            },
        ))
    }

    fn today_foreground_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, base) = (colors.on_primary(), colors.primary());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(on_primary)
                } else if states.contains(&WidgetState::Disabled) {
                    Some(base.with_opacity(0.38))
                } else {
                    Some(base)
                }
            },
        ))
    }

    fn today_background_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        self.day_background_color(app)
    }

    fn today_border(&self, app: &mut App) -> Option<BorderSide> {
        Some(BorderSide {
            color: self.context.colors(app).primary(),
            ..BorderSide::default()
        })
    }

    fn year_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_large.clone()
    }

    fn year_foreground_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, base) = (colors.on_primary(), colors.on_surface_variant());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(on_primary)
                } else if states.contains(&WidgetState::Disabled) {
                    Some(base.with_opacity(0.38))
                } else {
                    Some(base)
                }
            },
        ))
    }

    fn year_background_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let primary = self.context.colors(app).primary();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(primary)
                } else {
                    None
                }
            },
        ))
    }

    fn year_overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, on_surface_variant) = (colors.on_primary(), colors.on_surface_variant());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_primary.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_primary.with_opacity(0.1));
                    }
                } else {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_surface_variant.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_surface_variant.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_surface_variant.with_opacity(0.1));
                    }
                }
                None
            },
        ))
    }

    fn range_picker_shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn range_picker_surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn range_selection_background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).secondary_container())
    }

    fn range_selection_overlay_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let on_primary_container = self.context.colors(app).on_primary_container();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Pressed) {
                    return Some(on_primary_container.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(on_primary_container.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(on_primary_container.with_opacity(0.1));
                }
                None
            },
        ))
    }

    fn range_picker_header_background_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn range_picker_header_foreground_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface_variant())
    }

    fn range_picker_header_headline_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_large.clone()
    }

    fn range_picker_header_help_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_small.clone()
    }
}
