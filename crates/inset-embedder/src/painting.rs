//! Flutter counterpart: dart:ui `Canvas` / `Paint` / `Path` / `Picture` /
//! `Image`. These are valo types under Flutter names; the paragraph types live in
//! `paragraph.rs`.
//!
//! Recording uses valo method signatures (`draw_rect`, f32 `Rect`). Convert
//! framework geometry with [`From`] at the call (`rect.into()`).

use std::fmt::Display;
use std::sync::Arc;

use valo::DisplayList;
use valo::DisplayListBuilder;
use valo::Op;

/// Flutter `Canvas` — valo records into a display list.
pub type Canvas = DisplayListBuilder;

/// Flutter `Picture` — a finished display list, what a scene's layers hold and what a host
/// draws.
pub type Picture = DisplayList;

/// The picture's operations in replay order with every embedded list's following its embedding
/// op, for a test or a diagnostic that asks what a scene draws regardless of how its layers
/// nested it.
pub fn flattened_ops(picture: &Picture) -> Vec<Op> {
    let mut ops = Vec::new();
    flatten_into(picture, &mut ops);
    ops
}

fn flatten_into(picture: &Picture, ops: &mut Vec<Op>) {
    for op in picture.ops() {
        ops.push(op.clone());
        if let Op::DrawDisplayList { list, .. } = op {
            flatten_into(list, ops);
        }
    }
}

pub use valo::{
    Backdrop, BlendMode, BlurStyle, ClipOp, FillRule, Image, MaskBlur, Paint, PaintStyle, Path,
    PathBuilder, Stroke,
};

/// The full valo API for hosts and for recording that needs a type the
/// aliases do not cover (`Op`, `Context`, `Surface`).
pub use valo;

/// A description of a color filter to apply when drawing a shape or compositing
/// a layer with a particular [`Paint`]. A color filter is a function that takes
/// two colors, and outputs one color. When applied during compositing, it is
/// independently applied to each pixel of the layer being drawn before the
/// entire layer is merged with the destination.
///
/// Instances of this class are used with [`Paint`]'s color filter on `Paint`
/// objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorFilter {
    /// Applies the blend mode, with the color as the source and the layer being
    /// composited as the destination; Dart's `ColorFilter.mode`.
    Mode {
        /// The source color of the blend.
        color: crate::Color,
        /// How the source color is composited onto the destination.
        blend_mode: BlendMode,
    },

    /// A 4x5 row-major matrix over `[R, G, B, A]`; Dart's `ColorFilter.matrix`.
    ///
    /// Every pixel's color value, represented as an `[R, G, B, A]`, is matrix
    /// multiplied to create a new color:
    ///
    /// ```text
    /// | R' |   | a00 a01 a02 a03 a04 |   | R |
    /// | G' |   | a10 a11 a12 a13 a14 |   | G |
    /// | B' | = | a20 a21 a22 a23 a24 | * | B |
    /// | A' |   | a30 a31 a32 a33 a34 |   | A |
    /// | 1  |   |  0   0   0   0   1  |   | 1 |
    /// ```
    ///
    /// The matrix is in row-major order and the translation column is specified
    /// in unnormalized, 0...255, space.
    Matrix([f64; 20]),

    /// Applies the sRGB gamma curve to the RGB channels; Dart's
    /// `ColorFilter.linearToSrgbGamma`.
    LinearToSrgbGamma,

    /// Applies the inverse of the sRGB gamma curve to the RGB channels; Dart's
    /// `ColorFilter.srgbToLinearGamma`.
    SrgbToLinearGamma,
}

impl ColorFilter {
    /// Creates a color filter that applies the blend mode given as the second
    /// argument. The source color is the one given as the first argument, and the
    /// destination color is the one from the layer being composited.
    ///
    /// The output of this filter is then composited into the background according
    /// to the `Paint`'s blend mode, using the output of this filter as the source
    /// and the background as the destination.
    pub const fn mode(color: crate::Color, blend_mode: BlendMode) -> ColorFilter {
        ColorFilter::Mode { color, blend_mode }
    }

