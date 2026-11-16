//! Shared test scaffolding: a platform with one view to run widgets under, on the target
//! platform a case names, and pumped frames.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{ViewId, ViewRef};
use waft_foundation::{App, AppCell};
use waft_scheduler::SchedulerBinding;
use waft_test::TestView;
use waft_widgets::{BuildContext, Builder, IntoWidget, SizedBox, View, WidgetRef, run_widget};

/// The 800x600 physical view at 2x every test app draws into.
pub(crate) fn test_view() -> ViewRef {
    Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 2.0))
}

/// Mounts `child` under the platform's view and runs the first frame.
pub(crate) fn build(cell: &AppCell, child: WidgetRef) {
    {
        let mut app = cell.borrow_mut();
        let view = app
            .platform()
            .views()
            .into_iter()
            .next()
            .expect("an app from test_support::test_cell");
        run_widget(&mut app, View::new(view, child).into_widget());
    }
    // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
    cell.elapse(Duration::ZERO);
    pump(&mut cell.borrow_mut(), Duration::ZERO);
}

/// Runs one frame at `at`, of every view the platform has.
pub(crate) fn pump(app: &mut App, at: Duration) {
    let views: Vec<ViewId> = app
        .platform()
        .views()
        .iter()
        .map(|view| view.id())
        .collect();
    SchedulerBinding::handle_begin_frame(app, &views, Some(at));
    app.drain_microtasks();
    SchedulerBinding::handle_draw_frame(app, &views);
    app.drain_microtasks();
}

/// Pumps `wrap(probe)` and returns what `read` saw at the probe's
/// context.
pub(crate) fn probe<T: 'static>(
    tester: &mut waft_widgets::testing::WidgetTester,
    wrap: impl FnOnce(&App, WidgetRef) -> WidgetRef,
    read: impl Fn(&mut App, BuildContext) -> T + 'static,
) -> T {
    let seen = Rc::new(RefCell::new(None));
    let probe = Builder::new({
        let seen = Rc::clone(&seen);
        move |app, context| {
            *seen.borrow_mut() = Some(read(app, context));
            SizedBox::shrink().into_widget()
        }
    })
    .into_widget();
    let root = wrap(&tester.cell.borrow(), probe);
    tester.pump_widget(root);
    let value = seen.borrow_mut().take();
    value.expect("the probe built")
}

/// Dart's `MaterialApp(theme: theme, home: home)`.
pub(crate) fn material_app(theme: crate::theme_data::ThemeData, home: WidgetRef) -> WidgetRef {
    crate::app::MaterialApp::new()
        .theme(theme)
        .home(home)
        .into_widget()
}

/// Dart's `MaterialApp(restorationScopeId: restoration_scope_id, theme: theme, home: home)`.
pub(crate) fn restorable_material_app(
    restoration_scope_id: &str,
    theme: crate::theme_data::ThemeData,
    home: WidgetRef,
) -> WidgetRef {
    crate::app::MaterialApp::new()
        .restoration_scope_id(restoration_scope_id)
        .theme(theme)
        .home(home)
        .into_widget()
}

/// Dart's `MaterialApp(theme: theme, ..)` with the other arguments `configure` gives.
pub(crate) fn material_page_app(
    theme: crate::theme_data::ThemeData,
    configure: impl FnOnce(crate::app::MaterialApp) -> crate::app::MaterialApp,
) -> WidgetRef {
    configure(crate::app::MaterialApp::new().theme(theme)).into_widget()
}

/// What a case reads of what the tree painted, from valo's display lists: Dart's `paints`
/// matcher, until the recording canvas replaces it (the harness upgrade's step 4). The harness tester
/// in `waft_widgets::testing` reads it.
pub(crate) trait PaintInspection {
    /// The app the tester runs.
    fn app_cell(&self) -> &AppCell;

