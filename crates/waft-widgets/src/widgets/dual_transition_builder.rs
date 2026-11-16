//! Flutter counterpart: `widgets/dual_transition_builder.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::{
    AnimationRef, AnimationStatus, AnimationStatusListener, ProxyAnimation, ReverseAnimation,
    k_always_complete_animation, k_always_dismissed_animation,
};
use waft_foundation::{App, Handle, OptionRetained, Ref};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

/// Builder callback used by [`DualTransitionBuilder`].
///
/// The builder is expected to return a transition powered by the provided
/// `animation` and wrapping the provided `child`.
///
/// The `animation` provided to the builder always runs forward from 0.0 to 1.0.
///
/// An optional `child` widget which represents a pre-built widget subtree that
/// does not depend on the animation's value.
///
/// Passing a pre-built widget here and incorporating it into the returned
/// widget tree avoids rebuilding it for every frame of the animation and
/// can improve performance significantly in some cases.
pub type AnimatedTransitionBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnimationRef<f64>, Option<&WidgetRef>) -> WidgetRef>;

/// A transition builder that animates its [`child`](Self::child) based on the
/// [`AnimationStatus`] of the provided [`animation`](Self::animation).
///
/// This widget can be used to specify different enter and exit transitions for
/// a [`child`](Self::child).
///
/// While the [`animation`](Self::animation) runs forward, the [`child`](Self::child) is
/// animated according to [`forward_builder`](Self::forward_builder) and while the
/// [`animation`](Self::animation) is running in reverse, it is animated according to
/// [`reverse_builder`](Self::reverse_builder).
///
/// Using this builder allows the widget tree to maintain its shape by nesting
/// the enter and exit transitions. This ensures that no state information of
/// any descendant widget is lost when the transition starts or completes.
pub struct DualTransitionBuilder {
    pub key: Option<KeyRef>,

    /// The animation that drives the [`child`](Self::child)'s transition.
    ///
    /// When this animation runs forward, the [`child`](Self::child) transitions as specified
    /// by [`forward_builder`](Self::forward_builder). When it runs in reverse, the child
    /// transitions according to [`reverse_builder`](Self::reverse_builder).
    pub animation: AnimationRef<f64>,

    /// A builder for the transition that makes [`child`](Self::child) appear on screen.
    ///
    /// The [`child`](Self::child) should be fully visible when the provided `animation`
    /// reaches 1.0.
    ///
    /// The `animation` provided to this builder is running forward from 0.0 to
    /// 1.0 when [`animation`](Self::animation) runs _forward_. When
    /// [`animation`](Self::animation) runs in reverse, the given `animation` is set to
    /// `k_always_complete_animation`.
    ///
    /// See also:
    ///
    ///  * [`reverse_builder`](Self::reverse_builder), which builds the transition for making
    ///    the [`child`](Self::child) disappear from the screen.
    pub forward_builder: AnimatedTransitionBuilder,

    /// A builder for a transition that makes [`child`](Self::child) disappear from the
    /// screen.
    ///
    /// The [`child`](Self::child) should be fully invisible when the provided `animation`
    /// reaches 1.0.
    ///
    /// The `animation` provided to this builder is running forward from 0.0 to
    /// 1.0 when [`animation`](Self::animation) runs in _reverse_. When
    /// [`animation`](Self::animation) runs forward, the given `animation` is set to
    /// `k_always_dismissed_animation`.
    ///
    /// See also:
    ///
    ///  * [`forward_builder`](Self::forward_builder), which builds the transition for making
    ///    the [`child`](Self::child) appear on screen.
    pub reverse_builder: AnimatedTransitionBuilder,

