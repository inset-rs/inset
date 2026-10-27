//! ElevationHelper.cpp and DropShadowRecipe.h, using Inset's native shadow blur.

use crate::{Theme, ThemeResources};
use inset_embedder::{BlurStyle, Canvas, ClipOp, Color, Offset, RRect, Radius, Size};
use inset_foundation::App;
use inset_painting::{BoxShadow, draw_rrect};
use inset_rendering::CustomPainter;

/// Paints only outside the popup, so acrylic samples an undarkened background.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PopupShadow {
    /// Theme used by the popup's content.
    theme: Theme,

    /// The `Translation.Z` ApplyElevationEffect raises the popup to.
    translation_z: f64,

    /// The popup's corner radii, top-left first and clockwise.
    corner_radius: [f64; 4],
}

impl PopupShadow {
    /// A flyout's, `depth` submenus deep: ElevationHelper's base Z of 32 and 8 more per level.
    pub(crate) fn for_flyout(r: &ThemeResources, theme: Theme, depth: usize) -> PopupShadow {
        PopupShadow {
            theme,
            translation_z: 32.0 + depth as f64 * 8.0,
            corner_radius: r.overlay_corner_radius,
        }
    }

    /// A dialog's: ContentDialog raises its background element to a Z of 128.
    pub(crate) fn for_dialog(r: &ThemeResources, theme: Theme) -> PopupShadow {
        PopupShadow {
            theme,
            translation_z: 128.0,
            corner_radius: r.overlay_corner_radius,
        }
    }

    /// NumberBox's popup: `OnApplyTemplate` raises `PopupContentRoot` to a Z of
    /// `NumberBoxPopupShadowDepth`.
    pub(crate) fn for_number_box_popup(r: &ThemeResources, theme: Theme) -> PopupShadow {
        PopupShadow {
            theme,
            translation_z: r.number_box_popup_shadow_depth,
            corner_radius: r.overlay_corner_radius,
        }
    }

    /// ComboBox's popup: `OnOpen` gives `PopupBorder` ElevationHelper's base Z of 32; an
    /// editable box's popup squares the corners next to the box.
    pub(crate) fn for_combo_box_popup(theme: Theme, corner_radius: [f64; 4]) -> PopupShadow {
        PopupShadow {
            theme,
            translation_z: 32.0,
            corner_radius,
        }
    }

    /// Source shadow colors and offsets, with blur radii interpreted by BoxShadow.
    fn shadows(&self) -> [BoxShadow; 2] {
        // The recipe halves the Z translation, up to 64.
        let elevation = (self.translation_z / 2.0).min(64.0);
        let (ambient_blur, ambient_offset, ambient_opacity, directional_opacity) =
            if elevation <= 16.0 {
                (
                    2.0,
                    0.0,
                    0.0,
                    if self.theme == Theme::Light {
                        0.14
                    } else {
                        0.26
                    },
                )
            } else {
                let (ambient, directional) = if self.theme == Theme::Light {
                    (0.15, 0.19)
                } else {
                    (0.37, 0.37)
                };
                (elevation / 3.0, 2.0, ambient, directional)
            };

        let shadow = |blur, y, opacity: f64| {
            BoxShadow::new(
                Color::from_argb((opacity * 255.0) as i32, 0, 0, 0),
                Offset::new(0.0, y),
                blur,
                0.0,
                BlurStyle::Normal,
            )
        };

        [
            shadow(ambient_blur, ambient_offset, ambient_opacity),
            shadow(elevation, elevation * 0.5, directional_opacity),
        ]
    }
}

impl CustomPainter for PopupShadow {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let bounds = Offset::ZERO & size;
        let radii = self
            .corner_radius
            .map(|radius| radius.min(size.shortest_side() / 2.0));
        let outline = RRect::from_rect_and_corners(
            bounds,
            Radius::circular(radii[0]),
            Radius::circular(radii[1]),
            Radius::circular(radii[2]),
            Radius::circular(radii[3]),
        );

        canvas.save();
        canvas.clip_rrect_radii(
            bounds,
            radii.map(|radius| radius as f32),
            ClipOp::Difference,
        );
        for shadow in self.shadows() {
            if shadow.color.a == 0.0 {
                continue;
            }
            draw_rrect(canvas, outline.shift(shadow.offset), &shadow.to_paint());
        }
        canvas.restore();
    }

    /// The shadow never enlarges the popup's interactive area.
    fn hit_test(&self, _app: &App, _position: Offset) -> Option<bool> {
        Some(false)
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