    /// What the ink features of the first material draw, painted as its `paint` paints them
    /// at the origin: Dart's `paints` matcher on `Material.of(context)`.
    fn ink_list(&self) -> waft_embedder::valo::DisplayList {
        let render = first_render::<crate::material::RenderInkFeatures>(self.app_cell());
        let mut canvas = waft_embedder::Canvas::new();
        render.paint_ink_features(
            &mut self.app_cell().borrow_mut(),
            &mut canvas,
            waft_embedder::Offset::ZERO,
        );
        canvas.build()
    }

    /// The draws of [`ink_list`](Self::ink_list).
    fn ink_drawn(&self) -> Vec<Drawn> {
        Drawn::draws(&self.ink_list())
    }

    /// What `render` and its subtree draw, painted at the origin: Dart's `paints` matcher.
    fn painted(&self, render: waft_rendering::AnyRenderObject) -> Vec<Drawn> {
        self.pictures(render)
            .iter()
            .flat_map(|picture| Drawn::draws(picture))
            .collect()
    }

    /// Every op `render` and its subtree record, painted at the origin, for a case that reads
    /// a path or a shadow: Dart's `paints` matcher with the calls it names (`drawCircle` is a
    /// path that `as_oval` answers, `drawShadow` an `Op::DrawShadow`).
    fn painted_ops(&self, render: waft_rendering::AnyRenderObject) -> Vec<waft_embedder::valo::Op> {
        self.pictures(render)
            .iter()
            .flat_map(|picture| waft_embedder::flattened_ops(picture))
            .collect()
    }

    /// Dart's `paints..drrect(color:, outer:, inner:)`: whether `render` or its subtree paints
    /// the ring between `outer` and `inner` in `color` that a `RoundedRectangleBorder` with a
    /// side paints.
    fn paints_drrect(
        &self,
        render: waft_rendering::AnyRenderObject,
        color: waft_embedder::Color,
        outer: waft_embedder::RRect,
        inner: waft_embedder::RRect,
    ) -> bool {
        use waft_embedder::valo::Op;
        let mut canvas = waft_embedder::Canvas::new();
        let paint = waft_embedder::Paint {
            color: color.into(),
            ..waft_embedder::Paint::default()
        };
        waft_painting::draw_drrect(&mut canvas, outer, inner, &paint);
        let ring = canvas.build();
        let Some(Op::DrawPath {
            path: ring_path,
            fill_rule: ring_fill_rule,
            paint: ring_paint,
            ..
        }) = ring.ops().first()
        else {
            panic!("draw_drrect records a path");
        };
        self.pictures(render).iter().any(|picture| {
            picture.ops().iter().any(|op| {
                matches!(
                    op,
                    Op::DrawPath { path, fill_rule, paint, .. }
                        if path == ring_path
                            && fill_rule == ring_fill_rule
                            && paint.color == ring_paint.color
                )
            })
        })
    }

    /// Dart's `paints..rrect(rrect:, color:)`: whether `render` or its subtree fills `rrect`,
    /// in `color` when it is given.
    fn paints_rrect(
        &self,
        render: waft_rendering::AnyRenderObject,
        rrect: waft_embedder::RRect,
        color: Option<waft_embedder::Color>,
    ) -> bool {
        use waft_embedder::valo::Op;
        let mut canvas = waft_embedder::Canvas::new();
        waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
        let expected = canvas.build();
        let Some(Op::DrawPath {
            path: expected_path,
            fill_rule: expected_fill_rule,
            ..
        }) = expected.ops().first()
        else {
            panic!("draw_rrect records a path");
        };
        let color: Option<waft_embedder::valo::Color> = color.map(Into::into);
        self.pictures(render).iter().any(|picture| {
            picture.ops().iter().any(|op| {
                matches!(
                    op,
                    Op::DrawPath { path, fill_rule, paint, .. }
                        if path == expected_path
                            && fill_rule == expected_fill_rule
                            && color.is_none_or(|color| paint.color == color)
                )
            })
        })
    }

