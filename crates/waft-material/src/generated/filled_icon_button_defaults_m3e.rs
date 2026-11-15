//! Flutter counterpart: material_ui's `generated/filled_icon_button_defaults_m3e.g.dart`, a part of
//! `icon_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

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
};

use crate::button_style::{
    ButtonShapeVariant, ButtonSizeVariant, ButtonStyle, ButtonStyleGetters, IconButtonWidthVariant,
};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::theme_data_getters::DefaultsContext;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_FilledIconButtonDefaultsM3E`.
pub(crate) struct FilledIconButtonDefaultsM3E {
    /// Dart's `super(..)` arguments.
    button_style: ButtonStyle,
    context: DefaultsContext,
    toggleable: bool,
    size_variant: Option<ButtonSizeVariant>,
    icon_button_width: Option<IconButtonWidthVariant>,
    shape_variant: Option<ButtonShapeVariant>,
}

impl FilledIconButtonDefaultsM3E {
    pub(crate) fn new(
        context: BuildContext,
        toggleable: bool,
        size_variant: Option<ButtonSizeVariant>,
        icon_button_width: Option<IconButtonWidthVariant>,
        shape_variant: Option<ButtonShapeVariant>,
    ) -> FilledIconButtonDefaultsM3E {
        FilledIconButtonDefaultsM3E {
            button_style: ButtonStyle::new()
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER),
            context: DefaultsContext::new(context),
            toggleable,
            size_variant,
            icon_button_width,
            shape_variant,
        }
    }
}

impl ButtonStyleGetters for FilledIconButtonDefaultsM3E {
    fn button_style(&self) -> &ButtonStyle {
        &self.button_style
    }

    fn size_variant(&self, _app: &mut App) -> Option<ButtonSizeVariant> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(size_variant)
    }

    fn icon_button_width(&self, _app: &mut App) -> Option<IconButtonWidthVariant> {
        let icon_button_width = self.icon_button_width;
        let icon_button_width = icon_button_width.unwrap_or(IconButtonWidthVariant::Standard);
        Some(icon_button_width)
    }

    fn shape_variant(&self, _app: &mut App) -> Option<ButtonShapeVariant> {
        let shape_variant = self.shape_variant;
        let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
        Some(shape_variant)
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(
                        colors
                            .on_surface
                            .with_values(Some(0.1), None, None, None, None),
                    );
                }
                if toggleable && states.contains(&WidgetState::Selected) {
                    return Some(colors.primary);
                }
                if toggleable {
                    return Some(colors.surface_container());
                }
                Some(colors.primary)
            })
        })
    }

    fn foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some({
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
                    return Some(colors.on_primary);
                }
                if toggleable {
                    return Some(colors.on_surface_variant());
                }
                Some(colors.on_primary)
            })
        })
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let toggleable = self.toggleable;
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if toggleable && states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.on_primary.with_values(
                            Some(0.1),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.on_primary.with_values(
                            Some(0.08),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.on_primary.with_values(
                            Some(0.1),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                }
                if toggleable {
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
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(
                        colors
                            .on_primary
                            .with_values(Some(0.1), None, None, None, None),
                    );
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(
                        colors
                            .on_primary
                            .with_values(Some(0.08), None, None, None, None),
                    );
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(
                        colors
                            .on_primary
                            .with_values(Some(0.1), None, None, None, None),
                    );
                }
                Some(Colors::TRANSPARENT)
            },
        ))
    }

    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(Some(0.0))))
    }

    fn shadow_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        let size_variant = self.size_variant;
        let icon_button_width = self.icon_button_width;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        let icon_button_width = icon_button_width.unwrap_or(IconButtonWidthVariant::Standard);
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
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
    }

    fn minimum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        let size_variant = self.size_variant;
        let icon_button_width = self.icon_button_width;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        let icon_button_width = icon_button_width.unwrap_or(IconButtonWidthVariant::Standard);
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
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
    }

    fn maximum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(Rc::new(WidgetStatePropertyAll::new(Some(Size::INFINITE))))
    }

    fn icon_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        let size_variant = self.size_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        Some(Rc::new(WidgetStatePropertyAll::new(Some(
            match size_variant {
                ButtonSizeVariant::XSmall => 20.0,
                ButtonSizeVariant::Small => 24.0,
                ButtonSizeVariant::Medium => 24.0,
                ButtonSizeVariant::Large => 32.0,
                ButtonSizeVariant::XLarge => 40.0,
            },
        ))))
    }

    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        let toggleable = self.toggleable;
        let size_variant = self.size_variant;
        let shape_variant = self.shape_variant;
        let size_variant = size_variant.unwrap_or(ButtonSizeVariant::Small);
        let shape_variant = shape_variant.unwrap_or(ButtonShapeVariant::Round);
        Some(
            <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(move |states| {
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
            }),
        )
    }

    // `side` is null.
    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        let adaptive_clickable = WidgetStateMouseCursor::adaptive_clickable();
        Some(
            <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
                Some(adaptive_clickable.resolve(states))
            }),
        )
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

/// Dart's `const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(radius)))`.
fn rounded_rectangle(radius: f64) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}

/// Dart's `const StadiumBorder()`.
fn stadium() -> OutlinedBorderRef {
    OutlinedBorderRef::new(StadiumBorder::default())
}
