//! WinUI SplitView: the two-column template and the pane lifecycle from SplitView_Partial.cpp.

use super::split_view_visuals::{self, DisplayModeState};
use crate::{
    Brush, ColorTransition, ColumnDefinition, Grid, GridCell, GridLength, SizeObserver,
    ThemeResources, after_next_frame,
};
use inset_embedder::{Color, Offset, Path, PathBuilder, Rect, Size};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Alignment, EdgeInsets, transform_point};
use inset_rendering::{BoxConstraints, CustomClipper, StackFit};
use inset_scheduler::{FrameCallback, SchedulerBinding, Ticker};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{any::Any, fmt, rc::Rc, sync::Arc, time::Duration};

/// Specifies how the pane and content areas of a [`SplitView`] are displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitViewDisplayMode {
    /// The open pane covers content; the closed pane is hidden.
    #[default]
    Overlay,
    /// The open pane reserves its full width beside the content; the closed pane is hidden.
    Inline,
    /// The closed pane reserves its compact width; the open pane overlays the content.
    CompactOverlay,
    /// The pane reserves its compact width when closed and its full width when open.
    CompactInline,
}

impl SplitViewDisplayMode {
    /// Whether this mode allows interactions outside the pane to close it.
    fn light_dismissible(self) -> bool {
        matches!(self, Self::Overlay | Self::CompactOverlay)
    }
}

/// Specifies which side of a [`SplitView`] contains its pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitViewPanePlacement {
    /// Displays the pane on the left side.
    #[default]
    Left,
    /// Displays the pane on the right side.
    Right,
}

/// Specifies whether a light-dismiss layer visually dims the content.
///
/// This controls the layer's appearance, not whether tapping it dismisses the pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightDismissOverlayMode {
    /// Uses the platform default: transparent on desktop.
    #[default]
    Auto,
    /// Dims the content using the themed light-dismiss overlay brush.
    On,
    /// Keeps the dismiss layer transparent.
    Off,
}

/// `OverlayVisibilityStates`; desktop Auto leaves the dismiss layer transparent.
enum OverlayVisibilityState {
    OverlayNotVisible,
    OverlayVisible,
}

impl LightDismissOverlayMode {
    /// Selects the template state controlling the dismiss layer's fill.
    fn visual_state(self) -> OverlayVisibilityState {
        match self {
            Self::On => OverlayVisibilityState::OverlayVisible,
            Self::Auto | Self::Off => OverlayVisibilityState::OverlayNotVisible,
        }
    }
}

/// Read-only values supplied to the SplitView template. Auto resolves after the pane's layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitViewTemplateSettings {
    /// The resolved width of the fully open pane.
    pub open_pane_length: f64,
    /// The negated open width used by the left-pane translation.
    pub negative_open_pane_length: f64,
    /// The additional width revealed when expanding from compact to open.
    pub open_pane_length_minus_compact_length: f64,
    /// The negated expansion width used by left-pane clip and content animations.
    pub negative_open_pane_length_minus_compact_length: f64,
    /// The resolved open width expressed as a pixel Grid length.
    pub open_pane_grid_length: GridLength,
    /// The compact width expressed as a pixel Grid length.
    pub compact_pane_grid_length: GridLength,
}

impl SplitViewTemplateSettings {
    /// Derives animation and Grid values from the resolved open and compact widths.
    fn new(open: f64, compact: f64) -> Self {
        Self {
            open_pane_length: open,
            negative_open_pane_length: -open,
            open_pane_length_minus_compact_length: open - compact,
            negative_open_pane_length_minus_compact_length: -(open - compact),
            open_pane_grid_length: GridLength::pixel(open),
            compact_pane_grid_length: GridLength::pixel(compact),
        }
    }
}

/// `Cancel` is honored for light dismissal, not an explicit `is_pane_open = false` update.
#[derive(Clone, Copy, Debug, Default)]
pub struct SplitViewPaneClosingEventArgs {
    /// Set to `true` to cancel light dismissal; ignored for explicit owner-driven closes.
    pub cancel: bool,
}

/// Receives a requested `IsPaneOpen` value; the owner applies or rejects the change.
pub type SplitViewPaneChangedHandler = Rc<dyn Fn(&mut App, bool)>;

type PaneEvent = Listener;

