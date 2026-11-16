//! Flutter counterpart: `widgets/page_transitions_builder.dart`.
//!
//! Dart's `buildTransitions<T>(PageRoute<T> route, ..)` takes the route erased, as an
//! [`AnyPageRoute`]: routes carry no result type (`navigator.rs`), and a builder is held as an
//! `Rc<dyn PageTransitionsBuilder>`, which a generic method could not be called through.

use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationRef, Cubic, Curve, CurvedAnimation, Curves, Drive, ValueCurveTween,
    ValueTween,
};
use waft_embedder::{Color, Offset, Size};
use waft_foundation::{App, Handle, Listenable, MergingListenable, OptionRetained};
use waft_painting::AlignmentGeometry;

use crate::framework::{
    BuildContext, IntoWidget, State, StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{
    Align, ClipRect, ColoredBox, FractionalTranslation, OverflowBox, SizedBox,
};
use crate::widgets::layout_builder::LayoutBuilder;
use crate::widgets::pages::AnyPageRoute;
use crate::widgets::transitions::{
    AnimatedBuilder, DelegatedTransitionBuilder, FadeTransition, SlideTransition,
};
use waft_foundation::Ref;

/// Defines a page transition animation for a [`PageRoute`](crate::PageRoute).
///
/// PageTransitionsBuilder can be used directly with widget layer primitives in any design
/// system. Custom [`PageRoute`](crate::PageRoute) types can accept a PageTransitionsBuilder
/// and delegate to its [`build_transitions`](Self::build_transitions) method when overriding
/// [`ModalRoute::build_transitions`](crate::ModalRoute::build_transitions). This enables
/// reusable transition animations that work with `Navigator` and other navigation primitives.
///
/// As an example of usage, in the Material library this trait is used by
/// `PageTransitionsTheme` to define a `MaterialPageRoute` page transition animation. Apps can
/// configure the map of builders for `ThemeData.pageTransitionsTheme` to customize the
/// default `MaterialPageRoute` page transition animation for different platforms.
///
/// See also:
///
///  * `PageTransitionsTheme`, which uses this trait to configure page transitions.
///  * `MaterialPageRoute`, which uses this trait to build its transition.
///  * [`FadeUpwardsPageTransitionsBuilder`], which defines a page transition that's similar to
///    the one provided by Android O.
///  * [`OpenUpwardsPageTransitionsBuilder`], which defines a page transition that's similar to
///    the one provided by Android P.
///  * `ZoomPageTransitionsBuilder`, which defines the default page transition that's similar
///    to the one provided in Android Q.
///  * `CupertinoPageTransitionsBuilder`, which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * `FadeForwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android U.
pub trait PageTransitionsBuilder: Debug + 'static {
    /// Provides a secondary transition to the previous route.
    ///
    /// See [`ModalRoute::delegated_transition`](crate::ModalRoute::delegated_transition).
    fn delegated_transition(&self) -> Option<DelegatedTransitionBuilder> {
        None
    }

    /// See [`TransitionRoute::transition_duration`](crate::TransitionRoute::transition_duration).
    ///
    /// Defaults to 300 milliseconds.
    fn transition_duration(&self) -> Duration {
        Duration::from_millis(300)
    }

    /// See
    /// [`TransitionRoute::reverse_transition_duration`](crate::TransitionRoute::reverse_transition_duration).
    ///
    /// Defaults to 300 milliseconds.
    fn reverse_transition_duration(&self) -> Duration {
        self.transition_duration()
    }

    /// Wraps the child with one or more transition widgets which define how `route` arrives on
    /// and leaves the screen.
    ///
    /// Implementors override this method to create a transition animation.
    ///
    /// The `MaterialPageRoute.buildTransitions` method is an example of a method that uses
    /// this to build a transition. It looks up the current `PageTransitionsTheme` with
    /// `Theme.of(context).pageTransitionsTheme` and delegates to this method with a
    /// [`PageTransitionsBuilder`] based on the theme's `ThemeData.platform`.
    fn build_transitions(
        &self,
        app: &mut App,
        route: AnyPageRoute,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef;
}

/// A [`PageTransitionsBuilder`] held by value: what a Dart field, parameter or map value typed
/// `PageTransitionsBuilder` becomes.
///
/// It compares by identity, as Dart's `==` on a builder does, so that a theme holding builders
/// can compare them.
#[derive(Clone)]
pub struct PageTransitionsBuilderRef(Rc<dyn PageTransitionsBuilder>);

impl PageTransitionsBuilderRef {
    /// Erases a [`PageTransitionsBuilder`].
    pub fn new(builder: impl PageTransitionsBuilder) -> PageTransitionsBuilderRef {
        PageTransitionsBuilderRef(Rc::new(builder))
    }
}

impl Deref for PageTransitionsBuilderRef {
    type Target = dyn PageTransitionsBuilder;

    fn deref(&self) -> &dyn PageTransitionsBuilder {
        &*self.0
    }
}

impl PartialEq for PageTransitionsBuilderRef {
    fn eq(&self, other: &PageTransitionsBuilderRef) -> bool {
        // Dart's `==` on a `PageTransitionsBuilder` is object identity.
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Debug for PageTransitionsBuilderRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

// Slides the page upwards and fades it in, starting from 1/4 screen below the top. The
// transition is intended to match the default for Android O.
struct FadeUpwardsPageTransition {
    position_animation: AnimationRef<Offset>,
    opacity_animation: AnimationRef<f64>,
    child: WidgetRef,
}

impl FadeUpwardsPageTransition {
    /// `route_animation` is the route's linear 0.0 - 1.0 animation.
    fn new(
        app: &mut App,
        route_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> FadeUpwardsPageTransition {
        let position_animation = route_animation.drive(
            app,
            FadeUpwardsPageTransition::BOTTOM_UP_TWEEN
                .chain(FadeUpwardsPageTransition::fast_out_slow_in_tween()),
        );
        let opacity_animation =
            route_animation.drive(app, FadeUpwardsPageTransition::ease_in_tween());
        FadeUpwardsPageTransition {
            position_animation,
            opacity_animation,
            child,
        }
    }

    // Fractional offset from 1/4 screen below the top to fully on screen.
    const BOTTOM_UP_TWEEN: ValueTween<Offset> =
        ValueTween::new(Offset::new(0.0, 0.25), Offset::ZERO);

    fn fast_out_slow_in_tween() -> ValueCurveTween {
        ValueCurveTween::new(Curves::fast_out_slow_in())
    }

    fn ease_in_tween() -> ValueCurveTween {
        ValueCurveTween::new(Curves::ease_in())
    }
}

impl Debug for FadeUpwardsPageTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_FadeUpwardsPageTransition")
    }
}

impl StatelessWidget for FadeUpwardsPageTransition {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        SlideTransition::new(self.position_animation.clone())
            // TODO(ianh): tell the transform to be un-transformed for hit testing
            .child(FadeTransition::new(self.opacity_animation.clone()).child(self.child.clone()))
            .into_widget()
    }
}

/// A page transition builder that animates incoming pages by fading and sliding them upwards.
///
/// This transition combines two animations:
/// - A fade-in effect using an ease-in curve
/// - An upward slide starting from 25% below the top of the screen
///
/// The resulting animation creates a smooth entrance effect where the new page appears to rise
/// up while simultaneously becoming visible.
///
/// See also:
///
///  * [`OpenUpwardsPageTransitionsBuilder`], which defines a page transition that's similar to
///    the one provided by Android P.
///  * `ZoomPageTransitionsBuilder`, which defines the default page transition that's similar
///    to the one provided in Android Q.
///  * `CupertinoPageTransitionsBuilder`, which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * `PredictiveBackPageTransitionsBuilder`, which defines a page transition that allows
///    peeking behind the current route on Android U and above.
#[derive(Clone, Copy, Debug, Default)]
pub struct FadeUpwardsPageTransitionsBuilder;

impl FadeUpwardsPageTransitionsBuilder {
    /// Constructs a page transition animation that slides the page up.
    pub const fn new() -> FadeUpwardsPageTransitionsBuilder {
        FadeUpwardsPageTransitionsBuilder
    }
}

impl PageTransitionsBuilder for FadeUpwardsPageTransitionsBuilder {
    fn build_transitions(
        &self,
        app: &mut App,
        _route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        _secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        FadeUpwardsPageTransition::new(app, animation, child).into_widget()
    }
}

// This transition is intended to match the default for Android P.
struct OpenUpwardsPageTransition {
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    child: WidgetRef,
}

impl OpenUpwardsPageTransition {
    // The new page slides upwards just a little as its clip rectangle exposes the page from
    // bottom to top.
    const PRIMARY_TRANSLATION_TWEEN: ValueTween<Offset> =
        ValueTween::new(Offset::new(0.0, 0.05), Offset::ZERO);

    // The old page slides upwards a little as the new page appears.
    const SECONDARY_TRANSLATION_TWEEN: ValueTween<Offset> =
        ValueTween::new(Offset::ZERO, Offset::new(0.0, -0.025));

    // The scrim color that obscures the old page during the transition.
    const SCRIM_COLOR: Color = Color::new(0xFF000000);

    // The scrim obscures the old page by becoming increasingly opaque.
    const SCRIM_OPACITY_TWEEN: ValueTween<f64> = ValueTween::new(0.0, 0.25);

    // Used by all of the transition animations.
    const TRANSITION_CURVE: Cubic = Cubic::new(0.20, 0.00, 0.00, 1.00);

    fn new(
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> OpenUpwardsPageTransition {
        OpenUpwardsPageTransition {
            animation,
            secondary_animation,
            child,
        }
    }

    fn transition_curve() -> Rc<dyn Curve> {
        Rc::new(OpenUpwardsPageTransition::TRANSITION_CURVE)
    }
}

impl Debug for OpenUpwardsPageTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_OpenUpwardsPageTransition")
    }
}

