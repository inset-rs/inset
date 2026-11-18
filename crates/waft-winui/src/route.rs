//! WinUI's page navigation: a page enters as a `Frame` shows it, with
//! `NavigationThemeTransition` and its default `EntranceNavigationTransitionInfo`, and the page
//! it covers leaves and returns with the same transition's other storyboards.
//!
//! The values are the ones `ThemeTransitions.cpp` registers for each navigation trigger:
//!
//! - the entering page (`NavigatingTo`) starts 140 effective pixels lower, stays fully
//!   transparent for 150 ms, then slides up over 300 ms on the cubic `(0.1, 0.9, 0.2, 1.0)`
//!   with its opacity switched on at 150 ms;
//! - the covered page (`NavigatingAway`) fades out over 150 ms on the cubic
//!   `(0.7, 0.0, 1.0, 0.5)`;
//! - the page revealed by a back (`BackNavigatingTo`) stays invisible for 150 ms, then fades in
//!   over 300 ms on `(0.1, 0.9, 0.2, 1.0)`;
//! - the page leaving by a back (`BackNavigatingAway`) slides down 140 pixels over 150 ms on
//!   `(0.7, 0.0, 1.0, 0.5)` and is hidden at the end.

use std::any::Any;
use std::time::Duration;

use waft_animation::{AnimationRef, AnimationStatus, Cubic, Curve};
use waft_embedder::{Color, Offset};
use waft_foundation::Ref;
use waft_scheduler::TickerFuture;
use waft_widgets::{
    AnimatedBuilder, AnyModalRoute, AnyPageRoute, AnyRoute, AnyTransitionRoute, BuildContext,
    DelegatedTransitionBuilder, IntoWidget, LocalHistoryRoute, LocalHistoryRouteData, ModalRoute,
    ModalRouteData, Opacity, OverlayEntry, OverlayRoute, OverlayRouteData, PageRoute,
    PageRouteData, PredictiveBackRoute, Route, RouteData, RoutePopDisposition, RouteResult,
    RouteSettingsRef, Semantics, Transform, TransitionRoute, TransitionRouteData, WidgetBuilder,
    WidgetRef,
};

/// A page in a Fluent app, entering and leaving with WinUI's `Frame` transition.
///
/// Shaped as `MaterialPageRoute` is: a [`PageRoute`] whose
/// [`build_transitions`](ModalRoute::build_transitions) plays WinUI's entrance for its own page
/// and its other storyboards for the page it covers or reveals.
///
/// By default, when the route is replaced, the previous route remains in memory. Set
/// [`maintain_state`](Self::maintain_state) to false to free its resources.
///
/// See also:
///
///  * [`WidgetsApp::page_route_builder`](waft_widgets::WidgetsApp::page_route_builder), which a
///    Fluent app points at this route.
///  * [`FluentApp`](crate::FluentApp), which builds its pages with it.
pub struct FluentPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    /// Builds the page's content.
    pub builder: WidgetBuilder,
    maintain_state: bool,
    include_route_semantics: bool,
}

impl FluentPageRoute {
    /// How long a page takes to enter: the 150 ms the entrance stays still, then the 300 ms it
    /// slides up.
    pub const TRANSITION_DURATION: Duration = Duration::from_millis(450);

    /// How long a page takes to leave and the one beneath to return: the leaving page's 150 ms
    /// and the revealed page's 300 ms that follows it.
    pub const REVERSE_TRANSITION_DURATION: Duration = Duration::from_millis(450);

    /// How far below its place an entering page starts, in effective pixels.
    pub const TRANSLATION_OFFSET: f64 = 140.0;

    /// The 150 ms the entering page waits, and the covered page takes to fade, out of
    /// [`TRANSITION_DURATION`](Self::TRANSITION_DURATION).
    const OUT_DURATION: Duration = Duration::from_millis(150);

    /// The curve the entering page slides up on, and the revealed page fades in on.
    const IN_CURVE: Cubic = Cubic::new(0.1, 0.9, 0.2, 1.0);

    /// The curve the leaving and covered pages move on.
    const OUT_CURVE: Cubic = Cubic::new(0.7, 0.0, 1.0, 0.5);