/// Receives `PaneClosing` arguments before a light-dismiss close request is applied.
pub type SplitViewPaneClosingHandler = Rc<dyn Fn(&mut App, &mut SplitViewPaneClosingEventArgs)>;

/// A container with a content area and a pane that can expand, collapse or overlay the content.
///
/// The owner supplies [`is_pane_open`](Self::is_pane_open) and applies requested changes.
/// Open overlay modes require a native [`Overlay`] ancestor for dismissal outside the control.
/// Pane and content widgets retain their state when the pane opens or closes.
#[derive(Clone)]
pub struct SplitView {
    /// Identifies this widget when the parent rebuilds.
    pub key: Option<KeyRef>,
    /// Whether the pane is expanded to its open width.
    pub is_pane_open: bool,
    /// Requests a new open value from the owner after an uncanceled light dismissal.
    pub is_pane_open_changed: SplitViewPaneChangedHandler,
    /// The widget displayed in the pane area; retained while the pane is closed.
    pub pane: Option<WidgetRef>,
    /// The widget displayed in the main content area.
    pub content: Option<WidgetRef>,
    /// How the pane shares space with the content; defaults to Overlay.
    pub display_mode: SplitViewDisplayMode,
    /// The side containing the pane; defaults to Left.
    pub pane_placement: SplitViewPanePlacement,
    /// The open pane width; `f64::NAN` sizes to the pane content, and `None` is the theme's
    /// `SplitViewOpenPaneThemeLength`.
    pub open_pane_length: Option<f64>,
    /// The width reserved by a closed compact pane; the theme's `SplitViewCompactPaneThemeLength`
    /// when `None`.
    pub compact_pane_length: Option<f64>,
    /// Whether the dismiss layer dims the content; defaults to desktop Auto (transparent).
    pub light_dismiss_overlay_mode: LightDismissOverlayMode,
    /// The pane fill; `None` resolves the default brush from the current theme.
    pub pane_background: Option<Brush>,
    /// The background behind both pane and content; `None` leaves it unpainted.
    pub background: Option<Brush>,
    /// The pane border brush; the default theme brush is transparent.
    pub border_brush: Option<Brush>,
    /// Pane border widths in left, top, right, bottom order; the theme's
    /// `SplitViewLeftBorderThemeThickness` when `None`.
    pub border_thickness: Option<[f64; 4]>,
    /// Pane radii in top-left, top-right, bottom-right, bottom-left order; the theme's
    /// `SplitViewPaneRootCornerRadius` when `None`.
    pub corner_radius: Option<[f64; 4]>,
    /// Called after the first layout of an opening transition, before focus moves into the pane.
    pub pane_opening: Option<PaneEvent>,
    /// Called when the current opening transition completes.
    pub pane_opened: Option<PaneEvent>,
    /// Called before light dismissal, or after the first layout of an explicit closing update.
    pub pane_closing: Option<SplitViewPaneClosingHandler>,
    /// Called when the current closing transition completes.
    pub pane_closed: Option<PaneEvent>,
}

impl SplitView {
    /// The owner applies requested `IsPaneOpen` changes, as with other reveal controls.
    pub fn new(is_pane_open: bool, changed: impl Fn(&mut App, bool) + 'static) -> Self {
        Self {
            key: None,
            is_pane_open,
            is_pane_open_changed: Rc::new(changed),
            pane: None,
            content: None,
            display_mode: SplitViewDisplayMode::Overlay,
            pane_placement: SplitViewPanePlacement::Left,
            open_pane_length: None,
            compact_pane_length: None,
            light_dismiss_overlay_mode: LightDismissOverlayMode::Auto,
            pane_background: None,
            background: None,
            border_brush: None,
            border_thickness: None,
            corner_radius: None,
            pane_opening: None,
            pane_opened: None,
            pane_closing: None,
            pane_closed: None,
        }
    }

    /// Sets the widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Sets the widget displayed in the pane area.
    pub fn pane<K>(mut self, pane: impl IntoWidget<K>) -> Self {
        self.pane = Some(pane.into_widget());
        self
    }

