//! Flutter counterpart: `widgets/automatic_keep_alive.dart`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};

use reveal_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener};
use reveal_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, Notification, ParentDataElement, State, StateData,
    StatefulWidget, StatelessWidget, WidgetRef, downcast_widget,
};
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::sliver::KeepAlive;

/// Allows subtrees to request to be kept alive in lazy lists.
///
/// This widget is like [`KeepAlive`] but instead of being explicitly configured,
/// it listens to [`KeepAliveNotification`] messages from the [`child`](Self::child) and other
/// descendants.
///
/// The subtree is kept alive whenever there is one or more descendant that has
/// sent a [`KeepAliveNotification`] and not yet triggered its
/// [`KeepAliveNotification::handle`].
///
/// To send these notifications, consider using [`AutomaticKeepAliveClientMixin`].
///
/// See also:
///
///  * [`AutomaticKeepAliveClientMixin`], which is a mixin with convenience
///    methods for clients of [`AutomaticKeepAlive`]. Used with
///    [`State`] subclasses.
///  * [`KeepAlive`] which marks a child as needing to stay alive even when it's
///    in a lazy list that would otherwise remove it.
#[derive(Debug)]
pub struct AutomaticKeepAlive {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl AutomaticKeepAlive {
    /// Creates a widget that listens to [`KeepAliveNotification`]s and maintains a
    /// [`KeepAlive`] widget appropriately.
    pub fn new<K>(child: impl IntoWidget<K>) -> AutomaticKeepAlive {
        AutomaticKeepAlive {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `AutomaticKeepAlive(key:)`.
    pub fn key(mut self, key: KeyRef) -> AutomaticKeepAlive {
        self.key = Some(key);
        self
    }
}

impl StatefulWidget for AutomaticKeepAlive {
    type State = AutomaticKeepAliveState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AutomaticKeepAliveState {
        AutomaticKeepAliveState {
            state: StateData::new(),
            handles: None,
            child: None,
            keeping_alive: false,
        }
    }
}

/// Dart's `_AutomaticKeepAliveState`.
pub struct AutomaticKeepAliveState {
    state: StateData<AutomaticKeepAlive>,
    handles: Option<HashMap<Handle<KeepAliveHandle>, Listener>>,
    // In order to apply parent data out of turn, the child of the KeepAlive
    // widget must be the same across frames.
    child: Option<WidgetRef>,
    keeping_alive: bool,
}

impl AutomaticKeepAliveState {
    fn update_child(self: Handle<Self>, app: &mut App) {
        let child = self.widget(app).child.clone();
        let this = self;
        app.get_mut(self).child = Some(
            NotificationListener::<KeepAliveNotification>::new(child)
                .on_notification(move |app, notification| this.add_client(app, notification))
                .into_widget(),
        );
    }

    fn add_client(self: Handle<Self>, app: &mut App, notification: &KeepAliveNotification) -> bool {
        let handle = notification.handle;
        let callback = self.create_callback(handle);
        {
            let handles = app.get_mut(self).handles.get_or_insert_with(HashMap::new);
            debug_assert!(
                !handles.contains_key(&handle),
                "the same handle cannot be used in two KeepAliveNotifications without triggering it first"
            );
            handles.insert(handle, callback.clone());
        }
        Listenable::add_listener(&handle, app, callback);
        if !app.get(self).keeping_alive {
            app.get_mut(self).keeping_alive = true;
            if let Some(child_element) = self.get_child_element(app) {
                // If the child already exists, update it synchronously.
                self.update_parent_data_of_child(app, child_element);
            } else {
                // If the child doesn't exist yet, we got called during the very first
                // build of this subtree. Wait until the end of the frame to update
                // the child when the child is guaranteed to be present.
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _time_stamp| {
                        if !self.mounted(app) {
                            return;
                        }
                        let child_element = self.get_child_element(app);
                        debug_assert!(child_element.is_some());
                        self.update_parent_data_of_child(
                            app,
                            child_element
                                .expect("the KeepAlive child exists after the first build"),
                        );
                    }),
                );
            }
        }
        false
    }