    /// Creates a page route for `builder`'s page; Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `include_route_semantics` true, `barrier_dismissible` false.
    pub fn new(app: &mut waft_foundation::App, builder: WidgetBuilder) -> Ref<FluentPageRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let this = app.create(FluentPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            builder,
            maintain_state: true,
            include_route_semantics: true,
        });
        debug_assert!(TransitionRoute::opaque(this.handle(), app));
        this
    }

    /// The route `settings`, as Dart's `PageRoute` takes them.
    pub fn settings(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        settings: RouteSettingsRef,
    ) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Whether the route asks for focus when pushed, as Dart's `PageRoute` takes it.
    pub fn request_focus(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        request_focus: bool,
    ) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `MaterialPageRoute(maintainState:)`.
    pub fn maintain_state(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        maintain_state: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).maintain_state = maintain_state;
        self
    }

    /// Dart `MaterialPageRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        fullscreen_dialog: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `MaterialPageRoute(allowSnapshotting:)`.
    pub fn allow_snapshotting(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        allow_snapshotting: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `MaterialPageRoute(includeRouteSemantics:)`.
    pub fn include_route_semantics(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        include: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).include_route_semantics = include;
        self
    }

    /// Dart `MaterialPageRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Ref<Self>,
        app: &mut waft_foundation::App,
        dismissible: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).barrier_dismissible = dismissible;
        self
    }
}

impl Route for FluentPageRoute {
    waft_widgets::route_accessors!();

    fn as_transition_route(self: waft_foundation::Handle<Self>) -> Option<AnyTransitionRoute> {
        Some(TransitionRoute::as_transition_route(self))
    }

    fn as_modal_route(self: waft_foundation::Handle<Self>) -> Option<AnyModalRoute> {
        Some(ModalRoute::as_modal_route(self))
    }

    fn as_page_route(self: waft_foundation::Handle<Self>) -> Option<AnyPageRoute> {
        Some(PageRoute::as_page_route(self))
    }

    fn interface(
        self: waft_foundation::Handle<Self>,
        _id: std::any::TypeId,
    ) -> Option<Box<dyn Any>> {
        let _ = self;
        None
    }

    fn overlay_entries(
        self: waft_foundation::Handle<Self>,
        app: &waft_foundation::App,
    ) -> Vec<waft_foundation::Handle<OverlayEntry>> {
        OverlayRoute::overlay_entries(self, app)
    }

    fn install(self: waft_foundation::Handle<Self>, app: &mut waft_foundation::App) {
        ModalRoute::install(self, app)
    }

    fn did_push(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
    ) -> TickerFuture {
        ModalRoute::did_push(self, app)
    }

    fn did_add(self: waft_foundation::Handle<Self>, app: &mut waft_foundation::App) {
        ModalRoute::did_add(self, app)
    }

    fn did_replace(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        old_route: Option<AnyRoute>,
    ) {
        TransitionRoute::did_replace(self, app, old_route)
    }

    fn will_pop(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
    ) -> RoutePopDisposition {
        LocalHistoryRoute::will_pop(self, app)
    }

    fn pop_disposition(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
    ) -> RoutePopDisposition {
        ModalRoute::pop_disposition(self, app)
    }

    fn on_pop_invoked_with_result(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        did_pop: bool,
        result: RouteResult,
    ) {
        ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
    }

    fn will_handle_pop_internally(
        self: waft_foundation::Handle<Self>,
        app: &waft_foundation::App,
    ) -> bool {
        LocalHistoryRoute::will_handle_pop_internally(self, app)
    }

    fn did_pop(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        result: RouteResult,
    ) -> bool {
        LocalHistoryRoute::did_pop(self, app, result)
    }

    fn did_pop_next(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        next_route: AnyRoute,
    ) {
        ModalRoute::did_pop_next(self, app, next_route)
    }

    fn did_change_next(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        next_route: Option<AnyRoute>,
    ) {
        ModalRoute::did_change_next(self, app, next_route)
    }

    fn did_change_previous(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        previous_route: Option<AnyRoute>,
    ) {
        ModalRoute::did_change_previous(self, app, previous_route)
    }

    fn changed_internal_state(self: waft_foundation::Handle<Self>, app: &mut waft_foundation::App) {
        ModalRoute::changed_internal_state(self, app)
    }

    fn changed_external_state(self: waft_foundation::Handle<Self>, app: &mut waft_foundation::App) {
        ModalRoute::changed_external_state(self, app)
    }

    fn dispose(self: waft_foundation::Handle<Self>, app: &mut waft_foundation::App) {
        TransitionRoute::dispose(self, app)
    }
}

impl OverlayRoute for FluentPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for FluentPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for FluentPageRoute {
    waft_widgets::modal_transition_route_overrides!();

    fn opaque(self: waft_foundation::Handle<Self>, app: &waft_foundation::App) -> bool {
        PageRoute::opaque(self, app)
    }

    fn allow_snapshotting(self: waft_foundation::Handle<Self>, app: &waft_foundation::App) -> bool {
        PageRoute::allow_snapshotting(self, app)
    }

