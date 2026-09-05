//! dart:ui `Locale` (`platform_dispatcher.dart`).

use std::fmt;

/// An identifier used to select a user's language and formatting preferences.
///
/// This represents a [Unicode Language Identifier](https://www.unicode.org/reports/tr35/#Unicode_language_identifier)
/// (i.e. without Locale extensions), except variants are not supported.
///
/// Locales are canonicalized according to the "preferred value" entries in the
/// [IANA Language Subtag Registry](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry).
/// For example, `const Locale('he')` and `const Locale('iw')` are equal and both have the
/// [`language_code`](Self::language_code) `he`, because `iw` is a deprecated language subtag
/// that was replaced by the subtag `he`.
///
/// See also:
///
///  * `Localizations`, which provides localized resources to the widget tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale {
    /// The primary language subtag for the locale.
    ///
    /// This must not be null. It may be 'und', representing 'undefined'.
    ///
    /// This is expected to be string registered in the [IANA Language Subtag
    /// Registry](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry)
    /// with the type "language". The string specified must match the case of the
    /// string in the registry.
    ///
    /// Language subtags that are deprecated in the registry and have a preferred
    /// code are changed to their preferred code. For example, `const
    /// Locale('he')` and `const Locale('iw')` are equal, and both have the
    /// `language_code` `he`, because `iw` is a deprecated language subtag that was
    /// replaced by the subtag `he`.
    ///
    /// This must be a valid Unicode Language subtag as listed in [Unicode CLDR
    /// supplemental data](https://github.com/unicode-org/cldr/blob/main/common/validity/language.xml).
    ///
    /// See also:
    ///
    ///  * [`Locale::from_subtags`], which describes the conventions for creating
    ///    [`Locale`] objects.
    pub language_code: String,

    /// The script subtag for the locale.
    ///
    /// This may be null, indicating that there is no specified script subtag.
    ///
    /// This must be a valid Unicode Language Identifier script subtag as listed
    /// in [Unicode CLDR supplemental data](https://github.com/unicode-org/cldr/blob/main/common/validity/script.xml).
    ///
    /// See also:
    ///
    ///  * [`Locale::from_subtags`], which describes the conventions for creating
    ///    [`Locale`] objects.
    pub script_code: Option<String>,

    /// The region subtag for the locale.
    ///
    /// This may be null, indicating that there is no specified region subtag.
    ///
    /// This is expected to be string registered in the [IANA Language Subtag
    /// Registry](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry)
    /// with the type "region". The string specified must match the case of the
    /// string in the registry.
    ///
    /// Region subtags that are deprecated in the registry and have a preferred
    /// code are changed to their preferred code. For example, `const Locale('de',
    /// 'DE')` and `const Locale('de', 'DD')` are equal, and both have the
    /// `country_code` `DE`, because `DD` is a deprecated language subtag that was
    /// replaced by the subtag `DE`.
    ///
    /// See also:
    ///
    ///  * [`Locale::from_subtags`], which describes the conventions for creating
    ///    [`Locale`] objects.
    pub country_code: Option<String>,
}

impl Locale {
    /// Creates a new Locale object. The first argument is the primary language subtag,
    /// the second is the region (also referred to as 'country') subtag.
    ///
    /// For example:
    ///
    /// ```
    /// # use reveal_embedder::Locale;
    /// let swiss_french = Locale::new("fr").country_code("CH");
    /// let canadian_french = Locale::new("fr").country_code("CA");
    /// ```
    ///
    /// The primary language subtag must not be null. The region subtag is optional.
    /// When there is no region/country subtag, the parameter should be omitted or set
    /// to `None` instead of an empty string.
    ///
    /// The subtag values are _case sensitive_ and must be one of the valid subtags
    /// according to CLDR supplemental data:
    /// [language](https://github.com/unicode-org/cldr/blob/main/common/validity/language.xml),
    /// [region](https://github.com/unicode-org/cldr/blob/main/common/validity/region.xml).
    /// The primary language subtag must be at least two and at most eight lowercase letters,
    /// but not four letters. The region subtag must be two uppercase letters or three digits.
    /// See the [Unicode Language Identifier](https://www.unicode.org/reports/tr35/#Unicode_language_identifier)
    /// specification.
    ///
    /// Validity is not checked by default, but some methods may throw away invalid data.
    pub fn new(language_code: impl Into<String>) -> Locale {
        Locale {
            language_code: String::new(),
            script_code: None,
            country_code: None,
        }
        .language_code(language_code)
    }

