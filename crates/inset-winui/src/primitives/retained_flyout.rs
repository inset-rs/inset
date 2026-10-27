//! Flyout content lifetime adapted to Flutter's retained OverlayEntry mechanism.

use crate::ThemeResources;
use crate::ThemeScope;
use inset_embedder::{Offset, Rect, TextDirection, ViewId};
use inset_foundation::{App, Handle, Listener};
use inset_painting::transform_rect;
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// Owns flyout content independently of any widget that opens it.
///
/// One root-overlay entry remains mounted between openings. Its child State is
/// disposed only when this host is disposed or the root overlay is removed.
pub struct RetainedFlyoutHost {
    /// Creates the content within the opening target's theme and direction.
    builder: WidgetBuilder,

    /// Root entry, created on the first opening.
    entry: Option<Handle<OverlayEntry>>,

    /// Overlay in which the retained entry lives.
    overlay: Option<Handle<OverlayState>>,

    /// Mounted state that owns the content focus scope.
    state: Option<Handle<RetainedFlyoutState>>,

    /// Current opening target, cleared when hidden or when that target unmounts.
    target: Option<Handle<FlyoutTargetState>>,

    /// Theme last supplied by the current target.
    resources: Option<Rc<ThemeResources>>,

    /// Text direction last supplied by the current target.
    direction: TextDirection,

    /// Whether the retained content is visible.
    open: bool,

    /// Whether opening requests keyboard focus for the content.
    take_focus: bool,

    /// Focus to restore when the flyout closes.
    previous_focus: Option<AnyFocusNode>,

    /// An explicitly focused menu header takes precedence over earlier queued focus.
    pub(crate) return_focus: Option<AnyFocusNode>,

    /// Prevents queued work from using a disposed host.
    disposed: bool,

    /// Coalesces content updates requested while another widget is building.
    rebuild_pending: bool,

    /// Invalidates focus requests queued before a newer open or close.
    epoch: u64,

    /// Presenter-loaded callback after the open frame and focus request.
    pub(crate) opened: Option<Listener>,

    /// Presenter-hidden callback after the close frame.
    pub(crate) closed: Option<Listener>,

    /// Notifies the owner when root removal disposes retained content.
    pub(crate) on_dispose: Option<Listener>,

    /// Optional owner close request when the active target is removed.
    pub(crate) target_removed: Option<Listener>,

    /// Popup key handling also applies when the scope itself has focus.
    pub(crate) on_key_event: Option<FocusOnKeyEventCallback>,
}

impl fmt::Debug for RetainedFlyoutHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetainedFlyoutHost")
            .field("open", &self.open)
            .field("disposed", &self.disposed)
            .finish_non_exhaustive()
    }
}

