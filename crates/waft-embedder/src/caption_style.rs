//! The user's caption style: how the system's settings say subtitles and captions are to look,
//! read beside the cues a player hands out, which carry the media's own styling alone ([`Cue`]).
//! The systems that keep caption settings give the same ones, and tell when the user changes
//! them: Apple's `MACaptionAppearance`, Android's `CaptioningManager` with its `CaptionStyle`, and
//! Windows' `ClosedCaptionProperties`, which Chromium's `ui/native_theme/caption_style` reads on
//! Windows and macOS. Linux and the web keep none, and a platform there answers no
//! [`CaptionSettings`].
//!
//! Each setting says whether it applies always, over what the media says, or only where the media
//! says nothing, as Apple's `MACaptionAppearanceBehavior` says of each of its settings: Chromium's
//! and WebKit's cue style sheets make the first `!important` and leave the second for the media's
//! own style to override. A setting the system gives no value for is `None`, as Android's
//! `hasForegroundColor` false and Windows' `Default` say.
//!
//! The app merges the style into each cue as it draws it, as Media3's `SubtitleView` applies its
//! `CaptionStyleCompat`; the host merges nothing.
//!
//! [`Cue`]: crate::Cue

use crate::{Color, CueTextEdge, MediaChangeFuture};

/// The user's system-wide caption settings: Apple's `MACaptionAppearance`, Android's
/// `CaptioningManager`, Windows' `ClosedCaptionProperties`.
pub trait CaptionSettings {
    /// The style the user set, each setting saying whether it applies always or only where the
    /// media says nothing.
    fn style(&self) -> CaptionStyle;

    /// Resolves at the next change of the style, as a player's `changed()` does.
    fn changed(&self) -> MediaChangeFuture;
}

/// What the system's settings say of how subtitles and captions look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaptionStyle {
    /// The text's colour, opaque: Apple's foreground colour, Android's `foregroundColor`,
    /// Windows' `FontColor`.
    pub foreground_color: Option<CaptionSetting<Color>>,
    /// The text's opacity, from `0.0` to `1.0`: Apple's foreground opacity, the alpha of Android's
    /// `foregroundColor`, Windows' `FontOpacity`.
    pub foreground_opacity: Option<CaptionSetting<f64>>,
    /// The colour behind each character, opaque: Apple's background colour, Android's
    /// `backgroundColor`, Windows' `BackgroundColor`.
    pub background_color: Option<CaptionSetting<Color>>,
    /// The opacity behind each character: Apple's background opacity, the alpha of Android's
    /// `backgroundColor`, Windows' `BackgroundOpacity`.
    pub background_opacity: Option<CaptionSetting<f64>>,
    /// The colour of the box behind a whole cue, opaque: Apple's window colour, Android's
    /// `windowColor`, Windows' `RegionColor`.
    pub window_color: Option<CaptionSetting<Color>>,
    /// The opacity of the box behind a whole cue: Apple's window opacity, the alpha of Android's
    /// `windowColor`, Windows' `RegionOpacity`.
    pub window_opacity: Option<CaptionSetting<f64>>,
    /// The radius the box's corners are rounded by, in logical pixels: Apple's window rounded
    /// corner radius, which Chromium and WebKit read as the cue's `border-radius`.
    pub window_corner_radius: Option<CaptionSetting<f64>>,
    /// What is drawn round the characters: Apple's text edge style, Android's `edgeType`, Windows'
    /// `FontEffect`.
    pub edge: Option<CaptionSetting<CueTextEdge>>,
    /// The colour of the edge, with its opacity: Android's `edgeColor`. Apple and Windows give
    /// none.
    pub edge_color: Option<CaptionSetting<Color>>,
    /// The font: Apple's font for its default style, Android's typeface, Windows' `FontStyle`.
    pub typeface: Option<CaptionSetting<CaptionTypeface>>,
    /// How many times its own size text is drawn at: Apple's relative character size, Android's
    /// `getFontScale`, Windows' `FontSize` as a fraction.
    pub text_scale: Option<CaptionSetting<f64>>,
}

/// One of the user's caption settings, and whether it applies over what the media says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptionSetting<T> {
    pub value: T,
    pub behavior: CaptionBehavior,
}

impl<T> CaptionSetting<T> {
    /// `value`, applied over what the media says.
    pub fn always(value: T) -> CaptionSetting<T> {
        CaptionSetting {
            value,
            behavior: CaptionBehavior::Always,
        }
    }

    /// `value`, applied where the media says nothing.
    pub fn where_media_says_nothing(value: T) -> CaptionSetting<T> {
        CaptionSetting {
            value,
            behavior: CaptionBehavior::WhereMediaSaysNothing,
        }
    }
}

/// When a caption setting applies: Apple's `MACaptionAppearanceBehavior`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionBehavior {
    /// Over what the media says: Apple's `UseValue`, a setting Chromium's and WebKit's cue style
    /// sheets mark `!important`.
    Always,
    /// Only where the media says nothing: Apple's `UseContentIfAvailable`, and every setting of
    /// Android's, which Media3 draws beneath a cue's embedded styles.
    WhereMediaSaysNothing,
}

/// The font captions are drawn in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptionTypeface {
    /// A font family by name: the family of the font Apple's settings give for their default
    /// style, or Android's typeface by its family name, such as `serif-monospace`.
    Family(String),
    /// A kind of font, as caption standards (CEA-708) name them and Windows gives the user's
    /// choice.
    Generic(GenericCaptionFont),
}

/// The kinds of font caption standards name: Windows' `ClosedCaptionStyle` and Apple's
/// `MACaptionAppearanceFontStyle`, their defaults aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenericCaptionFont {
    MonospacedWithSerifs,
    ProportionalWithSerifs,
    MonospacedWithoutSerifs,
    ProportionalWithoutSerifs,
    Casual,
    Cursive,
    SmallCapitals,
}