    /// The widget below this [`DualTransitionBuilder`] in the tree.
    ///
    /// This child widget will be wrapped by the transitions built by
    /// [`forward_builder`](Self::forward_builder) and
    /// [`reverse_builder`](Self::reverse_builder).
    ///
    /// If a pre-built subtree is provided as the [`child`](Self::child), it will be passed to
    /// the [`reverse_builder`](Self::reverse_builder), and the result of that will in turn be
    /// passed to the [`forward_builder`](Self::forward_builder). This allows the builders to
    /// incorporate the subtree without rebuilding it on every frame.
    ///
    /// Using this pre-built child is entirely optional, but can improve
    /// performance significantly in some cases and is therefore a good practice.
    pub child: Option<WidgetRef>,
}

impl Debug for DualTransitionBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DualTransitionBuilder")
            .field("key", &self.key)
            .field("animation", &self.animation)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl DualTransitionBuilder {
    /// Creates a [`DualTransitionBuilder`].
    pub fn new(
        animation: AnimationRef<f64>,
        forward_builder: impl Fn(
            &mut App,
            BuildContext,
            AnimationRef<f64>,
            Option<&WidgetRef>,
        ) -> WidgetRef
        + 'static,
        reverse_builder: impl Fn(
            &mut App,
            BuildContext,
            AnimationRef<f64>,
            Option<&WidgetRef>,
        ) -> WidgetRef
        + 'static,
    ) -> DualTransitionBuilder {
        DualTransitionBuilder {
            key: None,
            animation,
            forward_builder: Rc::new(forward_builder),
            reverse_builder: Rc::new(reverse_builder),
            child: None,
        }
    }

    /// Dart `DualTransitionBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> DualTransitionBuilder {
        self.key = Some(key);
        self
    }

    /// Dart `DualTransitionBuilder(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> DualTransitionBuilder {
        self.child = Some(child.into_widget());
        self
    }
}

impl StatefulWidget for DualTransitionBuilder {
    type State = DualTransitionBuilderState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DualTransitionBuilderState {
        DualTransitionBuilderState {
            state: StateData::new(),
            effective_animation_status: AnimationStatus::Dismissed,
            forward_animation: None,
            reverse_animation: None,
        }
    }
}

/// Dart's `_DualTransitionBuilderState`.
///
/// The two proxies are Dart's field initialisers; they need the `App`, so `init_state` makes
/// them before anything reads them.
pub struct DualTransitionBuilderState {
    state: StateData<DualTransitionBuilder>,
    effective_animation_status: AnimationStatus,
    forward_animation: Option<Ref<ProxyAnimation>>,
    reverse_animation: Option<Ref<ProxyAnimation>>,
}

impl Debug for DualTransitionBuilderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DualTransitionBuilderState")
            .finish_non_exhaustive()
    }
}

impl DualTransitionBuilderState {
    fn forward_animation(self: Handle<Self>, app: &App) -> Handle<ProxyAnimation> {
        app.get(self)
            .forward_animation
            .handle()
            .expect("created in init_state")
    }

    fn reverse_animation(self: Handle<Self>, app: &App) -> Handle<ProxyAnimation> {
        app.get(self)
            .reverse_animation
            .handle()
            .expect("created in init_state")
    }

    fn animation_listener(self: Handle<Self>, app: &mut App, animation_status: AnimationStatus) {
        let old_effective = app.get(self).effective_animation_status;
        let effective = calculate_effective_animation_status(old_effective, animation_status);
        app.get_mut(self).effective_animation_status = effective;
        if old_effective != effective {
            self.update_animations(app);
        }
    }

    fn update_animations(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        let forward_animation = self.forward_animation(app);
        let reverse_animation = self.reverse_animation(app);
        match app.get(self).effective_animation_status {
            AnimationStatus::Dismissed | AnimationStatus::Forward => {
                forward_animation.set_parent(app, Some(animation));
                let dismissed = k_always_dismissed_animation(app);
                reverse_animation.set_parent(app, Some(dismissed));
            }
            AnimationStatus::Reverse | AnimationStatus::Completed => {
                let complete = k_always_complete_animation(app);
                forward_animation.set_parent(app, Some(complete));
                let reversed = ReverseAnimation::new(app, animation);
                reverse_animation.set_parent(app, Some(reversed.into_erased()));
            }
        }
    }
}

