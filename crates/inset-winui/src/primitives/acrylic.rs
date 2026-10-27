//! AcrylicBrush's source tint and luminosity calculation and in-window effect graph.
//!
//! Source: controls/dev/Materials/Acrylic/AcrylicBrush.cpp.

use crate::theme::AcrylicBrushResources;
use inset_embedder::{Backdrop, BlendMode, Canvas, Color, Paint, Rect};

/// The two effective source colours, after WinUI's opacity suppression and HSV clamp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AcrylicColors {
    /// The foreground of the semantic Color blend.
    pub tint: Color,
    /// The foreground of the semantic Luminosity blend.
    pub luminosity: Color,
}

impl AcrylicBrushResources {
    /// `GetEffectiveTintColor` and `GetEffectiveLuminosityColor`, including byte rounding.
    pub(crate) fn effective_colors(self) -> AcrylicColors {
        let color = self.tint_color;
        let [h, s, v] = rgb_to_hsv(color);
        let modifier = if self.tint_luminosity_opacity.is_some() {
            1.0
        } else {
            let mut opacity_modifier = 0.90;
            if v != 0.50 {
                let lowest_max_opacity = if v > 0.50 { 0.45 } else { 0.85 };
                let mut max_opacity_suppression = 0.90 - lowest_max_opacity;
                if s > 0.0 {
                    max_opacity_suppression *= (1.0 - s * 2.0).max(0.0);
                }
                opacity_modifier -= max_opacity_suppression * (v - 0.50).abs() / 0.50;
            }
            opacity_modifier
        };
        let tint_alpha = (color.a * 255.0 * self.tint_opacity * modifier).round() as i32;
        let tint = Color::from_argb(tint_alpha, byte(color.r), byte(color.g), byte(color.b));
        let luminosity = if let Some(opacity) = self.tint_luminosity_opacity {
            Color::from_argb(
                byte(opacity.clamp(0.0, 1.0)),
                byte(color.r),
                byte(color.g),
                byte(color.b),
            )
        } else {
            let original_alpha = (color.a * 255.0 * self.tint_opacity).round() / 255.0;
            let opacity = (original_alpha * (1.03 - 0.15) + 0.15).min(1.0);
            let [r, g, b] = hsv_to_rgb([h, s, v.clamp(0.125, 0.965)]);
            Color::from_argb(byte(opacity), byte(r), byte(g), byte(b))
        };
        AcrylicColors { tint, luminosity }
    }
}

/// Rounds a normalized channel to the source byte representation.
fn byte(value: f64) -> i32 {
    (value * 255.0).round() as i32
}

/// `ColorConversion.cpp::RgbToHsv` in normalized channel space.
fn rgb_to_hsv(color: Color) -> [f64; 3] {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let chroma = max - min;
    if chroma == 0.0 {
        return [0.0, 0.0, max];
    }
    let mut hue = if color.r == max {
        60.0 * (color.g - color.b) / chroma
    } else if color.g == max {
        120.0 + 60.0 * (color.b - color.r) / chroma
    } else {
        240.0 + 60.0 * (color.r - color.g) / chroma
    };
    if hue < 0.0 {
        hue += 360.0;
    }
    [hue, chroma / max, max]
}

/// `ColorConversion.cpp::HsvToRgb`'s six chroma sectors and common channel offset.
fn hsv_to_rgb([h, s, v]: [f64; 3]) -> [f64; 3] {
    let hue = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let chroma = s * v;
    let intermediate = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let rgb = if hue < 60.0 {
        [chroma, intermediate, 0.0]
    } else if hue < 120.0 {
        [intermediate, chroma, 0.0]
    } else if hue < 180.0 {
        [0.0, chroma, intermediate]
    } else if hue < 240.0 {
        [0.0, intermediate, chroma]
    } else if hue < 300.0 {
        [intermediate, 0.0, chroma]
    } else {
        [chroma, 0.0, intermediate]
    };
    rgb.map(|channel| channel + v - chroma)
}

/// Composes only the material; the caller clips the shape and paints children after this returns.
pub(super) fn paint_acrylic(canvas: &mut Canvas, bounds: Rect, resources: AcrylicBrushResources) {
    let colors = resources.effective_colors();
    if colors.tint.a == 1.0 {
        canvas.draw_rect(bounds, &Paint::from_color(colors.tint.into()));
        return;
    }
    let fallback = resources.fallback_color.with_alpha(255);
    // AcrylicBrush.cpp: opaque fallback under backdrop, hard-border Gaussian blur30.
    canvas.save_layer_backdrop(
        Some(bounds.into()),
        &Paint::default(),
        Backdrop::new(inset_embedder::valo::ImageFilter::compose(
            inset_embedder::valo::ImageFilter::blur(30.0, 30.0),
            inset_embedder::valo::ImageFilter::color(inset_embedder::valo::ColorFilter::Blend(
                fallback.into(),
                BlendMode::DstOver,
            )),
        )),
    );
    // The Windows effect enum names are swapped by a documented compositor bug.
    // These are the semantic modes described by CombineNoiseWithTintEffect_Luminosity.
    let mut luminosity = Paint::from_color(colors.luminosity.into());
    luminosity.blend_mode = BlendMode::Luminosity;
    canvas.draw_rect(bounds, &luminosity);
    let mut tint = Paint::from_color(colors.tint.into());
    tint.blend_mode = BlendMode::Color;
    canvas.draw_rect(bounds, &tint);
    canvas.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(color: Color, explicit: Option<f64>) -> AcrylicBrushResources {
        AcrylicBrushResources {
            tint_color: color,
            tint_opacity: 1.0,
            tint_luminosity_opacity: explicit,
            fallback_color: Color::new(0xFFFFFFFF),
        }
    }

    #[test]
    fn source_opacity_suppression_and_luminosity_clamps() {
        let white = recipe(Color::new(0xFFFFFFFF), None).effective_colors();
        let black = recipe(Color::new(0xFF000000), None).effective_colors();
        let red = recipe(Color::new(0xFFFF0000), None).effective_colors();
        assert_eq!(white.tint, Color::from_argb(115, 255, 255, 255));
        assert_eq!(black.tint, Color::from_argb(217, 0, 0, 0));
        assert_eq!(red.tint, Color::from_argb(230, 255, 0, 0));
        assert_eq!(white.luminosity, Color::from_argb(255, 246, 246, 246));
        assert_eq!(black.luminosity, Color::from_argb(255, 32, 32, 32));
    }

    #[test]
    fn explicit_luminosity_preserves_tint_and_uses_its_own_alpha() {
        let colors = recipe(Color::new(0x80FF0000), Some(0.25)).effective_colors();
        assert_eq!(colors.tint, Color::new(0x80FF0000));
        assert_eq!(colors.luminosity, Color::from_argb(64, 255, 0, 0));
    }
}
