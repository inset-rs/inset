//! Flutter counterpart: `rendering/custom_paint.dart` (`CustomPainter`, `RenderCustomPaint`).
//!
//! Semantics (`SemanticsBuilderCallback`, `CustomPainterSemantics`,
//! `CustomPainter.semanticsBuilder` / `shouldRebuildSemantics`) waits.

use std::any::Any;
use std::rc::Rc;

use reveal_embedder::{Canvas, Matrix4, Offset, Size, TextBaseline};
use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_painting::ClipContext;

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult, RenderBox, RenderBoxData};
use crate::object::{
    AnyRenderObject, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::proxy_box::{RenderProxyBoxMixin, same_instance};

/// The interface used by `CustomPaint` (in the widgets library) and
/// [`RenderCustomPaint`] (in the rendering library).
///
/// To implement a custom painter, implement this trait to define your custom paint delegate.
/// A [`CustomPainter`] must implement the [`paint`](Self::paint) and
/// [`should_repaint`](Self::should_repaint) methods, and may optionally also implement the
/// [`hit_test`](Self::hit_test) method.
///
/// The [`paint`](Self::paint) method is called whenever the custom object needs to be
/// repainted.
///
/// The [`should_repaint`](Self::should_repaint) method is called when a new instance of the
/// class is provided, to check if the new instance actually represents different information.
///
/// The most efficient way to trigger a repaint is to supply a [`repaint`](Self::repaint)
/// listenable, which notifies its listeners when it is time to repaint: the
/// [`RenderCustomPaint`] listens to it and repaints whenever the animation ticks, avoiding both
/// the build and layout phases of the pipeline.
///
/// The [`hit_test`](Self::hit_test) method is called when the user interacts with the
/// underlying render object, to determine if the user hit the object or missed it.
///
/// ## Composition and the sharing of canvases
///
/// Widgets (or rather, render objects) are composited together using a minimum
/// number of [`Canvas`]es, for performance reasons. As a result, a
/// [`CustomPainter`]'s [`Canvas`] may be the same as that used by other widgets
/// (including other `CustomPaint` widgets).
///
/// This is mostly unnoticeable, except when using unusual blend modes. For
/// example, trying to use `BlendMode::DstOut` to "punch a hole" through a
/// previously-drawn image may erase more than was intended, because previous
/// widgets will have been painted onto the same canvas.
///
/// To avoid this issue, consider using `Canvas::save_layer` and
/// `Canvas::restore` when using such blend modes. Creating new layers is
/// relatively expensive, however, and should be done sparingly to avoid
/// introducing jank.
pub trait CustomPainter: 'static {
    /// The [`Listenable`] that notifies when it is time to repaint.
    ///
    /// Dart's `CustomPainter` takes it as a constructor argument and forwards `addListener` /
    /// `removeListener` to it; here the painter answers with it and the same forwarding is the
    /// [`Listenable`] implementation of `dyn CustomPainter`.
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        None
    }

    /// Called whenever the object needs to paint. The given [`Canvas`] has its
    /// coordinate space configured such that the origin is at the top left of the
    /// box. The area of the box is the size of the `size` argument.
    ///
    /// Paint operations should remain inside the given area. Graphical
    /// operations outside the bounds may be silently ignored, clipped, or not
    /// clipped. It may sometimes be difficult to guarantee that a certain
    /// operation is inside the bounds (e.g., drawing a rectangle whose size is
    /// determined by user inputs). In that case, consider calling
    /// `Canvas::clip_rect` at the beginning of `paint` so everything that follows
    /// will be guaranteed to only draw within the clipped area.
    ///
    /// Implementations should be wary of correctly pairing any calls to
    /// `Canvas::save` / `Canvas::save_layer` and `Canvas::restore`, otherwise all
    /// subsequent painting on this canvas may be affected, with potentially
    /// hilarious but confusing results.
    ///
    /// To paint text on a [`Canvas`], use a `TextPainter`.
    ///
    /// `app` reaches what a Dart painter reads directly: an animation's value, the fonts.
    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size);

    /// Called whenever a new instance of the custom painter delegate class is
    /// provided to the [`RenderCustomPaint`] object, or any time that a new
    /// `CustomPaint` object is created with a new instance of the custom painter
    /// delegate class (which amounts to the same thing, because the latter is
    /// implemented in terms of the former).
    ///
    /// If the new instance represents different information than the old
    /// instance, then the method should return true, otherwise it should return
    /// false.
    ///
    /// If the method returns false, then the [`paint`](Self::paint) call might be optimized
    /// away.
    ///
    /// It's possible that the [`paint`](Self::paint) method will get called even if
    /// [`should_repaint`](Self::should_repaint) returns false (e.g. if an ancestor or
    /// descendant needed to be repainted). It's also possible that the [`paint`](Self::paint)
    /// method will get called without [`should_repaint`](Self::should_repaint) being called at
    /// all (e.g. if the box changes size).
    ///
    /// If a custom delegate has a particularly expensive paint function such that
    /// repaints should be avoided as much as possible, a `RepaintBoundary` or
    /// [`crate::RenderRepaintBoundary`] (or other render object with
    /// [`RenderObject::is_repaint_boundary`] set to true) might be helpful.
    ///
    /// `old_delegate` is only ever a painter of this painter's own type, as Dart's `covariant`
    /// parameter is.
    fn should_repaint(&self, app: &App, old_delegate: &dyn CustomPainter) -> bool;

    /// Called whenever a hit test is being performed on an object that is using
    /// this custom paint delegate.
    ///
    /// The given point is relative to the same coordinate space as the last
    /// [`paint`](Self::paint) call.
    ///
    /// The default behavior is to consider all points to be hits for
    /// background painters, and no points to be hits for foreground painters.
    ///
    /// Return `Some(true)` if the given position corresponds to a point on the drawn
    /// image that should be considered a "hit", `Some(false)` if it corresponds to a
    /// point that should be considered outside the painted image, and `None` to use
    /// the default behavior.
    fn hit_test(&self, app: &App, position: Offset) -> Option<bool> {
        let _ = (app, position);
        None
    }

    /// Downcast support for Dart's `runtimeType` comparison and `covariant` parameter.
    fn as_any(&self) -> &dyn Any;
}

