//! Flutter counterpart: dart:ui `Canvas` / `Paint` / `Path` / `Picture` /
//! `Image`. These are valo types under Flutter names, but for [`Path`], a valo path with the
//! fill type valo takes beside it; the paragraph types live in `paragraph.rs`.
//!
//! Recording uses valo method signatures (`draw_rect`, f32 `Rect`). Convert
//! framework geometry with [`From`] at the call (`rect.into()`).

use std::fmt::Display;
use std::sync::Arc;

use valo::DisplayList;
use valo::DisplayListBuilder;
use valo::Op;

use crate::{Matrix4, Offset, Rect};

/// Flutter `Canvas` — valo records into a display list.
pub type Canvas = DisplayListBuilder;

/// Flutter `Picture` — a finished display list, what a scene's layers hold and what a host
/// draws.
pub type Picture = DisplayList;

/// Flutter `StrokeCap` — valo's `Cap`: how a stroke ends at an open contour, `Butt` by default.
pub type StrokeCap = valo::Cap;

/// Flutter `PathFillType` — valo's `FillRule`: how the interior of a [`Path`] is calculated,
/// which valo takes beside the path's contours wherever it fills, clips to or hit tests them.
pub type PathFillType = valo::FillRule;

/// Flutter `PaintingStyle` — valo's `PaintStyle`: whether a [`Paint`] fills a shape or strokes
/// its outline. Its `Stroke` variant carries the [`Stroke`] (width, cap, join, miter limit) that
/// a Dart `Paint` keeps in fields of its own.
pub type PaintingStyle = valo::PaintStyle;

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
    Backdrop, BlendMode, BlurStyle, ClipOp, FillRule, Image, MaskBlur, Paint, PaintStyle,
    PathBuilder, PathOperation, Shader, Stroke,
};

/// A complex, one-dimensional subset of a plane: dart:ui's `Path`.
///
/// Its contours are a valo path, built with a [`PathBuilder`] and shared, not copied, by every
/// clone; its [`fill_type`](Self::fill_type) is what valo is handed with them wherever the path
/// is filled, clipped to or hit tested. A call that passes [`as_valo`](Self::as_valo) to valo
/// without the fill type fills the path with whatever rule it names instead.
///
/// Two paths are equal when their contours and fill types are, which is how a scene tells a
/// clip that changed from one that did not.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    contours: Arc<valo::Path>,
    /// Determines how the interior of this path is calculated.
    ///
    /// Defaults to the non-zero winding rule, [`PathFillType::NonZero`].
    pub fill_type: PathFillType,
}

impl Path {
    /// The path of `contours`, filled with the non-zero winding rule, as dart:ui's `Path()` is.
    pub fn new(contours: Arc<valo::Path>) -> Path {
        Path {
            contours,
            fill_type: PathFillType::NonZero,
        }
    }

    /// The contours, for valo's calls, which take the fill type beside them.
    pub fn as_valo(&self) -> &Arc<valo::Path> {
        &self.contours
    }

    /// Tests to see if the given point is within the path. (That is, whether the point would be
    /// in the visible portion of the path if the path was used with [`Canvas::clip_path`].)
    ///
    /// The `point` argument is interpreted as an offset from the origin.
    ///
    /// Returns true if the point is in the path, and false otherwise.
    pub fn contains(&self, point: Offset) -> bool {
        self.contours.contains(point.into(), self.fill_type)
    }

    /// Computes the bounding rectangle for this path.
    ///
    /// A path containing only axis-aligned points on the same straight line will have no area,
    /// and therefore `Rect::is_empty` will return true for such a path.
    pub fn get_bounds(&self) -> Rect {
        let bounds = self.contours.bounds();
        Rect::from_ltwh(
            f64::from(bounds.x),
            f64::from(bounds.y),
            f64::from(bounds.width),
            f64::from(bounds.height),
        )
    }

