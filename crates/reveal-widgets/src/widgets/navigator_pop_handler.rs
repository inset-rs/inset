//! Flutter counterpart: `widgets/navigator_pop_handler.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_foundation::{App, Handle};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::navigator::{NavigationNotification, RouteResult};
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::pop_scope::PopScope;

/// A signature for a function that is passed the result of a `Route`.
pub type PopResultCallback = Rc<dyn Fn(&mut App, RouteResult)>;

/// Enables the handling of system back gestures.
///
/// Typically wraps a nested `Navigator` widget and allows it to handle system
/// back gestures in the [`on_pop_with_result`](Self::on_pop_with_result) callback.
///
/// See also:
///
///  * [`PopScope`], which allows toggling the ability of a `Navigator` to
///    handle pops.
///  * [`NavigationNotification`], which indicates whether a `Navigator` in a
///    subtree can handle pops.
pub struct NavigatorPopHandler {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,
    /// The widget to place below this in the widget tree.
    ///
    /// Typically this is a `Navigator` that will handle the pop when
    /// [`on_pop_with_result`](Self::on_pop_with_result) is called.
    pub child: WidgetRef,
    /// Whether this widget's ability to handle system back gestures is enabled or
    /// disabled.
    ///
    /// When false, there will be no effect on system back gestures.
    ///
    /// This can be used, for example, when the nested `Navigator` is no longer
    /// active but remains in the widget tree, such as in an inactive tab.
    ///
    /// Defaults to true.
    pub enabled: bool,
    /// Called when a handleable pop event happens.
    ///
    /// For example, a pop is handleable when a `Navigator` in
    /// [`child`](Self::child) has multiple routes on its stack. It's not handleable when it
    /// has only a single route, and so this is not called.
    ///
    /// Typically this is used to pop the `Navigator` in [`child`](Self::child).
    ///
    /// The passed `result` is the result of the popped `Route`.
    pub on_pop_with_result: Option<PopResultCallback>,
}

impl NavigatorPopHandler {
    /// Creates an instance of [`NavigatorPopHandler`]; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's default: `enabled` true.
    pub fn new<K>(child: impl IntoWidget<K>) -> NavigatorPopHandler {
        NavigatorPopHandler {
            key: None,
            child: child.into_widget(),
            enabled: true,
            on_pop_with_result: None,
        }
    }

    /// Dart `NavigatorPopHandler(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigatorPopHandler {
        self.key = Some(key);
        self
    }

    /// Dart `NavigatorPopHandler(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> NavigatorPopHandler {
        self.enabled = enabled;
        self
    }

    /// Dart `NavigatorPopHandler(onPopWithResult:)`.
    pub fn on_pop_with_result(
        mut self,
        on_pop_with_result: impl Fn(&mut App, RouteResult) + 'static,
    ) -> NavigatorPopHandler {
        self.on_pop_with_result = Some(Rc::new(on_pop_with_result));
        self
    }
}

impl Debug for NavigatorPopHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigatorPopHandler")
            .field("enabled", &self.enabled)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for NavigatorPopHandler {
    type State = NavigatorPopHandlerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> NavigatorPopHandlerState {
        NavigatorPopHandlerState {
            state: StateData::new(),
            can_pop: true,
        }
    }
}

/// Dart's `_NavigatorPopHandlerState`.
pub struct NavigatorPopHandlerState {
    state: StateData<NavigatorPopHandler>,
    can_pop: bool,
}

impl State for NavigatorPopHandlerState {
    type Widget = NavigatorPopHandler;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let enabled = self.widget(app).enabled;
        let can_pop = app.get(self).can_pop;
        let child = self.widget(app).child.clone();
        // When the widget subtree indicates it can handle a pop, disable popping
        // here, so that it can be manually handled in `can_pop`.
        PopScope::new(
            // Listen to changes in the navigation stack in the widget subtree.
            NotificationListener::<NavigationNotification>::new(child)
                .on_notification(move |app, notification: &NavigationNotification| {
                    // If this subtree cannot handle pop, then set `can_pop` to true so
                    // that our `PopScope` will allow the `Navigator` higher in the tree to
                    // handle the pop instead.
                    let next_can_pop = !notification.can_handle_pop;
                    if next_can_pop != app.get(self).can_pop {
                        self.set_state(app, |state| state.can_pop = next_can_pop);
                    }
                    false
                })
                .into_widget(),
        )
        .can_pop(!enabled || can_pop)
        .on_pop_invoked_with_result(Rc::new(
            move |app: &mut App, did_pop: bool, result: RouteResult| {
                if did_pop {
                    return;
                }
                if let Some(on_pop_with_result) = self.widget(app).on_pop_with_result.clone() {
                    on_pop_with_result(app, result);
                }
            },
        ))
        .into_widget()
    }
}
