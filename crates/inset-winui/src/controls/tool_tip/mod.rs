//! WinUI ToolTip chrome, automatic ToolTipService ownership, and in-window popup placement.

mod positioning;

use crate::{Brush, ControlBorder, ThemeResources, control_text_style};
use inset_embedder::{Color, FontWeight, Offset, PointerDeviceKind, Rect, Size, TextDirection};
use inset_foundation::{App, Handle, Listener, Timer, ValueKey};
use inset_gestures::{GestureBinding, PointerEvent, PointerRoute};
use inset_painting::transform_rect;
use inset_rendering::{
    BoxConstraints, ChildLayoutId, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{HardwareKeyboard, KeyEvent, KeyEventCallback, LogicalKeyboardKey};
use inset_widgets::*;
use std::{any::Any, cell::Cell, fmt, rc::Rc, time::Duration};

/// XAML `PlacementMode`, shared by ToolTip and ToolTipService.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlacementMode {
    /// Center above the placement target.
    #[default]
    Top,
    /// Center below the placement target.
    Bottom,
    /// Center beside the target's left edge.
    Left,
    /// Center beside the target's right edge.
    Right,
    /// Use the pointer position; keyboard and explicit opening fall back to Top.
    Mouse,
}

/// XAML `ToolTip`: the content, style and positioning properties used by ToolTipService.
#[derive(Clone)]
pub struct ToolTip {
    /// The content presented by `LayoutRoot`.
    pub content: WidgetRef,
    /// Preferred side; the source algorithm tries other sides if this side does not fit.
    pub placement: PlacementMode,
    /// Optional rectangle in the owner's coordinates, including Slider's thumb rectangle.
    pub placement_rect: Option<Rect>,
    /// A local offset; None uses the input-specific source offset.
    pub horizontal_offset: Option<f64>,
    /// A local offset; None uses the input-specific source offset.
    pub vertical_offset: Option<f64>,
    /// Optional background brush override.
    pub background: Option<Brush>,
    /// Optional foreground color override.
    pub foreground: Option<Color>,
    /// Optional border brush override.
    pub border_brush: Option<Brush>,
    /// Border thickness in left, top, right, bottom order; the theme's `ToolTipBorderThemeThickness`
    /// when `None`.
    pub border_thickness: Option<[f64; 4]>,
    /// Content padding in left, top, right, bottom order; the theme's `ToolTipBorderPadding` when
    /// `None`.
    pub padding: Option<[f64; 4]>,
    /// Maximum popup width before text wraps; the theme's `ToolTipMaxWidth` when `None`.
    pub max_width: Option<f64>,
    /// Rounded corners in top-left, top-right, bottom-right, bottom-left order; the theme's
    /// `ControlCornerRadius` when `None`.
    pub corner_radius: Option<[f64; 4]>,
    /// Raised when the native overlay has opened.
    pub opened: Option<Listener>,
    /// Raised after the closing fade has removed the overlay.
    pub closed: Option<Listener>,
}

impl ToolTip {
    /// Creates a tooltip with arbitrary widget content.
    pub fn new<K>(content: impl IntoWidget<K>) -> Self {
        Self {
            content: content.into_widget(),
            placement: PlacementMode::Top,
            placement_rect: None,
            horizontal_offset: None,
            vertical_offset: None,
            background: None,
            foreground: None,
            border_brush: None,
            border_thickness: None,
            padding: None,
            max_width: None,
            corner_radius: None,
            opened: None,
            closed: None,
        }
    }

