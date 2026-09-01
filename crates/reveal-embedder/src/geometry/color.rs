//! Flutter counterpart: `engine/src/flutter/lib/ui/painting.dart` (`Color`, `ColorSpace`).

use std::fmt::{self, Debug};

use super::lerp::lerp_double_non_null;
use super::math::clamp_double;

fn scale_alpha(x: Color, factor: f64) -> Color {
    x.with_values(
        Some(clamp_double(x.a * factor, 0.0, 1.0)),
        None,
        None,
        None,
        None,
    )
}

fn wider_color_space(a: ColorSpace, b: ColorSpace) -> ColorSpace {
    if a == ColorSpace::DisplayP3 || b == ColorSpace::DisplayP3 {
        ColorSpace::DisplayP3
    } else {
        a
    }
}

/// An immutable color value in ARGB format.
///
/// Consider the light teal of the [Flutter logo](https://flutter.dev/brand). It
/// is fully opaque, with a red [`r`](Color::r) channel value of `0.2588` (or
/// `0x42` or `66` as an 8-bit value), a green [`g`](Color::g) channel value of
/// `0.6471` (or `0xA5` or `165` as an 8-bit value), and a blue [`b`](Color::b)
/// channel value of `0.9608` (or `0xF5` or `245` as an 8-bit value). In a common
/// [CSS hex color syntax](https://developer.mozilla.org/en-US/docs/Web/CSS/hex-color)
/// for RGB color values, it would be described as `#42A5F5`.
///
/// Here are some ways it could be constructed:
///
/// ```
/// # use reveal_embedder::{Color, ColorSpace};
/// let c1 = Color::from(1.0, 0.2588, 0.6471, 0.9608, ColorSpace::Srgb);
/// let c2 = Color::new(0xFF42A5F5);
/// let c3 = Color::from_argb(0xFF, 0x42, 0xA5, 0xF5);
/// let c4 = Color::from_argb(255, 66, 165, 245);
/// let c5 = Color::from_rgbo(66, 165, 245, 1.0);
/// ```
///
/// If you are having a problem with [`Color::new`] wherein it seems your color
/// is just not painting, check to make sure you are specifying the full 8
/// hexadecimal digits. If you only specify six, then the leading two digits are
/// assumed to be zero, which means fully-transparent:
///
/// ```
/// # use reveal_embedder::{Color, ColorSpace};
/// let c1 = Color::new(0xFFFFFF); // fully transparent white (invisible)
/// let c2 = Color::new(0xFFFFFFFF); // fully opaque white (visible)
///
/// // Or use double-based channel values:
/// let c3 = Color::from(1.0, 1.0, 1.0, 1.0, ColorSpace::Srgb);
/// ```
///
/// [`Color`]'s color components are stored as floating-point values. Care should
/// be taken if one does not want the literal equality provided by [`PartialEq`].
/// To test equality inside of Flutter tests consider using [`isSameColorAs`][].
///
/// See also:
///
///  * [Colors](https://api.flutter.dev/flutter/material/Colors-class.html),
///    which defines the colors found in the Material Design specification.
///  * [`isSameColorAs`][],
///    a Matcher to handle floating-point deltas when checking [`Color`] equality.
///
/// [`isSameColorAs`]: https://api.flutter.dev/flutter/flutter_test/isSameColorAs.html
#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    /// The alpha channel of this color.
    pub a: f64,
    /// The red channel of this color.
    pub r: f64,
    /// The green channel of this color.
    pub g: f64,
    /// The blue channel of this color.
    pub b: f64,
    /// The color space of this color.
    pub color_space: ColorSpace,
}

impl Color {
    /// Construct a [`ColorSpace::Srgb`] color from the lower 32 bits of an integer.
    ///
    /// The bits are interpreted as follows:
    ///
    /// * Bits 24-31 are the alpha value.
    /// * Bits 16-23 are the red value.
    /// * Bits 8-15 are the green value.
    /// * Bits 0-7 are the blue value.
    ///
    /// In other words, if AA is the alpha value in hex, RR the red value in hex,
    /// GG the green value in hex, and BB the blue value in hex, a color can be
    /// expressed as `Color::new(0xAARRGGBB)`.
    ///
    /// For example, to get a fully opaque orange, you would use
    /// `Color::new(0xFFFF9000)` (`FF` for the alpha, `FF` for the red, `90` for
    /// the green, and `00` for the blue).
    ///
    /// > **Note**
    /// > Each color is stored as floating-point color components, where the final
    /// > value of each component is approximated by storing `c / 255`, where `c`
    /// > is one of the four components (alpha, red, green, blue).
    pub const fn new(value: u32) -> Color {
        Self::from_argbc(
            (value >> 24) as i32,
            (value >> 16) as i32,
            (value >> 8) as i32,
            value as i32,
            ColorSpace::Srgb,
        )
    }