    /// Get the [`Element`] for the only [`KeepAlive`] child.
    ///
    /// While this widget is guaranteed to have a child, this may return null if
    /// the first build of that child has not completed yet.
    fn get_child_element(
        self: Handle<Self>,
        app: &App,
    ) -> Option<Handle<ParentDataElement<KeepAlive>>> {
        debug_assert!(self.mounted(app));
        let element = self.context(app);
        let mut child_element = None;
        // We use Element.visitChildren rather than context.visitChildElements
        // because we might be called during build, and context.visitChildElements
        // verifies that it is not called during build. Element.visitChildren does
        // not, instead it assumes that the caller will be careful. (See the
        // documentation for these methods for more details.)
        //
        // Here we know it's safe (with the exception outlined below) because we
        // just received a notification, which we wouldn't be able to do if we
        // hadn't built our child and its child -- our build method always builds
        // the same subtree and it always includes the node we're looking for
        // (KeepAlive) as the parent of the node that reports the notifications
        // (NotificationListener).
        //
        // If we are called during the first build of this subtree the links to the
        // children will not be hooked up yet. In that case this method returns
        // null despite the fact that we will have a child after the build
        // completes. It's the caller's responsibility to deal with this case.
        //
        // (We're only going down one level, to get our direct child.)
        element.visit_children(app, &mut |child| {
            child_element = Some(child);
        });
        let child_element = child_element?;
        debug_assert!(child_element.is_parent_data_element());
        Some(
            child_element
                .downcast::<ParentDataElement<KeepAlive>>(app)
                .expect("the only child is the KeepAlive ParentDataElement"),
        )
    }

    fn update_parent_data_of_child(
        self: Handle<Self>,
        app: &mut App,
        child_element: Handle<ParentDataElement<KeepAlive>>,
    ) {
        let context = self.context(app);
        let widget = self.build(app, context);
        let keep_alive =
            downcast_widget::<KeepAlive>(&*widget).expect("AutomaticKeepAlive builds a KeepAlive");
        child_element.apply_widget_out_of_turn(app, keep_alive);
    }

