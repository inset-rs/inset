//! Flutter counterpart: `widgets/heroes.dart`.
//!
//! [`Hero`] and the [`HeroController`] that flies matching heroes between two [`PageRoute`]s
//! in the navigator's overlay.
//!
//! [`PageRoute`]: crate::PageRoute

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexMap;
use inset_animation::{
    Animatable, Animation, AnimationStatus, AnimationStatusListener, AnyAnimation, Curve,
    CurveTween, CurvedAnimation, Curves, Interval, ProxyAnimation, RectTween, ReverseAnimation,
    Tween, k_always_complete_animation,
};
use inset_embedder::{Offset, Rect, Size};
use inset_foundation::{App, Handle, ListenableObject, Listener};
use inset_painting::transform_rect;
use inset_scheduler::{FrameCallback, SchedulerBinding};

use crate::framework::{
    AnyElement, BuildContext, GlobalKey, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef, downcast_widget,
};
use crate::widgets::basic::{IgnorePointer, KeyedSubtree, Offstage, Positioned, SizedBox};
use crate::widgets::implicit_animations::EdgeInsetsTween;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::navigator::{
    AnyRoute, Navigator, NavigatorObserver, NavigatorObserverData, NavigatorState,
};
use crate::widgets::overlay::{OverlayEntry, OverlayState};
use crate::widgets::pages::AnyPageRoute;
use crate::widgets::routes::AnyModalRoute;
use crate::widgets::ticker_provider::TickerMode;
use crate::widgets::transitions::{AnimatedBuilder, FadeTransition};

// ---------------------------------------------------------------------------------------------
// Tween<Rect?>

/// Dart's `Tween<Rect?>`: the tween a [`CreateRectTween`] returns and a hero flight animates
/// the hero's bounds with.
///
/// Rust has no subtyping, so a flight holds this as `Rc<dyn RectTweenObject>`; inset-animation's
/// `RectTween` — Dart's default — implements it, and so does any tween a caller supplies.
pub trait RectTweenObject: 'static {
    /// The value this variable has at the beginning of the animation.
    fn begin(&self, app: &App) -> Option<Rect>;

    /// The value this variable has at the end of the animation.
    fn end(&self, app: &App) -> Option<Rect>;

    /// Returns the value this variable has at the given animation clock value.
    fn lerp(&self, app: &App, t: f64) -> Option<Rect>;

    /// Returns the value this variable has at the given animation clock value, snapping to
    /// [`begin`](Self::begin) and [`end`](Self::end) at the ends of the interval.
    fn transform(&self, app: &App, t: f64) -> Option<Rect> {
        if t == 0.0 {
            return self.begin(app);
        }
        if t == 1.0 {
            return self.end(app);
        }
        self.lerp(app, t)
    }

    /// The current value of this object for the given animation.
    fn evaluate(&self, app: &App, animation: AnyAnimation<f64>) -> Option<Rect> {
        self.transform(app, animation.value(app))
    }
}

impl RectTweenObject for Handle<RectTween> {
    fn begin(&self, app: &App) -> Option<Rect> {
        app.get(*self).begin
    }

    fn end(&self, app: &App) -> Option<Rect> {
        app.get(*self).end
    }

    fn lerp(&self, app: &App, t: f64) -> Option<Rect> {
        RectTween::lerp(*self, app, t)
    }
}

/// Dart's `ReverseTween<Rect?>`: a tween that evaluates its parent in reverse.
///
/// inset-animation's `ReverseTween` wraps a concrete `Tween<T>`, which the erased tween a
/// [`CreateRectTween`] returns is not.
struct ReverseRectTween {
    parent: Rc<dyn RectTweenObject>,
}

impl RectTweenObject for ReverseRectTween {
    fn begin(&self, app: &App) -> Option<Rect> {
        self.parent.end(app)
    }

    fn end(&self, app: &App) -> Option<Rect> {
        self.parent.begin(app)
    }

    fn lerp(&self, app: &App, t: f64) -> Option<Rect> {
        self.parent.lerp(app, 1.0 - t)
    }
}

/// Signature for a function that takes two [`Rect`] instances and returns a `RectTween` that
/// transitions between them.
///
/// This is typically used with a [`HeroController`] to provide an animation for [`Hero`]
/// positions that looks nicer than a linear movement. For example, see `MaterialRectArcTween`.
pub type CreateRectTween =
    Rc<dyn Fn(&mut App, Option<Rect>, Option<Rect>) -> Rc<dyn RectTweenObject>>;

/// Signature for a function that builds a [`Hero`] placeholder widget given a child and a
/// [`Size`].
///
/// The child can optionally be part of the returned widget tree. The returned widget should
/// typically be constrained to `hero_size`, if it doesn't do so implicitly.
///
/// See also:
///
///  * `TransitionBuilder`, which is similar but only takes a [`BuildContext`] and a child
///    widget.
pub type HeroPlaceholderBuilder = Rc<dyn Fn(&mut App, BuildContext, Size, WidgetRef) -> WidgetRef>;

/// A function that lets [`Hero`]es self supply a widget that is shown during the hero's flight
/// from one route to another instead of default (which is to show the destination route's
/// instance of the hero).
pub type HeroFlightShuttleBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        AnyAnimation<f64>,
        HeroFlightDirection,
        BuildContext,
        BuildContext,
    ) -> WidgetRef,
>;

type OnFlightEnded = Rc<dyn Fn(&mut App, Handle<HeroFlight>)>;

/// Direction of the hero's flight based on the navigation operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroFlightDirection {
    /// A flight triggered by a route push.
    ///
    /// The animation goes from 0 to 1.
    ///
    /// If no custom [`HeroFlightShuttleBuilder`] is supplied, the top route's [`Hero`] child is
    /// shown in flight.
    Push,

    /// A flight triggered by a route pop.
    ///
    /// The animation goes from 1 to 0.
    ///
    /// If no custom [`HeroFlightShuttleBuilder`] is supplied, the bottom route's [`Hero`] child
    /// is shown in flight.
    Pop,
}

// ---------------------------------------------------------------------------------------------
// Hero

/// The identifier a [`Hero`] is matched by: Dart's `Object` tag, compared with `==` and used as
/// a map key.
///
/// The blanket implementation covers every value that can answer both questions, so a tag is
/// written as `Rc::new("photo")` or `Rc::new(42)`. Two tags of different concrete types never
/// match, as two Dart objects of different classes do not.
pub trait HeroTag: Any + Debug {
    /// Dart's `operator ==`. Must return false when `other` is a different concrete type, even
    /// if the two carry the same value.
    fn eq_tag(&self, other: &dyn HeroTag) -> bool;

    /// Dart's `hashCode`. Must mix in the concrete type so that two types carrying the same
    /// value do not collide.
    fn hash_tag(&self, state: &mut dyn Hasher);
}

impl<T: PartialEq + Hash + Debug + 'static> HeroTag for T {
    fn eq_tag(&self, other: &dyn HeroTag) -> bool {
        (other as &dyn Any)
            .downcast_ref::<T>()
            .is_some_and(|other| other == self)
    }

    fn hash_tag(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<T>().hash(&mut state);
        self.hash(&mut state);
    }
}

impl PartialEq for dyn HeroTag {
    fn eq(&self, other: &dyn HeroTag) -> bool {
        self.eq_tag(other)
    }
}

impl Eq for dyn HeroTag {}

impl Hash for dyn HeroTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_tag(state);
    }
}

/// A shared [`HeroTag`] (Dart's `Object tag` on a [`Hero`]).
pub type HeroTagRef = Rc<dyn HeroTag>;