    fn transition_duration(
        self: waft_foundation::Handle<Self>,
        _app: &mut waft_foundation::App,
    ) -> Duration {
        let _ = self;
        FluentPageRoute::TRANSITION_DURATION
    }

    fn reverse_transition_duration(
        self: waft_foundation::Handle<Self>,
        _app: &mut waft_foundation::App,
    ) -> Duration {
        let _ = self;
        FluentPageRoute::REVERSE_TRANSITION_DURATION
    }

    /// The page beneath runs its covered animation when another page enters on top of it.
    fn can_transition_to(
        self: waft_foundation::Handle<Self>,
        app: &waft_foundation::App,
        next_route: AnyTransitionRoute,
    ) -> bool {
        let next = next_route.as_route();
        match next.as_page_route(app) {
            Some(page) => !page.as_modal_route().fullscreen_dialog(app),
            None => false,
        }
    }

    /// A page revealed by a back fades in as the page above leaves.
    fn can_transition_from(
        self: waft_foundation::Handle<Self>,
        app: &waft_foundation::App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        previous_route.as_route().as_page_route(app).is_some()
            && !PageRoute::fullscreen_dialog(self, app)
    }
}

impl LocalHistoryRoute for FluentPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for FluentPageRoute {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(
        self: waft_foundation::Handle<Self>,
        app: &waft_foundation::App,
    ) -> bool {
        PageRoute::barrier_dismissible(self, app)
    }

    fn barrier_color(
        self: waft_foundation::Handle<Self>,
        _app: &waft_foundation::App,
    ) -> Option<Color> {
        let _ = self;
        None
    }

    fn barrier_label(
        self: waft_foundation::Handle<Self>,
        _app: &waft_foundation::App,
    ) -> Option<String> {
        let _ = self;
        None
    }

    fn delegated_transition(
        self: waft_foundation::Handle<Self>,
        _app: &waft_foundation::App,
    ) -> Option<DelegatedTransitionBuilder> {
        let _ = self;
        None
    }

    fn maintain_state(self: waft_foundation::Handle<Self>, app: &waft_foundation::App) -> bool {
        app.get(self).maintain_state
    }