    /// Constructs a color filter from a 4x5 row-major matrix. The matrix is
    /// interpreted as a 5x5 matrix, where the fifth row is the identity
    /// configuration.
    ///
    /// The identity matrix is:
    ///
    /// ```
    /// # use inset_embedder::ColorFilter;
    /// const IDENTITY: ColorFilter = ColorFilter::matrix([
    ///     1.0, 0.0, 0.0, 0.0, 0.0, //
    ///     0.0, 1.0, 0.0, 0.0, 0.0, //
    ///     0.0, 0.0, 1.0, 0.0, 0.0, //
    ///     0.0, 0.0, 0.0, 1.0, 0.0, //
    /// ]);
    /// ```
    pub const fn matrix(matrix: [f64; 20]) -> ColorFilter {
        ColorFilter::Matrix(matrix)
    }

    /// Constructs a color filter that applies the sRGB gamma curve to the RGB
    /// channels.
    pub const fn linear_to_srgb_gamma() -> ColorFilter {
        ColorFilter::LinearToSrgbGamma
    }

    /// Creates a color filter that applies the inverse of the sRGB gamma curve
    /// to the RGB channels.
    pub const fn srgb_to_linear_gamma() -> ColorFilter {
        ColorFilter::SrgbToLinearGamma
    }

    /// Creates a color filter that applies the given saturation to the RGB
    /// channels.
    pub fn saturation(saturation: f64) -> ColorFilter {
        const R_LUMINANCE: f64 = 0.2126;
        const G_LUMINANCE: f64 = 0.7152;
        const B_LUMINANCE: f64 = 0.0722;
        let inv_sat = 1.0 - saturation;

        ColorFilter::matrix([
            inv_sat * R_LUMINANCE + saturation,
            inv_sat * G_LUMINANCE,
            inv_sat * B_LUMINANCE,
            0.0,
            0.0,
            inv_sat * R_LUMINANCE,
            inv_sat * G_LUMINANCE + saturation,
            inv_sat * B_LUMINANCE,
            0.0,
            0.0,
            inv_sat * R_LUMINANCE,
            inv_sat * G_LUMINANCE,
            inv_sat * B_LUMINANCE + saturation,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
    }

    /// Dart's `debugShortDescription`.
    pub fn debug_short_description(&self) -> String {
        match self {
            ColorFilter::Mode { color, blend_mode } => {
                format!("ColorFilter.mode({color:?}, {blend_mode:?})")
            }
            ColorFilter::Matrix(matrix) => format!("ColorFilter.matrix({matrix:?})"),
            ColorFilter::LinearToSrgbGamma => String::from("ColorFilter.linearToSrgbGamma()"),
            ColorFilter::SrgbToLinearGamma => String::from("ColorFilter.srgbToLinearGamma()"),
        }
    }
}

impl std::fmt::Display for ColorFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.debug_short_description())
    }
}

impl ColorFilter {
    /// The valo filter that replays this one.
    pub fn to_valo(&self) -> valo::ColorFilter {
        match *self {
            ColorFilter::Mode { color, blend_mode } => {
                valo::ColorFilter::Blend(color.into(), blend_mode)
            }
            ColorFilter::Matrix(matrix) => valo::ColorFilter::Matrix(valo_matrix(matrix)),
            ColorFilter::LinearToSrgbGamma => valo::ColorFilter::LinearToSrgbGamma,
            ColorFilter::SrgbToLinearGamma => valo::ColorFilter::SrgbToLinearGamma,
        }
    }
}

/// A colour matrix in valo's terms: the translation column, which Flutter gives in 0...255,
/// in the 0...1 its colour channels are in.
fn valo_matrix(matrix: [f64; 20]) -> [f32; 20] {
    const ROW_LENGTH: usize = 5;
    const TRANSLATION_COLUMN: usize = 4;
    std::array::from_fn(|index| {
        let entry = matrix[index];
        let entry = if index % ROW_LENGTH == TRANSLATION_COLUMN {
            entry / 255.0
        } else {
            entry
        };
        entry as f32
    })
}