/// A widget that marks its child as being a candidate for
/// [hero animations](https://docs.flutter.dev/ui/animations/hero-animations).
///
/// When a `PageRoute` is pushed or popped with the [`Navigator`], the entire screen's content is
/// replaced. An old route disappears and a new route appears. If there's a common visual feature
/// on both routes then it can be helpful for orienting the user for the feature to physically
/// move from one page to the other during the routes' transition. Such an animation is called a
/// *hero animation*. The hero widgets "fly" in the navigator's overlay during the transition and
/// while they're in-flight they're, by default, not shown in their original locations in the old
/// and new routes.
///
/// To label a widget as such a feature, wrap it in a [`Hero`] widget. When navigation happens,
/// the [`Hero`] widgets on each route are identified by the [`HeroController`]. For each pair of
/// [`Hero`] widgets that have the same tag, a hero animation is triggered.
///
/// If a [`Hero`] is already in flight when navigation occurs, its flight animation will be
/// redirected to its new destination. The widget shown in-flight during the transition is, by
/// default, the destination route's [`Hero`]'s child.
///
/// For a hero animation to trigger, the hero has to exist on the very first frame of the new
/// page's animation.
///
/// Routes must not contain more than one [`Hero`] for each [`tag`](Self::tag).
///
/// ## Discussion
///
/// Heroes and the [`Navigator`]'s `Overlay` `Stack` must be axis-aligned for all this to work.
/// The top left and bottom right coordinates of each animated hero will be converted to global
/// coordinates and then from there converted to that `Stack`'s coordinate space, and the entire
/// hero subtree will, for the duration of the animation, be lifted out of its original place,
/// and positioned on that stack. If the [`Hero`] isn't axis aligned, this is going to fail in a
/// rather ugly fashion. Don't rotate your heroes!
///
/// To make the animations look good, it's critical that the widget tree for the hero in both
/// locations be essentially identical. The widget of the *target* is, by default, used to do the
/// transition: when going from route A to route B, route B's hero's widget is placed over route
/// A's hero's widget. Additionally, if the [`Hero`] subtree changes appearance based on an
/// `InheritedWidget` (such as [`MediaQuery`] or `Theme`), then the hero animation may have
/// discontinuity at the start or the end of the animation because route A and route B provides
/// different such `InheritedWidget`s. Consider providing a custom
/// [`flight_shuttle_builder`](Self::flight_shuttle_builder) to ensure smooth transitions. The
/// default flight shuttle builder interpolates [`MediaQuery`]'s paddings. If your [`Hero`] widget
/// uses custom `InheritedWidget`s and displays a discontinuity in the animation, try to provide
/// custom in-flight transition using [`flight_shuttle_builder`](Self::flight_shuttle_builder).
///
/// By default, both route A and route B's heroes are hidden while the transitioning widget is
/// animating in-flight above the 2 routes. [`placeholder_builder`](Self::placeholder_builder) can
/// be used to show a custom widget in their place instead once the transition has taken flight.
///
/// During the transition, the transition widget is animated to route B's hero's position, and
/// then the widget is inserted into route B. When going back from B to A, route A's hero's widget
/// is, by default, placed over where route B's hero's widget was, and then the animation goes the
/// other way.
///
/// ### Nested Navigators
///
/// If either or both routes contain nested [`Navigator`]s, only [`Hero`]es contained in the
/// top-most routes (as defined by `Route::is_current`) *of those nested [`Navigator`]s* are
/// considered for animation. Just like in the non-nested case the top-most routes containing
/// these [`Hero`]es in the nested [`Navigator`]s have to be `PageRoute`s.
pub struct Hero {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The identifier for this particular hero. If the tag of this hero matches the tag of a
    /// hero on a `PageRoute` that we're navigating to or from, then a hero animation will be
    /// triggered.
    pub tag: HeroTagRef,
    /// Defines how the destination hero's bounds change as it flies from the starting route to
    /// the destination route.
    ///
    /// A hero flight begins with the destination hero's [`child`](Self::child) aligned with the
    /// starting hero's child. The tween returned by this callback is used to compute the hero's
    /// bounds as the flight animation's value goes from 0.0 to 1.0.
    ///
    /// If this property is `None`, the default, then the value of
    /// [`HeroController::create_rect_tween`] is used.
    pub create_rect_tween: Option<CreateRectTween>,
    /// The widget subtree that will "fly" from one route to another during a [`Navigator`] push
    /// or pop transition.
    ///
    /// The appearance of this subtree should be similar to the appearance of the subtrees of any
    /// other heroes in the application with the same [`tag`](Self::tag). Changes in scale and
    /// aspect ratio work well in hero animations, changes in layout or composition do not.
    pub child: WidgetRef,
    /// Optional override to supply a widget that's shown during the hero's flight.
    ///
    /// This in-flight widget can depend on the route transition's animation as well as the
    /// incoming and outgoing routes' [`Hero`] descendants' widgets and layout.
    ///
    /// When both the source and destination [`Hero`]es provide a
    /// [`flight_shuttle_builder`](Self::flight_shuttle_builder), the destination's takes
    /// precedence.
    ///
    /// If none is provided, the destination route's hero child is shown in-flight by default.
    ///
    /// ## Limitations
    ///
    /// If a widget built by [`flight_shuttle_builder`](Self::flight_shuttle_builder) takes part
    /// in a [`Navigator`] push transition, that widget or its descendants must not have any
    /// `GlobalKey` that is used in the source hero's descendant widgets. That is because both
    /// subtrees will be included in the widget tree during the hero flight animation, and
    /// `GlobalKey`s must be unique across the entire widget tree.
    ///
    /// If the said `GlobalKey` is essential to your application, consider providing a custom
    /// [`placeholder_builder`](Self::placeholder_builder) for the source hero, to avoid the
    /// `GlobalKey` collision, such as a builder that builds an empty [`SizedBox`], keeping the
    /// hero [`child`](Self::child)'s original size.
    pub flight_shuttle_builder: Option<HeroFlightShuttleBuilder>,
    /// Placeholder widget left in place as the hero's [`child`](Self::child) once the flight
    /// takes off.
    ///
    /// By default the placeholder widget is an empty [`SizedBox`] keeping the hero child's
    /// original size, unless this hero is a source hero of a [`Navigator`] push transition, in
    /// which case [`child`](Self::child) will be a descendant of the placeholder and will be kept
    /// [`Offstage`] during the hero's flight.
    pub placeholder_builder: Option<HeroPlaceholderBuilder>,
    /// Whether to perform the hero transition if the `PageRoute` transition was triggered by a
    /// user gesture, such as a back swipe on iOS.
    ///
    /// If [`Hero`]es with the same [`tag`](Self::tag) on both the from and the to routes have
    /// [`transition_on_user_gestures`](Self::transition_on_user_gestures) set to true, a back
    /// swipe gesture will trigger the same hero animation as a programmatically triggered push or
    /// pop.
    ///
    /// The route being popped to or the bottom route must also have `PageRoute::maintain_state`
    /// set to true for a gesture triggered hero transition to work.
    ///
    /// Defaults to false.
    pub transition_on_user_gestures: bool,
    /// The curve to use in the forward direction.
    ///
    /// Defaults to `Curves::fast_out_slow_in()`.
    pub curve: Rc<dyn Curve>,
    /// The curve to use in the reverse direction.
    ///
    /// If this property is `None`, [`Hero::curve`]`.flipped()` is used.
    pub reverse_curve: Option<Rc<dyn Curve>>,
}

impl Hero {
    /// Create a hero; Dart's optional named arguments are the setters.
    ///
    /// The `child` parameter and all of its descendants must not be [`Hero`]es.
    ///
    /// Dart's defaults: `transition_on_user_gestures` false, `curve` `Curves::fast_out_slow_in()`.
    pub fn new<K>(tag: HeroTagRef, child: impl IntoWidget<K>) -> Hero {
        Hero {
            key: None,
            tag,
            create_rect_tween: None,
            child: child.into_widget(),
            flight_shuttle_builder: None,
            placeholder_builder: None,
            transition_on_user_gestures: false,
            curve: Curves::fast_out_slow_in(),
            reverse_curve: None,
        }
    }

    /// Dart `Hero(key:)`.
    pub fn key(mut self, key: KeyRef) -> Hero {
        self.key = Some(key);
        self
    }

    /// Dart `Hero(createRectTween:)`.
    pub fn create_rect_tween(mut self, create_rect_tween: CreateRectTween) -> Hero {
        self.create_rect_tween = Some(create_rect_tween);
        self
    }

    /// Dart `Hero(flightShuttleBuilder:)`.
    pub fn flight_shuttle_builder(
        mut self,
        flight_shuttle_builder: HeroFlightShuttleBuilder,
    ) -> Hero {
        self.flight_shuttle_builder = Some(flight_shuttle_builder);
        self
    }

    /// Dart `Hero(placeholderBuilder:)`.
    pub fn placeholder_builder(mut self, placeholder_builder: HeroPlaceholderBuilder) -> Hero {
        self.placeholder_builder = Some(placeholder_builder);
        self
    }

    /// Dart `Hero(transitionOnUserGestures:)`.
    pub fn transition_on_user_gestures(mut self, transition_on_user_gestures: bool) -> Hero {
        self.transition_on_user_gestures = transition_on_user_gestures;
        self
    }

    /// Dart `Hero(curve:)`.
    pub fn curve(mut self, curve: Rc<dyn Curve>) -> Hero {
        self.curve = curve;
        self
    }

    /// Dart `Hero(reverseCurve:)`.
    pub fn reverse_curve(mut self, reverse_curve: Rc<dyn Curve>) -> Hero {
        self.reverse_curve = Some(reverse_curve);
        self
    }

    /// Returns a map of all of the heroes in `context` indexed by hero tag that should be
    /// considered for animation when `navigator` transitions from one `PageRoute` to another.
    fn all_heroes_for(
        app: &mut App,
        context: BuildContext,
        is_user_gesture_transition: bool,
        navigator: Handle<NavigatorState>,
    ) -> IndexMap<HeroTagRef, Handle<HeroState>> {
        let mut result: IndexMap<HeroTagRef, Handle<HeroState>> = IndexMap::new();

        // Dart's `visitor` decides on each element and recurses; the walk itself only reads the
        // tree, so it collects the heroes in visit order and `invite_hero`'s `Navigator::of` /
        // `AnyModalRoute::of` / `end_flight`, which need `&mut App`, run after it.
        fn visitor(app: &App, element: AnyElement, heroes: &mut Vec<AnyElement>) {
            let widget = element.widget(app).clone();
            if downcast_widget::<Hero>(&*widget).is_some() {
                heroes.push(element);
            } else if downcast_widget::<HeroMode>(&*widget).is_some_and(|mode| !mode.enabled) {
                return;
            }
            element.visit_children(app, &mut |child| visitor(app, child, heroes));
        }

        let mut heroes = Vec::new();
        context.visit_child_elements(app, &mut |child| visitor(app, child, &mut heroes));

        for hero in heroes {
            let widget = hero.widget(app).clone();
            let hero_widget =
                downcast_widget::<Hero>(&*widget).expect("collected from the Hero branch");
            let tag = Rc::clone(&hero_widget.tag);
            let transition_on_user_gestures = hero_widget.transition_on_user_gestures;
            if Navigator::of(app, hero, false) == navigator {
                invite_hero(
                    app,
                    &mut result,
                    hero,
                    tag,
                    transition_on_user_gestures,
                    is_user_gesture_transition,
                );
            } else {
                // The nearest navigator to the hero is not the navigator that is currently
                // transitioning from one route to another. This means the hero is inside a nested
                // navigator and should only be considered for animation if it is part of the
                // top-most route in that nested navigator and if that route is also a
                // `PageRoute`.
                let hero_route = AnyModalRoute::of(app, hero);
                if let Some(hero_route) = hero_route
                    && hero_route.as_route().as_page_route(app).is_some()
                    && hero_route.as_route().is_current(app)
                {
                    invite_hero(
                        app,
                        &mut result,
                        hero,
                        tag,
                        transition_on_user_gestures,
                        is_user_gesture_transition,
                    );
                }
            }
        }

        fn invite_hero(
            app: &mut App,
            result: &mut IndexMap<HeroTagRef, Handle<HeroState>>,
            hero: AnyElement,
            tag: HeroTagRef,
            transition_on_user_gestures: bool,
            is_user_gesture_transition: bool,
        ) {
            debug_assert!(
                !result.contains_key(&tag),
                "There are multiple heroes that share the same tag within a subtree. Within each \
                 subtree for which heroes are to be animated (i.e. a PageRoute subtree), each \
                 Hero must have a unique non-null tag."
            );
            let hero_state = hero
                .state_handle::<HeroState>(app)
                .expect("a Hero element holds a HeroState");
            if !is_user_gesture_transition || transition_on_user_gestures {
                result.insert(tag, hero_state);
            } else {
                // If transition is not allowed, we need to make sure hero is not hidden.
                // A hero can be hidden previously due to hero transition.
                hero_state.end_flight(app, false);
            }
        }

        result
    }
}

