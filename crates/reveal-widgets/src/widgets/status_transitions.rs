//! Flutter counterpart: `widgets/status_transitions.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_animation::{AnimationStatusListener, AnyAnimation};
use reveal_foundation::{App, Handle};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, Stateful, StatefulWidget, WidgetRef,
    downcast_widget,
};

/// A widget that rebuilds when the given animation changes status.
pub trait StatusTransitionWidget: Debug + 'static {
    /// See [`Widget::key`](crate::Widget::key).
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// The animation to which this widget is listening.
    fn animation(&self) -> AnyAnimation<f64>;

    /// Override this method to build widgets that depend on the current status
    /// of the animation.
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef;
}

/// The kind tag of [`IntoWidget`] for a [`StatusTransitionWidget`].
pub struct StatusTransitionKind;

/// The [`StatefulWidget`] a [`StatusTransitionWidget`] becomes; its state is Dart's
/// `_StatusTransitionState`.
pub struct StatusTransition<W: StatusTransitionWidget>(pub W);

impl<W: StatusTransitionWidget> IntoWidget<StatusTransitionKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Stateful(StatusTransition(self)))
    }
}

impl<W: StatusTransitionWidget> StatefulWidget for StatusTransition<W> {
    type State = StatusTransitionState<W>;

    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_state(&self) -> StatusTransitionState<W> {
        StatusTransitionState {
            state: StateData::new(),
        }
    }
}

impl<W: StatusTransitionWidget> Debug for StatusTransition<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Dart's `_StatusTransitionState`.
pub struct StatusTransitionState<W: StatusTransitionWidget> {
    state: StateData<StatusTransition<W>>,
}

impl<W: StatusTransitionWidget> State for StatusTransitionState<W> {
    type Widget = StatusTransition<W>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).0.animation();
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::animation_status_changed),
        );
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &StatusTransition<W>) {
        let animation = self.widget(app).0.animation();
        let old_animation = old_widget.0.animation();
        if animation != old_animation {
            old_animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::animation_status_changed),
            );
            animation.add_status_listener(
                app,
                AnimationStatusListener::handle_method(self, Self::animation_status_changed),
            );
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).0.animation();
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::animation_status_changed),
        );
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = context.widget(app).clone();
        downcast_widget::<StatusTransition<W>>(&*widget)
            .expect("a StatusTransitionState builds its StatusTransitionWidget")
            .0
            .build(app, context)
    }
}

impl<W: StatusTransitionWidget> StatusTransitionState<W> {
    fn animation_status_changed(
        self: Handle<Self>,
        app: &mut App,
        _status: reveal_animation::AnimationStatus,
    ) {
        self.set_state(app, |_state| {
            // The animation's state is our build state, and it changed already.
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use reveal_animation::{
        AnimationBehavior, AnimationController, AnimationStatus, AnyAnimation, Curves,
    };
    use reveal_scheduler::{Ticker, TickerCallback, TickerProvider};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    /// flutter_test's `TestVSync`: vends ordinary tickers.
    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            Ticker::new(app, on_tick)
        }
    }

    /// A widget whose width says what the animation's status is.
    #[derive(Debug)]
    struct StatusWidth {
        animation: AnyAnimation<f64>,
        builds: Rc<Cell<u32>>,
    }

    impl StatusTransitionWidget for StatusWidth {
        fn animation(&self) -> AnyAnimation<f64> {
            self.animation
        }

        fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            let width = match self.animation.status(app) {
                AnimationStatus::Dismissed => 10.0,
                AnimationStatus::Forward => 20.0,
                AnimationStatus::Reverse => 30.0,
                AnimationStatus::Completed => 40.0,
            };
            SizedBox::new().width(width).height(10.0).into_widget()
        }
    }

    fn width_under_root(harness: &Harness, app: &App) -> f64 {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .size(app)
            .width()
    }

    #[test]
    fn a_status_transition_rebuilds_on_status_changes_only() {
        let mut app = App::new();
        let controller = AnimationController::create(
            &mut app,
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        );
        let builds = Rc::new(Cell::new(0));
        let harness = Harness::mount(
            &mut app,
            StatusWidth {
                animation: controller.view(),
                builds: Rc::clone(&builds),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1);
        assert_eq!(width_under_root(&harness, &app), 10.0);

        controller.animate_to(
            &mut app,
            1.0,
            Some(Duration::from_millis(100)),
            Curves::linear(),
        );
        harness.pump(&mut app);
        assert_eq!(width_under_root(&harness, &app), 20.0, "forward");
        let builds_after_forward = builds.get();

        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        assert_eq!(
            builds.get(),
            builds_after_forward,
            "a value change with no status change does not rebuild"
        );

        controller.set_value(&mut app, 1.0);
        harness.pump(&mut app);
        assert_eq!(width_under_root(&harness, &app), 40.0, "completed");

        harness.set_child(&mut app, SizedBox::square(Some(5.0)).into_widget());
        harness.pump(&mut app);
        let builds_after_unmount = builds.get();
        controller.set_value(&mut app, 0.0);
        harness.pump(&mut app);
        assert_eq!(
            builds.get(),
            builds_after_unmount,
            "dispose removed the status listener"
        );

        controller.dispose(&mut app);
    }
}
