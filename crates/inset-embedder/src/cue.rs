//! Subtitles and captions as a player hands them to the app: Media3's `Cue`, field for field, since
//! Media3 turns every subtitle format it reads into one, and each system's player gives the same
//! as data (Apple's legible output, Windows' timed text cues). The app draws them; the host merges
//! the user's caption settings into each cue first, so every field here is what to draw.
//!
//! Sizes and places are fractions of the picture the cue is drawn over, so a cue reads the same
//! at any size the picture is shown at.

use crate::Color;

/// One subtitle or caption to show: Media3's `Cue`.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub content: CueContent,
    /// Where its box sits from the top: a fraction of the picture's height, or a line number;
    /// `None` for the default, the bottom.
    pub line: Option<CueLine>,
    /// Which side of its box `line` places.
    pub line_anchor: CueAnchor,
    /// Where its box sits across, as a fraction of the picture's width; `None` for the default,
    /// the centre.
    pub position: Option<f32>,
    /// Which side of its box `position` places.
    pub position_anchor: CueAnchor,
    /// Its box's width as a fraction of the picture's; `None` to fit what it holds.
    pub size: Option<f32>,
    pub text_alignment: CueTextAlignment,
    /// Whether its text runs in columns, and which way they follow each other; `None` for rows.
    pub vertical: Option<CueVertical>,
    /// The colour of the box behind it; `None` for no box.
    pub window_color: Option<Color>,
}

impl Cue {
    /// A cue showing `content` where Media3's `Cue.Builder` puts one by default: centred at the
    /// bottom, its text centred, with no box behind it.
    pub fn new(content: CueContent) -> Cue {
        Cue {
            content,
            line: None,
            line_anchor: CueAnchor::Start,
            position: None,
            position_anchor: CueAnchor::Start,
            size: None,
            text_alignment: CueTextAlignment::Center,
            vertical: None,
            window_color: None,
        }
    }
}

/// What a cue shows.
#[derive(Clone, Debug, PartialEq)]
pub enum CueContent {
    /// Its text, in runs of one style.
    Text(Vec<CueTextRun>),
    /// A picture subtitle, as Blu-ray, DVD and DVB carry them, decoded.
    Image(CueImage),
}

/// Text of one style.
#[derive(Clone, Debug, PartialEq)]
pub struct CueTextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Color,
    /// The colour behind these characters; `None` for none.
    pub background: Option<Color>,
    /// The text's height as a fraction of the picture's.
    pub font_size: f32,
    /// `None` for the system's caption font.
    pub font_family: Option<String>,
    pub edge: CueTextEdge,
    pub edge_color: Color,
}

impl CueTextRun {
    /// The height Media3's `SubtitleView` gives text by default, `DEFAULT_TEXT_SIZE_FRACTION`: a
    /// fraction of the picture's.
    pub const DEFAULT_FONT_SIZE: f32 = 0.0533;

    /// `text` as Media3's default caption style draws it (`CaptionStyleCompat.DEFAULT`): white,
    /// at [`DEFAULT_FONT_SIZE`](Self::DEFAULT_FONT_SIZE), in the system's caption font, with
    /// nothing behind it and no edge; its edge colour is the black WebKit draws Apple's edges in.
    pub fn new(text: impl Into<String>) -> CueTextRun {
        CueTextRun {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            color: Color::new(0xFFFF_FFFF),
            background: None,
            font_size: CueTextRun::DEFAULT_FONT_SIZE,
            font_family: None,
            edge: CueTextEdge::None,
            edge_color: Color::new(0xFF00_0000),
        }
    }
}

/// A picture subtitle's pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct CueImage {
    pub width: u32,
    pub height: u32,
    /// RGBA, 8 bits a channel, unpremultiplied, rows from the top.
    pub pixels: Vec<u8>,
    /// Its height on screen as a fraction of the picture's; its width is the cue's `size`.
    pub display_height: f32,
}

/// Where a cue's box sits from the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CueLine {
    /// A fraction of the picture's height.
    Fraction(f32),
    /// A line number: from the top counting from 0, from the bottom counting from -1, in lines
    /// of the cue's first line's height.
    Number(i32),
}

/// Which side of a cue's box its `line` or `position` places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueAnchor {
    Start,
    Middle,
    End,
}

/// How a cue's lines align in its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueTextAlignment {
    Start,
    Center,
    End,
}

/// Which way a vertical cue's columns follow each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueVertical {
    RightToLeft,
    LeftToRight,
}

/// What is drawn round a run's characters to set them off from the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueTextEdge {
    None,
    Outline,
    DropShadow,
    Raised,
    Depressed,
}