impl StatefulWidget for OpenUpwardsPageTransition {
    type State = OpenUpwardsPageTransitionState;

    fn create_state(&self) -> OpenUpwardsPageTransitionState {
        OpenUpwardsPageTransitionState {
            state: StateData::new(),
            primary_animation: None,
            secondary_translation_curved_animation: None,
        }
    }
}

/// Dart's `_OpenUpwardsPageTransitionState`.
struct OpenUpwardsPageTransitionState {
    state: StateData<OpenUpwardsPageTransition>,
    primary_animation: Option<Ref<CurvedAnimation>>,
    secondary_translation_curved_animation: Option<Ref<CurvedAnimation>>,
}

impl OpenUpwardsPageTransitionState {
    fn set_animations(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let animation = widget.animation.clone();
        let secondary_animation = widget.secondary_animation.clone();
        let primary_animation = CurvedAnimation::new(
            app,
            animation,
            OpenUpwardsPageTransition::transition_curve(),
            Some(Rc::new(
                OpenUpwardsPageTransition::transition_curve().flipped(),
            )),
        );
        let secondary_translation_curved_animation = CurvedAnimation::new(
            app,
            secondary_animation,
            OpenUpwardsPageTransition::transition_curve(),
            Some(Rc::new(
                OpenUpwardsPageTransition::transition_curve().flipped(),
            )),
        );
        let state = app.get_mut(self);
        state.primary_animation = Some(primary_animation);
        state.secondary_translation_curved_animation = Some(secondary_translation_curved_animation);
    }

