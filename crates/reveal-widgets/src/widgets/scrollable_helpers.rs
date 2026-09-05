//! Flutter counterpart: `widgets/scrollable_helpers.dart`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curves;
use reveal_embedder::Clip;
use reveal_foundation::{App, Handle};
use reveal_painting::{AxisDirection, axis_direction_to_axis};

use crate::framework::{BuildContext, State};
use crate::widgets::actions::{Action, ActionData, ContextAction, Intent};
use crate::widgets::primary_scroll_controller::PrimaryScrollController;
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_controller::AnyScrollController;
use crate::widgets::scroll_metrics::ScrollMetrics;
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scrollable::{Scrollable, ScrollableState};

/// Describes the aspects of a `Scrollable` widget to inform inherited widgets
/// like [`crate::ScrollBehavior`] for decorating or enumerate the properties of combined
/// scrollables, such as `TwoDimensionalScrollable`.
///
/// Decorations like `GlowingOverscrollIndicator`s and `Scrollbar`s require
/// information about the scrollable in order to be initialized.
#[derive(Clone)]
pub struct ScrollableDetails {
    /// The direction in which the widget scrolls.
    pub direction: AxisDirection,

    /// An object that can be used to control the position to which the widget is scrolled.
    pub controller: Option<AnyScrollController>,

    /// How the widgets should respond to user input.
    pub physics: Option<ScrollPhysicsRef>,

    /// The clip a decorator such as a `StretchingOverscrollIndicator` should honour.
    ///
    /// This [`Clip`] does not affect the viewport's own clip behaviour, but is rather
    /// passed from the same value by `Scrollable` so that decorators honour the same clip.
    ///
    /// Defaults to `None`.
    pub decoration_clip_behavior: Option<Clip>,
}

impl ScrollableDetails {
    /// Creates a set of details describing the `Scrollable`.
    pub fn new(direction: AxisDirection) -> ScrollableDetails {
        ScrollableDetails {
            direction,
            controller: None,
            physics: None,
            decoration_clip_behavior: None,
        }
    }

    /// A constructor specific to a `Scrollable` with an [`reveal_painting::Axis::Vertical`].
    pub fn vertical(reverse: bool) -> ScrollableDetails {
        ScrollableDetails::new(if reverse {
            AxisDirection::Up
        } else {
            AxisDirection::Down
        })
    }

    /// A constructor specific to a `Scrollable` with an [`reveal_painting::Axis::Horizontal`].
    pub fn horizontal(reverse: bool) -> ScrollableDetails {
        ScrollableDetails::new(if reverse {
            AxisDirection::Left
        } else {
            AxisDirection::Right
        })
    }

    /// Dart `ScrollableDetails(controller:)`.
    pub fn controller(mut self, controller: AnyScrollController) -> ScrollableDetails {
        self.controller = Some(controller);
        self
    }

    /// Dart `ScrollableDetails(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> ScrollableDetails {
        self.physics = Some(physics);
        self
    }

    /// Dart `ScrollableDetails(decorationClipBehavior:)`.
    pub fn decoration_clip_behavior(mut self, decoration_clip_behavior: Clip) -> ScrollableDetails {
        self.decoration_clip_behavior = Some(decoration_clip_behavior);
        self
    }

    /// Deprecated setter for
    /// [`decoration_clip_behavior`](Self::decoration_clip_behavior).
    #[deprecated(
        note = "Migrate to decoration_clip_behavior. This property was deprecated so that its \
                application is clearer. This clip applies to decorators, and does not directly \
                clip a scroll view. This feature was deprecated after v3.9.0-1.0.pre."
    )]
    pub fn clip_behavior(self, clip_behavior: Clip) -> ScrollableDetails {
        self.decoration_clip_behavior(clip_behavior)
    }

    /// Copy the current [`ScrollableDetails`] with the given values replacing the
    /// current values.
    pub fn copy_with(&self) -> ScrollableDetails {
        self.clone()
    }
}

impl PartialEq for ScrollableDetails {
    fn eq(&self, other: &ScrollableDetails) -> bool {
        self.direction == other.direction
            && self.controller == other.controller
            && match (&self.physics, &other.physics) {
                (None, None) => true,
                (Some(physics), Some(other)) => Rc::ptr_eq(physics, other),
                _ => false,
            }
            && self.decoration_clip_behavior == other.decoration_clip_behavior
    }
}

