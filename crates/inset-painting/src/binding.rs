//! Flutter counterpart: `painting/binding.dart` (`PaintingBinding`).
//!
//! Only the font collection and `systemFonts` listenable are here; images are opened through
//! the host directly, and `shaderWarmUp` waits.

use inset_embedder::{FontCollection, FontId, set_default_font_manager};
use inset_foundation::{App, ChangeNotifierData, Handle};

/// Binding for the painting library: the fonts every paragraph shapes against.
///
/// Flutter's engine owns the font manager; here the binding holds the [`FontCollection`],
/// installed once at start-up from the platform's font source
/// ([`install_platform_fonts`](Self::install_platform_fonts)) and read by `RenderParagraph`
/// through [`fonts`](Self::fonts).
#[derive(Default)]
pub struct PaintingBinding {
    fonts: Option<Handle<FontCollection>>,
    /// Flutter's `_systemFonts`. Created with the binding so paragraphs can listen.
    system_fonts: Option<Handle<ChangeNotifierData>>,
}

impl PaintingBinding {
    /// The current [`PaintingBinding`], created on first use.
    pub fn instance(app: &mut App) -> Handle<PaintingBinding> {
        let this: Handle<PaintingBinding> = app.singleton();
        if app.get(this).system_fonts.is_none() {
            let system_fonts = app.create(ChangeNotifierData::new());
            app.get_mut(this).system_fonts = Some(system_fonts);
        }
        this
    }

    /// Listenable that notifies when the available fonts on the system have changed.
    ///
    /// System fonts can change when the system installs or removes a font. To correctly
    /// reflect the change, it is important to relayout text related widgets when this
    /// happens.
    ///
    /// Objects that show text and/or measure text (e.g. via `TextPainter` or `Paragraph`)
    /// should listen to this and redraw/remeasure.
    pub fn system_fonts(self: Handle<Self>, app: &App) -> Handle<ChangeNotifierData> {
        app.get(self)
            .system_fonts
            .expect("PaintingBinding::instance creates the systemFonts listenable")
    }

    /// Flutter `handleSystemMessage` of type `fontsChange`: notifies [`system_fonts`].
    pub fn handle_system_fonts_did_change(self: Handle<Self>, app: &mut App) {
        self.system_fonts(app).notify_listeners(app);
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
    /// (`Platform::font_source`) and its default families (`Platform::default_font_families`),
    /// the way Flutter's engine asks the OS for faces.
    pub fn install_platform_fonts(self: Handle<Self>, app: &mut App) -> Handle<FontCollection> {
        let platform = app.platform();
        let source = platform.font_source();
        let families = platform.default_font_families();
        self.install_fonts(app, |fonts| {
            if let Some(source) = source {
                // txt::FontCollection::CreateSktFontCollection:
                // setDefaultFontManager(mgr, GetDefaultFontFamilies()).
                set_default_font_manager(fonts, source, &families);
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
    use inset_embedder::{
        Dispatcher, FontSource, InertDispatcher, Instant, Platform, TargetPlatform, ViewId, ViewRef,
    };
    use inset_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use valo::{Font, FontAttrs};

    #[test]
    fn fonts_are_installed_once_and_read_back() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let binding = PaintingBinding::instance(&mut app);
        assert!(!binding.has_fonts(&app));
        let fonts = binding.install_platform_fonts(&mut app);
        assert!(binding.has_fonts(&app));
        assert_eq!(binding.fonts(&app), fonts);
        assert!(
            app.get(fonts).is_empty(),
            "the inert platform has no font source"
        );
        let _ = binding.system_fonts(&app);
    }

    #[test]
    fn system_fonts_notify_reaches_a_listener() {
        use inset_foundation::{Listenable, Listener};
        use std::cell::Cell;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let binding = PaintingBinding::instance(&mut app);
        let hit = Rc::new(Cell::new(false));
        let flag = Rc::clone(&hit);
        binding
            .system_fonts(&app)
            .add_listener(&mut app, Listener::new(move |_app| flag.set(true)));
        binding.handle_system_fonts_did_change(&mut app);
        assert!(hit.get());
    }

    #[test]
    fn registering_a_font_before_any_install_creates_the_collection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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

    /// A font source that answers no family, keeping the names it was asked for.
    struct AskedFamilies(Rc<RefCell<Vec<String>>>);

    impl FontSource for AskedFamilies {
        fn family(&mut self, name: &str) -> Vec<Font> {
            self.0.borrow_mut().push(name.to_owned());
            Vec::new()
        }

        fn face_for_codepoint(&mut self, _codepoint: char, _attrs: FontAttrs) -> Option<Font> {
            None
        }
    }

    /// A platform whose fonts are its own, as a web page's are, so it names its default family
    /// itself.
    struct OwnDefaultFamily {
        asked: Rc<RefCell<Vec<String>>>,
    }

    impl Platform for OwnDefaultFamily {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::Android
        }

        fn request_frame(&self, _view: ViewId) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn dispatcher(&self) -> Arc<dyn Dispatcher> {
            Arc::new(InertDispatcher)
        }

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn font_source(&self) -> Option<Box<dyn FontSource>> {
            Some(Box::new(AskedFamilies(Rc::clone(&self.asked))))
        }

        fn default_font_families(&self) -> Vec<String> {
            vec![String::from("Roboto")]
        }
    }

    #[test]
    fn the_platforms_own_default_families_are_installed() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let platform = OwnDefaultFamily {
            asked: Rc::clone(&asked),
        };
        let cell = AppCell::with_platform(Rc::new(platform));
        let mut app = cell.borrow_mut();
        PaintingBinding::instance(&mut app).install_platform_fonts(&mut app);
        assert_eq!(*asked.borrow(), ["Roboto"]);
    }

    #[test]
    #[should_panic(expected = "text needs fonts")]
    fn reading_fonts_before_install_panics() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let binding = PaintingBinding::instance(&mut app);
        binding.fonts(&app);
    }
}
