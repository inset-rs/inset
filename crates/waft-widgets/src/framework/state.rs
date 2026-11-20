//! `State`: the logic and internal state for a [`StatefulWidget`].
//!
//! A state object is one struct in the arena, reached through `Handle<Self>`; the framework
//! bookkeeping (Dart's `_element`, `_widget`, and the debug lifecycle) is the [`StateData`]
//! bag the struct holds under the field `state`.

use std::any::TypeId;
use std::marker::PhantomData;

use waft_foundation::{App, Handle, HandleId};

use super::element::BuildContext;
use super::elements::StatefulElement;
use super::widget::{StatefulWidget, WidgetRef, downcast_widget};

/// Tracks the lifecycle of [`State`] objects when asserts are enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateLifecycle {
    /// The [`State`] object has been created. [`State::init_state`] is called at this time.
    Created,
    /// The [`State::init_state`] method has been called but the [`State`] object is not yet
    /// ready to build. [`State::did_change_dependencies`] is called at this time.
    Initialized,
    /// The [`State`] object is ready to build and [`State::dispose`] has not yet been called.
    Ready,
    /// The [`State::dispose`] method has been called and the [`State`] object is no longer
    /// able to build.
    Defunct,
}

/// The framework's part of a [`State`]: the element it belongs to and its lifecycle.
pub struct StateData<W: StatefulWidget> {
    widget: Option<WidgetRef>,
    element: Option<Handle<StatefulElement>>,
    debug_lifecycle_state: StateLifecycle,
    /// The state's widget type. The bag holds nothing of it since its element stopped being
    /// generic; it stays so a state's `state: StateData<MyWidget>` field and
    /// [`State::state_data`] keep their shape.
    widget_type: PhantomData<fn() -> W>,
}

impl<W: StatefulWidget> StateData<W> {
    /// A state that is not yet attached to an element.
    pub fn new() -> StateData<W> {
        StateData {
            widget: None,
            element: None,
            debug_lifecycle_state: StateLifecycle::Created,
            widget_type: PhantomData,
        }
    }

    /// Dart's `State._widget`, set by the element before `init_state` and on every update.
    pub(crate) fn set_widget(&mut self, widget: WidgetRef) {
        self.widget = Some(widget);
    }

    pub(crate) fn element(&self) -> Option<Handle<StatefulElement>> {
        self.element
    }

    pub(crate) fn set_element(&mut self, element: Option<Handle<StatefulElement>>) {
        self.element = element;
    }

    pub(crate) fn debug_lifecycle_state(&self) -> StateLifecycle {
        self.debug_lifecycle_state
    }

    pub(crate) fn set_debug_lifecycle_state(&mut self, state: StateLifecycle) {
        self.debug_lifecycle_state = state;
    }
}

impl<W: StatefulWidget> Default for StateData<W> {
    fn default() -> StateData<W> {
        StateData::new()
    }
}

/// A [`State`] of any type, as the one [`StatefulElement`] holds it: its handle and the hooks
/// the element calls, so the element is not copied per state type. Opaque outside the framework.
#[derive(Clone, Copy)]
pub struct AnyState {
    id: HandleId,
    vtable: &'static StateVTable,
}

/// The hooks of one [`State`] type that its element calls.
struct StateVTable {
    type_id: TypeId,
    build: fn(&mut App, HandleId, BuildContext) -> WidgetRef,
    init_state: fn(&mut App, HandleId),
    did_change_dependencies: fn(&mut App, HandleId),
    did_update_widget: fn(&mut App, HandleId, &WidgetRef),
    activate: fn(&mut App, HandleId),
    deactivate: fn(&mut App, HandleId),
    dispose: fn(&mut App, HandleId),
    set_widget: fn(&mut App, HandleId, WidgetRef),
    element: fn(&App, HandleId) -> Option<Handle<StatefulElement>>,
    set_element: fn(&mut App, HandleId, Option<Handle<StatefulElement>>),
    debug_lifecycle_state: fn(&App, HandleId) -> StateLifecycle,
    set_debug_lifecycle_state: fn(&mut App, HandleId, StateLifecycle),
}