    /// Sets the widget displayed in the main content area.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> Self {
        self.content = Some(content.into_widget());
        self
    }

    /// Sets how the pane shares space with the content.
    pub fn display_mode(mut self, mode: SplitViewDisplayMode) -> Self {
        self.display_mode = mode;
        self
    }

    /// Sets which side contains the pane.
    pub fn pane_placement(mut self, placement: SplitViewPanePlacement) -> Self {
        self.pane_placement = placement;
        self
    }

    /// `f64::NAN` is WinUI's Auto length, measured from the pane content.
    ///
    /// Panics for negative lengths or infinity.
    pub fn open_pane_length(mut self, length: f64) -> Self {
        assert!(length.is_nan() || (length.is_finite() && length >= 0.0));
        self.open_pane_length = Some(length);
        self
    }

    /// Sets the width reserved when a compact pane is closed.
    ///
    /// Panics if the length is negative or non-finite.
    pub fn compact_pane_length(mut self, length: f64) -> Self {
        assert!(length.is_finite() && length >= 0.0);
        self.compact_pane_length = Some(length);
        self
    }

    /// Sets the appearance of the light-dismiss layer.
    pub fn light_dismiss_overlay_mode(mut self, mode: LightDismissOverlayMode) -> Self {
        self.light_dismiss_overlay_mode = mode;
        self
    }

    /// Sets the background behind the pane and content.
    pub fn background(mut self, brush: Brush) -> Self {
        self.background = Some(brush);
        self
    }

    /// Sets pane radii in top-left, top-right, bottom-right, bottom-left order.
    pub fn corner_radius_corners(mut self, radius: [f64; 4]) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Sets the pane fill brush.
    pub fn pane_background(mut self, brush: Brush) -> Self {
        self.pane_background = Some(brush);
        self
    }

    /// Sets the pane border brush.
    pub fn border_brush(mut self, brush: Brush) -> Self {
        self.border_brush = Some(brush);
        self
    }

    /// Sets the same pane border width on all four sides.
    pub fn border_thickness(mut self, thickness: f64) -> Self {
        self.border_thickness = Some([thickness; 4]);
        self
    }

    /// Sets pane border widths in left, top, right, bottom order.
    pub fn border_thickness_ltrb(mut self, thickness: [f64; 4]) -> Self {
        self.border_thickness = Some(thickness);
        self
    }

    /// Sets the same radius on all four pane corners.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = Some([radius; 4]);
        self
    }

    /// Registers a handler for the start of an opening transition.
    pub fn pane_opening(mut self, callback: Listener) -> Self {
        self.pane_opening = Some(callback);
        self
    }

    /// Registers a handler for completion of an opening transition.
    pub fn pane_opened(mut self, callback: Listener) -> Self {
        self.pane_opened = Some(callback);
        self
    }

    /// Registers a handler that can cancel light dismissal through its event arguments.
    pub fn pane_closing(
        mut self,
        callback: impl Fn(&mut App, &mut SplitViewPaneClosingEventArgs) + 'static,
    ) -> Self {
        self.pane_closing = Some(Rc::new(callback));
        self
    }

    /// Registers a handler for completion of a closing transition.
    pub fn pane_closed(mut self, callback: Listener) -> Self {
        self.pane_closed = Some(callback);
        self
    }

    /// Selects the template state for the current mode, open value and pane placement.
    fn visual_state(&self) -> DisplayModeState {
        use DisplayModeState::*;
        let right = self.pane_placement == SplitViewPanePlacement::Right;
        match (self.display_mode, self.is_pane_open, right) {
            (SplitViewDisplayMode::Overlay | SplitViewDisplayMode::Inline, false, _) => Closed,
            (_, false, false) => ClosedCompactLeft,
            (_, false, true) => ClosedCompactRight,
            (SplitViewDisplayMode::Overlay, true, false) => OpenOverlayLeft,
            (SplitViewDisplayMode::Overlay, true, true) => OpenOverlayRight,
            (SplitViewDisplayMode::CompactOverlay, true, false) => OpenCompactOverlayLeft,
            (SplitViewDisplayMode::CompactOverlay, true, true) => OpenCompactOverlayRight,
            (_, true, false) => OpenInlineLeft,
            (_, true, true) => OpenInlineRight,
        }
    }
}

impl fmt::Debug for SplitView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitView")
            .field("is_pane_open", &self.is_pane_open)
            .field("display_mode", &self.display_mode)
            .finish_non_exhaustive()
    }
}