    /// The pictures `render` and its subtree record, painted at the origin: the top-level
    /// pictures only, not those of the layers `render` pushes.
    fn pictures(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
        pictures_painted(self.app_cell(), render, false)
    }

    /// Every picture `render` records and every picture of the layers it pushes (repaint
    /// boundaries, transforms, clips and the rest), depth first in paint order: what Dart's
    /// `paints` reads from a `TestRecordingCanvas`, which paints a child that is a repaint
    /// boundary inline. Coordinates are those of each picture's own layer.
    fn pictures_deep(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
        pictures_painted(self.app_cell(), render, true)
    }

    /// Dart's `Material.of(tester.element(finder))` read by `paints`: what the ink features of
    /// the `Material` nearest above `element`, and their subtree, draw.
    fn nearest_material_ops(
        &self,
        element: waft_widgets::AnyElement,
    ) -> Vec<waft_embedder::valo::Op> {
        let ink_features = {
            let app = self.app_cell().borrow();
            waft_widgets::LookupBoundary::find_ancestor_render_object_of_type::<
                crate::material::RenderInkFeatures,
            >(&app, element)
            .expect("a Material above the element")
        };
        use waft_rendering::RenderBox;
        self.painted_ops_deep(ink_features.as_object())
    }

    /// The ops of [`pictures_deep`](Self::pictures_deep), flattened.
    fn painted_ops_deep(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<waft_embedder::valo::Op> {
        self.pictures_deep(render)
            .iter()
            .flat_map(|picture| waft_embedder::flattened_ops(picture))
            .collect()
    }
}

impl PaintInspection for waft_widgets::testing::WidgetTester {
    fn app_cell(&self) -> &AppCell {
        &self.cell
    }
}

/// Every render object of type `T` in the views of `cell`, in paint order.
fn render_objects<T: waft_rendering::RenderObject>(
    cell: &AppCell,
) -> Vec<waft_rendering::RenderHandle<T>> {
    let mut app = cell.borrow_mut();
    let binding = waft_rendering::RendererBinding::instance(&mut app);
    let mut found = Vec::new();
    for render_view in binding.render_views(&app) {
        collect(&app, render_view.as_object(), &mut found);
    }
    found
}

/// The first render object of type `T` in the views of `cell`.
fn first_render<T: waft_rendering::RenderObject>(
    cell: &AppCell,
) -> waft_rendering::RenderHandle<T> {
    render_objects(cell)
        .into_iter()
        .next()
        .expect("a render object of that type")
}

/// Paints `render` at the origin into a layer of its own and collects the pictures under
/// it, descending into pushed layers when `deep`.
fn pictures_painted(
    cell: &AppCell,
    render: waft_rendering::AnyRenderObject,
    deep: bool,
) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
    let mut app = cell.borrow_mut();
    assert!(
        !render.is_repaint_boundary(&app),
        "a repaint boundary would move its layer out of the scene"
    );
    use waft_rendering::ContainerLayer;
    let layer = waft_rendering::OffsetLayer::new(&mut app, waft_embedder::Offset::ZERO);
    let container = layer.as_container_layer();
    let bounds = render.paint_bounds(&app);
    let mut context = waft_rendering::PaintingContext::new(container, bounds);
    context.paint_child(&mut app, render, waft_embedder::Offset::ZERO);
    context.stop_recording_if_needed(&mut app);
    let mut pictures = Vec::new();
    collect_pictures(&app, container, deep, &mut pictures);
    pictures
}

/// Adds the pictures of the layers under `container` to `pictures`, in order, and those of the
/// layers nested under them, depth first, when `deep`.
fn collect_pictures(
    app: &App,
    container: waft_rendering::AnyContainerLayer,
    deep: bool,
    pictures: &mut Vec<std::sync::Arc<waft_embedder::Picture>>,
) {
    let mut child = container.first_child(app);
    while let Some(layer) = child {
        if let Some(picture) = app
            .handle::<waft_rendering::PictureLayer>(layer.id())
            .and_then(|picture| picture.picture(app))
        {
            pictures.push(std::sync::Arc::clone(picture));
        }
        if deep && let Some(nested) = layer.as_container_layer() {
            collect_pictures(app, nested, deep, pictures);
        }
        child = layer.next_sibling(app);
    }
}