    /// Creates text content that inherits the tooltip's font, foreground and wrapping.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Text::new(text))
    }

    /// Sets the preferred placement side.
    pub fn placement(mut self, placement: PlacementMode) -> Self {
        self.placement = placement;
        self
    }

    /// Sets an owner-relative placement rectangle, such as a Slider thumb's bounds.
    pub fn placement_rect(mut self, rect: Rect) -> Self {
        self.placement_rect = Some(rect);
        self
    }

    /// Sets the horizontal separation from the placement rectangle.
    pub fn horizontal_offset(mut self, offset: f64) -> Self {
        self.horizontal_offset = Some(offset);
        self
    }

    /// Sets the vertical separation from the placement rectangle.
    pub fn vertical_offset(mut self, offset: f64) -> Self {
        self.vertical_offset = Some(offset);
        self
    }

    /// Overrides the background brush.
    pub fn background(mut self, brush: Brush) -> Self {
        self.background = Some(brush);
        self
    }

    /// Overrides the text foreground.
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Overrides the border brush.
    pub fn border_brush(mut self, brush: Brush) -> Self {
        self.border_brush = Some(brush);
        self
    }

    /// Overrides all border sides equally.
    pub fn border_thickness(mut self, value: f64) -> Self {
        self.border_thickness = Some([value; 4]);
        self
    }

    /// Overrides content padding.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = Some(value);
        self
    }

    /// Overrides the wrapping width.
    pub fn max_width(mut self, width: f64) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Overrides all four corner radii equally.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some([value; 4]);
        self
    }

    /// Handles the Opened event.
    pub fn opened(mut self, callback: Listener) -> Self {
        self.opened = Some(callback);
        self
    }

    /// Handles the Closed event.
    pub fn closed(mut self, callback: Listener) -> Self {
        self.closed = Some(callback);
        self
    }
}

impl fmt::Debug for ToolTip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolTip")
            .field("placement", &self.placement)
            .field("max_width", &self.max_width)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ToolTip {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let style = control_text_style(
            &r,
            r.tool_tip_content_theme_font_size,
            FontWeight::W400,
            self.foreground.unwrap_or(r.tool_tip_foreground_brush),
        );
        // LayoutRoot, the template's ContentPresenter.
        let max_width = self.max_width.unwrap_or(r.tool_tip_max_width);
        ConstrainedBox::new(BoxConstraints::new().max_width(max_width))
            .child(
                ControlBorder::new(
                    self.background
                        .unwrap_or(Brush::Acrylic(r.tool_tip_background_brush)),
                    self.border_brush
                        .unwrap_or(Brush::Solid(r.tool_tip_border_brush)),
                )
                .border_thickness_ltrb(
                    self.border_thickness
                        .unwrap_or(r.tool_tip_border_theme_thickness),
                )
                .corner_radius_corners(self.corner_radius.unwrap_or(r.control_corner_radius))
                .padding(self.padding.unwrap_or(r.tool_tip_border_padding))
                .child(DefaultTextStyle::new(style, self.content.clone()).soft_wrap(true)),
            )
            .into_widget()
    }
}

/// ToolTipService attached properties, expressed as a stable wrapper around the owner widget.
#[derive(Clone, Debug)]
pub struct ToolTipService {
    /// Widget identity, retained while the tooltip opens and closes.
    pub key: Option<KeyRef>,
    /// The element that owns the tooltip and supplies its placement coordinates.
    pub child: WidgetRef,
    /// The tooltip to display.
    pub tool_tip: ToolTip,
    /// Whether automatic and explicit opening are enabled.
    pub is_enabled: bool,
    /// None uses automatic input; Some controls IsOpen explicitly without hover/focus delays.
    pub is_open: Option<bool>,
}

impl ToolTipService {
    /// Attaches a tooltip to its owner. A root Overlay ancestor is required when opening.
    pub fn new<K>(child: impl IntoWidget<K>, tool_tip: ToolTip) -> Self {
        Self {
            key: None,
            child: child.into_widget(),
            tool_tip,
            is_enabled: true,
            is_open: None,
        }
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Enables or disables this tooltip without disabling its owner.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Controls visibility explicitly; intended for controls such as Slider.
    pub fn is_open(mut self, value: bool) -> Self {
        self.is_open = Some(value);
        self
    }
}

/// The trigger that supplies the source's default delay and separation offsets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum InputMode {
    #[default]
    Explicit,
    Mouse,
    Touch,
    Keyboard,
}

/// One automatic owner per App, like WinUI's ToolTipServiceMetadata.
#[derive(Default)]
struct ServiceMetadata {
    active: Option<Handle<ToolTipServiceState>>,
    hovered: Vec<Handle<ToolTipServiceState>>,
    recent_close: bool,
    recent_timer: Option<Timer>,
    pointer_input: bool,
}

