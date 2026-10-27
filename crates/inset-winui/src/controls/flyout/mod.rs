//! Flyout.cpp and FlyoutBase_partial.cpp: reusable content, placement and closing events.

mod layout;
mod placement;
pub(crate) mod shadow;
pub use placement::FlyoutPlacementMode;

use crate::*;
use inset_embedder::{FontWeight, Offset, Rect, Size};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Listener};
use inset_gestures::{GestureBinding, PointerEvent, PointerRoute};
use inset_painting::{Axis, EdgeInsetsGeometry, transform_point, transform_rect};
use inset_rendering::BoxConstraints;
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use layout::{FlyoutLayout, FlyoutLayoutSettings};
use std::{collections::HashMap, fmt, rc::Rc};

/// Source policy for focus acquisition and outside input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlyoutShowMode {
    /// Resolves to Standard, as in UpdateStateToShowMode.
    Auto,
    /// Takes focus and consumes outside taps that close the flyout.
    #[default]
    Standard,
    /// Does not take focus and lets outside taps continue to underlying widgets.
    Transient,
    /// Also closes when the pointer moves more than 80 logical pixels away.
    TransientWithDismissOnPointerMoveAway,
}

impl FlyoutShowMode {
    /// Normalizes the source Auto value.
    fn effective(self) -> Self {
        if self == Self::Auto {
            Self::Standard
        } else {
            self
        }
    }
}

/// Per-opening overrides; placement is inherited when absent.
#[derive(Clone, Copy, Debug)]
pub struct FlyoutShowOptions {
    /// Override of the flyout's preferred side and alignment for this opening.
    pub placement: Option<FlyoutPlacementMode>,

    /// Optional point relative to the opening target.
    pub position: Option<Offset>,

    /// Optional target-relative rectangle avoided by point placement.
    pub exclusion_rect: Option<Rect>,

    /// Source default Auto resolves to Standard for ShowAtWithOptions.
    pub show_mode: FlyoutShowMode,
}

impl Default for FlyoutShowOptions {
    fn default() -> Self {
        Self {
            placement: None,
            position: None,
            exclusion_rect: None,
            show_mode: FlyoutShowMode::Auto,
        }
    }
}

/// Arguments passed before hiding a flyout.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlyoutBaseClosingEventArgs {
    /// Set true to leave the flyout open.
    pub cancel: bool,
}

/// Cancelable source Closing callback.
pub type FlyoutClosingHandler = Rc<dyn Fn(&mut App, &mut FlyoutBaseClosingEventArgs)>;

/// Native presenter customization, replacing a XAML presenter style.
pub type FlyoutPresenterBuilder = Rc<dyn Fn(&mut App, BuildContext, WidgetRef) -> WidgetRef>;

/// Source open/close phases, including time spent waiting for presenter layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Closed,
    Opening,
    Open,
    Closing,
}

/// One open flyout per root overlay or parent flyout, as tracked by FlyoutMetadata.
#[derive(Default)]
struct FlyoutMetadata {
    /// Current open or closing flyout for each scope.
    open: HashMap<HandleId, Handle<Flyout>>,

    /// Most recent opening waiting for that scope's current flyout to close.
    staged: HashMap<HandleId, (Handle<Flyout>, BuildContext, FlyoutShowOptions)>,
}

/// Identifies the parent flyout when opening another flyout from its content.
#[derive(Debug)]
struct FlyoutScope {
    /// Owning flyout for nested opening targets.
    flyout: Handle<Flyout>,

    /// Retained content inheriting this owner.
    child: WidgetRef,
}

impl InheritedWidget for FlyoutScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &Self) -> bool {
        self.flyout != old.flyout
    }
}

/// FlyoutPresenter's style: `MinWidth`, `MinHeight`, `MaxWidth` and `MaxHeight` from the theme's
/// `FlyoutThemeMinWidth`, `FlyoutThemeMinHeight`, `FlyoutThemeMaxWidth` and `FlyoutThemeMaxHeight`.
pub fn flyout_presenter_sizes(r: &ThemeResources) -> (Size, Size) {
    (
        Size::new(r.flyout_theme_min_width, r.flyout_theme_min_height),
        Size::new(r.flyout_theme_max_width, r.flyout_theme_max_height),
    )
}

