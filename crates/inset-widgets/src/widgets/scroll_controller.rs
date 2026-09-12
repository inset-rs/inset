//! Flutter counterpart: `widgets/scroll_controller.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use inset_animation::Curve;
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, HandleId, Listenable,
    ListenableObject, Listener, wait_all,
};

use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_position::{AnyScrollPosition, ScrollPosition};
use crate::widgets::scroll_position_with_single_context::ScrollPositionWithSingleContext;

/// Signature for when a [`ScrollController`] has added or removed a
/// [`AnyScrollPosition`].
///
/// Since a scroll position is not created and attached to a controller until
/// the `Scrollable` is built, this can be used to respond to the position being
/// attached to a controller.
///
/// By having access to the position directly, additional listeners can be
/// applied to aspects of the scroll position, like
/// [`ScrollPosition::is_scrolling_notifier`].
///
/// Used by [`ScrollController::on_attach`] and [`ScrollController::on_detach`].
pub type ScrollControllerCallback = Rc<dyn Fn(&mut App, AnyScrollPosition)>;

/// The fields of Dart's `ScrollController`, held under the field `scroll_controller`.
pub struct ScrollControllerData {
    initial_scroll_offset: f64,
    keep_scroll_offset: bool,
    on_attach: Option<ScrollControllerCallback>,
    on_detach: Option<ScrollControllerCallback>,
    debug_label: Option<String>,
    positions: Vec<AnyScrollPosition>,
}

impl ScrollControllerData {
    /// The bag of a controller with no attached positions.
    pub fn new(
        initial_scroll_offset: f64,
        keep_scroll_offset: bool,
        debug_label: Option<String>,
        on_attach: Option<ScrollControllerCallback>,
        on_detach: Option<ScrollControllerCallback>,
    ) -> ScrollControllerData {
        ScrollControllerData {
            initial_scroll_offset,
            keep_scroll_offset,
            on_attach,
            on_detach,
            debug_label,
            positions: Vec::new(),
        }
    }
}

/// The accessors [`ScrollControllerLeaf`] asks for, for a struct whose bag is the field
/// `scroll_controller`.
#[macro_export]
macro_rules! scroll_controller_accessors {
    () => {
        fn scroll_controller_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::ScrollControllerData {
            &app.get(self).scroll_controller
        }

        fn scroll_controller_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::ScrollControllerData {
            &mut app.get_mut(self).scroll_controller
        }
    };
}

