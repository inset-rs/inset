//! `CommandBar`'s overlay while it is open: the light-dismiss layer `AppBar::SetupOverlayState` puts under the bar, the bar itself where its layout transition element draws it, and `OverflowPopup` with the `CommandBarOverflowPresenter`, placed by `CommandBar::UpdateTemplateSettings`, `GetShouldOpenUp` and `CalculateOverflowContentHorizontalOffset` and revealed by the `DisplayModeStates` transitions.
//!
//! The overflow opens down unless the bar or the menu would pass the overlay's bottom, and is aligned with the bar's end edge unless that runs it off the window. A later editor must keep the direction decided here, after both are measured, and read at paint by the bar and the menu alike.

use super::layout_root::{AppBarMetrics, DisplayModeMotion};
use super::{AppBarClosedDisplayMode, CommandBarDisplayPhase};
use std::any::Any;
use std::fmt;
use std::rc::Rc;
use waft_embedder::{Clip, Matrix4, Offset, Rect, Size};
use waft_foundation::{App, ValueKey};
use waft_rendering::*;
use waft_widgets::*;

/// The overlay's parts.
pub(crate) fn part(name: &'static str) -> ChildLayoutId {
    Rc::new(ValueKey::new(name))
}

/// What the overlay's layout places by.
#[derive(Debug)]
pub(crate) struct CommandBarOverlayLayout {
    /// Where the bar sits in the overlay.
    pub(crate) target: Rect,
    pub(crate) metrics: Rc<AppBarMetrics>,
    pub(crate) has_bar: bool,
    pub(crate) has_overflow: bool,
    /// `HasPrimaryCommands`: the bar opens along with the menu, so it needs room too.
    pub(crate) has_primary_commands: bool,
    /// `m_overflowContentMinWidth`, or `m_overflowContentTouchMinWidth` for a bar opened by touch.
    pub(crate) overflow_min_width: f64,
    /// `m_overflowContentMaxWidth`.
    pub(crate) overflow_max_width: f64,
    pub(crate) rtl: bool,
    pub(crate) closed_display_mode: AppBarClosedDisplayMode,
    pub(crate) motion: DisplayModeMotion,
    /// `AppBarThemeCompactHeight` and `AppBarThemeMinimalHeight`, where an opening-down menu starts.
    pub(crate) compact_height_resource: f64,
    pub(crate) minimal_height: f64,
}

impl CommandBarOverlayLayout {
    /// `HasSpaceForAppBarToOpenDown`, rounding as WinUI does, a pixel of slack allowed.
    fn bar_has_space_to_open_down(&self, size: Size) -> bool {
        let bottom = self.target.top + self.metrics.content_height.get();
        bottom.round() <= (size.height() + 1.0).round()
    }

    /// `CalculateOverflowContentHorizontalOffset`: the menu's left edge.
    fn overflow_left(&self, size: Size, width: f64) -> f64 {
        // From the bar's start edge, toward its end.
        let start = if self.rtl {
            size.width() - self.target.right
        } else {
            self.target.left
        };
        let mut offset = self.target.width() - width;
        if offset < 0.0 && start + offset < 0.0 {
            offset = if start + width <= size.width() {
                0.0
            } else {
                -start
            };
        }
        if self.rtl {
            size.width() - (start + offset) - width
        } else {
            start + offset
        }
    }

    /// The menu's top: below the bar opening down, its `OverflowContentRootTransform` moving from the closed height to the content's; above the expanded bar opening up.
    fn overflow_top(&self, height: f64) -> f64 {
        let content = self.metrics.content_height.get();
        if self.metrics.opens_up.get() {
            let closed = match self.closed_display_mode {
                AppBarClosedDisplayMode::Compact => self.metrics.compact_height.get(),
                AppBarClosedDisplayMode::Minimal => self.minimal_height,
                AppBarClosedDisplayMode::Hidden => 0.0,
            };
            return self.target.top + (closed - content) - height;
        }
        let start = match self.closed_display_mode {
            AppBarClosedDisplayMode::Compact => self.compact_height_resource,
            AppBarClosedDisplayMode::Minimal => self.minimal_height,
            AppBarClosedDisplayMode::Hidden => 0.0,
        };
        let p = self.motion.progress;
        let y = match self.motion.phase {
            CommandBarDisplayPhase::Opening => start + (content - start) * p,
            CommandBarDisplayPhase::Closing => content + (start - content) * p,
            CommandBarDisplayPhase::Open | CommandBarDisplayPhase::Closed => content,
        };
        self.target.top + y
    }
}

impl MultiChildLayoutDelegate for CommandBarOverlayLayout {
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        children.layout_child(app, &part("barrier"), BoxConstraints::tight(size));
        children.position_child(app, &part("barrier"), Offset::ZERO);
        if self.has_bar {
            children.layout_child(
                app,
                &part("bar"),
                BoxConstraints::new()
                    .min_width(self.target.width())
                    .max_width(self.target.width())
                    .max_height(size.height()),
            );
            children.position_child(app, &part("bar"), self.target.top_left());
        }
        let bar_space = !self.has_primary_commands || self.bar_has_space_to_open_down(size);
        if !self.has_overflow {
            self.metrics.set_opens_up(app, !bar_space);
            return;
        }
        // `GetShouldOverflowOpenInFullWidth`: a window no wider than the menu's maximum.
        let (min_width, max_width) = if size.width() <= self.overflow_max_width {
            (size.width(), size.width())
        } else {
            (
                self.overflow_min_width.min(size.width()),
                self.overflow_max_width,
            )
        };
        let menu = children.layout_child(
            app,
            &part("overflow"),
            BoxConstraints::new()
                .min_width(min_width)
                .max_width(max_width)
                // `OverflowContentMaxHeight`: half the available height.
                .max_height(size.height() * 0.5),
        );
        // `HasSpaceForOverflowPopupToOpenDown`: the menu's bottom below the expanded bar.
        let overflow_space =
            self.target.top + self.metrics.content_height.get() + menu.height() <= size.height();
        self.metrics
            .set_opens_up(app, !bar_space || !overflow_space);
        let origin = Offset::new(
            self.overflow_left(size, menu.width()),
            self.overflow_top(menu.height()),
        );
        children.position_child(app, &part("overflow"), origin);
    }

    fn should_relayout(&self, _old: &dyn MultiChildLayoutDelegate) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The menu's reveal: `OverflowContentTransform` slides it in from the bar's edge inside the `SecondaryItemsControlShadowWrapper` clip, which `OverflowContentRootClipTransform` keeps at the expanding bar's edge when it opens up.
