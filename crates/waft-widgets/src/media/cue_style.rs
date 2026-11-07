//! What a cue is drawn in: the media's own styling with the user's caption style merged in, as
//! Media3's `SubtitleView` and `SubtitlePainter` merge a `CaptionStyleCompat` into each cue, each
//! setting by its behaviour as Chromium's and WebKit's cue style sheets cascade the system's: a
//! setting that applies always wins over what the media says, one that applies where the media
//! says nothing fills in, and where neither says, Media3's default style does
//! (`CaptionStyleCompat.DEFAULT`, `SubtitleView.DEFAULT_TEXT_SIZE_FRACTION`).
//!
//! The invariant: a colour's opacity is merged on its own, after its colour, since Apple and
//! Windows give each its own setting with its own behaviour.

use waft_embedder::{
    CaptionBehavior, CaptionSetting, CaptionStyle, CaptionTypeface, Color, Cue, CueTextEdge,
    CueTextRun, GenericCaptionFont,
};

/// Media3's default text colour: white.
const DEFAULT_FOREGROUND: Color = Color::new(0xFFFF_FFFF);
/// Media3's default colour behind the characters: black.
const DEFAULT_BACKGROUND: Color = Color::new(0xFF00_0000);
/// Media3's default window: none to see.
const DEFAULT_WINDOW: Color = Color::new(0x0000_0000);
/// The edge colour where neither the media nor the style gives one: the black Chromium and WebKit
/// draw the edges of Apple's style in, which gives no colour.
const DEFAULT_EDGE_COLOR: Color = Color::new(0xFF00_0000);

/// A run of a cue as it is drawn: every field said.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct StyledRun<'a> {
    pub(super) text: &'a str,
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underline: bool,
    pub(super) color: Color,
    pub(super) background: Color,
    /// The text's height as a fraction of the picture's.
    pub(super) font_size: f32,
    /// `None` for the platform's own font.
    pub(super) font_family: Option<String>,
    pub(super) edge: CueTextEdge,
    pub(super) edge_color: Color,
}

/// `runs` with `style` merged into each.
pub(super) fn styled_runs<'a>(runs: &'a [CueTextRun], style: &CaptionStyle) -> Vec<StyledRun<'a>> {
    runs.iter().map(|run| styled_run(run, style)).collect()
}

/// The colour of the box behind `cue`.
pub(super) fn window_color(cue: &Cue, style: &CaptionStyle) -> Color {
    merged_color(
        cue.window_color,
        style.window_color.as_ref(),
        style.window_opacity.as_ref(),
        DEFAULT_WINDOW,
    )
}

/// The radius the corners of the box behind a cue are rounded by, in logical pixels, which no
/// media says.
pub(super) fn window_corner_radius(style: &CaptionStyle) -> f64 {
    style
        .window_corner_radius
        .as_ref()
        .map_or(0.0, |radius| radius.value)
}

fn styled_run<'a>(run: &'a CueTextRun, style: &CaptionStyle) -> StyledRun<'a> {
    StyledRun {
        text: &run.text,
        bold: run.bold,
        italic: run.italic,
        underline: run.underline,
        color: merged_color(
            run.color,
            style.foreground_color.as_ref(),
            style.foreground_opacity.as_ref(),
            DEFAULT_FOREGROUND,
        ),
        background: merged_color(
            run.background,
            style.background_color.as_ref(),
            style.background_opacity.as_ref(),
            DEFAULT_BACKGROUND,
        ),
        font_size: font_size(run, style),
        font_family: said(run.font_family.clone(), family_of(style).as_ref()),
        edge: merged(run.edge, style.edge.as_ref(), CueTextEdge::None),
        edge_color: merged(
            run.edge_color,
            style.edge_color.as_ref(),
            DEFAULT_EDGE_COLOR,
        ),
    }
}

/// The run's height: the media's, or the default height scaled by the style's text scale, which
/// replaces the media's where it applies always, as WebKit's and Chromium's `font-size` does.
fn font_size(run: &CueTextRun, style: &CaptionStyle) -> f32 {
    let scaled = style.text_scale.as_ref().map(|scale| CaptionSetting {
        value: (f64::from(CueTextRun::DEFAULT_FONT_SIZE) * scale.value) as f32,
        behavior: scale.behavior,
    });
    merged(
        run.font_size,
        scaled.as_ref(),
        CueTextRun::DEFAULT_FONT_SIZE,
    )
}

