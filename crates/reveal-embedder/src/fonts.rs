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