    /// Creates a new Locale object from its language, script, and region subtags,
    /// each optional. The language defaults to `und`.
    ///
    /// The keyword arguments specify the subtags of the Locale.
    ///
    /// The subtag values are _case sensitive_ and must be valid subtags according to
    /// CLDR supplemental data:
    /// [language](https://github.com/unicode-org/cldr/blob/main/common/validity/language.xml),
    /// [script](https://github.com/unicode-org/cldr/blob/main/common/validity/script.xml) and
    /// [region](https://github.com/unicode-org/cldr/blob/main/common/validity/region.xml) for
    /// each of [`language_code`](Self::language_code), [`script_code`](Self::script_code) and
    /// [`country_code`](Self::country_code) respectively.
    ///
    /// The [`country_code`](Self::country_code) subtag is optional. When there is no
    /// country subtag, the parameter should be omitted instead of using an empty string.
    ///
    /// Validity is not checked by default, but some methods may throw away invalid data.
    pub fn from_subtags() -> Locale {
        Locale::new("und")
    }

    /// Dart `Locale.fromSubtags(languageCode:)`; a deprecated subtag becomes its
    /// preferred value.
    pub fn language_code(mut self, language_code: impl Into<String>) -> Locale {
        let language_code = language_code.into();
        debug_assert!(!language_code.is_empty());
        self.language_code = canonical_language_subtag(&language_code).to_owned();
        self
    }

    /// Dart `Locale.fromSubtags(scriptCode:)`.
    pub fn script_code(mut self, script_code: impl Into<String>) -> Locale {
        let script_code = script_code.into();
        debug_assert!(!script_code.is_empty());
        self.script_code = Some(script_code);
        self
    }

    /// Dart `Locale(languageCode, countryCode)` / `Locale.fromSubtags(countryCode:)`; a
    /// deprecated subtag becomes its preferred value, and `''` is no country at all.
    pub fn country_code(mut self, country_code: impl Into<String>) -> Locale {
        let country_code = country_code.into();
        self.country_code =
            (!country_code.is_empty()).then(|| canonical_region_subtag(&country_code).to_owned());
        self
    }

    /// Returns a syntactically valid Unicode BCP47 Locale Identifier.
    ///
    /// Some examples of such identifiers: "en", "es-419", "hi-Deva-IN" and
    /// "zh-Hans-CN". See <http://www.unicode.org/reports/tr35/> for technical
    /// details.
    pub fn to_language_tag(&self) -> String {
        self.raw_to_string('-')
    }

    fn raw_to_string(&self, separator: char) -> String {
        let mut out = self.language_code.clone();
        for subtag in [&self.script_code, &self.country_code]
            .into_iter()
            .flatten()
        {
            out.push(separator);
            out.push_str(subtag);
        }
        out
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw_to_string('_'))
    }
}