impl Debug for ScrollableDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = vec![format!("axisDirection: {:?}", self.direction)];
        if self.controller.is_some() {
            description.push("scroll controller: attached".to_string());
        }
        if let Some(physics) = &self.physics {
            description.push(format!("scroll physics: {physics:?}"));
        }
        if let Some(clip) = self.decoration_clip_behavior {
            description.push(format!("decorationClipBehavior: {clip:?}"));
        }
        write!(f, "ScrollableDetails({})", description.join(", "))
    }
}

/// A function that can calculate the offset for a type of scroll
/// increment given a [`ScrollIncrementDetails`].
///
/// This is the type of [`Scrollable::increment_calculator`], which is called from a
/// [`ScrollAction`].
pub type ScrollIncrementCalculator = Rc<dyn Fn(&ScrollIncrementDetails) -> f64>;

/// Describes the type of scroll increment that will be performed by a
/// [`ScrollAction`] on a `Scrollable`.
///
/// This is used to configure a [`ScrollIncrementDetails`] object to pass to a
/// [`ScrollIncrementCalculator`] function on a `Scrollable`.
///
/// This indicates the *intent* of the scroll, not necessarily the size. Not all
/// scrollable areas will have the concept of a "line" or "page", but they can
/// respond to the different standard key bindings that cause scrolling, which
/// are bound to keys that people use to indicate a "line" scroll (e.g.
/// control-arrowDown keys) or a "page" scroll (e.g. pageDown key). It is
/// recommended that at least the relative magnitudes of the scrolls match
/// expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollIncrementType {
    /// Indicates that the [`ScrollIncrementCalculator`] should return the scroll
    /// distance it should move when the user requests to scroll by a "line".
    ///
    /// The distance a "line" scrolls refers to what should happen when the key
    /// binding for "scroll down/up by a line" is triggered. It's up to the
    /// [`ScrollIncrementCalculator`] function to decide what that means for a
    /// particular scrollable.
    Line,

    /// Indicates that the [`ScrollIncrementCalculator`] should return the scroll
    /// distance it should move when the user requests to scroll by a "page".
    ///
    /// The distance a "page" scrolls refers to what should happen when the key
    /// binding for "scroll down/up by a page" is triggered. It's up to the
    /// [`ScrollIncrementCalculator`] function to decide what that means for a
    /// particular scrollable.
    Page,
}

/// A details object that describes the type of scroll increment being requested
/// of a [`ScrollIncrementCalculator`] function, as well as the current metrics
/// for the scrollable.
pub struct ScrollIncrementDetails {
    /// The type of scroll this is (e.g. line, page, etc.).
    pub r#type: ScrollIncrementType,

    /// The current metrics of the scrollable that is being scrolled.
    pub metrics: Rc<dyn ScrollMetrics>,
}

impl ScrollIncrementDetails {
    /// Creates a [`ScrollIncrementDetails`].
    pub fn new(
        r#type: ScrollIncrementType,
        metrics: Rc<dyn ScrollMetrics>,
    ) -> ScrollIncrementDetails {
        ScrollIncrementDetails { r#type, metrics }
    }
}

impl Debug for ScrollIncrementDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrollIncrementDetails")
            .field("type", &self.r#type)
            .field("metrics", &self.metrics)
            .finish()
    }
}

/// An [`Intent`] that represents scrolling the nearest scrollable by an amount
/// appropriate for the `type` specified.
///
/// The actual amount of the scroll is determined by the
/// [`Scrollable::increment_calculator`], or by its defaults if that is not
/// specified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollIntent {
    /// The direction in which to scroll the scrollable containing the focused
    /// widget.
    pub direction: AxisDirection,

    /// The type of scrolling that is intended.
    pub r#type: ScrollIncrementType,
}

impl ScrollIntent {
    /// Creates a [`ScrollIntent`] that requests scrolling in the given
    /// [`direction`](Self::direction), with [`ScrollIncrementType::Line`] — Dart's default.
    pub const fn new(direction: AxisDirection) -> ScrollIntent {
        ScrollIntent {
            direction,
            r#type: ScrollIncrementType::Line,
        }
    }