pub(crate) struct OverflowReveal {
    pub(crate) motion: DisplayModeMotion,
    pub(crate) closed_display_mode: AppBarClosedDisplayMode,
    pub(crate) minimal_height: f64,
    pub(crate) metrics: Rc<AppBarMetrics>,
    pub(crate) child: WidgetRef,
}

impl fmt::Debug for OverflowReveal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverflowReveal")
            .field("motion", &self.motion)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for OverflowReveal {
    type RenderObject = RenderOverflowReveal;

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderOverflowReveal {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                motion: self.motion,
                closed_display_mode: self.closed_display_mode,
                minimal_height: self.minimal_height,
                metrics: self.metrics.clone(),
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        object: RenderHandle<RenderOverflowReveal>,
    ) {
        let reveal = object.get_mut(app);
        reveal.motion = self.motion;
        reveal.closed_display_mode = self.closed_display_mode;
        reveal.minimal_height = self.minimal_height;
        reveal.metrics = self.metrics.clone();
        object.as_object().mark_needs_paint(app);
    }
}

impl SingleChildRenderObjectWidget for OverflowReveal {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// The render object behind [`OverflowReveal`].
pub(crate) struct RenderOverflowReveal {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    motion: DisplayModeMotion,
    closed_display_mode: AppBarClosedDisplayMode,
    minimal_height: f64,
    metrics: Rc<AppBarMetrics>,
}

impl RenderOverflowReveal {
    /// The clip's vertical offset and the menu's, from the closed state toward the open one; none once open.
    fn clip_and_slide(self: RenderHandle<Self>, app: &App) -> Option<(f64, f64)> {
        let this = self.get(app);
        let height = self.size(app).height();
        let metrics = &this.metrics;
        let up = metrics.opens_up.get();
        let content = metrics.content_height.get();
        let closed = match this.closed_display_mode {
            AppBarClosedDisplayMode::Compact => metrics.compact_height.get(),
            AppBarClosedDisplayMode::Minimal => this.minimal_height,
            AppBarClosedDisplayMode::Hidden => 0.0,
        };
        // How far the bar grows: `NegativeCompactVerticalDelta` and its siblings.
        let growth = content - closed;
        // `OverflowContentHeight` slides the menu out from behind the bar's edge.
        let (clip_from, slide_from) = if up { (growth, height) } else { (0.0, -height) };
        let away = match this.motion.phase {
            CommandBarDisplayPhase::Opening => 1.0 - this.motion.progress,
            CommandBarDisplayPhase::Closing => this.motion.progress,
            CommandBarDisplayPhase::Open => 0.0,
            CommandBarDisplayPhase::Closed => 1.0,
        };
        (away > 0.0).then_some((clip_from * away, slide_from * away))
    }
}

impl RenderObjectWithChildMixin for RenderOverflowReveal {
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

impl RenderObject for RenderOverflowReveal {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = match self.child(app) {
            Some(child) => {
                child.layout(app, constraints, true);
                child.size(app)
            }
            None => constraints.smallest(),
        };
        self.set_size(app, size);
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
        let Some((clip_y, slide)) = self.clip_and_slide(app) else {
            context.paint_child(app, child.as_object(), offset);
            self.as_object().set_layer(app, None);
            return;
        };
        let up = self.get(app).metrics.opens_up.get();
        let size = self.size(app);
        // The shadow spreads beyond the menu, so only the direction the menu moves in is clipped.
        let clip = if up {
            Rect::from_ltrb(-1.0e4, -1.0e4, size.width() + 1.0e4, size.height() + clip_y)
        } else {
            Rect::from_ltrb(-1.0e4, clip_y, size.width() + 1.0e4, size.height() + 1.0e4)
        };
        let old = self.as_object().layer_as::<ClipRectLayer>(app);
        let layer = context.push_clip_rect(
            app,
            self.as_object().needs_compositing(app),
            offset,
            clip,
            |app, context, offset| {
                context.paint_child(app, child.as_object(), offset + Offset::new(0.0, slide))
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
}

impl RenderBox for RenderOverflowReveal {
    waft_rendering::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let slide = self.clip_and_slide(app).map_or(0.0, |(_, slide)| slide);
        result.add_with_paint_offset(
            Some(Offset::new(0.0, slide)),
            position,
            |result, transformed| child.hit_test(app, result, transformed),
        )
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let slide = self.clip_and_slide(app).map_or(0.0, |(_, slide)| slide);
        *transform = transform.then(&Matrix4::translation(0.0, slide as f32));
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        match self.child(app) {
            Some(child) => child.get_dry_layout(app, constraints),
            None => constraints.smallest(),
        }
    }
}