/// A persistent flyout object whose child State survives closing and changing opening targets.
///
/// Create it once with the App, use contexts beneath FlyoutTarget for ShowAt,
/// and dispose it when the owning application state is disposed.
pub struct Flyout {
    /// Content displayed by the default presenter.
    pub content: WidgetRef,

    /// Preferred placement; the source default is Top.
    pub placement: FlyoutPlacementMode,

    /// Focus and outside-input policy used by ShowAt.
    pub show_mode: FlyoutShowMode,

    /// Default presenter's content padding in left, top, right, bottom order; the theme's
    /// `FlyoutContentPadding` when `None`.
    pub padding: Option<[f64; 4]>,

    /// Minimum and maximum presenter sizes, overriding the presenter style's.
    pub minimum_size: Option<Size>,
    pub maximum_size: Option<Size>,

    /// The presenter style's `MinWidth`, `MinHeight`, `MaxWidth` and `MaxHeight`, from the
    /// theme: FlyoutPresenter's unless a derived flyout's presenter has its own.
    pub presenter_sizes: fn(&ThemeResources) -> (Size, Size),

    /// Optional replacement presenter; receives the persistent content widget.
    pub flyout_presenter_style: Option<FlyoutPresenterBuilder>,

    /// Derived controls prepare their content before the public Opening event.
    pub(crate) on_opening: Option<Listener>,

    /// Called before mounting or showing the presenter.
    pub opening: Option<Listener>,

    /// Called after presenter layout and the initial focus request.
    pub opened: Option<Listener>,

    /// Can cancel hiding the flyout.
    pub closing: Option<FlyoutClosingHandler>,

    /// Called after the presenter is hidden.
    pub closed: Option<Listener>,

    /// Submenu popups share their root menu's dismissing layer.
    pub(crate) is_sub_menu: bool,

    /// A menu bar and its popups share a native TapRegion group.
    pub(crate) tap_region_group: Option<HandleId>,

    /// Point-positioned menu policy, set from its opening device.
    point_kind: placement::PointPlacementKind,

    /// Derived-control cleanup before the public Closed event.
    pub(crate) on_closed: Option<Listener>,

    /// Native retained content owner, initialized with this object's presenter builder.
    host: Option<Handle<RetainedFlyoutHost>>,

    /// Notifies compound buttons when the open state changes.
    notifier: ChangeNotifierData,

    /// Current source lifecycle phase.
    phase: Phase,

    /// Parent flyout or root overlay that owns the one-open-flyout slot.
    scope: Option<HandleId>,

    /// Parent whose outside-tap region also includes this presenter.
    parent: Option<Handle<Flyout>>,

    /// Opening context used to identify repeated ShowAt calls at the same target.
    target: Option<BuildContext>,

    /// Per-opening placement override.
    placement_override: Option<FlyoutPlacementMode>,

    /// Requested point transformed into root-overlay coordinates.
    position: Option<Offset>,

    /// Rectangle to avoid for point placement, in root-overlay coordinates.
    exclusion: Rect,

    /// Last arranged target bounds, also used by hidden retained content.
    last_target_bounds: Rect,

    /// Identifies the presenter's render box for pointer-distance dismissal.
    presenter_key: GlobalKey,

    /// Global pointer-movement listener used only by the pointer-distance show mode.
    pointer_route: Option<PointerRoute>,

    /// Guards late callbacks after explicit disposal.
    disposed: bool,
}

impl fmt::Debug for Flyout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Flyout")
            .field("placement", &self.placement)
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}