    /// Construct a color with floating-point color components.
    ///
    /// Color components allows arbitrary bit depths for color components to be
    /// supported. The values are interpreted relative to the [`ColorSpace`]
    /// argument.
    ///
    /// ## Example
    ///
    /// ```
    /// # use reveal_embedder::{Color, ColorSpace};
    /// // Fully opaque maximum red color
    /// let c1 = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::Srgb);
    ///
    /// // Partially transparent moderately blue and green color
    /// let c2 = Color::from(0.5, 0.0, 0.5, 0.5, ColorSpace::Srgb);
    ///
    /// // Fully transparent color
    /// let c3 = Color::from(0.0, 0.0, 0.0, 0.0, ColorSpace::Srgb);
    /// ```
    pub const fn from(
        alpha: f64,
        red: f64,
        green: f64,
        blue: f64,
        color_space: ColorSpace,
    ) -> Color {
        Color {
            a: alpha,
            r: red,
            g: green,
            b: blue,
            color_space,
        }
    }

    /// Construct an sRGB color from the lower 8 bits of four integers.
    ///
    /// * `a` is the alpha value, with 0 being transparent and 255 being fully
    ///   opaque.
    /// * `r` is [red](Color::red), from 0 to 255.
    /// * `g` is [green](Color::green), from 0 to 255.
    /// * `b` is [blue](Color::blue), from 0 to 255.
    ///
    /// Out of range values are brought into range using modulo 255.
    ///
    /// See also [`from_rgbo`](Color::from_rgbo), which takes the alpha value as
    /// a floating point value.
    ///
    /// > **Note**
    /// > Each color is stored as floating-point color components, where the final
    /// > value of each component is approximated by storing `c / 255`, where `c`
    /// > is one of the four components (alpha, red, green, blue).
    pub const fn from_argb(a: i32, r: i32, g: i32, b: i32) -> Color {
        Self::from_argbc(a, r, g, b, ColorSpace::Srgb)
    }

    const fn from_argbc(
        alpha: i32,
        red: i32,
        green: i32,
        blue: i32,
        color_space: ColorSpace,
    ) -> Color {
        Self::from_rgboc(
            red,
            green,
            blue,
            ((alpha & 0xff) as f64) / 255.0,
            color_space,
        )
    }

    /// Create an sRGB color from red, green, blue, and opacity, similar to
    /// `rgba()` in CSS.
    ///
    /// * `r` is [red](Color::red), from 0 to 255.
    /// * `g` is [green](Color::green), from 0 to 255.
    /// * `b` is [blue](Color::blue), from 0 to 255.
    /// * `opacity` is alpha channel of this color as a double, with 0.0 being
    ///   transparent and 1.0 being fully opaque.
    ///
    /// Out of range values are brought into range using modulo 255.
    ///
    /// See also [`from_argb`](Color::from_argb), which takes the opacity as an
    /// integer value.
    ///
    /// > **Note**
    /// > Each color is stored as floating-point color components, where the final
    /// > value of each component is approximated by storing `c / 255`, where `c`
    /// > is one of the four components (alpha, red, green, blue).
    pub const fn from_rgbo(r: i32, g: i32, b: i32, opacity: f64) -> Color {
        Self::from_rgboc(r, g, b, opacity, ColorSpace::Srgb)
    }

    const fn from_rgboc(r: i32, g: i32, b: i32, opacity: f64, color_space: ColorSpace) -> Color {
        Color {
            a: opacity,
            r: ((r & 0xff) as f64) / 255.0,
            g: ((g & 0xff) as f64) / 255.0,
            b: ((b & 0xff) as f64) / 255.0,
            color_space,
        }
    }

    fn float_to_int8(x: f64) -> i32 {
        (x * 255.0).round().clamp(0.0, 255.0) as i32
    }

