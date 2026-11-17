//! flutter_test's fonts: Flutter's engine `txt::TestFontManager` over the faces of
//! `runtime/test_font_data.cc`, of which Waft carries FlutterTest alone (no Ahem, no Cough).
//!
//! `assets/FlutterTest.ttf` is the engine's `kFlutterTestFont` byte for byte, which
//! `tools/gen_test_font.py` generates: a 1024 unit em, ascent 768 and descent 256, no line gap,
//! and every glyph an em wide, most of them a square filling the em. Text in it at a font size
//! measures one font size per glyph, and a line one font size high.

use crate::FontSource;
use valo::{FaceSet, Font, FontAttrs};

/// FlutterTest's bytes: the engine's `kFlutterTestFont`.
const FLUTTER_TEST_FONT: &[u8] = include_bytes!("../assets/FlutterTest.ttf");

/// The engine's `GetTestFontFamilyNames()[0]`: the family every name the test fonts do not
/// carry answers with.
const FLUTTER_TEST: &str = "FlutterTest";

/// Flutter's engine `TestFontManager` as `FontCollection::RegisterTestFonts` installs it: every
/// family answers with FlutterTest, and font fallback is off.
///
/// The engine asks it after the fonts an application registers, as a collection asks its
/// sources after its registered faces, and before the OS's, which it leaves unreached by
/// answering every name.
pub struct TestFontSource {
    flutter_test: Font,
}

impl TestFontSource {
    /// Over FlutterTest, registered under that family as the engine's
    /// `TypefaceFontAssetProvider::RegisterTypeface` names it; the file's own family is the
    /// generator's FreeType hinting name.
    pub fn new() -> TestFontSource {
        let mut faces = FaceSet::new();
        let id = faces
            .register(FLUTTER_TEST, FLUTTER_TEST_FONT.to_vec())
            .expect("FlutterTest is a TrueType font");
        TestFontSource {
            flutter_test: faces.get(id).clone(),
        }
    }
}

impl Default for TestFontSource {
    fn default() -> TestFontSource {
        TestFontSource::new()
    }
}

impl FontSource for TestFontSource {
    /// Flutter `TestFontManager::onMatchFamily`: the requested test family, else the first,
    /// FlutterTest. FlutterTest is the only one here, so every name answers with it.
    fn family(&mut self, _name: &str) -> Vec<Font> {
        vec![self.flutter_test.clone()]
    }

    /// `RegisterTestFonts` disables font fallback: a character FlutterTest has no glyph for
    /// draws FlutterTest's `.notdef`, a box an em wide.
    fn face_for_codepoint(&mut self, _codepoint: char, _attrs: FontAttrs) -> Option<Font> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        FontCollection, Paragraph, ParagraphBuilder, ParagraphConstraints, ParagraphStyle,
        TextStyle, set_default_font_manager,
    };

    use super::*;

    /// `text` at `font_size`, in a collection whose default family is `default_family`, the
    /// way the shell fills one from a platform's font source.
    fn laid_out(text: &str, style: TextStyle, default_family: &str) -> Paragraph {
        let mut fonts = FontCollection::new();
        set_default_font_manager(
            &mut fonts,
            Box::new(TestFontSource::new()),
            &[default_family.to_owned()],
        );
        let mut builder = ParagraphBuilder::new(ParagraphStyle::new());
        builder.push_style(style);
        builder.add_text(text);
        let mut paragraph = builder.build(&mut fonts);
        paragraph.layout(ParagraphConstraints::new(f64::INFINITY));
        paragraph
    }

    fn at_size(font_size: f64) -> TextStyle {
        TextStyle {
            font_size: Some(font_size),
            ..TextStyle::default()
        }
    }

    #[test]
    fn the_font_is_the_generators_flutter_test() {
        let font = TestFontSource::new().family(FLUTTER_TEST).remove(0);
        assert!(font.matches(FLUTTER_TEST));
        assert_eq!(font.units_per_em(), 1024.0);
        assert_eq!(font.ascent_px(1024.0), 768.0);
        assert_eq!(font.descent_px(1024.0), 256.0);
        assert_eq!(font.line_height_px(1024.0), 1024.0);
        for ch in ['T', 'I', 'P', 'p', 'x', '\u{c9}', ' ', '\u{4e2d}'] {
            assert!(font.covers(ch), "FlutterTest has a glyph for {ch:?}");
        }
    }

    #[test]
    fn every_family_answers_with_flutter_test() {
        let mut source = TestFontSource::new();
        for name in [FLUTTER_TEST, "Ahem", "Roboto", "CupertinoSystemText"] {
            let faces = source.family(name);
            assert_eq!(faces.len(), 1);
            assert!(
                faces[0].matches(FLUTTER_TEST),
                "{name} answers with FlutterTest"
            );
        }
        assert!(
            source
                .face_for_codepoint('a', FontAttrs::default())
                .is_none()
        );
    }

    #[test]
    fn a_glyph_is_a_square_of_the_font_size() {
        let paragraph = laid_out("TIP", at_size(14.0), "Roboto");
        assert_eq!(paragraph.longest_line(), 42.0);
        assert_eq!(paragraph.height(), 14.0);
        assert_eq!(paragraph.alphabetic_baseline(), 10.5);
    }

    #[test]
    fn a_named_family_flutter_test_lacks_is_flutter_test() {
        let style = TextStyle {
            font_family: Some("Roboto".into()),
            ..at_size(10.0)
        };
        let paragraph = laid_out("Hello", style, "CupertinoSystemText");
        assert_eq!(paragraph.longest_line(), 50.0);
        assert_eq!(paragraph.height(), 10.0);
    }
}