/// The preferred value of a deprecated language subtag; the subtag itself otherwise.
///
/// Mappings generated for language subtag registry as of 2019-02-27.
fn canonical_language_subtag(code: &str) -> &str {
    match code {
        "in" => "id",   // Indonesian; deprecated 1989-01-01
        "iw" => "he",   // Hebrew; deprecated 1989-01-01
        "ji" => "yi",   // Yiddish; deprecated 1989-01-01
        "jw" => "jv",   // Javanese; deprecated 2001-08-13
        "mo" => "ro",   // Moldavian, Moldovan; deprecated 2008-11-22
        "aam" => "aas", // Aramanik; deprecated 2015-02-12
        "adp" => "dz",  // Adap; deprecated 2015-02-12
        "aue" => "ktz", // ǂKxʼauǁʼein; deprecated 2015-02-12
        "ayx" => "nun", // Ayi (China); deprecated 2011-08-16
        "bgm" => "bcg", // Baga Mboteni; deprecated 2016-05-30
        "bjd" => "drl", // Bandjigali; deprecated 2012-08-12
        "ccq" => "rki", // Chaungtha; deprecated 2012-08-12
        "cjr" => "mom", // Chorotega; deprecated 2010-03-11
        "cka" => "cmr", // Khumi Awa Chin; deprecated 2012-08-12
        "cmk" => "xch", // Chimakum; deprecated 2010-03-11
        "coy" => "pij", // Coyaima; deprecated 2016-05-30
        "cqu" => "quh", // Chilean Quechua; deprecated 2016-05-30
        "drh" => "khk", // Darkhat; deprecated 2010-03-11
        "drw" => "prs", // Darwazi; deprecated 2010-03-11
        "gav" => "dev", // Gabutamon; deprecated 2010-03-11
        "gfx" => "vaj", // Mangetti Dune ǃXung; deprecated 2015-02-12
        "ggn" => "gvr", // Eastern Gurung; deprecated 2016-05-30
        "gti" => "nyc", // Gbati-ri; deprecated 2015-02-12
        "guv" => "duz", // Gey; deprecated 2016-05-30
        "hrr" => "jal", // Horuru; deprecated 2012-08-12
        "ibi" => "opa", // Ibilo; deprecated 2012-08-12
        "ilw" => "gal", // Talur; deprecated 2013-09-10
        "jeg" => "oyb", // Jeng; deprecated 2017-02-23
        "kgc" => "tdf", // Kasseng; deprecated 2016-05-30
        "kgh" => "kml", // Upper Tanudan Kalinga; deprecated 2012-08-12
        "koj" => "kwv", // Sara Dunjo; deprecated 2015-02-12
        "krm" => "bmf", // Krim; deprecated 2017-02-23
        "ktr" => "dtp", // Kota Marudu Tinagas; deprecated 2016-05-30
        "kvs" => "gdj", // Kunggara; deprecated 2016-05-30
        "kwq" => "yam", // Kwak; deprecated 2015-02-12
        "kxe" => "tvd", // Kakihum; deprecated 2015-02-12
        "kzj" => "dtp", // Coastal Kadazan; deprecated 2016-05-30
        "kzt" => "dtp", // Tambunan Dusun; deprecated 2016-05-30
        "lii" => "raq", // Lingkhim; deprecated 2015-02-12
        "lmm" => "rmx", // Lamam; deprecated 2014-02-28
        "meg" => "cir", // Mea; deprecated 2013-09-10
        "mst" => "mry", // Cataelano Mandaya; deprecated 2010-03-11
        "mwj" => "vaj", // Maligo; deprecated 2015-02-12
        "myt" => "mry", // Sangab Mandaya; deprecated 2010-03-11
        "nad" => "xny", // Nijadali; deprecated 2016-05-30
        "ncp" => "kdz", // Ndaktup; deprecated 2018-03-08
        "nnx" => "ngv", // Ngong; deprecated 2015-02-12
        "nts" => "pij", // Natagaimas; deprecated 2016-05-30
        "oun" => "vaj", // ǃOǃung; deprecated 2015-02-12
        "pcr" => "adx", // Panang; deprecated 2013-09-10
        "pmc" => "huw", // Palumata; deprecated 2016-05-30
        "pmu" => "phr", // Mirpur Panjabi; deprecated 2015-02-12
        "ppa" => "bfy", // Pao; deprecated 2016-05-30
        "ppr" => "lcq", // Piru; deprecated 2013-09-10
        "pry" => "prt", // Pray 3; deprecated 2016-05-30
        "puz" => "pub", // Purum Naga; deprecated 2014-02-28
        "sca" => "hle", // Sansu; deprecated 2012-08-12
        "skk" => "oyb", // Sok; deprecated 2017-02-23
        "tdu" => "dtp", // Tempasuk Dusun; deprecated 2016-05-30
        "thc" => "tpo", // Tai Hang Tong; deprecated 2016-05-30
        "thx" => "oyb", // The; deprecated 2015-02-12
        "tie" => "ras", // Tingal; deprecated 2011-08-16
        "tkk" => "twm", // Takpa; deprecated 2011-08-16
        "tlw" => "weo", // South Wemale; deprecated 2012-08-12
        "tmp" => "tyj", // Tai Mène; deprecated 2016-05-30
        "tne" => "kak", // Tinoc Kallahan; deprecated 2016-05-30
        "tnf" => "prs", // Tangshewi; deprecated 2010-03-11
        "tsf" => "taj", // Southwestern Tamang; deprecated 2015-02-12
        "uok" => "ema", // Uokha; deprecated 2015-02-12
        "xba" => "cax", // Kamba (Brazil); deprecated 2016-05-30
        "xia" => "acn", // Xiandao; deprecated 2013-09-10
        "xkh" => "waw", // Karahawyana; deprecated 2016-05-30
        "xsj" => "suj", // Subi; deprecated 2015-02-12
        "ybd" => "rki", // Yangbye; deprecated 2012-08-12
        "yma" => "lrr", // Yamphe; deprecated 2012-08-12
        "ymt" => "mtm", // Mator-Taygi-Karagas; deprecated 2015-02-12
        "yos" => "zom", // Yos; deprecated 2013-09-10
        "yuu" => "yug", // Yugh; deprecated 2014-02-28
        _ => code,
    }
}

