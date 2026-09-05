//! Flutter counterpart: `widgets/icon_data.dart`.
//!
//! `IconDataProperty` (diagnostics) and the `staticIconProvider` tree-shaker annotation
//! wait; neither changes what an icon is.

use std::fmt;

/// A description of an icon fulfilled by a font glyph.
///
/// See `Icons` for a number of predefined icons available for material design applications.
///
/// In release builds, the Flutter tool only includes the glyphs used by an app's `IconData`s
/// in the font; here the font is installed whole.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IconData {
    /// The Unicode code point at which this icon is stored in the icon font.
    pub code_point: u32,
    /// The font family from which the glyph for the [`code_point`](Self::code_point) will be
    /// selected.
    pub font_family: Option<String>,
    /// The name of the package from which the font family is included.
    ///
    /// The name is used by the `Icon` widget when configuring the `TextStyle` so that the
    /// given [`font_family`](Self::font_family) is obtained from the appropriate asset.
    ///
    /// See also:
    ///
    ///  * `TextStyle`, which describes how to use fonts from other packages.
    pub font_package: Option<String>,
    /// Whether this icon should be automatically mirrored in right-to-left environments.
    ///
    /// The `Icon` widget respects this value by mirroring the icon when the `Directionality`
    /// is `TextDirection.rtl`.
    pub match_text_direction: bool,
    /// The ordered list of font families to fall back on when a glyph cannot be found in a
    /// higher priority font family.
    ///
    /// For more details, refer to the documentation of `TextStyle`.
    pub font_family_fallback: Option<Vec<String>>,
}

impl IconData {
    /// Creates icon data.
    ///
    /// Rarely used directly. Instead, consider using one of the predefined icons like the
    /// `Icons` collection.
    ///
    /// The [`font_package`](Self::font_package) argument must be non-null when using a font
    /// family that is included in a package. This is used when selecting the font.
    pub const fn new(code_point: u32) -> IconData {
        IconData {
            code_point,
            font_family: None,
            font_package: None,
            match_text_direction: false,
            font_family_fallback: None,
        }
    }

    /// Dart `IconData(fontFamily:)`.
    pub fn font_family(mut self, font_family: impl Into<String>) -> IconData {
        self.font_family = Some(font_family.into());
        self
    }

    /// Dart `IconData(fontPackage:)`.
    pub fn font_package(mut self, font_package: impl Into<String>) -> IconData {
        self.font_package = Some(font_package.into());
        self
    }

    /// Dart `IconData(matchTextDirection:)`.
    pub const fn match_text_direction(mut self, match_text_direction: bool) -> IconData {
        self.match_text_direction = match_text_direction;
        self
    }

    /// Dart `IconData(fontFamilyFallback:)`.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> IconData {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }
}

impl fmt::Debug for IconData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IconData(U+{:05X})", self.code_point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_covers_every_field_and_debug_prints_the_code_point() {
        let a = IconData::new(0xe900)
            .font_family("CupertinoIcons")
            .font_package("cupertino_icons");
        let b = IconData::new(0xe900)
            .font_family("CupertinoIcons")
            .font_package("cupertino_icons");
        assert_eq!(a, b);
        assert_ne!(a, b.clone().match_text_direction(true));
        assert_ne!(a, IconData::new(0xe901));
        assert_eq!(format!("{a:?}"), "IconData(U+0E900)");
    }
}
