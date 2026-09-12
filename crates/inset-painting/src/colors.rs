//! Flutter counterpart: `painting/colors.dart`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

use inset_embedder::{Color, clamp_double, lerp_double};

fn unit_channel(component: f64) -> f64 {
    (component * 255.0).round().clamp(0.0, 255.0) / 255.0
}

fn get_hue(red: f64, green: f64, blue: f64, max: f64, delta: f64) -> f64 {
    let hue = if max == 0.0 {
        0.0
    } else if max == red {
        60.0 * (((green - blue) / delta) % 6.0)
    } else if max == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    if hue.is_nan() { 0.0 } else { hue }
}

fn color_from_hue(alpha: f64, hue: f64, chroma: f64, secondary: f64, match_: f64) -> Color {
    let (red, green, blue) = if hue < 60.0 {
        (chroma, secondary, 0.0)
    } else if hue < 120.0 {
        (secondary, chroma, 0.0)
    } else if hue < 180.0 {
        (0.0, chroma, secondary)
    } else if hue < 240.0 {
        (0.0, secondary, chroma)
    } else if hue < 300.0 {
        (secondary, 0.0, chroma)
    } else {
        (chroma, 0.0, secondary)
    };
    Color::from_argb(
        (alpha * (0xFF as f64)).round() as i32,
        ((red + match_) * (0xFF as f64)).round() as i32,
        ((green + match_) * (0xFF as f64)).round() as i32,
        ((blue + match_) * (0xFF as f64)).round() as i32,
    )
}

/// A color represented using `alpha`, `hue`, `saturation`, and `value`.
#[derive(Clone, Copy, PartialEq)]
pub struct HSVColor {
    /// Alpha, from 0.0 to 1.0.
    pub alpha: f64,
    /// Hue, from 0.0 to 360.0.
    pub hue: f64,
    /// Saturation, from 0.0 to 1.0.
    pub saturation: f64,
    /// Value, from 0.0 to 1.0.
    pub value: f64,
}

impl HSVColor {
    /// Creates a color. All arguments must be in their respective ranges.
    pub fn from_ahsv(alpha: f64, hue: f64, saturation: f64, value: f64) -> HSVColor {
        debug_assert!((0.0..=1.0).contains(&alpha));
        debug_assert!((0.0..=360.0).contains(&hue));
        debug_assert!((0.0..=1.0).contains(&saturation));
        debug_assert!((0.0..=1.0).contains(&value));
        HSVColor {
            alpha,
            hue,
            saturation,
            value,
        }
    }

    /// Creates an [`HSVColor`] from an RGB [`Color`].
    pub fn from_color(color: Color) -> HSVColor {
        let red = unit_channel(color.r);
        let green = unit_channel(color.g);
        let blue = unit_channel(color.b);
        let max = red.max(green.max(blue));
        let min = red.min(green.min(blue));
        let delta = max - min;
        let alpha = unit_channel(color.a);
        let hue = get_hue(red, green, blue, max, delta);
        let saturation = if max == 0.0 { 0.0 } else { delta / max };
        HSVColor::from_ahsv(alpha, hue, saturation, max)
    }

    /// Returns a copy of this color with the `alpha` parameter replaced.
    pub fn with_alpha(self, alpha: f64) -> HSVColor {
        HSVColor::from_ahsv(alpha, self.hue, self.saturation, self.value)
    }

    /// Returns a copy of this color with the `hue` parameter replaced.
    pub fn with_hue(self, hue: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, hue, self.saturation, self.value)
    }

    /// Returns a copy of this color with the `saturation` parameter replaced.
    pub fn with_saturation(self, saturation: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, self.hue, saturation, self.value)
    }

    /// Returns a copy of this color with the `value` parameter replaced.
    pub fn with_value(self, value: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, self.hue, self.saturation, value)
    }

    /// Returns this color in RGB.
    pub fn to_color(self) -> Color {
        let chroma = self.saturation * self.value;
        let secondary = chroma * (1.0 - (((self.hue / 60.0) % 2.0) - 1.0).abs());
        let match_ = self.value - chroma;
        color_from_hue(self.alpha, self.hue, chroma, secondary, match_)
    }

    fn scale_alpha(self, factor: f64) -> HSVColor {
        self.with_alpha(self.alpha * factor)
    }

    /// Linearly interpolate between two HSVColors.
    pub fn lerp(a: Option<HSVColor>, b: Option<HSVColor>, t: f64) -> Option<HSVColor> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale_alpha(t)),
            (Some(a), None) => Some(a.scale_alpha(1.0 - t)),
            (Some(a), Some(b)) => Some(HSVColor::from_ahsv(
                clamp_double(
                    lerp_double(Some(a.alpha), Some(b.alpha), t).unwrap(),
                    0.0,
                    1.0,
                ),
                lerp_double(Some(a.hue), Some(b.hue), t).unwrap() % 360.0,
                clamp_double(
                    lerp_double(Some(a.saturation), Some(b.saturation), t).unwrap(),
                    0.0,
                    1.0,
                ),
                clamp_double(
                    lerp_double(Some(a.value), Some(b.value), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )),
        }
    }
}

