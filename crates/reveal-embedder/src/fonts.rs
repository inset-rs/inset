//! The font collection a paragraph shapes against. Flutter's engine owns one per process and
//! asks the OS for faces; here the collection is the host's [`FontCollection`], filled from
//! [`Platform::font_source`](crate::Platform::font_source) and whatever the application
//! registers, and handed to [`ParagraphBuilder::build`](crate::ParagraphBuilder::build).

use valo::{Font, FontAttrs};
pub use valo::{FontCollection, FontDemand, FontId, FontSource};
use valo_system_fonts::SystemFonts;
pub use valo_system_fonts::fontmgr::{FontManager, Typeface};

/// The engine's font manager: the platform's installed fonts through a [`FontManager`], and
/// the two family names Flutter's engine defines for the platform's own user-interface font.
///
/// `CupertinoSystemText` is that font at text sizes and `CupertinoSystemDisplay` at display
/// sizes, each answered with every weight the platform has, as the engine registers them
/// (`txt/platform_mac.mm`: the display face at its 29-point breakpoint, the text face by
/// the default fallback). Every other family, and character fallback, goes to the manager.
pub struct SystemFontSource {
    fonts: SystemFonts,
}

/// Flutter's `CupertinoSystemText`, answered by the engine's default fallback at text sizes.
const CUPERTINO_TEXT: &str = "CupertinoSystemText";
/// Flutter's `CupertinoSystemDisplay`, the system font at and above the display breakpoint.
const CUPERTINO_DISPLAY: &str = "CupertinoSystemDisplay";
/// The point size the engine's default text-size lookup is made at.
const TEXT_SIZE: f32 = 17.0;
/// The engine's `kSFProDisplayBreakPoint`.
const DISPLAY_SIZE: f32 = 29.0;

impl SystemFontSource {
    /// Over `manager`: a host's own lookup, or one that answers across a boundary.
    pub fn new(manager: Box<dyn FontManager>) -> SystemFontSource {
        SystemFontSource {
            fonts: SystemFonts::with_manager(manager),
        }
    }

    /// Over this platform's own font manager.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn platform() -> SystemFontSource {
        SystemFontSource {
            fonts: SystemFonts::load(),
        }
    }

    /// The languages character fallback prefers, BCP 47, most preferred first (Flutter's
    /// `PlatformDispatcher.locales`).
    pub fn set_locales(&mut self, locales: Vec<String>) {
        self.fonts.set_locales(locales);
    }
}

impl FontSource for SystemFontSource {
    fn family(&mut self, name: &str) -> Vec<Font> {
        match name {
            CUPERTINO_TEXT => self.fonts.system_family(TEXT_SIZE),
            CUPERTINO_DISPLAY => self.fonts.system_family(DISPLAY_SIZE),
            _ => self.fonts.family(name),
        }
    }

    fn face_for_codepoint(&mut self, codepoint: char, attrs: FontAttrs) -> Option<Font> {
        self.fonts.face_for_codepoint(codepoint, attrs)
    }
}

/// Flutter `txt::GetDefaultFontFamilies()`: the family names
/// `FontCollection::setDefaultFontManager` looks up when a style leaves `fontFamily` unset.
///
/// On Apple, Flutter reads `[systemFontOfSize:14].familyName`. That face is what this source
/// answers as `CupertinoSystemText`, so that is the name the default manager is given.
pub fn default_font_families() -> Vec<String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        vec![CUPERTINO_TEXT.to_owned()]
    }
    #[cfg(target_os = "android")]
    {
        vec!["sans-serif".to_owned()]
    }
    #[cfg(target_os = "windows")]
    {
        vec!["Segoe UI".to_owned(), "Arial".to_owned()]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "Ubuntu".to_owned(),
            "Adwaita Sans".to_owned(),
            "Cantarell".to_owned(),
            "DejaVu Sans".to_owned(),
            "Liberation Sans".to_owned(),
            "Arial".to_owned(),
        ]
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "android",
        target_os = "windows",
        target_os = "linux",
    )))]
    {
        vec!["Arial".to_owned()]
    }
}

/// Flutter `skia::textlayout::FontCollection::setDefaultFontManager`: register the default
/// family's faces as the collection's fallback chain, then keep `source` for later lookups.
///
/// Valo has no default font manager. An empty style `families` list walks this fallback
/// chain, then `FontId(0)`. Without the chain, a later `register` (the Cupertino icon font)
/// becomes font 0 and unspecified-family text paints `.notdef`.
pub fn set_default_font_manager(fonts: &mut FontCollection, mut source: Box<dyn FontSource>) {
    for name in default_font_families() {
        for mut font in source.family(&name) {
            font.add_alias(&name);
            let id = fonts.add(font);
            fonts.add_fallback(id);
        }
    }
    fonts.add_boxed_source(source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_families_match_the_engine_on_this_os() {
        let names = default_font_families();
        assert!(!names.is_empty());
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        assert_eq!(names, vec![CUPERTINO_TEXT]);
        #[cfg(target_os = "android")]
        assert_eq!(names, vec!["sans-serif"]);
    }
}