    fn create_callback(self: Handle<Self>, handle: Handle<KeepAliveHandle>) -> Listener {
        Listener::new(move |app| {
            if cfg!(debug_assertions) && !self.mounted(app) {
                panic!(
                    "AutomaticKeepAlive handle triggered after AutomaticKeepAlive was disposed.\n\
                     Widgets should always trigger their KeepAliveNotification handle when they are \
                     deactivated, so that they (or their handle) do not send spurious events later \
                     when they are no longer in the tree."
                );
            }
            let callback = app
                .get_mut(self)
                .handles
                .as_mut()
                .expect("a client was registered")
                .remove(&handle)
                .expect("the firing handle is still registered");
            Listenable::remove_listener(&handle, app, &callback);
            if app
                .get(self)
                .handles
                .as_ref()
                .expect("a client was registered")
                .is_empty()
            {
                if SchedulerBinding::scheduler_phase(app) < SchedulerPhase::PersistentCallbacks {
                    // Build/layout haven't started yet so let's just schedule this for
                    // the next frame.
                    self.set_state(app, |state| {
                        state.keeping_alive = false;
                    });
                } else {
                    // We were probably notified by a descendant when they were yanked out
                    // of our subtree somehow. We're probably in the middle of build or
                    // layout, so there's really nothing we can do to clean up this mess
                    // short of just scheduling another build to do the cleanup. This is
                    // very unfortunate, and means (for instance) that garbage collection
                    // of these resources won't happen for another 16ms.
                    //
                    // The problem is there's really no way for us to distinguish these
                    // cases:
                    //
                    //  * We haven't built yet (or missed out chance to build), but
                    //    someone above us notified our descendant and our descendant is
                    //    disconnecting from us. If we could mark ourselves dirty we would
                    //    be able to clean everything this frame. (This is a pretty
                    //    unlikely scenario in practice. Usually things change before
                    //    build/layout, not during build/layout.)
                    //
                    //  * Our child changed, and as our old child went away, it notified
                    //    us. We can't setState, since we _just_ built. We can't apply the
                    //    parent data information to our child because we don't _have_ a
                    //    child at this instant. We really want to be able to change our
                    //    mind about how we built, so we can give the KeepAlive widget a
                    //    new value, but it's too late.
                    //
                    //  * A deep descendant in another build scope just got yanked, and in
                    //    the process notified us. We could apply new parent data
                    //    information, but it may or may not get applied this frame,
                    //    depending on whether said child is in the same layout scope.
                    //
                    //  * A descendant is being moved from one position under us to
                    //    another position under us. They just notified us of the removal,
                    //    at some point in the future they will notify us of the addition.
                    //    We don't want to do anything. (This is why we check that
                    //    _handles is still empty below.)
                    //
                    //  * We're being notified in the paint phase, or even in a post-frame
                    //    callback. Either way it is far too late for us to make our
                    //    parent lay out again this frame, so the garbage won't get
                    //    collected this frame.
                    //
                    //  * We are being torn out of the tree ourselves, as is our
                    //    descendant, and it notified us while it was being deactivated.
                    //    We don't need to do anything, but we don't know yet because we
                    //    haven't been deactivated yet. (This is why we check mounted
                    //    below before calling setState.)
                    //
                    // Long story short, we have to schedule a new frame and request a
                    // frame there, but this is generally a bad practice, and you should
                    // avoid it if possible.
                    app.get_mut(self).keeping_alive = false;
                    app.schedule_microtask(Listener::new(move |app| {
                        if self.mounted(app)
                            && app
                                .get(self)
                                .handles
                                .as_ref()
                                .expect("a client was registered")
                                .is_empty()
                        {
                            // If mounted is false, we went away as well, so there's nothing to do.
                            // If _handles is no longer empty, then another client (or the same
                            // client in a new place) registered itself before we had a chance to
                            // turn off keepalive, so again there's nothing to do.
                            self.set_state(app, |state| {
                                debug_assert!(!state.keeping_alive);
                            });
                        }
                    }));
                }
            }
        })
    }
}

impl State for AutomaticKeepAliveState {
    type Widget = AutomaticKeepAlive;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_child(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &AutomaticKeepAlive) {
        self.update_child(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(handles) = app.get_mut(self).handles.take() {
            for (handle, callback) in handles {
                Listenable::remove_listener(&handle, app, &callback);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let keeping_alive = app.get(self).keeping_alive;
        let child = app
            .get(self)
            .child
            .clone()
            .expect("the child is created in init_state");
        KeepAlive::new(keeping_alive, child).into_widget()
    }
}

/// Indicates that the subtree through which this notification bubbles must be
/// kept alive even if it would normally be discarded as an optimization.
///
/// For example, a focused text field might fire this notification to indicate
/// that it should not be disposed even if the user scrolls the field off
/// screen.
///
/// Each [`KeepAliveNotification`] is configured with a [`handle`](Self::handle) that consists of
/// a [`Listenable`] that is triggered when the subtree no longer needs to be kept
/// alive.
///
/// The [`handle`](Self::handle) should be triggered any time the sending widget is removed from
/// the tree (in [`State::deactivate`]). If the widget is then rebuilt and still
/// needs to be kept alive, it should immediately send a new notification
/// (possible with the very same [`Listenable`]) during build.
///
/// This notification is listened to by the [`AutomaticKeepAlive`] widget, which
/// is added to the tree automatically by `SliverList` (and `ListView`) and
/// `SliverGrid` (and `GridView`) widgets.
///
/// Failure to trigger the [`handle`](Self::handle) in the manner described above will likely
/// cause the [`AutomaticKeepAlive`] to lose track of whether the widget should be
/// kept alive or not, leading to memory leaks or lost data. For example, if the
/// widget that requested keepalive is removed from the subtree but doesn't
/// trigger its [`Listenable`] on the way out, then the subtree will continue to
/// be kept alive until the list itself is disposed. Similarly, if the
/// [`Listenable`] is triggered while the widget needs to be kept alive, but a new
/// [`KeepAliveNotification`] is not immediately sent, then the widget risks being
/// garbage collected while it wants to be kept alive.
///
/// It is an error to use the same [`handle`](Self::handle) in two [`KeepAliveNotification`]s
/// within the same [`AutomaticKeepAlive`] without triggering that [`handle`](Self::handle) before
/// the second notification is sent.
///
/// For a more convenient way to interact with [`AutomaticKeepAlive`] widgets,
/// consider using [`AutomaticKeepAliveClientMixin`], which uses
/// [`KeepAliveNotification`] internally.
#[derive(Debug)]
pub struct KeepAliveNotification {
    /// A [`Listenable`] that will inform its clients when the widget that fired the
    /// notification no longer needs to be kept alive.
    ///
    /// [`Handle<KeepAliveHandle>`](KeepAliveHandle) is that listenable. The
    /// [`Listenable`] should be triggered any time the sending widget is
    /// removed from the tree (in [`State::deactivate`]). If the widget is then
    /// rebuilt and still needs to be kept alive, it should immediately send a new
    /// notification (possible with the very same [`Listenable`]) during build.
    ///
    /// See also:
    ///
    ///  * [`KeepAliveHandle`], a convenience class for use with this property.
    pub handle: Handle<KeepAliveHandle>,
}

impl KeepAliveNotification {
    /// Creates a notification to indicate that a subtree must be kept alive.
    pub fn new(handle: Handle<KeepAliveHandle>) -> KeepAliveNotification {
        KeepAliveNotification { handle }
    }
}

impl Notification for KeepAliveNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A [`Listenable`] which can be manually triggered.
///
/// Used with [`KeepAliveNotification`] objects as their
/// [`KeepAliveNotification::handle`].
///
/// For a more convenient way to interact with [`AutomaticKeepAlive`] widgets,
/// consider using [`AutomaticKeepAliveClientMixin`], which uses a
/// [`KeepAliveHandle`] internally.
pub struct KeepAliveHandle {
    change_notifier: ChangeNotifierData,
}

impl KeepAliveHandle {
    /// A handle that has not been triggered.
    pub fn new(app: &mut App) -> Handle<KeepAliveHandle> {
        app.create(KeepAliveHandle {
            change_notifier: ChangeNotifierData::new(),
        })
    }

    /// Dart's `KeepAliveHandle.dispose`: notifies, then disposes the notifier.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
        app.get_mut(self).change_notifier.dispose();
        app.destroy(self);
    }
}

impl ChangeNotifier for KeepAliveHandle {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for KeepAliveHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeepAliveHandle").finish_non_exhaustive()
    }
}

/// The fields of Dart's `AutomaticKeepAliveClientMixin`.
#[derive(Default)]
pub struct AutomaticKeepAliveClientMixinData {
    /// Dart's `_keepAliveHandle`.
    pub keep_alive_handle: Option<Handle<KeepAliveHandle>>,
}

impl AutomaticKeepAliveClientMixinData {
    /// No keep-alive request has been sent.
    pub fn new() -> AutomaticKeepAliveClientMixinData {
        AutomaticKeepAliveClientMixinData::default()
    }
}

/// A mixin with convenience methods for clients of [`AutomaticKeepAlive`]. It is used
/// with [`State`] subclasses to manage keep-alive behavior in lazily built lists.
///
/// This mixin simplifies interaction with [`AutomaticKeepAlive`] by automatically
/// sending [`KeepAliveNotification`]s when necessary. Subclasses must implement
/// [`want_keep_alive`](Self::want_keep_alive) to indicate whether the widget should be
/// kept alive and call [`update_keep_alive`](Self::update_keep_alive) whenever its value
/// changes.
///
/// The mixin internally manages a [`KeepAliveHandle`], which is used to notify
/// the nearest [`AutomaticKeepAlive`] ancestor of changes in keep-alive
/// requirements. [`AutomaticKeepAlive`] listens for [`KeepAliveNotification`]s sent
/// by this mixin and dynamically wraps the subtree in a [`KeepAlive`] widget to
/// preserve its state when it is no longer visible in the viewport.
///
/// Subclasses must implement [`want_keep_alive`](Self::want_keep_alive), and their
/// [`build`](Self::build) methods must call [`AutomaticKeepAliveClientMixin::build`]
/// (though the return value should be ignored).
///
/// Then, whenever [`want_keep_alive`](Self::want_keep_alive)'s value changes (or might
/// change), the subclass should call [`update_keep_alive`](Self::update_keep_alive).
///
/// The state holds an [`AutomaticKeepAliveClientMixinData`] under the field
/// `automatic_keep_alive_client` and implements the accessor pair. Rust cannot
/// interpose a mixin on [`State`]'s hooks, so the state's own [`State::init_state`],
/// [`State::deactivate`] and [`State::build`] call the counterparts here, where
/// Dart's mixin body would run.
///
/// See also:
///
///  * [`AutomaticKeepAlive`], which listens to messages from this mixin.
///  * [`KeepAliveNotification`], the notifications sent by this mixin.
///  * [`KeepAlive`] which marks a child as needing to stay alive even when it's
///    in a lazy list that would otherwise remove it.
pub trait AutomaticKeepAliveClientMixin: State {
    /// Mixin field access.
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData;