impl Listenable for dyn CustomPainter {
    /// Register a closure to be notified when it is time to repaint.
    ///
    /// Forwards to the [`repaint`](CustomPainter::repaint) listenable, if it is not `None`.
    fn add_listener(&self, app: &mut App, listener: Listener) {
        if let Some(repaint) = self.repaint() {
            repaint.add_listener(app, listener);
        }
    }

    /// Remove a previously registered closure from the list of closures that the
    /// object notifies when it is time to repaint.
    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        if let Some(repaint) = self.repaint() {
            repaint.remove_listener(app, listener);
        }
    }
}

/// Delegates its painting to a [`painter`](Self::painter) and a
/// [`foreground_painter`](Self::foreground_painter).
pub struct RenderCustomPaint {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    painter: Option<Rc<dyn CustomPainter>>,
    foreground_painter: Option<Rc<dyn CustomPainter>>,
    preferred_size: Size,
    is_complex: bool,
    will_change: bool,
}

impl RenderCustomPaint {
    /// Creates a render object that delegates its painting.
    pub fn new(
        app: &mut App,
        painter: Option<Rc<dyn CustomPainter>>,
        foreground_painter: Option<Rc<dyn CustomPainter>>,
        preferred_size: Size,
        is_complex: bool,
        will_change: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderCustomPaint> {
        let this = RenderHandle::new_box(
            app,
            RenderCustomPaint {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                painter,
                foreground_painter,
                preferred_size,
                is_complex,
                will_change,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The background custom paint delegate.
    ///
    /// This painter, if non-`None`, is called to paint behind the children.
    pub fn painter(self: RenderHandle<Self>, app: &App) -> Option<Rc<dyn CustomPainter>> {
        self.get(app).painter.clone()
    }

    /// Set a new background custom paint delegate.
    ///
    /// If the new delegate is the same as the previous one, this does nothing.
    ///
    /// If the new delegate is the same class as the previous one, then the new
    /// delegate has its [`CustomPainter::should_repaint`] called; if the result is
    /// true, then the delegate will be called.
    ///
    /// If the new delegate is a different class than the previous one, then the
    /// delegate will be called.
    ///
    /// If the new value is `None`, then there is no background custom painter.
    pub fn set_painter(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<Rc<dyn CustomPainter>>,
    ) {
        let old_painter = self.get(app).painter.clone();
        if same_instance(old_painter.as_ref(), value.as_ref()) {
            return;
        }
        self.get_mut(app).painter = value.clone();
        self.did_update_painter(app, value.as_ref(), old_painter.as_ref());
    }

    /// The foreground custom paint delegate.
    ///
    /// This painter, if non-`None`, is called to paint in front of the children.
    pub fn foreground_painter(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<Rc<dyn CustomPainter>> {
        self.get(app).foreground_painter.clone()
    }

    /// Set a new foreground custom paint delegate.
    ///
    /// If the new delegate is the same as the previous one, this does nothing.
    ///
    /// If the new delegate is the same class as the previous one, then the new
    /// delegate has its [`CustomPainter::should_repaint`] called; if the result is
    /// true, then the delegate will be called.
    ///
    /// If the new delegate is a different class than the previous one, then the
    /// delegate will be called.
    ///
    /// If the new value is `None`, then there is no foreground custom painter.
    pub fn set_foreground_painter(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<Rc<dyn CustomPainter>>,
    ) {
        let old_painter = self.get(app).foreground_painter.clone();
        if same_instance(old_painter.as_ref(), value.as_ref()) {
            return;
        }
        self.get_mut(app).foreground_painter = value.clone();
        self.did_update_painter(app, value.as_ref(), old_painter.as_ref());
    }

    fn did_update_painter(
        self: RenderHandle<Self>,
        app: &mut App,
        new_painter: Option<&Rc<dyn CustomPainter>>,
        old_painter: Option<&Rc<dyn CustomPainter>>,
    ) {
        // Check if we need to repaint.
        let repaint = match (new_painter, old_painter) {
            (None, old_painter) => {
                debug_assert!(old_painter.is_some()); // We should be called only for changes.
                true
            }
            (Some(new_painter), Some(old_painter)) => {
                new_painter.as_any().type_id() != old_painter.as_any().type_id()
                    || new_painter.should_repaint(app, &**old_painter)
            }
            (Some(_), None) => true,
        };
        if repaint {
            self.mark_needs_paint(app);
        }
        if self.attached(app) {
            if let Some(old_painter) = old_painter {
                old_painter.remove_listener(app, &self.paint_listener());
            }
            if let Some(new_painter) = new_painter {
                new_painter.add_listener(app, self.paint_listener());
            }
        }
        // Dart also rebuilds semantics here; semantics waits.
    }

    /// The `markNeedsPaint` tear-off, equal to itself across registrations.
    fn paint_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), mark_needs_paint)
    }

    /// The size that this [`RenderCustomPaint`] should aim for, given the layout
    /// constraints, if there is no child.
    ///
    /// Defaults to [`Size::ZERO`].
    ///
    /// If there's a child, this is ignored, and the size of the child is used
    /// instead.
    pub fn preferred_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).preferred_size
    }