impl ChangeNotifier for Flyout {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

impl Flyout {
    /// Creates a source Flyout backed by the approved retained overlay adapter.
    pub fn new<K>(app: &mut App, content: impl IntoWidget<K>) -> Handle<Self> {
        let this = app.create(Self {
            content: content.into_widget(),
            placement: FlyoutPlacementMode::Top,
            show_mode: FlyoutShowMode::Standard,
            padding: None,
            minimum_size: None,
            maximum_size: None,
            presenter_sizes: flyout_presenter_sizes,
            flyout_presenter_style: None,
            opening: None,
            on_opening: None,
            opened: None,
            closing: None,
            closed: None,
            host: None,
            is_sub_menu: false,
            tap_region_group: None,
            point_kind: placement::PointPlacementKind::Flyout,
            on_closed: None,
            notifier: ChangeNotifierData::new(),
            phase: Phase::Closed,
            scope: None,
            parent: None,
            target: None,
            placement_override: None,
            position: None,
            exclusion: Rect::ZERO,
            last_target_bounds: Rect::ZERO,
            presenter_key: GlobalKey::new(),
            pointer_route: None,
            disposed: false,
        });
        let host = RetainedFlyoutHost::new(
            app,
            Rc::new(move |app, context| this.build_presenter(app, context)),
        );
        app.get_mut(host).on_key_event = Some(Rc::new(move |app, _, event| {
            if matches!(event, KeyEvent::Down(_))
                && event.logical_key() == LogicalKeyboardKey::ESCAPE
            {
                this.hide(app);
                KeyEventResult::Handled
            } else {
                KeyEventResult::Ignored
            }
        }));
        app.get_mut(host).on_dispose = Some(Listener::new(move |app| this.dispose(app)));
        app.get_mut(host).opened = Some(Listener::new(move |app| this.on_opened(app)));
        app.get_mut(host).closed = Some(Listener::new(move |app| this.on_closed(app)));
        app.get_mut(host).target_removed = Some(Listener::new(move |app| {
            this.hide(app);
        }));
        app.get_mut(this).host = Some(host);
        this
    }

    /// MenuFlyout's point-positioning policy depends on the input that opened it.
    pub(crate) fn menu_point_placement(self: Handle<Self>, app: &mut App, touch: bool) {
        app.get_mut(self).point_kind = if touch {
            placement::PointPlacementKind::TouchMenu
        } else {
            placement::PointPlacementKind::Menu
        };
    }

    /// Supplies the menu header that receives focus after its popup closes.
    pub(crate) fn return_focus_to(self: Handle<Self>, app: &mut App, node: AnyFocusNode) {
        let host = app.get(self).host.unwrap();
        app.get_mut(host).return_focus = Some(node);
    }

    /// Sets Content and updates the retained presenter.
    pub fn content<K>(
        self: Handle<Self>,
        app: &mut App,
        content: impl IntoWidget<K>,
    ) -> Handle<Self> {
        app.get_mut(self).content = content.into_widget();
        self.invalidate(app);
        self
    }

    /// Sets Placement.
    pub fn placement(
        self: Handle<Self>,
        app: &mut App,
        placement: FlyoutPlacementMode,
    ) -> Handle<Self> {
        app.get_mut(self).placement = placement;
        self.invalidate(app);
        self
    }

    /// Sets ShowMode.
    pub fn show_mode(self: Handle<Self>, app: &mut App, mode: FlyoutShowMode) -> Handle<Self> {
        app.get_mut(self).show_mode = mode.effective();
        self.update_pointer_route(app);
        self.invalidate(app);
        self
    }

    /// Sets the presenter style callback.
    pub fn flyout_presenter_style(
        self: Handle<Self>,
        app: &mut App,
        builder: FlyoutPresenterBuilder,
    ) -> Handle<Self> {
        app.get_mut(self).flyout_presenter_style = Some(builder);
        self.invalidate(app);
        self
    }

    /// Whether the popup is shown, including its initial layout frame.
    pub fn is_open(self: Handle<Self>, app: &App) -> bool {
        matches!(app.get(self).phase, Phase::Opening | Phase::Open)
    }

    /// Source ShowAt preserves the configured ShowMode.
    pub fn show_at(self: Handle<Self>, app: &mut App, target: BuildContext) {
        let options = FlyoutShowOptions {
            placement: None,
            show_mode: app.get(self).show_mode,
            ..Default::default()
        };
        self.show_at_with_options(app, target, options);
    }

