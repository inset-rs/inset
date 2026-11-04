// Ported from Media3's `SubtitleView.java` and `CanvasSubtitleOutput.java`
// (https://github.com/androidx/media), Copyright (C) 2016 The Android Open Source Project, licensed
// under the Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0). Changed: a
// widget that takes Inset's cues and always draws them in the user's caption style, which it
// reads from the ambient `MediaQuery`, so it has no style, text size or embedded-style switches
// of its own.

//! Media3's `SubtitleView`, drawing on a canvas as its `CanvasSubtitleOutput` does: a player's cues
//! over the rectangle of its picture, each painted by `subtitle_painter` with the user's caption
//! style merged in, as Media3's `setUserDefaultStyle` and `setUserDefaultTextSize` have it. It
//! fills the space it is given, which is the picture's rectangle where a `PlayerView` puts it, so a
//! cue's fractions are of the picture.
//!
//! The style is the ambient `MediaQuery`'s, which follows the platform's caption settings, so
//! the view is built again when the user changes them; where the platform keeps none, no setting
//! is merged and the media's own style shows.

use std::any::Any;
use std::fmt;

use inset_embedder::{Canvas, CaptionStyle, Cue, CueAnchor, CueLine, Rect, Size};
use inset_foundation::App;
use inset_rendering::CustomPainter;

use super::subtitle_painter;
use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::CustomPaint;
use crate::widgets::media_query::MediaQuery;

/// A view for displaying subtitle cues, laid out as large as it is let be.
#[derive(Clone)]
pub struct SubtitleView {
    pub key: Option<KeyRef>,
    /// The cues to display.
    pub cues: Vec<Cue>,
    /// The bottom padding to apply when a cue has no `line`, as a fraction of the view's height.
    pub bottom_padding_fraction: f32,
}

impl SubtitleView {
    /// The default bottom padding to apply when a cue has no `line`, as a fraction of the
    /// viewport height.
    pub const DEFAULT_BOTTOM_PADDING_FRACTION: f32 = 0.08;

    /// A view displaying `cues`: Media3's `setCues`.
    pub fn new(cues: Vec<Cue>) -> SubtitleView {
        SubtitleView {
            key: None,
            cues,
            bottom_padding_fraction: SubtitleView::DEFAULT_BOTTOM_PADDING_FRACTION,
        }
    }

    pub fn key(mut self, key: KeyRef) -> SubtitleView {
        self.key = Some(key);
        self
    }

    /// Media3's `setBottomPaddingFraction`.
    pub fn bottom_padding_fraction(mut self, bottom_padding_fraction: f32) -> SubtitleView {
        self.bottom_padding_fraction = bottom_padding_fraction;
        self
    }
}

impl fmt::Debug for SubtitleView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubtitleView")
            .field("cues", &self.cues)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for SubtitleView {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        CustomPaint::new()
            .painter(CanvasSubtitleOutput {
                cues: self.cues.clone(),
                style: MediaQuery::maybe_caption_style_of(app, context).unwrap_or_default(),
                bottom_padding_fraction: self.bottom_padding_fraction,
            })
            .size(Size::INFINITE)
            .into_widget()
    }
}

/// Media3's `CanvasSubtitleOutput`: paints each cue with `subtitle_painter`.
struct CanvasSubtitleOutput {
    cues: Vec<Cue>,
    /// The user's caption style, with no setting where the platform keeps none.
    style: CaptionStyle,
    bottom_padding_fraction: f32,
}

