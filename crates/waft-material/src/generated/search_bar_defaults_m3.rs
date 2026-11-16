//! Flutter counterpart: material_ui's `generated/search_bar_defaults_m3.g.dart`, a part of
//! `search_anchor.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Color, TextCapitalization};
use waft_foundation::App;
use waft_painting::{
    BorderSide, EdgeInsets, EdgeInsetsGeometry, OutlinedBorderRef, StadiumBorder, TextStyle,
};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
};

use crate::colors::Colors;
use crate::search_bar_theme::{SearchBarThemeData, SearchBarThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_SearchBarDefaultsM3`.
pub(crate) struct SearchBarDefaultsM3 {
    /// Dart's `super(..)` arguments: none.
    search_bar_theme_data: SearchBarThemeData,
    context: DefaultsContext,
}

impl SearchBarDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> SearchBarDefaultsM3 {
        SearchBarDefaultsM3 {
            search_bar_theme_data: SearchBarThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl SearchBarThemeDataGetters for SearchBarDefaultsM3 {
    fn search_bar_theme_data(&self) -> &SearchBarThemeData {
        &self.search_bar_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(self.context.colors(app).surface_container_high()))
    }

    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(all(6.0))
    }

    fn shadow_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(self.context.colors(app).shadow()))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let on_surface = self.context.colors(app).on_surface;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                return Some(on_surface.with_opacity(0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_surface.with_opacity(0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(Colors::TRANSPARENT);
            }
            Some(Colors::TRANSPARENT)
        }))
    }

    // No default side
    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        Some(all(OutlinedBorderRef::new(StadiumBorder::new(
            BorderSide::NONE,
        ))))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(all(EdgeInsets::symmetric(0.0, 8.0).into()))
    }

    fn text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let on_surface = self.context.colors(app).on_surface;
        let style = self
            .context
            .text_theme(app)
            .body_large
            .as_ref()
            .map(|body_large| body_large.copy_with().color(on_surface));
        Some(Rc::new(WidgetStatePropertyAll::new(style)))
    }

    fn hint_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        let style = self
            .context
            .text_theme(app)
            .body_large
            .as_ref()
            .map(|body_large| body_large.copy_with().color(on_surface_variant));
        Some(Rc::new(WidgetStatePropertyAll::new(style)))
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints {
            min_width: 360.0,
            max_width: 800.0,
            min_height: 56.0,
            ..BoxConstraints::new()
        })
    }

    fn text_capitalization(&self, _app: &mut App) -> Option<TextCapitalization> {
        Some(TextCapitalization::None)
    }
}

/// Dart's `const WidgetStatePropertyAll<T>(value)`.
fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(WidgetStatePropertyAll::new(Some(value)))
}