impl RetainedFlyoutHost {
    /// Creates a host; content is built lazily when first shown.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Handle<Self> {
        app.create(Self {
            builder,
            entry: None,
            overlay: None,
            state: None,
            target: None,
            resources: None,
            direction: TextDirection::Ltr,
            open: false,
            take_focus: true,
            previous_focus: None,
            return_focus: None,
            disposed: false,
            epoch: 0,
            rebuild_pending: false,
            opened: None,
            closed: None,
            target_removed: None,
            on_dispose: None,
            on_key_event: None,
        })
    }

    /// Whether the retained content is currently shown.
    pub fn is_open(self: Handle<Self>, app: &App) -> bool {
        app.get(self).open
    }

    /// Bounds of the current target in the root overlay's coordinate system.
    pub fn target_bounds(self: Handle<Self>, app: &App) -> Option<Rect> {
        let target = app.get(self).target?;
        if !app.contains(target) || !target.mounted(app) {
            return None;
        }
        let target_box = target.context(app).find_render_object(app)?.as_box()?;
        let overlay = app.get(self).overlay?;
        let root = overlay.context(app).find_render_object(app)?;
        let transform = target_box.as_object().get_transform_to(app, Some(root));
        Some(transform_rect(
            &transform,
            Offset::ZERO & target_box.size(app),
        ))
    }

    /// The view the content is shown in: its overlay's; none before the first opening.
    fn view(self: Handle<Self>, app: &App) -> Option<ViewId> {
        let overlay = app.get(self).overlay?;
        View::maybe_id_of(app, overlay.context(app))
    }

    /// Rebuilds retained content after a controller property changes.
    pub fn rebuild(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }
        if SchedulerBinding::scheduler_phase(app)
            == inset_scheduler::SchedulerPhase::PersistentCallbacks
        {
            if app.get(self).rebuild_pending {
                return;
            }
            app.get_mut(self).rebuild_pending = true;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) {
                        app.get_mut(self).rebuild_pending = false;
                        self.rebuild(app);
                    }
                }),
                self.view(app),
            );
        } else if let Some(state) = app.get(self).state {
            state.set_state(app, |_| {});
        }
    }

    /// Shows the same content from a context beneath a FlyoutTarget.
    pub fn show_at(self: Handle<Self>, app: &mut App, context: BuildContext, take_focus: bool) {
        assert!(
            !app.get(self).disposed,
            "cannot show a disposed flyout host"
        );
        let target = context
            .find_ancestor_state_of_type::<FlyoutTargetState>(app)
            .expect("show_at requires a context beneath FlyoutTarget");
        let overlay = Overlay::of(app, context, true);
        if let Some(previous_overlay) = app.get(self).overlay {
            assert_eq!(
                previous_overlay, overlay,
                "a retained flyout belongs to one root overlay"
            );
        }

        let resources = app
            .get(target)
            .resources
            .clone()
            .expect("a built target has a theme");
        let direction = app.get(target).direction;
        if let Some(previous_target) = app.get(self).target
            && previous_target != target
            && app.contains(previous_target)
        {
            app.get_mut(previous_target)
                .hosts
                .retain(|host| *host != self);
        }
        let previous_focus = app.get(self).return_focus.or_else(|| primary_focus(app));
        let already_owns_focus = app
            .get(self)
            .state
            .is_some_and(|state| app.get(state).focus.unwrap().as_node().has_focus(app));
        let host = app.get_mut(self);
        host.target = Some(target);
        host.resources = Some(resources);
        host.direction = direction;
        host.open = true;
        host.take_focus = take_focus;
        host.epoch += 1;
        if !already_owns_focus {
            host.previous_focus = previous_focus;
        }
        if !app.get(target).hosts.contains(&self) {
            app.get_mut(target).hosts.push(self);
        }

        if let Some(state) = app.get(self).state {
            let entry = app.get(self).entry.unwrap();
            // Keep this entry's key while moving all other entries below it.
            overlay.rearrange(app, vec![entry], Some(entry), None);
            state.set_state(app, |_| {});
            state.focus_after_frame(app);
        } else if app.get(self).entry.is_none() {
            let entry = OverlayEntry::new(
                app,
                Rc::new(move |_, _| RetainedFlyoutRoot { host: self }.into_widget()),
                false,
                true,
                false,
            );
            app.get_mut(self).entry = Some(entry);
            app.get_mut(self).overlay = Some(overlay);
            overlay.insert(app, entry, None, None);
        }
    }

    /// Hides the content without removing its entry or disposing child State.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed || !app.get(self).open {
            return;
        }
        if let Some(state) = app.get(self).state {
            state.restore_focus(app);
        }
        if let Some(target) = app.get(self).target
            && app.contains(target)
        {
            app.get_mut(target).hosts.retain(|host| *host != self);
        }
        let host = app.get_mut(self);
        host.open = false;
        host.target = None;
        host.previous_focus = None;
        host.epoch += 1;
        let epoch = host.epoch;
        self.rebuild(app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if app.contains(self)
                    && !app.get(self).disposed
                    && !app.get(self).open
                    && app.get(self).epoch == epoch
                    && let Some(closed) = app.get(self).closed.clone()
                {
                    closed.call(app);
                }
            }),
            self.view(app),
        );
    }

    /// Releases the overlay entry; its widget state disposes the focus scope on unmount.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }
        self.hide(app);
        app.get_mut(self).disposed = true;
        if let Some(entry) = app.get_mut(self).entry.take() {
            entry.remove(app);
            entry.dispose(app);
        }
        if let Some(disposed) = app.get(self).on_dispose.clone() {
            disposed.call(app);
        }
    }
}

/// Supplies an opening target's lifecycle, theme and direction to retained flyouts.
#[derive(Clone, Debug)]
pub struct FlyoutTarget {
    /// Target widget, including any Builder whose context is passed to show_at.
    pub child: WidgetRef,

    /// Identity of this target across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl FlyoutTarget {
    /// Wraps an opening target without taking ownership of its flyout content.
    pub fn new<K>(child: impl IntoWidget<K>) -> Self {
        Self {
            child: child.into_widget(),
            key: None,
        }
    }

    /// Sets the target's widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

/// Tracks only target lifetime and inherited settings, not flyout content State.
pub struct FlyoutTargetState {
    /// Native widget state.
    state: StateData<FlyoutTarget>,

    /// Hosts that have opened from this target.
    hosts: Vec<Handle<RetainedFlyoutHost>>,

    /// Theme obtained through this target's inherited dependencies.
    resources: Option<Rc<ThemeResources>>,

    /// Current inherited text direction.
    direction: TextDirection,
}

impl StatefulWidget for FlyoutTarget {
    type State = FlyoutTargetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        FlyoutTargetState {
            state: StateData::new(),
            hosts: Vec::new(),
            resources: None,
            direction: TextDirection::Ltr,
        }
    }
}

