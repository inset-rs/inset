//! The system's light or dark appearance, as the app was last told it. The main loop hands it each
//! theme a window reports and tells the app when it changed; the platform answers the app from it.

use std::cell::Cell;

use inset_embedder::Brightness;
use winit::window::Theme;

/// The system's appearance as the app was last told it: light until a window reports otherwise.
pub(crate) struct Appearance {
    brightness: Cell<Brightness>,
}

impl Default for Appearance {
    fn default() -> Appearance {
        Appearance {
            brightness: Cell::new(Brightness::Light),
        }
    }
}

impl Appearance {
    pub(crate) fn brightness(&self) -> Brightness {
        self.brightness.get()
    }

    /// Takes the system's `theme`; answers whether it differs from what the app was told, which
    /// the app is then to hear.
    pub(crate) fn take(&self, theme: Theme) -> bool {
        let brightness = brightness_of(theme);
        self.brightness.replace(brightness) != brightness
    }
}

fn brightness_of(theme: Theme) -> Brightness {
    match theme {
        Theme::Light => Brightness::Light,
        Theme::Dark => Brightness::Dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_is_told_a_theme_only_when_it_changes() {
        let appearance = Appearance::default();
        assert!(!appearance.take(Theme::Light), "light from the start");
        assert!(appearance.take(Theme::Dark));
        assert_eq!(appearance.brightness(), Brightness::Dark);
        assert!(!appearance.take(Theme::Dark), "told once");
    }
}