/// The trait both scroll-controller leaves implement: [`ScrollController`] and
/// [`TrackingScrollController`].
///
/// The virtuals a subclass overrides are the members with a default body here; the shared
/// bodies are associated functions on [`ScrollController`], which an override calls where
/// Dart writes `super.…`.
pub trait ScrollControllerLeaf: ChangeNotifier + Sized + 'static {
    /// Dart's `ScrollController` fields, held under the field `scroll_controller`
    /// ([`scroll_controller_accessors!`](crate::scroll_controller_accessors)).
    fn scroll_controller_data(self: Handle<Self>, app: &App) -> &ScrollControllerData;

    /// See [`scroll_controller_data`](Self::scroll_controller_data).
    fn scroll_controller_data_mut(self: Handle<Self>, app: &mut App) -> &mut ScrollControllerData;

    /// The initial value to use for [`offset`](Self::offset).
    ///
    /// New [`ScrollPosition`] objects that are created and attached to this
    /// controller will have their offset initialized to this value
    /// if [`keep_scroll_offset`](Self::keep_scroll_offset) is false or a scroll offset hasn't
    /// been saved yet.
    ///
    /// Defaults to 0.0.
    fn initial_scroll_offset(self: Handle<Self>, app: &App) -> f64 {
        ScrollController::initial_scroll_offset(self.as_controller(), app)
    }

    /// Register the given position with this controller.
    ///
    /// After this function returns, the [`animate_to`](Self::animate_to) and
    /// [`jump_to`](Self::jump_to) methods on this controller will manipulate the given
    /// position.
    fn attach(self: Handle<Self>, app: &mut App, position: AnyScrollPosition) {
        ScrollController::attach(self.as_controller(), app, position);
    }

    /// Unregister the given position with this controller.
    ///
    /// After this function returns, the [`animate_to`](Self::animate_to) and
    /// [`jump_to`](Self::jump_to) methods on this controller will not manipulate the given
    /// position.
    fn detach(self: Handle<Self>, app: &mut App, position: AnyScrollPosition) {
        ScrollController::detach(self.as_controller(), app, position);
    }

    /// Discards any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App) {
        ScrollController::dispose(self.as_controller(), app);
    }

    /// Creates a [`ScrollPosition`] for use by a `Scrollable` widget.
    ///
    /// A leaf can override this function to customize the [`ScrollPosition`]
    /// used by the scrollable widgets it controls. For example, `PageController`
    /// overrides this function to return a page-oriented scroll position
    /// that keeps the same page visible when the scrollable widget resizes.
    ///
    /// By default, returns a [`ScrollPositionWithSingleContext`].
    ///
    /// The arguments are generally passed to the [`ScrollPosition`] being created:
    ///
    ///  * `physics`: the [`crate::ScrollPhysics`] that determines how the
    ///    [`ScrollPosition`] should react to user interactions, how it should
    ///    simulate scrolling when released or flung, etc. It typically comes from the
    ///    `ScrollView` or other widget that creates the `Scrollable`, or, if none was
    ///    provided, from the ambient `ScrollConfiguration`.
    ///  * `context`: a [`ScrollContext`] used for communicating with the object
    ///    that is to own the [`ScrollPosition`] (typically, this is the `Scrollable`
    ///    itself).
    ///  * `old_position`: if this is not the first time a [`ScrollPosition`] has
    ///    been created for this `Scrollable`, this will be the previous instance.
    ///    This is used when the environment has changed and the `Scrollable`
    ///    needs to recreate the [`ScrollPosition`] object. It is `None` the first
    ///    time the [`ScrollPosition`] is created.
    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> AnyScrollPosition {
        ScrollController::create_scroll_position(
            self.as_controller(),
            app,
            physics,
            context,
            old_position,
        )
    }

    /// Add additional information to the given `description` for use by
    /// [`describe`](Self::describe).
    ///
    /// An override starts with a call to [`ScrollController::debug_fill_description`], where
    /// Dart writes `super.debugFillDescription(description)`.
    fn debug_fill_description(self: Handle<Self>, app: &App, description: &mut Vec<String>) {
        ScrollController::debug_fill_description(self.as_controller(), app, description);
    }

    /// Dart's `notifyListeners` tear-off: the listener this controller keeps on each attached
    /// position.
    fn notification_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::forward_notification)
    }

    /// The body behind [`notification_listener`](Self::notification_listener).
    fn forward_notification(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }

    // ---- one-line forwarders to the type-erased handle ----

    /// See [`AnyScrollController::keep_scroll_offset`].
    fn keep_scroll_offset(self: Handle<Self>, app: &App) -> bool {
        self.as_controller().keep_scroll_offset(app)
    }

    /// See [`AnyScrollController::on_attach`].
    fn on_attach(self: Handle<Self>, app: &App) -> Option<ScrollControllerCallback> {
        self.as_controller().on_attach(app)
    }

    /// See [`AnyScrollController::on_detach`].
    fn on_detach(self: Handle<Self>, app: &App) -> Option<ScrollControllerCallback> {
        self.as_controller().on_detach(app)
    }

    /// See [`AnyScrollController::debug_label`].
    fn debug_label(self: Handle<Self>, app: &App) -> Option<String> {
        self.as_controller().debug_label(app)
    }

    /// See [`AnyScrollController::positions`].
    fn positions(self: Handle<Self>, app: &App) -> Vec<AnyScrollPosition> {
        self.as_controller().positions(app)
    }

    /// See [`AnyScrollController::has_clients`].
    fn has_clients(self: Handle<Self>, app: &App) -> bool {
        self.as_controller().has_clients(app)
    }

    /// See [`AnyScrollController::position`].
    fn position(self: Handle<Self>, app: &App) -> AnyScrollPosition {
        self.as_controller().position(app)
    }

    /// See [`AnyScrollController::offset`].
    fn offset(self: Handle<Self>, app: &App) -> f64 {
        self.as_controller().offset(app)
    }

    /// See [`AnyScrollController::animate_to`].
    fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        offset: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        self.as_controller()
            .animate_to(app, offset, duration, curve)
    }

    /// See [`AnyScrollController::jump_to`].
    fn jump_to(self: Handle<Self>, app: &mut App, value: f64) {
        self.as_controller().jump_to(app, value);
    }

    /// See [`AnyScrollController::describe`].
    fn describe(self: Handle<Self>, app: &App) -> String {
        self.as_controller().describe(app)
    }

    /// This controller as the erased [`AnyScrollController`] — what to pass where a Dart API
    /// takes a `ScrollController`.
    fn as_controller(self: Handle<Self>) -> AnyScrollController {
        AnyScrollController {
            id: self.id(),
            vtable: const { &ScrollControllerVTable::of::<Self>() },
        }
    }
}