impl StateVTable {
    const fn of<S: State>() -> StateVTable {
        fn typed<S: State>(id: HandleId) -> Handle<S> {
            Handle::from_id(id)
        }
        StateVTable {
            type_id: TypeId::of::<S>(),
            build: |app, id, context| typed::<S>(id).build(app, context),
            init_state: |app, id| typed::<S>(id).init_state(app),
            did_change_dependencies: |app, id| typed::<S>(id).did_change_dependencies(app),
            did_update_widget: |app, id, old_widget| {
                let old_widget = downcast_widget::<S::Widget>(&**old_widget)
                    .expect("a StatefulElement holds its StatefulWidget");
                typed::<S>(id).did_update_widget(app, old_widget);
            },
            activate: |app, id| typed::<S>(id).activate(app),
            deactivate: |app, id| typed::<S>(id).deactivate(app),
            dispose: |app, id| typed::<S>(id).dispose(app),
            set_widget: |app, id, widget| typed::<S>(id).state_data_mut(app).set_widget(widget),
            element: |app, id| typed::<S>(id).state_data(app).element(),
            set_element: |app, id, element| typed::<S>(id).state_data_mut(app).set_element(element),
            debug_lifecycle_state: |app, id| typed::<S>(id).state_data(app).debug_lifecycle_state(),
            set_debug_lifecycle_state: |app, id, lifecycle| {
                typed::<S>(id)
                    .state_data_mut(app)
                    .set_debug_lifecycle_state(lifecycle);
            },
        }
    }
}

impl AnyState {
    pub(crate) fn of<S: State>(state: Handle<S>) -> AnyState {
        AnyState {
            id: state.id(),
            vtable: const { &StateVTable::of::<S>() },
        }
    }

    pub(crate) fn id(self) -> HandleId {
        self.id
    }

    pub(crate) fn type_id(self) -> TypeId {
        self.vtable.type_id
    }

    pub(crate) fn build(self, app: &mut App, context: BuildContext) -> WidgetRef {
        (self.vtable.build)(app, self.id, context)
    }

    pub(crate) fn init_state(self, app: &mut App) {
        (self.vtable.init_state)(app, self.id);
    }

    pub(crate) fn did_change_dependencies(self, app: &mut App) {
        (self.vtable.did_change_dependencies)(app, self.id);
    }

    pub(crate) fn did_update_widget(self, app: &mut App, old_widget: &WidgetRef) {
        (self.vtable.did_update_widget)(app, self.id, old_widget);
    }

    pub(crate) fn activate(self, app: &mut App) {
        (self.vtable.activate)(app, self.id);
    }

    pub(crate) fn deactivate(self, app: &mut App) {
        (self.vtable.deactivate)(app, self.id);
    }

    pub(crate) fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    pub(crate) fn set_widget(self, app: &mut App, widget: WidgetRef) {
        (self.vtable.set_widget)(app, self.id, widget);
    }

    pub(crate) fn element(self, app: &App) -> Option<Handle<StatefulElement>> {
        (self.vtable.element)(app, self.id)
    }

    pub(crate) fn set_element(self, app: &mut App, element: Option<Handle<StatefulElement>>) {
        (self.vtable.set_element)(app, self.id, element);
    }

    pub(crate) fn debug_lifecycle_state(self, app: &App) -> StateLifecycle {
        (self.vtable.debug_lifecycle_state)(app, self.id)
    }

    pub(crate) fn set_debug_lifecycle_state(self, app: &mut App, lifecycle: StateLifecycle) {
        (self.vtable.set_debug_lifecycle_state)(app, self.id, lifecycle);
    }
}