impl StatefulWidget for Hero {
    type State = HeroState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> HeroState {
        HeroState {
            state: StateData::new(),
            key: GlobalKey::new(),
            placeholder_size: None,
            should_include_child: true,
        }
    }
}

impl Debug for Hero {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hero")
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

/// The [`Hero`] widget displays different content based on whether it is in an animated
/// transition ("flight"), from/to another [`Hero`] with the same tag:
///   * When [`start_flight`](Self::start_flight) is called, the real content of this [`Hero`]
///     will be replaced by a "placeholder" widget.
///   * When the flight ends, the "toHero"'s [`end_flight`](Self::end_flight) method must be
///     called by the hero controller, so the real content of that [`Hero`] becomes visible again
///     when the animation completes.
///
/// Dart's `_HeroState`.
pub struct HeroState {
    state: StateData<Hero>,
    key: GlobalKey,
    placeholder_size: Option<Size>,
    // Whether the placeholder widget should wrap the hero's child widget as its own child, when
    // `placeholder_size` is non-null (i.e. the hero is currently in its flight animation). See
    // `start_flight`.
    should_include_child: bool,
}

impl HeroState {
    /// Replaces this hero's content with a placeholder of its current size.
    ///
    /// The `should_included_child_in_placeholder` flag dictates if the child widget of this hero
    /// should be included in the placeholder widget as a descendant.
    ///
    /// When a new hero flight animation takes place, a placeholder widget needs to be built to
    /// replace the original hero widget. When `should_included_child_in_placeholder` is set to
    /// true and [`Hero::placeholder_builder`] is `None`, the placeholder widget will include the
    /// original hero's child widget as a descendant, allowing the original element tree to be
    /// preserved.
    ///
    /// It is typically set to true for the *from* hero in a push transition, and false otherwise.
    pub fn start_flight(
        self: Handle<Self>,
        app: &mut App,
        should_included_child_in_placeholder: bool,
    ) {
        app.get_mut(self).should_include_child = should_included_child_in_placeholder;
        debug_assert!(self.mounted(app));
        let render_box = self
            .context(app)
            .find_render_object(app)
            .expect("a mounted Hero has a render object")
            .as_box()
            .expect("a Hero's render object is a box");
        debug_assert!(render_box.has_size(app));
        let size = render_box.size(app);
        self.set_state(app, |state| state.placeholder_size = Some(size));
    }

    /// Restores this hero's content once its flight is over.
    ///
    /// When `keep_placeholder` is true, the placeholder will continue to be shown after the
    /// flight ends. Otherwise the child of the hero will become visible and its [`TickerMode`]
    /// will be re-enabled.
    ///
    /// This method can be safely called even when this [`Hero`] is currently not in a flight.
    pub fn end_flight(self: Handle<Self>, app: &mut App, keep_placeholder: bool) {
        // Dart reads `_placeholderSize` on a defunct state too; an unmounted state's arena slot
        // is gone, so the containment check stands in for that read.
        if keep_placeholder || !app.contains(self) || app.get(self).placeholder_size.is_none() {
            return;
        }

        app.get_mut(self).placeholder_size = None;
        if self.mounted(app) {
            // Tell the widget to rebuild if it's mounted. `placeholder_size` has already been
            // updated.
            self.set_state(app, |_| {});
        }
    }
}

impl State for HeroState {
    type Widget = Hero;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            context
                .find_ancestor_widget_of_exact_type::<Hero>(app)
                .is_none(),
            "A Hero widget cannot be the descendant of another Hero widget."
        );

        let placeholder_size = app.get(self).placeholder_size;
        let show_placeholder = placeholder_size.is_some();

        if show_placeholder
            && let Some(placeholder_builder) = self.widget(app).placeholder_builder.clone()
        {
            let child = self.widget(app).child.clone();
            let size = placeholder_size.expect("the placeholder is showing");
            return placeholder_builder(app, context, size, child);
        }

        if show_placeholder && !app.get(self).should_include_child {
            let size = placeholder_size.expect("the placeholder is showing");
            return SizedBox::new()
                .width(size.width())
                .height(size.height())
                .into_widget();
        }

        let key = app.get(self).key.clone();
        let child = self.widget(app).child.clone();
        let mut sized = SizedBox::new();
        if let Some(size) = placeholder_size {
            sized = sized.width(size.width()).height(size.height());
        }
        sized
            .child(
                Offstage::new()
                    .offstage(show_placeholder)
                    .child(TickerMode::new(
                        !show_placeholder,
                        KeyedSubtree::new(child).key(Rc::new(key)),
                    )),
            )
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _HeroFlightManifest

/// Everything known about a hero flight that's to be started or diverted.
struct HeroFlightManifest {
    r#type: HeroFlightDirection,
    overlay: Handle<OverlayState>,
    from_route: AnyPageRoute,
    to_route: AnyPageRoute,
    from_hero: Handle<HeroState>,
    to_hero: Handle<HeroState>,
    create_rect_tween: Option<CreateRectTween>,
    shuttle_builder: HeroFlightShuttleBuilder,
    is_user_gesture_transition: bool,
    is_diverted: bool,

    // Dart's `tag` getter reads `fromHero.widget.tag`; a manifest outlives an unmounted hero's
    // arena slot, so the tag — which the constructor asserts the two heroes share — is captured.
    tag: HeroTagRef,

    animation: Option<Handle<CurvedAnimation>>,
    // Dart's three `late final` fields, computed on first read and cached.
    from_hero_location: Option<Rect>,
    to_hero_location: Option<Rect>,
    is_valid: Option<bool>,
}

impl HeroFlightManifest {
    #[expect(clippy::too_many_arguments, reason = "Dart's required named arguments")]
    fn new(
        app: &mut App,
        r#type: HeroFlightDirection,
        overlay: Handle<OverlayState>,
        from_route: AnyPageRoute,
        to_route: AnyPageRoute,
        from_hero: Handle<HeroState>,
        to_hero: Handle<HeroState>,
        create_rect_tween: Option<CreateRectTween>,
        shuttle_builder: HeroFlightShuttleBuilder,
        is_user_gesture_transition: bool,
        is_diverted: bool,
    ) -> Handle<HeroFlightManifest> {
        let tag = Rc::clone(&from_hero.widget(app).tag);
        debug_assert!(*tag == *to_hero.widget(app).tag);
        app.create(HeroFlightManifest {
            r#type,
            overlay,
            from_route,
            to_route,
            from_hero,
            to_hero,
            create_rect_tween,
            shuttle_builder,
            is_user_gesture_transition,
            is_diverted,
            tag,
            animation: None,
            from_hero_location: None,
            to_hero_location: None,
            is_valid: None,
        })
    }

    fn tag(self: Handle<Self>, app: &App) -> HeroTagRef {
        Rc::clone(&app.get(self).tag)
    }

    fn animation(self: Handle<Self>, app: &mut App) -> AnyAnimation<f64> {
        if let Some(animation) = app.get(self).animation {
            return animation.as_animation();
        }
        let (curve, reverse_curve, parent) = match app.get(self).r#type {
            HeroFlightDirection::Push => {
                let to_route = app.get(self).to_route;
                let parent = to_route
                    .as_modal_route()
                    .animation(app)
                    .expect("an installed route has an animation");
                let to_hero = app.get(self).to_hero;
                let widget = to_hero.widget(app);
                let curve = Rc::clone(&widget.curve);
                let reverse_curve = widget
                    .reverse_curve
                    .clone()
                    .unwrap_or_else(|| Rc::new(Rc::clone(&curve).flipped()));
                (curve, reverse_curve, parent)
            }
            HeroFlightDirection::Pop => {
                let from_route = app.get(self).from_route;
                let parent = from_route
                    .as_modal_route()
                    .animation(app)
                    .expect("an installed route has an animation");
                let from_hero = app.get(self).from_hero;
                let widget = from_hero.widget(app);
                let curve = Rc::clone(&widget.curve);
                let reverse_curve = widget
                    .reverse_curve
                    .clone()
                    .unwrap_or_else(|| Rc::new(Rc::clone(&curve).flipped()));
                (curve, reverse_curve, parent)
            }
        };

        let reverse_curve = (!app.get(self).is_diverted).then_some(reverse_curve);
        let animation = CurvedAnimation::create(app, parent, curve, reverse_curve);
        app.get_mut(self).animation = Some(animation);
        animation.as_animation()
    }

    fn create_hero_rect_tween(
        self: Handle<Self>,
        app: &mut App,
        begin: Option<Rect>,
        end: Option<Rect>,
    ) -> Rc<dyn RectTweenObject> {
        let to_hero = app.get(self).to_hero;
        let create_rect_tween = to_hero
            .widget(app)
            .create_rect_tween
            .clone()
            .or_else(|| app.get(self).create_rect_tween.clone());
        match create_rect_tween {
            Some(create_rect_tween) => create_rect_tween(app, begin, end),
            None => Rc::new(RectTween::new(app, begin, end)),
        }
    }