    fn fullscreen_dialog(self: waft_foundation::Handle<Self>, app: &waft_foundation::App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn build_page(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        let builder = std::rc::Rc::clone(&app.get(self).builder);
        let result = builder(app, context);
        if !app.get(self).include_route_semantics {
            return result;
        }
        Semantics::new()
            .scopes_route(true)
            .explicit_child_nodes(true)
            .child(result)
            .into_widget()
    }

    fn build_transitions(
        self: waft_foundation::Handle<Self>,
        app: &mut waft_foundation::App,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = self;
        fluent_transitions(app, animation, secondary_animation, child)
    }
}

impl PageRoute for FluentPageRoute {
    waft_widgets::page_route_accessors!();
}

/// The entrance on the route's own animation and the covered page's fade on its secondary one.
fn fluent_transitions(
    _app: &mut waft_foundation::App,
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    child: WidgetRef,
) -> WidgetRef {
    let primary: std::rc::Rc<dyn waft_foundation::Listenable> = std::rc::Rc::new(animation.clone());
    let secondary: std::rc::Rc<dyn waft_foundation::Listenable> =
        std::rc::Rc::new(secondary_animation.clone());
    let entering = AnimatedBuilder::new(primary, move |app, _context, child| {
        let (offset, opacity) = entrance(
            animation.value(app),
            animation.status(app) == AnimationStatus::Reverse,
        );
        let child = child
            .cloned()
            .unwrap_or_else(|| waft_widgets::SizedBox::shrink().into_widget());
        Opacity::new(opacity)
            .child(Transform::translate(Offset::new(0.0, offset)).child(child))
            .into_widget()
    })
    .child(child)
    .into_widget();
    AnimatedBuilder::new(secondary, move |app, _context, child| {
        let opacity = covered(
            secondary_animation.value(app),
            secondary_animation.status(app) == AnimationStatus::Reverse,
        );
        let child = child
            .cloned()
            .unwrap_or_else(|| waft_widgets::SizedBox::shrink().into_widget());
        Opacity::new(opacity).child(child).into_widget()
    })
    .child(entering)
    .into_widget()
}

/// How far down the page is and how visible, for `value` of its own animation: the entrance
/// while it runs forward, `BackNavigatingAway` while it reverses.
fn entrance(value: f64, reversing: bool) -> (f64, f64) {
    let duration = FluentPageRoute::TRANSITION_DURATION.as_secs_f64();
    let waited = FluentPageRoute::OUT_DURATION.as_secs_f64();
    if reversing {
        // The leaving page is away after 150 ms of the 450 ms a back takes; the rest of the
        // time belongs to the page being revealed.
        let elapsed = (1.0 - value) * duration;
        let progress = if elapsed >= waited {
            1.0
        } else {
            elapsed / waited
        };
        (
            FluentPageRoute::TRANSLATION_OFFSET * FluentPageRoute::OUT_CURVE.transform(progress),
            if progress < 1.0 { 1.0 } else { 0.0 },
        )
    } else {
        let elapsed = value * duration;
        let progress = ((elapsed - waited) / (duration - waited)).clamp(0.0, 1.0);
        (
            FluentPageRoute::TRANSLATION_OFFSET
                * (1.0 - FluentPageRoute::IN_CURVE.transform(progress)),
            if elapsed >= waited { 1.0 } else { 0.0 },
        )
    }
}

/// How visible the page is when it is covered, for `value` of its secondary animation: fading
/// out over 150 ms as a page enters, and from 150 ms into a back, fading in over 300 ms.
fn covered(value: f64, reversing: bool) -> f64 {
    let duration = FluentPageRoute::TRANSITION_DURATION.as_secs_f64();
    let waited = FluentPageRoute::OUT_DURATION.as_secs_f64();
    if reversing {
        let elapsed = (1.0 - value) * duration;
        if elapsed < waited {
            0.0
        } else if elapsed >= duration {
            1.0
        } else {
            FluentPageRoute::IN_CURVE.transform((elapsed - waited) / (duration - waited))
        }
    } else {
        let elapsed = (value * duration).min(waited);
        let progress = (elapsed / waited).clamp(0.0, 1.0);
        1.0 - FluentPageRoute::OUT_CURVE.transform(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entrance_waits_150_ms_invisible_140_pixels_low_then_slides_up() {
        let at = |ms: f64| entrance(ms / 450.0, false);
        assert_eq!(at(0.0), (140.0, 0.0));
        assert_eq!(at(149.0), (140.0, 0.0), "still waiting");
        assert_eq!(at(150.0), (140.0, 1.0), "the opacity keyframe at 150 ms");
        let (offset, opacity) = at(300.0);
        assert!(opacity == 1.0, "visible once the slide starts");
        assert!(
            offset > 0.0 && offset < 140.0,
            "already moving up: {offset}"
        );
        assert_eq!(at(450.0), (0.0, 1.0));
    }

    #[test]
    fn leaving_slides_down_over_150_ms_and_the_rest_of_the_back_belongs_to_the_page_below() {
        let at = |ms: f64| entrance(1.0 - ms / 450.0, true);
        assert_eq!(at(0.0), (0.0, 1.0));
        let (offset, opacity) = at(75.0);
        assert!(offset > 0.0 && offset < 140.0, "moving down: {offset}");
        assert_eq!(opacity, 1.0);
        assert_eq!(at(151.0), (140.0, 0.0));
        assert_eq!(at(450.0), (140.0, 0.0), "away while the page below returns");
    }

    #[test]
    fn a_covered_page_fades_out_in_150_ms() {
        assert_eq!(covered(0.0, false), 1.0);
        assert!(covered(75.0 / 450.0, false) > 0.0);
        assert_eq!(covered(150.0 / 450.0, false), 0.0);
        assert_eq!(covered(1.0, false), 0.0);
    }

    #[test]
    fn a_page_revealed_by_a_back_fades_in_after_150_ms() {
        let at = |ms: f64| covered(1.0 - ms / 450.0, true);
        assert_eq!(at(0.0), 0.0, "still covered at the start");
        assert_eq!(at(150.0), 0.0, "invisible until 150 ms");
        let middle = at(300.0);
        assert!(middle > 0.0 && middle < 1.0, "fading in: {middle}");
        assert_eq!(at(450.0), 1.0);
    }

    #[test]
    fn the_route_is_a_page_route_with_its_data() {
        let cell = waft_foundation::AppCell::new();
        let mut app = cell.borrow_mut();
        let route = FluentPageRoute::new(
            &mut app,
            std::rc::Rc::new(|_app, _context| waft_widgets::SizedBox::shrink().into_widget()),
        );
        let erased: waft_widgets::RouteRef = route.into_erased();
        assert!(erased.as_page_route(&app).is_some());
        assert!(erased.as_modal_route(&app).is_some());
        assert!(erased.as_transition_route(&app).is_some());
    }

    #[test]
    fn the_route_uses_winuis_durations() {
        assert_eq!(FluentPageRoute::TRANSITION_DURATION.as_millis(), 450);
        assert_eq!(
            FluentPageRoute::REVERSE_TRANSITION_DURATION.as_millis(),
            450
        );
        assert_eq!(FluentPageRoute::TRANSLATION_OFFSET, 140.0);
    }
}