impl Debug for HSVColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HSVColor({}, {}, {}, {})",
            self.alpha, self.hue, self.saturation, self.value
        )
    }
}

/// A color represented using `alpha`, `hue`, `saturation`, and `lightness`.
#[derive(Clone, Copy, PartialEq)]
pub struct HSLColor {
    /// Alpha, from 0.0 to 1.0.
    pub alpha: f64,
    /// Hue, from 0.0 to 360.0.
    pub hue: f64,
    /// Saturation, from 0.0 to 1.0.
    pub saturation: f64,
    /// Lightness, from 0.0 to 1.0.
    pub lightness: f64,
}

impl HSLColor {
    /// Creates a color. All arguments must be in their respective ranges.
    pub fn from_ahsl(alpha: f64, hue: f64, saturation: f64, lightness: f64) -> HSLColor {
        debug_assert!((0.0..=1.0).contains(&alpha));
        debug_assert!((0.0..=360.0).contains(&hue));
        debug_assert!((0.0..=1.0).contains(&saturation));
        debug_assert!((0.0..=1.0).contains(&lightness));
        HSLColor {
            alpha,
            hue,
            saturation,
            lightness,
        }
    }

    /// Creates an [`HSLColor`] from an RGB [`Color`].
    pub fn from_color(color: Color) -> HSLColor {
        let red = unit_channel(color.r);
        let green = unit_channel(color.g);
        let blue = unit_channel(color.b);
        let max = red.max(green.max(blue));
        let min = red.min(green.min(blue));
        let delta = max - min;
        let alpha = unit_channel(color.a);
        let hue = get_hue(red, green, blue, max, delta);
        let lightness = (max + min) / 2.0;
        let saturation = if min == max {
            0.0
        } else {
            clamp_double(delta / (1.0 - (2.0 * lightness - 1.0).abs()), 0.0, 1.0)
        };
        HSLColor::from_ahsl(alpha, hue, saturation, lightness)
    }

    /// Returns a copy of this color with the alpha parameter replaced.
    pub fn with_alpha(self, alpha: f64) -> HSLColor {
        HSLColor::from_ahsl(alpha, self.hue, self.saturation, self.lightness)
    }

    /// Returns a copy of this color with the `hue` parameter replaced.
    pub fn with_hue(self, hue: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, hue, self.saturation, self.lightness)
    }

    /// Returns a copy of this color with the `saturation` parameter replaced.
    pub fn with_saturation(self, saturation: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, self.hue, saturation, self.lightness)
    }

    /// Returns a copy of this color with the `lightness` parameter replaced.
    pub fn with_lightness(self, lightness: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, self.hue, self.saturation, lightness)
    }

    /// Returns this HSL color in RGB.
    pub fn to_color(self) -> Color {
        let chroma = (1.0 - (2.0 * self.lightness - 1.0).abs()) * self.saturation;
        let secondary = chroma * (1.0 - (((self.hue / 60.0) % 2.0) - 1.0).abs());
        let match_ = self.lightness - chroma / 2.0;
        color_from_hue(self.alpha, self.hue, chroma, secondary, match_)
    }

    fn scale_alpha(self, factor: f64) -> HSLColor {
        self.with_alpha(self.alpha * factor)
    }

    /// Linearly interpolate between two HSLColors.
    pub fn lerp(a: Option<HSLColor>, b: Option<HSLColor>, t: f64) -> Option<HSLColor> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale_alpha(t)),
            (Some(a), None) => Some(a.scale_alpha(1.0 - t)),
            (Some(a), Some(b)) => Some(HSLColor::from_ahsl(
                clamp_double(
                    lerp_double(Some(a.alpha), Some(b.alpha), t).unwrap(),
                    0.0,
                    1.0,
                ),
                lerp_double(Some(a.hue), Some(b.hue), t).unwrap() % 360.0,
                clamp_double(
                    lerp_double(Some(a.saturation), Some(b.saturation), t).unwrap(),
                    0.0,
                    1.0,
                ),
                clamp_double(
                    lerp_double(Some(a.lightness), Some(b.lightness), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )),
        }
    }
}