    /// The bounding box for `context`'s render object, in `ancestor_context`'s render object's
    /// coordinate space.
    fn bounding_box_for(
        app: &App,
        context: BuildContext,
        ancestor_context: Option<BuildContext>,
    ) -> Rect {
        debug_assert!(ancestor_context.is_some());
        let render_box = context
            .find_render_object(app)
            .expect("a mounted hero has a render object")
            .as_box()
            .expect("a hero's render object is a box");
        debug_assert!(render_box.has_size(app) && render_box.size(app).is_finite());
        let ancestor = ancestor_context.and_then(|context| context.find_render_object(app));
        transform_rect(
            &render_box.as_object().get_transform_to(app, ancestor),
            Offset::ZERO & render_box.size(app),
        )
    }

    /// The bounding box of `from_hero`, in `from_route`'s coordinate space.
    ///
    /// This property should only be accessed in [`HeroFlight::start`].
    fn from_hero_location(self: Handle<Self>, app: &mut App) -> Rect {
        if let Some(location) = app.get(self).from_hero_location {
            return location;
        }
        let context = app.get(self).from_hero.context(app);
        let ancestor_context = app
            .get(self)
            .from_route
            .as_modal_route()
            .subtree_context(app);
        let location = HeroFlightManifest::bounding_box_for(app, context, ancestor_context);
        app.get_mut(self).from_hero_location = Some(location);
        location
    }

    /// The bounding box of `to_hero`, in `to_route`'s coordinate space.
    ///
    /// This property should only be accessed in [`HeroFlight::start`] or [`HeroFlight::divert`].
    fn to_hero_location(self: Handle<Self>, app: &mut App) -> Rect {
        if let Some(location) = app.get(self).to_hero_location {
            return location;
        }
        let context = app.get(self).to_hero.context(app);
        let ancestor_context = app.get(self).to_route.as_modal_route().subtree_context(app);
        let location = HeroFlightManifest::bounding_box_for(app, context, ancestor_context);
        app.get_mut(self).to_hero_location = Some(location);
        location
    }

