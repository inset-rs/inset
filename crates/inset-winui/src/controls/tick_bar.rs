//! XAML `TickBar` (`dxaml/xcp/dxaml/lib/TickBar_Partial.cpp`): the tick marks of a `Slider`, one physical pixel each, laid out by `ArrangeOverride` from the slider's `TickFrequency`, its range, its thumb length and its direction.

use crate::{Orientation, less_than, less_than_or_close};
use inset_embedder::{Canvas, Color, Rect, Size};
use inset_foundation::App;
use inset_rendering::CustomPainter;
use inset_widgets::*;

/// `MIN_TICKMARK_GAP` of `TickBar_Partial.h`: Windows requires visual tick marks at least this far apart; closer logical ticks are thinned to a multiple.
pub const MIN_TICKMARK_GAP: f64 = 20.0;

/// XAML `TickPlacement`: which of the slider's tick bars show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TickPlacement {
    #[default]
    None,
    TopLeft,
    BottomRight,
    Outside,
    Inline,
}

impl TickPlacement {
    /// `Slider::OnTickPlacementChanged`: `TopTickBar` / `LeftTickBar` visibility.
    pub fn shows_top_left(self) -> bool {
        matches!(self, TickPlacement::TopLeft | TickPlacement::Outside)
    }

    /// `Slider::OnTickPlacementChanged`: `BottomTickBar` / `RightTickBar` visibility.
    pub fn shows_bottom_right(self) -> bool {
        matches!(self, TickPlacement::BottomRight | TickPlacement::Outside)
    }

    /// `Slider::OnTickPlacementChanged`: the inline tick bars' visibility.
    pub fn shows_inline(self) -> bool {
        self == TickPlacement::Inline
    }
}

/// A `TickBar` element: its `Fill` and what `ArrangeOverride` reads from the `Slider` it belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct TickBar {
    pub fill: Color,
    pub orientation: Orientation,
    /// The slider's `TickFrequency`; nothing is drawn when it is not positive.
    pub tick_frequency: f64,
    pub minimum: f64,
    pub maximum: f64,
    /// `Slider::GetThumbLength`: the thumb's extent along the orientation; the first tick sits at its centre.
    pub thumb_length: f64,
    pub is_direction_reversed: bool,
    /// `RootScale::GetRasterizationScaleForElement`: a tick mark is one physical pixel, `1 / zoom_scale` logical.
    pub zoom_scale: f64,
}

impl TickBar {
    pub fn new(fill: Color, orientation: Orientation) -> TickBar {
        TickBar {
            fill,
            orientation,
            tick_frequency: 0.0,
            minimum: 0.0,
            maximum: 1.0,
            thumb_length: 0.0,
            is_direction_reversed: false,
            zoom_scale: 1.0,
        }
    }

    pub fn tick_frequency(mut self, frequency: f64) -> TickBar {
        self.tick_frequency = frequency;
        self
    }

    pub fn range(mut self, minimum: f64, maximum: f64) -> TickBar {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    pub fn thumb_length(mut self, length: f64) -> TickBar {
        self.thumb_length = length;
        self
    }

    pub fn is_direction_reversed(mut self, reversed: bool) -> TickBar {
        self.is_direction_reversed = reversed;
        self
    }

    pub fn zoom_scale(mut self, scale: f64) -> TickBar {
        self.zoom_scale = scale;
        self
    }

    /// One logical pixel of the tick mark's thickness: `singlePixelWidthScaled`.
    pub fn tick_thickness(&self) -> f64 {
        1.0 / self.zoom_scale
    }

    /// `TickBar::ArrangeOverride`: where along a bar of `final_length` each tick mark starts.
    pub fn tick_offsets(&self, final_length: f64) -> Vec<f64> {
        if less_than_or_close(self.tick_frequency, 0.0) {
            return Vec::new();
        }
        let single_pixel = self.tick_thickness();
        // The range the tick marks are laid out in: the track less the thumb.
        let visual_range = final_length - self.thumb_length;
        // One tick mark at the end of each full interval, and one at the start.
        let num_intervals = ((self.maximum - self.minimum) / self.tick_frequency).max(1.0);
        let mut tick_mark_number = num_intervals.floor() as usize;
        let mut tick_mark_interval = (visual_range / num_intervals).max(1.0);
        if less_than(tick_mark_interval, MIN_TICKMARK_GAP) {
            // Draw only every n-th tick, n the smallest multiple that clears the gap.
            let ratio = (MIN_TICKMARK_GAP / tick_mark_interval).ceil() as usize;
            tick_mark_interval *= ratio as f64;
            tick_mark_number /= ratio;
        }
        tick_mark_number += 1;
        // The first tick mark sits in the middle of the thumb at its starting position.
        let thumb_offset = ((self.thumb_length - single_pixel) / 2.0).max(0.0);
        let vertical = self.orientation == Orientation::Vertical;
        (0..tick_mark_number)
            .map(|j| {
                let travelled = j as f64 * tick_mark_interval;
                if vertical == self.is_direction_reversed {
                    // A horizontal slider, or a vertical one reversed: ticks start at the beginning of the track.
                    thumb_offset + travelled
                } else {
                    // A vertical slider, or a horizontal one reversed: ticks start at the end, less the tick's own pixel.
                    final_length - (thumb_offset + single_pixel) - travelled
                }
            })
            .collect()
    }
}

impl StatelessWidget for TickBar {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        CustomPaint::new()
            .painter(TickBarPainter {
                tick_bar: self.clone(),
            })
            .into_widget()
    }
}

/// Paints the `Rectangle` children `ArrangeOverride` creates, one per tick mark.
#[derive(Clone, Debug, PartialEq)]
struct TickBarPainter {
    tick_bar: TickBar,
}

impl CustomPainter for TickBarPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let bar = &self.tick_bar;
        let thickness = bar.tick_thickness();
        let (final_length, tick_rect): (f64, fn(f64, f64, Size) -> Rect) = match bar.orientation {
            Orientation::Horizontal => (size.width(), |offset, thickness, size| {
                Rect::from_ltwh(offset, 0.0, thickness, size.height())
            }),
            Orientation::Vertical => (size.height(), |offset, thickness, size| {
                Rect::from_ltwh(0.0, offset, size.width(), thickness)
            }),
        };
        let paint = inset_embedder::Paint::from_color(bar.fill.into());
        for offset in bar.tick_offsets(final_length) {
            canvas.draw_rect(tick_rect(offset, thickness, size), &paint);
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