/// The preferred value of a deprecated region subtag; the subtag itself otherwise.
///
/// Mappings generated for language subtag registry as of 2019-02-27.
fn canonical_region_subtag(code: &str) -> &str {
    match code {
        "BU" => "MM", // Burma; deprecated 1989-12-05
        "DD" => "DE", // German Democratic Republic; deprecated 1990-10-30
        "FX" => "FR", // Metropolitan France; deprecated 1997-07-14
        "TP" => "TL", // East Timor; deprecated 2002-05-20
        "YD" => "YE", // Democratic Yemen; deprecated 1990-08-14
        "ZR" => "CD", // Zaire; deprecated 1997-07-14
        _ => code,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn deprecated_subtags_canonicalize_to_their_preferred_values() {
        let hebrew = Locale::new("iw");
        assert_eq!(hebrew.language_code, "he");
        assert_eq!(hebrew, Locale::new("he"));
        let east_germany = Locale::new("de").country_code("DD");
        assert_eq!(east_germany.country_code.as_deref(), Some("DE"));
        assert_eq!(east_germany, Locale::new("de").country_code("DE"));
    }

    #[test]
    fn formatting_joins_the_subtags_with_the_separator() {
        let locale = Locale::from_subtags()
            .language_code("zh")
            .script_code("Hans")
            .country_code("CN");
        assert_eq!(locale.to_string(), "zh_Hans_CN");
        assert_eq!(locale.to_language_tag(), "zh-Hans-CN");
        assert_eq!(Locale::new("en").to_string(), "en");
        assert_eq!(Locale::from_subtags().to_string(), "und");
    }

    #[test]
    fn a_locale_equals_and_hashes_by_its_canonical_subtags() {
        let mut set = HashSet::new();
        set.insert(Locale::new("fr").country_code("CH"));
        assert!(set.contains(&Locale::new("fr").country_code("CH")));
        assert!(!set.contains(&Locale::new("fr")));
        assert_eq!(Locale::new("fr").country_code(""), Locale::new("fr"));
        assert_ne!(
            Locale::new("sr").script_code("Latn"),
            Locale::new("sr").script_code("Cyrl")
        );
    }
}