    /// Whether this manifest is valid and can be used to start or divert a [`HeroFlight`].
    ///
    /// When starting or diverting a flight with a brand new manifest, this flag must be checked
    /// to ensure the rect tween the manifest produces does not contain coordinates that are
    /// infinite or NaN.
    fn is_valid(self: Handle<Self>, app: &mut App) -> bool {
        if let Some(is_valid) = app.get(self).is_valid {
            return is_valid;
        }
        let is_valid = self.to_hero_location(app).is_finite()
            && (app.get(self).is_diverted || self.from_hero_location(app).is_finite());
        app.get_mut(self).is_valid = Some(is_valid);
        is_valid
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = app.get(self).animation {
            animation.dispose(app);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _HeroFlight

/// Builds the in-flight hero widget.
struct HeroFlight {
    on_flight_ended: OnFlightEnded,

    hero_rect_tween: Option<Rc<dyn RectTweenObject>>,
    shuttle: Option<WidgetRef>,

    hero_opacity: AnyAnimation<f64>,
    proxy_animation: Handle<ProxyAnimation>,
    // The manifest will be available once `start` is called, throughout the flight's lifecycle.
    manifest: Option<Handle<HeroFlightManifest>>,

    overlay_entry: Option<Handle<OverlayEntry>>,
    aborted: bool,

    // Dart's `static final Animatable<double> _reverseTween`; a tween needs an App to live in,
    // so each flight mints its own.
    reverse_tween: Handle<Tween<f64>>,

    scheduled_perform_animation_update: bool,
    // The `navigator` Dart's local `delayedPerformAnimationUpdate` closure captures.
    scheduled_gesture_navigator: Option<Handle<NavigatorState>>,
}

impl HeroFlight {
    fn create(app: &mut App, on_flight_ended: OnFlightEnded) -> Handle<HeroFlight> {
        let hero_opacity = k_always_complete_animation(app);
        let proxy_animation = ProxyAnimation::new(app, None);
        let reverse_tween = Tween::new(app, Some(1.0), Some(0.0));
        let this = app.create(HeroFlight {
            on_flight_ended,
            hero_rect_tween: None,
            shuttle: None,
            hero_opacity,
            proxy_animation,
            manifest: None,
            overlay_entry: None,
            aborted: false,
            reverse_tween,
            scheduled_perform_animation_update: false,
            scheduled_gesture_navigator: None,
        });
        proxy_animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(this, HeroFlight::handle_animation_update),
        );
        this
    }

    fn manifest(self: Handle<Self>, app: &App) -> Handle<HeroFlightManifest> {
        app.get(self).manifest.expect("the flight has started")
    }

    fn set_manifest(self: Handle<Self>, app: &mut App, value: Handle<HeroFlightManifest>) {
        if let Some(manifest) = app.get(self).manifest {
            manifest.dispose(app);
        }
        app.get_mut(self).manifest = Some(value);
    }

    fn hero_rect_tween(self: Handle<Self>, app: &App) -> Rc<dyn RectTweenObject> {
        Rc::clone(
            app.get(self)
                .hero_rect_tween
                .as_ref()
                .expect("the flight has started"),
        )
    }

    /// The overlay entry's builder callback for the hero's overlay.
    fn build_overlay(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        if app.get(self).shuttle.is_none() {
            let manifest = self.manifest(app);
            let shuttle_builder = Rc::clone(&app.get(manifest).shuttle_builder);
            let animation = manifest.animation(app);
            let r#type = app.get(manifest).r#type;
            let from_hero_context = app.get(manifest).from_hero.context(app);
            let to_hero_context = app.get(manifest).to_hero.context(app);
            let shuttle = shuttle_builder(
                app,
                context,
                animation,
                r#type,
                from_hero_context,
                to_hero_context,
            );
            app.get_mut(self).shuttle = Some(shuttle);
        }
        debug_assert!(app.get(self).shuttle.is_some());

        let proxy_animation = app.get(self).proxy_animation;
        let shuttle = app
            .get(self)
            .shuttle
            .clone()
            .expect("built just above if it was missing");
        AnimatedBuilder::new(Rc::new(proxy_animation), move |app, _context, child| {
            let rect = self
                .hero_rect_tween(app)
                .evaluate(app, proxy_animation.as_animation())
                .expect("the hero rect tween has both ends");
            let mut fade = FadeTransition::new(app.get(self).hero_opacity);
            if let Some(child) = child {
                fade = fade.child(child.clone());
            }
            Positioned::from_rect(rect, IgnorePointer::new().child(fade)).into_widget()
        })
        .child(shuttle)
        .into_widget()
    }

    fn perform_animation_update(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if !status.is_animating() {
            let proxy_animation = app.get(self).proxy_animation;
            proxy_animation.set_parent(app, None);

            debug_assert!(app.get(self).overlay_entry.is_some());
            let overlay_entry = app
                .get(self)
                .overlay_entry
                .expect("a started flight has an overlay entry");
            overlay_entry.remove(app);
            overlay_entry.dispose(app);
            app.get_mut(self).overlay_entry = None;
            // We want to keep the hero underneath the current page hidden. If the status is
            // completed, `to_hero` will be the one on top and we keep `from_hero` hidden. If it
            // is dismissed, the animation is triggered but canceled before it finishes. In this
            // case, we keep `to_hero` hidden instead.
            let manifest = self.manifest(app);
            let from_hero = app.get(manifest).from_hero;
            let to_hero = app.get(manifest).to_hero;
            from_hero.end_flight(app, status.is_completed());
            to_hero.end_flight(app, status.is_dismissed());
            let on_flight_ended = Rc::clone(&app.get(self).on_flight_ended);
            on_flight_ended(app, self);
            Animation::remove_listener(
                proxy_animation,
                app,
                &Listener::handle_method(self, HeroFlight::on_tick),
            );
        }
    }

    fn handle_animation_update(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        // The animation will not finish until the user lifts their finger, so we should suppress
        // the status update if the gesture is in progress, and delay it until the finger is
        // lifted.
        let manifest = self.manifest(app);
        let navigator = app.get(manifest).from_route.as_route().navigator(app);
        if navigator.map(|navigator| navigator.user_gesture_in_progress(app)) != Some(true) {
            self.perform_animation_update(app, status);
            return;
        }

        if app.get(self).scheduled_perform_animation_update {
            return;
        }

        // The `navigator` must be non-null here, or the first if clause above would have returned
        // from this method.
        let navigator = navigator.expect("checked by the clause above");

        debug_assert!(navigator.user_gesture_in_progress(app));
        let flight = app.get_mut(self);
        flight.scheduled_perform_animation_update = true;
        flight.scheduled_gesture_navigator = Some(navigator);
        let notifier = navigator.user_gesture_in_progress_notifier(app);
        notifier.add_listener(
            app,
            Listener::handle_method(self, HeroFlight::delayed_perform_animation_update),
        );
    }

    fn delayed_perform_animation_update(self: Handle<Self>, app: &mut App) {
        let navigator = app
            .get(self)
            .scheduled_gesture_navigator
            .expect("set where the listener was registered");
        debug_assert!(!navigator.user_gesture_in_progress(app));
        debug_assert!(app.get(self).scheduled_perform_animation_update);
        let flight = app.get_mut(self);
        flight.scheduled_perform_animation_update = false;
        flight.scheduled_gesture_navigator = None;
        let notifier = navigator.user_gesture_in_progress_notifier(app);
        notifier.remove_listener(
            app,
            &Listener::handle_method(self, HeroFlight::delayed_perform_animation_update),
        );
        let status = app.get(self).proxy_animation.status(app);
        self.perform_animation_update(app, status);
    }

    /// Releases resources.
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(overlay_entry) = app.get(self).overlay_entry {
            overlay_entry.remove(app);
            overlay_entry.dispose(app);
            app.get_mut(self).overlay_entry = None;
            let proxy_animation = app.get(self).proxy_animation;
            proxy_animation.set_parent(app, None);
            Animation::remove_listener(
                proxy_animation,
                app,
                &Listener::handle_method(self, HeroFlight::on_tick),
            );
            proxy_animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, HeroFlight::handle_animation_update),
            );
        }
        if let Some(manifest) = app.get(self).manifest {
            manifest.dispose(app);
        }
    }

    fn on_tick(self: Handle<Self>, app: &mut App) {
        let manifest = self.manifest(app);
        let to_hero = app.get(manifest).to_hero;
        let to_hero_box = (!app.get(self).aborted && to_hero.mounted(app))
            .then(|| to_hero.context(app).find_render_object(app))
            .flatten()
            .map(|object| object.as_box().expect("a hero's render object is a box"));
        // Try to find the new origin of the `to_hero`, if the flight isn't aborted.
        let to_hero_origin = match to_hero_box {
            Some(to_hero_box)
                if to_hero_box.as_object().attached(app) && to_hero_box.has_size(app) =>
            {
                let ancestor = app
                    .get(manifest)
                    .to_route
                    .as_modal_route()
                    .subtree_context(app)
                    .and_then(|context| context.find_render_object(app))
                    .map(|object| {
                        object
                            .as_box()
                            .expect("a route's subtree render object is a box")
                            .as_object()
                    });
                Some(to_hero_box.local_to_global(app, Offset::ZERO, ancestor))
            }
            _ => None,
        };

        if let Some(to_hero_origin) = to_hero_origin
            && to_hero_origin.is_finite()
        {
            // If the new origin of `to_hero` is available and also paintable, try to update
            // `hero_rect_tween` with it.
            let hero_rect_tween = self.hero_rect_tween(app);
            let end = hero_rect_tween
                .end(app)
                .expect("the hero rect tween has both ends");
            if to_hero_origin != end.top_left() {
                let hero_rect_end = to_hero_origin & end.size();
                let begin = hero_rect_tween.begin(app);
                let hero_rect_tween =
                    manifest.create_hero_rect_tween(app, begin, Some(hero_rect_end));
                app.get_mut(self).hero_rect_tween = Some(hero_rect_tween);
            }
        } else if app.get(self).hero_opacity.status(app).is_completed() {
            // The `to_hero` no longer exists or it's no longer the flight's destination.
            // Continue flying while fading out.
            let proxy_animation = app.get(self).proxy_animation;
            let curve = Interval::new(proxy_animation.value(app), 1.0, Curves::linear());
            let curve_tween = CurveTween::new(app, Rc::new(curve));
            let reverse_tween = app.get(self).reverse_tween;
            let hero_opacity = proxy_animation.drive(app, reverse_tween.chain(curve_tween));
            app.get_mut(self).hero_opacity = hero_opacity;
        }
        // Update `aborted` for the next animation tick.
        app.get_mut(self).aborted =
            to_hero_origin.is_none_or(|to_hero_origin| !to_hero_origin.is_finite());
    }

    /// The simple case: we're either starting a push or a pop animation.
    fn start(self: Handle<Self>, app: &mut App, initial_manifest: Handle<HeroFlightManifest>) {
        debug_assert!(!app.get(self).aborted);
        if cfg!(debug_assertions) {
            let initial = initial_manifest.animation(app);
            let r#type = app.get(initial_manifest).r#type;
            assert!(match r#type {
                HeroFlightDirection::Pop =>
                // During user gesture transitions, the animation controller isn't driving the
                // reverse transition, so the status is not important.
                    app.get(initial_manifest).is_user_gesture_transition
                        || initial.status(app) == AnimationStatus::Reverse,
                HeroFlightDirection::Push =>
                    initial.value(app) == 0.0 && initial.status(app) == AnimationStatus::Forward,
            });
        }

        self.set_manifest(app, initial_manifest);

        let manifest = self.manifest(app);
        let proxy_animation = app.get(self).proxy_animation;
        let animation = manifest.animation(app);
        let should_include_child_in_placeholder = match app.get(manifest).r#type {
            HeroFlightDirection::Pop => {
                let parent = app.create(ReverseAnimation::new(animation)).as_animation();
                proxy_animation.set_parent(app, Some(parent));
                false
            }
            HeroFlightDirection::Push => {
                proxy_animation.set_parent(app, Some(animation));
                true
            }
        };

        let begin = manifest.from_hero_location(app);
        let end = manifest.to_hero_location(app);
        let hero_rect_tween = manifest.create_hero_rect_tween(app, Some(begin), Some(end));
        app.get_mut(self).hero_rect_tween = Some(hero_rect_tween);
        let from_hero = app.get(manifest).from_hero;
        let to_hero = app.get(manifest).to_hero;
        from_hero.start_flight(app, should_include_child_in_placeholder);
        to_hero.start_flight(app, false);
        let overlay_entry = OverlayEntry::new(
            app,
            Rc::new(move |app, context| self.build_overlay(app, context)),
            false,
            false,
            false,
        );
        app.get_mut(self).overlay_entry = Some(overlay_entry);
        app.get(manifest)
            .overlay
            .insert(app, overlay_entry, None, None);
        Animation::add_listener(
            proxy_animation,
            app,
            Listener::handle_method(self, HeroFlight::on_tick),
        );
    }

    /// While this flight's hero was in transition a push or a pop occurred for routes with the
    /// same hero. Redirect the in-flight hero to the new `to_route`.
    fn divert(self: Handle<Self>, app: &mut App, new_manifest: Handle<HeroFlightManifest>) {
        let manifest = self.manifest(app);
        debug_assert!(*manifest.tag(app) == *new_manifest.tag(app));
        let proxy_animation = app.get(self).proxy_animation;
        if app.get(manifest).r#type == HeroFlightDirection::Push
            && app.get(new_manifest).r#type == HeroFlightDirection::Pop
        {
            // A push flight was interrupted by a pop.
            let new_animation = new_manifest.animation(app);
            debug_assert!(new_animation.status(app) == AnimationStatus::Reverse);
            debug_assert!(app.get(manifest).from_hero == app.get(new_manifest).to_hero);
            debug_assert!(app.get(manifest).to_hero == app.get(new_manifest).from_hero);
            debug_assert!(app.get(manifest).from_route == app.get(new_manifest).to_route);
            debug_assert!(app.get(manifest).to_route == app.get(new_manifest).from_route);

            // The same hero rect tween is used in reverse, rather than creating a new one with
            // `create_hero_rect_tween(hero_rect.end, hero_rect.begin)`. That's because tweens like
            // `MaterialRectArcTween` may create a different path for swapped begin and end
            // parameters. We want the pop flight path to be the same (in reverse) as the push
            // flight path.
            let parent = app
                .create(ReverseAnimation::new(new_animation))
                .as_animation();
            proxy_animation.set_parent(app, Some(parent));
            let parent_tween = self.hero_rect_tween(app);
            app.get_mut(self).hero_rect_tween = Some(Rc::new(ReverseRectTween {
                parent: parent_tween,
            }));
        } else if app.get(manifest).r#type == HeroFlightDirection::Pop
            && app.get(new_manifest).r#type == HeroFlightDirection::Push
        {
            // A pop flight was interrupted by a push.
            let new_animation = new_manifest.animation(app);
            debug_assert!(new_animation.status(app) == AnimationStatus::Forward);
            debug_assert!(app.get(manifest).to_hero == app.get(new_manifest).from_hero);
            debug_assert!(app.get(manifest).to_route == app.get(new_manifest).from_route);

            let value = manifest.animation(app).value(app);
            let tween = Tween::new(app, Some(value), Some(1.0));
            let parent = new_animation.drive(app, tween);
            proxy_animation.set_parent(app, Some(parent));
            let from_hero = app.get(manifest).from_hero;
            let new_to_hero = app.get(new_manifest).to_hero;
            if from_hero != new_to_hero {
                from_hero.end_flight(app, true);
                new_to_hero.start_flight(app, false);
                let hero_rect_tween = self.hero_rect_tween(app);
                let begin = hero_rect_tween.end(app);
                let end = new_manifest.to_hero_location(app);
                let hero_rect_tween = manifest.create_hero_rect_tween(app, begin, Some(end));
                app.get_mut(self).hero_rect_tween = Some(hero_rect_tween);
            } else {
                // TODO(hansmuller): Use `ReverseRectTween` here per
                // github.com/flutter/flutter/pull/12203.
                let hero_rect_tween = self.hero_rect_tween(app);
                let begin = hero_rect_tween.end(app);
                let end = hero_rect_tween.begin(app);
                let hero_rect_tween = manifest.create_hero_rect_tween(app, begin, end);
                app.get_mut(self).hero_rect_tween = Some(hero_rect_tween);
            }
        } else {
            // A push or a pop flight is heading to a new route, i.e.
            // manifest.type == push && newManifest.type == push ||
            // manifest.type == pop && newManifest.type == pop
            debug_assert!(app.get(manifest).from_hero != app.get(new_manifest).from_hero);
            debug_assert!(app.get(manifest).to_hero != app.get(new_manifest).to_hero);

            let hero_rect_tween = self.hero_rect_tween(app);
            let begin = hero_rect_tween.evaluate(app, proxy_animation.as_animation());
            let end = new_manifest.to_hero_location(app);
            let hero_rect_tween = manifest.create_hero_rect_tween(app, begin, Some(end));
            app.get_mut(self).hero_rect_tween = Some(hero_rect_tween);
            app.get_mut(self).shuttle = None;

            let new_animation = new_manifest.animation(app);
            if app.get(new_manifest).r#type == HeroFlightDirection::Pop {
                let parent = app
                    .create(ReverseAnimation::new(new_animation))
                    .as_animation();
                proxy_animation.set_parent(app, Some(parent));
            } else {
                proxy_animation.set_parent(app, Some(new_animation));
            }

            let from_hero = app.get(manifest).from_hero;
            let to_hero = app.get(manifest).to_hero;
            from_hero.end_flight(app, true);
            to_hero.end_flight(app, true);

            // Let the heroes in each of the routes rebuild with their placeholders.
            let new_from_hero = app.get(new_manifest).from_hero;
            let new_to_hero = app.get(new_manifest).to_hero;
            let should_include_child_in_placeholder =
                app.get(new_manifest).r#type == HeroFlightDirection::Push;
            new_from_hero.start_flight(app, should_include_child_in_placeholder);
            new_to_hero.start_flight(app, false);

            // Let the transition overlay on top of the routes also rebuild since we cleared the
            // old shuttle.
            app.get(self)
                .overlay_entry
                .expect("a started flight has an overlay entry")
                .mark_needs_build(app);
        }

        self.set_manifest(app, new_manifest);
    }

    fn abort(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).aborted = true;
    }
}