/// A filter operation to apply to a raster image.
///
/// See also:
///
///  * `BackdropFilter`, a widget that applies [`ImageFilter`] to its rendering.
///  * `ImageFiltered`, a widget that applies [`ImageFilter`] to its children.
///  * `SceneBuilder.pushBackdropFilter`, which is the low-level API for using
///    this class as a backdrop filter.
///  * `SceneBuilder.pushImageFilter`, which is the low-level API for using
///    this class as a child layer filter.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageFilter {
    /// A Gaussian blur; Dart's `_GaussianBlurImageFilter`.
    Blur {
        /// The standard deviation along the horizontal axis.
        sigma_x: f64,
        /// The standard deviation along the vertical axis.
        sigma_y: f64,
        /// The area the blur is confined to, when given.
        bounds: Option<crate::Rect>,
    },

    /// A [`ColorFilter`] used as an image filter; Dart's `ColorFilter implements
    /// ImageFilter`.
    Color(ColorFilter),

    /// `outer` applied to the result of `inner`; Dart's `_ComposeImageFilter`.
    Compose {
        /// The filter that receives the filtered result of `inner`.
        outer: Arc<ImageFilter>,
        /// The filter that receives the original input.
        inner: Arc<ImageFilter>,
    },
}

impl ImageFilter {
    /// Creates an image filter that applies a Gaussian blur.
    pub const fn blur(sigma_x: f64, sigma_y: f64) -> ImageFilter {
        ImageFilter::Blur {
            sigma_x,
            sigma_y,
            bounds: None,
        }
    }

    /// Dart `ImageFilter.blur(bounds:)`.
    pub fn bounds(self, bounds: crate::Rect) -> ImageFilter {
        match self {
            ImageFilter::Blur {
                sigma_x, sigma_y, ..
            } => ImageFilter::Blur {
                sigma_x,
                sigma_y,
                bounds: Some(bounds),
            },
            other => other,
        }
    }

    /// Composes the `inner` filter with `outer`, to combine their effects.
    ///
    /// Creates a single [`ImageFilter`] that when applied, has the same effect as
    /// subsequently applying `inner` and `outer`, i.e.,
    /// `result = outer(inner(source))`.
    pub fn compose(outer: ImageFilter, inner: ImageFilter) -> ImageFilter {
        ImageFilter::Compose {
            outer: Arc::new(outer),
            inner: Arc::new(inner),
        }
    }

    /// Dart's `debugShortDescription`.
    pub fn debug_short_description(&self) -> String {
        match self {
            ImageFilter::Blur {
                sigma_x,
                sigma_y,
                bounds,
            } => {
                let bounds = match bounds {
                    Some(bounds) => format!(", bounds: {bounds:?}"),
                    None => String::new(),
                };
                format!("blur({sigma_x}, {sigma_y}, unspecified{bounds})")
            }
            ImageFilter::Color(filter) => filter.debug_short_description(),
            ImageFilter::Compose { outer, inner } => format!(
                "{} -> {}",
                inner.debug_short_description(),
                outer.debug_short_description()
            ),
        }
    }
}

impl std::fmt::Display for ImageFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFilter::Blur { .. } => write!(f, "ImageFilter.{}", self.debug_short_description()),
            ImageFilter::Color(filter) => Display::fmt(filter, f),
            ImageFilter::Compose { .. } => write!(
                f,
                "ImageFilter.compose(source -> {} -> result)",
                self.debug_short_description()
            ),
        }
    }
}