    /// Shows at a target beneath FlyoutTarget, staging behind a closing sibling flyout.
    pub fn show_at_with_options(
        self: Handle<Self>,
        app: &mut App,
        target: BuildContext,
        options: FlyoutShowOptions,
    ) {
        assert!(!app.get(self).disposed, "cannot show a disposed Flyout");
        if !target.mounted(app) {
            return;
        }
        let root_overlay = Overlay::of(app, target, true);
        let transform = target
            .find_render_object(app)
            .unwrap()
            .get_transform_to(app, root_overlay.context(app).find_render_object(app));
        let position = options.position.map(|point| {
            assert!(
                !point.dx().is_nan() && !point.dy().is_nan(),
                "flyout position cannot be NaN"
            );
            transform_point(&transform, point)
        });
        let exclusion = options
            .exclusion_rect
            .map(|rect| transform_rect(&transform, rect))
            .unwrap_or(Rect::ZERO);
        let same_position = app.get(self).position == position;
        app.get_mut(self).position = position;
        app.get_mut(self).exclusion = exclusion;
        app.get_mut(self).placement_override = options.placement;
        app.get_mut(self).show_mode = options.show_mode.effective();
        self.update_pointer_route(app);
        self.refresh_chain(app);

        let parent = target
            .get_inherited_widget_of_exact_type::<FlyoutScope>(app)
            .map(|scope| scope.flyout);
        let scope = parent
            .map(|parent| parent.id())
            .unwrap_or_else(|| Overlay::of(app, target, true).id());
        let metadata = app.singleton::<FlyoutMetadata>();
        let current = app
            .get(metadata)
            .open
            .get(&scope)
            .copied()
            .filter(|flyout| app.contains(*flyout) && !app.get(*flyout).disposed);
        if let Some(current) = current {
            if current == self && app.get(self).target == Some(target) && same_position {
                return;
            }

            let already_staged = app.get(metadata).staged.contains_key(&scope);
            if !already_staged {
                current.hide(app);
            }
            app.get_mut(metadata)
                .staged
                .insert(scope, (self, target, options));
            return;
        }
        app.get_mut(metadata).open.insert(scope, self);
        let flyout = app.get_mut(self);
        flyout.scope = Some(scope);
        flyout.parent = parent;
        flyout.target = Some(target);
        flyout.placement_override = options.placement;
        flyout.show_mode = options.show_mode.effective();
        flyout.phase = Phase::Opening;
        if let Some(prepare) = flyout.on_opening.clone() {
            prepare.call(app);
        }
        if let Some(opening) = app.get(self).opening.clone() {
            opening.call(app);
        }
        if app.get(self).disposed || app.get(self).phase != Phase::Opening {
            return;
        }
        let host = app.get(self).host.unwrap();
        host.show_at(
            app,
            target,
            app.get(self).show_mode == FlyoutShowMode::Standard,
        );
        self.update_pointer_route(app);
        self.refresh_chain(app);
        self.notify_listeners(app);
    }

    /// Raises Closing before hiding; returns false when the handler cancels.
    pub fn hide(self: Handle<Self>, app: &mut App) -> bool {
        if app.get(self).disposed {
            return true;
        }
        let mut args = FlyoutBaseClosingEventArgs::default();
        if let Some(closing) = app.get(self).closing.clone() {
            closing(app, &mut args);
        }
        if args.cancel {
            return false;
        }
        if matches!(app.get(self).phase, Phase::Closed | Phase::Closing) {
            return true;
        }
        let metadata = app.singleton::<FlyoutMetadata>();
        if let Some(child) = app.get(metadata).open.get(&self.id()).copied()
            && child.is_open(app)
        {
            child.hide(app);
        }
        self.remove_pointer_route(app);
        let host = app.get(self).host.unwrap();
        if host.is_open(app) {
            app.get_mut(self).phase = Phase::Closing;
            host.hide(app);
        } else {
            app.get_mut(self).phase = Phase::Closed;
            if let Some(scope) = app.get(self).scope {
                app.get_mut(metadata).open.remove(&scope);
            }
        }
        self.refresh_chain(app);
        self.notify_listeners(app);
        true
    }

    /// Releases this flyout and its retained content.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }
        self.remove_pointer_route(app);
        app.get_mut(self).disposed = true;
        app.get_mut(self).phase = Phase::Closed;
        app.get(self).host.unwrap().dispose(app);
        let metadata = app.singleton::<FlyoutMetadata>();
        app.get_mut(metadata).open.retain(|_, value| *value != self);
        app.get_mut(metadata)
            .staged
            .retain(|_, (flyout, _, _)| *flyout != self);
        app.get_mut(self).notifier.dispose();
    }

    /// Updates derived presenter content without broadcasting an open-state change.
    pub(crate) fn rebuild_content(self: Handle<Self>, app: &mut App) {
        app.get(self).host.unwrap().rebuild(app);
    }

