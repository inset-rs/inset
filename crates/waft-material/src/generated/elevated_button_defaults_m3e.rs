//! Flutter counterpart: material_ui's `generated/elevated_button_defaults_m3e.g.dart`, a part of
//! `elevated_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Color, Radius, Size};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef,
    RoundedRectangleBorder, StadiumBorder,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyAll,
    WidgetStatePropertyRef,
};

use crate::button_style::{ButtonShapeVariant, ButtonSizeVariant, ButtonStyle, ButtonStyleGetters};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::theme_data_getters::DefaultsContext;
use waft_painting::TextStyle;

/// Dart's `_ElevatedButtonDefaultsM3E`.
pub(crate) struct ElevatedButtonDefaultsM3E {
    /// Dart's `super(..)` arguments.
    button_style: ButtonStyle,
    context: DefaultsContext,
    toggleable: bool,
    size_variant: Option<ButtonSizeVariant>,
    shape_variant: Option<ButtonShapeVariant>,
}

impl ElevatedButtonDefaultsM3E {
    pub(crate) fn new(
        context: BuildContext,
        toggleable: bool,
        size_variant: Option<ButtonSizeVariant>,
        shape_variant: Option<ButtonShapeVariant>,
    ) -> ElevatedButtonDefaultsM3E {
        ElevatedButtonDefaultsM3E {
            button_style: ButtonStyle::new()
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER),
            context: DefaultsContext::new(context),
            toggleable,
            size_variant,
            shape_variant,
        }
    }
}

impl ButtonStyleGetters for ElevatedButtonDefaultsM3E {
    fn button_style(&self) -> &ButtonStyle {
        &self.button_style
    }