/// The setting where it applies always, else what the media says, else the setting, else
/// `default`.
fn merged<T: Clone>(media: Option<T>, setting: Option<&CaptionSetting<T>>, default: T) -> T {
    said(media, setting).unwrap_or(default)
}

/// The setting where it applies always, else what the media says, else the setting; `None` where
/// neither says.
fn said<T: Clone>(media: Option<T>, setting: Option<&CaptionSetting<T>>) -> Option<T> {
    match setting {
        Some(setting) if setting.behavior == CaptionBehavior::Always => Some(setting.value.clone()),
        _ => media.or_else(|| setting.map(|setting| setting.value.clone())),
    }
}

/// A colour merged as [`merged`] merges a setting, and its opacity merged on its own.
fn merged_color(
    media: Option<Color>,
    color: Option<&CaptionSetting<Color>>,
    opacity: Option<&CaptionSetting<f64>>,
    default: Color,
) -> Color {
    let merged_rgb = merged(media, color, default);
    let merged_opacity = merged(media.map(|color| color.a), opacity, default.a);
    merged_rgb.with_values(Some(merged_opacity.clamp(0.0, 1.0)), None, None, None, None)
}

/// The style's typeface as the family it names.
fn family_of(style: &CaptionStyle) -> Option<CaptionSetting<String>> {
    let typeface = style.typeface.as_ref()?;
    Some(CaptionSetting {
        value: family_name(&typeface.value).to_owned(),
        behavior: typeface.behavior,
    })
}