    /// Visible container bounds for pointer-distance and submenu-exit checks.
    pub(crate) fn presenter_bounds(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        let key = app.get(self).presenter_key.clone();
        let context = key.current_context(app)?;
        let render = context.find_render_object(app)?.as_box()?;
        Some(render.local_to_global(app, Offset::ZERO, None) & render.size(app))
    }

    /// Refreshes widgets using presenter properties or open-state visuals.
    fn invalidate(self: Handle<Self>, app: &mut App) {
        self.refresh_chain(app);
        self.notify_listeners(app);
    }

    /// Finds the first flyout in this chain of opening targets.
    fn root(self: Handle<Self>, app: &App) -> Handle<Self> {
        let mut root = self;
        while let Some(parent) = app.get(root).parent {
            root = parent;
        }
        root
    }

    /// Resolves the uppermost open flyout from the source child metadata.
    fn topmost(self: Handle<Self>, app: &mut App) -> Handle<Self> {
        let metadata = app.singleton::<FlyoutMetadata>();
        let mut current = self.root(app);
        let mut topmost = current;
        while let Some(child) = app.get(metadata).open.get(&current.id()).copied() {
            if !child.is_open(app) {
                break;
            }
            current = child;
            if !app.get(current).is_sub_menu {
                topmost = current;
            }
        }
        topmost
    }

    /// Rebuilds barriers when the uppermost flyout or its input policy changes.
    fn refresh_chain(self: Handle<Self>, app: &mut App) {
        let metadata = app.singleton::<FlyoutMetadata>();
        let mut current = Some(self.root(app));
        while let Some(flyout) = current {
            app.get(flyout).host.unwrap().rebuild(app);
            current = app.get(metadata).open.get(&flyout.id()).copied();
        }
    }

    /// Closes the child chain without opening a previously staged replacement.
    fn close_children(self: Handle<Self>, app: &mut App) {
        let metadata = app.singleton::<FlyoutMetadata>();
        app.get_mut(metadata).staged.remove(&self.id());
        if let Some(child) = app.get(metadata).open.get(&self.id()).copied() {
            child.hide(app);
        }
    }

    /// Presenter-loaded event, delivered after focus and layout by the retained host.
    fn on_opened(self: Handle<Self>, app: &mut App) {
        if !app.contains(self) || app.get(self).disposed || app.get(self).phase != Phase::Opening {
            return;
        }
        app.get_mut(self).phase = Phase::Open;
        if let Some(opened) = app.get(self).opened.clone() {
            opened.call(app);
        }
    }

    /// Presenter-unloaded event; releases the slot and opens the most recent staged flyout.
    fn on_closed(self: Handle<Self>, app: &mut App) {
        if !app.contains(self) || app.get(self).disposed || app.get(self).phase != Phase::Closing {
            return;
        }
        app.get_mut(self).phase = Phase::Closed;
        app.get_mut(self).placement_override = None;
        app.get_mut(self).position = None;
        if let Some(closed) = app.get(self).on_closed.clone() {
            closed.call(app);
        }
        if let Some(closed) = app.get(self).closed.clone() {
            closed.call(app);
        }
        if !app.contains(self) || app.get(self).disposed {
            return;
        }

        let scope = app.get(self).scope;
        let metadata = app.singleton::<FlyoutMetadata>();
        let staged = scope.and_then(|scope| {
            app.get_mut(metadata).open.remove(&scope);
            app.get_mut(metadata).staged.remove(&scope)
        });
        app.get_mut(self).target = None;
        app.get_mut(self).parent = None;
        if let Some((next, target, options)) = staged
            && app.contains(next)
            && !app.get(next).disposed
            && target.mounted(app)
        {
            next.show_at_with_options(app, target, options);
        }
    }

    /// Registers the source pointer-distance dismissal only while needed.
    fn update_pointer_route(self: Handle<Self>, app: &mut App) {
        self.remove_pointer_route(app);
        if self.is_open(app)
            && app.get(self).show_mode == FlyoutShowMode::TransientWithDismissOnPointerMoveAway
        {
            let route = PointerRoute::new(move |app, event| {
                if matches!(event, PointerEvent::Move(_) | PointerEvent::Hover(_)) {
                    self.pointer_moved(app, event.position());
                }
            });
            GestureBinding::instance(app)
                .pointer_router(app)
                .add_global_route(app, route.clone(), None);
            app.get_mut(self).pointer_route = Some(route);
        }
    }