    /// A 32 bit value representing this color.
    ///
    /// This getter is a *stub*. It is recommended instead to use the explicit
    /// [`to_argb32`](Color::to_argb32) method.
    #[deprecated(
        note = "Use component accessors like .r or .g, or to_argb32 for an explicit conversion"
    )]
    pub fn value(&self) -> u32 {
        self.to_argb32()
    }

    /// Returns a 32-bit value representing this color.
    ///
    /// The returned value is compatible with the default constructor
    /// ([`Color::new`]) but does *not* guarantee to result in the same color due
    /// to [imprecisions in numeric conversions](https://en.wikipedia.org/wiki/Floating-point_error_mitigation).
    ///
    /// Unlike accessing the floating point equivalent channels individually
    /// ([`a`](Color::a), [`r`](Color::r), [`g`](Color::g), [`b`](Color::b)),
    /// this method is intentionally *lossy*, and scales each channel using
    /// `(channel * 255.0).round().clamp(0, 255)`.
    ///
    /// While useful for storing a 32-bit integer value, prefer accessing the
    /// individual channels (and storing the double equivalent) where higher
    /// precision is required.
    ///
    /// The bits are assigned as follows:
    ///
    /// * Bits 24-31 represents the [`a`](Color::a) channel as an 8-bit unsigned integer.
    /// * Bits 16-23 represents the [`r`](Color::r) channel as an 8-bit unsigned integer.
    /// * Bits 8-15 represents the [`g`](Color::g) channel as an 8-bit unsigned integer.
    /// * Bits 0-7 represents the [`b`](Color::b) channel as an 8-bit unsigned integer.
    ///
    /// > **Warning**
    /// > The value returned by this getter implicitly converts floating-point
    /// > component values (such as `0.5`) into their 8-bit equivalent by using
    /// > the [`to_argb32`](Color::to_argb32) method; the returned value is not
    /// > guaranteed to be stable across different platforms or executions due to
    /// > the complexity of floating-point math.
    pub fn to_argb32(&self) -> u32 {
        #[allow(clippy::identity_op)]
        {
            ((Self::float_to_int8(self.a) as u32) << 24)
                | ((Self::float_to_int8(self.r) as u32) << 16)
                | ((Self::float_to_int8(self.g) as u32) << 8)
                | ((Self::float_to_int8(self.b) as u32) << 0)
        }
    }

    /// The alpha channel of this color in an 8 bit value.
    ///
    /// A value of 0 means this color is fully transparent. A value of 255 means
    /// this color is fully opaque.
    #[deprecated(note = "Use (*.a * 255.0).round().clamp(0, 255)")]
    #[allow(deprecated)]
    pub fn alpha(&self) -> i32 {
        ((0xff000000 & self.value()) >> 24) as i32
    }

    /// The alpha channel of this color as a double.
    ///
    /// A value of 0.0 means this color is fully transparent. A value of 1.0 means
    /// this color is fully opaque.
    #[deprecated(note = "Use .a.")]
    #[allow(deprecated)]
    pub fn opacity(&self) -> f64 {
        (self.alpha() as f64) / (0xFF as f64)
    }

    /// The red channel of this color in an 8 bit value.
    #[deprecated(note = "Use (*.r * 255.0).round().clamp(0, 255)")]
    #[allow(deprecated)]
    pub fn red(&self) -> i32 {
        ((0x00ff0000 & self.value()) >> 16) as i32
    }

    /// The green channel of this color in an 8 bit value.
    #[deprecated(note = "Use (*.g * 255.0).round().clamp(0, 255)")]
    #[allow(deprecated)]
    pub fn green(&self) -> i32 {
        ((0x0000ff00 & self.value()) >> 8) as i32
    }

    /// The blue channel of this color in an 8 bit value.
    #[deprecated(note = "Use (*.b * 255.0).round().clamp(0, 255)")]
    #[allow(deprecated)]
    pub fn blue(&self) -> i32 {
        #[allow(clippy::identity_op)]
        {
            ((0x000000ff & self.value()) >> 0) as i32
        }
    }

    /// Returns a new color with the provided components updated.
    ///
    /// Each component (`alpha`, `red`, `green`, `blue`) represents a
    /// floating-point value; see [`Color::from`] for details and examples.
    ///
    /// If `color_space` is provided, and is different than the current color
    /// space, the component values are updated before transforming them to the
    /// provided [`ColorSpace`].
    ///
    /// Example:
    /// ```
    /// # use reveal_embedder::Color;
    /// /// Create a color with 50% opacity.
    /// fn make_transparent(color: Color) -> Color {
    ///     color.with_values(Some(0.5), None, None, None, None)
    /// }
    /// ```
    pub fn with_values(
        &self,
        alpha: Option<f64>,
        red: Option<f64>,
        green: Option<f64>,
        blue: Option<f64>,
        color_space: Option<ColorSpace>,
    ) -> Color {
        let mut updated_components = None;
        if alpha.is_some() || red.is_some() || green.is_some() || blue.is_some() {
            updated_components = Some(Color::from(
                alpha.unwrap_or(self.a),
                red.unwrap_or(self.r),
                green.unwrap_or(self.g),
                blue.unwrap_or(self.b),
                self.color_space,
            ));
        }
        if let Some(color_space) = color_space
            && color_space != self.color_space
        {
            let transform = get_color_transform(self.color_space, color_space);
            transform.transform(updated_components.unwrap_or(*self), color_space)
        } else {
            updated_components.unwrap_or(*self)
        }
    }

    /// Returns a new color that matches this color with the alpha channel
    /// replaced with `a` (which ranges from 0 to 255).
    ///
    /// Out of range values will have unexpected effects.
    #[allow(deprecated)]
    pub fn with_alpha(&self, a: i32) -> Color {
        Color::from_argb(a, self.red(), self.green(), self.blue())
    }

    /// Returns a new color that matches this color with the alpha channel
    /// replaced with the given `opacity` (which ranges from 0.0 to 1.0).
    ///
    /// Out of range values will have unexpected effects.
    #[deprecated(note = "Use .with_values() to avoid precision loss.")]
    pub fn with_opacity(&self, opacity: f64) -> Color {
        debug_assert!((0.0..=1.0).contains(&opacity));
        self.with_alpha((255.0 * opacity).round() as i32)
    }

    /// Returns a new color that matches this color with the red channel replaced
    /// with `r` (which ranges from 0 to 255).
    ///
    /// Out of range values will have unexpected effects.
    #[allow(deprecated)]
    pub fn with_red(&self, r: i32) -> Color {
        Color::from_argb(self.alpha(), r, self.green(), self.blue())
    }

    /// Returns a new color that matches this color with the green channel
    /// replaced with `g` (which ranges from 0 to 255).
    ///
    /// Out of range values will have unexpected effects.
    #[allow(deprecated)]
    pub fn with_green(&self, g: i32) -> Color {
        Color::from_argb(self.alpha(), self.red(), g, self.blue())
    }

    /// Returns a new color that matches this color with the blue channel replaced
    /// with `b` (which ranges from 0 to 255).
    ///
    /// Out of range values will have unexpected effects.
    #[allow(deprecated)]
    pub fn with_blue(&self, b: i32) -> Color {
        Color::from_argb(self.alpha(), self.red(), self.green(), b)
    }

    // See <https://www.w3.org/TR/WCAG20/#relativeluminancedef>
    fn linearize_color_component(component: f64) -> f64 {
        if component <= 0.03928 {
            return component / 12.92;
        }
        ((component + 0.055) / 1.055).powf(2.4)
    }

    /// Returns a brightness value between 0 for darkest and 1 for lightest.
    ///
    /// Represents the relative luminance of the color. This value is computationally
    /// expensive to calculate.
    ///
    /// See <https://en.wikipedia.org/wiki/Relative_luminance>.
    pub fn compute_luminance(&self) -> f64 {
        debug_assert!(self.color_space != ColorSpace::ExtendedSrgb);
        // See <https://www.w3.org/TR/WCAG20/#relativeluminancedef>
        let r = Self::linearize_color_component(self.r);
        let g = Self::linearize_color_component(self.g);
        let b = Self::linearize_color_component(self.b);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Linearly interpolate between two colors.
    ///
    /// This is intended to be fast but as a result may be ugly. Consider
    /// `HSVColor` or writing custom logic for interpolating colors.
    ///
    /// If either color is null, this function linearly interpolates from a
    /// transparent instance of the other color. This is usually preferable to
    /// interpolating from `material.Colors.transparent` (`Color::new(0x00000000)`),
    /// which is specifically transparent *black*.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can
    /// easily be generated by curves such as `Curves.elasticInOut`). Each channel
    /// will be clamped to the range 0 to 255.
    ///
    /// Values for `t` are usually obtained from an `Animation<f64>`, such as
    /// an `AnimationController`.
    ///
    /// If the two colors are in different color spaces, both are converted to
    /// the wider gamut color space before interpolating. The result will be in
    /// the wider gamut color space. For example, interpolating between an sRGB
    /// color and a Display P3 color will produce a Display P3 result.
    pub fn lerp(x: Option<Color>, y: Option<Color>, t: f64) -> Option<Color> {
        debug_assert!(x.is_none_or(|c| c.color_space != ColorSpace::ExtendedSrgb));
        debug_assert!(y.is_none_or(|c| c.color_space != ColorSpace::ExtendedSrgb));
        match (x, y) {
            (None, None) => None,
            (Some(x), None) => Some(scale_alpha(x, 1.0 - t)),
            (None, Some(y)) => Some(scale_alpha(y, t)),
            (Some(x), Some(y)) => {
                let a;
                let b;
                let result_color_space;
                if x.color_space == y.color_space {
                    a = x;
                    b = y;
                    result_color_space = x.color_space;
                } else {
                    result_color_space = wider_color_space(x.color_space, y.color_space);
                    a = x.with_values(None, None, None, None, Some(result_color_space));
                    b = y.with_values(None, None, None, None, Some(result_color_space));
                }
                Some(Color::from(
                    clamp_double(lerp_double_non_null(a.a, b.a, t), 0.0, 1.0),
                    clamp_double(lerp_double_non_null(a.r, b.r, t), 0.0, 1.0),
                    clamp_double(lerp_double_non_null(a.g, b.g, t), 0.0, 1.0),
                    clamp_double(lerp_double_non_null(a.b, b.b, t), 0.0, 1.0),
                    result_color_space,
                ))
            }
        }
    }

    /// Combine the foreground color as a transparent color over top
    /// of a background color, and return the resulting combined color.
    ///
    /// This uses standard alpha blending ("SRC over DST") rules to produce a
    /// blended color from two colors. This can be used as a performance
    /// enhancement when trying to avoid needless alpha blending compositing
    /// operations for two things that are solid colors with the same shape, but
    /// overlay each other: instead, just paint one with the combined color.
    pub fn alpha_blend(foreground: Color, background: Color) -> Color {
        debug_assert!(foreground.color_space == background.color_space);
        debug_assert!(foreground.color_space != ColorSpace::ExtendedSrgb);
        let alpha = foreground.a;
        if alpha == 0.0 {
            // Foreground completely transparent.
            return background;
        }
        let inv_alpha = 1.0 - alpha;
        let mut back_alpha = background.a;
        if back_alpha == 1.0 {
            // Opaque background case
            Color::from(
                1.0,
                alpha * foreground.r + inv_alpha * background.r,
                alpha * foreground.g + inv_alpha * background.g,
                alpha * foreground.b + inv_alpha * background.b,
                foreground.color_space,
            )
        } else {
            // General case
            back_alpha *= inv_alpha;
            let out_alpha = alpha + back_alpha;
            debug_assert!(out_alpha != 0.0);
            Color::from(
                out_alpha,
                (foreground.r * alpha + background.r * back_alpha) / out_alpha,
                (foreground.g * alpha + background.g * back_alpha) / out_alpha,
                (foreground.b * alpha + background.b * back_alpha) / out_alpha,
                foreground.color_space,
            )
        }
    }

    /// Returns an alpha value representative of the provided `opacity` value.
    ///
    /// The `opacity` value may not be null.
    pub fn get_alpha_from_opacity(opacity: f64) -> i32 {
        (clamp_double(opacity, 0.0, 1.0) * 255.0).round() as i32
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Color(alpha: {:.4}, red: {:.4}, green: {:.4}, blue: {:.4}, colorSpace: {:?})",
            self.a, self.r, self.g, self.b, self.color_space
        )
    }
}