    fn dispose_animations(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let animations = [
            state.primary_animation.handle(),
            state.secondary_translation_curved_animation.handle(),
        ];
        for animation in animations.into_iter().flatten() {
            animation.dispose(app);
        }
    }

    /// The scrim, the clip and the two translations over the size the page is given.
    fn build_for_constraints(self: Handle<Self>, app: &mut App, size: Size) -> WidgetRef {
        let state = app.get(self);
        let primary_animation: AnimationRef<f64> = state
            .primary_animation
            .as_ref()
            .expect("init_state ran first")
            .erase();
        let secondary_translation_curved_animation: AnimationRef<f64> = state
            .secondary_translation_curved_animation
            .as_ref()
            .expect("init_state ran first")
            .erase();

        // Gradually expose the new page from bottom to top.
        let clip_animation =
            ValueTween::new(0.0, size.height()).animate(app, primary_animation.clone());

        let opacity_animation =
            OpenUpwardsPageTransition::SCRIM_OPACITY_TWEEN.animate(app, primary_animation.clone());
        let primary_translation_animation =
            OpenUpwardsPageTransition::PRIMARY_TRANSLATION_TWEEN.animate(app, primary_animation);

        let secondary_translation_animation =
            OpenUpwardsPageTransition::SECONDARY_TRANSLATION_TWEEN
                .animate(app, secondary_translation_curved_animation);

        let widget = self.widget(app);
        let animation: Rc<dyn Listenable> = Rc::new(widget.animation.clone());
        let secondary_animation: Rc<dyn Listenable> = Rc::new(widget.secondary_animation.clone());
        let merged = Rc::new(MergingListenable::new(vec![
            Some(animation),
            Some(secondary_animation),
        ]));
        AnimatedBuilder::new(merged, move |app, _context, _child| {
            // Dart's `withOpacity`, deprecated there too: its rounding to an 8-bit alpha is
            // part of the behaviour being ported.
            #[allow(deprecated)]
            let scrim_color =
                OpenUpwardsPageTransition::SCRIM_COLOR.with_opacity(opacity_animation.value(app));
            ColoredBox::new(scrim_color)
                .child(
                    Align::new()
                        .alignment(AlignmentGeometry::BOTTOM_LEFT)
                        .child(
                            ClipRect::new().child(
                                SizedBox::new().height(clip_animation.value(app)).child(
                                    OverflowBox::new()
                                        .alignment(AlignmentGeometry::BOTTOM_LEFT)
                                        .max_height(size.height())
                                        .child(
                                            FractionalTranslation::new(
                                                secondary_translation_animation.value(app),
                                            )
                                            .child(
                                                FractionalTranslation::new(
                                                    primary_translation_animation.value(app),
                                                )
                                                .child(self.widget(app).child.clone()),
                                            ),
                                        ),
                                ),
                            ),
                        ),
                )
                .into_widget()
        })
        .into_widget()
    }
}

