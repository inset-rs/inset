//! Flutter counterpart: material_ui's `tab_indicator.dart`.

use std::any::Any;
use std::fmt;

use waft_embedder::{
    Canvas, Offset, Paint, PaintStyle, Path, PathBuilder, RRect, Rect, StrokeCap,
    rrect_radii_elliptical,
};
use waft_foundation::{App, VoidCallback};
use waft_painting::{
    BorderRadius, BorderSide, BorderStyle, BoxPainter, Decoration, EdgeInsetsGeometry,
    ImageConfiguration, TextDirection, draw_line, draw_rrect,
};

use crate::colors::Colors;

/// Used with `TabBar::indicator` to draw a horizontal line below the
/// selected tab.
///
/// The selected tab underline is inset from the tab's boundary by [`insets`](Self::insets).
/// The [`border_side`](Self::border_side) defines the line's color and weight.
///
/// The `TabBar::indicator_size` property can be used to define the indicator's
/// bounds in terms of its (centered) widget with `TabBarIndicatorSize::Label`,
/// or the entire tab with `TabBarIndicatorSize::Tab`.
#[derive(Clone)]
pub struct UnderlineTabIndicator {
    /// The radius of the indicator's corners.
    ///
    /// If this value is non-null, rounded rectangular tab indicator is
    /// drawn, otherwise rectangular tab indicator is drawn.
    pub border_radius: Option<BorderRadius>,

    /// The color and weight of the horizontal line drawn below the selected tab.
    pub border_side: BorderSide,

    /// Locates the selected tab's underline relative to the tab's boundary.
    ///
    /// The `TabBar::indicator_size` property can be used to define the tab
    /// indicator's bounds in terms of its (centered) tab widget with
    /// `TabBarIndicatorSize::Label`, or the entire tab with
    /// `TabBarIndicatorSize::Tab`.
    pub insets: EdgeInsetsGeometry,
}

impl Default for UnderlineTabIndicator {
    fn default() -> UnderlineTabIndicator {
        UnderlineTabIndicator {
            border_radius: None,
            border_side: BorderSide::new(
                Colors::WHITE,
                2.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ),
            insets: EdgeInsetsGeometry::ZERO,
        }
    }
}

impl UnderlineTabIndicator {
    /// Create an underline style selected tab indicator; Dart's named arguments are the
    /// setters.
    pub fn new() -> UnderlineTabIndicator {
        UnderlineTabIndicator::default()
    }

    /// Dart `UnderlineTabIndicator(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> UnderlineTabIndicator {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `UnderlineTabIndicator(borderSide:)`.
    pub fn border_side(mut self, border_side: BorderSide) -> UnderlineTabIndicator {
        self.border_side = border_side;
        self
    }

    /// Dart `UnderlineTabIndicator(insets:)`.
    pub fn insets(mut self, insets: impl Into<EdgeInsetsGeometry>) -> UnderlineTabIndicator {
        self.insets = insets.into();
        self
    }

    fn indicator_rect_for(&self, rect: Rect, text_direction: TextDirection) -> Rect {
        let indicator = self.insets.resolve(Some(text_direction)).deflate_rect(rect);
        Rect::from_ltwh(
            indicator.left,
            indicator.bottom - self.border_side.width,
            indicator.width(),
            self.border_side.width,
        )
    }
}

impl Decoration for UnderlineTabIndicator {
    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<UnderlineTabIndicator>()) {
            return Some(Box::new(
                UnderlineTabIndicator::new()
                    .border_side(BorderSide::lerp(a.border_side, self.border_side, t))
                    .insets(
                        EdgeInsetsGeometry::lerp(Some(a.insets), Some(self.insets), t)
                            .expect("two insets lerp to insets"),
                    ),
            ));
        }
        None
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<UnderlineTabIndicator>()) {
            return Some(Box::new(
                UnderlineTabIndicator::new()
                    .border_side(BorderSide::lerp(self.border_side, b.border_side, t))
                    .insets(
                        EdgeInsetsGeometry::lerp(Some(self.insets), Some(b.insets), t)
                            .expect("two insets lerp to insets"),
                    ),
            ));
        }
        None
    }

    fn create_box_painter(&self, on_changed: Option<VoidCallback>) -> Box<dyn BoxPainter> {
        Box::new(UnderlinePainter::new(
            self.clone(),
            self.border_radius,
            on_changed,
        ))
    }

    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Path {
        let indicator = self.indicator_rect_for(rect, text_direction);
        let mut path = PathBuilder::new();
        if let Some(border_radius) = self.border_radius {
            let rrect = border_radius.to_rrect(indicator);
            path.rrect_radii_elliptical(rrect.outer_rect(), rrect_radii_elliptical(rrect));
            return path.build().into();
        }
        path.rect(indicator.into());
        path.build().into()
    }

    fn clone_box(&self) -> Box<dyn Decoration> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for UnderlineTabIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnderlineTabIndicator")
            .field("border_radius", &self.border_radius)
            .field("border_side", &self.border_side)
            .field("insets", &self.insets)
            .finish()
    }
}

/// Dart's `_UnderlinePainter`.
struct UnderlinePainter {
    decoration: UnderlineTabIndicator,
    border_radius: Option<BorderRadius>,
    on_changed: Option<VoidCallback>,
}

impl UnderlinePainter {
    fn new(
        decoration: UnderlineTabIndicator,
        border_radius: Option<BorderRadius>,
        on_changed: Option<VoidCallback>,
    ) -> UnderlinePainter {
        UnderlinePainter {
            decoration,
            border_radius,
            on_changed,
        }
    }
}

impl BoxPainter for UnderlinePainter {
    fn paint(&mut self, canvas: &mut Canvas, offset: Offset, configuration: &ImageConfiguration) {
        debug_assert!(configuration.size.is_some());
        let rect = offset & configuration.size.expect("asserted above");
        let text_direction = configuration
            .text_direction
            .expect("an indicator is painted with a text direction");
        if let Some(border_radius) = self.border_radius {
            let paint = Paint::from_color(self.decoration.border_side.color.into());
            let indicator = self.decoration.indicator_rect_for(rect, text_direction);
            let rrect = RRect::from_rect_and_corners(
                indicator,
                border_radius.top_left,
                border_radius.top_right,
                border_radius.bottom_right,
                border_radius.bottom_left,
            );
            draw_rrect(canvas, rrect, &paint);
        } else {
            let mut paint = self.decoration.border_side.to_paint();
            if let PaintStyle::Stroke(stroke) = &mut paint.style {
                stroke.cap = StrokeCap::Square;
            }
            let indicator = self
                .decoration
                .indicator_rect_for(rect, text_direction)
                .deflate(self.decoration.border_side.width / 2.0);
            draw_line(
                canvas,
                indicator.bottom_left(),
                indicator.bottom_right(),
                &paint,
            );
        }
    }

    fn on_changed(&self) -> Option<&dyn Fn(&mut App)> {
        self.on_changed.as_deref()
    }
}
