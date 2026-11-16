//! `CommandBar`'s `LayoutRoot` and `AppBar::MeasureOverride` (`AppBar_Partial.cpp`): the bar takes the height of its closed display mode while its `ContentRoot` keeps its own, and `DisplayModeStates` shows the part of it the state calls for through the `LayoutRoot` clip and the `ContentRoot` translation.
//!
//! WinUI clips to `TemplateSettings.ClipRect`, the content's size, moved by `ClipGeometryTransform`, and moves the content by `ContentTransform`; both start from the vertical deltas of `AppBar::UpdateTemplateSettings`, the closed height less the content height. Here the clip and the translation are worked out at paint and hit testing from the display mode, the motion and the opening direction the overflow layout records in [`AppBarMetrics`], so the direction can be decided after the bar is laid out. A later editor must keep paint and hit testing reading the same values, or the bar takes presses where it is not drawn.

use super::{AppBarClosedDisplayMode, CommandBarDisplayPhase};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use waft_embedder::{Clip, Matrix4, Offset, Rect, Size};
use waft_foundation::App;
use waft_rendering::*;
use waft_widgets::*;

/// Told after a layout that changed what the bar builds.
pub(crate) type LayoutChanged = Rc<dyn Fn(&mut App)>;

/// What the bar's layout found, read by its state and by the overflow's layout.
#[derive(Default)]
pub(crate) struct AppBarMetrics {
    /// `m_contentHeight`: `ContentRoot`'s height.
    pub(crate) content_height: Cell<f64>,
    /// `m_compactHeight`: the height of the closed `Compact` bar.
    pub(crate) compact_height: Cell<f64>,
    /// The bar's height as laid out: the closed display mode's.
    pub(crate) closed_height: Cell<f64>,
    /// The bar's width as laid out.
    pub(crate) width: Cell<f64>,
    /// `GetShouldOpenUp`, as the overflow's layout last decided it.
    pub(crate) opens_up: Cell<bool>,
    /// The `MoreButton`'s rectangle in `ContentRoot`, when shown.
    pub(crate) more_button: Cell<Option<Rect>>,
    /// Whether the `MoreButton` is shown (`EffectiveOverflowButtonVisibility`).
    pub(crate) more_button_shown: Cell<bool>,
    /// The layout root to repaint when the direction changes.
    root: Cell<Option<RenderHandle<RenderAppBarLayoutRoot>>>,
    /// Told after a layout that changed the content or the compact height, which `EffectiveOverflowButtonVisibility` reads.
    pub(crate) heights_changed: RefCell<Option<LayoutChanged>>,
}

impl AppBarMetrics {
    /// Records the opening direction, repainting the bar if it changed.
    pub(crate) fn set_opens_up(&self, app: &mut App, opens_up: bool) {
        if self.opens_up.replace(opens_up) != opens_up
            && let Some(root) = self.root.get()
            && app.contains(root.handle())
        {
            root.as_object().mark_needs_paint(app);
        }
    }

    /// `ClipGeometryTransform.Y` and `ContentTransform.Y` for `mode` and `motion`, given the closed height and the content height: the clip shows the closed band until the bar opens downward, and the content moves up when it opens upward (`DisplayModeStates` and its `VisualTransition`s).
    pub(crate) fn translations(
        &self,
        mode: AppBarClosedDisplayMode,
        motion: DisplayModeMotion,
        minimal_height: f64,
    ) -> (f64, f64) {
        let content = self.content_height.get();
        let closed = match mode {
            AppBarClosedDisplayMode::Compact => self.compact_height.get(),
            AppBarClosedDisplayMode::Minimal => minimal_height,
            AppBarClosedDisplayMode::Hidden => 0.0,
        };
        // `CompactVerticalDelta`, `MinimalVerticalDelta`, `HiddenVerticalDelta`.
        let delta = closed - content;
        let up = self.opens_up.get();
        let p = motion.progress;
        let lerp = |from: f64, to: f64| from + (to - from) * p;
        match motion.phase {
            CommandBarDisplayPhase::Closed => (delta, 0.0),
            CommandBarDisplayPhase::Open if up => (delta, delta),
            CommandBarDisplayPhase::Open => (0.0, 0.0),
            CommandBarDisplayPhase::Opening if up => (delta, lerp(0.0, delta)),
            CommandBarDisplayPhase::Opening => (lerp(delta, 0.0), 0.0),
            CommandBarDisplayPhase::Closing if up => (delta, lerp(delta, 0.0)),
            CommandBarDisplayPhase::Closing => (lerp(0.0, delta), 0.0),
        }
    }
}

impl fmt::Debug for AppBarMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppBarMetrics")
            .field("content_height", &self.content_height.get())
            .field("compact_height", &self.compact_height.get())
            .field("opens_up", &self.opens_up.get())
            .finish_non_exhaustive()
    }
}

/// Where the bar is between closed and open, with the progress of the transition eased by its key spline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayModeMotion {
    pub(crate) phase: CommandBarDisplayPhase,
    pub(crate) progress: f64,
}

/// What `AppBar::MeasureOverride` reads besides the content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AppBarLayout {
    pub(crate) closed_display_mode: AppBarClosedDisplayMode,
    pub(crate) is_open: bool,
    /// `HasRightLabelDynamicPrimaryCommand || HasNonLabeledDynamicPrimaryCommand`: the closed compact bar is as tall as its content.
    pub(crate) grows_with_content: bool,
    /// `GetMinCompactHeight`: `AppBarThemeCompactHeight`.
    pub(crate) min_compact_height: f64,
    /// `AppBarThemeMinimalHeight`.
    pub(crate) minimal_height: f64,
    pub(crate) motion: DisplayModeMotion,
}

