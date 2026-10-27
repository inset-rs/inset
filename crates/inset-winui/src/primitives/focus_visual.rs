//! XAML's system focus visual (`UseSystemFocusVisuals="True"`): a two-pixel primary ring outside the control with a one-pixel secondary ring inside it, offset by the control's `FocusVisualMargin`, shown when the control requests a focus highlight.

use crate::{Brush, Theme};
use inset_embedder::{Canvas, Color, Offset, RRect, Radius, Size};
use inset_foundation::App;
use inset_rendering::CustomPainter;
use inset_widgets::*;

/// XAML `SystemFocusVisualPrimaryThickness`.
pub const FOCUS_VISUAL_PRIMARY_THICKNESS: f64 = 2.0;
/// XAML `SystemFocusVisualSecondaryThickness`.
pub const FOCUS_VISUAL_SECONDARY_THICKNESS: f64 = 1.0;

/// Draws the focus rings around `child` when `visible`, `margin` (left, top, right, bottom; negative grows outward, as XAML's `FocusVisualMargin`) away from its bounds.
#[derive(Clone, Debug)]
pub struct FocusVisual {
    pub child: WidgetRef,
    pub visible: bool,
    pub margin: [f64; 4],
    pub corner_radius: f64,
    pub theme: Theme,
    /// XAML `FocusVisualPrimaryBrush`; `None` is `SystemControlFocusVisualPrimaryBrush`.
    pub primary_brush: Option<Color>,
    /// XAML `FocusVisualSecondaryBrush`; `None` is `SystemControlFocusVisualSecondaryBrush`.
    pub secondary_brush: Option<Color>,
}

impl FocusVisual {
    pub fn new<K>(child: impl IntoWidget<K>, theme: Theme) -> FocusVisual {
        FocusVisual {
            child: child.into_widget(),
            visible: false,
            margin: [0.0; 4],
            corner_radius: 0.0,
            theme,
            primary_brush: None,
            secondary_brush: None,
        }
    }

    pub fn visible(mut self, visible: bool) -> FocusVisual {
        self.visible = visible;
        self
    }

    /// XAML `FocusVisualMargin`.
    pub fn margin(mut self, margin: [f64; 4]) -> FocusVisual {
        self.margin = margin;
        self
    }

    pub fn corner_radius(mut self, radius: f64) -> FocusVisual {
        self.corner_radius = radius;
        self
    }

    /// XAML `FocusVisualPrimaryBrush`.
    pub fn primary_brush(mut self, color: Color) -> FocusVisual {
        self.primary_brush = Some(color);
        self
    }

    /// XAML `FocusVisualSecondaryBrush`.
    pub fn secondary_brush(mut self, color: Color) -> FocusVisual {
        self.secondary_brush = Some(color);
        self
    }

    /// `SystemControlFocusVisualPrimaryBrush`: the system's base-high colour, black in light and white in dark.
    pub fn primary_color(theme: Theme) -> Color {
        match theme {
            Theme::Light => Color::from_argb(255, 0, 0, 0),
            Theme::Dark => Color::from_argb(255, 255, 255, 255),
        }
    }

    /// `SystemControlFocusVisualSecondaryBrush`: the system's alt-medium colour, white at 60 % in light and black at 60 % in dark.
    pub fn secondary_color(theme: Theme) -> Color {
        match theme {
            Theme::Light => Color::from_argb(153, 255, 255, 255),
            Theme::Dark => Color::from_argb(153, 0, 0, 0),
        }
    }
}

impl StatelessWidget for FocusVisual {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // The tree keeps the same shape whether the ring shows or not, so the child's elements
        // (and a pointer's cached hit-test path through them) survive focus arriving mid-press.
        let [left, top, right, bottom] = self.margin;
        let ring = FocusRingPainter {
            visible: self.visible,
            primary: Brush::Solid(
                self.primary_brush
                    .unwrap_or_else(|| FocusVisual::primary_color(self.theme)),
            ),
            secondary: Brush::Solid(
                self.secondary_brush
                    .unwrap_or_else(|| FocusVisual::secondary_color(self.theme)),
            ),
            corner_radius: self.corner_radius - left.min(0.0),
        };
        // Passthrough: the child sees the constraints it would see without the ring.
        Stack::new()
            .fit(inset_rendering::StackFit::Passthrough)
            .clip_behavior(inset_embedder::Clip::None)
            .children(vec![
                self.child.clone(),
                Positioned::new(CustomPaint::new().painter(ring))
                    .left(left)
                    .top(top)
                    .right(right)
                    .bottom(bottom)
                    .into_widget(),
            ])
            .into_widget()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FocusRingPainter {
    visible: bool,
    primary: Brush,
    secondary: Brush,
    corner_radius: f64,
}

impl CustomPainter for FocusRingPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        if !self.visible {
            return;
        }
        let bounds = Offset::ZERO & size;
        let outer = RRect::from_rect_and_radius(bounds, Radius::circular(self.corner_radius));
        self.primary.paint_rrect_stroke(
            canvas,
            outer.deflate(FOCUS_VISUAL_PRIMARY_THICKNESS / 2.0),
            bounds,
            FOCUS_VISUAL_PRIMARY_THICKNESS,
        );
        self.secondary.paint_rrect_stroke(
            canvas,
            outer.deflate(FOCUS_VISUAL_PRIMARY_THICKNESS + FOCUS_VISUAL_SECONDARY_THICKNESS / 2.0),
            bounds,
            FOCUS_VISUAL_SECONDARY_THICKNESS,
        );
    }

    /// The ring is drawn over the control but never in front of it for the pointer.
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
