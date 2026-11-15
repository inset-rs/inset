//! Flutter counterpart: material_ui's `generated/outlined_icon_button_defaults_m3e.g.dart`, a Dart `part`
//! of `icon_button.dart`: `IconButton`'s Material 3 Expressive build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

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
};

use crate::button_style::{
    ButtonShapeVariant, ButtonSizeVariant, ButtonStyle, IconButtonWidthVariant,
};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Dart's `_OutlinedIconButtonDefaultsM3E(context, toggleable, sizeVariant, iconButtonWidth,
/// shapeVariant)`: the [`ButtonStyle`] whose fields are the values of that subclass's
/// getters.
pub(crate) fn outlined_icon_button_defaults_m3e(
    app: &mut App,
    context: BuildContext,
    toggleable: bool,
    size_variant: Option<ButtonSizeVariant>,
    icon_button_width: Option<IconButtonWidthVariant>,
    shape_variant: Option<ButtonShapeVariant>,
) -> ButtonStyle {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
    let icon_button_width = icon_button_width.unwrap_or(IconButtonWidthVariant::Standard);
    let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
    let adaptive_clickable = WidgetStateMouseCursor::adaptive_clickable();

    ButtonStyle::new()
        .animation_duration(K_THEME_CHANGE_DURATION)
        .enable_feedback(true)
        .alignment(AlignmentGeometry::CENTER)
        .size_variant(size_variant)
        .icon_button_width(icon_button_width)
        .shape_variant(shape_variant)
        .background_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    if toggleable && states.contains(&WidgetState::Selected) {
                        return Some(colors.on_surface.with_values(
                            Some(0.1),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                    return Some(Colors::TRANSPARENT);
                }
                if toggleable && states.contains(&WidgetState::Selected) {
                    return Some(colors.inverse_surface());
                }
                Some(Colors::TRANSPARENT)
            })
        })
        .foreground_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(
                        colors
                            .on_surface
                            .with_values(Some(0.38), None, None, None, None),
                    );
                }
                if toggleable && states.contains(&WidgetState::Selected) {
                    return Some(colors.on_inverse_surface());
                }
                Some(colors.on_surface_variant())
            })
        })
        .overlay_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if toggleable && states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.on_inverse_surface().with_values(
                            Some(0.1),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.on_inverse_surface().with_values(
                            Some(0.08),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.on_inverse_surface().with_values(
                            Some(0.1),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface_variant().with_values(
                        Some(0.1),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface_variant().with_values(
                        Some(0.08),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface_variant().with_values(
                        Some(0.1),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
                Some(Colors::TRANSPARENT)
            })
        })
        .elevation(Rc::new(WidgetStatePropertyAll::new(Some(0.0))))
        .shadow_color(Rc::new(WidgetStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
        .surface_tint_color(Rc::new(WidgetStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
        .padding(Rc::new(WidgetStatePropertyAll::new(Some(
            match size_variant {
                ButtonSizeVariant::XSmall => match icon_button_width {
                    IconButtonWidthVariant::Narrow => {
                        EdgeInsetsGeometry::from_steb(4.0, 6.0, 4.0, 6.0)
                    }
                    IconButtonWidthVariant::Standard => {
                        EdgeInsetsGeometry::from_steb(6.0, 6.0, 6.0, 6.0)
                    }
                    IconButtonWidthVariant::Wide => {
                        EdgeInsetsGeometry::from_steb(10.0, 6.0, 10.0, 6.0)
                    }
                },
                ButtonSizeVariant::Small => match icon_button_width {
                    IconButtonWidthVariant::Narrow => {
                        EdgeInsetsGeometry::from_steb(4.0, 8.0, 4.0, 8.0)
                    }
                    IconButtonWidthVariant::Standard => {
                        EdgeInsetsGeometry::from_steb(8.0, 8.0, 8.0, 8.0)
                    }
                    IconButtonWidthVariant::Wide => {
                        EdgeInsetsGeometry::from_steb(14.0, 8.0, 14.0, 8.0)
                    }
                },
                ButtonSizeVariant::Medium => match icon_button_width {
                    IconButtonWidthVariant::Narrow => {
                        EdgeInsetsGeometry::from_steb(12.0, 16.0, 12.0, 16.0)
                    }
                    IconButtonWidthVariant::Standard => {
                        EdgeInsetsGeometry::from_steb(16.0, 16.0, 16.0, 16.0)
                    }
                    IconButtonWidthVariant::Wide => {
                        EdgeInsetsGeometry::from_steb(24.0, 16.0, 24.0, 16.0)
                    }
                },
                ButtonSizeVariant::Large => match icon_button_width {
                    IconButtonWidthVariant::Narrow => {
                        EdgeInsetsGeometry::from_steb(16.0, 32.0, 16.0, 32.0)
                    }
                    IconButtonWidthVariant::Standard => {
                        EdgeInsetsGeometry::from_steb(32.0, 32.0, 32.0, 32.0)
                    }
                    IconButtonWidthVariant::Wide => {
                        EdgeInsetsGeometry::from_steb(48.0, 32.0, 48.0, 32.0)
                    }
                },
                ButtonSizeVariant::XLarge => match icon_button_width {
                    IconButtonWidthVariant::Narrow => {
                        EdgeInsetsGeometry::from_steb(32.0, 48.0, 32.0, 48.0)
                    }
                    IconButtonWidthVariant::Standard => {
                        EdgeInsetsGeometry::from_steb(48.0, 48.0, 48.0, 48.0)
                    }
                    IconButtonWidthVariant::Wide => {
                        EdgeInsetsGeometry::from_steb(72.0, 48.0, 72.0, 48.0)
                    }
                },
            },
        ))))
        .minimum_size(Rc::new(WidgetStatePropertyAll::new(Some(
            match size_variant {
                ButtonSizeVariant::XSmall => match icon_button_width {
                    IconButtonWidthVariant::Narrow => Size::new(28.0, 32.0),
                    IconButtonWidthVariant::Standard => Size::new(32.0, 32.0),
                    IconButtonWidthVariant::Wide => Size::new(40.0, 32.0),
                },
                ButtonSizeVariant::Small => match icon_button_width {
                    IconButtonWidthVariant::Narrow => Size::new(32.0, 40.0),
                    IconButtonWidthVariant::Standard => Size::new(40.0, 40.0),
                    IconButtonWidthVariant::Wide => Size::new(52.0, 40.0),
                },
                ButtonSizeVariant::Medium => match icon_button_width {
                    IconButtonWidthVariant::Narrow => Size::new(48.0, 56.0),
                    IconButtonWidthVariant::Standard => Size::new(56.0, 56.0),
                    IconButtonWidthVariant::Wide => Size::new(72.0, 56.0),
                },
                ButtonSizeVariant::Large => match icon_button_width {
                    IconButtonWidthVariant::Narrow => Size::new(64.0, 96.0),
                    IconButtonWidthVariant::Standard => Size::new(96.0, 96.0),
                    IconButtonWidthVariant::Wide => Size::new(128.0, 96.0),
                },
                ButtonSizeVariant::XLarge => match icon_button_width {
                    IconButtonWidthVariant::Narrow => Size::new(104.0, 136.0),
                    IconButtonWidthVariant::Standard => Size::new(136.0, 136.0),
                    IconButtonWidthVariant::Wide => Size::new(184.0, 136.0),
                },
            },
        ))))
        .maximum_size(Rc::new(WidgetStatePropertyAll::new(Some(Size::INFINITE))))
        .icon_size(Rc::new(WidgetStatePropertyAll::new(Some(
            match size_variant {
                ButtonSizeVariant::XSmall => 20.0,
                ButtonSizeVariant::Small => 24.0,
                ButtonSizeVariant::Medium => 24.0,
                ButtonSizeVariant::Large => 32.0,
                ButtonSizeVariant::XLarge => 40.0,
            },
        ))))
        .shape(
            <dyn WidgetStateProperty<Option<Box<dyn OutlinedBorder>>>>::resolve_with(
                move |states| {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(match size_variant {
                            ButtonSizeVariant::XSmall => rounded_rectangle(8.0),
                            ButtonSizeVariant::Small => rounded_rectangle(8.0),
                            ButtonSizeVariant::Medium => rounded_rectangle(12.0),
                            ButtonSizeVariant::Large => rounded_rectangle(16.0),
                            ButtonSizeVariant::XLarge => rounded_rectangle(16.0),
                        });
                    }
                    if toggleable && states.contains(&WidgetState::Selected) {
                        return Some(match shape_variant {
                            ButtonShapeVariant::Round => match size_variant {
                                ButtonSizeVariant::XSmall => rounded_rectangle(12.0),
                                ButtonSizeVariant::Small => rounded_rectangle(12.0),
                                ButtonSizeVariant::Medium => rounded_rectangle(16.0),
                                ButtonSizeVariant::Large => rounded_rectangle(28.0),
                                ButtonSizeVariant::XLarge => rounded_rectangle(28.0),
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
                            ButtonSizeVariant::XSmall => rounded_rectangle(12.0),
                            ButtonSizeVariant::Small => rounded_rectangle(12.0),
                            ButtonSizeVariant::Medium => rounded_rectangle(16.0),
                            ButtonSizeVariant::Large => rounded_rectangle(28.0),
                            ButtonSizeVariant::XLarge => rounded_rectangle(28.0),
                        },
                    })
                },
            ),
        )
        .side(<dyn WidgetStateProperty<Option<BorderSide>>>::resolve_with(
            move |states| {
                if toggleable && states.contains(&WidgetState::Selected) {
                    return None;
                }
                if states.contains(&WidgetState::Disabled) {
                    return Some(BorderSide {
                        color: colors.outline_variant(),
                        width: match size_variant {
                            ButtonSizeVariant::XSmall => 1.0,
                            ButtonSizeVariant::Small => 1.0,
                            ButtonSizeVariant::Medium => 1.0,
                            ButtonSizeVariant::Large => 2.0,
                            ButtonSizeVariant::XLarge => 3.0,
                        },
                        ..BorderSide::default()
                    });
                }
                Some(BorderSide {
                    color: colors.outline_variant(),
                    width: match size_variant {
                        ButtonSizeVariant::XSmall => 1.0,
                        ButtonSizeVariant::Small => 1.0,
                        ButtonSizeVariant::Medium => 1.0,
                        ButtonSizeVariant::Large => 2.0,
                        ButtonSizeVariant::XLarge => 3.0,
                    },
                    ..BorderSide::default()
                })
            },
        ))
        .mouse_cursor(
            <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
                Some(adaptive_clickable.resolve(states))
            }),
        )
        .visual_density(VisualDensity::STANDARD)
        .tap_target_size(MaterialTapTargetSize::Padded)
        .splash_factory(theme.splash_factory.clone())
}

/// Dart's `const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(radius)))`.
fn rounded_rectangle(radius: f64) -> Box<dyn OutlinedBorder> {
    Box::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}

/// Dart's `const StadiumBorder()`.
fn stadium() -> Box<dyn OutlinedBorder> {
    Box::new(StadiumBorder::default())
}