/// When a transition is interrupted midway we just want to play the ongoing
/// animation in reverse. Switching to the actual reverse transition would
/// yield a disjoint experience since the forward and reverse transitions are
/// very different.
fn calculate_effective_animation_status(
    last_effective: AnimationStatus,
    current: AnimationStatus,
) -> AnimationStatus {
    match current {
        AnimationStatus::Dismissed | AnimationStatus::Completed => current,
        AnimationStatus::Forward => match last_effective {
            AnimationStatus::Dismissed | AnimationStatus::Completed | AnimationStatus::Forward => {
                current
            }
            AnimationStatus::Reverse => last_effective,
        },
        AnimationStatus::Reverse => match last_effective {
            AnimationStatus::Dismissed | AnimationStatus::Completed | AnimationStatus::Reverse => {
                current
            }
            AnimationStatus::Forward => last_effective,
        },
    }
}

impl State for DualTransitionBuilderState {
    type Widget = DualTransitionBuilder;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let forward_animation = ProxyAnimation::new(app, None);
        let reverse_animation = ProxyAnimation::new(app, None);
        let state = app.get_mut(self);
        state.forward_animation = Some(forward_animation);
        state.reverse_animation = Some(reverse_animation);
        let animation = self.widget(app).animation.handle();
        app.get_mut(self).effective_animation_status = animation.status(app);
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::animation_listener),
        );
        self.update_animations(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DualTransitionBuilder) {
        let animation = self.widget(app).animation.handle();
        if old_widget.animation != animation {
            old_widget.animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::animation_listener),
            );
            animation.add_status_listener(
                app,
                AnimationStatusListener::handle_method(self, Self::animation_listener),
            );
            let status = animation.status(app);
            self.animation_listener(app, status);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.handle();
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::animation_listener),
        );
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let forward_builder = Rc::clone(&widget.forward_builder);
        let reverse_builder = Rc::clone(&widget.reverse_builder);
        let child = widget.child.clone();
        let state = app.get(self);
        let forward_animation: AnimationRef<f64> = state
            .forward_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let reverse_animation: AnimationRef<f64> = state
            .reverse_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let reversed = reverse_builder(app, context, reverse_animation, child.as_ref());
        forward_builder(app, context, forward_animation, Some(&reversed))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;
    use waft_animation::Drive;
    use waft_foundation::Ref;

    use waft_animation::{AnimationBehavior, AnimationController, AnyAnimation, Tween};
    use waft_embedder::Color;

    use super::*;
    use crate::testing::{TestVSync, WidgetTester, test_widgets};
    use crate::widgets::basic::{Center, SizedBox};
    use crate::widgets::container::Container;
    use crate::widgets::transitions::{FadeTransition, ScaleTransition};

    /// What the two builders were last given: Dart's `_getScale` and `_getOpacity` read the
    /// `ScaleTransition` and `FadeTransition` the builders return, whose animations these are.
    #[derive(Clone, Default)]
    struct Observed {
        scale: Rc<Cell<Option<AnyAnimation<f64>>>>,
        opacity: Rc<Cell<Option<AnyAnimation<f64>>>>,
    }

    impl Observed {
        fn scale(&self, app: &App) -> f64 {
            self.scale.get().expect("built").value(app)
        }

        fn opacity(&self, app: &App) -> f64 {
            self.opacity.get().expect("built").value(app)
        }
    }

    fn controller(app: &mut App, value: Option<f64>) -> Ref<AnimationController> {
        AnimationController::new(
            app,
            value,
            Some(Duration::from_millis(300)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    fn tree(animation: AnimationRef<f64>, observed: &Observed, child: WidgetRef) -> WidgetRef {
        let scale = Rc::clone(&observed.scale);
        let opacity = Rc::clone(&observed.opacity);
        Center::new()
            .child(
                DualTransitionBuilder::new(
                    animation,
                    move |_app, _context, animation, child| {
                        scale.set(Some(animation.handle()));
                        ScaleTransition::new(animation)
                            .child(child.expect("a child").clone())
                            .into_widget()
                    },
                    move |app, _context, animation, child| {
                        let tween = Tween::new(app, Some(1.0), Some(0.0));
                        let faded = animation.drive(app, tween);
                        opacity.set(Some(faded.handle()));
                        FadeTransition::new(faded)
                            .child(child.expect("a child").clone())
                            .into_widget()
                    },
                )
                .child(child),
            )
            .into_widget()
    }

    fn green_box() -> WidgetRef {
        Container::new()
            .color(Color::new(0xFF00FF00))
            .height(100.0)
            .width(100.0)
            .into_widget()
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn assert_scale_and_opacity(
        tester: &WidgetTester,
        observed: &Observed,
        scale: f64,
        opacity: f64,
    ) {
        let app = tester.cell.borrow();
        assert_eq!(observed.scale(&app), scale);
        assert_eq!(observed.opacity(&app), opacity);
    }

    /// Pumps the tree for `controller`.
    fn pump_tree(
        tester: &mut WidgetTester,
        controller: &Ref<AnimationController>,
        child: WidgetRef,
    ) -> Observed {
        let observed = Observed::default();
        tester.pump_widget(tree(controller.erase(), &observed, child));
        observed
    }

    #[test]
    fn runs_animations() {
        test_widgets(|tester| {
            let controller = tester.update(|app| controller(app, None));
            let observed = pump_tree(tester, &controller, green_box());
            assert_scale_and_opacity(tester, &observed, 0.0, 1.0);

            tester.update(|app| controller.forward(app, None));
            tester.pump();
            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 0.5, 1.0);

            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 1.0, 1.0);

            tester.pump_by(ms(300));
            assert_scale_and_opacity(tester, &observed, 1.0, 1.0);

            tester.update(|app| controller.reverse(app, None));
            tester.pump();
            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 1.0, 0.5);

            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 1.0, 0.0);

            tester.pump_by(ms(300));
            assert_scale_and_opacity(tester, &observed, 0.0, 1.0);
            tester.update(|app| controller.dispose(app));
        });
    }

    /// Not a Dart case: the proxies and the `ReverseAnimation` a state made are freed once the
    /// unmounted tree lets go of them, as Dart's collector frees them.
    #[test]
    fn a_disposed_state_frees_its_proxies_and_reverse_animation() {
        test_widgets(|tester| {
            let controller = tester.update(|app| controller(app, Some(1.0)));
            let forward: Rc<Cell<Option<AnyAnimation<f64>>>> = Rc::default();
            let reverse: Rc<Cell<Option<AnyAnimation<f64>>>> = Rc::default();
            let tree = {
                let (forward, reverse) = (Rc::clone(&forward), Rc::clone(&reverse));
                DualTransitionBuilder::new(
                    controller.erase(),
                    move |_app, _context, animation, child| {
                        forward.set(Some(animation.handle()));
                        child.cloned().expect("a child")
                    },
                    move |_app, _context, animation, child| {
                        reverse.set(Some(animation.handle()));
                        child.cloned().expect("a child")
                    },
                )
                .child(green_box())
                .into_widget()
            };
            tester.pump_widget(Center::new().child(tree));
            let proxies = [forward, reverse].map(|built| built.get().expect("the builder ran"));
            let reverse_parent = {
                let app = tester.cell.borrow();
                let reverse = app
                    .handle::<ProxyAnimation>(proxies[1].id())
                    .expect("the reverse proxy");
                reverse
                    .parent(&app)
                    .expect("a completed animation is reversed")
            };

            tester.pump_widget(SizedBox::new());
            let app = tester.cell.borrow();
            for freed in [proxies[0].id(), proxies[1].id(), reverse_parent.id()] {
                assert!(!app.contains(freed), "{freed:?} is freed");
            }
            assert!(
                app.contains(&controller),
                "the widget's animation is its owner's"
            );
        });
    }

    #[test]
    fn keeps_state() {
        test_widgets(|tester| {
            let controller = tester.update(|app| controller(app, None));
            let probe = Probe::default();
            pump_tree(tester, &controller, probe.widget());
            assert_eq!((probe.inits.get(), probe.disposes.get()), (1, 0));

            for direction in [AnimationController::forward, AnimationController::reverse] {
                tester.update(|app| direction(controller.handle(), app, None));
                for step in [0, 150, 150, 300] {
                    tester.pump_by(ms(step));
                    assert_eq!((probe.inits.get(), probe.disposes.get()), (1, 0));
                }
            }
            tester.update(|app| controller.dispose(app));
        });
    }

    #[test]
    fn does_not_jump_when_interrupted_forward() {
        test_widgets(|tester| {
            let controller = tester.update(|app| controller(app, None));
            let observed = pump_tree(tester, &controller, green_box());
            assert_scale_and_opacity(tester, &observed, 0.0, 1.0);

            tester.update(|app| controller.forward(app, None));
            tester.pump();
            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 0.5, 1.0);

            tester.update(|app| controller.reverse(app, None));
            assert_scale_and_opacity(tester, &observed, 0.5, 1.0);
            tester.pump();
            assert_scale_and_opacity(tester, &observed, 0.5, 1.0);

            tester.pump_by(ms(75));
            assert_scale_and_opacity(tester, &observed, 0.25, 1.0);

            tester.pump_by(ms(75));
            assert_scale_and_opacity(tester, &observed, 0.0, 1.0);

            tester.pump_by(ms(300));
            assert_scale_and_opacity(tester, &observed, 0.0, 1.0);
            tester.update(|app| controller.dispose(app));
        });
    }

    #[test]
    fn does_not_jump_when_interrupted_reverse() {
        test_widgets(|tester| {
            let controller = tester.update(|app| controller(app, Some(1.0)));
            let observed = pump_tree(tester, &controller, green_box());
            assert_scale_and_opacity(tester, &observed, 1.0, 1.0);

            tester.update(|app| controller.reverse(app, None));
            tester.pump();
            tester.pump_by(ms(150));
            assert_scale_and_opacity(tester, &observed, 1.0, 0.5);

            tester.update(|app| controller.forward(app, None));
            assert_scale_and_opacity(tester, &observed, 1.0, 0.5);
            tester.pump();
            assert_scale_and_opacity(tester, &observed, 1.0, 0.5);

            tester.pump_by(ms(75));
            assert_scale_and_opacity(tester, &observed, 1.0, 0.75);

            tester.pump_by(ms(75));
            assert_scale_and_opacity(tester, &observed, 1.0, 1.0);

            tester.pump_by(ms(300));
            assert_scale_and_opacity(tester, &observed, 1.0, 1.0);
            tester.update(|app| controller.dispose(app));
        });
    }

    /// Dart's `_StatefulTestWidget`, counting its state's life: `same(state)` is one `init_state`
    /// and no `dispose`.
    #[derive(Clone, Default)]
    struct Probe {
        inits: Rc<Cell<u32>>,
        disposes: Rc<Cell<u32>>,
    }

    impl Probe {
        fn widget(&self) -> WidgetRef {
            ProbeWidget {
                probe: self.clone(),
            }
            .into_widget()
        }
    }

    #[derive(Debug)]
    struct ProbeWidget {
        probe: Probe,
    }

    impl Debug for Probe {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Probe")
        }
    }

    impl StatefulWidget for ProbeWidget {
        type State = ProbeState;

        fn create_state(&self) -> ProbeState {
            ProbeState {
                state: StateData::new(),
            }
        }
    }

    struct ProbeState {
        state: StateData<ProbeWidget>,
    }

    impl State for ProbeState {
        type Widget = ProbeWidget;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let inits = &self.widget(app).probe.inits;
            inits.set(inits.get() + 1);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let disposes = &self.widget(app).probe.disposes;
            disposes.set(disposes.get() + 1);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().width(10.0).height(10.0).into_widget()
        }
    }
}
