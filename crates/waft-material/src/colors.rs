//! Flutter counterpart: material_ui's `colors.dart`.
//!
//! Dart's `MaterialColor` and `MaterialAccentColor` extend `ColorSwatch<int>`, which extends
//! `Color`. Here each is a [`ColorExtension`] that derefs to painting's [`ColorSwatch`] and
//! becomes the [`AnyColor`] it is in Dart through `into_any` (see waft-painting's
//! `PORTING.md`). The swatches in [`Colors`] are `const` tables, as Dart's are.

use std::any::Any;
use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;

use waft_embedder::Color;
use waft_painting::{AnyColor, ColorExtension, ColorSwatch};

/// Defines a single color as well a color swatch with ten shades of the color.
///
/// The color's shades are referred to by index. The greater the index, the
/// darker the color. There are 10 valid indices: 50, 100, 200, ..., 900.
/// The value of this color should the same the value of index 500 and `shade500`.
///
/// ## Updating to `ColorScheme`
///
/// The `ColorScheme` is preferred for
/// representing colors in applications that are configured
/// for Material 3 (see `ThemeData.useMaterial3`).
/// For more information on colors in Material 3 see
/// the spec at <https://m3.material.io/styles/color/the-color-system>.
///
///{@template material_ui.colors.colorRoles}
/// In Material 3, colors are represented using color roles and
/// corresponding tokens. Each property in the `ColorScheme` class
/// represents one color role as defined in the spec above.
/// {@endtemplate}
///
/// ### Material 3 Colors in Flutter
///
///{@template material_ui.colors.settingColors}
/// Flutter's Material widgets can be assigned colors at the widget level
/// using widget properties,
/// or at the app level using theme classes.
///
/// For example, you can set the background of the `AppBar` by
/// setting the `AppBar.backgroundColor` to a specific `Color` value.
///
/// To globally set the AppBar background color for your app, you
/// can set the `ThemeData.appBarTheme` property for your `MaterialApp`
/// using the `ThemeData` class. You can also override
/// the default appearance of all the `AppBar`s in a widget subtree by
/// placing the `AppBarTheme` at the root of the subtree.
///
/// Alternatively, you can set the `ThemeData.colorScheme` property
/// to a custom `ColorScheme`. This creates a unified `ColorScheme` to be
/// used across the app. The `AppBar.backgroundColor` uses the
/// `ColorScheme.surface` by default.
///{@endtemplate}
///
/// ### Migrating from `MaterialColor` to `ColorScheme`
///
/// In most cases, there are new properties in Flutter widgets that
/// accept a `ColorScheme` instead of a `MaterialColor`.
///
/// For example, you may have previously constructed a `ThemeData`
/// using a primarySwatch:
///
/// ```dart
/// ThemeData(
///   primarySwatch: Colors.amber,
/// )
/// ```
///
/// In Material 3, you can use the `ColorScheme` class to
/// construct a `ThemeData` with the same color palette
/// by using the `ColorScheme.fromSeed` constructor:
///
///  ```dart
/// ThemeData(
///   colorScheme: ColorScheme.fromSeed(seedColor: Colors.amber),
/// )
/// ```
///
/// The `ColorScheme.fromSeed` constructor
/// will generate a set of tonal palettes,
/// which are used to create the color scheme.
///
/// Alternatively you can use the `ColorScheme.fromSwatch` constructor:
///
/// ```dart
/// ThemeData(
///  colorScheme: ColorScheme.fromSwatch(primarySwatch: Colors.amber),
/// )
/// ```
///
/// The `ColorScheme.fromSwatch` constructor will
/// create the color scheme directly from the specific
/// color values used in the `MaterialColor`.
///
///
/// See also:
///
///  * `Colors`, which defines all of the standard material colors.
#[derive(Clone, PartialEq)]
pub struct MaterialColor {
    swatch: ColorSwatch<i32>,
}

impl MaterialColor {
    /// Creates a color swatch with a variety of shades.
    ///
    /// The `primary` argument should be the 32 bit ARGB value of one of the
    /// values in the swatch, as would be passed to the `Color.new` constructor
    /// for that same color, and as is exposed by `value`. (This is distinct from
    /// the specific index of the color in the swatch.)
    pub fn new(primary: Color, swatch: impl IntoIterator<Item = (i32, Color)>) -> MaterialColor {
        MaterialColor {
            swatch: ColorSwatch::new(primary, swatch),
        }
    }

    /// Dart's `const MaterialColor(primary, {...})`, for a table written in the source.
    pub const fn from_static(primary: Color, swatch: &'static [(i32, Color)]) -> MaterialColor {
        MaterialColor {
            swatch: ColorSwatch::from_static(primary, swatch),
        }
    }

    /// The lightest shade.
    pub fn shade50(&self) -> Color {
        self.get(&50).expect("a Material swatch has shade 50")
    }

    /// The second lightest shade.
    pub fn shade100(&self) -> Color {
        self.get(&100).expect("a Material swatch has shade 100")
    }

    /// The third lightest shade.
    pub fn shade200(&self) -> Color {
        self.get(&200).expect("a Material swatch has shade 200")
    }

    /// The fourth lightest shade.
    pub fn shade300(&self) -> Color {
        self.get(&300).expect("a Material swatch has shade 300")
    }

    /// The fifth lightest shade.
    pub fn shade400(&self) -> Color {
        self.get(&400).expect("a Material swatch has shade 400")
    }

    /// The default shade.
    pub fn shade500(&self) -> Color {
        self.get(&500).expect("a Material swatch has shade 500")
    }

    /// The fourth darkest shade.
    pub fn shade600(&self) -> Color {
        self.get(&600).expect("a Material swatch has shade 600")
    }

    /// The third darkest shade.
    pub fn shade700(&self) -> Color {
        self.get(&700).expect("a Material swatch has shade 700")
    }

    /// The second darkest shade.
    pub fn shade800(&self) -> Color {
        self.get(&800).expect("a Material swatch has shade 800")
    }

    /// The darkest shade.
    pub fn shade900(&self) -> Color {
        self.get(&900).expect("a Material swatch has shade 900")
    }

    /// The swatch as the `Color` it is in Dart.
    pub fn into_any(self) -> AnyColor {
        AnyColor::with_shared(self.swatch.primary(), Rc::new(self))
    }
}

impl Deref for MaterialColor {
    type Target = ColorSwatch<i32>;

    fn deref(&self) -> &ColorSwatch<i32> {
        &self.swatch
    }
}

impl ColorExtension for MaterialColor {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<MaterialColor>()
            .is_some_and(|other| other == self)
    }
}

impl From<MaterialColor> for AnyColor {
    fn from(color: MaterialColor) -> AnyColor {
        color.into_any()
    }
}

