//! Flutter counterpart: `widgets/transitions.dart`.
//!
//! `AnimatedWidget` and `FadeTransition`; the other transitions wait.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_animation::AnyAnimation;
use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_rendering::{
    AnyRenderObject, RenderAnimatedOpacity, RenderAnimatedOpacityMixin, RenderBox, RenderHandle,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, State,
    StateData, Stateful, StatefulWidget, WidgetRef, downcast_widget,
};

/// A widget that rebuilds when the given [`Listenable`] changes value.
///
/// [`AnimatedWidget`] is most commonly used with `Animation` objects, which are
/// [`Listenable`], but it can be used with any [`Listenable`], including
/// `ChangeNotifier` and `ValueNotifier`.
///
/// [`AnimatedWidget`] is most useful for widgets that are otherwise stateless. To
/// use [`AnimatedWidget`], implement this trait with its [`build`](Self::build) function;
/// the value becomes a tree node with `.into_widget()`, like every widget kind.
///
/// ## Relationship to `ImplicitlyAnimatedWidget`s
///
/// [`AnimatedWidget`]s (and their subclasses) take an explicit [`Listenable`] as
/// argument, which is usually an `Animation` derived from an
/// `AnimationController`. In most cases, the lifecycle of that
/// `AnimationController` has to be managed manually by the developer.
/// In contrast to that, `ImplicitlyAnimatedWidget`s (and their subclasses)
/// automatically manage their own internal `AnimationController` making those
/// classes easier to use as no external `Animation` has to be provided by the
/// developer. If you only need to set a target value for the animation and
/// configure its duration/curve, consider using (a subclass of)
/// `ImplicitlyAnimatedWidget`s instead of (a subclass of) this class.
///
/// ## Common animated widgets
///
/// A number of animated widgets ship with the framework. They are usually named
/// `FooTransition`, where `Foo` is the name of the non-animated
/// version of that widget. The subclasses of this class should not be confused
/// with subclasses of `ImplicitlyAnimatedWidget` (see above), which are usually
/// named `AnimatedFoo`. Commonly used animated widgets include:
///
///  * `ListenableBuilder`, which uses a builder pattern that is useful for
///    complex [`Listenable`] use cases.
///  * `AnimatedBuilder`, which uses a builder pattern that is useful for
///    complex `Animation` use cases.
///  * `AlignTransition`, which is an animated version of `Align`.
///  * `DecoratedBoxTransition`, which is an animated version of `DecoratedBox`.
///  * `DefaultTextStyleTransition`, which is an animated version of
///    `DefaultTextStyle`.
///  * `PositionedTransition`, which is an animated version of `Positioned`.
///  * `RelativePositionedTransition`, which is an animated version of
///    `Positioned`.
///  * `RotationTransition`, which animates the rotation of a widget.
///  * `ScaleTransition`, which animates the scale of a widget.
///  * `SizeTransition`, which animates its own size.
///  * `SlideTransition`, which animates the position of a widget relative to
///    its normal position.
///  * [`FadeTransition`], which is an animated version of `Opacity`.
///  * `AnimatedModalBarrier`, which is an animated version of `ModalBarrier`.
pub trait AnimatedWidget: Debug + 'static {
    /// See [`Widget::key`](crate::Widget::key).
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// The [`Listenable`] to which this widget is listening.
    ///
    /// Commonly an `Animation` or a `ChangeNotifier`.
    ///
    /// Dart compares the old and new widget's listenable by identity; here that is
    /// `Rc::ptr_eq`, so a widget that keeps one `Rc` and clones it into each rebuilt
    /// value keeps its subscription, and one that wraps a handle anew each call is
    /// re-subscribed on every update (the same listener set either way).
    fn listenable(&self) -> Rc<dyn Listenable>;

    /// Override this method to build widgets that depend on the state of the
    /// listenable (e.g., the current value of the animation).
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef;
}

/// The kind tag of [`IntoWidget`] for an [`AnimatedWidget`].
pub struct AnimatedKind;

/// The [`StatefulWidget`] an [`AnimatedWidget`] becomes; its state is Dart's
/// `_AnimatedState`. Subclasses typically do not override `createState`, so this is the
/// one state every animated widget gets.
pub struct Animated<W: AnimatedWidget>(pub W);

impl<W: AnimatedWidget> IntoWidget<AnimatedKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Stateful(Animated(self)))
    }
}

impl<W: AnimatedWidget> StatefulWidget for Animated<W> {
    type State = AnimatedState<W>;

    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_state(&self) -> AnimatedState<W> {
        AnimatedState {
            state: StateData::new(),
        }
    }
}