// ---------------------------------------------------------------------------------------------
// HeroController

/// A [`Navigator`] observer that manages [`Hero`] transitions.
///
/// An instance of [`HeroController`] should be used in [`Navigator::observers`].
pub struct HeroController {
    navigator_observer: NavigatorObserverData,
    /// Used to create rect tweens that interpolate the position of heroes in flight.
    ///
    /// If `None`, the controller uses a linear `RectTween`.
    pub create_rect_tween: Option<CreateRectTween>,
    // All of the heroes that are currently in the overlay and in motion. Indexed by the hero tag.
    flights: IndexMap<HeroTagRef, Handle<HeroFlight>>,
}

impl HeroController {
    /// Creates a hero controller; Dart's optional named argument is the setter.
    pub fn new(app: &mut App) -> Handle<HeroController> {
        app.create(HeroController {
            navigator_observer: NavigatorObserverData::new(),
            create_rect_tween: None,
            flights: IndexMap::new(),
        })
    }

    /// Dart `HeroController(createRectTween:)`.
    pub fn create_rect_tween(
        self: Handle<Self>,
        app: &mut App,
        create_rect_tween: CreateRectTween,
    ) -> Handle<Self> {
        app.get_mut(self).create_rect_tween = Some(create_rect_tween);
        self
    }

    /// If we're transitioning between different page routes, start a hero transition after the
    /// `to_route` has been laid out with its animation's value at 1.0.
    fn maybe_start_hero_transition(
        self: Handle<Self>,
        app: &mut App,
        from_route: Option<AnyRoute>,
        to_route: Option<AnyRoute>,
        is_user_gesture_transition: bool,
    ) {
        if to_route == from_route {
            return;
        }
        let (Some(to_route), Some(from_route)) = (
            to_route.and_then(|route| route.as_page_route(app)),
            from_route.and_then(|route| route.as_page_route(app)),
        ) else {
            return;
        };
        let new_route_animation = to_route
            .as_modal_route()
            .animation(app)
            .expect("an installed route has an animation");
        let old_route_animation = from_route
            .as_modal_route()
            .animation(app)
            .expect("an installed route has an animation");
        let flight_type = match (
            is_user_gesture_transition,
            old_route_animation.status(app),
            new_route_animation.status(app),
        ) {
            (true, _, _) | (_, AnimationStatus::Reverse, _) => Some(HeroFlightDirection::Pop),
            (_, _, AnimationStatus::Forward) => Some(HeroFlightDirection::Push),
            _ => None,
        };

        // A user gesture may have already completed the pop, or we might be the initial route.
        if let Some(flight_type) = flight_type {
            match flight_type {
                HeroFlightDirection::Pop => {
                    if old_route_animation.value(app) == 0.0 {
                        return;
                    }
                }
                HeroFlightDirection::Push => {
                    if new_route_animation.value(app) == 1.0 {
                        return;
                    }
                }
            }
        }

        // For pop transitions driven by a user gesture: if the "to" page has `maintain_state`
        // true, then the hero's final dimensions can be measured immediately because their page's
        // layout is still valid. Unless due to directly adding routes to the pages stack causing
        // the route to never get laid out.
        let from_route_render_box = to_route
            .as_modal_route()
            .subtree_context(app)
            .and_then(|context| context.find_render_object(app))
            .map(|object| {
                object
                    .as_box()
                    .expect("a route's subtree render object is a box")
            });
        let has_valid_size = from_route_render_box
            .is_some_and(|render_box| render_box.has_size(app) && render_box.size(app).is_finite());
        if is_user_gesture_transition
            && flight_type == Some(HeroFlightDirection::Pop)
            && to_route.as_modal_route().maintain_state(app)
            && has_valid_size
        {
            self.start_hero_transition(
                app,
                from_route,
                to_route,
                flight_type,
                is_user_gesture_transition,
            );
        } else {
            // Otherwise, delay measuring until the end of the next frame to allow the 'to' route
            // to build and layout.

            // Putting a route offstage changes its animation value to 1.0. Once this frame
            // completes, we'll know where the heroes in the `to` route are going to end up, and
            // the `to` route will go back onstage.
            let offstage = new_route_animation.value(app) == 0.0;
            to_route.as_modal_route().set_offstage(app, offstage);
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if from_route.as_route().navigator(app).is_none()
                        || to_route.as_route().navigator(app).is_none()
                    {
                        return;
                    }
                    self.start_hero_transition(
                        app,
                        from_route,
                        to_route,
                        flight_type,
                        is_user_gesture_transition,
                    );
                }),
            );
        }
    }

    /// Find the matching pairs of heroes in from and to and either start a new hero flight, or
    /// divert an existing one.
    fn start_hero_transition(
        self: Handle<Self>,
        app: &mut App,
        from: AnyPageRoute,
        to: AnyPageRoute,
        flight_type: Option<HeroFlightDirection>,
        is_user_gesture_transition: bool,
    ) {
        // If the `to` route was offstage, then we're implicitly restoring its animation value back
        // to what it was before it was "moved" offstage.
        to.as_modal_route().set_offstage(app, false);

        let navigator = self.navigator(app);
        let overlay = navigator.and_then(|navigator| navigator.overlay(app));
        // If the navigator or the overlay was removed before this end-of-frame callback was
        // called, then don't actually start a transition, and we don't have to worry about any
        // hero widget we might have hidden in a previous flight, or ongoing flights.
        let (Some(navigator), Some(overlay)) = (navigator, overlay) else {
            return;
        };

        // At this point, the toHeroes may have been built and laid out for the first time.
        //
        // If `from_subtree_context` is `None`, call `end_flight` on all toHeroes, for good
        // measure. If `to_subtree_context` is `None` abort existing flights.
        let from_subtree_context = from.as_modal_route().subtree_context(app);
        let from_heroes = match from_subtree_context {
            Some(context) => {
                Hero::all_heroes_for(app, context, is_user_gesture_transition, navigator)
            }
            None => IndexMap::new(),
        };
        let to_subtree_context = to.as_modal_route().subtree_context(app);
        let mut to_heroes = match to_subtree_context {
            Some(context) => {
                Hero::all_heroes_for(app, context, is_user_gesture_transition, navigator)
            }
            None => IndexMap::new(),
        };

        for (tag, from_hero) in from_heroes {
            let to_hero = to_heroes.get(&tag).copied();
            let existing_flight = app.get(self).flights.get(&tag).copied();
            let manifest = match (to_hero, flight_type) {
                (Some(to_hero), Some(flight_type)) => {
                    let shuttle_builder = to_hero
                        .widget(app)
                        .flight_shuttle_builder
                        .clone()
                        .or_else(|| from_hero.widget(app).flight_shuttle_builder.clone())
                        .unwrap_or_else(|| {
                            Rc::new(HeroController::default_hero_flight_shuttle_builder)
                        });
                    let create_rect_tween = app.get(self).create_rect_tween.clone();
                    Some(HeroFlightManifest::new(
                        app,
                        flight_type,
                        overlay,
                        from,
                        to,
                        from_hero,
                        to_hero,
                        create_rect_tween,
                        shuttle_builder,
                        is_user_gesture_transition,
                        existing_flight.is_some(),
                    ))
                }
                _ => None,
            };

            // Only proceed with a valid manifest. Otherwise abort the existing flight, and call
            // `end_flight` when this for loop finishes.
            match manifest {
                Some(manifest) if manifest.is_valid(app) => {
                    to_heroes.shift_remove(&tag);
                    match existing_flight {
                        Some(existing_flight) => existing_flight.divert(app, manifest),
                        None => {
                            // Dart's cascade runs `start` before the map assignment.
                            let flight = HeroFlight::create(
                                app,
                                Rc::new(move |app, flight| self.handle_flight_ended(app, flight)),
                            );
                            flight.start(app, manifest);
                            app.get_mut(self).flights.insert(tag, flight);
                        }
                    }
                }
                _ => {
                    if let Some(existing_flight) = existing_flight {
                        existing_flight.abort(app);
                    }
                }
            }
        }

        // The remaining entries in `to_heroes` are those that failed to participate in a new
        // flight (for not having a valid manifest).
        //
        // This can happen in a route pop transition when a `from_hero` is no longer mounted, or
        // kept alive by the `KeepAlive` mechanism but no longer visible.
        for to_hero in to_heroes.into_values() {
            to_hero.end_flight(app, false);
        }
    }

    fn handle_flight_ended(self: Handle<Self>, app: &mut App, flight: Handle<HeroFlight>) {
        let tag = flight.manifest(app).tag(app);
        if let Some(flight) = app.get_mut(self).flights.shift_remove(&tag) {
            flight.dispose(app);
        }
    }

    fn default_hero_flight_shuttle_builder(
        app: &mut App,
        _flight_context: BuildContext,
        animation: AnyAnimation<f64>,
        flight_direction: HeroFlightDirection,
        from_hero_context: BuildContext,
        to_hero_context: BuildContext,
    ) -> WidgetRef {
        let to_hero_widget = to_hero_context.widget(app).clone();
        let to_hero =
            downcast_widget::<Hero>(&*to_hero_widget).expect("a hero context holds a Hero");
        let to_hero_child = to_hero.child.clone();

        let to_media_query_data = MediaQuery::maybe_of(app, to_hero_context);
        let from_media_query_data = MediaQuery::maybe_of(app, from_hero_context);

        let (Some(to_media_query_data), Some(from_media_query_data)) =
            (to_media_query_data, from_media_query_data)
        else {
            return to_hero_child;
        };

        let from_hero_padding = from_media_query_data.padding;
        let to_hero_padding = to_media_query_data.padding;

        AnimatedBuilder::new(Rc::new(animation), move |app, _context, _child| {
            let padding = match flight_direction {
                HeroFlightDirection::Push => {
                    EdgeInsetsTween::new(Some(from_hero_padding), Some(to_hero_padding))
                        .evaluate(app, animation)
                }
                HeroFlightDirection::Pop => {
                    EdgeInsetsTween::new(Some(to_hero_padding), Some(from_hero_padding))
                        .evaluate(app, animation)
                }
            };
            MediaQuery::new(
                to_media_query_data.copy_with().padding(padding),
                to_hero_child.clone(),
            )
            .into_widget()
        })
        .into_widget()
    }

    /// Releases resources.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let flights: Vec<Handle<HeroFlight>> = app.get(self).flights.values().copied().collect();
        for flight in flights {
            flight.dispose(app);
        }
    }
}