/// The accessors [`State`] asks for, for a struct whose bag is the field `state`.
#[macro_export]
macro_rules! state_accessors {
    () => {
        fn state_data(
            self: $crate::__private::foundation::Handle<Self>,
            app: &$crate::__private::foundation::App,
        ) -> &$crate::StateData<Self::Widget> {
            &app.get(self).state
        }

        fn state_data_mut(
            self: $crate::__private::foundation::Handle<Self>,
            app: &mut $crate::__private::foundation::App,
        ) -> &mut $crate::StateData<Self::Widget> {
            &mut app.get_mut(self).state
        }
    };
}

/// The logic and internal state for a [`StatefulWidget`].
///
/// State is information that (1) can be read synchronously when the widget is built and (2)
/// might change during the lifetime of the widget. It is the responsibility of the widget
/// implementer to ensure that the [`State`] is promptly notified when such state changes,
/// using [`set_state`](Self::set_state).
///
/// [`State`] objects are created by the framework by calling the
/// [`StatefulWidget::create_state`] method when inflating a [`StatefulWidget`] to insert it
/// into the tree. Because a given [`StatefulWidget`] instance can be inflated multiple times
/// (e.g., the widget is incorporated into the tree in multiple places at once), there might
/// be more than one [`State`] object associated with a given [`StatefulWidget`] instance.
/// Similarly, if a [`StatefulWidget`] is removed from the tree and later inserted in to the
/// tree again, the framework will call [`StatefulWidget::create_state`] again to create a
/// fresh [`State`] object, simplifying the lifecycle of [`State`] objects.
///
/// Methods take `self: Handle<Self>`: the state lives in the arena, and [`widget`](Self::widget)
/// / [`context`](Self::context) read the element it is attached to.
///
/// # When a widget needs one
///
/// A [`State`] is for what a widget *owns* as a Flutter object across its builds: a
/// [`TextEditingController`](crate::TextEditingController), a [`FocusNode`](crate::FocusNode), an
/// [`AnimationController`](crate::AnimationController) it drives. The app's own data is not
/// state: it lives in an [`Entity`](waft_foundation::Entity), which a
/// [`StatelessWidget`](crate::StatelessWidget) reads in `build` and is rebuilt by when it
/// notifies. A stateful widget whose only job is to hold a counter or a fetched list is app data
/// in the wrong place.
pub trait State: Sized + 'static {
    /// The widget this state belongs to; Dart's `State<T>` type argument.
    type Widget: StatefulWidget<State = Self>;

    /// The framework's bookkeeping, held under the field `state`
    /// ([`state_accessors!`](crate::state_accessors)).
    fn state_data(self: Handle<Self>, app: &App) -> &StateData<Self::Widget>;

    /// See [`state_data`](Self::state_data).
    fn state_data_mut(self: Handle<Self>, app: &mut App) -> &mut StateData<Self::Widget>;

    /// The current configuration.
    ///
    /// A [`State`] object's configuration is the corresponding [`StatefulWidget`] instance.
    /// This property is initialized by the framework before calling
    /// [`init_state`](Self::init_state). If the parent updates this location in the tree to a
    /// new widget with the same `runtimeType` and [`Widget::key`](super::Widget::key) as the
    /// current configuration, the framework will update this property to refer to the new
    /// widget and then call [`did_update_widget`](Self::did_update_widget), passing the old
    /// configuration as an argument.
    fn widget(self: Handle<Self>, app: &App) -> &Self::Widget {
        let widget = self
            .state_data(app)
            .widget
            .as_ref()
            .expect("the framework sets the widget before init_state");
        downcast_widget::<Self::Widget>(&**widget)
            .expect("a StatefulElement holds its StatefulWidget")
    }

    /// The location in the tree where this widget builds.
    ///
    /// The framework associates [`State`] objects with a [`BuildContext`] after creating them
    /// with [`StatefulWidget::create_state`] and before calling
    /// [`init_state`](Self::init_state). The association is permanent: the [`State`] object
    /// will never change its [`BuildContext`]. However, the [`BuildContext`] itself can be
    /// moved around the tree.
    ///
    /// After calling [`dispose`](Self::dispose), the framework severs the [`State`] object's
    /// connection with the [`BuildContext`].
    fn context(self: Handle<Self>, app: &App) -> BuildContext {
        use super::element::Element;
        self.state_data(app)
            .element()
            .expect(
                "This widget has been unmounted, so the State no longer has a context (and \
                 should be considered defunct). Consider canceling any active work during \
                 \"dispose\" or using the \"mounted\" getter to determine if the State is still \
                 active.",
            )
            .as_element()
    }

    /// Whether this [`State`] object is currently in a tree.
    ///
    /// After creating a [`State`] object and before calling [`init_state`](Self::init_state),
    /// the framework "mounts" the [`State`] object by associating it with a [`BuildContext`].
    /// The [`State`] object remains mounted until the framework calls
    /// [`dispose`](Self::dispose), after which time the framework will never ask the
    /// [`State`] object to [`build`](Self::build) again.
    ///
    /// It is an error to call [`set_state`](Self::set_state) unless
    /// [`mounted`](Self::mounted) is true.
    fn mounted(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.state_data(app).element().is_some()
    }

    /// Called when this object is inserted into the tree.
    ///
    /// The framework will call this method exactly once for each [`State`] object it creates.
    ///
    /// Override this method to perform initialization that depends on the location at which
    /// this object was inserted into the tree (i.e., [`context`](Self::context)) or on the
    /// widget used to configure this object (i.e., [`widget`](Self::widget)).
    ///
    /// You should not use `BuildContext::depend_on_inherited_widget_of_exact_type` from this
    /// method. However, [`did_change_dependencies`](Self::did_change_dependencies) will be
    /// called immediately following this method, and
    /// `BuildContext::depend_on_inherited_widget_of_exact_type` can be used there.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Called whenever the widget configuration changes.
    ///
    /// If the parent widget rebuilds and requests that this location in the tree update to
    /// display a new widget with the same `runtimeType` and [`Widget::key`](super::Widget::key),
    /// the framework will update the [`widget`](Self::widget) property of this [`State`]
    /// object to refer to the new widget and then call this method with the previous widget
    /// as an argument.
    ///
    /// Override this method to respond when the [`widget`](Self::widget) changes (e.g., to
    /// start implicit animations).
    ///
    /// The framework always calls [`build`](Self::build) after calling
    /// [`did_update_widget`](Self::did_update_widget), which means any calls to
    /// [`set_state`](Self::set_state) in [`did_update_widget`](Self::did_update_widget) are
    /// redundant.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Self::Widget) {
        let _ = (self, app, old_widget);
    }

    /// Notify the framework that the internal state of this object has changed.
    ///
    /// Whenever you change the internal state of a [`State`] object, make the change in a
    /// function that you pass to [`set_state`](Self::set_state):
    ///
    /// ```text
    /// self.set_state(app, |state| { state.my_state = new_value; });
    /// ```
    ///
    /// The provided callback is immediately called synchronously. It must not return a
    /// future (the callback cannot be `async`), since then it would be unclear when the
    /// state was actually being set.
    ///
    /// Calling [`set_state`](Self::set_state) notifies the framework that the internal state
    /// of this object has changed in a way that might impact the user interface in this
    /// subtree, which causes the framework to schedule a [`build`](Self::build) for this
    /// [`State`] object.
    ///
    /// If you just change the state directly without calling
    /// [`set_state`](Self::set_state), the framework might not schedule a
    /// [`build`](Self::build) and the user interface for this subtree might not be updated
    /// to reflect the new state.
    fn set_state(self: Handle<Self>, app: &mut App, f: impl FnOnce(&mut Self)) {
        use super::element::Element;
        if cfg!(debug_assertions) {
            let lifecycle = self.state_data(app).debug_lifecycle_state();
            assert!(
                lifecycle != StateLifecycle::Defunct,
                "setState() called after dispose(): this error happens if you call setState() \
                 on a State object for a widget that no longer appears in the widget tree \
                 (e.g., whose parent widget no longer includes the widget in its build)."
            );
            assert!(
                !(lifecycle == StateLifecycle::Created && !self.mounted(app)),
                "setState() called in constructor: this happens when you call setState() on a \
                 State object for a widget that hasn't been inserted into the widget tree yet."
            );
        }
        f(app.get_mut(self));
        let element = self
            .state_data(app)
            .element()
            .expect("set_state needs a mounted state");
        element.as_element().mark_needs_build(app);
    }

    /// Called when this object is removed from the tree.
    ///
    /// The framework calls this method whenever it removes this [`State`] object from the
    /// tree. In some cases, the framework will reinsert the [`State`] object into another
    /// part of the tree (e.g., if the subtree containing this [`State`] object is grafted
    /// from one location in the tree to another due to the use of a `GlobalKey`). If that
    /// happens, the framework will call [`activate`](Self::activate) to give the [`State`]
    /// object a chance to reacquire any resources that it released in
    /// [`deactivate`](Self::deactivate). It will then also call [`build`](Self::build) to give
    /// the [`State`] object a chance to adapt to its new location in the tree. If the
    /// framework does reinsert this subtree, it will do so before the end of the animation
    /// frame in which the subtree was removed from the tree. For this reason, [`State`]
    /// objects can defer releasing most resources until the framework calls their
    /// [`dispose`](Self::dispose) method.
    fn deactivate(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Called when this object is reinserted into the tree after having been removed via
    /// [`deactivate`](Self::deactivate).
    ///
    /// In most cases, after a [`State`] object has been deactivated, it is _not_ reinserted
    /// into the tree, and its [`dispose`](Self::dispose) method will be called to signal that
    /// it is ready to be garbage collected.
    ///
    /// In some cases, however, after a [`State`] object has been deactivated, the framework
    /// will reinsert it into another part of the tree (e.g., if the subtree containing this
    /// [`State`] object is grafted from one location in the tree to another due to the use
    /// of a `GlobalKey`). If that happens, the framework will call
    /// [`activate`](Self::activate) to give the [`State`] object a chance to reacquire any
    /// resources that it released in [`deactivate`](Self::deactivate). It will then also call
    /// [`build`](Self::build) to give the object a chance to adapt to its new location in the
    /// tree.
    fn activate(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Called when this object is removed from the tree permanently.
    ///
    /// The framework calls this method when this [`State`] object will never build again.
    /// After the framework calls [`dispose`](Self::dispose), the [`State`] object is
    /// considered unmounted and the [`mounted`](Self::mounted) property is false. It is an
    /// error to call [`set_state`](Self::set_state) at this point.
    ///
    /// Subclasses should override this method to release any resources retained by this
    /// object (e.g., stop any active animations).
    fn dispose(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Describes the part of the user interface represented by this widget.
    ///
    /// The framework calls this method in a number of different situations. For example:
    ///
    ///  * After calling [`init_state`](Self::init_state).
    ///  * After calling [`did_update_widget`](Self::did_update_widget).
    ///  * After receiving a call to [`set_state`](Self::set_state).
    ///  * After a dependency of this [`State`] object changes (e.g., an `InheritedWidget`
    ///    referenced by the previous [`build`](Self::build) changes).
    ///  * After calling [`deactivate`](Self::deactivate) and then reinserting the [`State`]
    ///    object into the tree at another location.
    ///
    /// This method can potentially be called in every frame and should not have any side
    /// effects beyond building a widget.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef;

    /// Called when a dependency of this [`State`] object changes.
    ///
    /// For example, if the previous call to [`build`](Self::build) referenced an
    /// `InheritedWidget` that later changed, the framework would call this method to notify
    /// this object about the change.
    ///
    /// This method is also called immediately after [`init_state`](Self::init_state). It is
    /// safe to call `BuildContext::depend_on_inherited_widget_of_exact_type` from this method.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }
}
