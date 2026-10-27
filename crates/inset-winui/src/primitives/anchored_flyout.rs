//! In-window interactive popup used by NavigationView's attached-flyout templates.

use crate::{Brush, ControlBorder, ThemeResources};
use inset_embedder::{Color, Offset, Rect, Size, TextDirection};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_painting::transform_rect;
use inset_rendering::{
    BoxConstraints, ChildLayoutId, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{any::Any, rc::Rc};

/// The attached placements NavigationView's templates request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AnchoredFlyoutPlacement {
    /// Overflow menu below the button, with trailing edges aligned.
    #[default]
    BottomEdgeAlignedRight,
    /// Top-primary hierarchical children below the item, with leading edges aligned.
    BottomEdgeAlignedLeft,
    /// Hierarchical children beside their item, with top edges aligned.
    RightEdgeAlignedTop,
}

/// A controlled, interactive root-overlay attachment; its owner handles dismissal requests.
#[derive(Clone, Debug)]
pub(crate) struct AnchoredFlyout {
    /// Whether the owner wants the overlay mounted.
    open: bool,
    /// The placement target, retained while the popup is closed.
    anchor: WidgetRef,
    /// Intrinsically sized content inside the native scrolling presenter.
    content: WidgetRef,
    /// Escape and pointer light-dismiss request; does not fire on an explicit property close.
    dismissed: Listener,
    /// Source attached placement.
    placement: AnchoredFlyoutPlacement,
    /// ContentPresenter padding, overridden by NavigationView's styles; the theme's
    /// `FlyoutContentPadding` when `None`.
    padding: Option<[f64; 4]>,
    /// Presenter margin adjustment, including children-flyout top margin -4.
    offset: Offset,
    /// Style maximum width; the theme's `FlyoutThemeMaxWidth` when `None`.
    max_width: Option<f64>,
}

impl AnchoredFlyout {
    /// Attaches a controlled popup to an anchor in a tree with a native Overlay ancestor.
    pub(crate) fn new<A, C>(
        open: bool,
        anchor: impl IntoWidget<A>,
        content: impl IntoWidget<C>,
        dismissed: Listener,
    ) -> Self {
        Self {
            open,
            anchor: anchor.into_widget(),
            content: content.into_widget(),
            dismissed,
            placement: AnchoredFlyoutPlacement::default(),
            padding: None,
            offset: Offset::ZERO,
            max_width: None,
        }
    }

    /// Selects one of NavigationView's source placements.
    pub(crate) fn placement(mut self, placement: AnchoredFlyoutPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the presenter's content padding in left, top, right, bottom order.
    pub(crate) fn padding(mut self, padding: [f64; 4]) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Applies the source presenter margin adjustment after placement.
    pub(crate) fn offset(mut self, offset: Offset) -> Self {
        self.offset = offset;
        self
    }

    /// Overrides the source FlyoutThemeMaxWidth.
    pub(crate) fn max_width(mut self, width: f64) -> Self {
        self.max_width = Some(width.max(0.0));
        self
    }
}

/// Owns native overlay and focus handles for the lifetime of the attachment.
pub(crate) struct AnchoredFlyoutState {
    /// Native state identity.
    state: StateData<AnchoredFlyout>,
    /// Portal to the root overlay.
    portal: Option<Handle<OverlayPortalController>>,
    /// Closed-loop popup focus scope.
    focus: Option<Handle<FocusScopeNode>>,
    /// Focus target saved before opening.
    previous: Option<AnyFocusNode>,
    /// Invalidates delayed focus callbacks after rapid open/close changes.
    epoch: u64,
}

impl StatefulWidget for AnchoredFlyout {
    type State = AnchoredFlyoutState;
    fn create_state(&self) -> Self::State {
        AnchoredFlyoutState {
            state: StateData::new(),
            portal: None,
            focus: None,
            previous: None,
            epoch: 0,
        }
    }
}

impl AnchoredFlyoutState {
    /// Restores a surviving opener, without overriding a newer popup's focus.
    fn restore(self: Handle<Self>, app: &mut App) {
        let previous = app.get_mut(self).previous.take();
        let owns_focus = app.get(self).focus.unwrap().as_node().has_focus(app);
        if owns_focus
            && let Some(previous) = previous.filter(|p| {
                app.contains(p.id())
                    && !app.is_disposed(p.id())
                    && p.context(app).is_some()
                    && p.can_request_focus(app)
            })
        {
            previous.request_focus(app, None);
        }
    }

