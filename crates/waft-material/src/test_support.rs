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

/// What a case reads of what the tree painted, through the harness tester in
/// `waft_widgets::testing`: the ops `waft_rendering::testing` paints a render object into, for its
/// `paints` matcher, and Material's ink. `painted` and `ink_drawn` are the `Drawn` reading, which
/// harness C's sweep moves onto that matcher.
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
        waft_rendering::testing::painted_ops(&mut self.app_cell().borrow_mut(), render)
    }

    /// The pictures `render` and its subtree record, painted at the origin: the top-level
    /// pictures only, not those of the layers `render` pushes.
    fn pictures(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
        waft_rendering::testing::pictures(&mut self.app_cell().borrow_mut(), render)
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

    /// Every op `render` records and every op of the layers it pushes, flattened: see
    /// `waft_rendering::testing::pictures_deep`.
    fn painted_ops_deep(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<waft_embedder::valo::Op> {
        waft_rendering::testing::painted_ops_deep(&mut self.app_cell().borrow_mut(), render)
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