    /// Sets [`preferred_size`](Self::preferred_size).
    pub fn set_preferred_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.preferred_size(app) == value {
            return;
        }
        self.get_mut(app).preferred_size = value;
        self.mark_needs_layout(app);
    }

    /// Whether to hint that this layer's painting should be cached.
    ///
    /// The compositor contains a raster cache that holds bitmaps of layers in
    /// order to avoid the cost of repeatedly rendering those layers on each
    /// frame. If this flag is not set, then the compositor will apply its own
    /// heuristics to decide whether the layer containing this render object is
    /// complex enough to benefit from caching.
    pub fn is_complex(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_complex
    }

    /// Sets [`is_complex`](Self::is_complex). Dart's plain field.
    pub fn set_is_complex(self: RenderHandle<Self>, app: &mut App, value: bool) {
        self.get_mut(app).is_complex = value;
    }

    /// Whether the raster cache should be told that this painting is likely
    /// to change in the next frame.
    ///
    /// This hint tells the compositor not to cache the layer containing this
    /// render object because the cache will not be used in the future. If this
    /// hint is not set, the compositor will apply its own heuristics to decide
    /// whether this layer is likely to be reused in the future.
    pub fn will_change(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).will_change
    }

    /// Sets [`will_change`](Self::will_change). Dart's plain field.
    pub fn set_will_change(self: RenderHandle<Self>, app: &mut App, value: bool) {
        self.get_mut(app).will_change = value;
    }

    fn paint_with_painter(
        self: RenderHandle<Self>,
        app: &mut App,
        canvas: &mut Canvas,
        offset: Offset,
        painter: &dyn CustomPainter,
    ) {
        canvas.save();
        let debug_previous_canvas_save_count = canvas.save_count();
        if offset != Offset::ZERO {
            canvas.translate(offset.dx() as f32, offset.dy() as f32);
        }
        let size = self.size(app);
        painter.paint(app, canvas, size);
        // This isn't perfect. For example, we can't catch the case of
        // someone first restoring, then setting a transform or whatnot,
        // then saving.
        debug_assert_eq!(
            debug_previous_canvas_save_count,
            canvas.save_count(),
            "the custom painter had mismatching save and restore calls"
        );
        canvas.restore();
    }

    fn set_raster_cache_hints(self: RenderHandle<Self>, app: &App, context: &mut PaintingContext) {
        if self.get(app).is_complex {
            context.set_is_complex_hint();
        }
        if self.get(app).will_change {
            context.set_will_change_hint();
        }
    }
}