/// The canvas method a recorded op stands for, as flutter_test's `TestRecordingCanvas` names
/// it. Valo records `drawCircle`, `drawRRect`, `drawPath` and `drawOval` as one op; they are
/// told apart by the path (an oval, a rounded rect, any other). A `drawDRRect` is the even-odd path
/// between two contours that `draw_drrect` records, and a `drawRRect` of a rounded rect without
/// corners a `drawRect` (`draw_rrect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanvasCall {
    DrawCircle,
    DrawRRect,
    DrawPath,
    DrawDRRect,
    DrawRect,
    DrawShadow,
    DrawParagraph,
    Scale,
    Other,
}

/// What `op` records, as the canvas method that made it.
pub(crate) fn canvas_call_of(op: &waft_embedder::valo::Op) -> CanvasCall {
    use waft_embedder::valo::Op;
    match op {
        Op::DrawPath {
            path, fill_rule, ..
        } => {
            if path.as_oval().is_some() {
                CanvasCall::DrawCircle
            } else if path.as_round_rect().is_some() {
                CanvasCall::DrawRRect
            } else if is_drrect(path, *fill_rule) {
                CanvasCall::DrawDRRect
            } else {
                CanvasCall::DrawPath
            }
        }
        Op::DrawRect { .. } => CanvasCall::DrawRect,
        Op::DrawShadow { .. } => CanvasCall::DrawShadow,
        Op::GlyphRun { .. } => CanvasCall::DrawParagraph,
        Op::Transform(matrix) => {
            let [a, b, c, d, tx, ty] = matrix.to_affine();
            // A scale by one is the identity, which a translation by zero is too: both read as a
            // scale, the call Dart's `scale` matcher means.
            if b == 0.0 && c == 0.0 && tx == 0.0 && ty == 0.0 && (a > 0.0 || d > 0.0) {
                CanvasCall::Scale
            } else {
                CanvasCall::Other
            }
        }
        _ => CanvasCall::Other,
    }
}

/// Whether `path`, filled with `fill_rule`, is what `draw_drrect` records for `drawDRRect`: the
/// area between two contours, filled even-odd. Valo keeps no record of the call, so a
/// `drawPath` of such a path reads as a `drawDRRect` too.
fn is_drrect(path: &waft_embedder::valo::Path, fill_rule: waft_embedder::valo::FillRule) -> bool {
    fill_rule == waft_embedder::valo::FillRule::EvenOdd && path.flatten(1.0).len() == 2
}

/// One call of Dart's `paints` matcher, each argument checked only when given.
#[derive(Clone, Debug)]
pub(crate) enum PaintCall {
    /// `..path(color:, style:, strokeWidth:)`.
    Path {
        color: Option<waft_embedder::Color>,
        stroke_width: Option<f64>,
        stroked: Option<bool>,
    },
    /// `..rect(color:)`.
    Rect {
        rect: Option<waft_embedder::Rect>,
        color: Option<waft_embedder::Color>,
    },
    /// `..rrect(rrect:, color:)`.
    RRect {
        rrect: Option<waft_embedder::RRect>,
        color: Option<waft_embedder::Color>,
    },
    /// `..drrect(color:)`.
    DRRect { color: Option<waft_embedder::Color> },
    /// `..circle(color:, style: PaintingStyle.stroke, strokeWidth:)`.
    StrokedCircle {
        color: Option<waft_embedder::Color>,
        stroke_width: f64,
    },
    /// `..circle(x:, y:, radius:, color:)`.
    Circle {
        x: Option<f64>,
        y: Option<f64>,
        radius: Option<f64>,
        color: Option<waft_embedder::Color>,
    },
    /// `..shadow(elevation:, color:)`.
    Shadow {
        elevation: Option<f64>,
        color: Option<waft_embedder::Color>,
    },
    /// `..paragraph()`.
    Paragraph,
    /// `..scale()`.
    Scale,
}