impl CustomPainter for CanvasSubtitleOutput {
    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        if self.cues.is_empty() {
            return;
        }
        let cue_box = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
        if cue_box.is_empty() {
            // No space to draw subtitles.
            return;
        }
        for cue in &self.cues {
            let cue = match cue.vertical {
                Some(_) => reposition_vertical_cue(cue),
                None => cue.clone(),
            };
            subtitle_painter::draw(
                app,
                canvas,
                &cue,
                &self.style,
                f64::from(self.bottom_padding_fraction),
                cue_box,
            );
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<CanvasSubtitleOutput>()
        else {
            return true;
        };
        self.cues != old_delegate.cues
            || self.style != old_delegate.style
            || self.bottom_padding_fraction != old_delegate.bottom_padding_fraction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Reposition a vertical cue for horizontal display.
///
/// This view doesn't support rendering vertical text, but if we naively interpret a vertical
/// cue's `position` and `line` for horizontal display then the cues will often be displayed in
/// unexpected positions. For example, the 'default' position for right-to-left vertical subtitles
/// is the right-hand edge of the viewport, so cues that would appear vertically in this position
/// should appear horizontally at the bottom of the viewport (generally the default position).
/// Similarly left-edge right-to-left vertical cues should be shown at the top of a horizontal
/// viewport.
///
/// There isn't a meaningful way to transform `position` and related values (e.g. alignment), so
/// we clear these and allow `subtitle_painter` to do the default behaviour of centering the cue.
fn reposition_vertical_cue(cue: &Cue) -> Cue {
    let line = cue.line.map(|line| match line {
        CueLine::Fraction(line) => CueLine::Fraction(1.0 - line),
        CueLine::Number(line) => CueLine::Number(-line - 1),
    });
    let line_anchor = match cue.line_anchor {
        CueAnchor::End => CueAnchor::Start,
        CueAnchor::Start => CueAnchor::End,
        CueAnchor::Middle => CueAnchor::Middle,
    };
    Cue {
        position: None,
        position_anchor: CueAnchor::Start,
        text_alignment: inset_embedder::CueTextAlignment::Center,
        line,
        line_anchor,
        ..cue.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::valo::Op;
    use inset_embedder::{
        CaptionSetting, Color, CueContent, CueTextRun, CueVertical, SystemFontSource,
        TargetPlatform,
    };
    use inset_foundation::AppCell;
    use inset_test::{MockCaptionSettings, TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::test_harness::pump_every_view;

    const RED: Color = Color::new(0xFFFF_0000);
    const GREEN: Color = Color::new(0xFF00_FF00);
    const BLUE: Color = Color::new(0xFF00_00FF);

    /// A style whose text is `color` over whatever the media says.
    fn text_in(color: Color) -> CaptionStyle {
        CaptionStyle {
            foreground_color: Some(CaptionSetting::always(color)),
            ..CaptionStyle::default()
        }
    }

    /// The colours of the glyph runs `view`'s last scene draws.
    fn glyph_colors(view: &TestView) -> Vec<inset_embedder::valo::Color> {
        let scene = view.last_scene().expect("a frame was presented");
        inset_embedder::flattened_ops(&inset_embedder::slices(&scene, |_| None).picture_with_room())
            .into_iter()
            .filter_map(|op| match op {
                Op::GlyphRun { paint, .. } => Some(paint.color),
                _ => None,
            })
            .collect()
    }

    /// A subtitle view on `platform`, drawn once, showing a cue whose text the media colours blue.
    fn showing_a_blue_cue(platform: TestPlatform) -> (Rc<TestView>, Rc<AppCell>) {
        let view = Rc::new(TestView::new(640.0, 360.0));
        let platform = platform
            .on(TargetPlatform::MacOS)
            .with_view(Rc::clone(&view) as inset_embedder::ViewRef)
            .with_font_source(|| Box::new(SystemFontSource::platform()));
        let cell = AppCell::with_platform(Rc::new(platform) as inset_embedder::PlatformRef);
        {
            let mut app = cell.borrow_mut();
            inset_painting::PaintingBinding::instance(&mut app).install_platform_fonts(&mut app);
        }
        let mut said = CueTextRun::new("Hello");
        said.color = Some(BLUE);
        let cue = Cue::new(CueContent::Text(vec![said]));
        run_app(
            &mut cell.borrow_mut(),
            SubtitleView::new(vec![cue]).into_widget(),
        );
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
        (view, cell)
    }

    #[test]
    fn a_subtitle_view_draws_in_the_users_caption_style_and_follows_its_changes() {
        let settings = MockCaptionSettings::new(text_in(RED));
        let (view, cell) = showing_a_blue_cue(
            TestPlatform::new().with_caption_settings(Rc::new(settings.clone())),
        );
        assert!(glyph_colors(&view).contains(&RED.into()), "the user's red");

        settings.set_style(text_in(GREEN));
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(16));
        let colors = glyph_colors(&view);
        assert!(colors.contains(&GREEN.into()), "the new style: {colors:?}");
        assert!(!colors.contains(&RED.into()));
    }

    #[test]
    fn a_subtitle_view_draws_the_medias_own_style_where_the_platform_has_no_caption_settings() {
        let (view, _cell) = showing_a_blue_cue(TestPlatform::new());
        let colors = glyph_colors(&view);
        assert!(
            colors.contains(&BLUE.into()),
            "the media's blue: {colors:?}"
        );
    }

    #[test]
    fn a_vertical_cue_is_shown_across_on_the_opposite_side_centred() {
        let mut cue = Cue::new(CueContent::Text(vec![CueTextRun::new("縦")]));
        cue.vertical = Some(CueVertical::RightToLeft);
        cue.line = Some(CueLine::Fraction(0.9));
        cue.line_anchor = CueAnchor::End;
        cue.position = Some(0.2);
        let shown = reposition_vertical_cue(&cue);
        assert_eq!(shown.line, Some(CueLine::Fraction(1.0 - 0.9)));
        assert_eq!(shown.line_anchor, CueAnchor::Start);
        assert_eq!(shown.position, None);

        cue.line = Some(CueLine::Number(0));
        assert_eq!(
            reposition_vertical_cue(&cue).line,
            Some(CueLine::Number(-1))
        );
    }
}