/// `LayoutRoot` over `ContentRoot`.
#[derive(Debug)]
pub(crate) struct AppBarLayoutRoot {
    pub(crate) layout: AppBarLayout,
    pub(crate) metrics: Rc<AppBarMetrics>,
    pub(crate) child: WidgetRef,
}

impl RenderObjectWidget for AppBarLayoutRoot {
    type RenderObject = RenderAppBarLayoutRoot;

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderAppBarLayoutRoot {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                layout: self.layout,
                metrics: self.metrics.clone(),
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        object: RenderHandle<RenderAppBarLayoutRoot>,
    ) {
        let old = object.get(app).layout;
        if old == self.layout && Rc::ptr_eq(&object.get(app).metrics, &self.metrics) {
            return;
        }
        object.get_mut(app).layout = self.layout;
        object.get_mut(app).metrics = self.metrics.clone();
        if old.motion != self.layout.motion
            && (AppBarLayout {
                motion: old.motion,
                ..self.layout
            }) == old
        {
            object.as_object().mark_needs_paint(app);
        } else {
            object.as_object().mark_needs_layout(app);
        }
    }
}

impl SingleChildRenderObjectWidget for AppBarLayoutRoot {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// The render object behind [`AppBarLayoutRoot`].
pub(crate) struct RenderAppBarLayoutRoot {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    layout: AppBarLayout,
    metrics: Rc<AppBarMetrics>,
}

impl RenderAppBarLayoutRoot {
    /// The clip rectangle and the content's offset, in the bar's coordinates.
    fn clip_and_shift(self: RenderHandle<Self>, app: &App) -> (Rect, f64) {
        let this = self.get(app);
        let (clip_y, content_y) = this.metrics.translations(
            this.layout.closed_display_mode,
            this.layout.motion,
            this.layout.minimal_height,
        );
        let width = self.size(app).width();
        let content = this.metrics.content_height.get();
        (Rect::from_ltwh(0.0, clip_y, width, content), content_y)
    }
}

impl RenderObjectWithChildMixin for RenderAppBarLayoutRoot {
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

impl RenderObject for RenderAppBarLayoutRoot {
    waft_rendering::render_object_accessors!();

    /// `AppBar::MeasureOverride` and `ArrangeOverride`: the content at its own height, the bar at its closed display mode's.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let layout = self.get(app).layout;
        // The style's `HorizontalAlignment` is `Stretch`.
        let min_width = if constraints.max_width.is_finite() {
            constraints.max_width
        } else {
            constraints.min_width
        };
        let content = match self.child(app) {
            Some(child) => {
                child.layout(
                    app,
                    BoxConstraints::new()
                        .min_width(min_width)
                        .max_width(constraints.max_width)
                        .max_height(constraints.max_height),
                    true,
                );
                child.size(app)
            }
            None => Size::ZERO,
        };
        let metrics = self.get(app).metrics.clone();
        let before = (metrics.content_height.get(), metrics.compact_height.get());
        metrics.content_height.set(content.height());
        let height = match layout.closed_display_mode {
            AppBarClosedDisplayMode::Compact => {
                if !layout.grows_with_content {
                    metrics.compact_height.set(layout.min_compact_height);
                } else if !layout.is_open {
                    metrics
                        .compact_height
                        .set(layout.min_compact_height.max(content.height()));
                }
                metrics.compact_height.get()
            }
            AppBarClosedDisplayMode::Minimal => layout.minimal_height,
            AppBarClosedDisplayMode::Hidden => 0.0,
        };
        let size = constraints.constrain(Size::new(content.width(), height));
        metrics.closed_height.set(size.height());
        metrics.width.set(size.width());
        self.set_size(app, size);
        if before != (metrics.content_height.get(), metrics.compact_height.get()) {
            let changed = metrics.heights_changed.borrow().clone();
            if let Some(changed) = changed {
                changed(app);
            }
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        let (clip, shift) = self.clip_and_shift(app);
        if clip.is_empty() {
            self.as_object().set_layer(app, None);
            return;
        }
        let old = self.as_object().layer_as::<ClipRectLayer>(app);
        let layer = context.push_clip_rect(
            app,
            self.as_object().needs_compositing(app),
            offset,
            clip,
            |app, context, offset| {
                context.paint_child(app, child.as_object(), offset + Offset::new(0.0, shift))
            },
            Clip::HardEdge,
            old,
        );
        self.as_object()
            .set_layer(app, layer.map(|layer| layer.as_container_layer()));
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

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: waft_foundation::Handle<PipelineOwner>,
    ) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        self.get(app).metrics.root.set(Some(self));
    }
}

impl RenderBox for RenderAppBarLayoutRoot {
    waft_rendering::render_box_accessors!();

    /// The bar is hit where it is drawn: its clip, which reaches past its closed height while it is open.
    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let (clip, shift) = self.clip_and_shift(app);
        if !clip.contains(position) {
            return false;
        }
        let Some(child) = self.child(app) else {
            return false;
        };
        let hit = result.add_with_paint_offset(
            Some(Offset::new(0.0, shift)),
            position,
            |result, transformed| child.hit_test(app, result, transformed),
        );
        if hit {
            result.add(BoxHitTestEntry::new(self.as_box(), position).into());
        }
        hit
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let (_, shift) = self.clip_and_shift(app);
        *transform = transform.then(&Matrix4::translation(0.0, shift as f32));
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.smallest()
    }
}