impl PaintCall {
    /// `..path(color:)`.
    pub(crate) fn path(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::Path {
            color,
            stroke_width: None,
            stroked: None,
        }
    }

    /// `..rect(color:)`.
    pub(crate) fn rect(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::Rect { rect: None, color }
    }

    /// `..rect(rect:, color:)`.
    pub(crate) fn rect_at(
        rect: waft_embedder::Rect,
        color: Option<waft_embedder::Color>,
    ) -> PaintCall {
        PaintCall::Rect {
            rect: Some(rect),
            color,
        }
    }

    /// `..rrect(color:)`.
    pub(crate) fn rrect(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::RRect { rrect: None, color }
    }

    /// `..rrect(rrect:, color:)`.
    pub(crate) fn rrect_at(
        rrect: waft_embedder::RRect,
        color: Option<waft_embedder::Color>,
    ) -> PaintCall {
        PaintCall::RRect {
            rrect: Some(rrect),
            color,
        }
    }

    /// `..drrect(color:)`.
    pub(crate) fn drrect(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::DRRect { color }
    }

    /// `..circle(color:, style: PaintingStyle.stroke, strokeWidth:)`.
    pub(crate) fn stroked_circle(color: waft_embedder::Color, stroke_width: f64) -> PaintCall {
        PaintCall::StrokedCircle {
            color: Some(color),
            stroke_width,
        }
    }

    /// `..circle(color:)`.
    pub(crate) fn circle(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::Circle {
            x: None,
            y: None,
            radius: None,
            color,
        }
    }

    /// `..shadow(color:)`.
    pub(crate) fn shadow(color: Option<waft_embedder::Color>) -> PaintCall {
        PaintCall::Shadow {
            elevation: None,
            color,
        }
    }

    /// Whether `op` is a call of this method, whatever its arguments.
    fn is_call(&self, op: &waft_embedder::valo::Op) -> bool {
        let call = canvas_call_of(op);
        match self {
            PaintCall::Path { .. } => call == CanvasCall::DrawPath,
            PaintCall::Rect { .. } => call == CanvasCall::DrawRect,
            // `draw_rrect` records a rounded rect without corners as `drawRect`.
            PaintCall::RRect { rrect, .. } => {
                call == CanvasCall::DrawRRect
                    || (call == CanvasCall::DrawRect && rrect.is_none_or(drawn_as_rect))
            }
            PaintCall::DRRect { .. } => call == CanvasCall::DrawDRRect,
            PaintCall::Circle { .. } | PaintCall::StrokedCircle { .. } => {
                call == CanvasCall::DrawCircle
            }
            PaintCall::Shadow { .. } => call == CanvasCall::DrawShadow,
            PaintCall::Paragraph => call == CanvasCall::DrawParagraph,
            PaintCall::Scale => call == CanvasCall::Scale,
        }
    }