impl Debug for MaterialColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MaterialColor(primary value: {:?})",
            self.swatch.primary()
        )
    }
}

/// Defines a single accent color as well a swatch of four shades of the
/// accent color.
///
/// The color's shades are referred to by index, the colors with smaller
/// indices are lighter, larger indices are darker. There are four valid
/// indices: 100, 200, 400, and 700. The value of this color should be the
/// same as the value of index 200 and `shade200`.
///
/// See also:
///
///  * `Colors`, which defines all of the standard material colors.
///  * <https://material.io/go/design-theming#color-color-schemes>
#[derive(Clone, PartialEq)]
pub struct MaterialAccentColor {
    swatch: ColorSwatch<i32>,
}

impl MaterialAccentColor {
    /// Creates a color swatch with a variety of shades appropriate for accent
    /// colors.
    pub fn new(
        primary: Color,
        swatch: impl IntoIterator<Item = (i32, Color)>,
    ) -> MaterialAccentColor {
        MaterialAccentColor {
            swatch: ColorSwatch::new(primary, swatch),
        }
    }

    /// Dart's `const MaterialAccentColor(primary, {...})`, for a table written in the source.
    pub const fn from_static(
        primary: Color,
        swatch: &'static [(i32, Color)],
    ) -> MaterialAccentColor {
        MaterialAccentColor {
            swatch: ColorSwatch::from_static(primary, swatch),
        }
    }

    /// The lightest shade.
    pub fn shade100(&self) -> Color {
        self.get(&100).expect("a Material swatch has shade 100")
    }

    /// The default shade.
    pub fn shade200(&self) -> Color {
        self.get(&200).expect("a Material swatch has shade 200")
    }

    /// The second darkest shade.
    pub fn shade400(&self) -> Color {
        self.get(&400).expect("a Material swatch has shade 400")
    }

    /// The darkest shade.
    pub fn shade700(&self) -> Color {
        self.get(&700).expect("a Material swatch has shade 700")
    }

    /// The swatch as the `Color` it is in Dart.
    pub fn into_any(self) -> AnyColor {
        AnyColor::with_shared(self.swatch.primary(), Rc::new(self))
    }
}

impl Deref for MaterialAccentColor {
    type Target = ColorSwatch<i32>;

    fn deref(&self) -> &ColorSwatch<i32> {
        &self.swatch
    }
}

impl ColorExtension for MaterialAccentColor {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<MaterialAccentColor>()
            .is_some_and(|other| other == self)
    }
}

impl From<MaterialAccentColor> for AnyColor {
    fn from(color: MaterialAccentColor) -> AnyColor {
        color.into_any()
    }
}

impl Debug for MaterialAccentColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MaterialAccentColor(primary value: {:?})",
            self.swatch.primary()
        )
    }
}

/// `Color` and `ColorSwatch` constants which represent Material design's
/// [color palette](https://material.io/design/color/).
///
/// Instead of using an absolute color from these palettes, consider using
/// `Theme.of` to obtain the local `ThemeData.colorScheme`, which defines
/// the colors that most of the Material components use by default.
///
///
/// Most swatches have colors from 100 to 900 in increments of one hundred, plus
/// the color 50. The smaller the number, the more pale the color. The greater
/// the number, the darker the color. The accent swatches (e.g. [`RED_ACCENT`](Self::RED_ACCENT)) only
/// have the values 100, 200, 400, and 700.
///
/// In addition, a series of blacks and whites with common opacities are
/// available. For example, [`BLACK54`](Self::BLACK54) is a pure black with 54% opacity.
///
/// <callout-box>
///
// TODO(framework): Add unit tests to this code snippet.
// https://github.com/flutter/flutter/issues/188530
///
/// To select a specific color from one of the swatches, index into the swatch
/// using an integer for the specific color desired, as follows:
///
/// ```dart
/// Color selection = Colors.green[400]!; // Selects a mid-range green.
/// ```
///
/// </callout-box>
///
/// <callout-box>
///
// TODO(framework): Add unit tests to this code snippet.
// https://github.com/flutter/flutter/issues/188530
///
/// Each `ColorSwatch` constant is a color and can used directly. For example:
///
/// ```dart
/// Container(
///   color: Colors.blue, // same as Colors.blue[500] or Colors.blue.shade500
/// )
/// ```
///
/// </callout-box>
///
/// ## Color palettes
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.brown.png)
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.grey.png)
///
/// ## Blacks and whites
///
/// These colors are identified by their transparency. The low transparency
/// levels (e.g. [`Colors::WHITE12`](Colors::WHITE12) and [`Colors::WHITE10`](Colors::WHITE10)) are very hard to see and
/// should be avoided in general. They are intended for very subtle effects.
///
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
///
/// The [`Colors::TRANSPARENT`](Colors::TRANSPARENT) color isn't shown here because it is entirely
/// invisible!
///
/// See also:
///
///  * Cookbook: [Use themes to share colors and font styles](https://docs.flutter.dev/cookbook/design/themes)
pub struct Colors;

impl Colors {
    /// Completely invisible.
    pub const TRANSPARENT: Color = Color::new(0x00000000);

    /// Completely opaque black.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    ///  * [`WHITE`](Self::WHITE), a solid white color.
    ///  * [`TRANSPARENT`](Self::TRANSPARENT), a fully-transparent color.
    pub const BLACK: Color = Color::new(0xFF000000);

    /// Black with 87% opacity.
    ///
    /// This is a good contrasting color for text in light themes.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * `Typography.black`, which uses this color for its text styles.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`BLACK`](Self::BLACK), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    pub const BLACK87: Color = Color::new(0xDD000000);

    /// Black with 54% opacity.
    ///
    /// This is a color commonly used for headings in light themes. It's also used
    /// as the mask color behind dialogs.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * `Typography.black`, which uses this color for its text styles.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    pub const BLACK54: Color = Color::new(0x8A000000);

    /// Black with 45% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    pub const BLACK45: Color = Color::new(0x73000000);

    /// Black with 38% opacity.
    ///
    /// For light themes, i.e. when the Theme's `ThemeData.brightness` is
    /// `Brightness.light`, this color is used for disabled icons and for
    /// placeholder text in `DataTable`.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    pub const BLACK38: Color = Color::new(0x61000000);

    /// Black with 26% opacity.
    ///
    /// Used for disabled radio buttons and the text of disabled flat buttons in light themes.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// See also:
    ///
    ///  * `ThemeData.disabledColor`, which uses this color by default in light themes.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK12`](Self::BLACK12), which
    ///    are variants on this color but with different opacities.
    pub const BLACK26: Color = Color::new(0x42000000);