/// Retains pane animation, focus and measured template values across widget updates.
pub struct SplitViewState {
    state: StateData<SplitView>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    ticker: Option<Handle<Ticker>>,
    pane_focus: Option<Handle<FocusScopeNode>>,
    content_focus: Option<Handle<FocusScopeNode>>,
    /// The focus target saved on overlay opening and consumed when restoring focus.
    previous_focus: Option<AnyFocusNode>,
    /// Hosts the outer dismiss layer above the application's content.
    portal: Option<Handle<OverlayPortalController>>,
    /// The source state of the currently evaluated transition.
    from: DisplayModeState,
    /// Elapsed time within the current transition.
    elapsed: Duration,
    /// Total transition duration; zero selects the settled target state.
    duration: Duration,
    /// Invalidates completion callbacks when a transition is replaced.
    epoch: u64,
    /// Whether the pending visual-state change should emit an opened or closed event.
    opening_or_closing: bool,
    /// Suppresses duplicate closing notifications while a light-dismiss request is pending.
    closing_by_dismiss: bool,
    /// Latest measured pane width, retained even in explicit-width mode for a later Auto update.
    measured_pane_length: f64,
    /// Live values captured at interruption for tracks without an explicit starting keyframe; set
    /// when a transition starts.
    initial_visuals: Option<split_view_visuals::Visuals>,
    /// The pane's lengths as the last build resolved them.
    lengths: (f64, f64),
    /// Previous window metrics used to detect changes that should light-dismiss the pane.
    root_metrics: Option<(Size, f64, EdgeInsets, EdgeInsets)>,
}

impl StatefulWidget for SplitView {
    type State = SplitViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SplitViewState {
        SplitViewState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            pane_focus: None,
            content_focus: None,
            previous_focus: None,
            portal: None,
            from: self.visual_state(),
            elapsed: Duration::ZERO,
            duration: Duration::ZERO,
            epoch: 0,
            opening_or_closing: false,
            closing_by_dismiss: false,
            measured_pane_length: 0.0,
            root_metrics: None,
            initial_visuals: None,
            lengths: (0.0, 0.0),
        }
    }
}

impl SingleTickerProviderStateMixin for SplitViewState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl SplitViewState {
    /// Returns the resolved lengths used by this control's template.
    ///
    /// For Auto pane width, these values reflect the latest completed layout.
    pub fn template_settings(self: Handle<Self>, app: &App) -> SplitViewTemplateSettings {
        let (open, compact) = app.get(self).lengths;
        let open = if open.is_nan() {
            app.get(self).measured_pane_length
        } else {
            open
        };
        SplitViewTemplateSettings::new(open, compact)
    }

    /// Whether an open overlay pane can start another light-dismiss request.
    fn can_light_dismiss(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_pane_open
            && self.widget(app).display_mode.light_dismissible()
            && !app.get(self).closing_by_dismiss
    }

