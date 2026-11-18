//! Flutter counterpart: material_ui's `generated/time_picker_defaults_m3.g.dart`, a part of
//! `time_picker.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{
    AnyColor, AnyTextStyle, BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry,
    OutlinedBorderRef, RoundedRectangleBorder, ShapeBorderRef, TextStyle,
};
use waft_widgets::{
    BuildContext, Directionality, WidgetState, WidgetStatePropertyAll, WidgetStatePropertyRef,
    WidgetStateTextStyle, WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::input_border::OutlineInputBorder;
use crate::input_decorator::InputDecorationThemeData;
use crate::text_button::TextButton;
use crate::theme_data_getters::DefaultsContext;
use crate::time_picker::{TimePickerDefaults, TimePickerEntryMode, state_color};

/// Dart's `_TimePickerDefaultsM3`.
pub(crate) struct TimePickerDefaultsM3 {
    context: DefaultsContext,
    entry_mode: TimePickerEntryMode,
}

impl TimePickerDefaultsM3 {
    pub(crate) fn new(context: BuildContext, entry_mode: TimePickerEntryMode) -> Self {
        TimePickerDefaultsM3 {
            context: DefaultsContext::new(context),
            entry_mode,
        }
    }

    fn hour_minute_text_color_property(&self, app: &mut App) -> Rc<dyn Fn(&WidgetStates) -> Color> {
        let colors = self.context.colors(app);
        let (on_primary_container, on_surface) =
            (colors.on_primary_container(), colors.on_surface());
        Rc::new(move |states| {
            if states.contains(&WidgetState::Selected) {
                on_primary_container
            } else {
                on_surface
            }
        })
    }
}

impl TimePickerDefaults for TimePickerDefaultsM3 {
    fn background_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).surface_container_high().into()
    }

    fn cancel_button_style(&self, _app: &mut App) -> ButtonStyle {
        TextButton::style_from().build()
    }

    fn confirm_button_style(&self, _app: &mut App) -> ButtonStyle {
        TextButton::style_from().build()
    }

    fn day_period_border_side(&self, app: &mut App) -> BorderSide {
        BorderSide {
            color: self.context.colors(app).outline(),
            ..BorderSide::default()
        }
    }

    fn day_period_color(&self, app: &mut App) -> AnyColor {
        let tertiary_container = self.context.colors(app).tertiary_container();
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                tertiary_container
            } else {
                Colors::TRANSPARENT
            }
        })
    }

    fn day_period_shape(&self, app: &mut App) -> OutlinedBorderRef {
        OutlinedBorderRef::new(RoundedRectangleBorder::new(
            self.day_period_border_side(app),
            BorderRadiusGeometry::circular(8.0),
        ))
    }

    fn day_period_portrait_size(&self, _app: &mut App) -> Size {
        Size::new(52.0, 80.0)
    }

    fn day_period_landscape_size(&self, _app: &mut App) -> Size {
        Size::new(216.0, 38.0)
    }

    fn day_period_input_size(&self, app: &mut App) -> Size {
        let portrait = self.day_period_portrait_size(app);
        Size::new(portrait.width(), portrait.height() - 8.0)
    }

    fn day_period_text_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (on_tertiary_container, on_surface_variant) =
            (colors.on_tertiary_container(), colors.on_surface_variant());
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                on_tertiary_container
            } else {
                on_surface_variant
            }
        })
    }

    fn day_period_text_style(&self, app: &mut App) -> AnyTextStyle {
        let color = self.day_period_text_color(app);
        self.context
            .text_theme(app)
            .title_medium
            .as_ref()
            .expect("the text theme has titleMedium")
            .copy_with()
            .color(color)
            .into()
    }

    fn dial_background_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).surface_container_highest().into()
    }

    fn dial_hand_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).primary().into()
    }

    fn dial_size(&self, _app: &mut App) -> Size {
        Size::square(256.0)
    }

    fn hand_width(&self, _app: &mut App) -> f64 {
        2.0
    }

    fn dot_radius(&self, _app: &mut App) -> f64 {
        48.0 / 2.0
    }

    fn center_radius(&self, _app: &mut App) -> f64 {
        8.0 / 2.0
    }

    fn dial_text_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (on_primary, on_surface) = (colors.on_primary(), colors.on_surface());
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                on_primary
            } else {
                on_surface
            }
        })
    }

    fn dial_text_style(&self, app: &mut App) -> AnyTextStyle {
        self.context
            .text_theme(app)
            .body_large
            .clone()
            .expect("the text theme has bodyLarge")
            .into()
    }

    fn elevation(&self, _app: &mut App) -> f64 {
        6.0
    }

    fn entry_mode_icon_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).on_surface().into()
    }

    fn help_text_style(&self, app: &mut App) -> AnyTextStyle {
        let text_style = self
            .context
            .text_theme(app)
            .label_medium
            .clone()
            .expect("the text theme has labelMedium");
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        WidgetStateTextStyle::resolve_with(move |_states| {
            text_style.copy_with().color(on_surface_variant)
        })
        .into()
    }

    fn padding(&self, _app: &mut App) -> EdgeInsetsGeometry {
        EdgeInsets::all(24.0).into()
    }

    fn hour_minute_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (primary_container, on_primary_container, surface_container_highest, on_surface) = (
            colors.primary_container(),
            colors.on_primary_container(),
            colors.surface_container_highest(),
            colors.on_surface(),
        );
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                let mut overlay_color = primary_container;
                if states.contains(&WidgetState::Pressed) {
                    overlay_color = on_primary_container;
                } else if states.contains(&WidgetState::Hovered) {
                    overlay_color = on_primary_container.with_opacity(0.08);
                } else if states.contains(&WidgetState::Focused) {
                    overlay_color = on_primary_container.with_opacity(0.1);
                }
                Color::alpha_blend(overlay_color, primary_container)
            } else {
                let mut overlay_color = surface_container_highest;
                if states.contains(&WidgetState::Pressed) {
                    overlay_color = on_surface;
                } else if states.contains(&WidgetState::Hovered) {
                    overlay_color = on_surface.with_opacity(0.08);
                } else if states.contains(&WidgetState::Focused) {
                    overlay_color = on_surface.with_opacity(0.1);
                }
                Color::alpha_blend(overlay_color, surface_container_highest)
            }
        })
    }

    fn hour_minute_shape(&self, _app: &mut App) -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::circular(8.0),
        ))
    }

    fn hour_minute_size(&self, _app: &mut App) -> Size {
        Size::new(96.0, 80.0)
    }

    fn hour_minute_size_24_hour(&self, app: &mut App) -> Size {
        Size::new(114.0, self.hour_minute_size(app).height())
    }

    fn hour_minute_input_size(&self, app: &mut App) -> Size {
        let size = self.hour_minute_size(app);
        Size::new(size.width(), size.height() - 8.0)
    }

    fn hour_minute_input_size_24_hour(&self, app: &mut App) -> Size {
        let size = self.hour_minute_size_24_hour(app);
        Size::new(size.width(), size.height() - 8.0)
    }

    fn hour_minute_text_color(&self, app: &mut App) -> AnyColor {
        let resolve = self.hour_minute_text_color_property(app);
        state_color(move |states| resolve(states))
    }

    fn hour_minute_text_style(&self, app: &mut App) -> AnyTextStyle {
        let text_theme = self.context.text_theme(app);
        let style = match self.entry_mode {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => {
                text_theme.display_large.clone()
            }
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => {
                text_theme.display_medium.clone()
            }
        }
        .expect("the text theme has the display styles");
        let resolve = self.hour_minute_text_color_property(app);
        WidgetStateTextStyle::resolve_with(move |states| style.copy_with().color(resolve(states)))
            .into()
    }

    fn input_decoration_theme(&self, app: &mut App) -> InputDecorationThemeData {
        let selector_radius = BorderRadiusGeometry::circular(8.0)
            .resolve(Some(Directionality::of(app, self.context.context())));
        let fill_color = self.hour_minute_color(app);
        let hint_style = self
            .hour_minute_text_style(app)
            .value()
            .copy_with()
            .color(self.context.colors(app).on_surface().with_opacity(0.36));
        let colors = self.context.colors(app);
        let (primary_container, error, primary) =
            (colors.primary_container(), colors.error(), colors.primary());
        InputDecorationThemeData::new()
            .content_padding(EdgeInsetsGeometry::from(EdgeInsets::ZERO))
            .filled(true)
            .fill_color(fill_color)
            .focus_color(primary_container)
            .enabled_border(
                OutlineInputBorder::new()
                    .border_radius(selector_radius)
                    .border_side(BorderSide {
                        color: Colors::TRANSPARENT,
                        ..BorderSide::default()
                    }),
            )
            .error_border(
                OutlineInputBorder::new()
                    .border_radius(selector_radius)
                    .border_side(BorderSide {
                        color: error,
                        width: 2.0,
                        ..BorderSide::default()
                    }),
            )
            .focused_border(
                OutlineInputBorder::new()
                    .border_radius(selector_radius)
                    .border_side(BorderSide {
                        color: primary,
                        width: 2.0,
                        ..BorderSide::default()
                    }),
            )
            .focused_error_border(
                OutlineInputBorder::new()
                    .border_radius(selector_radius)
                    .border_side(BorderSide {
                        color: error,
                        width: 2.0,
                        ..BorderSide::default()
                    }),
            )
            .hint_style(hint_style)
            .error_style(TextStyle::new().font_size(0.0))
    }

    fn shape(&self, _app: &mut App) -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::circular(28.0),
        ))
    }

    fn time_selector_separator_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
            self.context.colors(app).on_surface(),
        ))))
    }

    fn time_selector_separator_text_style(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(
            self.context.text_theme(app).display_large.clone(),
        )))
    }
}