impl NavigatorObserver for HeroController {
    crate::navigator_observer_accessors!();

    fn did_change_top(
        self: Handle<Self>,
        app: &mut App,
        top_route: AnyRoute,
        previous_top_route: Option<AnyRoute>,
    ) {
        debug_assert!(top_route.is_current(app));
        debug_assert!(self.navigator(app).is_some());
        let Some(previous_top_route) = previous_top_route else {
            return;
        };
        // Don't trigger another flight when a pop is committed as a user gesture back swipe is
        // snapped.
        let navigator = self.navigator(app).expect("asserted above");
        if !navigator.user_gesture_in_progress(app) {
            self.maybe_start_hero_transition(app, Some(previous_top_route), Some(top_route), false);
        }
    }

    fn did_start_user_gesture(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        debug_assert!(self.navigator(app).is_some());
        self.maybe_start_hero_transition(app, Some(route), previous_route, true);
    }

    fn did_stop_user_gesture(self: Handle<Self>, app: &mut App) {
        let navigator = self
            .navigator(app)
            .expect("a gesture is only stopped on an observed navigator");
        if navigator.user_gesture_in_progress(app) {
            return;
        }

        // When the user gesture ends, if the user horizontal drag gesture initiated the flight
        // (i.e. the back swipe) didn't move towards the pop direction at all, the animation will
        // not play and thus the status update callback `handle_animation_update` will never be
        // called when the gesture finishes. In this case the initiated flight needs to be manually
        // invalidated.
        let is_invalid_flight = |app: &App, flight: Handle<HeroFlight>| {
            let manifest = flight.manifest(app);
            app.get(manifest).is_user_gesture_transition
                && app.get(manifest).r#type == HeroFlightDirection::Pop
                && app.get(flight).proxy_animation.status(app).is_dismissed()
        };

        let invalid_flights: Vec<Handle<HeroFlight>> = app
            .get(self)
            .flights
            .values()
            .copied()
            .filter(|flight| is_invalid_flight(app, *flight))
            .collect();

        // Treat these invalidated flights as dismissed. Calling `handle_animation_update` will
        // also remove the flight from `flights`.
        for flight in invalid_flights {
            flight.handle_animation_update(app, AnimationStatus::Dismissed);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// HeroMode

/// Enables or disables [`Hero`]es in the widget subtree.
///
/// When [`enabled`](Self::enabled) is false, all [`Hero`] widgets in this subtree will not be
/// involved in hero animations.
///
/// When [`enabled`](Self::enabled) is true (the default), [`Hero`] widgets may be involved in hero
/// animations, as usual.
#[derive(Debug)]
pub struct HeroMode {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The subtree to place inside the [`HeroMode`].
    pub child: WidgetRef,
    /// Whether or not [`Hero`]es are enabled in this subtree.
    ///
    /// If this property is false, the [`Hero`]es in this subtree will not animate on route
    /// changes. Otherwise, they will animate as usual.
    ///
    /// Defaults to true.
    pub enabled: bool,
}

impl HeroMode {
    /// Creates a widget that enables or disables [`Hero`]es; Dart's optional named arguments are
    /// the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> HeroMode {
        HeroMode {
            key: None,
            child: child.into_widget(),
            enabled: true,
        }
    }

    /// Dart `HeroMode(key:)`.
    pub fn key(mut self, key: KeyRef) -> HeroMode {
        self.key = Some(key);
        self
    }

    /// Dart `HeroMode(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> HeroMode {
        self.enabled = enabled;
        self
    }
}

impl StatelessWidget for HeroMode {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::time::Duration;

    use inset_embedder::{
        Picture, Platform, PlatformRef, TargetPlatform, TextDirection, View as EmbedderView,
        ViewConstraints, ViewId, ViewMetrics, ViewRef,
    };
    use inset_painting::{AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry};
    use inset_rendering::AnyRenderBox;
    use inset_scheduler::SchedulerBinding;

    use super::*;
    use crate::binding::run_app;
    use crate::widgets::basic::{Align, Directionality, Padding};
    use crate::widgets::navigator::{HeroControllerScope, Route, RouteSettings};
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::routes::RoutePageBuilder;

    const VIEW_WIDTH: f64 = 300.0;
    const VIEW_HEIGHT: f64 = 200.0;

    struct TestView;

    impl EmbedderView for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics {
                physical_size: [VIEW_WIDTH, VIEW_HEIGHT],
                physical_constraints: ViewConstraints::tight(VIEW_WIDTH, VIEW_HEIGHT),
                device_pixel_ratio: 1.0,
                ..ViewMetrics::default()
            }
        }

