//! flutter's `rendering_tester.dart`: [`layout`] and [`pump_frame`] over a render tree of its own,
//! for a test of render objects with no widget tree.
//!
//! Dart's `TestRenderingFlutterBinding` puts the box under its `RenderView`, an 800x600 test view,
//! and flushes the pipeline owner up to the phase asked. Here each `App` has one such root, made
//! when a case first lays a box out: a repaint boundary laid out tight to 800x600, with its own
//! pipeline owner and its paint layer attached, standing in for the render view.

use waft_embedder::{Offset, Size};
use waft_foundation::{App, Handle, Ref};
use waft_painting::{AlignmentGeometry, PaintingBinding};

use crate::{
    AnyRenderBox, BoxConstraints, ContainerLayer, ErasedLayer, OffsetLayer, PipelineOwner,
    RenderBox, RenderConstrainedBox, RenderHandle, RenderObjectWithChildMixin, RenderPositionedBox,
    RenderRepaintBoundary,
};

/// Installs flutter_test's fonts if the app has none: what
/// `TestRenderingFlutterBinding` does before a case lays text out.
pub fn install_test_fonts(app: &mut App) {
    let binding = PaintingBinding::instance(app);
    if !binding.has_fonts(app) {
        binding.install_flutter_test_fonts(app);
    }
}

/// The size of the test view, which the root lays its child out at.
pub const TEST_VIEW_SIZE: Size = Size::new(800.0, 600.0);

/// Dart's `EnginePhase`: how far [`pump_frame`] takes a frame. Each phase runs the ones before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnginePhase {
    /// `flushLayout`.
    Layout,
    /// `flushCompositingBits`.
    CompositingBits,
    /// `flushPaint`.
    Paint,
}

/// The root a case's boxes are laid out under: Dart's `TestRenderingFlutterBinding.renderView`.
#[derive(Default)]
struct TestRenderRoot {
    owner: Option<Ref<PipelineOwner>>,
    root: Option<RenderHandle<RenderRepaintBoundary>>,
    paint_layer: Option<Handle<OffsetLayer>>,
}

/// The app's test root, made with its pipeline owner and paint layer the first time.
fn test_root(app: &mut App) -> (Handle<PipelineOwner>, RenderHandle<RenderRepaintBoundary>) {
    install_test_fonts(app);
    let this: Handle<TestRenderRoot> = app.singleton();
    if let (Some(owner), Some(root)) = (&app.get(this).owner, app.get(this).root) {
        return (owner.handle(), root);
    }
    let root = RenderRepaintBoundary::new(app, None);
    let owner = PipelineOwner::new(app, None, None);
    owner.set_root_node(app, Some(root.as_object()));
    root.as_box()
        .layout(app, BoxConstraints::tight(TEST_VIEW_SIZE), false);
    let paint_layer = OffsetLayer::new(app, Offset::ZERO);
    paint_layer.as_layer().attach(app, root.as_object().id());
    root.as_object()
        .schedule_initial_paint(app, paint_layer.as_container_layer());
    let handle = owner.handle();
    let data = app.get_mut(this);
    data.owner = Some(owner);
    data.root = Some(root);
    data.paint_layer = Some(paint_layer);
    (handle, root)
}

/// Dart's `layout(box)`: makes `render_box` the test view's child, laid out tight to the view,
/// and pumps a frame through layout.
pub fn layout(app: &mut App, render_box: AnyRenderBox) {
    layout_with(
        app,
        render_box,
        None,
        AlignmentGeometry::CENTER,
        EnginePhase::Layout,
    );
}

/// Dart's `layout(box, constraints:, alignment:, phase:)`: with `constraints`, the box is laid
/// out within them, placed in the view at `alignment`; the frame is pumped through `phase`.
pub fn layout_with(
    app: &mut App,
    render_box: AnyRenderBox,
    constraints: Option<BoxConstraints>,
    alignment: AlignmentGeometry,
    phase: EnginePhase,
) {
    debug_assert!(
        render_box.as_object().parent(app).is_none(),
        "a laid out box has no parent"
    );
    let (_, root) = test_root(app);
    root.set_child(app, None);
    let child = match constraints {
        Some(constraints) => {
            let constrained = RenderConstrainedBox::new(app, constraints, Some(render_box));
            RenderPositionedBox::new(app, alignment, None, None, None, Some(constrained.as_box()))
                .as_box()
        }
        None => render_box,
    };
    root.set_child(app, Some(child));
    root.as_box()
        .layout(app, BoxConstraints::tight(TEST_VIEW_SIZE), false);
    pump_frame(app, phase);
}

/// Dart's `pumpFrame(phase:)`: flushes the test root's pipeline owner through `phase`.
pub fn pump_frame(app: &mut App, phase: EnginePhase) {
    let (owner, _) = test_root(app);
    owner.flush_layout(app);
    if phase == EnginePhase::Layout {
        return;
    }
    owner.flush_compositing_bits(app);
    if phase == EnginePhase::CompositingBits {
        return;
    }
    owner.flush_paint(app);
}

/// The test view's root: Dart's `TestRenderingFlutterBinding.instance.renderView`, whose child
/// [`layout`] set.
pub fn test_render_root(app: &mut App) -> RenderHandle<RenderRepaintBoundary> {
    test_root(app).1
}

/// The pipeline owner of the test view's tree.
pub fn test_pipeline_owner(app: &mut App) -> Handle<PipelineOwner> {
    test_root(app).0
}