/// The color space describes the colors that are available to an `Image`.
///
/// This value can help decide which `ImageByteFormat` to use with
/// `Image.toByteData`. Images that are in the [`ColorSpace::ExtendedSrgb`]
/// color space should use something like `ImageByteFormat.rawExtendedRgba128`
/// so that colors outside of the sRGB gamut aren't lost.
///
/// This is also the result of `Image.colorSpace`.
///
/// See also: https://en.wikipedia.org/wiki/Color_space
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// The sRGB color space.
    ///
    /// You may know this as the standard color space for the web or the color
    /// space of non-wide-gamut Flutter apps.
    ///
    /// See also: https://en.wikipedia.org/wiki/SRGB
    Srgb,

    /// A color space that is backwards compatible with sRGB but can represent
    /// colors outside of that gamut with values outside of \[0..1\]. In order to
    /// see the extended values an `ImageByteFormat` like
    /// `ImageByteFormat.rawExtendedRgba128` must be used.
    ExtendedSrgb,

    /// The Display P3 color space.
    ///
    /// This is a wide gamut color space that has broad hardware support. It's
    /// supported in cases like using Impeller on iOS. When used on a platform
    /// that doesn't support Display P3, the colors will be clamped to sRGB.
    ///
    /// See also: https://en.wikipedia.org/wiki/DCI-P3
    DisplayP3,
}