impl ImageFilter {
    /// The valo filter that replays this one, step for step and in the same order.
    pub fn to_valo(&self) -> valo::ImageFilter {
        match self {
            ImageFilter::Blur {
                sigma_x,
                sigma_y,
                bounds,
            } => {
                let (sigma_x, sigma_y) = (*sigma_x as f32, *sigma_y as f32);
                match bounds {
                    Some(bounds) => {
                        valo::ImageFilter::bounded_blur(sigma_x, sigma_y, (*bounds).into())
                    }
                    None => valo::ImageFilter::blur(sigma_x, sigma_y),
                }
            }
            ImageFilter::Color(filter) => valo::ImageFilter::Color(filter.to_valo()),
            ImageFilter::Compose { outer, inner } => {
                valo::ImageFilter::compose(outer.to_valo(), inner.to_valo())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorFilter, ImageFilter, valo};
    use crate::{BlendMode, Color, Rect};

    #[test]
    fn a_saturation_of_one_is_the_identity_matrix() {
        let ColorFilter::Matrix(matrix) = ColorFilter::saturation(1.0) else {
            panic!("saturation is a matrix filter");
        };
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, 0.0, //
        ];
        for (value, expected) in matrix.iter().zip(identity) {
            assert!((value - expected).abs() < 1e-12, "{matrix:?}");
        }
    }

    #[test]
    fn a_blur_reports_its_bounds_and_a_composition_reads_inner_then_outer() {
        let blur = ImageFilter::blur(2.0, 3.0).bounds(Rect::from_ltwh(0.0, 0.0, 4.0, 5.0));
        assert_eq!(
            blur.debug_short_description(),
            "blur(2, 3, unspecified, bounds: Rect.fromLTRB(0.0, 0.0, 4.0, 5.0))"
        );

        let tint = ImageFilter::Color(ColorFilter::mode(
            Color::new(0xFF000000),
            BlendMode::SrcOver,
        ));
        let composed = ImageFilter::compose(tint, ImageFilter::blur(2.0, 3.0));
        assert_eq!(
            composed.to_string(),
            "ImageFilter.compose(source -> blur(2, 3, unspecified) -> \
             ColorFilter.mode(Color(alpha: 1.0000, red: 0.0000, green: 0.0000, blue: 0.0000, \
             colorSpace: ColorSpace.sRGB), SrcOver) -> result)"
        );
    }

    #[test]
    fn a_colour_matrix_reaches_valo_with_its_translation_in_the_channels_range() {
        let mut matrix = [0.0; 20];
        matrix[0] = 2.0;
        matrix[4] = 51.0;
        matrix[18] = 1.0;
        matrix[19] = 255.0;
        let valo::ColorFilter::Matrix(replayed) = ColorFilter::matrix(matrix).to_valo() else {
            panic!("a matrix reaches valo as a matrix");
        };
        assert_eq!(
            [replayed[0], replayed[4], replayed[18], replayed[19]],
            [2.0, 0.2, 1.0, 1.0]
        );
    }

    #[test]
    fn a_mode_filter_reaches_valo_as_a_blend_and_a_gamma_curve_as_the_same_curve() {
        let green = Color::new(0x8C00FF00);
        assert_eq!(
            ColorFilter::mode(green, BlendMode::Darken).to_valo(),
            valo::ColorFilter::Blend(green.into(), BlendMode::Darken)
        );
        assert_eq!(
            ColorFilter::linear_to_srgb_gamma().to_valo(),
            valo::ColorFilter::LinearToSrgbGamma
        );
        assert_eq!(
            ColorFilter::srgb_to_linear_gamma().to_valo(),
            valo::ColorFilter::SrgbToLinearGamma
        );
    }

    #[test]
    fn a_composition_reaches_valo_in_its_order_with_both_sigmas_and_the_bounds() {
        let tint = ColorFilter::mode(Color::new(0xFF0000FF), BlendMode::Multiply);
        let bounds = Rect::from_ltwh(0.0, 0.0, 4.0, 4.0);
        let filter = ImageFilter::compose(
            ImageFilter::Color(tint),
            ImageFilter::blur(2.0, 5.0).bounds(bounds),
        );
        assert_eq!(
            filter.to_valo(),
            valo::ImageFilter::compose(
                valo::ImageFilter::Color(valo::ColorFilter::Blend(
                    Color::new(0xFF0000FF).into(),
                    BlendMode::Multiply
                )),
                valo::ImageFilter::bounded_blur(2.0, 5.0, bounds.into()),
            )
        );
    }

    #[test]
    fn a_gamma_curve_reaches_valo_in_its_place_in_a_composition() {
        let filter = ImageFilter::compose(
            ImageFilter::Color(ColorFilter::linear_to_srgb_gamma()),
            ImageFilter::blur(3.0, 3.0),
        );
        assert_eq!(
            filter.to_valo(),
            valo::ImageFilter::compose(
                valo::ImageFilter::Color(valo::ColorFilter::LinearToSrgbGamma),
                valo::ImageFilter::blur(3.0, 3.0),
            )
        );
    }
}