impl State for OpenUpwardsPageTransitionState {
    type Widget = OpenUpwardsPageTransition;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.set_animations(app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &OpenUpwardsPageTransition,
    ) {
        let widget = self.widget(app);
        if old_widget.animation != widget.animation
            || old_widget.secondary_animation != widget.secondary_animation
        {
            self.dispose_animations(app);
            self.set_animations(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_animations(app);
    }

    fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
        LayoutBuilder::new(move |app, _context, constraints| {
            let size = constraints.biggest();
            self.build_for_constraints(app, size)
        })
        .into_widget()
    }
}

/// A page transition builder that animates incoming pages by revealing them from bottom to
/// top with a clipping effect.
///
/// This transition combines several animations:
/// - A clip animation that gradually reveals the new page from bottom to top
/// - A subtle upward slide of the new page (5% translation)
/// - A slight upward movement of the old page (2.5% translation)
/// - A darkening scrim effect on the background
///
/// The resulting animation creates a layered effect where the new page appears to open upward
/// from the bottom of the screen while the previous page slides back slightly.
///
/// See also:
///
///  * [`FadeUpwardsPageTransitionsBuilder`], which defines a page transition that's similar to
///    the one provided by Android O.
///  * `ZoomPageTransitionsBuilder`, which defines the default page transition that's similar
///    to the one provided in Android Q.
///  * `CupertinoPageTransitionsBuilder`, which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * `PredictiveBackPageTransitionsBuilder`, which defines a page transition that allows
///    peeking behind the current route on Android.
///  * `FadeForwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android U.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenUpwardsPageTransitionsBuilder;

impl OpenUpwardsPageTransitionsBuilder {
    /// Constructs a page transition animation that matches the transition used on Android P.
    pub const fn new() -> OpenUpwardsPageTransitionsBuilder {
        OpenUpwardsPageTransitionsBuilder
    }
}

impl PageTransitionsBuilder for OpenUpwardsPageTransitionsBuilder {
    fn build_transitions(
        &self,
        _app: &mut App,
        _route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        OpenUpwardsPageTransition::new(animation, secondary_animation, child).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use waft_animation::AnimationRef;
    use waft_animation::Drive;

    use waft_embedder::TextDirection;
    use waft_painting::TextStyle;

    use super::*;
    use crate::framework::GlobalKey;
    use crate::testing::{WidgetTester, test_widgets};
    use crate::widgets::basic::{Center, Directionality};
    use crate::widgets::navigator::{Navigator, NavigatorState, RouteRef, RouteSettings};
    use crate::widgets::pages::{PageRoute, PageRouteBuilder};
    use crate::widgets::text::Text;
    use crate::widgets::transitions::ScaleTransition;

    /// The test's `_CustomPageRoute`: a page route whose transitions are `transitions_builder`'s,
    /// handed the route itself.
    fn custom_page_route(
        app: &mut App,
        settings: &RouteSettings,
        transitions_builder: PageTransitionsBuilderRef,
        builder: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static,
    ) -> RouteRef {
        let route = PageRouteBuilder::new(
            app,
            Rc::new(move |app, context, _animation, _secondary_animation| builder(app, context)),
        )
        .settings(app, settings.clone().into());
        // The transitions builder lives on the route: it names the route without keeping it.
        let page_route = PageRoute::as_page_route(route.handle());
        let route = route.transitions_builder(
            app,
            Rc::new(move |app, context, animation, secondary_animation, child| {
                transitions_builder.build_transitions(
                    app,
                    page_route,
                    context,
                    animation,
                    secondary_animation,
                    child,
                )
            }),
        );
        route.erase()
    }

    /// A navigator under the `Directionality` its overlay needs, building `route` for every
    /// name.
    fn navigator(
        key: &GlobalKey,
        route: impl Fn(&mut App, &RouteSettings) -> RouteRef + 'static,
    ) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Navigator::new()
                .key(key.clone().into())
                .on_generate_route(move |app, settings| Some(route(app, settings))),
        )
        .into_widget()
    }