    /// Removes an installed route before closing or disposal.
    fn remove_pointer_route(self: Handle<Self>, app: &mut App) {
        if let Some(route) = app.get_mut(self).pointer_route.take() {
            GestureBinding::instance(app)
                .pointer_router(app)
                .remove_global_route(app, &route);
        }
    }

    /// OnRootVisualPointerMoved uses squared distance to the presenter's rectangle.
    fn pointer_moved(self: Handle<Self>, app: &mut App, position: Offset) {
        let key = app.get(self).presenter_key.clone();
        let Some(context) = key.current_context(app) else {
            return;
        };
        let Some(render) = context
            .find_render_object(app)
            .and_then(|object| object.as_box())
        else {
            return;
        };
        let top_left = render.local_to_global(app, Offset::ZERO, None);
        let bounds = top_left & render.size(app);
        let x = (bounds.left - position.dx())
            .max(position.dx() - bounds.right)
            .max(0.0);
        let y = (bounds.top - position.dy())
            .max(position.dy() - bounds.bottom)
            .max(0.0);
        if x * x + y * y > 6400.0 {
            self.hide(app);
        }
    }

    /// Default FlyoutPresenter style, or the source-derived replacement presenter.
    fn build_presenter(self: Handle<Self>, _app: &mut App, _: BuildContext) -> WidgetRef {
        FlyoutScope {
            flyout: self,
            child: LayoutBuilder::new(move |app, context, constraints| {
                let host = app.get(self).host.unwrap();
                if let Some(bounds) = host.target_bounds(app) {
                    app.get_mut(self).last_target_bounds = bounds;
                }
                let r = ThemeResources::of(app, context);
                let flyout = app.get(self);
                let (style_minimum, style_maximum) = (flyout.presenter_sizes)(&r);
                let target = flyout.last_target_bounds;
                let submenu = flyout.is_sub_menu;
                let point_kind = flyout.point_kind;
                let position = flyout.position;
                let exclusion = flyout.exclusion;
                let placement = flyout.placement_override.unwrap_or(flyout.placement);
                let minimum = flyout.minimum_size.unwrap_or(style_minimum);
                let maximum = flyout.maximum_size.unwrap_or(style_maximum);
                let content = flyout.content.clone();
                let style = flyout.flyout_presenter_style.clone();
                let key: KeyRef = Rc::new(flyout.presenter_key.clone());
                let open = self.is_open(app);
                let padding = flyout.padding.unwrap_or(r.flyout_content_padding);
                let root = self.root(app);
                let group = app.get(root).tap_region_group;
                let grouped = group.is_some();
                let topmost = self.topmost(app);
                let standard = app.get(topmost).show_mode == FlyoutShowMode::Standard;
                let presenter = if let Some(style) = style {
                    style(app, context, content)
                } else {
                    FlyoutPresenter { content, padding }.into_widget()
                };
                let mut depth = 0;
                let mut ancestor = self;
                while app.get(ancestor).is_sub_menu {
                    let Some(parent) = app.get(ancestor).parent else {
                        break;
                    };
                    depth += 1;
                    ancestor = parent;
                }
                let presenter = CustomPaint::new()
                    .painter(shadow::PopupShadow::for_flyout(
                        &r,
                        ThemeResources::of(app, context).theme,
                        depth,
                    ))
                    .child(presenter);
                let foreground = ThemeResources::of(app, context).text_fill_color_primary;
                let presenter = DefaultTextStyle::new(
                    control_text_style(
                        &r,
                        r.control_content_theme_font_size,
                        FontWeight::NORMAL,
                        foreground,
                    ),
                    presenter,
                );
                let presenter = KeyedSubtree::new(presenter).key(key);
                let presenter = Focus::new(
                    inset_widgets::Listener::new()
                        .on_pointer_down(Rc::new(move |app, _| {
                            let topmost = self.topmost(app);
                            if app.get(topmost).show_mode != FlyoutShowMode::Standard {
                                self.close_children(app);
                            }
                        }))
                        .child(
                            TapRegion::new(presenter)
                                .group_id(group.unwrap_or(root.id()))
                                .consume_outside_taps(open && grouped)
                                .enabled(open)
                                .on_tap_outside(Rc::new(move |app, _| {
                                    if self != root || !self.is_open(app) {
                                        return;
                                    }
                                    let topmost = self.topmost(app);
                                    if grouped
                                        || app.get(topmost).show_mode != FlyoutShowMode::Standard
                                    {
                                        let metadata = app.singleton::<FlyoutMetadata>();
                                        if let Some(scope) = app.get(self).scope {
                                            app.get_mut(metadata).staged.remove(&scope);
                                        }
                                        self.hide(app);
                                    }
                                })),
                        ),
                )
                .can_request_focus(false);
                let insets = MediaQuery::view_insets_of(app, context);
                let area = Rect::from_ltwh(
                    0.0,
                    0.0,
                    constraints.max_width,
                    (constraints.max_height - insets.bottom).max(0.0),
                );
                let layout = FlyoutLayout {
                    settings: FlyoutLayoutSettings {
                        target,
                        position,
                        exclusion,
                        submenu,
                        point_kind,
                        placement,
                        direction: Directionality::of(app, context),
                        minimum,
                        maximum,
                        container: area,
                        allow_fallbacks: true,
                    },
                    child: presenter.into_widget(),
                }
                .into_widget();
                Stack::new()
                    .children([
                        if open && standard && !grouped && self == topmost {
                            ModalBarrier::new()
                                .on_dismiss(Listener::new(move |app| {
                                    self.hide(app);
                                }))
                                .into_widget()
                        } else {
                            SizedBox::shrink().into_widget()
                        },
                        layout,
                    ])
                    .into_widget()
            })
            .into_widget(),
        }
        .into_widget()
    }
}

