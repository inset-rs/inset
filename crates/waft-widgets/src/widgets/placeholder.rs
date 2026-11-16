//! Flutter `widgets/placeholder.dart`.

use std::any::Any;
use std::fmt::{self, Debug};

use waft_embedder::valo::FillRule;
use waft_embedder::{Canvas, Color, Offset, Paint, PaintStyle, PathBuilder, Size, Stroke};
use waft_foundation::App;
use waft_rendering::CustomPainter;

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{CustomPaint, LimitedBox};

struct PlaceholderPainter {
    color: Color,
    stroke_width: f64,
}

impl CustomPainter for PlaceholderPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let paint = Paint {
            color: self.color.into(),
            style: PaintStyle::Stroke(Stroke::new(self.stroke_width as f32)),
            ..Paint::default()
        };
        let rect = Offset::ZERO & size;
        let mut path = PathBuilder::new();
        path.rect(rect.into());
        path.move_to(rect.top_right());
        path.line_to(rect.bottom_left());
        path.move_to(rect.top_left());
        path.line_to(rect.bottom_right());
        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let Some(old_painter) = old_painter.as_any().downcast_ref::<PlaceholderPainter>() else {
            return true;
        };
        old_painter.color != self.color || old_painter.stroke_width != self.stroke_width
    }

    fn hit_test(&self, _app: &App, _position: Offset) -> Option<bool> {
        Some(false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A widget that draws a box that represents where other widgets will one day be added.
///
/// This widget is useful during development to indicate that the interface is not yet
/// complete.
///
/// By default, the placeholder is sized to fit its container. If the placeholder is in an
/// unbounded space, it will size itself according to the given
/// [`fallback_width`](Self::fallback_width) and [`fallback_height`](Self::fallback_height).
pub struct Placeholder {
    pub key: Option<KeyRef>,
    /// The color to draw the placeholder box.
    pub color: Color,
    /// The width of the lines in the placeholder box.
    pub stroke_width: f64,
    /// The width to use when the placeholder is in a situation with an unbounded width.
    ///
    /// See also:
    ///
    ///  * [`fallback_height`](Self::fallback_height), the same but vertically.
    pub fallback_width: f64,
    /// The height to use when the placeholder is in a situation with an unbounded height.
    ///
    /// See also:
    ///
    ///  * [`fallback_width`](Self::fallback_width), the same but horizontally.
    pub fallback_height: f64,
    /// The widget below this widget in the tree.
    ///
    /// The placeholder is drawn below the child, so the child stays visible.
    pub child: Option<WidgetRef>,
}

impl Placeholder {
    /// Creates a widget which draws a box.
    pub fn new() -> Placeholder {
        Placeholder::default()
    }

    /// Dart `Placeholder(key:)`.
    pub fn key(mut self, key: KeyRef) -> Placeholder {
        self.key = Some(key);
        self
    }

    /// Dart `Placeholder(color:)`.
    pub fn color(mut self, color: Color) -> Placeholder {
        self.color = color;
        self
    }

    /// Dart `Placeholder(strokeWidth:)`.
    pub fn stroke_width(mut self, stroke_width: f64) -> Placeholder {
        self.stroke_width = stroke_width;
        self
    }

    /// Dart `Placeholder(fallbackWidth:)`.
    pub fn fallback_width(mut self, fallback_width: f64) -> Placeholder {
        self.fallback_width = fallback_width;
        self
    }

    /// Dart `Placeholder(fallbackHeight:)`.
    pub fn fallback_height(mut self, fallback_height: f64) -> Placeholder {
        self.fallback_height = fallback_height;
        self
    }

    /// Dart `Placeholder(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Placeholder {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for Placeholder {
    fn default() -> Placeholder {
        Placeholder {
            key: None,
            color: Color::new(0xFF455A64), // Blue Grey 700
            stroke_width: 2.0,
            fallback_width: 400.0,
            fallback_height: 400.0,
            child: None,
        }
    }
}

impl StatelessWidget for Placeholder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut custom_paint =
            CustomPaint::new()
                .size(Size::INFINITE)
                .painter(PlaceholderPainter {
                    color: self.color,
                    stroke_width: self.stroke_width,
                });
        if let Some(child) = &self.child {
            custom_paint = custom_paint.child(child.clone());
        }
        LimitedBox::new()
            .max_width(self.fallback_width)
            .max_height(self.fallback_height)
            .child(custom_paint)
            .into_widget()
    }
}