impl State for FlyoutTargetState {
    type Widget = FlyoutTarget;
    inset_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let resources = ThemeResources::of(app, self.context(app));
        let direction = Directionality::of(app, self.context(app));
        app.get_mut(self).resources = Some(resources.clone());
        app.get_mut(self).direction = direction;
        let hosts = app.get(self).hosts.clone();
        for host in hosts {
            if app.contains(host) && !app.get(host).disposed && app.get(host).target == Some(self) {
                app.get_mut(host).resources = Some(resources.clone());
                app.get_mut(host).direction = direction;
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _| {
                        if app.contains(host)
                            && !app.get(host).disposed
                            && let Some(state) = app.get(host).state
                        {
                            state.set_state(app, |_| {});
                        }
                    }),
                    View::maybe_id_of(app, self.context(app)),
                );
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let hosts = app.get(self).hosts.clone();
        for host in hosts {
            if app.contains(host) && !app.get(host).disposed && app.get(host).target == Some(self) {
                // Target removal may happen during build, so close after that pass finishes.
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _| {
                        if app.contains(host)
                            && !app.get(host).disposed
                            && app.get(host).target == Some(self)
                        {
                            if let Some(removed) = app.get(host).target_removed.clone() {
                                removed.call(app);
                            } else {
                                host.hide(app);
                            }
                        }
                    }),
                    // Disposed, the target is in no view any more.
                    None,
                );
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).child.clone()
    }
}

/// Root entry widget whose identity does not change when the opening target changes.
#[derive(Debug)]
struct RetainedFlyoutRoot {
    /// Application-owned host.
    host: Handle<RetainedFlyoutHost>,
}

/// Owns focus resources for the root entry, independently of target widget lifetime.
struct RetainedFlyoutState {
    /// Native widget state.
    state: StateData<RetainedFlyoutRoot>,

    /// Focus scope owned by this retained subtree.
    focus: Option<Handle<FocusScopeNode>>,
}

impl StatefulWidget for RetainedFlyoutRoot {
    type State = RetainedFlyoutState;

    fn create_state(&self) -> Self::State {
        RetainedFlyoutState {
            state: StateData::new(),
            focus: None,
        }
    }
}

impl RetainedFlyoutState {
    /// Requests content focus only if this opening is still current after layout.
    fn focus_after_frame(self: Handle<Self>, app: &mut App) {
        let host = self.widget(app).host;
        let epoch = app.get(host).epoch;
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self)
                    || !self.mounted(app)
                    || !app.contains(host)
                    || app.get(host).disposed
                    || !app.get(host).open
                    || app.get(host).epoch != epoch
                {
                    return;
                }
                if app.get(host).take_focus {
                    let scope = app.get(self).focus.unwrap().as_node();
                    let first = scope
                        .traversal_descendants(app)
                        .into_iter()
                        .find(|node| node.can_request_focus(app));
                    first.unwrap_or(scope).request_focus(app, None);
                }
                if let Some(opened) = app.get(host).opened.clone() {
                    opened.call(app);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Returns focus only if this popup still owns it and the previous node survives.
    fn restore_focus(self: Handle<Self>, app: &mut App) {
        let host = self.widget(app).host;
        let previous = app.get(host).previous_focus;
        if app.get(self).focus.unwrap().as_node().has_focus(app)
            && let Some(previous) = previous.filter(|node| {
                app.contains(node.id())
                    && !app.is_disposed(node.id())
                    && node.context(app).is_some()
                    && node.can_request_focus(app)
            })
        {
            previous.request_focus(app, None);
        }
    }
}

impl State for RetainedFlyoutState {
    type Widget = RetainedFlyoutRoot;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let host = self.widget(app).host;
        let focus = FocusScopeNode::new(app);
        focus.set_traversal_edge_behavior(app, TraversalEdgeBehavior::ClosedLoop);
        app.get_mut(self).focus = Some(focus);
        app.get_mut(host).state = Some(self);
        self.focus_after_frame(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let focus = app.get(self).focus.unwrap();
        focus.as_node().dispose(app);
        app.destroy(focus);
        let host = self.widget(app).host;
        if app.contains(host) {
            app.get_mut(host).state = None;
            // Root-overlay teardown releases the retained entry even when its application owner survives.
            host.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let host = self.widget(app).host;
        let data = app.get(host);
        let open = data.open;
        let resources = data
            .resources
            .clone()
            .expect("a shown flyout has a target theme");
        let direction = data.direction;
        let builder = data.builder.clone();
        let on_key_event = data.on_key_event.clone();
        let themed_content = ThemeScope::with_resources(
            resources,
            Directionality::new(
                direction,
                Builder::new(move |app, context| builder(app, context)),
            ),
        );
        let mut scope = FocusScope::new(themed_content).node(app.get(self).focus.unwrap());
        if let Some(handler) = on_key_event {
            scope = scope.on_key_event(handler);
        }
        Offstage::new()
            .offstage(!open)
            .child(TickerMode::new(
                open,
                ExcludeFocus::new(scope).excluding(!open),
            ))
            .into_widget()
    }
}