    /// Dart `ScrollIntent(type:)`.
    pub const fn r#type(mut self, r#type: ScrollIncrementType) -> ScrollIntent {
        self.r#type = r#type;
        self
    }
}

impl Intent for ScrollIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that scrolls the relevant `Scrollable` by the amount configured
/// in the [`ScrollIntent`] given to it.
///
/// If a `Scrollable` cannot be found above the given [`BuildContext`], the
/// [`PrimaryScrollController`] will be considered for default handling of
/// [`ScrollAction`]s.
///
/// If [`Scrollable::increment_calculator`] is `None` for the scrollable, the default
/// for a `ScrollIntent::type` set to [`ScrollIncrementType::Page`] is 80% of the
/// size of the scroll window, and for [`ScrollIncrementType::Line`], 50 logical
/// pixels.
pub struct ScrollAction {
    action: ActionData,
}

impl ScrollAction {
    /// Creates a [`ScrollAction`].
    pub fn new(app: &mut App) -> Handle<ScrollAction> {
        app.create(ScrollAction {
            action: ActionData::new(),
        })
    }

    /// Returns the scroll increment for a single scroll request, for use when
    /// scrolling using a hardware keyboard.
    ///
    /// Must not be called when the position has no pixels, or when any of the position
    /// metrics (pixels, viewportDimension, maxScrollExtent, minScrollExtent) are
    /// unavailable. The widget must have already been laid out so that the position
    /// fields are valid.
    fn calculate_scroll_increment(
        app: &mut App,
        state: Handle<ScrollableState>,
        r#type: ScrollIncrementType,
    ) -> f64 {
        let position = state.position(app);
        debug_assert!(position.has_pixels(app));
        debug_assert!(
            state
                .resolved_physics(app)
                .is_none_or(|physics| physics.should_accept_user_offset(&position.copy_with(app)))
        );
        if let Some(increment_calculator) = state.widget(app).increment_calculator.clone() {
            let metrics: Rc<dyn ScrollMetrics> = Rc::new(position.copy_with(app));
            return increment_calculator(&ScrollIncrementDetails::new(r#type, metrics));
        }
        match r#type {
            ScrollIncrementType::Line => 50.0,
            ScrollIncrementType::Page => 0.8 * position.viewport_dimension(app),
        }
    }

    /// Find out how much of an increment to move by, taking the different
    /// directions into account.
    pub fn get_directional_increment(
        app: &mut App,
        state: Handle<ScrollableState>,
        intent: &ScrollIntent,
    ) -> f64 {
        if axis_direction_to_axis(intent.direction)
            == axis_direction_to_axis(state.axis_direction(app))
        {
            let increment = ScrollAction::calculate_scroll_increment(app, state, intent.r#type);
            return if intent.direction == state.axis_direction(app) {
                increment
            } else {
                -increment
            };
        }
        0.0
    }
}

impl Action for ScrollAction {
    type Intent = ScrollIntent;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl ContextAction for ScrollAction {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        _intent: &ScrollIntent,
        context: Option<BuildContext>,
    ) -> bool {
        let Some(context) = context else {
            return false;
        };
        if Scrollable::maybe_of(app, context, None).is_some() {
            return true;
        }
        PrimaryScrollController::maybe_of(app, context)
            .is_some_and(|controller| controller.has_clients(app))
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &ScrollIntent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let context = context.expect("Cannot scroll without a context.");
        let mut state = Scrollable::maybe_of(app, context, None);
        if state.is_none() {
            let primary_scroll_controller = PrimaryScrollController::of(app, context);
            debug_assert!(
                primary_scroll_controller.positions(app).len() == 1,
                "A ScrollAction was invoked with the PrimaryScrollController, but more than one \
                 ScrollPosition is attached. Only one ScrollPosition can be manipulated by a \
                 ScrollAction at a time."
            );

            let notification_context = primary_scroll_controller
                .position(app)
                .context(app)
                .notification_context(app);
            if let Some(notification_context) = notification_context {
                state = Scrollable::maybe_of(app, notification_context, None);
            }
        }
        let state = state?;
        debug_assert!(
            state.position(app).has_pixels(app),
            "Scrollable must be laid out before it can be scrolled via a ScrollAction"
        );

        // Don't do anything if the user isn't allowed to scroll.
        let position = state.position(app);
        if let Some(physics) = state.resolved_physics(app)
            && !physics.should_accept_user_offset(&position.copy_with(app))
        {
            return None;
        }
        let increment = ScrollAction::get_directional_increment(app, state, intent);
        if increment == 0.0 {
            return None;
        }
        position.move_to(
            app,
            position.pixels(app) + increment,
            Some(Duration::from_millis(100)),
            Some(Curves::ease_in_out()),
            None,
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::cell::Cell;
    use std::collections::HashMap;

    use reveal_embedder::Offset;
    use reveal_foundation::App;
    use reveal_painting::Axis;
    use reveal_rendering::{
        AnyRenderObject, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle,
        RenderSliver, RenderSliverToBoxAdapter, RenderViewport, RenderViewportBase,
    };
    use reveal_scheduler::SchedulerBinding;

    use super::*;
    use crate::framework::{
        Element, IntoWidget, LeafRenderObjectWidget, RenderObjectWidget, WidgetRef,
    };
    use crate::test_harness::{Harness, VIEW_HEIGHT};
    use crate::widgets::actions::{Actions, AnyAction};
    use crate::widgets::basic::Builder;
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};
    use crate::widgets::scroll_physics::NeverScrollableScrollPhysics;
    use crate::widgets::scrollable::ViewportBuilder;
    use reveal_rendering::AnyViewportOffset;

    /// A leaf whose render object is a `RenderViewport` over 800 logical pixels of content.
    #[derive(Debug)]
    struct TestViewport {
        offset: AnyViewportOffset,
    }

    impl RenderObjectWidget for TestViewport {
        type RenderObject = RenderViewport;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            let content = RenderConstrainedBox::new(
                app,
                BoxConstraints::tight(reveal_embedder::Size::new(300.0, 800.0)),
                None,
            );
            let sliver = RenderSliverToBoxAdapter::new(app, Some(RenderBox::as_box(content)));
            RenderViewport::new(
                app,
                AxisDirection::Right,
                self.offset,
                Some(vec![RenderSliver::as_sliver(sliver)]),
                None,
            )
            .as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: RenderHandle<RenderViewport>,
        ) {
            RenderViewportBase::set_offset(render_object, app, self.offset);
        }
    }

    impl LeafRenderObjectWidget for TestViewport {}

    /// Mounts a `Scrollable` over 800 pixels of content under an `Actions` widget that maps
    /// [`ScrollIntent`] to a [`ScrollAction`], and hands back the context inside the viewport.
    fn mount_scroll_action(
        app: &mut App,
        configure: impl FnOnce(Scrollable) -> Scrollable,
    ) -> (Harness, Handle<ScrollableState>, BuildContext) {
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        let viewport: ViewportBuilder = Rc::new(move |_app, _context, offset| {
            let sink = Rc::clone(&sink);
            let probe = Builder::new(move |_app, context| {
                sink.set(Some(context));
                TestViewport { offset }.into_widget()
            });
            probe.into_widget()
        });
        let action = ScrollAction::new(app);
        let actions = HashMap::from([(
            TypeId::of::<ScrollIntent>(),
            Action::as_action(action) as AnyAction,
        )]);
        let scrollable = configure(Scrollable::new(viewport));
        let tree: WidgetRef =
            MediaQuery::new(MediaQueryData::new(), Actions::new(actions, scrollable)).into_widget();
        let harness = Harness::mount(app, tree);
        harness.pump(app);
        let mut element = harness.root.as_element();
        let state = loop {
            element = element.children(app)[0];
            if let Some(state) = element.state_handle::<ScrollableState>(app) {
                break state;
            }
        };
        let context = captured.get().expect("the viewport built");
        (harness, state, context)
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn a_scroll_intent_reaches_the_ambient_scrollable_through_actions() {
        let mut app = App::new();
        let (_harness, state, context) = mount_scroll_action(&mut app, |scrollable| scrollable);
        assert!(
            ScrollContext::notification_context(&state, &mut app).is_some(),
            "no notification context"
        );

        Actions::invoke(
            &mut app,
            context,
            &ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Line),
        );
        // The animation has not ticked yet, so the target is only reached after the frames.
        for frame in 0..20 {
            pump_frame(&mut app, Duration::from_millis(16 * frame));
        }
        assert_eq!(state.position(&app).pixels(&app), 50.0);

        // A page is 80% of the viewport.
        Actions::invoke(
            &mut app,
            context,
            &ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page),
        );
        for frame in 20..40 {
            pump_frame(&mut app, Duration::from_millis(16 * frame));
        }
        assert_eq!(state.position(&app).pixels(&app), 50.0 + 0.8 * VIEW_HEIGHT);

        // Scrolling back up moves by the same increment in the other direction.
        Actions::invoke(
            &mut app,
            context,
            &ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Line),
        );
        for frame in 40..60 {
            pump_frame(&mut app, Duration::from_millis(16 * frame));
        }
        assert_eq!(state.position(&app).pixels(&app), 0.8 * VIEW_HEIGHT);
    }