    /// See [`automatic_keep_alive_client_data`](Self::automatic_keep_alive_client_data).
    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData;

    /// Whether the current instance should be kept alive.
    ///
    /// Call [`update_keep_alive`](Self::update_keep_alive) whenever this getter's value changes.
    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool;

    /// Ensures that any [`AutomaticKeepAlive`] ancestors are in a good state, by
    /// firing a [`KeepAliveNotification`] or triggering the [`KeepAliveHandle`] as
    /// appropriate.
    fn update_keep_alive(self: Handle<Self>, app: &mut App) {
        if self.want_keep_alive(app) {
            if self
                .automatic_keep_alive_client_data(app)
                .keep_alive_handle
                .is_none()
            {
                self.ensure_keep_alive(app);
            }
        } else if self
            .automatic_keep_alive_client_data(app)
            .keep_alive_handle
            .is_some()
        {
            self.release_keep_alive(app);
        }
    }

    /// The mixin's part of [`State::init_state`]; the state calls it where Dart's
    /// `super.initState()` would reach the mixin.
    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.want_keep_alive(app) {
            self.ensure_keep_alive(app);
        }
    }

    /// The mixin's part of [`State::deactivate`]; the state calls it where Dart's
    /// `super.deactivate()` would reach the mixin.
    fn deactivate(self: Handle<Self>, app: &mut App) {
        if self
            .automatic_keep_alive_client_data(app)
            .keep_alive_handle
            .is_some()
        {
            self.release_keep_alive(app);
        }
    }