    /// Whether `op`, a call of this method, has the arguments given, compared exactly as Dart's
    /// predicates do, at the f32 precision Valo records in.
    fn matches(&self, op: &waft_embedder::valo::Op) -> bool {
        use waft_embedder::valo::{Op, PaintStyle};
        let color_matches = |painted: waft_embedder::valo::Color,
                             expected: &Option<waft_embedder::Color>| {
            expected.is_none_or(|expected| colors_match(painted, expected))
        };
        match (self, op) {
            (
                PaintCall::Path {
                    color,
                    stroke_width,
                    stroked,
                },
                Op::DrawPath { paint, .. },
            ) => {
                let width = match &paint.style {
                    PaintStyle::Stroke(stroke) => Some(stroke.width),
                    PaintStyle::Fill => None,
                };
                color_matches(paint.color, color)
                    && stroked.is_none_or(|stroked| stroked == width.is_some())
                    && stroke_width
                        .is_none_or(|expected| width.is_some_and(|width| f32_is(width, expected)))
            }
            (PaintCall::Rect { rect, color }, Op::DrawRect { rect: painted, paint, .. }) => {
                color_matches(paint.color, color)
                    && rect.is_none_or(|rect| waft_embedder::valo::Rect::from(rect) == *painted)
            }
            (PaintCall::DRRect { color }, Op::DrawPath { paint, .. }) => {
                color_matches(paint.color, color)
            }
            (PaintCall::RRect { rrect, color }, Op::DrawPath { path, paint, .. }) => {
                color_matches(paint.color, color)
                    && rrect.is_none_or(|rrect| {
                        let mut canvas = waft_embedder::Canvas::new();
                        waft_painting::draw_rrect(
                            &mut canvas,
                            rrect,
                            &waft_embedder::Paint::default(),
                        );
                        let list = canvas.build();
                        matches!(list.ops().first(), Some(Op::DrawPath { path: expected, .. }) if expected == path)
                    })
            }
            (PaintCall::RRect { rrect, color }, Op::DrawRect { rect, paint, .. }) => {
                color_matches(paint.color, color)
                    && rrect.is_none_or(|rrect| {
                        waft_embedder::valo::Rect::from(rrect.outer_rect()) == *rect
                    })
            }
            (
                PaintCall::Circle {
                    x,
                    y,
                    radius,
                    color,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                let Some(oval) = path.as_oval() else {
                    return false;
                };
                color_matches(paint.color, color)
                    && x.is_none_or(|x| f32_is(oval.x + oval.width / 2.0, x))
                    && y.is_none_or(|y| f32_is(oval.y + oval.height / 2.0, y))
                    && radius.is_none_or(|radius| f32_is(oval.width / 2.0, radius))
            }
            (
                PaintCall::Shadow { elevation, color },
                Op::DrawShadow {
                    color: shadow,
                    elevation: shadow_elevation,
                    ..
                },
            ) => {
                color.is_none_or(|color| colors_match(*shadow, color))
                    && elevation.is_none_or(|elevation| f32_is(*shadow_elevation, elevation))
            }
            (
                PaintCall::StrokedCircle {
                    color,
                    stroke_width,
                },
                Op::DrawPath { paint, .. },
            ) => {
                color_matches(paint.color, color)
                    && matches!(&paint.style, PaintStyle::Stroke(stroke) if f32_is(stroke.width, *stroke_width))
            }
            (PaintCall::Paragraph, Op::GlyphRun { .. }) | (PaintCall::Scale, Op::Transform(_)) => {
                true
            }
            _ => false,
        }
    }
}

/// Whether `draw_rrect` records `rrect` as a `drawRect`: every corner's radii are zero.
fn drawn_as_rect(rrect: waft_embedder::RRect) -> bool {
    [
        rrect.tl_radius_x,
        rrect.tl_radius_y,
        rrect.tr_radius_x,
        rrect.tr_radius_y,
        rrect.br_radius_x,
        rrect.br_radius_y,
        rrect.bl_radius_x,
        rrect.bl_radius_y,
    ]
    .iter()
    .all(|radius| *radius == 0.0)
}

/// Dart's exact `==` on a value Valo recorded as an `f32`: equal within that value's rounding.
fn f32_is(recorded: f32, expected: f64) -> bool {
    (f64::from(recorded) - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0)
}

/// flutter_test's `_colorsMatch`: each channel within 1/255.
pub(crate) fn colors_match(
    painted: waft_embedder::valo::Color,
    expected: waft_embedder::Color,
) -> bool {
    const LIMIT: f64 = 1.0 / 255.0;
    (f64::from(painted.a) - expected.a).abs() < LIMIT
        && (f64::from(painted.r) - expected.r).abs() < LIMIT
        && (f64::from(painted.g) - expected.g).abs() < LIMIT
        && (f64::from(painted.b) - expected.b).abs() < LIMIT
}

/// Dart's `expect(render, paints..a..b..)`: each call is the next call of its method after the
/// last one matched, and has the arguments given; other methods' calls between are skipped.
pub(crate) fn paints_calls(ops: &[waft_embedder::valo::Op], calls: &[PaintCall]) -> bool {
    let mut ops = ops.iter();
    calls.iter().all(|expected| {
        ops.by_ref()
            .find(|op| expected.is_call(op))
            .is_some_and(|op| expected.matches(op))
    })
}

/// Dart's `paintsExactlyCountTimes(method, count)`: how many calls of `call` the ops record.
pub(crate) fn count_calls(ops: &[waft_embedder::valo::Op], call: CanvasCall) -> usize {
    ops.iter().filter(|op| canvas_call_of(op) == call).count()
}

/// Dart's `isSameColorAs` matcher, with its default threshold.
pub(crate) fn is_same_color_as(a: waft_embedder::Color, b: waft_embedder::Color) -> bool {
    const THRESHOLD: f64 = 0.004;
    (a.a - b.a).abs() <= THRESHOLD
        && (a.r - b.r).abs() <= THRESHOLD
        && (a.g - b.g).abs() <= THRESHOLD
        && (a.b - b.b).abs() <= THRESHOLD
}

fn collect<T: waft_rendering::RenderObject>(
    app: &App,
    node: waft_rendering::AnyRenderObject,
    found: &mut Vec<waft_rendering::RenderHandle<T>>,
) {
    if let Some(typed) = node.downcast::<T>(app) {
        found.push(typed);
    }
    node.visit_children(app, &mut |child| collect(app, child, found));
}

/// One recorded canvas op, reduced to what Dart's `paints` matchers compare.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drawn {
    /// `drawRect`: its colour as `0xAARRGGBB` and its rect.
    Rect(u32, waft_embedder::Rect),
    /// A path (`drawCircle`, `drawRRect`, `drawPath`): its colour and its local bounds.
    Path(u32, waft_embedder::Rect),
    /// `drawPaint`: its colour.
    Paint(u32),
    /// A clip.
    Clip,
    /// `save` / `restore` / a transform.
    State,
}