/// Controls a scrollable widget.
///
/// Scroll controllers are typically stored as member variables in `State`
/// objects and are reused in each `State::build`. A single scroll controller can
/// be used to control multiple scrollable widgets, but some operations, such
/// as reading the scroll [`offset`](ScrollControllerLeaf::offset), require the controller to
/// be used with a single scrollable widget.
///
/// A scroll controller creates a [`ScrollPosition`] to manage the state specific
/// to an individual `Scrollable` widget. To use a custom [`ScrollPosition`],
/// write a second [`ScrollControllerLeaf`] that overrides
/// [`create_scroll_position`](ScrollControllerLeaf::create_scroll_position).
///
/// Typically used with `ListView`, `GridView`, `CustomScrollView`.
///
/// The associated functions on this type are the shared bodies of Dart's `ScrollController`;
/// a leaf calls one where Dart writes `super.…`.
///
/// See also:
///
///  * `Scrollable`, which is the lower-level widget that creates and associates
///    [`ScrollPosition`] objects with [`ScrollController`] objects.
///  * [`ScrollPosition`], which manages the scroll offset for an individual
///    scrolling widget.
///  * [`crate::ScrollNotification`] and `NotificationListener`, which can be used to
///    listen to scrolling occur without using a [`ScrollController`].
pub struct ScrollController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
}

impl ScrollController {
    /// Creates a controller for a scrollable widget.
    ///
    /// Dart's defaults: `initial_scroll_offset` 0.0, `keep_scroll_offset` true, and no
    /// `debug_label`, `on_attach` or `on_detach`.
    pub fn new(
        app: &mut App,
        initial_scroll_offset: f64,
        keep_scroll_offset: bool,
        debug_label: Option<String>,
        on_attach: Option<ScrollControllerCallback>,
        on_detach: Option<ScrollControllerCallback>,
    ) -> Handle<ScrollController> {
        app.create(ScrollController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(
                initial_scroll_offset,
                keep_scroll_offset,
                debug_label,
                on_attach,
                on_detach,
            ),
        })
    }

    /// A controller with all of Dart's defaults; `ScrollController()`.
    pub fn default(app: &mut App) -> Handle<ScrollController> {
        ScrollController::new(app, 0.0, true, None, None, None)
    }

    /// See [`ScrollControllerLeaf::initial_scroll_offset`].
    pub fn initial_scroll_offset(controller: AnyScrollController, app: &App) -> f64 {
        controller.data(app).initial_scroll_offset
    }

    /// See [`ScrollControllerLeaf::attach`].
    pub fn attach(controller: AnyScrollController, app: &mut App, position: AnyScrollPosition) {
        debug_assert!(!controller.data(app).positions.contains(&position));
        controller.data_mut(app).positions.push(position);
        position.add_listener(app, controller.notification_listener());
        if let Some(on_attach) = controller.on_attach(app) {
            on_attach(app, position);
        }
    }

    /// See [`ScrollControllerLeaf::detach`].
    pub fn detach(controller: AnyScrollController, app: &mut App, position: AnyScrollPosition) {
        debug_assert!(controller.data(app).positions.contains(&position));
        if let Some(on_detach) = controller.on_detach(app) {
            on_detach(app, position);
        }
        position.remove_listener(app, &controller.notification_listener());
        controller
            .data_mut(app)
            .positions
            .retain(|attached| *attached != position);
    }

    /// See [`ScrollControllerLeaf::dispose`].
    pub fn dispose(controller: AnyScrollController, app: &mut App) {
        let listener = controller.notification_listener();
        for position in controller.positions(app) {
            position.remove_listener(app, &listener);
        }
        controller.dispose_change_notifier(app);
    }

    /// See [`ScrollControllerLeaf::create_scroll_position`].
    pub fn create_scroll_position(
        controller: AnyScrollController,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> AnyScrollPosition {
        let initial_pixels = controller.initial_scroll_offset(app);
        let keep_scroll_offset = controller.keep_scroll_offset(app);
        let debug_label = controller.debug_label(app);
        ScrollPositionWithSingleContext::new(
            app,
            physics,
            context,
            Some(initial_pixels),
            keep_scroll_offset,
            old_position,
            debug_label,
        )
        .as_scroll_position()
    }

    /// See [`ScrollControllerLeaf::debug_fill_description`].
    pub fn debug_fill_description(
        controller: AnyScrollController,
        app: &App,
        description: &mut Vec<String>,
    ) {
        if let Some(debug_label) = controller.debug_label(app) {
            description.push(debug_label);
        }
        let initial_scroll_offset = controller.initial_scroll_offset(app);
        if initial_scroll_offset != 0.0 {
            description.push(format!("initialScrollOffset: {initial_scroll_offset:.1}, "));
        }
        let positions = controller.positions(app);
        match positions.len() {
            0 => description.push("no clients".to_string()),
            // Don't actually list the client itself, since its description may refer to us.
            1 => description.push(format!("one client, offset {:.1}", controller.offset(app))),
            count => description.push(format!("{count} clients")),
        }
    }
}