    /// Returns a copy of the path with all the segments of every subpath translated by the given
    /// offset: Skia's `makeTransform` by a translation, so a path built as a rounded rectangle or
    /// an oval stays one, and a shadow drawn from it keeps its closed form.
    pub fn shift(&self, offset: Offset) -> Path {
        if offset == Offset::ZERO {
            return self.clone();
        }
        self.transform(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ))
    }

    /// Returns a copy of the path with all the segments of every sub path transformed by the
    /// given matrix.
    pub fn transform(&self, matrix: &Matrix4) -> Path {
        Path {
            contours: self.contours.transformed(matrix),
            fill_type: self.fill_type,
        }
    }
}

impl From<Arc<valo::Path>> for Path {
    fn from(contours: Arc<valo::Path>) -> Path {
        Path::new(contours)
    }
}

/// Combines the two paths according to the manner specified by the given `operation`: dart:ui's
/// `Path.combine`, Skia's `Op`, which reads each path with its own fill type.
///
/// The resulting path will be constructed from non-overlapping contours. The curve order is
/// reduced where possible so that cubics may be turned into quadratics, and quadratics maybe
/// turned into lines. It fills with the even-odd rule, as Skia's `Op` sets on what it writes.
///
/// Panics where Dart throws its `StateError`: Skia's `Op` gives no path for these two.
pub fn path_combine(operation: PathOperation, path1: &Path, path2: &Path) -> Path {
    let contours = path1
        .contours
        .combine(path1.fill_type, operation, &path2.contours, path2.fill_type)
        .unwrap_or_else(|_| {
            panic!(
                "Path.combine() failed.  This may be due an invalid path; in particular, check for NaN values."
            )
        });
    Path {
        contours,
        fill_type: PathFillType::EvenOdd,
    }
}

/// The full valo API for hosts and for recording that needs a type the
/// aliases do not cover (`Op`, `Context`, `Surface`).
pub use valo;

/// Quality levels for image sampling in [`ImageFilter`] and [`Shader`] objects that sample
/// images and for [`Canvas`] operations that render images.
///
/// When scaling up typically the quality is lowest at [`none`](Self::None), higher at
/// [`low`](Self::Low) and [`medium`](Self::Medium), and for very large scale factors (over 10x)
/// the highest at [`high`](Self::High).
///
/// When scaling down, [`medium`](Self::Medium) provides the best quality especially when scaling
/// an image to less than half its size or for animating the scale factor between such
/// reductions. Otherwise, [`low`](Self::Low) and [`high`](Self::High) provide similar effects for
/// reductions of between 50% and 100% but the image may lose detail and have dropouts below 50%.
///
/// To get high quality when scaling images up and down, or when the scale is unknown,
/// [`medium`](Self::Medium) is typically a good balanced choice.
///
/// See also:
///
///  * [`FragmentShader::set_image_sampler`](crate::FragmentShader::set_image_sampler).
///  * [`ImageFilter::shader`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterQuality {
    /// The fastest filtering method, albeit also the lowest quality.
    ///
    /// This value results in a "Nearest Neighbor" algorithm which just repeats or eliminates
    /// pixels as an image is scaled up or down.
    None,

    /// Better quality than [`none`](Self::None), faster than [`medium`](Self::Medium).
    ///
    /// This value results in a "Bilinear" algorithm which smoothly interpolates between pixels in
    /// an image.
    Low,

    /// The best all around filtering method that is only worse than [`high`](Self::High) at
    /// extremely large scale factors.
    ///
    /// This value improves upon the "Bilinear" algorithm specified by [`low`](Self::Low) by
    /// utilizing a Mipmap that pre-computes high quality lower resolutions of the image at half
    /// (and quarter and eighth, etc.) sizes and then blends between those to prevent loss of
    /// detail at small scale sizes.
    Medium,

    /// Best possible quality when scaling up images by scale factors larger than 5-10x.
    ///
    /// When images are scaled down, this can be worse than [`medium`](Self::Medium) for scales
    /// smaller than 0.5x, or when animating the scale factor.
    ///
    /// This option is also the slowest.
    ///
    /// This value results in a standard "Bicubic" algorithm which uses a 3rd order equation to
    /// smooth the abrupt transitions between pixels while preserving some of the sense of an edge
    /// and avoiding sharp peaks in the result.
    High,
}