        fn present(&self, _picture: std::sync::Arc<Picture>) {}
    }

    struct TestPlatform {
        view: ViewRef,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }
    }

    /// An [`App`] with a single view; the navigator needs a binding for its global keys.
    fn app_with_view() -> Rc<AppCell> {
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::new(TestView),
        });
        AppCell::with_platform(platform)
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn mount(cell: &AppCell, child: WidgetRef) {
        run_app(&mut cell.borrow_mut(), child);
        cell.elapse(Duration::ZERO);
        pump_frame(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// Runs frames until every route transition has settled.
    fn settle(app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..40 {
            at += Duration::from_millis(20);
            pump_frame(app, at);
        }
        at
    }

    fn tag() -> HeroTagRef {
        Rc::new("photo")
    }

    /// A page holding one [`Hero`] of `width` x `height` at (`left`, `top`).
    fn hero_page(
        left: f64,
        top: f64,
        width: f64,
        height: f64,
        hero_mode: bool,
    ) -> RoutePageBuilder {
        Rc::new(move |_app, _context, _animation, _secondary| {
            let hero = Hero::new(tag(), SizedBox::new().width(width).height(height));
            let child: WidgetRef = if hero_mode {
                hero.into_widget()
            } else {
                HeroMode::new(hero).enabled(false).into_widget()
            };
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(
                    Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, 0.0, 0.0)).child(child),
                )
                .into_widget()
        })
    }

    fn hero_route(app: &mut App, name: &str, page: RoutePageBuilder) -> Handle<PageRouteBuilder> {
        PageRouteBuilder::new(app, page)
            .settings(app, RouteSettings::new().name(name.to_string()).into())
    }

    /// A navigator hosting `controller`, under the `Directionality` the overlay's theater needs.
    fn hero_navigator(
        key: &GlobalKey,
        controller: Handle<HeroController>,
        first_page: RoutePageBuilder,
    ) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            HeroControllerScope::new(
                controller,
                Navigator::new()
                    .key(Rc::new(key.clone()))
                    .on_generate_route(move |app, _settings| {
                        Some(hero_route(app, "/", first_page.clone()).as_route())
                    }),
            ),
        )
        .into_widget()
    }

    fn navigator_state(key: &GlobalKey, app: &mut App) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(app)
            .expect("a mounted navigator")
    }

    /// The theater's children, bottom-most first; the top-most is the entry inserted last.
    fn overlay_children(app: &mut App, navigator: Handle<NavigatorState>) -> Vec<AnyRenderBox> {
        let overlay = navigator.overlay(app).expect("a mounted overlay");
        let theater = overlay
            .context(app)
            .find_render_object(app)
            .expect("the overlay is laid out");
        let mut children = Vec::new();
        theater.visit_children(app, &mut |child| {
            children.push(child.as_box().expect("a theater child is a box"));
        });
        children
    }

    /// The bounds of the top-most overlay child — the hero flight's shuttle while one is flying.
    fn top_overlay_rect(app: &mut App, navigator: Handle<NavigatorState>) -> Rect {
        let top = *overlay_children(app, navigator)
            .last()
            .expect("the overlay has children");
        top.local_to_global(app, Offset::ZERO, None) & top.size(app)
    }

    fn only_flight(app: &App, controller: Handle<HeroController>) -> Handle<HeroFlight> {
        let flights = &app.get(controller).flights;
        assert_eq!(flights.len(), 1, "one flight per matching tag");
        *flights.values().next().expect("checked just above")
    }

    // ---- the tests ----

    #[test]
    fn a_hero_flies_between_two_page_routes() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let controller = HeroController::new(&mut app);
        drop(app);
        mount(
            &cell,
            hero_navigator(&key, controller, hero_page(10.0, 20.0, 40.0, 30.0, true)),
        );
        let mut app = cell.borrow_mut();
        let navigator = navigator_state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        let entries_before = overlay_children(&mut app, navigator).len();

        let second = hero_route(
            &mut app,
            "second",
            hero_page(150.0, 100.0, 80.0, 60.0, true),
        );
        navigator.push(&mut app, second.as_route());

        // The observer's post-frame callback measures the destination and starts the flight.
        let mut at = at + Duration::from_millis(20);
        pump_frame(&mut app, at);
        let flight = only_flight(&app, controller);
        let manifest = flight.manifest(&app);
        let (from_hero, to_hero) = (app.get(manifest).from_hero, app.get(manifest).to_hero);
        assert_eq!(
            app.get(manifest).r#type,
            HeroFlightDirection::Push,
            "a push flies forwards"
        );
        assert_eq!(
            (
                app.get(from_hero).placeholder_size,
                app.get(to_hero).placeholder_size
            ),
            (Some(Size::new(40.0, 30.0)), Some(Size::new(80.0, 60.0))),
            "both heroes hand their place to a placeholder of their own size"
        );
        assert_eq!(
            manifest.from_hero_location(&mut app),
            Rect::from_ltrb(10.0, 20.0, 50.0, 50.0),
            "the source hero is measured in its route's coordinate space"
        );
        assert_eq!(
            manifest.to_hero_location(&mut app),
            Rect::from_ltrb(150.0, 100.0, 230.0, 160.0),
            "the destination hero is measured after the offstage frame laid it out"
        );
        let tween = flight.hero_rect_tween(&app);
        assert_eq!(
            (tween.transform(&app, 0.0), tween.transform(&app, 1.0)),
            (
                Some(Rect::from_ltrb(10.0, 20.0, 50.0, 50.0)),
                Some(Rect::from_ltrb(150.0, 100.0, 230.0, 160.0))
            ),
            "the flight interpolates from the source bounds to the destination bounds"
        );

        // The shuttle is the entry the flight inserted on top of the navigator's overlay.
        at += Duration::from_millis(20);
        pump_frame(&mut app, at);
        let children = overlay_children(&mut app, navigator);
        assert!(
            children.len() > entries_before,
            "the flight added an overlay entry"
        );
        let mut rect = top_overlay_rect(&mut app, navigator);
        assert!(
            rect.left < 80.0 && rect.width() < 60.0,
            "the shuttle starts at the source hero's bounds, not the destination's: {rect:?}"
        );

        // ... and it moves towards the destination as the route transition runs.
        for _ in 0..4 {
            at += Duration::from_millis(40);
            pump_frame(&mut app, at);
            let moved = top_overlay_rect(&mut app, navigator);
            assert!(
                moved.left > rect.left && moved.width() > rect.width(),
                "the shuttle's rect grows towards the destination: {rect:?} then {moved:?}"
            );
            rect = moved;
        }

        settle(&mut app, at);
        assert!(
            app.get(controller).flights.is_empty(),
            "the finished flight is removed from the controller"
        );
        assert_eq!(
            overlay_children(&mut app, navigator).len(),
            entries_before + 1,
            "the shuttle's entry is gone; only the pushed route's entry remains"
        );
        assert!(
            app.get(to_hero).placeholder_size.is_none(),
            "the destination hero is visible again"
        );
        assert!(
            app.get(from_hero).placeholder_size.is_some(),
            "the source hero stays hidden underneath the route on top"
        );
    }

    #[test]
    fn popping_back_flies_the_hero_home_and_clears_its_placeholder() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let controller = HeroController::new(&mut app);
        drop(app);
        mount(
            &cell,
            hero_navigator(&key, controller, hero_page(10.0, 20.0, 40.0, 30.0, true)),
        );
        let mut app = cell.borrow_mut();
        let navigator = navigator_state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let second = hero_route(
            &mut app,
            "second",
            hero_page(150.0, 100.0, 80.0, 60.0, true),
        );
        navigator.push(&mut app, second.as_route());
        let at = settle(&mut app, at + Duration::from_millis(20));
        assert!(
            app.get(controller).flights.is_empty(),
            "the push flight is over"
        );

        navigator.pop(&mut app, None);
        let mut at = at + Duration::from_millis(20);
        pump_frame(&mut app, at);
        let flight = only_flight(&app, controller);
        let manifest = flight.manifest(&app);
        assert_eq!(
            app.get(manifest).r#type,
            HeroFlightDirection::Pop,
            "a pop flies backwards"
        );
        let to_hero = app.get(manifest).to_hero;

        at += Duration::from_millis(20);
        pump_frame(&mut app, at);
        let mut rect = top_overlay_rect(&mut app, navigator);
        assert!(
            rect.left > 80.0,
            "the shuttle starts at the popped route's hero: {rect:?}"
        );
        for _ in 0..4 {
            at += Duration::from_millis(40);
            pump_frame(&mut app, at);
            let moved = top_overlay_rect(&mut app, navigator);
            assert!(
                moved.left < rect.left,
                "the shuttle flies back: {rect:?} then {moved:?}"
            );
            rect = moved;
        }

        settle(&mut app, at);
        assert!(app.get(controller).flights.is_empty());
        assert!(
            app.get(to_hero).placeholder_size.is_none(),
            "the hero that stays on screen shows its child again"
        );
    }

    #[test]
    fn a_disabled_hero_mode_keeps_the_hero_out_of_the_flight() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let controller = HeroController::new(&mut app);
        drop(app);
        mount(
            &cell,
            hero_navigator(&key, controller, hero_page(10.0, 20.0, 40.0, 30.0, false)),
        );
        let mut app = cell.borrow_mut();
        let navigator = navigator_state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        let entries_before = overlay_children(&mut app, navigator).len();

        let second = hero_route(
            &mut app,
            "second",
            hero_page(150.0, 100.0, 80.0, 60.0, true),
        );
        navigator.push(&mut app, second.as_route());
        let at = at + Duration::from_millis(20);
        pump_frame(&mut app, at);
        assert!(
            app.get(controller).flights.is_empty(),
            "the source hero is behind a disabled HeroMode, so no pair matches"
        );

        settle(&mut app, at);
        assert_eq!(
            overlay_children(&mut app, navigator).len(),
            entries_before + 1,
            "nothing but the pushed route was added to the overlay"
        );
    }

    #[test]
    fn a_custom_flight_shuttle_builder_supplies_the_in_flight_widget() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let controller = HeroController::new(&mut app);
        let built = Rc::new(std::cell::Cell::new(0u32));
        let shuttle_builder: HeroFlightShuttleBuilder = {
            let built = Rc::clone(&built);
            Rc::new(move |_app, _flight, _animation, direction, _from, _to| {
                built.set(built.get() + 1);
                assert_eq!(direction, HeroFlightDirection::Push);
                SizedBox::new().width(11.0).height(13.0).into_widget()
            })
        };
        drop(app);
        mount(
            &cell,
            hero_navigator(&key, controller, hero_page(10.0, 20.0, 40.0, 30.0, true)),
        );
        let mut app = cell.borrow_mut();
        let navigator = navigator_state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let page: RoutePageBuilder = Rc::new(move |_app, _context, _animation, _secondary| {
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(
                    Padding::new(EdgeInsetsGeometry::from_ltrb(150.0, 100.0, 0.0, 0.0)).child(
                        Hero::new(tag(), SizedBox::new().width(80.0).height(60.0))
                            .flight_shuttle_builder(shuttle_builder.clone()),
                    ),
                )
                .into_widget()
        });
        let second = hero_route(&mut app, "second", page);
        navigator.push(&mut app, second.as_route());
        let mut at = at + Duration::from_millis(20);
        pump_frame(&mut app, at);
        at += Duration::from_millis(20);
        pump_frame(&mut app, at);

        assert_eq!(built.get(), 1, "the shuttle is built once and then reused");
        // `Positioned` sizes the shuttle to the flight's rect, so the shuttle's own 11x13 only
        // shows up as the child the overlay entry built.
        assert!(!app.get(controller).flights.is_empty());
        settle(&mut app, at);
        assert!(app.get(controller).flights.is_empty());
    }

    #[test]
    fn hero_tags_match_by_value_and_type() {
        let a: HeroTagRef = Rc::new("photo");
        let b: HeroTagRef = Rc::new("photo");
        let c: HeroTagRef = Rc::new("other");
        let number: HeroTagRef = Rc::new(1_i32);
        assert!(*a == *b);
        assert!(*a != *c);
        assert!(*a != *number, "two tags of different types never match");

        let mut flights: IndexMap<HeroTagRef, u32> = IndexMap::new();
        flights.insert(Rc::clone(&a), 1);
        assert_eq!(flights.get(&b).copied(), Some(1));
        assert_eq!(flights.get(&number), None);
    }

    #[test]
    fn a_reverse_rect_tween_swaps_its_parent_ends() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let begin = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        let end = Rect::from_ltrb(20.0, 20.0, 40.0, 40.0);
        let parent: Rc<dyn RectTweenObject> =
            Rc::new(RectTween::new(&mut app, Some(begin), Some(end)));
        let reversed = ReverseRectTween {
            parent: Rc::clone(&parent),
        };
        assert_eq!(reversed.begin(&app), Some(end));
        assert_eq!(reversed.end(&app), Some(begin));
        assert_eq!(reversed.transform(&app, 0.0), Some(end));
        assert_eq!(reversed.transform(&app, 1.0), Some(begin));
        assert_eq!(
            reversed.lerp(&app, 0.25),
            parent.lerp(&app, 0.75),
            "the parent is evaluated in reverse"
        );
    }

    #[test]
    fn an_edge_insets_tween_interpolates_the_shuttle_padding() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let tween = EdgeInsetsTween::new(
            Some(EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 0.0)),
            Some(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)),
        );
        assert_eq!(
            tween.transform(&app, 0.5),
            EdgeInsets::from_ltrb(5.0, 10.0, 15.0, 20.0)
        );
        assert_eq!(
            tween.transform(&app, 1.0),
            EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)
        );
    }
}