impl Debug for HSLColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HSLColor({}, {}, {}, {})",
            self.alpha, self.hue, self.saturation, self.lightness
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // colors_test.dart 'HSVColor control test'.
    #[test]
    fn hsv_color_control_test() {
        let color = HSVColor::from_ahsv(0.7, 28.0, 0.3, 0.6);
        assert_eq!(
            color.with_alpha(0.8),
            HSVColor::from_ahsv(0.8, 28.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_hue(123.0),
            HSVColor::from_ahsv(0.7, 123.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_saturation(0.9),
            HSVColor::from_ahsv(0.7, 28.0, 0.9, 0.6)
        );
        assert_eq!(
            color.with_value(0.1),
            HSVColor::from_ahsv(0.7, 28.0, 0.3, 0.1)
        );
        assert_eq!(color.to_color(), Color::new(0xb399816b));

        let result = HSVColor::lerp(
            Some(color),
            Some(HSVColor::from_ahsv(0.3, 128.0, 0.7, 0.2)),
            0.25,
        )
        .unwrap();
        assert!((result.alpha - 0.6).abs() < 1e-10);
        assert!((result.hue - 53.0).abs() < 1e-10);
        assert!(result.saturation > 0.3999 && result.saturation < 0.4001);
        assert!((result.value - 0.5).abs() < 1e-10);
    }

    // colors_test.dart 'HSVColor hue sweep test'.
    #[test]
    fn hsv_color_hue_sweep() {
        let mut output = Vec::new();
        let mut hue = 0.0;
        while hue <= 360.0 {
            output.push(HSVColor::from_ahsv(1.0, hue, 1.0, 1.0).to_color());
            hue += 36.0;
        }
        let expected = [
            Color::new(0xffff0000),
            Color::new(0xffff9900),
            Color::new(0xffccff00),
            Color::new(0xff33ff00),
            Color::new(0xff00ff66),
            Color::new(0xff00ffff),
            Color::new(0xff0066ff),
            Color::new(0xff3300ff),
            Color::new(0xffcc00ff),
            Color::new(0xffff0099),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSVColor saturation sweep test'.
    #[test]
    fn hsv_color_saturation_sweep() {
        let mut output = Vec::new();
        let mut saturation = 0.0;
        while saturation < 1.0 {
            output.push(HSVColor::from_ahsv(1.0, 0.0, saturation, 1.0).to_color());
            saturation += 0.1;
        }
        let expected = [
            Color::new(0xffffffff),
            Color::new(0xffffe6e6),
            Color::new(0xffffcccc),
            Color::new(0xffffb3b3),
            Color::new(0xffff9999),
            Color::new(0xffff8080),
            Color::new(0xffff6666),
            Color::new(0xffff4d4d),
            Color::new(0xffff3333),
            Color::new(0xffff1a1a),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSVColor value sweep test'.
    #[test]
    fn hsv_color_value_sweep() {
        let mut output = Vec::new();
        let mut value = 0.0;
        while value < 1.0 {
            output.push(HSVColor::from_ahsv(1.0, 0.0, 1.0, value).to_color());
            value += 0.1;
        }
        let expected = [
            Color::new(0xff000000),
            Color::new(0xff1a0000),
            Color::new(0xff330000),
            Color::new(0xff4d0000),
            Color::new(0xff660000),
            Color::new(0xff800000),
            Color::new(0xff990000),
            Color::new(0xffb30000),
            Color::new(0xffcc0000),
            Color::new(0xffe50000),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSLColor control test'.
    #[test]
    fn hsl_color_control_test() {
        let color = HSLColor::from_ahsl(0.7, 28.0, 0.3, 0.6);
        assert_eq!(
            color.with_alpha(0.8),
            HSLColor::from_ahsl(0.8, 28.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_hue(123.0),
            HSLColor::from_ahsl(0.7, 123.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_saturation(0.9),
            HSLColor::from_ahsl(0.7, 28.0, 0.9, 0.6)
        );
        assert_eq!(
            color.with_lightness(0.1),
            HSLColor::from_ahsl(0.7, 28.0, 0.3, 0.1)
        );
        assert_eq!(color.to_color(), Color::new(0xb3b8977a));

        let result = HSLColor::lerp(
            Some(color),
            Some(HSLColor::from_ahsl(0.3, 128.0, 0.7, 0.2)),
            0.25,
        )
        .unwrap();
        assert!((result.alpha - 0.6).abs() < 1e-10);
        assert!((result.hue - 53.0).abs() < 1e-10);
        assert!(result.saturation > 0.3999 && result.saturation < 0.4001);
        assert!((result.lightness - 0.5).abs() < 1e-10);
    }
}

/// What a subclass of `Color` adds to its value.
///
/// Flutter's `CupertinoDynamicColor`, `WidgetStateColor`, and [`ColorSwatch`] extend `Color`
/// and travel wherever a `Color` is stored, to be recognised later by an `is` check
/// (`CupertinoDynamicColor.resolve`, `WidgetStateProperty.resolveAs`). Here the engine's
/// [`Color`] is a plain value; a subclass is an extension that an [`AnyColor`] carries beside
/// the value the subclass constructor passed to `super`.
pub trait ColorExtension: Any + Debug {
    /// The extension as `Any`, for Dart's `is` checks ([`AnyColor::extension`]).
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==` override on the subclass, called once the values compared equal. `other`
    /// is the same runtime type when it downcasts; any other type is unequal, as Dart's
    /// `runtimeType` check makes it.
    fn eq_extension(&self, other: &dyn ColorExtension) -> bool;
}

/// How an [`AnyColor`] holds its extension: a `const` table entry, or an object built at
/// run time (a `WidgetStateColor.resolveWith` closure, a resolved dynamic color).
#[derive(Clone)]
pub enum ColorExtensionRef {
    Static(&'static dyn ColorExtension),
    Shared(Rc<dyn ColorExtension>),
}

impl ColorExtensionRef {
    fn get(&self) -> &dyn ColorExtension {
        match self {
            ColorExtensionRef::Static(extension) => *extension,
            ColorExtensionRef::Shared(extension) => &**extension,
        }
    }
}

/// A `Color` that may be an instance of a `Color` subclass.
///
/// Painting and the framework store this wherever Flutter stores a `Color`. The engine's
/// [`Color`] is the value the subclass passed to `super`, reached by [`color`](Self::color)
/// or by deref, so every `Color` method is available and returns a plain [`Color`], as an
/// inherited method does in Dart. The subclass is recovered with
/// [`extension`](Self::extension), Dart's `is` check.
///
/// Equality follows Dart's: two plain colors compare by value, two extended colors compare
/// by value and by the subclass's `==`, and a plain color never equals an extended one.
#[derive(Clone)]
pub struct AnyColor {
    color: Color,
    extension: Option<ColorExtensionRef>,
}

impl AnyColor {
    /// A plain color.
    pub const fn new(color: Color) -> AnyColor {
        AnyColor {
            color,
            extension: None,
        }
    }

    /// A subclass instance from a `const` table: `color` is what the subclass passes to
    /// `super`, `extension` the subclass itself.
    pub const fn with_static(color: Color, extension: &'static dyn ColorExtension) -> AnyColor {
        AnyColor {
            color,
            extension: Some(ColorExtensionRef::Static(extension)),
        }
    }

    /// A subclass instance built at run time.
    pub fn with_shared(color: Color, extension: Rc<dyn ColorExtension>) -> AnyColor {
        AnyColor {
            color,
            extension: Some(ColorExtensionRef::Shared(extension)),
        }
    }

    /// The color as the engine sees it: the subclass's `super` value.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Dart's `color is T`: the subclass instance, if this color is one.
    pub fn extension<T: ColorExtension>(&self) -> Option<&T> {
        self.extension
            .as_ref()
            .and_then(|extension| extension.get().as_any().downcast_ref::<T>())
    }

    /// Whether this color is a subclass instance.
    pub fn has_extension(&self) -> bool {
        self.extension.is_some()
    }

    /// `Color.lerp` on the values; the subclasses do not survive interpolation, as in Dart.
    pub fn lerp(a: Option<&AnyColor>, b: Option<&AnyColor>, t: f64) -> Option<Color> {
        Color::lerp(a.map(AnyColor::color), b.map(AnyColor::color), t)
    }
}

impl Deref for AnyColor {
    type Target = Color;

    fn deref(&self) -> &Color {
        &self.color
    }
}

impl From<Color> for AnyColor {
    fn from(color: Color) -> AnyColor {
        AnyColor::new(color)
    }
}

impl PartialEq for AnyColor {
    fn eq(&self, other: &AnyColor) -> bool {
        if self.color != other.color {
            return false;
        }
        match (&self.extension, &other.extension) {
            (None, None) => true,
            (Some(mine), Some(theirs)) => mine.get().eq_extension(theirs.get()),
            _ => false,
        }
    }
}

impl PartialEq<Color> for AnyColor {
    fn eq(&self, other: &Color) -> bool {
        self.extension.is_none() && self.color == *other
    }
}

impl Debug for AnyColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.extension {
            Some(extension) => extension.get().fmt(f),
            None => self.color.fmt(f),
        }
    }
}

/// A color that has a small table of related colors called a "swatch".
///
/// The table is accessed by key values of type `T`.
///
/// See also:
///
///  * `MaterialColor` and `MaterialAccentColor`, which define Material Design primary and
///    accent color swatches.
///  * `Colors`, which defines all of the standard Material Design colors.
#[derive(Clone)]
pub struct ColorSwatch<T: Clone + Eq + Hash + Debug + 'static> {
    primary: Color,
    swatch: HashMap<T, Color>,
}

impl<T: Clone + Eq + Hash + Debug + 'static> ColorSwatch<T> {
    /// Creates a color that has a small table of related colors called a "swatch".
    ///
    /// The `primary` argument should be the 32 bit ARGB value of one of the values in the
    /// swatch, as would be passed to the `Color.new` constructor for that same color, and as
    /// is exposed by `value`. (This is distinct from the specific index of the color in the
    /// swatch.)
    pub fn new(primary: Color, swatch: HashMap<T, Color>) -> ColorSwatch<T> {
        ColorSwatch { primary, swatch }
    }

    /// The primary color: what Dart's swatch is, as a `Color`.
    pub fn primary(&self) -> Color {
        self.primary
    }

    /// Returns an element of the swatch table.
    pub fn get(&self, key: &T) -> Option<Color> {
        self.swatch.get(key).copied()
    }

    /// Returns the valid keys for accessing operator[].
    pub fn keys(&self) -> impl Iterator<Item = &T> {
        self.swatch.keys()
    }

    /// Linearly interpolate between two [`ColorSwatch`]es.
    ///
    /// It delegates to `Color.lerp` to proportionally scale the interpolation.
    ///
    /// If either color is null, this function linearly interpolates from a transparent
    /// instance of the other color.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning that the
    /// interpolation has not started, returning `a` (or something equivalent to `a`), 1.0
    /// meaning that the interpolation has finished, returning `b` (or something equivalent to
    /// `b`), and values in between meaning that the interpolation is at the relevant point on
    /// the timeline between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can easily be
    /// generated by curves such as `Curves.elasticInOut`).
    ///
    /// Values for `t` are usually obtained from an `Animation<double>`, such as an
    /// `AnimationController`.
    pub fn lerp(
        a: Option<&ColorSwatch<T>>,
        b: Option<&ColorSwatch<T>>,
        t: f64,
    ) -> Option<ColorSwatch<T>> {
        let swatch: HashMap<T, Color> = match (a, b) {
            (None, None) => return None,
            (Some(a), None) => a
                .swatch
                .iter()
                .map(|(key, color)| {
                    (
                        key.clone(),
                        Color::lerp(Some(*color), None, t).expect("one side"),
                    )
                })
                .collect(),
            (None, Some(b)) => b
                .swatch
                .iter()
                .map(|(key, color)| {
                    (
                        key.clone(),
                        Color::lerp(None, Some(*color), t).expect("one side"),
                    )
                })
                .collect(),
            (Some(a), Some(b)) => a
                .swatch
                .iter()
                .map(|(key, color)| {
                    (
                        key.clone(),
                        Color::lerp(Some(*color), b.get(key), t).expect("one side"),
                    )
                })
                .collect(),
        };
        let primary = Color::lerp(a.map(ColorSwatch::primary), b.map(ColorSwatch::primary), t)
            .expect("one side");
        Some(ColorSwatch::new(primary, swatch))
    }

    /// The swatch as the `Color` it is in Dart.
    pub fn into_any(self) -> AnyColor {
        AnyColor::with_shared(self.primary, Rc::new(self))
    }
}

impl<T: Clone + Eq + Hash + Debug + 'static> ColorExtension for ColorSwatch<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<ColorSwatch<T>>()
            .is_some_and(|other| other.swatch == self.swatch)
    }
}

impl<T: Clone + Eq + Hash + Debug + 'static> PartialEq for ColorSwatch<T> {
    fn eq(&self, other: &ColorSwatch<T>) -> bool {
        self.primary == other.primary && self.swatch == other.swatch
    }
}

impl<T: Clone + Eq + Hash + Debug + 'static> Debug for ColorSwatch<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ColorSwatch(primary value: {:?})", self.primary)
    }
}

impl<T: Clone + Eq + Hash + Debug + 'static> From<ColorSwatch<T>> for AnyColor {
    fn from(swatch: ColorSwatch<T>) -> AnyColor {
        swatch.into_any()
    }
}

#[cfg(test)]
mod any_color_tests {
    use super::*;

    /// A stand-in subclass: a color with a name, equal when the names match.
    #[derive(Debug)]
    struct Named {
        name: &'static str,
    }

    impl ColorExtension for Named {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
            other
                .as_any()
                .downcast_ref::<Named>()
                .is_some_and(|other| other.name == self.name)
        }
    }

    const RED: Color = Color::from_argb(255, 255, 0, 0);
    const BLUE: Color = Color::from_argb(255, 0, 0, 255);
    static NAMED_RED: Named = Named { name: "red" };
    const NAMED: AnyColor = AnyColor::with_static(RED, &NAMED_RED);

    #[test]
    fn a_plain_color_and_a_subclass_with_the_same_value_are_not_equal() {
        let plain = AnyColor::from(RED);
        assert_eq!(plain, AnyColor::new(RED));
        assert_eq!(plain, RED);
        assert_ne!(plain, NAMED);
        assert_ne!(NAMED, RED);
        assert_eq!(
            NAMED,
            AnyColor::with_shared(RED, Rc::new(Named { name: "red" }))
        );
        assert_ne!(
            NAMED,
            AnyColor::with_shared(RED, Rc::new(Named { name: "crimson" }))
        );
        assert_ne!(
            NAMED,
            AnyColor::with_shared(BLUE, Rc::new(Named { name: "red" }))
        );
    }

    #[test]
    fn the_extension_is_recovered_by_type_and_the_value_by_deref() {
        assert_eq!(
            NAMED.extension::<Named>().map(|named| named.name),
            Some("red")
        );
        assert!(NAMED.extension::<ColorSwatch<i32>>().is_none());
        assert!(AnyColor::new(RED).extension::<Named>().is_none());
        assert_eq!(NAMED.color(), RED);
        assert_eq!(NAMED.to_argb32(), RED.to_argb32());
        assert_eq!(NAMED.with_alpha(128), RED.with_alpha(128));
        assert_eq!(format!("{NAMED:?}"), "Named { name: \"red\" }");
    }

    #[test]
    fn lerp_drops_the_subclass() {
        let halfway = AnyColor::lerp(Some(&NAMED), Some(&AnyColor::new(BLUE)), 0.5).unwrap();
        assert_eq!(halfway, Color::lerp(Some(RED), Some(BLUE), 0.5).unwrap());
        assert!(AnyColor::lerp(None, None, 0.5).is_none());
    }

    #[test]
    fn a_swatch_indexes_its_table_and_interpolates_per_key() {
        let swatch = ColorSwatch::new(RED, HashMap::from([(100, RED), (900, BLUE)]));
        assert_eq!(swatch.get(&100), Some(RED));
        assert_eq!(swatch.get(&500), None);
        let any = swatch.clone().into_any();
        assert_eq!(any.color(), RED);
        assert_eq!(any.extension::<ColorSwatch<i32>>(), Some(&swatch));
        let other = ColorSwatch::new(BLUE, HashMap::from([(100, BLUE)]));
        let lerped = ColorSwatch::lerp(Some(&swatch), Some(&other), 0.5).unwrap();
        assert_eq!(
            lerped.primary(),
            Color::lerp(Some(RED), Some(BLUE), 0.5).unwrap()
        );
        assert_eq!(lerped.get(&100), Color::lerp(Some(RED), Some(BLUE), 0.5));
        assert_eq!(lerped.get(&900), Color::lerp(Some(BLUE), None, 0.5));
        assert!(ColorSwatch::<i32>::lerp(None, None, 0.5).is_none());
    }
}
