//! Reports layout size changes after the frame so controls can safely update state.

use inset_embedder::{Matrix4, Offset, Size, TextBaseline};
use inset_foundation::{App, Handle};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, PaintingContext,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// Receives the previous and current child size after layout; the first previous size is zero.
pub type SizeChangedCallback = Rc<dyn Fn(&mut App, Size, Size)>;

/// A transparent layout proxy. Multiple layouts in one frame report only the final size.
/// The first report uses `Size::ZERO` as the previous size.
pub struct SizeObserver {
    /// Identifies this observer across parent rebuilds.
    pub key: Option<KeyRef>,
    /// The child whose laid-out size is observed without changing its constraints.
    pub child: WidgetRef,
    /// Receives coalesced size changes after the frame, while the render object remains attached.
    pub on_changed: SizeChangedCallback,
}

impl SizeObserver {
    /// Creates a layout observer with a child and a size-change callback.
    pub fn new<K>(child: impl IntoWidget<K>, on_changed: SizeChangedCallback) -> Self {
        Self {
            key: None,
            child: child.into_widget(),
            on_changed,
        }
    }

    /// Sets the widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for SizeObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SizeObserver")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for SizeObserver {
    type RenderObject = RenderSizeObserver;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderSizeObserver {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                on_changed: self.on_changed.clone(),
                observed: Size::ZERO,
                reported: Size::ZERO,
                pending: false,
                epoch: 0,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSizeObserver>,
    ) {
        render_object.get_mut(app).on_changed = self.on_changed.clone();
    }
}

impl SingleChildRenderObjectWidget for SizeObserver {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// A layout proxy that schedules size notifications without mutating the tree during layout.
pub struct RenderSizeObserver {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    on_changed: SizeChangedCallback,
    /// The last size produced by layout, which may not yet have been reported.
    observed: Size,
    /// The size delivered by the most recent callback.
    reported: Size,
    /// Whether this attachment already has a queued post-frame callback.
    pending: bool,
    /// Invalidates queued callbacks when this render object detaches.
    epoch: u64,
}

impl RenderSizeObserver {
    /// Queues one notification per frame and discards callbacks from an earlier attachment.
    fn schedule_report(self: RenderHandle<Self>, app: &mut App) {
        if self.get(app).pending || self.get(app).observed == self.get(app).reported {
            return;
        }
        self.get_mut(app).pending = true;
        let epoch = self.get(app).epoch;
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self.handle()) || app.is_disposed(self.handle()) {
                    return;
                }
                if self.get(app).epoch != epoch || !self.as_object().attached(app) {
                    return;
                }
                self.get_mut(app).pending = false;
                let old = self.get(app).reported;
                let new = self.get(app).observed;
                if old == new {
                    return;
                }
                self.get_mut(app).reported = new;
                let callback = self.get(app).on_changed.clone();
                callback(app, old, new);
            }),
            // Queued during layout, which every view of a frame shares: this frame's end runs it.
            None,
        );
    }
}

impl RenderObjectWithChildMixin for RenderSizeObserver {
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

impl RenderProxyBoxMixin for RenderSizeObserver {}

impl RenderObject for RenderSizeObserver {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        self.get_mut(app).observed = self.as_box().size(app);
        self.schedule_report(app);
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        self.schedule_report(app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).epoch += 1;
        self.get_mut(app).pending = false;
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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

impl RenderBox for RenderSizeObserver {
    inset_rendering::render_box_accessors!();

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

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
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
}