impl FilterQuality {
    /// How valo samples an image at this quality: the engine's `ImageFilter::SamplingFromIndex`
    /// as Impeller's `ToSamplerDescriptor` turns it into a sampler, clamped at the edges as
    /// `ReusableFragmentShader::SetImageSampler` clamps them. Impeller samples
    /// [`High`](Self::High), the engine's cubic, as it samples [`Medium`](Self::Medium).
    pub fn to_valo(self) -> valo::Sampling {
        let (filter, mipmap) = match self {
            FilterQuality::None => (valo::Filter::Nearest, valo::MipmapMode::None),
            FilterQuality::Low => (valo::Filter::Linear, valo::MipmapMode::None),
            FilterQuality::Medium | FilterQuality::High => {
                (valo::Filter::Linear, valo::MipmapMode::Linear)
            }
        };
        valo::Sampling {
            filter,
            mipmap,
            tile_x: valo::TileMode::Clamp,
            tile_y: valo::TileMode::Clamp,
        }
    }
}

impl std::fmt::Display for FilterQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            FilterQuality::None => "none",
            FilterQuality::Low => "low",
            FilterQuality::Medium => "medium",
            FilterQuality::High => "high",
        };
        write!(f, "FilterQuality.{name}")
    }
}

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
    /// # use waft_embedder::ColorFilter;
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

    /// The input moved by a matrix; Dart's `_MatrixImageFilter`, made by
    /// [`ImageFilter::matrix`].
    Matrix {
        /// The matrix that moves the input.
        data: crate::Matrix4,
        /// The quality the moved input is sampled at.
        filter_quality: FilterQuality,
    },

    /// A [`FragmentShader`](crate::FragmentShader)'s program run over the input; Dart's
    /// `_FragmentShaderImageFilter`, made by [`ImageFilter::shader`].
    Shader {
        /// The shader's uniforms and images as they were when the filter was made, its first
        /// image left for the input, which is sampled at `filter_quality`.
        shader: valo::RuntimeShader,
        /// The quality the input is sampled at.
        filter_quality: FilterQuality,
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

    /// Creates an image filter that applies a matrix transformation.
    ///
    /// For example, applying a positive scale matrix (see `Matrix4.diagonal3`) when used with
    /// `BackdropFilter` would magnify the background image.
    ///
    /// The `filter_quality` argument sets the quality level the moved input is sampled at;
    /// Flutter's default is [`FilterQuality::Medium`].
    pub const fn matrix(matrix4: crate::Matrix4, filter_quality: FilterQuality) -> ImageFilter {
        ImageFilter::Matrix {
            data: matrix4,
            filter_quality,
        }
    }

    /// Composes the `inner` filter with `outer`, to combine their effects.
    ///
    /// Creates a single [`ImageFilter`] that when applied, has the same effect as
    /// subsequently applying `inner` and `outer`, i.e.,
    /// `result = outer(inner(source))`.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "a shader filter's images are the web's GPU objects, which stay on their \
                      thread there; a filter crosses threads with its picture elsewhere"
        )
    )]
    pub fn compose(outer: ImageFilter, inner: ImageFilter) -> ImageFilter {
        ImageFilter::Compose {
            outer: Arc::new(outer),
            inner: Arc::new(inner),
        }
    }

    /// Creates an image filter from a [`FragmentShader`](crate::FragmentShader).
    ///
    /// The fragment shader provided here has additional requirements to be used by the engine
    /// for filtering. The first uniform value must be a `vec2`, this will be set by the engine to
    /// the size of the bound texture. There must also be at least one image, the first of which
    /// will be set by the engine to contain the filter input.
    ///
    /// The `filter_quality` argument sets the quality level used to sample the filter input;
    /// Flutter's default is [`FilterQuality::None`].
    ///
    /// For example, the following is a valid fragment shader that can be used with this API.
    /// Note that the uniform names are not required to have any particular value.
    ///
    /// ```wgsl
    /// struct Uniforms { u_size: vec2<f32>, u_time: f32 }
    /// @group(2) @binding(0) var<uniform> u: Uniforms;
    /// @group(1) @binding(0) var u_texture_input: texture_2d<f32>;
    /// @group(1) @binding(1) var u_texture_input_sampler: sampler;
    ///
    /// fn shade(position: vec2<f32>) -> vec4<f32> {
    ///     let uv = position / u.u_size;
    ///     return textureSampleLevel(u_texture_input, u_texture_input_sampler, uv, 0.0) * u.u_time;
    /// }
    /// ```
    ///
    /// The uniforms and images are those `shader` has now; setting them afterwards changes the
    /// filter made next, not this one.
    ///
    /// # Panics
    ///
    /// Where the shader's first uniform is not a `vec2` or it has no image, as Dart throws a
    /// `StateError`.
    pub fn shader(shader: &crate::FragmentShader, filter_quality: FilterQuality) -> ImageFilter {
        let invalid_floats = shader.float_count() < 2;
        let invalid_sampler = !shader.validate_image_filter();
        if invalid_floats || invalid_sampler {
            let mut buffer = String::from(
                "ImageFilter.shader requires that the first uniform is a vec2 and at least one \
                 sampler uniform is present.\n",
            );
            if invalid_floats {
                buffer.push_str("The shader has fewer than two float uniforms.\n");
            }
            if invalid_sampler {
                buffer.push_str("The shader is missing a sampler uniform.\n");
            }
            panic!("{buffer}");
        }
        let shader = shader.as_image_filter(filter_quality.to_valo());
        if let Err(error) = valo::ImageFilter::runtime(shader.clone()) {
            panic!("ImageFilter.shader requires that the first uniform is a vec2: {error}");
        }
        ImageFilter::Shader {
            shader,
            filter_quality,
        }
    }

    /// Whether [`ImageFilter::shader`] is supported on the current backend: always, as valo
    /// runs every shader the way Impeller does.
    pub const fn is_shader_filter_supported() -> bool {
        true
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
            ImageFilter::Matrix {
                data,
                filter_quality,
            } => format!("matrix({:?}, {filter_quality})", data.to_flutter_array()),
            ImageFilter::Compose { outer, inner } => format!(
                "{} -> {}",
                inner.debug_short_description(),
                outer.debug_short_description()
            ),
            ImageFilter::Shader { .. } => String::from("shader"),
        }
    }
}