/// The overlay lifecycle of a ToolTipService owner.
pub struct ToolTipServiceState {
    state: StateData<ToolTipService>,
    portal: Option<Handle<OverlayPortalController>>,
    open_timer: Option<Timer>,
    close_timer: Option<Timer>,
    pointer_route: Option<PointerRoute>,
    key_handler: Option<KeyEventCallback>,
    input: InputMode,
    open: bool,
    visible: bool,
    suppressed: bool,
    pointer: Offset,
    popup_bounds: Rc<Cell<Rect>>,
    root_size: Option<Size>,
    epoch: u64,
}

impl StatefulWidget for ToolTipService {
    type State = ToolTipServiceState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ToolTipServiceState {
        ToolTipServiceState {
            state: StateData::new(),
            portal: None,
            open_timer: None,
            close_timer: None,
            pointer_route: None,
            key_handler: None,
            input: InputMode::Explicit,
            open: false,
            visible: false,
            suppressed: false,
            pointer: Offset::ZERO,
            popup_bounds: Rc::new(Cell::new(Rect::ZERO)),
            root_size: None,
            epoch: 0,
        }
    }
}

impl ToolTipServiceState {
    /// Whether the tooltip is open, including its closing fade.
    pub fn is_open(self: Handle<Self>, app: &App) -> bool {
        app.get(self).open
    }

    /// Whether automatic input is allowed to open this owner.
    fn automatic(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_enabled && self.widget(app).is_open.is_none()
    }

    /// Starts the source's input-specific show delay and transfers ownership from any previous tooltip.
    fn begin(self: Handle<Self>, app: &mut App, input: InputMode) {
        if !self.automatic(app) || app.get(self).suppressed {
            return;
        }
        let metadata = app.singleton::<ServiceMetadata>();
        if app.get(metadata).active == Some(self) {
            return;
        }
        if let Some(previous) = app.get(metadata).active
            && app.contains(previous)
            && previous.mounted(app)
        {
            previous.close(app);
        }
        app.get_mut(metadata).active = Some(self);
        app.get_mut(self).input = input;
        let recent = app.get(metadata).recent_close;
        // The source casts 1.5 to INT64 before multiplying its mouse reshow delay.
        let delay = match input {
            InputMode::Touch if recent => 0,
            InputMode::Touch => 400,
            InputMode::Mouse if recent => 400,
            _ => 800,
        };
        let timer = Timer::new(
            app,
            Duration::from_millis(delay),
            Listener::new(move |app| {
                if app.contains(self) && self.mounted(app) {
                    app.get_mut(self).open_timer = None;
                    self.show(app);
                }
            }),
        );
        app.get_mut(self).open_timer = Some(timer);
    }

    /// Opens the root overlay; state changes and callbacks remain outside widget construction.
    fn show(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).is_enabled {
            return;
        }
        if let Some(timer) = app.get_mut(self).close_timer.take() {
            timer.cancel(app);
        }
        if app.get(self).open {
            self.set_state(app, |s| {
                s.visible = true;
                s.epoch += 1;
            });
            return;
        }
        assert!(
            Overlay::maybe_of(app, self.context(app), true).is_some(),
            "ToolTipService requires an Overlay ancestor"
        );
        if let Some(timer) = app.get_mut(self).close_timer.take() {
            timer.cancel(app);
        }
        self.set_state(app, |s| {
            s.open = true;
            s.visible = false;
            s.epoch += 1;
        });
        let epoch = app.get(self).epoch;
        app.get(self).portal.unwrap().show(app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) || app.get(self).epoch != epoch {
                    return;
                }
                self.set_state(app, |s| s.visible = true);
                if let Some(callback) = self.widget(app).tool_tip.opened.clone() {
                    callback.call(app);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Cancels a pending show or fades out the active tooltip and relinquishes service ownership.
    fn close(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).open_timer.take() {
            timer.cancel(app);
        }
        app.get_mut(self).suppressed = true;
        let metadata = app.singleton::<ServiceMetadata>();
        if app.get(metadata).active == Some(self) {
            app.get_mut(metadata).active = None;
        }
        if !app.get(self).open || app.get(self).close_timer.is_some() {
            return;
        }
        if let Some(timer) = app.get_mut(metadata).recent_timer.take() {
            timer.cancel(app);
        }
        app.get_mut(metadata).recent_close = true;
        let timer = Timer::new(
            app,
            Duration::from_millis(200),
            Listener::new(move |app| {
                app.get_mut(metadata).recent_close = false;
                app.get_mut(metadata).recent_timer = None;
            }),
        );
        app.get_mut(metadata).recent_timer = Some(timer);
        self.set_state(app, |s| {
            s.visible = false;
            s.epoch += 1;
        });
        let epoch = app.get(self).epoch;
        let timer = Timer::new(
            app,
            Duration::from_millis(150),
            Listener::new(move |app| {
                if !app.contains(self) || !self.mounted(app) || app.get(self).epoch != epoch {
                    return;
                }
                app.get_mut(self).close_timer = None;
                app.get(self).portal.unwrap().hide(app);
                self.set_state(app, |s| s.open = false);
                if let Some(callback) = self.widget(app).tool_tip.closed.clone() {
                    callback.call(app);
                }
            }),
        );
        app.get_mut(self).close_timer = Some(timer);
    }