impl<W: AnimatedWidget> Debug for Animated<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Dart's `_AnimatedState`.
pub struct AnimatedState<W: AnimatedWidget> {
    state: StateData<Animated<W>>,
}

impl<W: AnimatedWidget> State for AnimatedState<W> {
    type Widget = Animated<W>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let listenable = self.widget(app).0.listenable();
        listenable.add_listener(app, Listener::handle_method(self, Self::handle_change));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Animated<W>) {
        let listenable = self.widget(app).0.listenable();
        let old_listenable = old_widget.0.listenable();
        if !Rc::ptr_eq(&listenable, &old_listenable) {
            old_listenable
                .remove_listener(app, &Listener::handle_method(self, Self::handle_change));
            listenable.add_listener(app, Listener::handle_method(self, Self::handle_change));
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let listenable = self.widget(app).0.listenable();
        listenable.remove_listener(app, &Listener::handle_method(self, Self::handle_change));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = context.widget(app).clone();
        downcast_widget::<Animated<W>>(&*widget)
            .expect("an AnimatedState builds its AnimatedWidget")
            .0
            .build(app, context)
    }
}

impl<W: AnimatedWidget> AnimatedState<W> {
    fn handle_change(self: Handle<Self>, app: &mut App) {
        if !self.mounted(app) {
            return;
        }
        self.set_state(app, |_state| {
            // The listenable's state is our build state, and it changed already.
        });
    }
}

/// Animates the opacity of a widget.
///
/// For a widget that automatically animates between the sizes of two children,
/// fading between them, see `AnimatedCrossFade`.
///
/// ## Hit testing
///
/// Setting the [`opacity`](Self::opacity) to zero does not prevent hit testing from being
/// applied to the descendants of the [`FadeTransition`] widget. This can be
/// confusing for the user, who may not see anything, and may believe the area
/// of the interface where the [`FadeTransition`] is hiding a widget to be
/// non-interactive.
///
/// With certain widgets, such as `Flow`, that compute their positions only when
/// they are painted, this can actually lead to bugs (from unexpected geometry
/// to exceptions), because those widgets are not painted by the
/// [`FadeTransition`] widget at all when the [`opacity`](Self::opacity) animation reaches
/// zero.
///
/// To avoid such problems, it is generally a good idea to combine this widget
/// with an `IgnorePointer` that one enables when the [`opacity`](Self::opacity) animation
/// reaches zero. This prevents interactions with any children in the subtree
/// when the [`child`](Self::child) is not visible. For performance reasons, when
/// implementing this, care should be taken not to rebuild the relevant widget (e.g. by
/// calling `State::set_state`) except at the transition point.
///
/// See also:
///
///  * `Opacity`, which does not animate changes in opacity.
///  * `AnimatedOpacity`, which animates changes in opacity without taking an
///    explicit `Animation` argument.
#[derive(Debug)]
pub struct FadeTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the opacity of the child.
    ///
    /// If the current value of the opacity animation is v, the child will be
    /// painted with an opacity of v. For example, if v is 0.5, the child will be
    /// blended 50% with its background. Similarly, if v is 0.0, the child will be
    /// completely transparent.
    pub opacity: AnyAnimation<f64>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl RenderObjectWidget for FadeTransition {
    type RenderObject = RenderAnimatedOpacity;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderAnimatedOpacity::new(app, self.opacity, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderAnimatedOpacity>,
    ) {
        render_object.set_opacity(app, self.opacity);
    }
}

impl SingleChildRenderObjectWidget for FadeTransition {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use reveal_animation::{AnimationBehavior, AnimationController};
    use reveal_embedder::Size;
    use reveal_foundation::{ChangeNotifier, ValueNotifier};
    use reveal_rendering::{BoxConstraints, RenderConstrainedBox, RenderObjectWithChildMixin};
    use reveal_scheduler::{Ticker, TickerCallback, TickerProvider};

    use super::*;
    use crate::framework::LeafRenderObjectWidget;
    use crate::test_harness::Harness;