/// The family `typeface` names. A kind of font is named by the family Chromium draws it in
/// (`caption_style_win.cc`), as only Windows gives a kind; small capitals are drawn in their
/// family's own letters, since a run takes no font features.
fn family_name(typeface: &CaptionTypeface) -> &str {
    match typeface {
        CaptionTypeface::Family(family) => family,
        CaptionTypeface::Generic(kind) => match kind {
            GenericCaptionFont::MonospacedWithSerifs => "Courier New",
            GenericCaptionFont::ProportionalWithSerifs => "Times New Roman",
            GenericCaptionFont::MonospacedWithoutSerifs => "Consolas",
            GenericCaptionFont::ProportionalWithoutSerifs | GenericCaptionFont::SmallCapitals => {
                "Tahoma"
            }
            GenericCaptionFont::Casual => "Segoe Print",
            GenericCaptionFont::Cursive => "Segoe Script",
        },
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{CueContent, GenericCaptionFont};

    use super::*;

    const RED: Color = Color::new(0xFFFF_0000);
    const BLUE: Color = Color::new(0xFF00_00FF);
    const YELLOW: Color = Color::new(0xFFFF_FF00);

    fn run(text: &str) -> CueTextRun {
        CueTextRun::new(text)
    }

    #[test]
    fn where_neither_the_media_nor_the_user_says_media3s_default_style_draws() {
        let plain = run("Hi");
        let drawn = styled_run(&plain, &CaptionStyle::default());
        assert_eq!(drawn.color, DEFAULT_FOREGROUND);
        assert_eq!(drawn.background, DEFAULT_BACKGROUND);
        assert_eq!(drawn.font_size, CueTextRun::DEFAULT_FONT_SIZE);
        assert_eq!(drawn.font_family, None);
        assert_eq!(drawn.edge, CueTextEdge::None);
        let cue = Cue::new(CueContent::Text(vec![plain.clone()]));
        assert_eq!(window_color(&cue, &CaptionStyle::default()), DEFAULT_WINDOW);
        assert_eq!(window_corner_radius(&CaptionStyle::default()), 0.0);
    }

    #[test]
    fn a_setting_that_applies_always_wins_and_one_for_where_the_media_says_nothing_fills_in() {
        let plain = run("Hi");
        let mut said = run("Hi");
        said.color = Some(RED);
        said.edge = Some(CueTextEdge::Raised);
        let always = CaptionStyle {
            foreground_color: Some(CaptionSetting::always(BLUE)),
            edge: Some(CaptionSetting::always(CueTextEdge::Outline)),
            ..CaptionStyle::default()
        };
        let drawn = styled_run(&said, &always);
        assert_eq!(drawn.color, BLUE, "the user's colour over the media's");
        assert_eq!(drawn.edge, CueTextEdge::Outline);

        let filling_in = CaptionStyle {
            foreground_color: Some(CaptionSetting::where_media_says_nothing(BLUE)),
            edge: Some(CaptionSetting::where_media_says_nothing(
                CueTextEdge::Outline,
            )),
            ..CaptionStyle::default()
        };
        let drawn = styled_run(&said, &filling_in);
        assert_eq!(drawn.color, RED, "the media's colour where it says one");
        assert_eq!(drawn.edge, CueTextEdge::Raised);
        let drawn = styled_run(&plain, &filling_in);
        assert_eq!(drawn.color, BLUE, "the user's where the media says none");
        assert_eq!(drawn.edge, CueTextEdge::Outline);
        assert_eq!(drawn.edge_color, DEFAULT_EDGE_COLOR);
    }

    #[test]
    fn a_colours_opacity_is_merged_on_its_own() {
        let mut said = run("Hi");
        said.color = Some(RED.with_values(Some(0.5), None, None, None, None));
        let style = CaptionStyle {
            foreground_color: Some(CaptionSetting::always(BLUE)),
            foreground_opacity: Some(CaptionSetting::where_media_says_nothing(0.8)),
            background_color: Some(CaptionSetting::where_media_says_nothing(YELLOW)),
            background_opacity: Some(CaptionSetting::always(0.25)),
            ..CaptionStyle::default()
        };
        let drawn = styled_run(&said, &style);
        assert_eq!(
            drawn.color,
            BLUE.with_values(Some(0.5), None, None, None, None),
            "the user's colour at the media's opacity"
        );
        assert_eq!(
            drawn.background,
            YELLOW.with_values(Some(0.25), None, None, None, None),
            "the user's colour, which the media leaves unsaid, at the user's opacity"
        );
    }

    #[test]
    fn the_text_scale_scales_the_default_size_and_replaces_the_medias_only_where_it_applies_always()
    {
        let plain = run("Hi");
        let mut said = run("Hi");
        said.font_size = Some(0.1);
        let scaled = |behavior| CaptionStyle {
            text_scale: Some(CaptionSetting {
                value: 2.0,
                behavior,
            }),
            ..CaptionStyle::default()
        };
        let twice_the_default = CueTextRun::DEFAULT_FONT_SIZE * 2.0;
        assert_eq!(
            styled_run(&said, &scaled(CaptionBehavior::WhereMediaSaysNothing)).font_size,
            0.1
        );
        assert_eq!(
            styled_run(&plain, &scaled(CaptionBehavior::WhereMediaSaysNothing)).font_size,
            twice_the_default
        );
        assert_eq!(
            styled_run(&said, &scaled(CaptionBehavior::Always)).font_size,
            twice_the_default
        );
    }

    #[test]
    fn a_typeface_is_drawn_in_its_family_and_a_kind_of_font_in_the_family_chromium_names() {
        let plain = run("Hi");
        let typeface = |typeface| CaptionStyle {
            typeface: Some(CaptionSetting::where_media_says_nothing(typeface)),
            ..CaptionStyle::default()
        };
        let family = styled_run(
            &plain,
            &typeface(CaptionTypeface::Family("Avenir".to_owned())),
        );
        assert_eq!(family.font_family.as_deref(), Some("Avenir"));
        let kind = styled_run(
            &plain,
            &typeface(CaptionTypeface::Generic(
                GenericCaptionFont::MonospacedWithSerifs,
            )),
        );
        assert_eq!(kind.font_family.as_deref(), Some("Courier New"));
        let mut said = run("Hi");
        said.font_family = Some("Menlo".to_owned());
        let media = styled_run(
            &said,
            &typeface(CaptionTypeface::Family("Avenir".to_owned())),
        );
        assert_eq!(media.font_family.as_deref(), Some("Menlo"));
    }

    #[test]
    fn the_window_is_the_medias_unless_the_users_applies_always() {
        let mut cue = Cue::new(CueContent::Text(vec![run("Hi")]));
        cue.window_color = Some(RED);
        let style = |setting| CaptionStyle {
            window_color: Some(setting),
            window_opacity: Some(CaptionSetting::where_media_says_nothing(0.5)),
            window_corner_radius: Some(CaptionSetting::where_media_says_nothing(4.0)),
            ..CaptionStyle::default()
        };
        assert_eq!(
            window_color(&cue, &style(CaptionSetting::where_media_says_nothing(BLUE))),
            RED
        );
        assert_eq!(
            window_color(&cue, &style(CaptionSetting::always(BLUE))),
            BLUE,
            "the user's colour at the media's opacity"
        );
        assert_eq!(
            window_corner_radius(&style(CaptionSetting::always(BLUE))),
            4.0
        );
    }
}
