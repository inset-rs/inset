//! Flutter counterpart: `painting/binding.dart` (`PaintingBinding`).
//!
//! Only the font collection is here; `imageCache`, `shaderWarmUp`, and the `systemFonts`
//! listenable wait.

use reveal_embedder::{FontCollection, FontId};
use reveal_foundation::{App, Handle};

/// Binding for the painting library: the fonts every paragraph shapes against.
///
/// Flutter's engine owns the font manager; here the binding holds the [`FontCollection`],
/// installed once at start-up from the platform's font source
/// ([`install_platform_fonts`](Self::install_platform_fonts)) and read by `RenderParagraph`
/// through [`fonts`](Self::fonts).
#[derive(Default)]
pub struct PaintingBinding {
    fonts: Option<Handle<FontCollection>>,
}

impl PaintingBinding {
    /// The current [`PaintingBinding`], created on first use.
    pub fn instance(app: &mut App) -> Handle<PaintingBinding> {
        app.singleton()
    }

    /// Installs the app-wide font collection, filled by `install`.
    ///
    /// The shell calls [`install_platform_fonts`](Self::install_platform_fonts) at start-up;
    /// a test, or an application that bundles its fonts, calls this instead.
    pub fn install_fonts(
        self: Handle<Self>,
        app: &mut App,
        install: impl FnOnce(&mut FontCollection),
    ) -> Handle<FontCollection> {
        let mut fonts = FontCollection::new();
        install(&mut fonts);
        let fonts = app.create(fonts);
        app.get_mut(self).fonts = Some(fonts);
        fonts
    }

    /// Installs the app-wide font collection seeded with the platform's own font lookup
    /// (`Platform::font_source`), the way Flutter's engine asks the OS for faces.
    pub fn install_platform_fonts(self: Handle<Self>, app: &mut App) -> Handle<FontCollection> {
        let source = app.platform().font_source();
        self.install_fonts(app, |fonts| {
            if let Some(source) = source {
                fonts.add_boxed_source(source);
            }
        })
    }

    /// Registers `bytes` as a face of `family` in the app-wide font collection, installing
    /// an empty collection first when the shell has not installed one.
    ///
    /// Flutter's engine loads the font assets a `pubspec.yaml` declares into the same font
    /// manager the platform's faces live in; a crate that bundles a font does this. Returns
    /// `None` when the bytes are not a font.
    pub fn register_font(
        self: Handle<Self>,
        app: &mut App,
        family: &str,
        bytes: Vec<u8>,
    ) -> Option<FontId> {
        let fonts = match app.get(self).fonts {
            Some(fonts) => fonts,
            None => self.install_fonts(app, |_| {}),
        };
        app.get_mut(fonts).register(family, bytes)
    }

    /// Whether a font collection has been installed.
    pub fn has_fonts(self: Handle<Self>, app: &App) -> bool {
        app.get(self).fonts.is_some()
    }

    /// The app-wide font collection.
    ///
    /// # Panics
    ///
    /// If none was installed.
    pub fn fonts(self: Handle<Self>, app: &App) -> Handle<FontCollection> {
        app.get(self).fonts.expect(
            "text needs fonts: call `PaintingBinding::install_fonts` (the shell installs the \
             platform's at start-up)",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_are_installed_once_and_read_back() {
        let mut app = App::new();
        let binding = PaintingBinding::instance(&mut app);
        assert!(!binding.has_fonts(&app));
        let fonts = binding.install_platform_fonts(&mut app);
        assert!(binding.has_fonts(&app));
        assert_eq!(binding.fonts(&app), fonts);
        assert!(
            app.get(fonts).is_empty(),
            "the inert platform has no font source"
        );
    }

    #[test]
    fn registering_a_font_before_any_install_creates_the_collection() {
        let mut app = App::new();
        let binding = PaintingBinding::instance(&mut app);
        assert_eq!(
            binding.register_font(&mut app, "Nonesuch", b"not a font".to_vec()),
            None
        );
        assert!(
            binding.has_fonts(&app),
            "the collection the face would have joined"
        );
    }

    #[test]
    #[should_panic(expected = "text needs fonts")]
    fn reading_fonts_before_install_panics() {
        let mut app = App::new();
        let binding = PaintingBinding::instance(&mut app);
        binding.fonts(&app);
    }
}