    /// flutter_test's `TestVSync`: vends ordinary tickers.
    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            Ticker::new(app, on_tick)
        }
    }

    /// `SizedBox` in miniature.
    #[derive(Debug)]
    struct Sized {
        size: Size,
    }

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: RenderHandle<RenderConstrainedBox>,
        ) {
            render_object.set_additional_constraints(app, BoxConstraints::tight(self.size));
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    fn new_controller(app: &mut App) -> Handle<AnimationController> {
        AnimationController::create(
            app,
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    fn fade(opacity: AnyAnimation<f64>) -> WidgetRef {
        FadeTransition {
            key: None,
            opacity,
            child: Some(
                Sized {
                    size: Size::new(10.0, 10.0),
                }
                .into_widget(),
            ),
        }
        .into_widget()
    }

    fn fade_under_root(harness: &Harness, app: &App) -> RenderHandle<RenderAnimatedOpacity> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderAnimatedOpacity>(app)
            .expect("a RenderAnimatedOpacity")
    }

    fn root_child_size(harness: &Harness, app: &App) -> Size {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .size(app)
    }

    // transitions_test.dart 'FadeTransition animates'
    #[test]
    fn a_fade_transition_hands_its_animation_to_the_render_object() {
        let mut app = App::new();
        let controller = new_controller(&mut app);
        let harness = Harness::mount(&mut app, fade(controller.view()));
        harness.pump(&mut app);
        let render_object = fade_under_root(&harness, &app);
        assert_eq!(render_object.opacity(&app), controller.view());
        assert_eq!(render_object.opacity(&app).value(&app), 0.0);
        assert!(
            !render_object.as_object().is_repaint_boundary(&app),
            "fully transparent"
        );

        for value in [0.25, 0.5, 0.75, 1.0] {
            controller.set_value(&mut app, value);
            harness.pump(&mut app);
            assert_eq!(render_object.opacity(&app).value(&app), value);
        }
        assert!(
            render_object.as_object().is_repaint_boundary(&app),
            "the render object heard the controller"
        );

        let other = new_controller(&mut app);
        harness.set_child(&mut app, fade(other.view()));
        harness.pump(&mut app);
        assert_eq!(
            fade_under_root(&harness, &app),
            render_object,
            "reconfigured in place"
        );
        assert_eq!(render_object.opacity(&app), other.view());

        controller.dispose(&mut app);
        other.dispose(&mut app);
    }

    /// An animated widget sized by a notifier's value.
    #[derive(Debug)]
    struct Grower {
        notifier: Handle<ValueNotifier<f64>>,
        builds: Rc<Cell<u32>>,
    }

    impl AnimatedWidget for Grower {
        fn listenable(&self) -> Rc<dyn Listenable> {
            Rc::new(self.notifier)
        }

        fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            let width = *app.get(self.notifier).value();
            Sized {
                size: Size::new(width, 10.0),
            }
            .into_widget()
        }
    }

    fn has_listeners(app: &App, notifier: Handle<ValueNotifier<f64>>) -> bool {
        app.get(notifier).change_notifier_data().has_listeners()
    }

    #[test]
    fn an_animated_widget_rebuilds_when_its_listenable_notifies() {
        let mut app = App::new();
        let notifier = app.create(ValueNotifier::new(10.0));
        let builds = Rc::new(Cell::new(0));
        let harness = Harness::mount(
            &mut app,
            Grower {
                notifier,
                builds: Rc::clone(&builds),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1);
        assert!(has_listeners(&app, notifier));
        assert_eq!(root_child_size(&harness, &app), Size::new(10.0, 10.0));

        notifier.set_value(&mut app, 20.0);
        assert_eq!(builds.get(), 1, "rebuilt at the next frame, not inline");
        harness.pump(&mut app);
        assert_eq!(builds.get(), 2);
        assert_eq!(root_child_size(&harness, &app), Size::new(20.0, 10.0));
    }

    #[test]
    fn an_animated_widget_follows_a_new_listenable_and_unsubscribes_on_unmount() {
        let mut app = App::new();
        let first = app.create(ValueNotifier::new(10.0));
        let second = app.create(ValueNotifier::new(30.0));
        let builds = Rc::new(Cell::new(0));
        let grower = |notifier| {
            Grower {
                notifier,
                builds: Rc::clone(&builds),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, grower(first));
        harness.pump(&mut app);

        harness.set_child(&mut app, grower(second));
        harness.pump(&mut app);
        assert!(
            !has_listeners(&app, first),
            "did_update_widget left the old listenable"
        );
        assert!(has_listeners(&app, second));
        assert_eq!(root_child_size(&harness, &app), Size::new(30.0, 10.0));

        harness.set_child(
            &mut app,
            Sized {
                size: Size::new(5.0, 5.0),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert!(!has_listeners(&app, second), "dispose removed the listener");
        let builds_before = builds.get();
        second.set_value(&mut app, 40.0);
        harness.pump(&mut app);
        assert_eq!(
            builds.get(),
            builds_before,
            "a disposed state never builds again"
        );
    }
}