impl ChangeNotifier for ScrollController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for ScrollController {
    crate::scroll_controller_accessors!();
}

/// A [`ScrollController`] whose
/// [`initial_scroll_offset`](ScrollControllerLeaf::initial_scroll_offset) tracks its most
/// recently updated [`ScrollPosition`].
///
/// This type can be used to synchronize the scroll offset of two or more
/// lazily created scroll views that share a single [`TrackingScrollController`].
/// It tracks the most recently updated scroll position and reports it as its
/// initial scroll offset.
pub struct TrackingScrollController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    position_to_listener: IndexMap<AnyScrollPosition, Listener>,
    last_updated: Option<AnyScrollPosition>,
    last_updated_offset: Option<f64>,
}

impl TrackingScrollController {
    /// Creates a scroll controller that continually updates its
    /// [`initial_scroll_offset`](ScrollControllerLeaf::initial_scroll_offset) to match the
    /// last scroll notification it received.
    pub fn new(
        app: &mut App,
        initial_scroll_offset: f64,
        keep_scroll_offset: bool,
        debug_label: Option<String>,
        on_attach: Option<ScrollControllerCallback>,
        on_detach: Option<ScrollControllerCallback>,
    ) -> Handle<TrackingScrollController> {
        app.create(TrackingScrollController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(
                initial_scroll_offset,
                keep_scroll_offset,
                debug_label,
                on_attach,
                on_detach,
            ),
            position_to_listener: IndexMap::new(),
            last_updated: None,
            last_updated_offset: None,
        })
    }

    /// A tracking controller with all of Dart's defaults.
    pub fn default(app: &mut App) -> Handle<TrackingScrollController> {
        TrackingScrollController::new(app, 0.0, true, None, None, None)
    }

    /// The last [`ScrollPosition`] to change. Returns `None` if there aren't any
    /// attached scroll positions, or there hasn't been any scrolling yet, or the
    /// last position to change has since been removed.
    pub fn most_recently_updated_position(
        self: Handle<Self>,
        app: &App,
    ) -> Option<AnyScrollPosition> {
        app.get(self).last_updated
    }
}

impl ChangeNotifier for TrackingScrollController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for TrackingScrollController {
    crate::scroll_controller_accessors!();

    /// Returns the scroll offset of the
    /// [`most_recently_updated_position`](TrackingScrollController::most_recently_updated_position)
    /// or, if that is `None`, the initial scroll offset provided to the constructor.
    fn initial_scroll_offset(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .last_updated_offset
            .unwrap_or_else(|| ScrollController::initial_scroll_offset(self.as_controller(), app))
    }