fn argb(color: waft_embedder::valo::Color) -> u32 {
    let channel = |value: f32| (value * 255.0).round() as u32;
    (channel(color.a) << 24) | (channel(color.r) << 16) | (channel(color.g) << 8) | channel(color.b)
}

fn rect_of(rect: waft_embedder::valo::Rect) -> waft_embedder::Rect {
    waft_embedder::Rect::from_ltwh(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.width),
        f64::from(rect.height),
    )
}

impl Drawn {
    /// The ops of `list`, reduced.
    pub(crate) fn all(list: &waft_embedder::valo::DisplayList) -> Vec<Drawn> {
        use waft_embedder::valo::Op;
        list.ops()
            .iter()
            .filter_map(|op| match op {
                Op::DrawRect { paint, rect, .. } => {
                    Some(Drawn::Rect(argb(paint.color), rect_of(*rect)))
                }
                Op::DrawPath {
                    paint,
                    content_bounds,
                    ..
                } => Some(Drawn::Path(argb(paint.color), rect_of(*content_bounds))),
                Op::DrawPaint { paint, .. } => Some(Drawn::Paint(argb(paint.color))),
                Op::ClipPath { .. } => Some(Drawn::Clip),
                Op::Save | Op::Restore | Op::Transform(_) => Some(Drawn::State),
                _ => None,
            })
            .collect()
    }

    /// The draws of `list`, without clips and canvas state.
    pub(crate) fn draws(list: &waft_embedder::valo::DisplayList) -> Vec<Drawn> {
        Drawn::all(list)
            .into_iter()
            .filter(|drawn| !matches!(drawn, Drawn::Clip | Drawn::State))
            .collect()
    }
}