fn mark_needs_paint(this: Handle<RenderCustomPaint>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

impl RenderObjectWithChildMixin for RenderCustomPaint {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderProxyBoxMixin for RenderCustomPaint {
    fn compute_size_for_no_child(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.constrain(self.get(app).preferred_size)
    }
}

impl RenderObject for RenderCustomPaint {
    crate::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        if let Some(painter) = self.painter(app) {
            painter.add_listener(app, self.paint_listener());
        }
        if let Some(foreground_painter) = self.foreground_painter(app) {
            foreground_painter.add_listener(app, self.paint_listener());
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(painter) = self.painter(app) {
            painter.remove_listener(app, &self.paint_listener());
        }
        if let Some(foreground_painter) = self.foreground_painter(app) {
            foreground_painter.remove_listener(app, &self.paint_listener());
        }
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(painter) = self.painter(app) {
            self.paint_with_painter(app, context.canvas(), offset, &*painter);
            self.set_raster_cache_hints(app, context);
        }
        RenderProxyBoxMixin::paint(self, app, context, offset);
        if let Some(foreground_painter) = self.foreground_painter(app) {
            self.paint_with_painter(app, context.canvas(), offset, &*foreground_painter);
            self.set_raster_cache_hints(app, context);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderCustomPaint {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if self.child(app).is_none() {
            let width = self.preferred_size(app).width();
            return if width.is_finite() { width } else { 0.0 };
        }
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if self.child(app).is_none() {
            let width = self.preferred_size(app).width();
            return if width.is_finite() { width } else { 0.0 };
        }
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if self.child(app).is_none() {
            let height = self.preferred_size(app).height();
            return if height.is_finite() { height } else { 0.0 };
        }
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if self.child(app).is_none() {
            let height = self.preferred_size(app).height();
            return if height.is_finite() { height } else { 0.0 };
        }
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        match &self.get(app).painter {
            Some(painter) => painter.hit_test(app, position).unwrap_or(true),
            None => false,
        }
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if let Some(foreground_painter) = self.foreground_painter(app)
            && foreground_painter.hit_test(app, position).unwrap_or(false)
        {
            return true;
        }
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::Cell;

    use reveal_embedder::{Color, Paint, valo};
    use reveal_foundation::ValueNotifier;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::box_::BoxHitTestResult;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::object::AnyRenderObject;
    use crate::proxy_box::RenderRepaintBoundary;

    const RED: Color = Color::from_argb(255, 255, 0, 0);
    const BLUE: Color = Color::from_argb(255, 0, 0, 255);
    const GREEN: Color = Color::from_argb(255, 0, 255, 0);

    /// Fills the box with `color`, counting its paints; `hit` is what it reports for every
    /// position, and `should_repaint` what it answers about any older painter.
    struct Recorder {
        color: Color,
        paints: Rc<Cell<u32>>,
        hit: Option<bool>,
        repaint: Option<Rc<dyn Listenable>>,
        should_repaint: bool,
    }

    impl Recorder {
        fn new(color: Color) -> Recorder {
            Recorder {
                color,
                paints: Rc::new(Cell::new(0)),
                hit: None,
                repaint: None,
                should_repaint: false,
            }
        }
    }

    impl CustomPainter for Recorder {
        fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
            self.repaint.as_ref()
        }

        fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
            self.paints.set(self.paints.get() + 1);
            canvas.draw_rect(
                Offset::ZERO & size,
                &Paint {
                    color: self.color.into(),
                    ..Paint::default()
                },
            );
        }

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            self.should_repaint
        }

        fn hit_test(&self, _app: &App, _position: Offset) -> Option<bool> {
            self.hit
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn schedule_root_paint(app: &mut App, node: AnyRenderObject) {
        let root = OffsetLayer::new(app, Offset::ZERO);
        root.as_layer().attach(app, node.id());
        node.schedule_initial_paint(app, root.as_container_layer());
    }

    /// The test binding's first frame: attach, lay the root out, schedule and flush paint.
    fn first_frame(app: &mut App, root: AnyRenderBox) -> Handle<PipelineOwner> {
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.layout(app, BoxConstraints::tight(Size::new(100.0, 100.0)), false);
        schedule_root_paint(app, root.as_object());
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
        owner
    }

    fn pump_frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
    }

    /// The color of every fill in a repaint boundary's recording, in paint order.
    fn drawn_colors(app: &App, boundary: AnyRenderObject) -> Vec<valo::Color> {
        let layer = boundary.debug_layer(app).expect("painted");
        layer
            .depth_first_iterate_children(app)
            .into_iter()
            .filter_map(|child| {
                app.handle::<PictureLayer>(child.id())
                    .and_then(|picture| picture.picture(app).map(|p| p.ops().to_vec()))
            })
            .flatten()
            .filter_map(|op| match op {
                valo::Op::DrawRect { paint, .. } => Some(paint.color),
                _ => None,
            })
            .collect()
    }

    fn hit_test(app: &mut App, target: AnyRenderBox, position: Offset) -> (bool, usize) {
        let mut result = HitTestResult::new();
        let is_hit = target.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
        (is_hit, result.path().len())
    }

    #[test]
    fn both_painters_paint_around_the_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let background = Rc::new(Recorder::new(RED));
        let foreground = Rc::new(Recorder::new(BLUE));
        let child = crate::proxy_box::RenderColoredBox::new(&mut app, GREEN, true, None);
        let custom = RenderCustomPaint::new(
            &mut app,
            Some(background.clone()),
            Some(foreground.clone()),
            Size::ZERO,
            false,
            false,
            Some(child.as_box()),
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(custom.as_box()));
        first_frame(&mut app, root.as_box());

        assert_eq!((background.paints.get(), foreground.paints.get()), (1, 1));
        assert_eq!(
            drawn_colors(&app, root.as_object()),
            [RED.into(), GREEN.into(), BLUE.into()],
            "background, child, foreground"
        );
    }

    #[test]
    fn a_painter_repaints_when_its_repaint_listenable_notifies() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(0));
        let painter = Rc::new(Recorder {
            repaint: Some(Rc::new(notifier)),
            ..Recorder::new(RED)
        });
        let custom = RenderCustomPaint::new(
            &mut app,
            Some(painter.clone()),
            None,
            Size::new(10.0, 10.0),
            false,
            false,
            None,
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(custom.as_box()));
        let owner = first_frame(&mut app, root.as_box());
        assert_eq!(painter.paints.get(), 1);
        assert!(!custom.as_object().debug_needs_paint(&app));

        notifier.set_value(&mut app, 1);
        assert!(custom.as_object().debug_needs_paint(&app));
        pump_frame(&mut app, owner);
        assert_eq!(painter.paints.get(), 2);
    }

    /// Dart's `_didUpdatePainter`: the new painter decides, and the listener moves with it.
    #[test]
    fn replacing_the_painter_asks_it_whether_to_repaint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(0));
        let painter = Rc::new(Recorder {
            repaint: Some(Rc::new(notifier)),
            ..Recorder::new(RED)
        });
        let custom = RenderCustomPaint::new(
            &mut app,
            Some(painter.clone()),
            None,
            Size::new(10.0, 10.0),
            false,
            false,
            None,
        );
        let root = RenderRepaintBoundary::new(&mut app, Some(custom.as_box()));
        let owner = first_frame(&mut app, root.as_box());

        let quiet = Rc::new(Recorder::new(BLUE));
        custom.set_painter(&mut app, Some(quiet.clone()));
        assert!(
            !custom.as_object().debug_needs_paint(&app),
            "should_repaint is false"
        );
        notifier.set_value(&mut app, 1);
        assert!(
            !custom.as_object().debug_needs_paint(&app),
            "the old painter's listenable was unsubscribed"
        );

        let eager = Rc::new(Recorder {
            should_repaint: true,
            ..Recorder::new(BLUE)
        });
        custom.set_painter(&mut app, Some(eager.clone()));
        assert!(custom.as_object().debug_needs_paint(&app));
        pump_frame(&mut app, owner);
        assert_eq!((quiet.paints.get(), eager.paints.get()), (0, 1));

        custom.set_painter(&mut app, None);
        assert!(
            custom.as_object().debug_needs_paint(&app),
            "a removal repaints"
        );
    }

    #[test]
    fn preferred_size_is_used_without_a_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let custom = RenderCustomPaint::new(
            &mut app,
            None,
            None,
            Size::new(20.0, 30.0),
            false,
            false,
            None,
        );
        custom.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(custom.size(&app), Size::new(20.0, 30.0));

        custom.layout(
            &mut app,
            BoxConstraints::loose(Size::new(10.0, 10.0)),
            false,
        );
        assert_eq!(
            custom.size(&app),
            Size::new(10.0, 10.0),
            "the preferred size is constrained"
        );
    }

    #[test]
    fn a_child_sizes_the_custom_paint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child = crate::proxy_box::RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(40.0, 40.0)),
            None,
        );
        let custom = RenderCustomPaint::new(
            &mut app,
            None,
            None,
            Size::new(20.0, 30.0),
            false,
            false,
            Some(child.as_box()),
        );
        custom.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(custom.size(&app), Size::new(40.0, 40.0));
    }

    #[test]
    fn the_painters_answer_hit_tests() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let background = Rc::new(Recorder::new(RED));
        let custom = RenderCustomPaint::new(
            &mut app,
            Some(background),
            None,
            Size::new(100.0, 100.0),
            false,
            false,
            None,
        );
        custom.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(
            hit_test(&mut app, custom.as_box(), Offset::new(50.0, 50.0)),
            (true, 1),
            "a background painter is hit by default"
        );

        let missing = Rc::new(Recorder {
            hit: Some(false),
            ..Recorder::new(RED)
        });
        custom.set_painter(&mut app, Some(missing));
        assert_eq!(
            hit_test(&mut app, custom.as_box(), Offset::new(50.0, 50.0)),
            (false, 0)
        );

        custom.set_painter(&mut app, None);
        assert_eq!(
            hit_test(&mut app, custom.as_box(), Offset::new(50.0, 50.0)),
            (false, 0),
            "no painter, no hit"
        );

        let foreground = Rc::new(Recorder {
            hit: Some(true),
            ..Recorder::new(BLUE)
        });
        custom.set_foreground_painter(&mut app, Some(foreground));
        assert_eq!(
            hit_test(&mut app, custom.as_box(), Offset::new(50.0, 50.0)),
            (true, 1),
            "a foreground painter answers for the children"
        );
    }
}