    fn attach(self: Handle<Self>, app: &mut App, position: AnyScrollPosition) {
        ScrollController::attach(self.as_controller(), app, position);
        debug_assert!(!app.get(self).position_to_listener.contains_key(&position));
        let listener = Listener::new(move |app| {
            let pixels = position.pixels(app);
            let tracker = app.get_mut(self);
            tracker.last_updated = Some(position);
            tracker.last_updated_offset = Some(pixels);
        });
        app.get_mut(self)
            .position_to_listener
            .insert(position, listener.clone());
        position.add_listener(app, listener);
    }

    fn detach(self: Handle<Self>, app: &mut App, position: AnyScrollPosition) {
        ScrollController::detach(self.as_controller(), app, position);
        debug_assert!(app.get(self).position_to_listener.contains_key(&position));
        let listener = app
            .get_mut(self)
            .position_to_listener
            .shift_remove(&position)
            .expect("an attached position has a listener");
        position.remove_listener(app, &listener);
        if app.get(self).last_updated == Some(position) {
            app.get_mut(self).last_updated = None;
        }
        if app.get(self).position_to_listener.is_empty() {
            app.get_mut(self).last_updated_offset = None;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for position in self.positions(app) {
            let listener = app.get(self).position_to_listener.get(&position).cloned();
            let listener = listener.expect("an attached position has a listener");
            position.remove_listener(app, &listener);
        }
        ScrollController::dispose(self.as_controller(), app);
    }
}

/// The [`ScrollControllerVTable::create_scroll_position`] slot.
type CreateScrollPositionFn = fn(
    &mut App,
    HandleId,
    ScrollPhysicsRef,
    Rc<dyn ScrollContext>,
    Option<AnyScrollPosition>,
) -> AnyScrollPosition;

/// The vtable of an erased [`AnyScrollController`]: one `&'static` table per leaf type.
struct ScrollControllerVTable {
    type_name: fn() -> &'static str,
    type_id: fn() -> TypeId,
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    dispose_change_notifier: fn(&mut App, HandleId),
    data: fn(&App, HandleId) -> &ScrollControllerData,
    data_mut: fn(&mut App, HandleId) -> &mut ScrollControllerData,
    notification_listener: fn(HandleId) -> Listener,
    initial_scroll_offset: fn(&App, HandleId) -> f64,
    attach: fn(&mut App, HandleId, AnyScrollPosition),
    detach: fn(&mut App, HandleId, AnyScrollPosition),
    dispose: fn(&mut App, HandleId),
    create_scroll_position: CreateScrollPositionFn,
    debug_fill_description: fn(&App, HandleId, &mut Vec<String>),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl ScrollControllerVTable {
    /// The table for one leaf type.
    const fn of<C: ScrollControllerLeaf>() -> ScrollControllerVTable {
        ScrollControllerVTable {
            type_name: std::any::type_name::<C>,
            type_id: TypeId::of::<C>,
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<C>(id), app, listener);
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<C>(id), app, listener);
            },
            dispose_change_notifier: |app, id| {
                app.get_mut(resolve::<C>(id))
                    .change_notifier_data_mut()
                    .dispose();
            },
            data: |app, id| C::scroll_controller_data(resolve(id), app),
            data_mut: |app, id| C::scroll_controller_data_mut(resolve(id), app),
            notification_listener: |id| C::notification_listener(resolve(id)),
            initial_scroll_offset: |app, id| C::initial_scroll_offset(resolve(id), app),
            attach: |app, id, position| C::attach(resolve(id), app, position),
            detach: |app, id, position| C::detach(resolve(id), app, position),
            dispose: |app, id| C::dispose(resolve(id), app),
            create_scroll_position: |app, id, physics, context, old_position| {
                C::create_scroll_position(resolve(id), app, physics, context, old_position)
            },
            debug_fill_description: |app, id, description| {
                C::debug_fill_description(resolve(id), app, description);
            },
        }
    }
}

/// Erased [`ScrollControllerLeaf`]: what a field or parameter Dart types as
/// `ScrollController` becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyScrollController {
    id: HandleId,
    vtable: &'static ScrollControllerVTable,
}

