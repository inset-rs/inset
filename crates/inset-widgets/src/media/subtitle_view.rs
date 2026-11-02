// Ported from Media3's `SubtitleView.java` and `CanvasSubtitleOutput.java`
// (https://github.com/androidx/media), Copyright (C) 2016 The Android Open Source Project, licensed
// under the Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0). Changed: a
// widget that takes Inset's cues, whose runs carry the caption style the host merged in, so it has
// no style, text size or embedded-style switches of its own.

//! Media3's `SubtitleView`, drawing on a canvas as its `CanvasSubtitleOutput` does: a player's cues
//! over the rectangle of its picture, each painted by `subtitle_painter`. It fills the space it is
//! given, which is the picture's rectangle where a `PlayerView` puts it, so a cue's fractions are
//! of the picture.

use std::any::Any;
use std::fmt;

use inset_embedder::{Canvas, Cue, CueAnchor, CueLine, Rect, Size};
use inset_foundation::App;
use inset_rendering::CustomPainter;

use super::subtitle_painter;
use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::CustomPaint;

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

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        CustomPaint::new()
            .painter(CanvasSubtitleOutput {
                cues: self.cues.clone(),
                bottom_padding_fraction: self.bottom_padding_fraction,
            })
            .size(Size::INFINITE)
            .into_widget()
    }
}

/// Media3's `CanvasSubtitleOutput`: paints each cue with `subtitle_painter`.
struct CanvasSubtitleOutput {
    cues: Vec<Cue>,
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
    use inset_embedder::{CueContent, CueTextRun, CueVertical};

    use super::*;

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