    /// Opens or closes outside build, then focuses the mounted presenter's first child.
    fn update(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).epoch += 1;
        let epoch = app.get(self).epoch;
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) || app.get(self).epoch != epoch {
                    return;
                }
                let portal = app.get(self).portal.unwrap();
                if self.widget(app).open {
                    app.get_mut(self).previous = primary_focus(app);
                    portal.show(app);
                    SchedulerBinding::add_post_frame_callback(
                        app,
                        FrameCallback::new(move |app, _| {
                            if !app.contains(self)
                                || !self.mounted(app)
                                || app.get(self).epoch != epoch
                                || !self.widget(app).open
                            {
                                return;
                            }
                            let scope = app.get(self).focus.unwrap().as_node();
                            let first = scope
                                .traversal_descendants(app)
                                .into_iter()
                                .find(|n| n.can_request_focus(app));
                            first.unwrap_or(scope).request_focus(app, None);
                        }),
                        View::maybe_id_of(app, self.context(app)),
                    );
                } else {
                    self.restore(app);
                    portal.hide(app);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }
}

impl State for AnchoredFlyoutState {
    type Widget = AnchoredFlyout;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let focus = FocusScopeNode::new(app);
        focus.set_traversal_edge_behavior(app, TraversalEdgeBehavior::ClosedLoop);
        let portal = OverlayPortalController::new(app, Some("Navigation attached flyout".into()));
        app.get_mut(self).focus = Some(focus);
        app.get_mut(self).portal = Some(portal);
        self.update(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &AnchoredFlyout) {
        if old.open != self.widget(app).open {
            self.update(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.restore(app);
        let focus = app.get(self).focus.unwrap();
        focus.as_node().dispose(app);
        app.destroy(focus);
        app.destroy(app.get(self).portal.unwrap());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let rtl = Directionality::maybe_of(app, context) == Some(TextDirection::Rtl);
        OverlayPortal::overlay_child_layout_builder(
            app.get(self).portal.unwrap(),
            move |app, context, info| {
                let widget = self.widget(app).clone();
                let r = ThemeResources::of(app, context);
                // NavigationView presenter template: ScrollViewer > ContentPresenter chrome/content.
                let presenter = ControlBorder::new(
                    Brush::Acrylic(r.flyout_presenter_background),
                    Brush::Solid(r.flyout_border_theme_brush),
                )
                .border_thickness_ltrb(r.flyout_border_theme_thickness)
                .corner_radius_corners(r.overlay_corner_radius)
                .padding(widget.padding.unwrap_or(r.flyout_content_padding))
                .child(widget.content);
                let popup = FocusScope::new(SingleChildScrollView::new().child(presenter))
                    .node(app.get(self).focus.unwrap())
                    .on_key_event(Rc::new(move |app, _, event| {
                        if matches!(event, KeyEvent::Down(_))
                            && matches!(
                                event.logical_key(),
                                LogicalKeyboardKey::ESCAPE | LogicalKeyboardKey::GAME_BUTTON_B
                            )
                        {
                            self.widget(app).dismissed.clone().call(app);
                            KeyEventResult::Handled
                        } else {
                            KeyEventResult::Ignored
                        }
                    }));
                let barrier = inset_widgets::Listener::new()
                    .behavior(inset_rendering::HitTestBehavior::Opaque)
                    .on_pointer_down(Rc::new(move |app, _| {
                        self.widget(app).dismissed.clone().call(app)
                    }))
                    .child(ColoredBox::new(Color::new(0)));
                let target = transform_rect(
                    &info.child_paint_transform(),
                    Offset::ZERO & info.child_size(),
                );
                CustomMultiChildLayout::new(Rc::new(FlyoutLayout {
                    target,
                    placement: widget.placement,
                    offset: widget.offset,
                    max_width: widget.max_width.unwrap_or(r.flyout_theme_max_width),
                    minimum: Size::new(r.flyout_theme_min_width, r.flyout_theme_min_height),
                    max_height: r.flyout_theme_max_height,
                    rtl,
                }))
                .children([
                    LayoutId::new(id("barrier"), barrier).into_widget(),
                    LayoutId::new(
                        id("presenter"),
                        inset_widgets::Listener::new()
                            .behavior(inset_rendering::HitTestBehavior::Opaque)
                            .child(popup),
                    )
                    .into_widget(),
                ])
                .into_widget()
            },
        )
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(self.widget(app).anchor.clone())
        .into_widget()
    }
}

/// Stable template part identity.
fn id(value: &'static str) -> ChildLayoutId {
    Rc::new(ValueKey::new(value))
}

/// Source major placement axis after RTL adjustment.
#[derive(Clone, Copy, Debug)]
enum Side {
    Bottom,
    Top,
    Left,
    Right,
}

/// Places one measured presenter and the full-root light-dismiss barrier.
#[derive(Debug)]
struct FlyoutLayout {
    /// Anchor bounds in root-overlay coordinates.
    target: Rect,
    /// Requested source placement.
    placement: AnchoredFlyoutPlacement,
    /// Template margin adjustment.
    offset: Offset,
    /// Presenter style constraint.
    max_width: f64,
    /// The style's `FlyoutThemeMinWidth` and `FlyoutThemeMinHeight`.
    minimum: Size,
    /// The style's `FlyoutThemeMaxHeight`.
    max_height: f64,
    /// Effective flow direction.
    rtl: bool,
}

impl MultiChildLayoutDelegate for FlyoutLayout {
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        children.layout_child(app, &id("barrier"), BoxConstraints::tight(size));
        children.position_child(app, &id("barrier"), Offset::ZERO);
        let t = self.target;
        let horizontal = self.placement == AnchoredFlyoutPlacement::RightEdgeAlignedTop;
        let mut side = if horizontal {
            if self.rtl { Side::Left } else { Side::Right }
        } else {
            Side::Bottom
        };
        let available = |side| match side {
            Side::Bottom => size.height() - t.bottom - 8.0,
            Side::Top => t.top - 8.0,
            Side::Right => size.width() - t.right - 8.0,
            Side::Left => t.left - 8.0,
        };
        let opposite = match side {
            Side::Bottom => Side::Top,
            Side::Top => Side::Bottom,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        let minimum = if horizontal {
            self.minimum.width()
        } else {
            self.minimum.height()
        };
        if available(side) < minimum && available(opposite) > available(side) {
            side = opposite;
        }
        let width = self
            .max_width
            .min((size.width() - 8.0).max(0.0))
            .min(if horizontal {
                available(side).max(minimum)
            } else {
                f64::INFINITY
            });
        let height = self
            .max_height
            .min((size.height() - 8.0).max(0.0))
            .min(if horizontal {
                f64::INFINITY
            } else {
                available(side).max(minimum)
            });
        let measured = children.layout_child(
            app,
            &id("presenter"),
            BoxConstraints::new()
                .min_width(self.minimum.width().min(width))
                .max_width(width)
                .min_height(self.minimum.height().min(height))
                .max_height(height),
        );
        let x = match side {
            Side::Right => t.right + 4.0,
            Side::Left => t.left - measured.width() - 4.0,
            _ => {
                if self.rtl ^ (self.placement == AnchoredFlyoutPlacement::BottomEdgeAlignedLeft) {
                    t.left
                } else {
                    t.right - measured.width()
                }
            }
        };
        let y = match side {
            Side::Bottom => t.bottom + 4.0,
            Side::Top => t.top - measured.height() - 4.0,
            _ => t.top,
        };
        children.position_child(
            app,
            &id("presenter"),
            Offset::new(
                (x + self.offset.dx()).clamp(0.0, (size.width() - measured.width()).max(0.0)),
                (y + self.offset.dy()).clamp(0.0, (size.height() - measured.height()).max(0.0)),
            ),
        );
    }
    fn should_relayout(&self, _: &dyn MultiChildLayoutDelegate) -> bool {
        true
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