    /// Raises `PaneClosing` to give the owner a chance to cancel before requesting closure.
    fn dismiss(self: Handle<Self>, app: &mut App) {
        if !app.contains(self) || !self.mounted(app) || !self.can_light_dismiss(app) {
            return;
        }
        let mut args = SplitViewPaneClosingEventArgs::default();
        let callback = self.widget(app).pane_closing.clone();
        app.get_mut(self).closing_by_dismiss = true;
        if let Some(callback) = callback {
            callback(app, &mut args);
        }
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        if args.cancel {
            app.get_mut(self).closing_by_dismiss = false;
            return;
        }
        let changed = self.widget(app).is_pane_open_changed.clone();
        changed(app, false);
        after_next_frame(
            app,
            View::maybe_id_of(app, self.context(app)),
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) && self.widget(app).is_pane_open {
                    app.get_mut(self).closing_by_dismiss = false;
                }
            }),
        );
    }

    /// Restores the focus target saved on opening, falling back to the content if it is gone.
    fn restore_focus(self: Handle<Self>, app: &mut App) {
        if let Some(previous) = app.get_mut(self).previous_focus.take() {
            if app.contains(previous.id())
                && !app.is_disposed(previous.id())
                && previous.context(app).is_some()
                && previous.parent(app).is_some()
                && previous.can_request_focus(app)
            {
                previous.request_focus(app, None);
            } else {
                app.get(self)
                    .content_focus
                    .unwrap()
                    .as_node()
                    .next_focus(app);
            }
        }
    }

    /// Saves the current focus target and moves focus into an opening overlay pane.
    fn focus_pane(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).display_mode.light_dismissible() || !self.widget(app).is_pane_open {
            return;
        }
        let pane = app.get(self).pane_focus.unwrap();
        if app.get(self).previous_focus.is_none() {
            app.get_mut(self).previous_focus = primary_focus(app);
        }
        let first = pane
            .as_node()
            .traversal_descendants(app)
            .into_iter()
            .find(|node| node.can_request_focus(app));
        first.unwrap_or(pane.as_node()).request_focus(app, None);
    }

    /// Delivers the opened or closed event only for the current transition.
    fn complete(self: Handle<Self>, app: &mut App, epoch: u64) {
        if !self.mounted(app) || app.get(self).epoch != epoch || !app.get(self).opening_or_closing {
            return;
        }
        app.get_mut(self).opening_or_closing = false;
        let open = self.widget(app).is_pane_open;
        if !open {
            app.get_mut(self).closing_by_dismiss = false;
        }
        let callback = if open {
            self.widget(app).pane_opened.clone()
        } else {
            self.widget(app).pane_closed.clone()
        };
        if let Some(callback) = callback {
            callback.call(app);
        }
    }

    /// Advances the active storyboard and queues its completion after the rendered frame.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        if !self.mounted(app) {
            return;
        }
        self.set_state(app, |s| s.elapsed = elapsed);
        if elapsed >= app.get(self).duration {
            app.get(self).ticker.unwrap().stop(app, false);
            let epoch = app.get(self).epoch;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) {
                        self.complete(app, epoch);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    /// Updates the outer dismiss layer after build, when native overlay mutations are allowed.
    fn update_portal(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) {
                    return;
                }
                let portal = app.get(self).portal.unwrap();
                let show = self.widget(app).is_pane_open
                    && self.widget(app).display_mode.light_dismissible();
                if show != portal.is_showing(app) {
                    if show {
                        portal.show(app);
                    } else {
                        portal.hide(app);
                    }
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }
}

/// `PaneLayout` is the pane's lengths, border and corners: the widget's own, else the theme's.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PaneLayout {
    /// The open pane width; `f64::NAN` sizes to the pane content.
    open: f64,
    /// The width reserved by a closed compact pane.
    compact: f64,
    /// Pane border widths in left, top, right, bottom order.
    border_thickness: [f64; 4],
    /// Pane radii in top-left, top-right, bottom-right, bottom-left order.
    corner_radius: [f64; 4],
}

impl SplitView {
    /// `pane_layout` is the pane's lengths, border and corners, the theme's where the widget sets
    /// none, as the style's setters do.
    fn pane_layout(&self, r: &ThemeResources) -> PaneLayout {
        PaneLayout {
            open: self
                .open_pane_length
                .unwrap_or(r.split_view_open_pane_theme_length),
            compact: self
                .compact_pane_length
                .unwrap_or(r.split_view_compact_pane_theme_length),
            border_thickness: self
                .border_thickness
                .unwrap_or(r.split_view_left_border_theme_thickness),
            corner_radius: self
                .corner_radius
                .unwrap_or(r.split_view_pane_root_corner_radius),
        }
    }
}