    fn page_text(key: &GlobalKey, text: &str) -> WidgetRef {
        Text::new(text)
            .key(key.clone().into())
            .style(
                TextStyle::new()
                    .color(Color::new(0xFFFFFFFF))
                    .font_size(24.0),
            )
            .into_widget()
    }

    /// The test's two pages, `Page 1` on blue for `/` and `Page 2` on green for the rest, with
    /// their texts under `page_1` and `page_2`.
    fn two_pages(
        page_1: &GlobalKey,
        page_2: &GlobalKey,
        settings: &RouteSettings,
    ) -> impl Fn(&mut App, BuildContext) -> WidgetRef + 'static {
        let first = settings.name.as_deref() == Some("/");
        let page_1 = page_1.clone();
        let page_2 = page_2.clone();
        move |_app, _context| {
            if first {
                ColoredBox::new(Color::new(0xFF2196F3))
                    .child(Center::new().child(page_text(&page_1, "Page 1")))
                    .into_widget()
            } else {
                ColoredBox::new(Color::new(0xFF4CAF50))
                    .child(Center::new().child(page_text(&page_2, "Page 2")))
                    .into_widget()
            }
        }
    }

    fn state(key: &GlobalKey, tester: &WidgetTester) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(&tester.cell.borrow())
            .expect("a mounted navigator")
    }

    /// Dart's `tester.getTopLeft(find.byKey(key))`.
    fn top_left(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        let render = tester.render_object(key).as_box().expect("a box");
        tester.global_rect(render).top_left()
    }

    /// How many onstage texts show `text`: Dart's `find.text(text)`, which skips offstage ones.
    fn shown(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    /// The opacity of the `FadeTransition` nearest above the widget under `key`.
    fn opacity_above(tester: &WidgetTester, key: &GlobalKey) -> f64 {
        let app = tester.cell.borrow();
        let context = key.current_context(&app).expect("the widget is mounted");
        let opacity = context
            .find_ancestor_widget_of_exact_type::<FadeTransition>(&app)
            .expect("a FadeTransition above")
            .opacity
            .handle();
        opacity.value(&app)
    }

    /// The test's `onBuildTransitions` signature: [`PageTransitionsBuilder::build_transitions`]'s.
    type BuildTransitions = dyn Fn(
        &mut App,
        AnyPageRoute,
        BuildContext,
        AnimationRef<f64>,
        AnimationRef<f64>,
        WidgetRef,
    ) -> WidgetRef;

    /// The test's `_TestPageTransitionsBuilder`.
    struct TestPageTransitionsBuilder {
        on_build_transitions: Rc<BuildTransitions>,
    }

    impl Debug for TestPageTransitionsBuilder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("_TestPageTransitionsBuilder")
        }
    }

    impl PageTransitionsBuilder for TestPageTransitionsBuilder {
        fn build_transitions(
            &self,
            app: &mut App,
            route: AnyPageRoute,
            context: BuildContext,
            animation: AnimationRef<f64>,
            secondary_animation: AnimationRef<f64>,
            child: WidgetRef,
        ) -> WidgetRef {
            (self.on_build_transitions)(app, route, context, animation, secondary_animation, child)
        }
    }

    /// The opacity and the scale animation of one route's transition.
    type RouteTransition = (AnimationRef<f64>, AnimationRef<f64>);

    #[test]
    fn page_transitions_builder_works_with_custom_navigator_and_page_route() {
        test_widgets(|tester| {
            // The opacity and scale animations of each route's transition, by route name.
            let transitions: Rc<RefCell<HashMap<String, RouteTransition>>> = Rc::default();
            let custom_transitions_builder =
                PageTransitionsBuilderRef::new(TestPageTransitionsBuilder {
                    on_build_transitions: Rc::new({
                        let transitions = Rc::clone(&transitions);
                        move |app, route, _context, animation, _secondary_animation, child| {
                            let curve_tween =
                                waft_animation::CurveTween::new(app, Curves::ease_in_out());
                            let scale_tween = waft_animation::Tween::new(app, Some(0.5), Some(1.0));
                            let scale = animation.drive(app, scale_tween.chain(curve_tween));
                            let name = route.as_route().settings(app).name().map(str::to_string);
                            transitions.borrow_mut().insert(
                                name.expect("a named route"),
                                (animation.clone(), scale.clone()),
                            );
                            FadeTransition::new(animation)
                                .child(ScaleTransition::new(scale).child(child))
                                .into_widget()
                        }
                    }),
                });
            let key = GlobalKey::new();
            let navigate = GlobalKey::new();
            let second_page = GlobalKey::new();
            tester.pump_widget(navigator(&key, {
                let navigate = navigate.clone();
                let second_page = second_page.clone();
                move |app, settings| {
                    let first = settings.name.as_deref() == Some("/");
                    let navigate = navigate.clone();
                    let second_page = second_page.clone();
                    custom_page_route(
                        app,
                        settings,
                        custom_transitions_builder.clone(),
                        move |_app, _context| {
                            if first {
                                Center::new()
                                    .child(SizedBox::new().width(200.0).height(50.0).child(
                                        ColoredBox::new(Color::new(0xFF2196F3)).child(
                                            Center::new().child(page_text(&navigate, "Navigate")),
                                        ),
                                    ))
                                    .into_widget()
                            } else {
                                ColoredBox::new(Color::new(0xFF4CAF50))
                                    .child(
                                        Center::new().child(page_text(&second_page, "Second Page")),
                                    )
                                    .into_widget()
                            }
                        },
                    )
                }
            }));
            assert_eq!(shown(tester, "Navigate"), 1);
            assert_eq!(shown(tester, "Second Page"), 0);

            // Flutter taps the "Navigate" box, whose handler pushes `/second`.
            let navigator_state = state(&key, tester);
            tester.update(|app| navigator_state.push_named(app, "/second", None));
            tester.pump();

            assert_eq!(shown(tester, "Navigate"), 1);
            assert_eq!(shown(tester, "Second Page"), 1);

            tester.pump_by(Duration::from_millis(50));

            // Each route was handed to `build_transitions` as itself (the first Flutter case's
            // `capturedRoute!.settings.name`).
            let mut names: Vec<String> = transitions.borrow().keys().cloned().collect();
            names.sort();
            assert_eq!(names, ["/", "/second"]);
            {
                let app = tester.cell.borrow();
                for (opacity, scale) in transitions.borrow().values() {
                    let opacity = opacity.value(&app);
                    assert!(opacity > 0.0 && opacity <= 1.0, "opacity {opacity}");
                    let scale = scale.value(&app);
                    assert!((0.5..=1.0).contains(&scale), "scale {scale}");
                }
            }

            tester.pump_and_settle();

            assert_eq!(shown(tester, "Navigate"), 0);
            assert_eq!(shown(tester, "Second Page"), 1);
        });
    }

    /// The test's navigator whose two pages, `Page 1` and `Page 2`, take their transitions from
    /// `builder`, pumped; answers the navigator's state.
    fn pump_two_pages(
        tester: &mut WidgetTester,
        builder: PageTransitionsBuilderRef,
        page_1: &GlobalKey,
        page_2: &GlobalKey,
    ) -> Handle<NavigatorState> {
        let key = GlobalKey::new();
        tester.pump_widget(navigator(&key, {
            let page_1 = page_1.clone();
            let page_2 = page_2.clone();
            move |app, settings| {
                let pages = two_pages(&page_1, &page_2, settings);
                custom_page_route(app, settings, builder.clone(), pages)
            }
        }));
        state(&key, tester)
    }

    #[test]
    fn fade_upwards_page_transitions_builder_test() {
        test_widgets(|tester| {
            let page_1 = GlobalKey::new();
            let page_2 = GlobalKey::new();
            let builder = PageTransitionsBuilderRef::new(FadeUpwardsPageTransitionsBuilder::new());
            let navigator_state = pump_two_pages(tester, builder, &page_1, &page_2);
            let widget_1_top_left = top_left(tester, &page_1);

            // Flutter taps "Page 1", whose handler pushes `/second`.
            tester.update(|app| navigator_state.push_named(app, "/second", None));
            tester.pump();
            tester.pump_by(Duration::from_millis(1));

            let widget_2_opacity = opacity_above(tester, &page_2);
            let widget_2_top_left = top_left(tester, &page_2);

            assert!(widget_1_top_left.dx() == widget_2_top_left.dx());
            assert!(widget_1_top_left.dy() < widget_2_top_left.dy());
            assert!(widget_2_opacity < 0.01);

            tester.pump_by(Duration::from_millis(300));

            assert_eq!(shown(tester, "Page 1"), 0);
            assert_eq!(shown(tester, "Page 2"), 1);

            tester.update(|app| navigator_state.pop(app, None));
            tester.pump();
            tester.pump_by(Duration::from_millis(1));

            let widget_2_opacity = opacity_above(tester, &page_2);
            let widget_2_top_left = top_left(tester, &page_2);

            assert!(widget_1_top_left.dy() < widget_2_top_left.dy());
            assert!(widget_2_opacity < 1.0);

            tester.pump_by(Duration::from_millis(300));

            assert_eq!(shown(tester, "Page 1"), 1);
            assert_eq!(shown(tester, "Page 2"), 0);
        });
    }

    /// Checks that a frame 100 ms into a push with `builder` leaves no new arena objects.
    fn assert_a_mid_push_frame_leaves_nothing(builder: fn() -> PageTransitionsBuilderRef) {
        test_widgets(|tester| {
            let page_1 = GlobalKey::new();
            let page_2 = GlobalKey::new();
            let navigator_state = pump_two_pages(tester, builder(), &page_1, &page_2);
            tester.update(|app| navigator_state.push_named(app, "/second", None));
            tester.pump();
            tester.pump_by(Duration::from_millis(100));
            tester.assert_leaves_nothing_behind(|tester| tester.pump_by(Duration::from_millis(16)));
        });
    }

    /// Not a Dart case: the animations each frame's `_FadeUpwardsPageTransition` drives go with
    /// the last transition that reads them, as Dart's collector frees them.
    #[test]
    fn a_frame_of_a_fade_upwards_push_leaves_no_new_arena_objects() {
        assert_a_mid_push_frame_leaves_nothing(|| {
            PageTransitionsBuilderRef::new(FadeUpwardsPageTransitionsBuilder::new())
        });
    }

    /// Not a Dart case: the animations each layout of `_OpenUpwardsPageTransition` drives go
    /// with the last builder that reads them, as Dart's collector frees them.
    #[test]
    fn a_frame_of_an_open_upwards_push_leaves_no_new_arena_objects() {
        assert_a_mid_push_frame_leaves_nothing(|| {
            PageTransitionsBuilderRef::new(OpenUpwardsPageTransitionsBuilder::new())
        });
    }

    #[test]
    fn open_upwards_page_transitions_builder_test() {
        test_widgets(|tester| {
            let page_1 = GlobalKey::new();
            let page_2 = GlobalKey::new();
            let builder = PageTransitionsBuilderRef::new(OpenUpwardsPageTransitionsBuilder::new());
            let navigator_state = pump_two_pages(tester, builder, &page_1, &page_2);
            let widget_1_top_left = top_left(tester, &page_1);

            // Flutter taps "Page 1", whose handler pushes `/second`.
            tester.update(|app| navigator_state.push_named(app, "/second", None));
            tester.pump();
            tester.pump_by(Duration::from_millis(1));

            assert_eq!(shown(tester, "Page 1"), 1);
            assert_eq!(shown(tester, "Page 2"), 1);

            let widget_2_top_left = top_left(tester, &page_2);

            assert_eq!(widget_1_top_left.dx(), widget_2_top_left.dx());
            assert!(widget_1_top_left.dy() <= widget_2_top_left.dy());

            tester.pump_by(Duration::from_millis(300));

            // After animation, only Page 2 should be visible.
            assert_eq!(shown(tester, "Page 1"), 0);
            assert_eq!(shown(tester, "Page 2"), 1);

            tester.update(|app| navigator_state.pop(app, None));
            tester.pump();
            tester.pump_by(Duration::from_millis(1));

            assert_eq!(shown(tester, "Page 1"), 1);
            assert_eq!(shown(tester, "Page 2"), 1);

            tester.pump_by(Duration::from_millis(300));

            // After reverse animation, only Page 1 should be visible.
            assert_eq!(shown(tester, "Page 1"), 1);
            assert_eq!(shown(tester, "Page 2"), 0);
        });
    }
}
