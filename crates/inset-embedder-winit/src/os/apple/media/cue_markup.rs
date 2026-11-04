//! A subtitle as AVFoundation's legible output hands it, an attributed string whose attributes are
//! Core Media's text markup (`CMTextMarkup.h`), as the framework's [`Cue`]. A run of the string
//! with one set of attributes is a [`CueTextRun`]; the attributes `CMTextMarkup.h` says cover the
//! whole string (alignment, places, size, vertical layout, the box's colour) are read from its
//! first character. Where the markup gives no value, the field is left unsaid, as `Cue::new` and
//! `CueTextRun::new` leave it, for the app to fill in from the user's caption style: the legible
//! output hands over the media's own styling alone (`cues`).
//!
//! Core Media gives a cue's place as the centre of its text across and of the cue down
//! (`TextPositionPercentageRelativeToWritingDirection`,
//! `OrthogonalLinePositionPercentageRelativeToWritingDirection`), so both anchors are the middle.

use inset_embedder::{
    Color, ColorSpace, Cue, CueAnchor, CueContent, CueLine, CueTextAlignment, CueTextEdge,
    CueTextRun, CueVertical,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_media::{
    kCMTextMarkupAlignmentType_End, kCMTextMarkupAlignmentType_Left,
    kCMTextMarkupAlignmentType_Right, kCMTextMarkupAlignmentType_Start,
    kCMTextMarkupAttribute_Alignment, kCMTextMarkupAttribute_BackgroundColorARGB,
    kCMTextMarkupAttribute_BaseFontSizePercentageRelativeToVideoHeight,
    kCMTextMarkupAttribute_BoldStyle, kCMTextMarkupAttribute_CharacterBackgroundColorARGB,
    kCMTextMarkupAttribute_CharacterEdgeStyle, kCMTextMarkupAttribute_FontFamilyName,
    kCMTextMarkupAttribute_ForegroundColorARGB, kCMTextMarkupAttribute_ItalicStyle,
    kCMTextMarkupAttribute_OrthogonalLinePositionPercentageRelativeToWritingDirection,
    kCMTextMarkupAttribute_RelativeFontSize,
    kCMTextMarkupAttribute_TextPositionPercentageRelativeToWritingDirection,
    kCMTextMarkupAttribute_UnderlineStyle, kCMTextMarkupAttribute_VerticalLayout,
    kCMTextMarkupAttribute_WritingDirectionSizePercentage,
    kCMTextMarkupCharacterEdgeStyle_Depressed, kCMTextMarkupCharacterEdgeStyle_DropShadow,
    kCMTextMarkupCharacterEdgeStyle_None, kCMTextMarkupCharacterEdgeStyle_Raised,
    kCMTextMarkupCharacterEdgeStyle_Uniform, kCMTextVerticalLayout_LeftToRight,
    kCMTextVerticalLayout_RightToLeft,
};
use objc2_foundation::{NSArray, NSAttributedString, NSDictionary, NSNumber, NSRange, NSString};

use super::as_ns_string;

/// The attributes of a run of an attributed string.
type Attributes = NSDictionary<NSString, AnyObject>;

/// A Core Media markup constant as the NSString key or value it is.
macro_rules! markup {
    ($constant:ident) => {
        // SAFETY: a constant Core Media exports for the life of the process.
        as_ns_string(unsafe { $constant })
    };
}

/// `string`, a subtitle from AVFoundation's legible output, as a cue.
pub(super) fn cue_of(string: &NSAttributedString) -> Cue {
    let mut cue = Cue::new(CueContent::Text(runs_of(string)));
    if string.length() == 0 {
        return cue;
    }
    // SAFETY: an index inside the string, and no range asked for.
    let whole = unsafe { string.attributesAtIndex_effectiveRange(0, std::ptr::null_mut()) };
    place(&mut cue, &whole);
    cue.text_alignment = text_alignment_of(&whole);
    cue.vertical = vertical_of(&whole);
    cue.window_color = color(&whole, markup!(kCMTextMarkupAttribute_BackgroundColorARGB));
    cue
}

/// Each run of `string` with one set of attributes, in order.
fn runs_of(string: &NSAttributedString) -> Vec<CueTextRun> {
    let text = string.string();
    let length = string.length();
    let mut runs = Vec::new();
    let mut location = 0;
    while location < length {
        let mut range = NSRange::new(location, 0);
        // SAFETY: an index inside the string, and a range to write the run's into.
        let attributes = unsafe { string.attributesAtIndex_effectiveRange(location, &mut range) };
        let run_text = text.substringWithRange(range);
        runs.push(run_of(run_text.to_string(), &attributes));
        location = range.location + range.length.max(1);
    }
    runs
}

/// `text` in the style `attributes` give it.
fn run_of(text: String, attributes: &Attributes) -> CueTextRun {
    let mut run = CueTextRun::new(text);
    run.bold = flag(attributes, markup!(kCMTextMarkupAttribute_BoldStyle));
    run.italic = flag(attributes, markup!(kCMTextMarkupAttribute_ItalicStyle));
    run.underline = flag(attributes, markup!(kCMTextMarkupAttribute_UnderlineStyle));
    run.color = color(
        attributes,
        markup!(kCMTextMarkupAttribute_ForegroundColorARGB),
    );
    run.background = color(
        attributes,
        markup!(kCMTextMarkupAttribute_CharacterBackgroundColorARGB),
    );
    run.font_size = font_size_of(attributes);
    run.font_family = string(attributes, markup!(kCMTextMarkupAttribute_FontFamilyName))
        .map(|family| family.to_string());
    run.edge = edge_of(attributes);
    run
}

/// Where the cue's box goes: its line and position at their middles, and its width.
fn place(cue: &mut Cue, whole: &Attributes) {
    let fraction = |key| number(whole, key).map(|percent| (percent / 100.0) as f32);
    if let Some(line) = fraction(markup!(
        kCMTextMarkupAttribute_OrthogonalLinePositionPercentageRelativeToWritingDirection
    )) {
        cue.line = Some(CueLine::Fraction(line));
        cue.line_anchor = CueAnchor::Middle;
    }
    if let Some(position) = fraction(markup!(
        kCMTextMarkupAttribute_TextPositionPercentageRelativeToWritingDirection
    )) {
        cue.position = Some(position);
        cue.position_anchor = CueAnchor::Middle;
    }
    cue.size = fraction(markup!(
        kCMTextMarkupAttribute_WritingDirectionSizePercentage
    ));
}

/// The text's height as a fraction of the picture's: the base size, a percentage of the video's
/// height, times the relative size, a percentage of the base, which is the default height where
/// the markup gives none; `None` where it gives neither.
fn font_size_of(attributes: &Attributes) -> Option<f32> {
    let base = number(
        attributes,
        markup!(kCMTextMarkupAttribute_BaseFontSizePercentageRelativeToVideoHeight),
    )
    .map(|percent| percent / 100.0);
    let relative = number(attributes, markup!(kCMTextMarkupAttribute_RelativeFontSize))
        .map(|percent| percent / 100.0);
    if base.is_none() && relative.is_none() {
        return None;
    }
    let base = base.unwrap_or(f64::from(CueTextRun::DEFAULT_FONT_SIZE));
    Some((base * relative.unwrap_or(1.0)) as f32)
}

/// The cue's alignment; the middle where the markup gives none, as `CMTextMarkup.h` says. Left
/// and right are the start and the end, as Media3 reads WebVTT's.
fn text_alignment_of(whole: &Attributes) -> CueTextAlignment {
    let Some(alignment) = string(whole, markup!(kCMTextMarkupAttribute_Alignment)) else {
        return CueTextAlignment::Center;
    };
    let is = |value: &NSString| &*alignment == value;
    if is(markup!(kCMTextMarkupAlignmentType_Start)) || is(markup!(kCMTextMarkupAlignmentType_Left))
    {
        CueTextAlignment::Start
    } else if is(markup!(kCMTextMarkupAlignmentType_End))
        || is(markup!(kCMTextMarkupAlignmentType_Right))
    {
        CueTextAlignment::End
    } else {
        CueTextAlignment::Center
    }
}

fn vertical_of(whole: &Attributes) -> Option<CueVertical> {
    let layout = string(whole, markup!(kCMTextMarkupAttribute_VerticalLayout))?;
    if &*layout == markup!(kCMTextVerticalLayout_LeftToRight) {
        Some(CueVertical::LeftToRight)
    } else if &*layout == markup!(kCMTextVerticalLayout_RightToLeft) {
        Some(CueVertical::RightToLeft)
    } else {
        None
    }
}

/// The edge round a run's characters; a uniform edge is an outline. `None` where the markup
/// names none it knows.
fn edge_of(attributes: &Attributes) -> Option<CueTextEdge> {
    let edge = string(
        attributes,
        markup!(kCMTextMarkupAttribute_CharacterEdgeStyle),
    )?;
    let is = |value: &NSString| &*edge == value;
    if is(markup!(kCMTextMarkupCharacterEdgeStyle_None)) {
        Some(CueTextEdge::None)
    } else if is(markup!(kCMTextMarkupCharacterEdgeStyle_Uniform)) {
        Some(CueTextEdge::Outline)
    } else if is(markup!(kCMTextMarkupCharacterEdgeStyle_DropShadow)) {
        Some(CueTextEdge::DropShadow)
    } else if is(markup!(kCMTextMarkupCharacterEdgeStyle_Raised)) {
        Some(CueTextEdge::Raised)
    } else if is(markup!(kCMTextMarkupCharacterEdgeStyle_Depressed)) {
        Some(CueTextEdge::Depressed)
    } else {
        None
    }
}

fn value(attributes: &Attributes, key: &NSString) -> Option<Retained<AnyObject>> {
    attributes.objectForKey(key)
}

fn number(attributes: &Attributes, key: &NSString) -> Option<f64> {
    let value = value(attributes, key)?;
    value.downcast_ref::<NSNumber>().map(NSNumber::doubleValue)
}

/// A boolean attribute: a `CFBoolean`, which is an `NSNumber`; false where it is absent.
fn flag(attributes: &Attributes, key: &NSString) -> bool {
    value(attributes, key)
        .and_then(|value| value.downcast_ref::<NSNumber>().map(NSNumber::boolValue))
        .unwrap_or(false)
}

fn string(attributes: &Attributes, key: &NSString) -> Option<Retained<NSString>> {
    value(attributes, key)?.downcast::<NSString>().ok()
}

/// A colour attribute: four numbers from 0 to 1, alpha, red, green and blue, in sRGB.
fn color(attributes: &Attributes, key: &NSString) -> Option<Color> {
    let value = value(attributes, key)?;
    let channels = value.downcast_ref::<NSArray>()?;
    let channel = |index| {
        channels
            .objectAtIndex(index)
            .downcast_ref::<NSNumber>()
            .map(NSNumber::doubleValue)
    };
    if channels.count() != 4 {
        return None;
    }
    Some(Color::from(
        channel(0)?,
        channel(1)?,
        channel(2)?,
        channel(3)?,
        ColorSpace::Srgb,
    ))
}

#[cfg(test)]
mod tests {
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSCopying, NSMutableAttributedString, NSNumber};

    use super::*;

    /// A run of `text` with `attributes`, each a markup key and its value.
    fn run(
        text: &str,
        attributes: &[(&NSString, Retained<AnyObject>)],
    ) -> Retained<NSAttributedString> {
        let keys: Vec<&NSString> = attributes.iter().map(|(key, _)| *key).collect();
        let values: Vec<&AnyObject> = attributes.iter().map(|(_, value)| &**value).collect();
        let attributes = NSDictionary::from_slices(&keys, &values);
        // SAFETY: Core Media's markup keys with values of the types `CMTextMarkup.h` gives.
        unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attributes) }
    }

    fn number(value: f64) -> Retained<AnyObject> {
        NSNumber::new_f64(value).into()
    }

    fn yes() -> Retained<AnyObject> {
        NSNumber::new_bool(true).into()
    }

    fn argb(channels: [f64; 4]) -> Retained<AnyObject> {
        let channels = channels.map(NSNumber::new_f64);
        NSArray::from_retained_slice(&channels).into()
    }

    fn named(constant: &NSString) -> Retained<AnyObject> {
        constant.copy().into()
    }

    /// The attributes `CMTextMarkup.h` has cover the whole string, which every run carries.
    fn whole() -> Vec<(&'static NSString, Retained<AnyObject>)> {
        vec![
            (
                markup!(kCMTextMarkupAttribute_OrthogonalLinePositionPercentageRelativeToWritingDirection),
                number(85.0),
            ),
            (
                markup!(kCMTextMarkupAttribute_TextPositionPercentageRelativeToWritingDirection),
                number(50.0),
            ),
            (
                markup!(kCMTextMarkupAttribute_WritingDirectionSizePercentage),
                number(80.0),
            ),
            (
                markup!(kCMTextMarkupAttribute_Alignment),
                named(markup!(kCMTextMarkupAlignmentType_Start)),
            ),
            (
                markup!(kCMTextMarkupAttribute_BackgroundColorARGB),
                argb([0.5, 0.0, 0.0, 0.0]),
            ),
            (
                markup!(kCMTextMarkupAttribute_BaseFontSizePercentageRelativeToVideoHeight),
                number(5.0),
            ),
        ]
    }

    #[test]
    fn a_legible_outputs_string_is_a_cue_with_a_run_per_style() {
        let mut first = whole();
        first.push((markup!(kCMTextMarkupAttribute_ItalicStyle), yes()));
        first.push((
            markup!(kCMTextMarkupAttribute_ForegroundColorARGB),
            argb([1.0, 1.0, 0.0, 0.0]),
        ));
        let mut second = whole();
        second.push((markup!(kCMTextMarkupAttribute_BoldStyle), yes()));
        second.push((
            markup!(kCMTextMarkupAttribute_RelativeFontSize),
            number(120.0),
        ));
        second.push((
            markup!(kCMTextMarkupAttribute_CharacterEdgeStyle),
            named(markup!(kCMTextMarkupCharacterEdgeStyle_DropShadow)),
        ));
        let string = NSMutableAttributedString::from_attributed_nsstring(&run("Hello ", &first));
        string.appendAttributedString(&run("world", &second));

        let cue = cue_of(&string);
        assert_eq!(cue.line, Some(CueLine::Fraction(0.85)));
        assert_eq!(cue.line_anchor, CueAnchor::Middle);
        assert_eq!(cue.position, Some(0.5));
        assert_eq!(cue.position_anchor, CueAnchor::Middle);
        assert_eq!(cue.size, Some(0.8));
        assert_eq!(cue.text_alignment, CueTextAlignment::Start);
        assert_eq!(cue.vertical, None);
        assert_eq!(
            cue.window_color,
            Some(Color::from(0.5, 0.0, 0.0, 0.0, ColorSpace::Srgb))
        );
        let CueContent::Text(runs) = cue.content else {
            panic!("a text cue");
        };
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "Hello ");
        assert!(runs[0].italic && !runs[0].bold);
        assert_eq!(runs[0].color, Some(Color::new(0xFFFF_0000)));
        assert_eq!(runs[0].font_size, Some(0.05));
        assert_eq!(runs[0].edge, None);
        assert_eq!(runs[1].text, "world");
        assert!(runs[1].bold && !runs[1].italic);
        assert_eq!(runs[1].color, None, "unsaid where none is given");
        assert_eq!(runs[1].font_size, Some(0.06));
        assert_eq!(runs[1].edge, Some(CueTextEdge::DropShadow));
        assert_eq!(runs[1].edge_color, None, "Core Media gives no edge colour");
    }

    #[test]
    fn a_string_without_markup_is_a_cue_in_the_default_place_that_says_no_style() {
        let cue = cue_of(&NSAttributedString::from_nsstring(&NSString::from_str(
            "Hi",
        )));
        assert_eq!(cue, Cue::new(CueContent::Text(vec![CueTextRun::new("Hi")])));
        assert_eq!(cue.window_color, None);
    }

    #[test]
    fn a_relative_size_alone_is_of_the_default_height() {
        let string = run(
            "Hi",
            &[(
                markup!(kCMTextMarkupAttribute_RelativeFontSize),
                number(200.0),
            )],
        );
        let CueContent::Text(runs) = cue_of(&string).content else {
            panic!("a text cue");
        };
        assert_eq!(runs[0].font_size, Some(CueTextRun::DEFAULT_FONT_SIZE * 2.0));
    }
}
