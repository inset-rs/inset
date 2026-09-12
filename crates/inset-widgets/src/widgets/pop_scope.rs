//! Flutter counterpart: `widgets/pop_scope.dart`.

use std::rc::Rc;

use inset_foundation::{App, Handle, ValueListenable, ValueNotifier};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::navigator::RouteResult;
use crate::widgets::routes::{
    AnyModalRoute, PopEntry, PopEntryObject, PopInvokedWithResultCallback,
};

/// Manages back navigation gestures.
///
/// The [`can_pop`](Self::can_pop) parameter disables back gestures when set to false.
///
/// The [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result) parameter reports when
/// pop navigation was attempted, and `did_pop` indicates whether or not the navigation was
/// successful. The `result` contains the pop result.
///
/// Android has a system back gesture that is a swipe inward from near the edge of the screen.
/// It is recognized by Android before being passed to Flutter.
///
/// If [`can_pop`](Self::can_pop) is false, then a system back gesture will not pop the route
/// off of the enclosing `Navigator`.
/// [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result) will still be called, and
/// `did_pop` will be false. Programmatically attempting pop navigation will also result in a
/// call to [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result), with `did_pop`
/// indicating success or failure.
///
/// If [`can_pop`](Self::can_pop) is true, then a system back gesture will cause the enclosing
/// `Navigator` to receive a pop as usual.
/// [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result) will be called with
/// `did_pop` as true, unless the pop failed for reasons unrelated to [`PopScope`], in which
/// case it will be false.
///
/// See also:
///
///  * [`ModalRoute::register_pop_entry`](crate::ModalRoute::register_pop_entry) and
///    [`ModalRoute::unregister_pop_entry`](crate::ModalRoute::unregister_pop_entry), which this
///    widget uses to integrate with the navigation system.
pub struct PopScope {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// Called after a route pop was handled.
    ///
    /// It's not possible to prevent the pop from happening at the time that this callback is
    /// called; the pop has already happened. Use [`can_pop`](Self::can_pop) to disable pops in
    /// advance.
    ///
    /// This will still be called even when the pop is canceled. A pop is canceled when the
    /// relevant [`Route::pop_disposition`](crate::Route::pop_disposition) reports
    /// [`RoutePopDisposition::DoNotPop`](crate::RoutePopDisposition::DoNotPop), such as when
    /// [`can_pop`](Self::can_pop) is set to false on a [`PopScope`]. The `did_pop` parameter
    /// indicates whether or not the back navigation actually happened successfully.
    ///
    /// The `result` contains the pop result.
    pub on_pop_invoked_with_result: Option<PopInvokedWithResultCallback>,
    /// When false, blocks the current route from being popped.
    ///
    /// This includes the root route, where upon popping, the app would exit.
    ///
    /// If multiple [`PopScope`] widgets appear in a route's widget subtree, then each and every
    /// `can_pop` must be true in order for the route to be able to pop.
    pub can_pop: bool,
}

impl PopScope {
    /// Creates a widget that registers a callback to veto attempts by the user to dismiss the
    /// enclosing [`ModalRoute`](crate::ModalRoute); Dart's optional arguments are the setters.
    ///
    /// Dart's default: `can_pop` true.
    pub fn new<K>(child: impl IntoWidget<K>) -> PopScope {
        PopScope {
            key: None,
            child: child.into_widget(),
            on_pop_invoked_with_result: None,
            can_pop: true,
        }
    }

    /// Dart `PopScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> PopScope {
        self.key = Some(key);
        self
    }

    /// Dart `PopScope(onPopInvokedWithResult:)`.
    pub fn on_pop_invoked_with_result(
        mut self,
        on_pop_invoked_with_result: PopInvokedWithResultCallback,
    ) -> PopScope {
        self.on_pop_invoked_with_result = Some(on_pop_invoked_with_result);
        self
    }

    /// Dart `PopScope(canPop:)`.
    pub fn can_pop(mut self, can_pop: bool) -> PopScope {
        self.can_pop = can_pop;
        self
    }

    /// Dart's `_callPopInvoked`, as a closure the caller runs once it has released the arena.
    fn pop_invocation(&self, did_pop: bool, result: RouteResult) -> impl FnOnce(&mut App) + use<> {
        let callback = self.on_pop_invoked_with_result.clone();
        move |app: &mut App| {
            if let Some(callback) = callback {
                callback(app, did_pop, result);
            }
        }
    }
}

impl std::fmt::Debug for PopScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopScope")
            .field("can_pop", &self.can_pop)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PopScope {
    type State = PopScopeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> PopScopeState {
        PopScopeState {
            state: StateData::new(),
            route: None,
            can_pop_notifier: None,
            pop_entry: None,
        }
    }
}

/// Dart's `_PopScopeState`.
pub struct PopScopeState {
    state: StateData<PopScope>,
    route: Option<AnyModalRoute>,
    can_pop_notifier: Option<Handle<ValueNotifier<bool>>>,
    /// This state as the `PopEntry` it registers, minted once in `init_state`: the route
    /// unregisters by identity, and Dart passes `this`.
    pop_entry: Option<Rc<dyn PopEntry>>,
}

impl PopScopeState {
    fn pop_entry(self: Handle<Self>, app: &App) -> Rc<dyn PopEntry> {
        Rc::clone(
            app.get(self)
                .pop_entry
                .as_ref()
                .expect("init_state has run"),
        )
    }
}

impl PopEntryObject for PopScopeState {
    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        // The widget's callback is taken out of the arena before it can re-enter it.
        let call = self.widget(app).pop_invocation(did_pop, result);
        call(app);
    }

    fn can_pop_notifier(self: Handle<Self>, app: &App) -> Rc<dyn ValueListenable<bool>> {
        Rc::new(app.get(self).can_pop_notifier.expect("init_state has run"))
    }
}

impl State for PopScopeState {
    type Widget = PopScope;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let can_pop = self.widget(app).can_pop;
        app.get_mut(self).can_pop_notifier = Some(app.create(ValueNotifier::new(can_pop)));
        app.get_mut(self).pop_entry = Some(Rc::new(self));
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let next_route = AnyModalRoute::of(app, context);
        if next_route != app.get(self).route {
            if let Some(route) = app.get(self).route {
                route.unregister_pop_entry(app, self.pop_entry(app));
            }
            app.get_mut(self).route = next_route;
            if let Some(route) = next_route {
                route.register_pop_entry(app, self.pop_entry(app));
            }
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &PopScope) {
        let can_pop = self.widget(app).can_pop;
        let notifier = app.get(self).can_pop_notifier.expect("init_state has run");
        notifier.set_value(app, can_pop);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(route) = app.get(self).route {
            route.unregister_pop_entry(app, self.pop_entry(app));
        }
        let notifier = app.get(self).can_pop_notifier.expect("init_state has run");
        app.get_mut(notifier).dispose();
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.widget(app).child.clone()
    }
}