impl Debug for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Placeholder")
            .field("color", &self.color)
            .field("stroke_width", &self.stroke_width)
            .field("fallback_width", &self.fallback_width)
            .field("fallback_height", &self.fallback_height)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::TextDirection;
    use waft_embedder::valo::Op;
    use waft_rendering::{
        RenderBox, RenderCustomPaint, RenderLimitedBox, RenderObjectWithChildMixin,
    };

    use super::*;
    use crate::framework::GlobalKey;
    use crate::testing::{WidgetTester, test_widgets};
    use crate::widgets::basic::{Center, Directionality, Positioned, SizedBox, Stack};
    use crate::widgets::text::Text;

    /// The `drawPath` calls of the placeholder's painter at its size: Dart's `paints..path()`.
    fn painted_paths(tester: &WidgetTester, key: &GlobalKey) -> Vec<Paint> {
        let render = tester.render_object(key);
        let mut app = tester.cell.borrow_mut();
        let custom_paint = render
            .downcast::<RenderLimitedBox>(&app)
            .and_then(|limited_box| limited_box.child(&app))
            .and_then(|child| child.as_object().downcast::<RenderCustomPaint>(&app))
            .expect("the placeholder's custom paint");
        let size = custom_paint.size(&app);
        let painter = custom_paint.painter(&app).expect("the placeholder painter");
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, size);
        canvas
            .build()
            .ops()
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { paint, .. } => Some(paint.clone()),
                _ => None,
            })
            .collect()
    }

    fn keyed_placeholder(key: &GlobalKey) -> Placeholder {
        Placeholder::new().key(key.clone().into())
    }

    #[test]
    fn placeholder() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(keyed_placeholder(&key));
            assert_eq!(tester.get_size(&key), Size::new(800.0, 600.0));
            tester.pump_widget(Center::new().child(keyed_placeholder(&key)));
            assert_eq!(tester.get_size(&key), Size::new(800.0, 600.0));
            tester.pump_widget(
                Stack::new()
                    .text_direction(TextDirection::Ltr)
                    .children([Positioned::new(keyed_placeholder(&key))
                        .top(0.0)
                        .bottom(0.0)
                        .into_widget()]),
            );
            assert_eq!(tester.get_size(&key), Size::new(400.0, 600.0));
            tester.pump_widget(
                Stack::new()
                    .text_direction(TextDirection::Ltr)
                    .children([Positioned::new(keyed_placeholder(&key))
                        .left(0.0)
                        .right(0.0)
                        .into_widget()]),
            );
            assert_eq!(tester.get_size(&key), Size::new(800.0, 400.0));
            tester.pump_widget(
                Stack::new()
                    .text_direction(TextDirection::Ltr)
                    .children([Positioned::new(
                        keyed_placeholder(&key)
                            .fallback_width(200.0)
                            .fallback_height(300.0),
                    )
                    .top(0.0)
                    .into_widget()]),
            );
            assert_eq!(tester.get_size(&key), Size::new(200.0, 300.0));
        });
    }

    #[test]
    fn placeholder_color() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(keyed_placeholder(&key));
            let paths = painted_paths(tester, &key);
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].color, Color::new(0xFF455A64).into());
            tester.pump_widget(keyed_placeholder(&key).color(Color::new(0xFF00FF00)));
            assert_eq!(
                painted_paths(tester, &key)[0].color,
                Color::new(0xFF00FF00).into()
            );
        });
    }

    #[test]
    fn placeholder_stroke_width() {
        let stroke_width = |paint: &Paint| match &paint.style {
            PaintStyle::Stroke(stroke) => Some(stroke.width),
            PaintStyle::Fill => None,
        };
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(keyed_placeholder(&key));
            assert_eq!(stroke_width(&painted_paths(tester, &key)[0]), Some(2.0));
            tester.pump_widget(keyed_placeholder(&key).stroke_width(10.0));
            assert_eq!(stroke_width(&painted_paths(tester, &key)[0]), Some(10.0));
        });
    }

    #[test]
    fn placeholder_child_widget() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let label = GlobalKey::new();
            tester.pump_widget(keyed_placeholder(&key));
            assert!(label.current_context(&tester.cell.borrow()).is_none());
            tester.pump_widget(keyed_placeholder(&key).child(Directionality::new(
                TextDirection::Ltr,
                Text::new("Label").key(label.clone().into()),
            )));
            assert!(label.current_context(&tester.cell.borrow()).is_some());
        });
    }

    #[test]
    fn placeholder_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(keyed_placeholder(&key))),
            ));
            assert_eq!(tester.get_size(&key), Size::ZERO);
        });
    }
}