    #[test]
    fn a_scroll_intent_across_the_axis_moves_nothing() {
        let mut app = App::new();
        let (_harness, state, context) = mount_scroll_action(&mut app, |scrollable| scrollable);

        assert_eq!(
            ScrollAction::get_directional_increment(
                &mut app,
                state,
                &ScrollIntent::new(AxisDirection::Right)
            ),
            0.0
        );
        Actions::invoke(&mut app, context, &ScrollIntent::new(AxisDirection::Right));
        for frame in 0..20 {
            pump_frame(&mut app, Duration::from_millis(16 * frame));
        }
        assert_eq!(state.position(&app).pixels(&app), 0.0);
    }

    #[test]
    fn a_scroll_action_is_disabled_without_a_scrollable_and_refuses_physics_that_do_not_scroll() {
        let mut app = App::new();
        let action = ScrollAction::new(&mut app);
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        let harness = Harness::mount(
            &mut app,
            Builder::new(move |_app, context| {
                sink.set(Some(context));
                crate::widgets::basic::SizedBox::shrink().into_widget()
            })
            .into_widget(),
        );
        harness.pump(&mut app);
        let context = captured.get().expect("the builder ran");
        assert!(!ContextAction::is_enabled(
            action,
            &mut app,
            &ScrollIntent::new(AxisDirection::Down),
            Some(context)
        ));

        // A scrollable whose physics refuse a user offset stops the action before it moves.
        let (_harness, state, context) = mount_scroll_action(&mut app, |scrollable| {
            scrollable.physics(Rc::new(NeverScrollableScrollPhysics::new()))
        });
        Actions::invoke(&mut app, context, &ScrollIntent::new(AxisDirection::Down));
        for frame in 0..20 {
            pump_frame(&mut app, Duration::from_millis(16 * frame));
        }
        assert_eq!(state.position(&app).pixels(&app), 0.0);
    }

    #[test]
    fn scrollable_details_compare_their_axis_controller_physics_and_clip() {
        let axis = ScrollableDetails::vertical(false);
        assert_eq!(axis.direction, AxisDirection::Down);
        assert_eq!(
            ScrollableDetails::vertical(true).direction,
            AxisDirection::Up
        );
        assert_eq!(
            ScrollableDetails::horizontal(false).direction,
            AxisDirection::Right
        );
        assert_eq!(
            ScrollableDetails::horizontal(true).direction,
            AxisDirection::Left
        );

        let physics: ScrollPhysicsRef = Rc::new(NeverScrollableScrollPhysics::new());
        let with_physics = axis.copy_with().physics(Rc::clone(&physics));
        assert_ne!(with_physics, axis);
        assert_eq!(with_physics, axis.copy_with().physics(physics));
        assert_ne!(
            with_physics,
            with_physics
                .copy_with()
                .decoration_clip_behavior(Clip::None)
        );
        assert!(format!("{with_physics:?}").contains("scroll physics"));
        let _ = Axis::Vertical;
        let _ = Offset::ZERO;
    }
}