/// Source Border > ScrollViewer > padded ContentPresenter, with both scroll axes enabled.
#[derive(Clone, Debug)]
struct FlyoutPresenter {
    /// Caller-supplied content.
    content: WidgetRef,

    /// Source content margins inside the scrolling area.
    padding: [f64; 4],
}

/// Owns the two native scroll controllers for the retained presenter.
struct FlyoutPresenterState {
    /// Native state identity.
    state: StateData<FlyoutPresenter>,

    /// Horizontal scrolling lifetime.
    horizontal: Option<Handle<ScrollViewportController>>,

    /// Vertical scrolling lifetime.
    vertical: Option<Handle<ScrollViewportController>>,
}

impl StatefulWidget for FlyoutPresenter {
    type State = FlyoutPresenterState;

    fn create_state(&self) -> Self::State {
        FlyoutPresenterState {
            state: StateData::new(),
            horizontal: None,
            vertical: None,
        }
    }
}

impl State for FlyoutPresenterState {
    type Widget = FlyoutPresenter;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).horizontal = Some(ScrollViewportController::new(app));
        app.get_mut(self).vertical = Some(ScrollViewportController::new(app));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).horizontal.unwrap().dispose(app);
        app.get(self).vertical.unwrap().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let horizontal = app.get(self).horizontal.unwrap();
        let vertical = app.get(self).vertical.unwrap();
        let widget = self.widget(app).clone();
        ControlBorder::new(
            Brush::Acrylic(resources.flyout_presenter_background),
            Brush::Solid(resources.flyout_border_theme_brush),
        )
        .border_thickness_ltrb(resources.flyout_border_theme_thickness)
        .corner_radius_corners(resources.overlay_corner_radius)
        .child(
            LayoutBuilder::new(move |_, _, constraints| {
                ScrollBarViewport::new(
                    vertical,
                    ScrollBarViewport::new(
                        horizontal,
                        ConstrainedBox::new(
                            BoxConstraints::new()
                                .min_width(constraints.min_width)
                                .min_height(constraints.min_height),
                        )
                        .child(
                            Padding::new(EdgeInsetsGeometry::from_ltrb(
                                widget.padding[0],
                                widget.padding[1],
                                widget.padding[2],
                                widget.padding[3],
                            ))
                            .child(widget.content.clone()),
                        ),
                    )
                    .axis(Axis::Horizontal),
                )
                .into_widget()
            })
            .into_widget(),
        )
        .into_widget()
    }
}