impl Debug for ColorSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorSpace::Srgb => write!(f, "ColorSpace.sRGB"),
            ColorSpace::ExtendedSrgb => write!(f, "ColorSpace.extendedSRGB"),
            ColorSpace::DisplayP3 => write!(f, "ColorSpace.displayP3"),
        }
    }
}

enum ColorTransform {
    Identity,
    Clamp,
    P3ToSrgb,
    SrgbToP3,
}

impl ColorTransform {
    fn transform(self, color: Color, result_color_space: ColorSpace) -> Color {
        match self {
            ColorTransform::Identity => color,
            ColorTransform::Clamp => Color::from(
                clamp_double(color.a, 0.0, 1.0),
                clamp_double(color.r, 0.0, 1.0),
                clamp_double(color.g, 0.0, 1.0),
                clamp_double(color.b, 0.0, 1.0),
                result_color_space,
            ),
            ColorTransform::P3ToSrgb => {
                let r_lin = srgb_eotf_extended(color.r);
                let g_lin = srgb_eotf_extended(color.g);
                let b_lin = srgb_eotf_extended(color.b);

                let r_out = K_P3_TO_SRGB_LINEAR[0] * r_lin
                    + K_P3_TO_SRGB_LINEAR[1] * g_lin
                    + K_P3_TO_SRGB_LINEAR[2] * b_lin;
                let g_out = K_P3_TO_SRGB_LINEAR[3] * r_lin
                    + K_P3_TO_SRGB_LINEAR[4] * g_lin
                    + K_P3_TO_SRGB_LINEAR[5] * b_lin;
                let b_out = K_P3_TO_SRGB_LINEAR[6] * r_lin
                    + K_P3_TO_SRGB_LINEAR[7] * g_lin
                    + K_P3_TO_SRGB_LINEAR[8] * b_lin;

                Color::from(
                    color.a,
                    srgb_oetf_extended(r_out),
                    srgb_oetf_extended(g_out),
                    srgb_oetf_extended(b_out),
                    result_color_space,
                )
            }
            ColorTransform::SrgbToP3 => {
                let r_lin = srgb_eotf_extended(color.r);
                let g_lin = srgb_eotf_extended(color.g);
                let b_lin = srgb_eotf_extended(color.b);

                let r_out = K_SRGB_TO_P3_LINEAR[0] * r_lin
                    + K_SRGB_TO_P3_LINEAR[1] * g_lin
                    + K_SRGB_TO_P3_LINEAR[2] * b_lin;
                let g_out = K_SRGB_TO_P3_LINEAR[3] * r_lin
                    + K_SRGB_TO_P3_LINEAR[4] * g_lin
                    + K_SRGB_TO_P3_LINEAR[5] * b_lin;
                let b_out = K_SRGB_TO_P3_LINEAR[6] * r_lin
                    + K_SRGB_TO_P3_LINEAR[7] * g_lin
                    + K_SRGB_TO_P3_LINEAR[8] * b_lin;

                Color::from(
                    color.a,
                    srgb_oetf_extended(r_out),
                    srgb_oetf_extended(g_out),
                    srgb_oetf_extended(b_out),
                    result_color_space,
                )
            }
        }
    }
}

// sRGB standard constants for transfer functions.
// See https://en.wikipedia.org/wiki/SRGB.
const K_SRGB_GAMMA: f64 = 2.4;
const K_SRGB_LINEAR_THRESHOLD: f64 = 0.04045;
const K_SRGB_LINEAR_SLOPE: f64 = 12.92;
const K_SRGB_ENCODED_OFFSET: f64 = 0.055;
const K_SRGB_ENCODED_DIVISOR: f64 = 1.055;
const K_SRGB_LINEAR_TO_ENCODED_THRESHOLD: f64 = 0.0031308;