impl AnyScrollController {
    fn data(self, app: &App) -> &ScrollControllerData {
        (self.vtable.data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut ScrollControllerData {
        (self.vtable.data_mut)(app, self.id)
    }

    fn dispose_change_notifier(self, app: &mut App) {
        (self.vtable.dispose_change_notifier)(app, self.id);
    }

    /// Dart's `controller.runtimeType`, for the identity checks a `Scrollable` makes.
    pub fn type_id(self) -> TypeId {
        (self.vtable.type_id)()
    }

    /// See [`ScrollControllerLeaf::notification_listener`].
    pub fn notification_listener(self) -> Listener {
        (self.vtable.notification_listener)(self.id)
    }

    /// See [`ScrollControllerLeaf::initial_scroll_offset`].
    pub fn initial_scroll_offset(self, app: &App) -> f64 {
        (self.vtable.initial_scroll_offset)(app, self.id)
    }

    /// Each time a scroll completes, save the current scroll
    /// [`offset`](Self::offset) with `PageStorage` and restore it if this controller's
    /// scrollable is recreated.
    ///
    /// If this property is set to false, the scroll offset is never saved
    /// and [`initial_scroll_offset`](Self::initial_scroll_offset) is always used to initialize
    /// the scroll offset. If true (the default), the initial scroll offset is used the
    /// first time the controller's scrollable is created, since there's no
    /// scroll offset to restore yet. Subsequently the saved offset is
    /// restored and the initial scroll offset is ignored.
    pub fn keep_scroll_offset(self, app: &App) -> bool {
        self.data(app).keep_scroll_offset
    }

    /// Called when a [`ScrollPosition`] is attached to the scroll controller.
    ///
    /// Since a scroll position is not attached until a `Scrollable` is actually
    /// built, this can be used to respond to a new position being attached.
    pub fn on_attach(self, app: &App) -> Option<ScrollControllerCallback> {
        self.data(app).on_attach.clone()
    }

    /// Called when a [`ScrollPosition`] is detached from the scroll controller.
    pub fn on_detach(self, app: &App) -> Option<ScrollControllerCallback> {
        self.data(app).on_detach.clone()
    }

    /// A label that is used in the [`describe`](Self::describe) output. Intended to aid with
    /// identifying scroll controller instances in debug output.
    pub fn debug_label(self, app: &App) -> Option<String> {
        self.data(app).debug_label.clone()
    }

    /// The currently attached positions.
    ///
    /// This should not be mutated directly. [`ScrollPosition`] objects can be added
    /// and removed using [`attach`](Self::attach) and [`detach`](Self::detach).
    pub fn positions(self, app: &App) -> Vec<AnyScrollPosition> {
        self.data(app).positions.clone()
    }

    /// Whether any [`ScrollPosition`] objects have attached themselves to this
    /// controller using the [`attach`](Self::attach) method.
    ///
    /// If this is false, then members that interact with the [`ScrollPosition`],
    /// such as [`position`](Self::position), [`offset`](Self::offset),
    /// [`animate_to`](Self::animate_to), and [`jump_to`](Self::jump_to), must not be called.
    pub fn has_clients(self, app: &App) -> bool {
        !self.data(app).positions.is_empty()
    }

    /// Returns the attached [`ScrollPosition`], from which the actual scroll offset
    /// of the `ScrollView` can be obtained.
    ///
    /// Calling this is only valid when only a single position is attached.
    pub fn position(self, app: &App) -> AnyScrollPosition {
        let positions = &self.data(app).positions;
        assert!(
            !positions.is_empty(),
            "ScrollController not attached to any scroll views."
        );
        assert!(
            positions.len() == 1,
            "ScrollController attached to multiple scroll views."
        );
        positions[0]
    }

    /// The current scroll offset of the scrollable widget.
    ///
    /// Requires the controller to be controlling exactly one scrollable widget.
    pub fn offset(self, app: &App) -> f64 {
        self.position(app).pixels(app)
    }

    /// Animates the position from its current value to the given value.
    ///
    /// Any active animation is canceled. If the user is currently scrolling, that
    /// action is canceled.
    ///
    /// An animation will be interrupted whenever the user attempts to scroll
    /// manually, or whenever another activity is started, or whenever the
    /// animation reaches the edge of the viewport and attempts to overscroll. (If
    /// the [`ScrollPosition`] does not overscroll but instead allows scrolling
    /// beyond the extents, then going beyond the extents will not interrupt the
    /// animation.)
    ///
    /// The animation is indifferent to changes to the viewport or content
    /// dimensions.
    ///
    /// Once the animation has completed, the scroll position will attempt to
    /// begin a ballistic activity in case its value is not stable.
    ///
    /// The duration must not be zero. To jump to a particular value without an
    /// animation, use [`jump_to`](Self::jump_to).
    ///
    /// The returned future completes once every attached position's animation has ended.
    pub fn animate_to(
        self,
        app: &mut App,
        offset: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        assert!(
            self.has_clients(app),
            "ScrollController not attached to any scroll views."
        );
        let futures: Vec<CompleterFuture<()>> = self
            .positions(app)
            .into_iter()
            .map(|position| position.animate_to(app, offset, duration, curve.clone()))
            .collect();
        CompleterFuture::spawn(app, async move {
            wait_all(futures).await;
        })
    }

    /// Jumps the scroll position from its current value to the given value,
    /// without animation, and without checking if the new value is in range.
    ///
    /// Any active animation is canceled. If the user is currently scrolling, that
    /// action is canceled.
    ///
    /// If this method changes the scroll position, a sequence of start/update/end
    /// scroll notifications will be dispatched. No overscroll notifications can
    /// be generated by this method.
    ///
    /// Immediately after the jump, a ballistic activity is started, in case the
    /// value was out of range.
    pub fn jump_to(self, app: &mut App, value: f64) {
        assert!(
            self.has_clients(app),
            "ScrollController not attached to any scroll views."
        );
        for position in self.positions(app) {
            position.jump_to(app, value);
        }
    }

    /// See [`ScrollControllerLeaf::attach`].
    pub fn attach(self, app: &mut App, position: AnyScrollPosition) {
        (self.vtable.attach)(app, self.id, position);
    }

    /// See [`ScrollControllerLeaf::detach`].
    pub fn detach(self, app: &mut App, position: AnyScrollPosition) {
        (self.vtable.detach)(app, self.id, position);
    }

    /// See [`ScrollControllerLeaf::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// See [`ScrollControllerLeaf::create_scroll_position`].
    pub fn create_scroll_position(
        self,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> AnyScrollPosition {
        (self.vtable.create_scroll_position)(app, self.id, physics, context, old_position)
    }

    /// See [`ScrollControllerLeaf::debug_fill_description`].
    pub fn debug_fill_description(self, app: &App, description: &mut Vec<String>) {
        (self.vtable.debug_fill_description)(app, self.id, description);
    }

    /// Dart's `toString`, which reads the arena and so cannot be [`std::fmt::Debug`].
    pub fn describe(self, app: &App) -> String {
        let mut description = Vec::new();
        self.debug_fill_description(app, &mut description);
        let name = (self.vtable.type_name)();
        let short = name.rsplit("::").next().unwrap_or(name);
        format!("{short}#{:?}({})", self.id, description.join(", "))
    }
}

impl PartialEq for AnyScrollController {
    fn eq(&self, other: &AnyScrollController) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyScrollController {}

impl Debug for AnyScrollController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyScrollController({:?})", self.id)
    }
}

impl Hash for AnyScrollController {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Listenable for AnyScrollController {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener);
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::Cell;

    use inset_animation::Curves;
    use inset_foundation::ListenableObject;
    use inset_scheduler::SchedulerBinding;

    use super::*;
    use crate::test_harness::{ScrollHarness, mount_scroll_harness};
    use crate::widgets::scroll_physics::ClampingScrollPhysics;

    fn pump(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn attached(
        app: &mut App,
        harness: &ScrollHarness,
        controller: AnyScrollController,
    ) -> AnyScrollPosition {
        let position = controller.create_scroll_position(
            app,
            Rc::new(ClampingScrollPhysics::new()),
            Rc::clone(&harness.scroll_context),
            None,
        );
        controller.attach(app, position);
        position.apply_viewport_dimension(app, 100.0);
        position.apply_content_dimensions(app, 0.0, 400.0);
        app.drain_microtasks();
        position
    }

    #[test]
    fn a_controller_creates_its_position_at_the_initial_scroll_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_scroll_harness(&mut app);
        let controller =
            ScrollController::new(&mut app, 25.0, true, Some("list".to_string()), None, None);
        assert!(!controller.has_clients(&app));
        assert_eq!(controller.initial_scroll_offset(&app), 25.0);

        let position = attached(&mut app, &harness, controller.as_controller());

        assert!(controller.has_clients(&app));
        assert_eq!(controller.offset(&app), 25.0);
        assert_eq!(controller.positions(&app), vec![position]);
        assert!(
            controller
                .describe(&app)
                .contains("one client, offset 25.0")
        );

        controller.detach(&mut app, position);
        assert!(!controller.has_clients(&app));
        assert!(controller.describe(&app).contains("no clients"));
    }

    #[test]
    fn a_controller_forwards_its_positions_notifications() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_scroll_harness(&mut app);
        let controller = ScrollController::default(&mut app);
        let position = attached(&mut app, &harness, controller.as_controller());

        let notified = Rc::new(Cell::new(0));
        let counter = notified.clone();
        ListenableObject::add_listener(
            controller,
            &mut app,
            Listener::new(move |_app| counter.set(counter.get() + 1)),
        );

        controller.jump_to(&mut app, 60.0);
        assert_eq!(controller.offset(&app), 60.0);
        assert_eq!(notified.get(), 1);

        controller.detach(&mut app, position);
        position.jump_to(&mut app, 90.0);
        assert_eq!(notified.get(), 1);
    }

    /// `animate_to` drives every attached position, and its future resolves only once the last
    /// of them has settled, at the checkpoint that runs the continuation.
    #[test]
    fn animate_to_resolves_after_every_position_has_settled() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_scroll_harness(&mut app);
        let controller = ScrollController::default(&mut app);
        let first = attached(&mut app, &harness, controller.as_controller());
        let second = attached(&mut app, &harness, controller.as_controller());

        let done = controller.animate_to(
            &mut app,
            200.0,
            Duration::from_millis(100),
            Curves::linear(),
        );
        pump(&mut app, Duration::ZERO);
        pump(&mut app, Duration::from_millis(50));
        assert!((first.pixels(&app) - 100.0).abs() < 0.001);
        assert!((second.pixels(&app) - 100.0).abs() < 0.001);
        assert!(!done.is_completed());

        pump(&mut app, Duration::from_millis(100));
        assert_eq!(first.pixels(&app), 200.0);
        assert_eq!(second.pixels(&app), 200.0);
        // The frame past the duration ends both activities.
        pump(&mut app, Duration::from_millis(150));
        assert!(
            !done.is_completed(),
            "the wait over the positions resolves at the checkpoint"
        );
        drop(app);
        cell.checkpoint();
        assert!(done.is_completed());
    }

