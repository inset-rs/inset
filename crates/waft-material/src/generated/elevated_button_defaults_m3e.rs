//! Flutter counterpart: material_ui's `generated/elevated_button_defaults_m3e.g.dart`, a part
//! of `elevated_button.dart`.
//!
//! Dart's private `_ElevatedButtonDefaultsM3E` is a `ButtonStyle` whose getters read the
//! context's theme; [`elevated_button_defaults_m3e`] returns the style those getters answer,
//! read once from the theme as the button's `default_style_of` asks for it. Its static
//! `iconLabelSpace` is [`icon_label_space`].

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Color, Radius, Size};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry, OutlinedBorder,
    RoundedRectangleBorder, StadiumBorder,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyAll,
    WidgetStatePropertyRef,
};

use crate::button_style::{ButtonShapeVariant, ButtonSizeVariant, ButtonStyle};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Dart's `_ElevatedButtonDefaultsM3E(context, toggleable, sizeVariant, shapeVariant)`.
pub(crate) fn elevated_button_defaults_m3e(
    app: &mut App,
    context: BuildContext,
    toggleable: bool,
    size_variant: Option<ButtonSizeVariant>,
    shape_variant: Option<ButtonShapeVariant>,
) -> ButtonStyle {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let (primary, on_primary, on_surface, surface_container_low) = (
        colors.primary,
        colors.on_primary,
        colors.on_surface,
        colors.surface_container_low(),
    );
    let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
    let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
    ButtonStyle::new()
        .animation_duration(K_THEME_CHANGE_DURATION)
        .enable_feedback(true)
        .alignment(AlignmentGeometry::CENTER)
        .size_variant(size_variant)
        .shape_variant(shape_variant)
        .text_style(Rc::new(WidgetStatePropertyAll::new(match size_variant {
            ButtonSizeVariant::XSmall => theme.text_theme.label_large.clone(),
            ButtonSizeVariant::Small => theme.text_theme.label_large.clone(),
            ButtonSizeVariant::Medium => theme.text_theme.title_medium.clone(),
            ButtonSizeVariant::Large => theme.text_theme.headline_small.clone(),
            ButtonSizeVariant::XLarge => theme.text_theme.headline_large.clone(),
        })))
        .background_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
        .foreground_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
        .overlay_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
        .shadow_color(all(colors.shadow()))
        .surface_tint_color(all(Colors::TRANSPARENT))
        .elevation(<dyn WidgetStateProperty<_>>::resolve_with(|states| {
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
        .padding(all(match size_variant {
            ButtonSizeVariant::XSmall => EdgeInsetsGeometry::symmetric(0.0, 12.0),
            ButtonSizeVariant::Small => EdgeInsetsGeometry::symmetric(0.0, 16.0),
            ButtonSizeVariant::Medium => EdgeInsetsGeometry::symmetric(0.0, 24.0),
            ButtonSizeVariant::Large => EdgeInsetsGeometry::symmetric(0.0, 48.0),
            ButtonSizeVariant::XLarge => EdgeInsetsGeometry::symmetric(0.0, 64.0),
        }))
        .minimum_size(all(match size_variant {
            ButtonSizeVariant::XSmall => Size::new(64.0, 32.0),
            ButtonSizeVariant::Small => Size::new(64.0, 40.0),
            ButtonSizeVariant::Medium => Size::new(64.0, 56.0),
            ButtonSizeVariant::Large => Size::new(64.0, 96.0),
            ButtonSizeVariant::XLarge => Size::new(64.0, 136.0),
        }))
        .maximum_size(all(Size::INFINITE))
        .icon_size(all(match size_variant {
            ButtonSizeVariant::XSmall => 20.0,
            ButtonSizeVariant::Small => 20.0,
            ButtonSizeVariant::Medium => 24.0,
            ButtonSizeVariant::Large => 32.0,
            ButtonSizeVariant::XLarge => 40.0,
        }))
        .icon_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
        .shape(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
        .mouse_cursor(adaptive_clickable())
        .visual_density(VisualDensity::STANDARD)
        .tap_target_size(MaterialTapTargetSize::Padded)
        .splash_factory(theme.splash_factory.clone())
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
fn rounded(radius: f64) -> Box<dyn OutlinedBorder> {
    Box::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}

/// Dart's `const StadiumBorder()`.
fn stadium() -> Box<dyn OutlinedBorder> {
    Box::new(StadiumBorder::new(BorderSide::NONE))
}

/// Dart's `const WidgetStatePropertyAll<T>(value)`.
fn all<T: Clone + Debug + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(WidgetStatePropertyAll::new(Some(value)))
}

/// Dart's `WidgetStateMouseCursor.adaptiveClickable` as a `WidgetStateProperty<MouseCursor?>`.
fn adaptive_clickable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
    let cursor = WidgetStateMouseCursor::adaptive_clickable();
    <dyn WidgetStateProperty<_>>::resolve_with(move |states| Some(cursor.resolve(states)))
}