/// sRGB electro-optical transfer function (gamma decode to linear).
fn srgb_eotf(v: f64) -> f64 {
    if v <= K_SRGB_LINEAR_THRESHOLD {
        v / K_SRGB_LINEAR_SLOPE
    } else {
        ((v + K_SRGB_ENCODED_OFFSET) / K_SRGB_ENCODED_DIVISOR).powf(K_SRGB_GAMMA)
    }
}

/// sRGB opto-electronic transfer function (linear to gamma encode).
fn srgb_oetf(v: f64) -> f64 {
    if v <= K_SRGB_LINEAR_TO_ENCODED_THRESHOLD {
        v * K_SRGB_LINEAR_SLOPE
    } else {
        K_SRGB_ENCODED_DIVISOR * v.powf(1.0 / K_SRGB_GAMMA) - K_SRGB_ENCODED_OFFSET
    }
}

/// Extended versions that handle negative values by mirroring.
fn srgb_eotf_extended(v: f64) -> f64 {
    if v < 0.0 {
        -srgb_eotf(-v)
    } else {
        srgb_eotf(v)
    }
}

fn srgb_oetf_extended(v: f64) -> f64 {
    if v < 0.0 {
        -srgb_oetf(-v)
    } else {
        srgb_oetf(v)
    }
}

/// Display P3 to sRGB 3x3 matrix in linear space.
/// M = sRGB_XYZ_to_RGB * P3_RGB_to_XYZ
const K_P3_TO_SRGB_LINEAR: [f64; 9] = [
    1.2249401, -0.2249402, 0.0, -0.0420569, 1.0420571, 0.0, -0.0196376, -0.0786507, 1.0982884,
];

/// sRGB to Display P3 3x3 matrix in linear space (inverse of [`K_P3_TO_SRGB_LINEAR`]).
const K_SRGB_TO_P3_LINEAR: [f64; 9] = [
    0.8224622, 0.1775380, 0.0, 0.0331942, 0.9668058, 0.0, 0.0170806, 0.0723974, 0.9105220,
];