    /// Black with 12% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blacks.png)
    ///
    /// Used for the background of disabled raised buttons in light themes.
    ///
    /// See also:
    ///
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), which
    ///    are variants on this color but with different opacities.
    pub const BLACK12: Color = Color::new(0x1F000000);

    /// Completely opaque white.
    ///
    /// This is a good contrasting color for the `ThemeData.primaryColor` in the
    /// dark theme. See `ThemeData.brightness`.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `Typography.white`, which uses this color for its text styles.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE70`](Self::WHITE70), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30), [`WHITE12`](Self::WHITE12),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color but with different
    ///    opacities.
    ///  * [`BLACK`](Self::BLACK), a solid black color.
    ///  * [`TRANSPARENT`](Self::TRANSPARENT), a fully-transparent color.
    pub const WHITE: Color = Color::new(0xFFFFFFFF);

    /// White with 70% opacity.
    ///
    /// This is a color commonly used for headings in dark themes.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `Typography.white`, which uses this color for its text styles.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30), [`WHITE12`](Self::WHITE12),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color but with different
    ///    opacities.
    pub const WHITE70: Color = Color::new(0xB3FFFFFF);

    /// White with 60% opacity.
    ///
    /// Used for medium-emphasis text and hint text when `ThemeData.brightness` is
    /// set to `Brightness.dark`.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `ExpandIcon`, which uses this color for dark themes.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE`](Self::WHITE), [`WHITE54`](Self::WHITE54), [`WHITE30`](Self::WHITE30), [`WHITE38`](Self::WHITE38), [`WHITE12`](Self::WHITE12), [`WHITE10`](Self::WHITE10), which
    ///    are variants on this color but with different opacities.
    pub const WHITE60: Color = Color::new(0x99FFFFFF);

    /// White with 54% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30), [`WHITE12`](Self::WHITE12), [`WHITE10`](Self::WHITE10), which
    ///    are variants on this color but with different opacities.
    pub const WHITE54: Color = Color::new(0x8AFFFFFF);

    /// White with 38% opacity.
    ///
    /// Used for disabled radio buttons and the text of disabled flat buttons in dark themes.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `ThemeData.disabledColor`, which uses this color by default in dark themes.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE70`](Self::WHITE70), [`WHITE30`](Self::WHITE30), [`WHITE12`](Self::WHITE12),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color but with different
    ///    opacities.
    pub const WHITE38: Color = Color::new(0x62FFFFFF);

    /// White with 30% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE70`](Self::WHITE70), [`WHITE38`](Self::WHITE38), [`WHITE12`](Self::WHITE12),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color but with different
    ///    opacities.
    pub const WHITE30: Color = Color::new(0x4DFFFFFF);

    /// White with 24% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// Used for the splash color for filled buttons.
    ///
    /// See also:
    ///
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE70`](Self::WHITE70), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color
    ///    but with different opacities.
    pub const WHITE24: Color = Color::new(0x3DFFFFFF);

    /// White with 12% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// Used for the background of disabled raised buttons in dark themes.
    ///
    /// See also:
    ///
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE70`](Self::WHITE70), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30),
    ///    [`WHITE10`](Self::WHITE10), which are variants on this color but with different
    ///    opacities.
    pub const WHITE12: Color = Color::new(0x1FFFFFFF);

    /// White with 10% opacity.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.whites.png)
    ///
    /// See also:
    ///
    ///  * [`WHITE`](Self::WHITE), [`WHITE60`](Self::WHITE60), [`WHITE54`](Self::WHITE54), [`WHITE70`](Self::WHITE70), [`WHITE38`](Self::WHITE38), [`WHITE30`](Self::WHITE30),
    ///    [`WHITE12`](Self::WHITE12), which are variants on this color
    ///    but with different opacities.
    ///  * [`TRANSPARENT`](Self::TRANSPARENT), a fully-transparent color, not far from this one.
    pub const WHITE10: Color = Color::new(0x1AFFFFFF);

    /// The red primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.red[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`RED_ACCENT`](Self::RED_ACCENT), the corresponding accent colors.
    ///  * [`DEEP_ORANGE`](Self::DEEP_ORANGE) and [`PINK`](Self::PINK), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const RED: MaterialColor = MaterialColor::from_static(
        Color::new(Self::RED_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFFEBEE)),
            (100, Color::new(0xFFFFCDD2)),
            (200, Color::new(0xFFEF9A9A)),
            (300, Color::new(0xFFE57373)),
            (400, Color::new(0xFFEF5350)),
            (500, Color::new(Self::RED_PRIMARY_VALUE)),
            (600, Color::new(0xFFE53935)),
            (700, Color::new(0xFFD32F2F)),
            (800, Color::new(0xFFC62828)),
            (900, Color::new(0xFFB71C1C)),
        ],
    );
    const RED_PRIMARY_VALUE: u32 = 0xFFF44336;

    /// The red accent swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.redAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`RED`](Self::RED), the corresponding primary colors.
    ///  * [`DEEP_ORANGE_ACCENT`](Self::DEEP_ORANGE_ACCENT) and [`PINK_ACCENT`](Self::PINK_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const RED_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::RED_ACCENT_VALUE),
        &[
            (100, Color::new(0xFFFF8A80)),
            (200, Color::new(Self::RED_ACCENT_VALUE)),
            (400, Color::new(0xFFFF1744)),
            (700, Color::new(0xFFD50000)),
        ],
    );
    const RED_ACCENT_VALUE: u32 = 0xFFFF5252;

    /// The pink primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.pink[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`PINK_ACCENT`](Self::PINK_ACCENT), the corresponding accent colors.
    ///  * [`RED`](Self::RED) and [`PURPLE`](Self::PURPLE), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const PINK: MaterialColor = MaterialColor::from_static(
        Color::new(Self::PINK_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFCE4EC)),
            (100, Color::new(0xFFF8BBD0)),
            (200, Color::new(0xFFF48FB1)),
            (300, Color::new(0xFFF06292)),
            (400, Color::new(0xFFEC407A)),
            (500, Color::new(Self::PINK_PRIMARY_VALUE)),
            (600, Color::new(0xFFD81B60)),
            (700, Color::new(0xFFC2185B)),
            (800, Color::new(0xFFAD1457)),
            (900, Color::new(0xFF880E4F)),
        ],
    );
    const PINK_PRIMARY_VALUE: u32 = 0xFFE91E63;

    /// The pink accent color swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.pinkAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`PINK`](Self::PINK), the corresponding primary colors.
    ///  * [`RED_ACCENT`](Self::RED_ACCENT) and [`PURPLE_ACCENT`](Self::PURPLE_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const PINK_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::PINK_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFFF80AB)),
            (200, Color::new(Self::PINK_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFF50057)),
            (700, Color::new(0xFFC51162)),
        ],
    );
    const PINK_ACCENT_PRIMARY_VALUE: u32 = 0xFFFF4081;

    /// The purple primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.purple[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`PURPLE_ACCENT`](Self::PURPLE_ACCENT), the corresponding accent colors.
    ///  * [`DEEP_PURPLE`](Self::DEEP_PURPLE) and [`PINK`](Self::PINK), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const PURPLE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::PURPLE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFF3E5F5)),
            (100, Color::new(0xFFE1BEE7)),
            (200, Color::new(0xFFCE93D8)),
            (300, Color::new(0xFFBA68C8)),
            (400, Color::new(0xFFAB47BC)),
            (500, Color::new(Self::PURPLE_PRIMARY_VALUE)),
            (600, Color::new(0xFF8E24AA)),
            (700, Color::new(0xFF7B1FA2)),
            (800, Color::new(0xFF6A1B9A)),
            (900, Color::new(0xFF4A148C)),
        ],
    );
    const PURPLE_PRIMARY_VALUE: u32 = 0xFF9C27B0;

    /// The purple accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pink.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.pinkAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.purpleAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`PURPLE`](Self::PURPLE), the corresponding primary colors.
    ///  * [`DEEP_PURPLE_ACCENT`](Self::DEEP_PURPLE_ACCENT) and [`PINK_ACCENT`](Self::PINK_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const PURPLE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::PURPLE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFEA80FC)),
            (200, Color::new(Self::PURPLE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFD500F9)),
            (700, Color::new(0xFFAA00FF)),
        ],
    );
    const PURPLE_ACCENT_PRIMARY_VALUE: u32 = 0xFFE040FB;

    /// The deep purple primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.deepPurple[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`DEEP_PURPLE_ACCENT`](Self::DEEP_PURPLE_ACCENT), the corresponding accent colors.
    ///  * [`PURPLE`](Self::PURPLE) and [`INDIGO`](Self::INDIGO), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const DEEP_PURPLE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::DEEP_PURPLE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFEDE7F6)),
            (100, Color::new(0xFFD1C4E9)),
            (200, Color::new(0xFFB39DDB)),
            (300, Color::new(0xFF9575CD)),
            (400, Color::new(0xFF7E57C2)),
            (500, Color::new(Self::DEEP_PURPLE_PRIMARY_VALUE)),
            (600, Color::new(0xFF5E35B1)),
            (700, Color::new(0xFF512DA8)),
            (800, Color::new(0xFF4527A0)),
            (900, Color::new(0xFF311B92)),
        ],
    );
    const DEEP_PURPLE_PRIMARY_VALUE: u32 = 0xFF673AB7;

    /// The deep purple accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.purpleAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.deepPurpleAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`DEEP_PURPLE`](Self::DEEP_PURPLE), the corresponding primary colors.
    ///  * [`PURPLE_ACCENT`](Self::PURPLE_ACCENT) and [`INDIGO_ACCENT`](Self::INDIGO_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const DEEP_PURPLE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::DEEP_PURPLE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFB388FF)),
            (200, Color::new(Self::DEEP_PURPLE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF651FFF)),
            (700, Color::new(0xFF6200EA)),
        ],
    );
    const DEEP_PURPLE_ACCENT_PRIMARY_VALUE: u32 = 0xFF7C4DFF;

    /// The indigo primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.indigo[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`INDIGO_ACCENT`](Self::INDIGO_ACCENT), the corresponding accent colors.
    ///  * [`BLUE`](Self::BLUE) and [`DEEP_PURPLE`](Self::DEEP_PURPLE), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const INDIGO: MaterialColor = MaterialColor::from_static(
        Color::new(Self::INDIGO_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE8EAF6)),
            (100, Color::new(0xFFC5CAE9)),
            (200, Color::new(0xFF9FA8DA)),
            (300, Color::new(0xFF7986CB)),
            (400, Color::new(0xFF5C6BC0)),
            (500, Color::new(Self::INDIGO_PRIMARY_VALUE)),
            (600, Color::new(0xFF3949AB)),
            (700, Color::new(0xFF303F9F)),
            (800, Color::new(0xFF283593)),
            (900, Color::new(0xFF1A237E)),
        ],
    );
    const INDIGO_PRIMARY_VALUE: u32 = 0xFF3F51B5;

    /// The indigo accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurple.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepPurpleAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.indigoAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`INDIGO`](Self::INDIGO), the corresponding primary colors.
    ///  * [`BLUE_ACCENT`](Self::BLUE_ACCENT) and [`DEEP_PURPLE_ACCENT`](Self::DEEP_PURPLE_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const INDIGO_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::INDIGO_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFF8C9EFF)),
            (200, Color::new(Self::INDIGO_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF3D5AFE)),
            (700, Color::new(0xFF304FFE)),
        ],
    );
    const INDIGO_ACCENT_PRIMARY_VALUE: u32 = 0xFF536DFE;

    /// The blue primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.blue[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`BLUE_ACCENT`](Self::BLUE_ACCENT), the corresponding accent colors.
    ///  * [`INDIGO`](Self::INDIGO), [`LIGHT_BLUE`](Self::LIGHT_BLUE), and [`BLUE_GREY`](Self::BLUE_GREY), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const BLUE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::BLUE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE3F2FD)),
            (100, Color::new(0xFFBBDEFB)),
            (200, Color::new(0xFF90CAF9)),
            (300, Color::new(0xFF64B5F6)),
            (400, Color::new(0xFF42A5F5)),
            (500, Color::new(Self::BLUE_PRIMARY_VALUE)),
            (600, Color::new(0xFF1E88E5)),
            (700, Color::new(0xFF1976D2)),
            (800, Color::new(0xFF1565C0)),
            (900, Color::new(0xFF0D47A1)),
        ],
    );
    const BLUE_PRIMARY_VALUE: u32 = 0xFF2196F3;

    /// The blue accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigo.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.indigoAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.blueAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`BLUE`](Self::BLUE), the corresponding primary colors.
    ///  * [`INDIGO_ACCENT`](Self::INDIGO_ACCENT) and [`LIGHT_BLUE_ACCENT`](Self::LIGHT_BLUE_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const BLUE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::BLUE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFF82B1FF)),
            (200, Color::new(Self::BLUE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF2979FF)),
            (700, Color::new(0xFF2962FF)),
        ],
    );
    const BLUE_ACCENT_PRIMARY_VALUE: u32 = 0xFF448AFF;

    /// The light blue primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.lightBlue[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIGHT_BLUE_ACCENT`](Self::LIGHT_BLUE_ACCENT), the corresponding accent colors.
    ///  * [`BLUE`](Self::BLUE) and [`CYAN`](Self::CYAN), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIGHT_BLUE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::LIGHT_BLUE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE1F5FE)),
            (100, Color::new(0xFFB3E5FC)),
            (200, Color::new(0xFF81D4FA)),
            (300, Color::new(0xFF4FC3F7)),
            (400, Color::new(0xFF29B6F6)),
            (500, Color::new(Self::LIGHT_BLUE_PRIMARY_VALUE)),
            (600, Color::new(0xFF039BE5)),
            (700, Color::new(0xFF0288D1)),
            (800, Color::new(0xFF0277BD)),
            (900, Color::new(0xFF01579B)),
        ],
    );
    const LIGHT_BLUE_PRIMARY_VALUE: u32 = 0xFF03A9F4;

    /// The light blue accent swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.lightBlueAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIGHT_BLUE`](Self::LIGHT_BLUE), the corresponding primary colors.
    ///  * [`BLUE_ACCENT`](Self::BLUE_ACCENT) and [`CYAN_ACCENT`](Self::CYAN_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIGHT_BLUE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::LIGHT_BLUE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFF80D8FF)),
            (200, Color::new(Self::LIGHT_BLUE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF00B0FF)),
            (700, Color::new(0xFF0091EA)),
        ],
    );
    const LIGHT_BLUE_ACCENT_PRIMARY_VALUE: u32 = 0xFF40C4FF;

    /// The cyan primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.cyan[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`CYAN_ACCENT`](Self::CYAN_ACCENT), the corresponding accent colors.
    ///  * [`LIGHT_BLUE`](Self::LIGHT_BLUE), [`TEAL`](Self::TEAL), and [`BLUE_GREY`](Self::BLUE_GREY), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const CYAN: MaterialColor = MaterialColor::from_static(
        Color::new(Self::CYAN_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE0F7FA)),
            (100, Color::new(0xFFB2EBF2)),
            (200, Color::new(0xFF80DEEA)),
            (300, Color::new(0xFF4DD0E1)),
            (400, Color::new(0xFF26C6DA)),
            (500, Color::new(Self::CYAN_PRIMARY_VALUE)),
            (600, Color::new(0xFF00ACC1)),
            (700, Color::new(0xFF0097A7)),
            (800, Color::new(0xFF00838F)),
            (900, Color::new(0xFF006064)),
        ],
    );
    const CYAN_PRIMARY_VALUE: u32 = 0xFF00BCD4;

    /// The cyan accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlue.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightBlueAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.cyanAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`CYAN`](Self::CYAN), the corresponding primary colors.
    ///  * [`LIGHT_BLUE_ACCENT`](Self::LIGHT_BLUE_ACCENT) and [`TEAL_ACCENT`](Self::TEAL_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const CYAN_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::CYAN_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFF84FFFF)),
            (200, Color::new(Self::CYAN_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF00E5FF)),
            (700, Color::new(0xFF00B8D4)),
        ],
    );
    const CYAN_ACCENT_PRIMARY_VALUE: u32 = 0xFF18FFFF;

    /// The teal primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.teal[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`TEAL_ACCENT`](Self::TEAL_ACCENT), the corresponding accent colors.
    ///  * [`GREEN`](Self::GREEN) and [`CYAN`](Self::CYAN), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const TEAL: MaterialColor = MaterialColor::from_static(
        Color::new(Self::TEAL_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE0F2F1)),
            (100, Color::new(0xFFB2DFDB)),
            (200, Color::new(0xFF80CBC4)),
            (300, Color::new(0xFF4DB6AC)),
            (400, Color::new(0xFF26A69A)),
            (500, Color::new(Self::TEAL_PRIMARY_VALUE)),
            (600, Color::new(0xFF00897B)),
            (700, Color::new(0xFF00796B)),
            (800, Color::new(0xFF00695C)),
            (900, Color::new(0xFF004D40)),
        ],
    );
    const TEAL_PRIMARY_VALUE: u32 = 0xFF009688;

    /// The teal accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyanAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.tealAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`TEAL`](Self::TEAL), the corresponding primary colors.
    ///  * [`GREEN_ACCENT`](Self::GREEN_ACCENT) and [`CYAN_ACCENT`](Self::CYAN_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const TEAL_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::TEAL_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFA7FFEB)),
            (200, Color::new(Self::TEAL_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF1DE9B6)),
            (700, Color::new(0xFF00BFA5)),
        ],
    );
    const TEAL_ACCENT_PRIMARY_VALUE: u32 = 0xFF64FFDA;

    /// The green primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.green[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`GREEN_ACCENT`](Self::GREEN_ACCENT), the corresponding accent colors.
    ///  * [`TEAL`](Self::TEAL), [`LIGHT_GREEN`](Self::LIGHT_GREEN), and [`LIME`](Self::LIME), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const GREEN: MaterialColor = MaterialColor::from_static(
        Color::new(Self::GREEN_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFE8F5E9)),
            (100, Color::new(0xFFC8E6C9)),
            (200, Color::new(0xFFA5D6A7)),
            (300, Color::new(0xFF81C784)),
            (400, Color::new(0xFF66BB6A)),
            (500, Color::new(Self::GREEN_PRIMARY_VALUE)),
            (600, Color::new(0xFF43A047)),
            (700, Color::new(0xFF388E3C)),
            (800, Color::new(0xFF2E7D32)),
            (900, Color::new(0xFF1B5E20)),
        ],
    );
    const GREEN_PRIMARY_VALUE: u32 = 0xFF4CAF50;

    /// The green accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.teal.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.tealAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.greenAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`GREEN`](Self::GREEN), the corresponding primary colors.
    ///  * [`TEAL_ACCENT`](Self::TEAL_ACCENT), [`LIGHT_GREEN_ACCENT`](Self::LIGHT_GREEN_ACCENT), and [`LIME_ACCENT`](Self::LIME_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const GREEN_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::GREEN_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFB9F6CA)),
            (200, Color::new(Self::GREEN_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF00E676)),
            (700, Color::new(0xFF00C853)),
        ],
    );
    const GREEN_ACCENT_PRIMARY_VALUE: u32 = 0xFF69F0AE;

    /// The light green primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.lightGreen[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIGHT_GREEN_ACCENT`](Self::LIGHT_GREEN_ACCENT), the corresponding accent colors.
    ///  * [`GREEN`](Self::GREEN) and [`LIME`](Self::LIME), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIGHT_GREEN: MaterialColor = MaterialColor::from_static(
        Color::new(Self::LIGHT_GREEN_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFF1F8E9)),
            (100, Color::new(0xFFDCEDC8)),
            (200, Color::new(0xFFC5E1A5)),
            (300, Color::new(0xFFAED581)),
            (400, Color::new(0xFF9CCC65)),
            (500, Color::new(Self::LIGHT_GREEN_PRIMARY_VALUE)),
            (600, Color::new(0xFF7CB342)),
            (700, Color::new(0xFF689F38)),
            (800, Color::new(0xFF558B2F)),
            (900, Color::new(0xFF33691E)),
        ],
    );
    const LIGHT_GREEN_PRIMARY_VALUE: u32 = 0xFF8BC34A;

    /// The light green accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.green.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.greenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.lightGreenAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIGHT_GREEN`](Self::LIGHT_GREEN), the corresponding primary colors.
    ///  * [`GREEN_ACCENT`](Self::GREEN_ACCENT) and [`LIME_ACCENT`](Self::LIME_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIGHT_GREEN_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::LIGHT_GREEN_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFCCFF90)),
            (200, Color::new(Self::LIGHT_GREEN_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFF76FF03)),
            (700, Color::new(0xFF64DD17)),
        ],
    );
    const LIGHT_GREEN_ACCENT_PRIMARY_VALUE: u32 = 0xFFB2FF59;

    /// The lime primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.lime[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIME_ACCENT`](Self::LIME_ACCENT), the corresponding accent colors.
    ///  * [`LIGHT_GREEN`](Self::LIGHT_GREEN) and [`YELLOW`](Self::YELLOW), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIME: MaterialColor = MaterialColor::from_static(
        Color::new(Self::LIME_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFF9FBE7)),
            (100, Color::new(0xFFF0F4C3)),
            (200, Color::new(0xFFE6EE9C)),
            (300, Color::new(0xFFDCE775)),
            (400, Color::new(0xFFD4E157)),
            (500, Color::new(Self::LIME_PRIMARY_VALUE)),
            (600, Color::new(0xFFC0CA33)),
            (700, Color::new(0xFFAFB42B)),
            (800, Color::new(0xFF9E9D24)),
            (900, Color::new(0xFF827717)),
        ],
    );
    const LIME_PRIMARY_VALUE: u32 = 0xFFCDDC39;

    /// The lime accent primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreen.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lightGreenAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.limeAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`LIME`](Self::LIME), the corresponding primary colors.
    ///  * [`LIGHT_GREEN_ACCENT`](Self::LIGHT_GREEN_ACCENT) and [`YELLOW_ACCENT`](Self::YELLOW_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const LIME_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::LIME_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFF4FF81)),
            (200, Color::new(Self::LIME_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFC6FF00)),
            (700, Color::new(0xFFAEEA00)),
        ],
    );
    const LIME_ACCENT_PRIMARY_VALUE: u32 = 0xFFEEFF41;

    /// The yellow primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.yellow[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`YELLOW_ACCENT`](Self::YELLOW_ACCENT), the corresponding accent colors.
    ///  * [`LIME`](Self::LIME) and [`AMBER`](Self::AMBER), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const YELLOW: MaterialColor = MaterialColor::from_static(
        Color::new(Self::YELLOW_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFFFDE7)),
            (100, Color::new(0xFFFFF9C4)),
            (200, Color::new(0xFFFFF59D)),
            (300, Color::new(0xFFFFF176)),
            (400, Color::new(0xFFFFEE58)),
            (500, Color::new(Self::YELLOW_PRIMARY_VALUE)),
            (600, Color::new(0xFFFDD835)),
            (700, Color::new(0xFFFBC02D)),
            (800, Color::new(0xFFF9A825)),
            (900, Color::new(0xFFF57F17)),
        ],
    );
    const YELLOW_PRIMARY_VALUE: u32 = 0xFFFFEB3B;

    /// The yellow accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.lime.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.limeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.yellowAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`YELLOW`](Self::YELLOW), the corresponding primary colors.
    ///  * [`LIME_ACCENT`](Self::LIME_ACCENT) and [`AMBER_ACCENT`](Self::AMBER_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const YELLOW_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::YELLOW_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFFFFF8D)),
            (200, Color::new(Self::YELLOW_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFFFEA00)),
            (700, Color::new(0xFFFFD600)),
        ],
    );
    const YELLOW_ACCENT_PRIMARY_VALUE: u32 = 0xFFFFFF00;

    /// The amber primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.amber[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`AMBER_ACCENT`](Self::AMBER_ACCENT), the corresponding accent colors.
    ///  * [`YELLOW`](Self::YELLOW) and [`ORANGE`](Self::ORANGE), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const AMBER: MaterialColor = MaterialColor::from_static(
        Color::new(Self::AMBER_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFFF8E1)),
            (100, Color::new(0xFFFFECB3)),
            (200, Color::new(0xFFFFE082)),
            (300, Color::new(0xFFFFD54F)),
            (400, Color::new(0xFFFFCA28)),
            (500, Color::new(Self::AMBER_PRIMARY_VALUE)),
            (600, Color::new(0xFFFFB300)),
            (700, Color::new(0xFFFFA000)),
            (800, Color::new(0xFFFF8F00)),
            (900, Color::new(0xFFFF6F00)),
        ],
    );
    const AMBER_PRIMARY_VALUE: u32 = 0xFFFFC107;

    /// The amber accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellow.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.yellowAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.amberAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`AMBER`](Self::AMBER), the corresponding primary colors.
    ///  * [`YELLOW_ACCENT`](Self::YELLOW_ACCENT) and [`ORANGE_ACCENT`](Self::ORANGE_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const AMBER_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::AMBER_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFFFE57F)),
            (200, Color::new(Self::AMBER_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFFFC400)),
            (700, Color::new(0xFFFFAB00)),
        ],
    );
    const AMBER_ACCENT_PRIMARY_VALUE: u32 = 0xFFFFD740;

    /// The orange primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.brown.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.orange[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`ORANGE_ACCENT`](Self::ORANGE_ACCENT), the corresponding accent colors.
    ///  * [`AMBER`](Self::AMBER), [`DEEP_ORANGE`](Self::DEEP_ORANGE), and [`BROWN`](Self::BROWN), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const ORANGE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::ORANGE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFFF3E0)),
            (100, Color::new(0xFFFFE0B2)),
            (200, Color::new(0xFFFFCC80)),
            (300, Color::new(0xFFFFB74D)),
            (400, Color::new(0xFFFFA726)),
            (500, Color::new(Self::ORANGE_PRIMARY_VALUE)),
            (600, Color::new(0xFFFB8C00)),
            (700, Color::new(0xFFF57C00)),
            (800, Color::new(0xFFEF6C00)),
            (900, Color::new(0xFFE65100)),
        ],
    );
    const ORANGE_PRIMARY_VALUE: u32 = 0xFFFF9800;

    /// The orange accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amber.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.amberAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.orangeAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`ORANGE`](Self::ORANGE), the corresponding primary colors.
    ///  * [`AMBER_ACCENT`](Self::AMBER_ACCENT) and [`DEEP_ORANGE_ACCENT`](Self::DEEP_ORANGE_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const ORANGE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::ORANGE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFFFD180)),
            (200, Color::new(Self::ORANGE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFFF9100)),
            (700, Color::new(0xFFFF6D00)),
        ],
    );
    const ORANGE_ACCENT_PRIMARY_VALUE: u32 = 0xFFFFAB40;

    /// The deep orange primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.brown.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.deepOrange[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`DEEP_ORANGE_ACCENT`](Self::DEEP_ORANGE_ACCENT), the corresponding accent colors.
    ///  * [`ORANGE`](Self::ORANGE), [`RED`](Self::RED), and [`BROWN`](Self::BROWN), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const DEEP_ORANGE: MaterialColor = MaterialColor::from_static(
        Color::new(Self::DEEP_ORANGE_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFBE9E7)),
            (100, Color::new(0xFFFFCCBC)),
            (200, Color::new(0xFFFFAB91)),
            (300, Color::new(0xFFFF8A65)),
            (400, Color::new(0xFFFF7043)),
            (500, Color::new(Self::DEEP_ORANGE_PRIMARY_VALUE)),
            (600, Color::new(0xFFF4511E)),
            (700, Color::new(0xFFE64A19)),
            (800, Color::new(0xFFD84315)),
            (900, Color::new(0xFFBF360C)),
        ],
    );
    const DEEP_ORANGE_PRIMARY_VALUE: u32 = 0xFFFF5722;

    /// The deep orange accent color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.deepOrangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orangeAccent.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.red.png)
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.redAccent.png)
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.deepOrangeAccent[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`DEEP_ORANGE`](Self::DEEP_ORANGE), the corresponding primary colors.
    ///  * [`ORANGE_ACCENT`](Self::ORANGE_ACCENT) [`RED_ACCENT`](Self::RED_ACCENT), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const DEEP_ORANGE_ACCENT: MaterialAccentColor = MaterialAccentColor::from_static(
        Color::new(Self::DEEP_ORANGE_ACCENT_PRIMARY_VALUE),
        &[
            (100, Color::new(0xFFFF9E80)),
            (200, Color::new(Self::DEEP_ORANGE_ACCENT_PRIMARY_VALUE)),
            (400, Color::new(0xFFFF3D00)),
            (700, Color::new(0xFFDD2C00)),
        ],
    );
    const DEEP_ORANGE_ACCENT_PRIMARY_VALUE: u32 = 0xFFFF6E40;

    /// The brown primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.brown.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.orange.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
    ///
    /// This swatch has no corresponding accent color and swatch.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.brown[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`ORANGE`](Self::ORANGE) and [`BLUE_GREY`](Self::BLUE_GREY), vaguely similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const BROWN: MaterialColor = MaterialColor::from_static(
        Color::new(Self::BROWN_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFEFEBE9)),
            (100, Color::new(0xFFD7CCC8)),
            (200, Color::new(0xFFBCAAA4)),
            (300, Color::new(0xFFA1887F)),
            (400, Color::new(0xFF8D6E63)),
            (500, Color::new(Self::BROWN_PRIMARY_VALUE)),
            (600, Color::new(0xFF6D4C41)),
            (700, Color::new(0xFF5D4037)),
            (800, Color::new(0xFF4E342E)),
            (900, Color::new(0xFF3E2723)),
        ],
    );
    const BROWN_PRIMARY_VALUE: u32 = 0xFF795548;

    /// The grey primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.grey.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.brown.png)
    ///
    /// This swatch has no corresponding accent swatch.
    ///
    /// This swatch, in addition to the values 50 and 100 to 900 in 100
    /// increments, also features the special values 350 and 850. The 350 value is
    /// used for raised button while pressed in light themes, and 850 is used for
    /// the background color of the dark theme. See `ThemeData.brightness`.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.grey[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`BLUE_GREY`](Self::BLUE_GREY) and [`BROWN`](Self::BROWN), somewhat similar colors.
    ///  * [`BLACK`](Self::BLACK), [`BLACK87`](Self::BLACK87), [`BLACK54`](Self::BLACK54), [`BLACK45`](Self::BLACK45), [`BLACK38`](Self::BLACK38), [`BLACK26`](Self::BLACK26), [`BLACK12`](Self::BLACK12), which
    ///    provide a different approach to showing shades of grey.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const GREY: MaterialColor = MaterialColor::from_static(
        Color::new(Self::GREY_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFFAFAFA)),
            (100, Color::new(0xFFF5F5F5)),
            (200, Color::new(0xFFEEEEEE)),
            (300, Color::new(0xFFE0E0E0)),
            (350, Color::new(0xFFD6D6D6)),
            (400, Color::new(0xFFBDBDBD)),
            (500, Color::new(Self::GREY_PRIMARY_VALUE)),
            (600, Color::new(0xFF757575)),
            (700, Color::new(0xFF616161)),
            (800, Color::new(0xFF424242)),
            (850, Color::new(0xFF303030)),
            (900, Color::new(0xFF212121)),
        ],
    );
    const GREY_PRIMARY_VALUE: u32 = 0xFF9E9E9E;

    /// The blue-grey primary color and swatch.
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blueGrey.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.grey.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.cyan.png)
    ///
    /// ![](https://flutter.github.io/assets-for-api-docs/assets/material/Colors.blue.png)
    ///
    /// This swatch has no corresponding accent swatch.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// ```dart
    /// Icon(
    ///   Icons.widgets,
    ///   color: Colors.blueGrey[400],
    /// )
    /// ```
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * [`GREY`](Self::GREY), [`CYAN`](Self::CYAN), and [`BLUE`](Self::BLUE), similar colors.
    ///  * `Theme.of`, which allows you to select colors from the current theme
    ///    rather than hard-coding colors in your build methods.
    pub const BLUE_GREY: MaterialColor = MaterialColor::from_static(
        Color::new(Self::BLUE_GREY_PRIMARY_VALUE),
        &[
            (50, Color::new(0xFFECEFF1)),
            (100, Color::new(0xFFCFD8DC)),
            (200, Color::new(0xFFB0BEC5)),
            (300, Color::new(0xFF90A4AE)),
            (400, Color::new(0xFF78909C)),
            (500, Color::new(Self::BLUE_GREY_PRIMARY_VALUE)),
            (600, Color::new(0xFF546E7A)),
            (700, Color::new(0xFF455A64)),
            (800, Color::new(0xFF37474F)),
            (900, Color::new(0xFF263238)),
        ],
    );
    const BLUE_GREY_PRIMARY_VALUE: u32 = 0xFF607D8B;

    /// The Material Design primary color swatches, excluding grey.
    pub const PRIMARIES: &'static [MaterialColor] = &[
        Self::RED,
        Self::PINK,
        Self::PURPLE,
        Self::DEEP_PURPLE,
        Self::INDIGO,
        Self::BLUE,
        Self::LIGHT_BLUE,
        Self::CYAN,
        Self::TEAL,
        Self::GREEN,
        Self::LIGHT_GREEN,
        Self::LIME,
        Self::YELLOW,
        Self::AMBER,
        Self::ORANGE,
        Self::DEEP_ORANGE,
        Self::BROWN,
        // The grey swatch is intentionally omitted because when picking a color
        // randomly from this list to colorize an application, picking grey suddenly
        // makes the app look disabled.
        Self::BLUE_GREY,
    ];

    /// The Material Design accent color swatches.
    pub const ACCENTS: &'static [MaterialAccentColor] = &[
        Self::RED_ACCENT,
        Self::PINK_ACCENT,
        Self::PURPLE_ACCENT,
        Self::DEEP_PURPLE_ACCENT,
        Self::INDIGO_ACCENT,
        Self::BLUE_ACCENT,
        Self::LIGHT_BLUE_ACCENT,
        Self::CYAN_ACCENT,
        Self::TEAL_ACCENT,
        Self::GREEN_ACCENT,
        Self::LIGHT_GREEN_ACCENT,
        Self::LIME_ACCENT,
        Self::YELLOW_ACCENT,
        Self::AMBER_ACCENT,
        Self::ORANGE_ACCENT,
        Self::DEEP_ORANGE_ACCENT,
    ];
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const PRIMARY_KEYS: [i32; 10] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900];
    const ACCENT_KEYS: [i32; 4] = [100, 200, 400, 700];

    #[test]
    fn material_color_basic_functionality() {
        let color = MaterialColor::new(
            Color::new(500),
            [
                (50, Color::new(0x00000050)),
                (100, Color::new(0x00000100)),
                (200, Color::new(0x00000200)),
                (300, Color::new(0x00000300)),
                (400, Color::new(0x00000400)),
                (500, Color::new(0x00000500)),
                (600, Color::new(0x00000600)),
                (700, Color::new(0x00000700)),
                (800, Color::new(0x00000800)),
                (900, Color::new(0x00000900)),
            ],
        );

        assert_eq!(color.primary().to_argb32(), 500);

        assert_eq!(color.get(&50).unwrap().to_argb32(), 0x00000050);
        assert_eq!(color.get(&100).unwrap().to_argb32(), 0x00000100);
        assert_eq!(color.get(&200).unwrap().to_argb32(), 0x00000200);
        assert_eq!(color.get(&300).unwrap().to_argb32(), 0x00000300);
        assert_eq!(color.get(&400).unwrap().to_argb32(), 0x00000400);
        assert_eq!(color.get(&500).unwrap().to_argb32(), 0x00000500);
        assert_eq!(color.get(&600).unwrap().to_argb32(), 0x00000600);
        assert_eq!(color.get(&700).unwrap().to_argb32(), 0x00000700);
        assert_eq!(color.get(&800).unwrap().to_argb32(), 0x00000800);
        assert_eq!(color.get(&900).unwrap().to_argb32(), 0x00000900);

        assert_eq!(color.shade50().to_argb32(), 0x00000050);
        assert_eq!(color.shade100().to_argb32(), 0x00000100);
        assert_eq!(color.shade200().to_argb32(), 0x00000200);
        assert_eq!(color.shade300().to_argb32(), 0x00000300);
        assert_eq!(color.shade400().to_argb32(), 0x00000400);
        assert_eq!(color.shade500().to_argb32(), 0x00000500);
        assert_eq!(color.shade600().to_argb32(), 0x00000600);
        assert_eq!(color.shade700().to_argb32(), 0x00000700);
        assert_eq!(color.shade800().to_argb32(), 0x00000800);
        assert_eq!(color.shade900().to_argb32(), 0x00000900);
    }

    fn distinct_shades(swatch: &ColorSwatch<i32>, keys: &[i32]) -> usize {
        keys.iter()
            .map(|key| swatch.get(key).unwrap().to_argb32())
            .collect::<HashSet<u32>>()
            .len()
    }

    #[test]
    fn colors_swatches_do_not_contain_duplicates() {
        for color in Colors::PRIMARIES {
            assert_eq!(distinct_shades(color, &PRIMARY_KEYS), PRIMARY_KEYS.len());
        }

        assert_eq!(
            distinct_shades(&Colors::GREY, &PRIMARY_KEYS),
            PRIMARY_KEYS.len()
        );

        for color in Colors::ACCENTS {
            assert_eq!(distinct_shades(color, &ACCENT_KEYS), ACCENT_KEYS.len());
        }
    }

    #[test]
    fn all_color_swatch_colors_are_opaque_and_equal_their_primary_color() {
        for color in Colors::PRIMARIES {
            assert_eq!(color.primary().to_argb32(), color.shade500().to_argb32());
            for key in PRIMARY_KEYS {
                assert_eq!(color.get(&key).unwrap().to_argb32() >> 24, 0xFF);
            }
        }

        assert_eq!(
            Colors::GREY.primary().to_argb32(),
            Colors::GREY.shade500().to_argb32()
        );
        for key in PRIMARY_KEYS {
            assert_eq!(Colors::GREY.get(&key).unwrap().to_argb32() >> 24, 0xFF);
        }

        for color in Colors::ACCENTS {
            assert_eq!(color.primary().to_argb32(), color.shade200().to_argb32());
            for key in ACCENT_KEYS {
                assert_eq!(color.get(&key).unwrap().to_argb32() >> 24, 0xFF);
            }
        }
    }

    #[test]
    fn material_color_swatch_comparison() {
        let sample_map = [(50, Color::new(0xFFE1F5FE)), (100, Color::new(0xFFB3E5FC))];
        let another_map = [(50, Color::new(0xFFE1F5FE)), (100, Color::new(0xFFB3E5FC))];
        let first = MaterialColor::new(Color::new(0), sample_map);
        let second = MaterialColor::new(Color::new(0), sample_map);
        let third = MaterialColor::new(Color::new(0), another_map);
        assert_eq!(first, second);
        assert_eq!(first, third);
    }
}