    /// Records nested pointer owners and starts their tooltip delay.
    fn enter(self: Handle<Self>, app: &mut App, point: Offset) {
        app.get_mut(self).pointer = point;
        app.get_mut(self).suppressed = false;
        let metadata = app.singleton::<ServiceMetadata>();
        if !app.get(metadata).hovered.contains(&self) {
            app.get_mut(metadata).hovered.push(self);
        }
        self.begin(app, InputMode::Mouse);
    }

    /// Removes an owner from the hover list; open tooltips retain their convex safe zone.
    fn leave(self: Handle<Self>, app: &mut App) {
        let metadata = app.singleton::<ServiceMetadata>();
        app.get_mut(metadata).hovered.retain(|owner| *owner != self);
        app.get_mut(self).suppressed = false;
        if !app.get(self).open {
            self.close(app);
            app.get_mut(self).suppressed = false;
        }
    }

    /// Routes in-window input through the source's safe-zone and dismissal policy.
    fn pointer_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        let metadata = app.singleton::<ServiceMetadata>();
        if matches!(event, PointerEvent::Down(_)) {
            app.get_mut(metadata).pointer_input = true;
        }
        if !self.automatic(app) {
            return;
        }
        if app.get(metadata).active == Some(self) {
            match event {
                PointerEvent::Down(_)
                    if app.get(self).input == InputMode::Touch && !app.get(self).open => {}
                PointerEvent::Up(_) if app.get(self).input == InputMode::Touch => self.close(app),
                PointerEvent::Down(_)
                | PointerEvent::Cancel(_)
                | PointerEvent::Removed(_)
                | PointerEvent::Scroll(_) => self.close(app),
                PointerEvent::Hover(_) | PointerEvent::Move(_) => {
                    if !app.get(self).open {
                        app.get_mut(self).pointer = event.position();
                    }
                    if app.get(self).open {
                        let owner = self
                            .context(app)
                            .find_render_object(app)
                            .and_then(|r| r.as_box());
                        if let Some(owner) = owner {
                            let origin = owner.local_to_global(app, Offset::ZERO, None);
                            let owner_rect = origin & owner.size(app);
                            if !positioning::safe_zone(
                                event.position(),
                                owner_rect,
                                app.get(self).popup_bounds.get(),
                            ) {
                                self.close(app);
                            }
                        }
                    }
                }
                _ => {}
            }
        } else if app.get(metadata).active.is_none()
            && matches!(event, PointerEvent::Hover(_) | PointerEvent::Move(_))
            && app.get(metadata).hovered.last() == Some(&self)
        {
            self.begin(app, InputMode::Mouse);
        }
    }
}

