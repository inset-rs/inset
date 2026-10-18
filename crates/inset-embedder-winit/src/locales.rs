//! The user's languages, as the app was last told them (`Platform::locales`): the system's list,
//! most preferred first, read as the app starts, and again when the system says it changed, which
//! only macOS says (`system_changes.rs`).
//!
//! Every system lists them as language tags, which `sys_locale` reads: macOS and iOS their
//! preferred languages, as `NSLocale.preferredLanguages` lists them, Windows the user's preferred
//! UI languages, and the other systems the `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG`
//! variables, which are what GLib reads for Flutter's Linux embedder.

use std::cell::RefCell;

use inset_embedder::Locale;

/// The user's languages as the app was last told them.
pub(crate) struct Locales {
    locales: RefCell<Vec<Locale>>,
}

impl Locales {
    /// The system's languages now.
    pub(crate) fn read() -> Locales {
        Locales {
            locales: RefCell::new(system_locales()),
        }
    }

    pub(crate) fn locales(&self) -> Vec<Locale> {
        self.locales.borrow().clone()
    }

    /// Reads the system's languages again; answers whether they differ from what the app was
    /// told, which the app is then to hear.
    pub(crate) fn read_again(&self) -> bool {
        let locales = system_locales();
        if *self.locales.borrow() == locales {
            return false;
        }
        *self.locales.borrow_mut() = locales;
        true
    }
}

fn system_locales() -> Vec<Locale> {
    locales_of(sys_locale::get_locales())
}

/// The locales `tags` name, most preferred first, each once: two tags that differ only in what a
/// locale leaves out, such as `de-DE` and `de-DE-u-co-phonebk`, name the same one.
fn locales_of(tags: impl IntoIterator<Item = String>) -> Vec<Locale> {
    let mut locales = Vec::new();
    for locale in tags.into_iter().filter_map(|tag| locale_of(&tag)) {
        if !locales.contains(&locale) {
            locales.push(locale);
        }
    }
    locales
}

/// The locale a BCP 47 language tag names, such as `zh-Hans-CN`: its language subtag, and the
/// script and region subtags after it, told apart by their shape as `NSLocale` tells them. What a
/// locale has no place for, variants and extensions, is left out. `None` for an empty tag.
fn locale_of(tag: &str) -> Option<Locale> {
    let mut subtags = tag.split('-');
    let language = subtags.next().filter(|language| !language.is_empty())?;
    // A one-letter subtag starts an extension or a private use, which run to the end.
    let mut rest = subtags.take_while(|subtag| subtag.len() > 1).peekable();
    let mut locale = Locale::new(language);
    if let Some(script) = rest.next_if(|subtag| is_script(subtag)) {
        locale = locale.script_code(script);
    }
    if let Some(region) = rest.next_if(|subtag| is_region(subtag)) {
        locale = locale.country_code(region);
    }
    Some(locale)
}

/// Whether `subtag` has a script's shape: four letters, `Hans`.
fn is_script(subtag: &str) -> bool {
    subtag.len() == 4 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

/// Whether `subtag` has a region's shape: two letters, `CN`, or three digits, `419`.
fn is_region(subtag: &str) -> bool {
    (subtag.len() == 2 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()))
        || (subtag.len() == 3 && subtag.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_names_its_language_script_and_region() {
        assert_eq!(
            locale_of("en-US"),
            Some(Locale::new("en").country_code("US"))
        );
        assert_eq!(
            locale_of("zh-Hans-CN"),
            Some(Locale::new("zh").script_code("Hans").country_code("CN"))
        );
        assert_eq!(
            locale_of("sr-Latn"),
            Some(Locale::new("sr").script_code("Latn"))
        );
        assert_eq!(
            locale_of("es-419"),
            Some(Locale::new("es").country_code("419"))
        );
        assert_eq!(locale_of("fr"), Some(Locale::new("fr")));
    }

    #[test]
    fn what_a_locale_has_no_place_for_is_left_out() {
        assert_eq!(
            locale_of("de-DE-u-co-phonebk"),
            Some(Locale::new("de").country_code("DE"))
        );
        assert_eq!(locale_of("en-x-private"), Some(Locale::new("en")));
        assert_eq!(
            locale_of("ca-ES-valencia"),
            Some(Locale::new("ca").country_code("ES"))
        );
        assert_eq!(locale_of(""), None);
    }

    #[test]
    fn the_same_locale_is_listed_once_in_the_order_preferred() {
        let tags = ["en-GB", "fr", "en-GB", "de-DE-u-co-phonebk", "de-DE"];
        assert_eq!(
            locales_of(tags.map(String::from)),
            vec![
                Locale::new("en").country_code("GB"),
                Locale::new("fr"),
                Locale::new("de").country_code("DE"),
            ]
        );
    }
}