fn get_color_transform(source: ColorSpace, destination: ColorSpace) -> ColorTransform {
    match source {
        ColorSpace::Srgb => match destination {
            ColorSpace::Srgb => ColorTransform::Identity,
            ColorSpace::ExtendedSrgb => ColorTransform::Identity,
            ColorSpace::DisplayP3 => ColorTransform::SrgbToP3,
        },
        ColorSpace::ExtendedSrgb => match destination {
            ColorSpace::Srgb => ColorTransform::Clamp,
            ColorSpace::ExtendedSrgb => ColorTransform::Identity,
            ColorSpace::DisplayP3 => ColorTransform::Clamp,
        },
        ColorSpace::DisplayP3 => match destination {
            ColorSpace::Srgb => ColorTransform::Clamp,
            ColorSpace::ExtendedSrgb => ColorTransform::P3ToSrgb,
            ColorSpace::DisplayP3 => ColorTransform::Identity,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color_matches(actual: Color, expected: Color, threshold: f64) -> bool {
        actual.color_space == expected.color_space
            && (actual.a - expected.a).abs() <= threshold
            && (actual.r - expected.r).abs() <= threshold
            && (actual.g - expected.g).abs() <= threshold
            && (actual.b - expected.b).abs() <= threshold
    }

    // engine color_test.dart plus the requested Color(0xFF42A5F5) pin.
    #[test]
    fn packed_constructor_from_argb_and_from_agree() {
        let packed = Color::new(0xFF42A5F5);
        let from_argb = Color::from_argb(0xFF, 0x42, 0xA5, 0xF5);
        let from = Color::from(
            1.0,
            0x42 as f64 / 255.0,
            0xA5 as f64 / 255.0,
            0xF5 as f64 / 255.0,
            ColorSpace::Srgb,
        );
        let from_rgbo = Color::from_rgbo(0x42, 0xA5, 0xF5, 1.0);

        assert_eq!(packed, from_argb);
        assert_eq!(packed, from);
        assert_eq!(packed, from_rgbo);
        assert_eq!(packed.to_argb32(), 0xFF42A5F5);
        assert_eq!(packed.a, 1.0);
        assert_eq!(packed.color_space, ColorSpace::Srgb);
    }

    #[test]
    #[allow(deprecated)]
    fn color_accessors_should_work() {
        let foo = Color::new(0x12345678);
        assert_eq!(foo.alpha(), 0x12);
        assert_eq!(foo.red(), 0x34);
        assert_eq!(foo.green(), 0x56);
        assert_eq!(foo.blue(), 0x78);
    }

    #[test]
    fn two_colors_are_equal_when_components_and_space_match() {
        assert_eq!(Color::new(0x12345678), Color::new(0x12345678));
        assert_ne!(Color::new(0x12345678), Color::new(0x87654321));
        let srgb = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::Srgb);
        let p3 = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::DisplayP3);
        assert_ne!(srgb, p3);
    }

    #[test]
    fn lerp_walks_the_line_and_clamps_extrapolation() {
        assert_eq!(
            Color::lerp(
                Some(Color::new(0x00000000)),
                Some(Color::new(0xFFFFFFFF)),
                0.0
            ),
            Some(Color::new(0x00000000))
        );
        assert!(color_matches(
            Color::lerp(
                Some(Color::new(0x00000000)),
                Some(Color::new(0xFFFFFFFF)),
                0.5
            )
            .unwrap(),
            Color::new(0x7F7F7F7F),
            1.0 / 255.0
        ));
        assert_eq!(
            Color::lerp(
                Some(Color::new(0x00000000)),
                Some(Color::new(0xFFFFFFFF)),
                1.0
            ),
            Some(Color::new(0xFFFFFFFF))
        );
        assert_eq!(
            Color::lerp(
                Some(Color::new(0x00000000)),
                Some(Color::new(0xFFFFFFFF)),
                -0.1
            ),
            Some(Color::new(0x00000000))
        );
        assert_eq!(
            Color::lerp(
                Some(Color::new(0x00000000)),
                Some(Color::new(0xFFFFFFFF)),
                1.1
            ),
            Some(Color::new(0xFFFFFFFF))
        );

        // Prevent regression: https://github.com/flutter/flutter/issues/67423
        assert_eq!(
            Color::lerp(
                Some(Color::new(0xFFFFFFFF)),
                Some(Color::new(0xFFFFFFFF)),
                0.04
            ),
            Some(Color::new(0xFFFFFFFF))
        );

        assert_eq!(Color::lerp(None, None, 0.5), None);
        assert_eq!(
            Color::lerp(Some(Color::new(0xFFFFFFFF)), None, 0.0),
            Some(Color::new(0xFFFFFFFF))
        );
    }

    #[test]
    fn lerp_same_color_spaces() {
        assert_eq!(
            Color::lerp(
                Some(Color::from(1.0, 0.0, 0.0, 0.0, ColorSpace::DisplayP3)),
                Some(Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::DisplayP3)),
                0.2,
            ),
            Some(Color::from(1.0, 0.2, 0.0, 0.0, ColorSpace::DisplayP3))
        );
    }

    #[test]
    fn lerp_mixed_color_spaces_produces_display_p3() {
        let result = Color::lerp(
            Some(Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::DisplayP3)),
            Some(Color::from(1.0, 0.0, 0.0, 1.0, ColorSpace::Srgb)),
            0.0,
        )
        .unwrap();
        assert_eq!(result.color_space, ColorSpace::DisplayP3);
    }

    #[test]
    fn lerp_mixed_color_spaces_srgb_x_and_display_p3_y_at_t_0() {
        let result = Color::lerp(
            Some(Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::Srgb)),
            Some(Color::from(1.0, 0.0, 1.0, 0.0, ColorSpace::DisplayP3)),
            0.0,
        )
        .unwrap();
        assert_eq!(result.color_space, ColorSpace::DisplayP3);
        let expected_p3 = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::Srgb).with_values(
            None,
            None,
            None,
            None,
            Some(ColorSpace::DisplayP3),
        );
        assert!(color_matches(result, expected_p3, 1.0 / 255.0));
    }

    #[test]
    fn lerp_mixed_color_spaces_at_midpoint() {
        let srgb_red = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::Srgb);
        let p3_green = Color::from(1.0, 0.0, 1.0, 0.0, ColorSpace::DisplayP3);
        let result = Color::lerp(Some(srgb_red), Some(p3_green), 0.5).unwrap();
        assert_eq!(result.color_space, ColorSpace::DisplayP3);
        let srgb_red_as_p3 =
            srgb_red.with_values(None, None, None, None, Some(ColorSpace::DisplayP3));
        let expected = Color::from(
            (srgb_red_as_p3.a + p3_green.a) / 2.0,
            (srgb_red_as_p3.r + p3_green.r) / 2.0,
            (srgb_red_as_p3.g + p3_green.g) / 2.0,
            (srgb_red_as_p3.b + p3_green.b) / 2.0,
            ColorSpace::DisplayP3,
        );
        assert!(color_matches(result, expected, 1e-4));
    }

    #[test]
    fn alpha_blend_matches_src_over_dst() {
        assert_eq!(
            Color::alpha_blend(Color::new(0x00000000), Color::new(0x00000000)),
            Color::new(0x00000000)
        );
        assert_eq!(
            Color::alpha_blend(Color::new(0x00000000), Color::new(0xFFFFFFFF)),
            Color::new(0xFFFFFFFF)
        );
        assert_eq!(
            Color::alpha_blend(Color::new(0xFFFFFFFF), Color::new(0x00000000)),
            Color::new(0xFFFFFFFF)
        );
        assert_eq!(
            Color::alpha_blend(Color::new(0xFFFFFFFF), Color::new(0xFFFFFFFF)),
            Color::new(0xFFFFFFFF)
        );
        assert_eq!(
            Color::alpha_blend(Color::new(0x80FFFFFF), Color::new(0xFF000000)),
            Color::new(0xFF808080)
        );
        assert!(color_matches(
            Color::alpha_blend(Color::new(0x80808080), Color::new(0xFFFFFFFF)),
            Color::new(0xFFBFBFBF),
            1.0 / 255.0
        ));
        assert!(color_matches(
            Color::alpha_blend(Color::new(0x80808080), Color::new(0xFF000000)),
            Color::new(0xFF404040),
            1.0 / 255.0
        ));
        assert_eq!(
            Color::alpha_blend(
                Color::from(0.5, 1.0, 1.0, 1.0, ColorSpace::DisplayP3),
                Color::from(1.0, 0.0, 0.0, 0.0, ColorSpace::DisplayP3),
            ),
            Color::from(1.0, 0.5, 0.5, 0.5, ColorSpace::DisplayP3)
        );
    }

    #[test]
    fn compute_gray_luminance() {
        // Each color component is at 20%.
        let light_gray = Color::new(0xFF333333);
        // Relative luminance's formula is just the linearized color value for gray.
        // ((0.2 + 0.055) / 1.055) ^ 2.4.
        assert_eq!(light_gray.compute_luminance(), 0.033104766570885055);
    }

    #[test]
    fn compute_color_luminance() {
        let bright_red = Color::new(0xFFFF3B30);
        // 0.2126 * ((1.0 + 0.055) / 1.055) ^ 2.4 +
        // 0.7152 * ((0.23137254902 +0.055) / 1.055) ^ 2.4 +
        // 0.0722 * ((0.18823529411 + 0.055) / 1.055) ^ 2.4
        assert_eq!(bright_red.compute_luminance(), 0.24601329637099723);
    }

    #[test]
    #[allow(deprecated)]
    fn from_and_accessors() {
        let color = Color::from(0.1, 0.2, 0.3, 0.4, ColorSpace::Srgb);
        assert_eq!(color.a, 0.1);
        assert_eq!(color.r, 0.2);
        assert_eq!(color.g, 0.3);
        assert_eq!(color.b, 0.4);
        assert_eq!(color.color_space, ColorSpace::Srgb);

        assert_eq!(color.alpha(), 26);
        assert_eq!(color.red(), 51);
        assert_eq!(color.green(), 77);
        assert_eq!(color.blue(), 102);

        assert_eq!(color.value(), 0x1a334d66);
        assert_eq!(color.to_argb32(), 0x1a334d66);
    }

    #[test]
    #[allow(deprecated)]
    fn from_argb_and_accessors() {
        let color = Color::from_argb(10, 20, 35, 47);
        assert_eq!(color.alpha(), 10);
        assert_eq!(color.red(), 20);
        assert_eq!(color.green(), 35);
        assert_eq!(color.blue(), 47);
    }

    #[test]
    #[allow(deprecated)]
    fn constructor_and_accessors() {
        let color = Color::new(0xffeeddcc);
        assert_eq!(color.alpha(), 0xff);
        assert_eq!(color.red(), 0xee);
        assert_eq!(color.green(), 0xdd);
        assert_eq!(color.blue(), 0xcc);
    }

    #[test]
    fn p3_to_extended_srgb() {
        let p3 = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::DisplayP3);
        let srgb = p3.with_values(None, None, None, None, Some(ColorSpace::ExtendedSrgb));
        assert_eq!(srgb.a, 1.0);
        assert!((srgb.r - 1.0931).abs() < 1e-4);
        assert!((srgb.g - -0.22684034705162098).abs() < 1e-4);
        assert!((srgb.b - -0.15007957816123998).abs() < 1e-4);
        assert_eq!(srgb.color_space, ColorSpace::ExtendedSrgb);
    }

    #[test]
    fn p3_to_srgb() {
        let p3 = Color::from(1.0, 1.0, 0.0, 0.0, ColorSpace::DisplayP3);
        let srgb = p3.with_values(None, None, None, None, Some(ColorSpace::Srgb));
        assert_eq!(srgb.a, 1.0);
        assert!((srgb.r - 1.0).abs() < 1e-4);
        assert!((srgb.g - 0.0).abs() < 1e-4);
        assert!((srgb.b - 0.0).abs() < 1e-4);
        assert_eq!(srgb.color_space, ColorSpace::Srgb);
    }

    #[test]
    fn extended_srgb_to_p3() {
        let srgb = Color::from(1.0, 1.0931, -0.2268, -0.1501, ColorSpace::ExtendedSrgb);
        let p3 = srgb.with_values(None, None, None, None, Some(ColorSpace::DisplayP3));
        assert_eq!(p3.a, 1.0);
        assert!((p3.r - 1.0).abs() < 1e-4);
        assert!((p3.g - 0.0).abs() < 1e-4);
        assert!((p3.b - 0.0).abs() < 1e-4);
        assert_eq!(p3.color_space, ColorSpace::DisplayP3);
    }

    #[test]
    fn extended_srgb_to_p3_clamped() {
        let srgb = Color::from(1.0, 2.0, 0.0, 0.0, ColorSpace::ExtendedSrgb);
        let p3 = srgb.with_values(None, None, None, None, Some(ColorSpace::DisplayP3));
        assert_eq!(srgb.a, 1.0);
        assert!(p3.r <= 1.0);
        assert!(p3.g <= 1.0);
        assert!(p3.b <= 1.0);
        assert!(p3.r >= 0.0);
        assert!(p3.g >= 0.0);
        assert!(p3.b >= 0.0);
    }

    #[test]
    fn debug_matches_darts_to_string() {
        let color = Color::new(0xFF42A5F5);
        assert_eq!(
            format!("{color:?}"),
            "Color(alpha: 1.0000, red: 0.2588, green: 0.6471, blue: 0.9608, colorSpace: ColorSpace.sRGB)"
        );
    }
}