impl State for ToolTipServiceState {
    type Widget = ToolTipService;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).portal = Some(OverlayPortalController::new(
            app,
            Some("ToolTip popup".into()),
        ));
        let route = PointerRoute::new(move |app, event| {
            if app.contains(self) && self.mounted(app) {
                self.pointer_event(app, event);
            }
        });
        let router = GestureBinding::instance(app).pointer_router(app);
        router.add_global_route(app, route.clone(), None);
        app.get_mut(self).pointer_route = Some(route);
        let handler: KeyEventCallback = Rc::new(move |app, event| {
            if !app.contains(self) || !self.mounted(app) || !matches!(event, KeyEvent::Down(_)) {
                return false;
            }
            let metadata = app.singleton::<ServiceMetadata>();
            app.get_mut(metadata).pointer_input = false;
            if app.get(metadata).active == Some(self)
                && (!app.get(self).open || !special_key(event.logical_key()))
            {
                self.close(app);
            }
            false
        });
        HardwareKeyboard::instance(app).add_handler(app, handler.clone());
        app.get_mut(self).key_handler = Some(handler);
        if self.widget(app).is_open == Some(true) {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) && self.mounted(app) {
                        self.show(app);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ToolTipService) {
        if old.is_open != self.widget(app).is_open || old.is_enabled != self.widget(app).is_enabled
        {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if !app.contains(self) || !self.mounted(app) {
                        return;
                    }
                    if self.widget(app).is_enabled && self.widget(app).is_open == Some(true) {
                        app.get_mut(self).input = InputMode::Explicit;
                        self.show(app);
                    } else {
                        self.close(app);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let size = MediaQuery::maybe_size_of(app, self.context(app));
        let changed = app.get(self).root_size.is_some() && app.get(self).root_size != size;
        app.get_mut(self).root_size = size;
        if changed && app.get(self).input != InputMode::Explicit {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) && self.mounted(app) {
                        self.close(app);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for timer in [
            app.get_mut(self).open_timer.take(),
            app.get_mut(self).close_timer.take(),
        ]
        .into_iter()
        .flatten()
        {
            timer.cancel(app);
        }
        let metadata = app.singleton::<ServiceMetadata>();
        app.get_mut(metadata).hovered.retain(|owner| *owner != self);
        if app.get(metadata).active == Some(self) {
            app.get_mut(metadata).active = None;
        }
        if let Some(route) = app.get_mut(self).pointer_route.take() {
            GestureBinding::instance(app)
                .pointer_router(app)
                .remove_global_route(app, &route);
        }
        if let Some(handler) = app.get_mut(self).key_handler.take() {
            HardwareKeyboard::instance(app).remove_handler(app, &handler);
        }
        app.destroy(app.get(self).portal.unwrap());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        let child = Focus::new(child)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_focus_change(move |app, focused| {
                let metadata = app.singleton::<ServiceMetadata>();
                if focused && !app.get(metadata).pointer_input {
                    app.get_mut(self).suppressed = false;
                    self.begin(app, InputMode::Keyboard);
                } else if !focused && app.get(self).input == InputMode::Keyboard {
                    self.close(app);
                }
            });
        let child = MouseRegion::new()
            .opaque(false)
            .on_enter(Rc::new(move |app, event| self.enter(app, event.position)))
            .on_exit(Rc::new(move |app, _| self.leave(app)))
            .child(child);
        let child = inset_widgets::Listener::new()
            .on_pointer_down(Rc::new(move |app, event| {
                if event.kind == PointerDeviceKind::Touch {
                    app.get_mut(self).pointer = event.position;
                    app.get_mut(self).suppressed = false;
                    self.begin(app, InputMode::Touch);
                }
            }))
            .child(child);
        let rtl = Directionality::maybe_of(app, context) == Some(TextDirection::Rtl);
        OverlayPortal::overlay_child_layout_builder(
            app.get(self).portal.unwrap(),
            move |app, _, info| {
                let widget = self.widget(app).clone();
                let owner_rect = transform_rect(
                    &info.child_paint_transform(),
                    Offset::ZERO & info.child_size(),
                );
                let dock = widget
                    .tool_tip
                    .placement_rect
                    .map(|r| transform_rect(&info.child_paint_transform(), r))
                    .unwrap_or(owner_rect);
                let owner_global = self
                    .context(app)
                    .find_render_object(app)
                    .and_then(|r| r.as_box())
                    .map(|r| r.local_to_global(app, Offset::ZERO, None))
                    .unwrap_or(Offset::ZERO);
                let root_offset = owner_global - owner_rect.top_left();
                let s = app.get(self);
                let input = s.input;
                let pointer = s.pointer - root_offset;
                let rect = if widget.tool_tip.placement_rect.is_none()
                    && matches!(input, InputMode::Mouse | InputMode::Touch)
                {
                    Rect::from_ltwh(pointer.dx(), pointer.dy(), 0.0, 0.0)
                } else {
                    dock
                };
                let default_offset = if widget.tool_tip.placement == PlacementMode::Mouse
                    && matches!(input, InputMode::Mouse | InputMode::Touch)
                {
                    0.0
                } else {
                    match input {
                        InputMode::Explicit => 0.0,
                        InputMode::Keyboard => 12.0,
                        InputMode::Mouse => 20.0,
                        InputMode::Touch => 44.0,
                    }
                };
                let offset = Offset::new(
                    widget.tool_tip.horizontal_offset.unwrap_or(default_offset),
                    widget.tool_tip.vertical_offset.unwrap_or(default_offset),
                );
                let mut side = widget.tool_tip.placement;
                if rtl {
                    side = match side {
                        PlacementMode::Left => PlacementMode::Right,
                        PlacementMode::Right => PlacementMode::Left,
                        _ => side,
                    };
                }
                if side == PlacementMode::Mouse
                    && !matches!(input, InputMode::Mouse | InputMode::Touch)
                {
                    side = PlacementMode::Top;
                }
                let delegate = PopupLayout {
                    target: rect,
                    placement: side,
                    offset,
                    pointer,
                    rtl,
                    root_offset,
                    bounds: s.popup_bounds.clone(),
                };
                let popup = ExcludeFocus::new(
                    AnimatedOpacity::new(
                        if s.visible { 1.0 } else { 0.0 },
                        Duration::from_millis(150),
                    )
                    .child(widget.tool_tip),
                );
                CustomMultiChildLayout::new(Rc::new(delegate))
                    .children([LayoutId::new(popup_id(), popup).into_widget()])
                    .into_widget()
            },
        )
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(child)
        .into_widget()
    }
}

/// Keys WinUI allows to leave an already-open tooltip visible.
fn special_key(key: LogicalKeyboardKey) -> bool {
    matches!(
        key,
        LogicalKeyboardKey::ALT
            | LogicalKeyboardKey::BACKSPACE
            | LogicalKeyboardKey::DELETE
            | LogicalKeyboardKey::ARROW_DOWN
            | LogicalKeyboardKey::END
            | LogicalKeyboardKey::HOME
            | LogicalKeyboardKey::INSERT
            | LogicalKeyboardKey::ARROW_LEFT
            | LogicalKeyboardKey::PAGE_DOWN
            | LogicalKeyboardKey::PAGE_UP
            | LogicalKeyboardKey::ARROW_RIGHT
            | LogicalKeyboardKey::SPACE
            | LogicalKeyboardKey::ARROW_UP
    )
}

/// The single layout child, named like the template's LayoutRoot.
fn popup_id() -> ChildLayoutId {
    Rc::new(ValueKey::new("ToolTip.LayoutRoot"))
}

/// Places the measured popup in the root overlay without rebuilding after measurement.
#[derive(Debug)]
struct PopupLayout {
    target: Rect,
    placement: PlacementMode,
    offset: Offset,
    pointer: Offset,
    rtl: bool,
    root_offset: Offset,
    bounds: Rc<Cell<Rect>>,
}

impl MultiChildLayoutDelegate for PopupLayout {
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        let measured = children.layout_child(app, &popup_id(), BoxConstraints::loose(size));
        let bounds = Offset::ZERO & size;
        let rect = if self.placement == PlacementMode::Mouse {
            positioning::mouse_position(bounds, measured, self.pointer, self.offset, self.rtl)
        } else {
            let target = Rect::from_ltrb(
                self.target.left - self.offset.dx(),
                self.target.top - self.offset.dy(),
                self.target.right + self.offset.dx(),
                self.target.bottom + self.offset.dy(),
            );
            positioning::relative_position(bounds, measured, target, self.placement)
        };
        children.position_child(app, &popup_id(), rect.top_left());
        self.bounds.set(rect.shift(self.root_offset));
    }

    fn should_relayout(&self, _old: &dyn MultiChildLayoutDelegate) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