impl State for SplitViewState {
    type Widget = SplitView;
    inset_widgets::state_accessors!();
    fn init_state(self: Handle<Self>, app: &mut App) {
        let pane = FocusScopeNode::new(app);
        let content = FocusScopeNode::new(app);
        content.set_traversal_edge_behavior(app, TraversalEdgeBehavior::ParentScope);
        let portal = OverlayPortalController::new(app, Some("SplitView outer dismissal".into()));
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        let s = app.get_mut(self);
        s.pane_focus = Some(pane);
        s.content_focus = Some(content);
        s.portal = Some(portal);
        s.ticker = Some(ticker);
        self.update_portal(app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) {
                    self.focus_pane(app);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &SplitView) {
        let widget = self.widget(app).clone();
        let change = old.is_pane_open != widget.is_pane_open;
        let from = old.visual_state();
        let to = widget.visual_state();
        if old.display_mode != widget.display_mode {
            self.restore_focus(app);
        }
        self.update_portal(app);
        let r = ThemeResources::of(app, self.context(app));
        let (old_pane, pane) = (old.pane_layout(&r), widget.pane_layout(&r));
        let lengths_changed = !(old_pane.open == pane.open
            || (old_pane.open.is_nan() && pane.open.is_nan()))
            || old_pane.compact != pane.compact;
        if from == to && !lengths_changed {
            return;
        }
        let previous = app.get(self);
        let old_open = if old_pane.open.is_nan() {
            previous.measured_pane_length
        } else {
            old_pane.open
        };
        let initial = if previous.duration.is_zero() {
            split_view_visuals::settled(from, old_open, old_pane.compact)
        } else {
            split_view_visuals::transition_from_visuals(
                &r,
                previous.from,
                from,
                previous.elapsed,
                old_open,
                old_pane.compact,
                previous
                    .initial_visuals
                    .expect("set when a transition starts"),
            )
        };
        let ticker = app.get(self).ticker.unwrap();
        ticker.stop(app, false);
        let duration = if lengths_changed {
            Duration::ZERO
        } else {
            split_view_visuals::transition_duration(&r, from, to).unwrap_or_default()
        };
        let s = app.get_mut(self);
        s.initial_visuals = Some(initial);
        s.from = from;
        s.duration = duration;
        s.elapsed = Duration::ZERO;
        s.epoch += 1;
        s.opening_or_closing |= change;
        let epoch = s.epoch;
        if change {
            let by_dismiss = app.get(self).closing_by_dismiss;
            if !widget.is_pane_open {
                self.restore_focus(app);
            }
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if !app.contains(self) || !self.mounted(app) || app.get(self).epoch != epoch {
                        return;
                    }
                    if widget.is_pane_open {
                        if let Some(callback) = &widget.pane_opening {
                            callback.call(app);
                        }
                        if app.contains(self) && self.mounted(app) {
                            self.focus_pane(app);
                        }
                    } else if !by_dismiss && let Some(callback) = &widget.pane_closing {
                        callback(app, &mut SplitViewPaneClosingEventArgs::default());
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
        if duration.is_zero() {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) {
                        self.complete(app, epoch);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        } else {
            ticker.start(app);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let metrics = MediaQuery::maybe_of(app, self.context(app))
            .map(|m| (m.size, m.device_pixel_ratio, m.view_insets, m.view_padding));
        let previous = app.get(self).root_metrics;
        app.get_mut(self).root_metrics = metrics;
        if previous.is_some() && previous != metrics {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) && self.mounted(app) {
                        self.dismiss(app);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // OnUnloaded tears down dismissal; focus restoration belongs to closing or changing mode,
        // not to a subtree whose focus attachments have already been detached.
        app.get_mut(self).previous_focus = None;
        app.get(self).ticker.unwrap().dispose(app);
        let pane = app.get(self).pane_focus.unwrap();
        let content = app.get(self).content_focus.unwrap();
        pane.as_node().dispose(app);
        content.as_node().dispose(app);
        app.destroy(pane);
        app.destroy(content);
        app.destroy(app.get(self).portal.unwrap());
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let pane = app.get(self).pane_focus.unwrap();
        pane.set_traversal_edge_behavior(
            app,
            if widget.is_pane_open && widget.display_mode.light_dismissible() {
                TraversalEdgeBehavior::ClosedLoop
            } else {
                TraversalEdgeBehavior::ParentScope
            },
        );
        let resources = ThemeResources::of(app, context);
        let body = LayoutBuilder::new(move |app, _, constraints| {
            self.build_template(app, constraints, resources.clone())
        });
        let body = SizeObserver::new(
            body.into_widget(),
            Rc::new(move |app, old, _new| {
                if (old.width() != 0.0 || old.height() != 0.0) && self.can_light_dismiss(app) {
                    self.dismiss(app);
                }
            }),
        )
        .into_widget();
        let body = Focus::new(body)
            .skip_traversal(true)
            .can_request_focus(false)
            .on_key_event(Rc::new(move |app, _, event| {
                if matches!(event, KeyEvent::Down(_))
                    && matches!(
                        event.logical_key(),
                        LogicalKeyboardKey::ESCAPE | LogicalKeyboardKey::GAME_BUTTON_B
                    )
                    && self.can_light_dismiss(app)
                {
                    self.dismiss(app);
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            }))
            .into_widget();
        if Overlay::maybe_of(app, context, true).is_none() {
            assert!(
                !widget.is_pane_open || !widget.display_mode.light_dismissible(),
                "SplitView overlay modes require an Overlay ancestor for window-wide light dismissal"
            );
            return body;
        }
        OverlayPortal::overlay_child_layout_builder(
            app.get(self).portal.unwrap(),
            move |_app, _, info| {
                let size = info.child_size();
                let transform = info.child_paint_transform();
                let corners = [
                    Offset::ZERO,
                    Offset::new(0.0, size.height()),
                    Offset::new(size.width(), size.height()),
                    Offset::new(size.width(), 0.0),
                ]
                .map(|p| transform_point(&transform, p));
                ClipPath::new()
                    .clipper(Rc::new(OutsideClipper(corners)))
                    .child(
                        ModalBarrier::new().on_dismiss(Listener::new(move |app| self.dismiss(app))),
                    )
                    .into_widget()
            },
        )
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(body)
        .into_widget()
    }
}

impl SplitViewState {
    /// Builds the named template parts from the current visual-state setters and storyboard tracks.
    fn build_template(
        self: Handle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        resources: Rc<ThemeResources>,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let pane = widget.pane_layout(&resources);
        app.get_mut(self).lengths = (pane.open, pane.compact);
        let open = if pane.open.is_nan() {
            app.get(self).measured_pane_length
        } else {
            pane.open
        };
        let state = app.get(self);
        let v = if state.duration.is_zero() {
            split_view_visuals::settled(widget.visual_state(), open, pane.compact)
        } else {
            split_view_visuals::transition_from_visuals(
                &resources,
                state.from,
                widget.visual_state(),
                state.elapsed,
                open,
                pane.compact,
                state.initial_visuals.expect("set when a transition starts"),
            )
        };
        let pane_focus = state.pane_focus.unwrap();
        let content_focus = state.content_focus.unwrap();
        let pane_child = widget
            .pane
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        // The content probe stays at the same tree position when switching Auto and explicit widths.
        let pane_child = SizeObserver::new(
            pane_child,
            Rc::new(move |app, _, new| {
                if app.get(self).measured_pane_length != new.width() {
                    if app.get(self).lengths.0.is_nan() {
                        self.set_state(app, |s| s.measured_pane_length = new.width());
                    } else {
                        app.get_mut(self).measured_pane_length = new.width();
                    }
                }
            }),
        );
        let auto = pane.open.is_nan();
        let pane_child = OverflowBox::new()
            .alignment(Alignment::TOP_LEFT.into())
            .min_width(if auto {
                0.0
            } else {
                (open - pane.border_thickness[0] - pane.border_thickness[2]).max(0.0)
            })
            .max_width(if auto {
                constraints.max_width
            } else {
                (open - pane.border_thickness[0] - pane.border_thickness[2]).max(0.0)
            })
            .child(
                Align::new()
                    .alignment(Alignment::TOP_LEFT.into())
                    .width_factor(1.0)
                    .child(pane_child),
            );
        let pane_child = FocusScope::new(pane_child)
            .node(pane_focus)
            .descendants_are_focusable(v.pane_visible)
            .into_widget();
        let background = widget.pane_background.unwrap_or(Brush::Solid(
            resources.system_control_page_background_chrome_low_brush,
        ));
        let border_brush = widget.border_brush.unwrap_or(Brush::Solid(
            resources.system_control_foreground_transparent_brush,
        ));
        // PaneRoot: Grid chrome and its default BrushTransition (150 ms in SimplePropertiesMetadata.g.h).
        let pane_root = ColorTransition::new(
            background.representative_color(),
            Duration::from_millis(150),
            move |_app, color| {
                Grid::new()
                    .background(if matches!(background, Brush::Solid(_)) {
                        Brush::Solid(color)
                    } else {
                        background
                    })
                    .border_brush(border_brush)
                    .border_thickness_ltrb(pane.border_thickness)
                    .corner_radius_corners(pane.corner_radius)
                    .children([pane_child.clone()])
                    .into_widget()
            },
        );
        let pane_root = SizedBox::new().width(open).child(pane_root);
        // PaneClipRectangle / PaneClipRectangleTransform, then PaneTransform.
        // HCPaneBorder is transparent in both emitted themes.
        let pane_root = ClipRect::new()
            .clipper(PaneClipper(v.clip_translate))
            .child(pane_root);
        let pane_root = Transform::translate(Offset::new(v.pane_translate, 0.0)).child(pane_root);
        let pane_root = Visibility::new(pane_root)
            .visible(v.pane_visible)
            .maintain_state(true);
        let pane_root = OverflowBox::new()
            .min_width(open)
            .max_width(open)
            .alignment(
                if v.pane_right {
                    Alignment::CENTER_RIGHT
                } else {
                    Alignment::CENTER_LEFT
                }
                .into(),
            )
            .child(pane_root);
        // ContentRoot: the content Border followed by LightDismissLayer.
        let mut content = vec![
            FocusScope::new(
                widget
                    .content
                    .unwrap_or_else(|| SizedBox::shrink().into_widget()),
            )
            .node(content_focus)
            .into_widget(),
        ];
        let color = match widget.light_dismiss_overlay_mode.visual_state() {
            OverlayVisibilityState::OverlayVisible => {
                resources.split_view_light_dismiss_overlay_background
            }
            OverlayVisibilityState::OverlayNotVisible => Color::from_argb(0, 0, 0, 0),
        };
        content.push(
            Visibility::new(
                Opacity::new(v.overlay_opacity).child(
                    ModalBarrier::new()
                        .color(color)
                        .on_dismiss(Listener::new(move |app| self.dismiss(app))),
                ),
            )
            .visible(v.overlay_visible)
            .maintain_state(true)
            .into_widget(),
        );
        let content_root = Transform::translate(Offset::new(v.content_translate, 0.0))
            .child(Stack::new().fit(StackFit::Expand).children(content));
        let mut root = Grid::new();
        if let Some(background) = widget.background {
            root = root.background(background);
        }
        // ColumnDefinition1 / ColumnDefinition2. PaneRoot's Canvas.ZIndex=1 puts it above ContentRoot.
        root.column_definitions([
            ColumnDefinition::new(v.column1),
            ColumnDefinition::new(v.column2),
        ])
        .children([
            GridCell::new(content_root)
                .column(v.content_column)
                .column_span(v.content_span)
                .into_widget(),
            GridCell::new(pane_root)
                .column(v.pane_column)
                .column_span(v.pane_span)
                .into_widget(),
        ])
        .into_widget()
    }
}

/// The pane rectangle translated by `PaneClipRectangleTransform` before `PaneTransform`.
#[derive(Debug)]
struct PaneClipper(f64);
impl CustomClipper<Rect> for PaneClipper {
    fn get_clip(&self, size: Size) -> Rect {
        Rect::from_ltwh(self.0, 0.0, size.width(), size.height())
    }

    fn should_reclip(&self, old: &dyn CustomClipper<Rect>) -> bool {
        old.as_any()
            .downcast_ref::<Self>()
            .is_none_or(|old| old.0 != self.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The outer dismiss layer that detects interactions outside the SplitView bounds.
///
/// WinUI uses four polygonal elements for DComp input; the native overlay uses a clipped hole.
#[derive(Debug)]
struct OutsideClipper([Offset; 4]);
impl CustomClipper<Arc<Path>> for OutsideClipper {
    fn get_clip(&self, size: Size) -> Arc<Path> {
        let mut hole = self.0;
        // A reflected ancestor reverses the hole's winding.
        let signed_area: f64 = (0..4)
            .map(|i| {
                let a = hole[i];
                let b = hole[(i + 1) % 4];
                a.dx() * b.dy() - b.dx() * a.dy()
            })
            .sum();
        if signed_area > 0.0 {
            hole.reverse();
        }
        let mut path = PathBuilder::new();
        for polygon in [
            [
                Offset::ZERO,
                Offset::new(size.width(), 0.0),
                Offset::new(size.width(), size.height()),
                Offset::new(0.0, size.height()),
            ],
            hole,
        ] {
            path.move_to((polygon[0].dx() as f32, polygon[0].dy() as f32));
            for p in &polygon[1..] {
                path.line_to((p.dx() as f32, p.dy() as f32));
            }
            path.close();
        }
        path.build()
    }

    fn should_reclip(&self, old: &dyn CustomClipper<Arc<Path>>) -> bool {
        old.as_any()
            .downcast_ref::<Self>()
            .is_none_or(|old| old.0 != self.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