impl std::fmt::Display for ImageFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFilter::Blur { .. } | ImageFilter::Matrix { .. } => {
                write!(f, "ImageFilter.{}", self.debug_short_description())
            }
            ImageFilter::Color(filter) => Display::fmt(filter, f),
            ImageFilter::Compose { .. } => write!(
                f,
                "ImageFilter.compose(source -> {} -> result)",
                self.debug_short_description()
            ),
            ImageFilter::Shader {
                shader,
                filter_quality,
            } => write!(
                f,
                "ImageFilter.shader(Shader#{}, {filter_quality})",
                shader.effect.id()
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
            ImageFilter::Matrix {
                data,
                filter_quality,
            } => valo::ImageFilter::matrix(*data, filter_quality.to_valo()),
            ImageFilter::Compose { outer, inner } => {
                valo::ImageFilter::compose(outer.to_valo(), inner.to_valo())
            }
            ImageFilter::Shader { shader, .. } => valo::ImageFilter::Runtime(shader.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        ColorFilter, FilterQuality, ImageFilter, Path, PathBuilder, PathFillType, PathOperation,
        path_combine, valo,
    };
    use crate::{BlendMode, Color, Matrix4, Offset, Rect};

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
    fn a_matrix_filter_reaches_valo_with_its_sampling_and_prints_as_darts() {
        let scale = Matrix4::scale(2.0, 2.0);
        let filter = ImageFilter::matrix(scale, FilterQuality::Low);
        assert_eq!(
            filter.to_valo(),
            valo::ImageFilter::matrix(scale, FilterQuality::Low.to_valo())
        );
        assert_eq!(
            filter.to_string(),
            "ImageFilter.matrix([2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, \
             0.0, 0.0, 0.0, 1.0], FilterQuality.low)"
        );
    }

    #[test]
    fn combining_two_paths_cuts_one_from_the_other() {
        let mut host = PathBuilder::new();
        host.rect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0).into());
        let mut guest = PathBuilder::new();
        guest.rect(Rect::from_ltwh(50.0, -10.0, 100.0, 60.0).into());
        let notched = path_combine(
            PathOperation::Difference,
            &Path::new(host.build()),
            &Path::new(guest.build()),
        );
        assert_eq!(notched.fill_type, PathFillType::EvenOdd);
        let contains = |x: f64, y: f64| notched.contains(Offset::new(x, y));
        assert!(contains(25.0, 25.0));
        assert!(!contains(75.0, 25.0), "the guest is cut out");
        assert!(contains(75.0, 75.0));
        assert!(!contains(125.0, 25.0), "nothing of the guest is added");
    }

    #[test]
    fn a_combined_path_leaves_the_hole_of_a_guest_wholly_inside() {
        let mut host = PathBuilder::new();
        host.rect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0).into());
        let mut guest = PathBuilder::new();
        guest.rect(Rect::from_ltwh(25.0, 25.0, 50.0, 50.0).into());
        let holed = path_combine(
            PathOperation::Difference,
            &Path::new(host.build()),
            &Path::new(guest.build()),
        );
        assert!(holed.contains(Offset::new(10.0, 10.0)));
        assert!(
            !holed.contains(Offset::new(50.0, 50.0)),
            "the guest is a hole"
        );
    }

    #[test]
    fn combining_reads_each_path_with_its_own_fill_type() {
        let mut ring = PathBuilder::new();
        ring.rect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0).into());
        ring.rect(Rect::from_ltwh(25.0, 25.0, 50.0, 50.0).into());
        let mut even_odd = Path::new(ring.build());
        even_odd.fill_type = PathFillType::EvenOdd;
        let non_zero = Path::new(Arc::clone(even_odd.as_valo()));
        let mut point = PathBuilder::new();
        point.rect(Rect::from_ltwh(40.0, 40.0, 20.0, 20.0).into());
        let point = Path::new(point.build());

        let through_the_hole = path_combine(PathOperation::Intersect, &even_odd, &point);
        assert!(!through_the_hole.contains(Offset::new(50.0, 50.0)));
        let over_the_fill = path_combine(PathOperation::Intersect, &non_zero, &point);
        assert!(over_the_fill.contains(Offset::new(50.0, 50.0)));
    }

    #[test]
    fn a_path_keeps_its_fill_type_through_a_shift_and_a_transform() {
        let mut builder = PathBuilder::new();
        builder.rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0).into());
        let mut path = Path::new(builder.build());
        assert_eq!(path.fill_type, PathFillType::NonZero);
        path.fill_type = PathFillType::EvenOdd;
        let shifted = path.shift(Offset::new(5.0, 0.0));
        assert_eq!(shifted.fill_type, PathFillType::EvenOdd);
        assert!(shifted.contains(Offset::new(12.0, 5.0)));
        assert!(!shifted.contains(Offset::new(2.0, 5.0)));
        let scaled = path.transform(&Matrix4::scale(2.0, 2.0));
        assert_eq!(scaled.fill_type, PathFillType::EvenOdd);
        assert!(scaled.contains(Offset::new(15.0, 15.0)));
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