    fn size_variant(&self, _app: &mut App) -> Option<ButtonSizeVariant> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(size_variant)
    }

    fn shape_variant(&self, _app: &mut App) -> Option<ButtonShapeVariant> {
        let shape_variant = self.shape_variant;
        let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
        Some(shape_variant)
    }

    fn text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let size_variant = self.size_variant;
        let theme = self.context.theme(app);
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(Rc::new(WidgetStatePropertyAll::new(match size_variant {
            ButtonSizeVariant::XSmall => theme.text_theme.label_large.clone(),
            ButtonSizeVariant::Small => theme.text_theme.label_large.clone(),
            ButtonSizeVariant::Medium => theme.text_theme.title_medium.clone(),
            ButtonSizeVariant::Large => theme.text_theme.headline_small.clone(),
            ButtonSizeVariant::XLarge => theme.text_theme.headline_large.clone(),
        })))
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary();
        let on_surface = colors.on_surface();
        let surface_container_low = colors.surface_container_low();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(with_alpha(on_surface, 0.1));
            }
            if toggleable && states.contains(&WidgetState::Selected) {
                return Some(primary);
            }
            if toggleable {
                return Some(surface_container_low);
            }
            Some(surface_container_low)
        }))
    }

    fn foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary();
        let on_primary = colors.on_primary();
        let on_surface = colors.on_surface();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(with_alpha(on_surface, 0.38));
            }
            if toggleable && states.contains(&WidgetState::Selected) {
                return Some(on_primary);
            }
            if toggleable {
                return Some(primary);
            }
            Some(primary)
        }))
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary();
        let on_primary = colors.on_primary();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if toggleable && states.contains(&WidgetState::Selected) {
                if states.contains(&WidgetState::Pressed) {
                    return Some(with_alpha(on_primary, 0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(with_alpha(on_primary, 0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(with_alpha(on_primary, 0.1));
                }
                return None;
            }
            if toggleable {
                if states.contains(&WidgetState::Pressed) {
                    return Some(with_alpha(primary, 0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(with_alpha(primary, 0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(with_alpha(primary, 0.1));
                }
                return None;
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(with_alpha(primary, 0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(with_alpha(primary, 0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(with_alpha(primary, 0.1));
            }
            None
        }))
    }

    fn shadow_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.shadow()))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(<dyn WidgetStateProperty<_>>::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(0.0);
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(1.0);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(1.0);
            }
            Some(1.0)
        }))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(all(match size_variant {
            ButtonSizeVariant::XSmall => EdgeInsetsGeometry::symmetric(0.0, 12.0),
            ButtonSizeVariant::Small => EdgeInsetsGeometry::symmetric(0.0, 16.0),
            ButtonSizeVariant::Medium => EdgeInsetsGeometry::symmetric(0.0, 24.0),
            ButtonSizeVariant::Large => EdgeInsetsGeometry::symmetric(0.0, 48.0),
            ButtonSizeVariant::XLarge => EdgeInsetsGeometry::symmetric(0.0, 64.0),
        }))
    }

    fn minimum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(all(match size_variant {
            ButtonSizeVariant::XSmall => Size::new(64.0, 32.0),
            ButtonSizeVariant::Small => Size::new(64.0, 40.0),
            ButtonSizeVariant::Medium => Size::new(64.0, 56.0),
            ButtonSizeVariant::Large => Size::new(64.0, 96.0),
            ButtonSizeVariant::XLarge => Size::new(64.0, 136.0),
        }))
    }

    fn maximum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(all(Size::INFINITE))
    }

    fn icon_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(all(match size_variant {
            ButtonSizeVariant::XSmall => 20.0,
            ButtonSizeVariant::Small => 20.0,
            ButtonSizeVariant::Medium => 24.0,
            ButtonSizeVariant::Large => 32.0,
            ButtonSizeVariant::XLarge => 40.0,
        }))
    }

    fn icon_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary();
        let on_primary = colors.on_primary();
        let on_surface = colors.on_surface();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(with_alpha(on_surface, 0.38));
            }
            if toggleable && states.contains(&WidgetState::Selected) {
                return Some(on_primary);
            }
            if toggleable {
                return Some(primary);
            }
            Some(primary)
        }))
    }

    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        let toggleable = self.toggleable;
        let size_variant = self.size_variant;
        let shape_variant = self.shape_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                return Some(match size_variant {
                    ButtonSizeVariant::XSmall => rounded(8.0),
                    ButtonSizeVariant::Small => rounded(8.0),
                    ButtonSizeVariant::Medium => rounded(12.0),
                    ButtonSizeVariant::Large => rounded(16.0),
                    ButtonSizeVariant::XLarge => rounded(16.0),
                });
            }
            if toggleable && states.contains(&WidgetState::Selected) {
                return Some(match shape_variant {
                    ButtonShapeVariant::Round => match size_variant {
                        ButtonSizeVariant::XSmall => rounded(12.0),
                        ButtonSizeVariant::Small => rounded(12.0),
                        ButtonSizeVariant::Medium => rounded(16.0),
                        ButtonSizeVariant::Large => rounded(28.0),
                        ButtonSizeVariant::XLarge => rounded(28.0),
                    },
                    ButtonShapeVariant::Square => match size_variant {
                        ButtonSizeVariant::XSmall => stadium(),
                        ButtonSizeVariant::Small => stadium(),
                        ButtonSizeVariant::Medium => stadium(),
                        ButtonSizeVariant::Large => stadium(),
                        ButtonSizeVariant::XLarge => stadium(),
                    },
                });
            }
            Some(match shape_variant {
                ButtonShapeVariant::Round => match size_variant {
                    ButtonSizeVariant::XSmall => stadium(),
                    ButtonSizeVariant::Small => stadium(),
                    ButtonSizeVariant::Medium => stadium(),
                    ButtonSizeVariant::Large => stadium(),
                    ButtonSizeVariant::XLarge => stadium(),
                },
                ButtonShapeVariant::Square => match size_variant {
                    ButtonSizeVariant::XSmall => rounded(12.0),
                    ButtonSizeVariant::Small => rounded(12.0),
                    ButtonSizeVariant::Medium => rounded(16.0),
                    ButtonSizeVariant::Large => rounded(28.0),
                    ButtonSizeVariant::XLarge => rounded(28.0),
                },
            })
        }))
    }

    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        Some(adaptive_clickable())
    }

    fn visual_density(&self, _app: &mut App) -> Option<VisualDensity> {
        Some(VisualDensity::STANDARD)
    }

    fn tap_target_size(&self, _app: &mut App) -> Option<MaterialTapTargetSize> {
        Some(MaterialTapTargetSize::Padded)
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }
}

/// The space between the icon and the label for the given `size_variant`.
pub(crate) fn icon_label_space(size_variant: ButtonSizeVariant) -> f64 {
    match size_variant {
        ButtonSizeVariant::XSmall => 4.0,
        ButtonSizeVariant::Small => 8.0,
        ButtonSizeVariant::Medium => 8.0,
        ButtonSizeVariant::Large => 12.0,
        ButtonSizeVariant::XLarge => 16.0,
    }
}

/// Dart's `color.withValues(alpha: alpha)`.
fn with_alpha(color: Color, alpha: f64) -> Color {
    color.with_values(Some(alpha), None, None, None, None)
}

/// Dart's `const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(r)))`.
fn rounded(radius: f64) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}

/// Dart's `const StadiumBorder()`.
fn stadium() -> OutlinedBorderRef {
    OutlinedBorderRef::new(StadiumBorder::new(BorderSide::NONE))
}

/// Dart's `const WidgetStatePropertyAll<T>(value)`.
fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(WidgetStatePropertyAll::new(Some(value)))
}

/// Dart's `WidgetStateMouseCursor.adaptiveClickable` as a `WidgetStateProperty<MouseCursor?>`.
fn adaptive_clickable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
    let cursor = WidgetStateMouseCursor::adaptive_clickable();
    <dyn WidgetStateProperty<_>>::resolve_with(move |states| Some(cursor.resolve(states)))
}