    /// The mixin's part of [`State::build`]; the state calls it where Dart's
    /// `super.build` would reach the mixin. The return value must be ignored.
    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        if self.want_keep_alive(app)
            && self
                .automatic_keep_alive_client_data(app)
                .keep_alive_handle
                .is_none()
        {
            self.ensure_keep_alive(app);
            // Whenever wantKeepAlive's value changes (or might change), the
            // subclass should call [updateKeepAlive].
            // That will ensure that the keepalive is disabled (or enabled)
            // without requiring a rebuild.
        }
        NullWidget.into_widget()
    }
}

/// Dart's private members of `AutomaticKeepAliveClientMixin`.
trait AutomaticKeepAliveClientMixinPrivate: AutomaticKeepAliveClientMixin {
    fn ensure_keep_alive(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            self.automatic_keep_alive_client_data(app)
                .keep_alive_handle
                .is_none()
        );
        let handle = KeepAliveHandle::new(app);
        self.automatic_keep_alive_client_data_mut(app)
            .keep_alive_handle = Some(handle);
        KeepAliveNotification::new(handle).dispatch(app, Some(self.context(app)));
    }

    fn release_keep_alive(self: Handle<Self>, app: &mut App) {
        let handle = self
            .automatic_keep_alive_client_data_mut(app)
            .keep_alive_handle
            .take()
            .expect("a keep-alive handle is registered");
        handle.dispose(app);
    }
}

