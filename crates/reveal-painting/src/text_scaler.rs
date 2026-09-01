//! Flutter counterpart: `painting/text_scaler.dart`.

use std::fmt::{self, Debug};

use reveal_embedder::clamp_double;

/// A class that describes how textual contents should be scaled for better
/// readability.
///
/// The [`scale`](TextScaler::scale) function computes the scaled font size given
/// the original unscaled font size specified by app developers.
///
/// Non-exhaustive: a later arm for `SystemTextScaler` (widgets) or a user
/// subclass.
#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub enum TextScaler {
    /// Dart's `_LinearTextScaler`.
    Linear {
        /// The estimated number of font pixels for each logical pixel.
        text_scale_factor: f64,
    },
    /// Dart's `_ClampedTextScaler`.
    Clamped {
        /// The scaler whose output is clamped.
        scaler: Box<TextScaler>,
        /// Lower bound on the scale factor.
        min_scale: f64,
        /// Upper bound on the scale factor.
        max_scale: f64,
    },
}

impl TextScaler {
    /// Creates a proportional [`TextScaler`] that scales the incoming font size
    /// by multiplying it with the given `text_scale_factor`.
    pub fn linear(text_scale_factor: f64) -> TextScaler {
        debug_assert!(text_scale_factor >= 0.0);
        TextScaler::Linear { text_scale_factor }
    }

    /// A [`TextScaler`] that doesn't scale the input font size.
    ///
    /// This is equivalent to `TextScaler::linear(1.0)`, the
    /// [`scale`](Self::scale) implementation always returns the input font size
    /// as-is.
    pub const NO_SCALING: TextScaler = TextScaler::Linear {
        text_scale_factor: 1.0,
    };

    /// Computes the scaled font size (in logical pixels) with the given
    /// unscaled `font_size` (in logical pixels).
    ///
    /// The input `font_size` must be finite and non-negative.
    pub fn scale(&self, font_size: f64) -> f64 {
        debug_assert!(font_size >= 0.0);
        debug_assert!(font_size.is_finite());
        match self {
            TextScaler::Linear { text_scale_factor } => font_size * text_scale_factor,
            TextScaler::Clamped {
                scaler,
                min_scale,
                max_scale,
            } => clamp_double(
                scaler.scale(font_size),
                min_scale * font_size,
                max_scale * font_size,
            ),
        }
    }

    /// The estimated number of font pixels for each logical pixel.
    ///
    /// The value of this property is only an estimate, so it may not reflect
    /// the exact text scaling strategy this [`TextScaler`] represents,
    /// especially when this [`TextScaler`] is not linear. Consider using
    /// [`scale`](Self::scale) instead.
    pub fn text_scale_factor(&self) -> f64 {
        match self {
            TextScaler::Linear { text_scale_factor } => *text_scale_factor,
            TextScaler::Clamped {
                scaler,
                min_scale,
                max_scale,
            } => clamp_double(scaler.text_scale_factor(), *min_scale, *max_scale),
        }
    }

    /// Returns a new [`TextScaler`] that restricts the scaled font size to
    /// within the range `[min_scale_factor * font_size, max_scale_factor *
    /// font_size]`.
    pub fn clamp(&self, min_scale_factor: f64, max_scale_factor: f64) -> TextScaler {
        debug_assert!(max_scale_factor >= min_scale_factor);
        debug_assert!(!max_scale_factor.is_nan());
        debug_assert!(min_scale_factor.is_finite());
        debug_assert!(min_scale_factor >= 0.0);

        match self {
            TextScaler::Linear { text_scale_factor } => {
                let new_scale_factor =
                    clamp_double(*text_scale_factor, min_scale_factor, max_scale_factor);
                if new_scale_factor == *text_scale_factor {
                    self.clone()
                } else {
                    TextScaler::linear(new_scale_factor)
                }
            }
            TextScaler::Clamped {
                scaler,
                min_scale,
                max_scale,
            } => {
                let new_min_scale = (*min_scale).max(min_scale_factor);
                let new_max_scale = (*max_scale).min(max_scale_factor);

                if new_max_scale <= new_min_scale {
                    // Ranges don't overlap or collapse to a single point.
                    TextScaler::linear(clamp_double(*min_scale, min_scale_factor, max_scale_factor))
                } else {
                    TextScaler::Clamped {
                        scaler: scaler.clone(),
                        min_scale: new_min_scale,
                        max_scale: new_max_scale,
                    }
                }
            }
        }
    }
}

impl Debug for TextScaler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextScaler::Linear { text_scale_factor } => {
                if *text_scale_factor == 1.0 {
                    write!(f, "no scaling")
                } else {
                    write!(f, "linear ({text_scale_factor}x)")
                }
            }
            TextScaler::Clamped {
                scaler,
                min_scale,
                max_scale,
            } => write!(f, "{scaler:?} clamped [{min_scale}, {max_scale}]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // text_scaler_test.dart 'Linear TextScaler' / 'equality'.
    #[test]
    fn linear_equality() {
        let a = TextScaler::linear(3.0);
        let b = TextScaler::NO_SCALING.clamp(3.0, f64::INFINITY);
        let c = TextScaler::linear(3.0);
        let d = TextScaler::NO_SCALING.clamp(1.0, 5.0).clamp(3.0, 6.0);
        let list = [a.clone(), b, c, d];
        for lhs in &list {
            for rhs in &list {
                assert_eq!(lhs, rhs);
            }
        }
    }

    // text_scaler_test.dart 'Linear TextScaler' / 'clamping'.
    #[test]
    fn linear_clamping() {
        assert_eq!(
            TextScaler::NO_SCALING.clamp(3.0, f64::INFINITY),
            TextScaler::linear(3.0)
        );
        assert_eq!(
            TextScaler::linear(5.0).clamp(0.0, 3.0),
            TextScaler::linear(3.0)
        );
        assert_eq!(
            TextScaler::linear(5.0).clamp(3.0, 3.0),
            TextScaler::linear(3.0)
        );
    }
}