    #[test]
    fn on_attach_and_on_detach_see_the_position() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_scroll_harness(&mut app);
        let attaches = Rc::new(Cell::new(0));
        let detaches = Rc::new(Cell::new(0));
        let attached_count = attaches.clone();
        let detached_count = detaches.clone();
        let controller = ScrollController::new(
            &mut app,
            0.0,
            true,
            None,
            Some(Rc::new(move |_app, _position| {
                attached_count.set(attached_count.get() + 1);
            })),
            Some(Rc::new(move |_app, _position| {
                detached_count.set(detached_count.get() + 1);
            })),
        );

        let position = attached(&mut app, &harness, controller.as_controller());
        assert_eq!(attaches.get(), 1);
        assert_eq!(detaches.get(), 0);

        controller.detach(&mut app, position);
        assert_eq!(detaches.get(), 1);
    }

    #[test]
    fn a_tracking_controller_reports_the_most_recently_updated_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_scroll_harness(&mut app);
        let controller = TrackingScrollController::new(&mut app, 10.0, true, None, None, None);
        assert_eq!(controller.initial_scroll_offset(&app), 10.0);
        assert!(controller.most_recently_updated_position(&app).is_none());

        let position = attached(&mut app, &harness, controller.as_controller());
        assert_eq!(controller.initial_scroll_offset(&app), 10.0);

        position.jump_to(&mut app, 80.0);
        assert_eq!(controller.initial_scroll_offset(&app), 80.0);
        assert_eq!(
            controller.most_recently_updated_position(&app),
            Some(position)
        );

        controller.detach(&mut app, position);
        assert!(controller.most_recently_updated_position(&app).is_none());
        assert_eq!(controller.initial_scroll_offset(&app), 10.0);
    }
}