impl<S: AutomaticKeepAliveClientMixin> AutomaticKeepAliveClientMixinPrivate for S {}

/// Dart's `_NullWidget`.
#[derive(Debug)]
struct NullWidget;

impl StatelessWidget for NullWidget {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        panic!(
            "Widgets that mix AutomaticKeepAliveClientMixin into their State must \
             call super.build() but must ignore the return value of the superclass."
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_foundation::{AppCell, ValueKey};

    use super::*;
    use crate::framework::{Element, KeyRef};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[derive(Debug)]
    struct KeepClient {
        want: bool,
    }

    impl StatefulWidget for KeepClient {
        type State = KeepClientState;

        fn create_state(&self) -> KeepClientState {
            KeepClientState {
                state: StateData::new(),
                automatic_keep_alive_client: AutomaticKeepAliveClientMixinData::new(),
            }
        }
    }

    struct KeepClientState {
        state: StateData<KeepClient>,
        automatic_keep_alive_client: AutomaticKeepAliveClientMixinData,
    }

    impl AutomaticKeepAliveClientMixin for KeepClientState {
        fn automatic_keep_alive_client_data(
            self: Handle<Self>,
            app: &App,
        ) -> &AutomaticKeepAliveClientMixinData {
            &app.get(self).automatic_keep_alive_client
        }

        fn automatic_keep_alive_client_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AutomaticKeepAliveClientMixinData {
            &mut app.get_mut(self).automatic_keep_alive_client
        }

        fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
            self.widget(app).want
        }
    }

    impl State for KeepClientState {
        type Widget = KeepClient;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            AutomaticKeepAliveClientMixin::init_state(self, app);
        }

        fn deactivate(self: Handle<Self>, app: &mut App) {
            AutomaticKeepAliveClientMixin::deactivate(self, app);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let _ = AutomaticKeepAliveClientMixin::build(self, app);
            SizedBox::shrink().into_widget()
        }
    }

    fn host_state(harness: &Harness, app: &App) -> Handle<KeepClientState> {
        let mut element = harness.root.as_element();
        loop {
            if let Some(state) = element.state_handle::<KeepClientState>(app) {
                return state;
            }
            element = element.children(app)[0];
        }
    }

    #[test]
    fn mixin_want_keep_alive_true_dispatches() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, KeepClient { want: true }.into_widget());
        harness.pump(&mut app);
        let state = host_state(&harness, &app);
        assert!(
            app.get(state)
                .automatic_keep_alive_client
                .keep_alive_handle
                .is_some()
        );
        AutomaticKeepAliveClientMixin::update_keep_alive(state, &mut app);
        assert!(
            app.get(state)
                .automatic_keep_alive_client
                .keep_alive_handle
                .is_some()
        );
        let _ =
            AutomaticKeepAlive::new(SizedBox::shrink()).key(Rc::new(ValueKey::new(1)) as KeyRef);
    }

    #[test]
    fn keep_alive_handle_dispose_notifies() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let handle = KeepAliveHandle::new(&mut app);
        let notified = Rc::new(Cell::new(false));
        Listenable::add_listener(
            &handle,
            &mut app,
            Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| notified.set(true)
            }),
        );
        handle.dispose(&mut app);
        assert!(notified.get());
    }
}
