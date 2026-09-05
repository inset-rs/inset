//! Flutter counterpart: `cupertino/scrollbar.dart`.

use std::fmt::{self, Debug};
use std::time::Duration;

use reveal_animation::{Animation, AnimationBehavior, AnimationController};
use reveal_embedder::{Color, Offset, Radius, TargetPlatform};
use reveal_foundation::{App, Handle, Listener};
use reveal_gestures::{TapDownDetails, Velocity};
use reveal_painting::EdgeInsetsGeometry;
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use reveal_widgets::{
    AnyScrollController, BuildContext, Directionality, IntoWidget, KeyRef, MediaQuery,
    RawScrollbarData, RawScrollbarLeaf, RawScrollbarState, RawScrollbarStateData,
    RawScrollbarStateLeaf, ScrollConfiguration, ScrollNotificationPredicate, ScrollbarOrientation,
    State, StateData, StatefulWidget, TickerProviderStateMixin, TickerProviderStateMixinData,
    WidgetRef, default_scroll_notification_predicate,
};

use crate::colors::CupertinoDynamicColor;

// All values eyeballed.
const K_SCROLLBAR_MIN_LENGTH: f64 = 36.0;
const K_SCROLLBAR_MIN_OVERSCROLL_LENGTH: f64 = 8.0;
const K_SCROLLBAR_TIME_TO_FADE: Duration = Duration::from_millis(1200);
const K_SCROLLBAR_FADE_DURATION: Duration = Duration::from_millis(250);
const K_SCROLLBAR_RESIZE_DURATION: Duration = Duration::from_millis(100);

/// Extracted from iOS 13.1 beta using Debug View Hierarchy.
fn k_scrollbar_color() -> CupertinoDynamicColor {
    CupertinoDynamicColor::with_brightness(Color::new(0x59000000), Color::new(0x80FFFFFF))
}

// This is the amount of space from the top of a vertical scrollbar to the top edge of the
// scrollable, measured when the vertical scrollbar overscrolls to the top.
const K_SCROLLBAR_MAIN_AXIS_MARGIN: f64 = 3.0;
const K_SCROLLBAR_CROSS_AXIS_MARGIN: f64 = 3.0;

/// An iOS style scrollbar.
///
/// To add a scrollbar to a `ScrollView`, wrap the scroll view widget in a
/// [`CupertinoScrollbar`] widget.
///
/// A scrollbar thumb indicates which portion of a `ScrollView` is actually
/// visible. By default, the thumb will fade in and out as the child scroll view
/// scrolls; when [`thumb_visibility`](Self::thumb_visibility) is true, the thumb will
/// always be visible.
///
/// When dragging a [`CupertinoScrollbar`] thumb, the thickness and radius will
/// animate from [`thickness`](Self::thickness) and [`radius`](Self::radius) to
/// [`thickness_while_dragging`](Self::thickness_while_dragging) and
/// [`radius_while_dragging`](Self::radius_while_dragging), respectively.
pub struct CupertinoScrollbar {
    raw_scrollbar: RawScrollbarData,

    /// The thickness of the scrollbar when it's being dragged by the user.
    ///
    /// When the user starts dragging the scrollbar, the thickness will animate
    /// from [`thickness`](Self::thickness) to this value, then animate back when the user
    /// stops dragging the scrollbar.
    pub thickness_while_dragging: f64,

    /// The radius of the scrollbar edges when the scrollbar is being dragged by
    /// the user.
    ///
    /// When the user starts dragging the scrollbar, the radius will animate from
    /// [`radius`](Self::radius) to this value, then animate back when the user stops
    /// dragging the scrollbar.
    pub radius_while_dragging: Radius,
}

impl CupertinoScrollbar {
    /// Default value for [`thickness`](Self::thickness) if it's not specified in
    /// [`CupertinoScrollbar`].
    pub const DEFAULT_THICKNESS: f64 = 3.0;

    /// Default value for [`thickness_while_dragging`](Self::thickness_while_dragging) if it's
    /// not specified in [`CupertinoScrollbar`].
    pub const DEFAULT_THICKNESS_WHILE_DRAGGING: f64 = 8.0;

    /// Default value for [`radius`](Self::radius) if it's not specified in
    /// [`CupertinoScrollbar`].
    pub const DEFAULT_RADIUS: Radius = Radius::circular(1.5);

    /// Default value for [`radius_while_dragging`](Self::radius_while_dragging) if it's not
    /// specified in [`CupertinoScrollbar`].
    pub const DEFAULT_RADIUS_WHILE_DRAGGING: Radius = Radius::circular(4.0);

    /// Creates an iOS style scrollbar that wraps the given child.
    ///
    /// The child should be a source of `ScrollNotification` notifications,
    /// typically a `Scrollable` widget.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoScrollbar {
        let mut raw_scrollbar = RawScrollbarData::new(child.into_widget());
        raw_scrollbar.thumb_visibility = Some(false);
        raw_scrollbar.thickness = Some(CupertinoScrollbar::DEFAULT_THICKNESS);
        raw_scrollbar.radius = Some(CupertinoScrollbar::DEFAULT_RADIUS);
        raw_scrollbar.main_axis_margin = K_SCROLLBAR_MAIN_AXIS_MARGIN;
        raw_scrollbar.fade_duration = K_SCROLLBAR_FADE_DURATION;
        raw_scrollbar.time_to_fade = K_SCROLLBAR_TIME_TO_FADE;
        raw_scrollbar.press_duration = Duration::from_millis(100);
        raw_scrollbar.notification_predicate =
            std::rc::Rc::new(default_scroll_notification_predicate);
        CupertinoScrollbar {
            raw_scrollbar,
            thickness_while_dragging: CupertinoScrollbar::DEFAULT_THICKNESS_WHILE_DRAGGING,
            radius_while_dragging: CupertinoScrollbar::DEFAULT_RADIUS_WHILE_DRAGGING,
        }
    }

    /// Dart `CupertinoScrollbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoScrollbar {
        self.raw_scrollbar.key = Some(key);
        self
    }

    /// Dart `CupertinoScrollbar(controller:)`.
    pub fn controller(mut self, controller: AnyScrollController) -> CupertinoScrollbar {
        self.raw_scrollbar.controller = Some(controller);
        self
    }

    /// Dart `CupertinoScrollbar(thumbVisibility:)`.
    pub fn thumb_visibility(mut self, thumb_visibility: bool) -> CupertinoScrollbar {
        self.raw_scrollbar.thumb_visibility = Some(thumb_visibility);
        self
    }

    /// Dart `CupertinoScrollbar(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> CupertinoScrollbar {
        debug_assert!(thickness < f64::INFINITY);
        self.raw_scrollbar.thickness = Some(thickness);
        self
    }

    /// Dart `CupertinoScrollbar(thicknessWhileDragging:)`.
    pub fn thickness_while_dragging(mut self, thickness_while_dragging: f64) -> CupertinoScrollbar {
        debug_assert!(thickness_while_dragging < f64::INFINITY);
        self.thickness_while_dragging = thickness_while_dragging;
        self
    }

    /// Dart `CupertinoScrollbar(radius:)`.
    pub fn radius(mut self, radius: Radius) -> CupertinoScrollbar {
        self.raw_scrollbar.radius = Some(radius);
        self
    }

    /// Dart `CupertinoScrollbar(radiusWhileDragging:)`.
    pub fn radius_while_dragging(mut self, radius_while_dragging: Radius) -> CupertinoScrollbar {
        self.radius_while_dragging = radius_while_dragging;
        self
    }

    /// Dart `CupertinoScrollbar(notificationPredicate:)`.
    pub fn notification_predicate(
        mut self,
        notification_predicate: ScrollNotificationPredicate,
    ) -> CupertinoScrollbar {
        self.raw_scrollbar.notification_predicate = notification_predicate;
        self
    }

    /// Dart `CupertinoScrollbar(scrollbarOrientation:)`.
    pub fn scrollbar_orientation(
        mut self,
        scrollbar_orientation: ScrollbarOrientation,
    ) -> CupertinoScrollbar {
        self.raw_scrollbar.scrollbar_orientation = Some(scrollbar_orientation);
        self
    }

    /// Dart `CupertinoScrollbar(mainAxisMargin:)`.
    pub fn main_axis_margin(mut self, main_axis_margin: f64) -> CupertinoScrollbar {
        self.raw_scrollbar.main_axis_margin = main_axis_margin;
        self
    }
}

impl RawScrollbarLeaf for CupertinoScrollbar {
    fn raw_scrollbar_data(&self) -> &RawScrollbarData {
        &self.raw_scrollbar
    }

    fn raw_scrollbar_data_mut(&mut self) -> &mut RawScrollbarData {
        &mut self.raw_scrollbar
    }
}

impl Debug for CupertinoScrollbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoScrollbar")
            .field("thicknessWhileDragging", &self.thickness_while_dragging)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoScrollbar {
    type State = CupertinoScrollbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.raw_scrollbar.key.as_ref()
    }

    fn create_state(&self) -> CupertinoScrollbarState {
        CupertinoScrollbarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            raw_scrollbar_state: RawScrollbarStateData::new(),
            thickness_animation_controller: None,
        }
    }
}

/// Dart's `_CupertinoScrollbarState`: the [`CupertinoScrollbar`]'s state over
/// [`RawScrollbarState`]'s shape.
pub struct CupertinoScrollbarState {
    state: StateData<CupertinoScrollbar>,
    ticker_provider: TickerProviderStateMixinData,
    raw_scrollbar_state: RawScrollbarStateData,
    thickness_animation_controller: Option<Handle<AnimationController>>,
}

impl CupertinoScrollbarState {
    fn thickness_animation_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<AnimationController> {
        app.get(self)
            .thickness_animation_controller
            .expect("initState creates the controller")
    }

    fn thickness(self: Handle<Self>, app: &mut App) -> f64 {
        let value = self.thickness_animation_controller(app).value(app);
        let widget = self.widget(app);
        let thickness = widget
            .raw_scrollbar
            .thickness
            .expect("the constructor always supplies a thickness");
        thickness + value * (widget.thickness_while_dragging - thickness)
    }

    fn radius(self: Handle<Self>, app: &mut App) -> Radius {
        let value = self.thickness_animation_controller(app).value(app);
        let widget = self.widget(app);
        Radius::lerp(
            widget.raw_scrollbar.radius,
            Some(widget.radius_while_dragging),
            value,
        )
        .expect("both radii are non-null")
    }
}

impl State for CupertinoScrollbarState {
    type Widget = CupertinoScrollbar;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::init_state(self, app);
        let controller = AnimationController::create(
            app,
            None,
            Some(K_SCROLLBAR_RESIZE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).thickness_animation_controller = Some(controller);
        controller.add_listener(
            app,
            Listener::new(move |app: &mut App| {
                self.update_scrollbar_painter(app);
            }),
        );
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::did_change_dependencies(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoScrollbar) {
        RawScrollbarState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.thickness_animation_controller(app).dispose(app);
        RawScrollbarState::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawScrollbarState::build(self, app, context)
    }
}

impl RawScrollbarStateLeaf for CupertinoScrollbarState {
    reveal_widgets::raw_scrollbar_state_accessors!();

    fn update_scrollbar_painter(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let color = CupertinoDynamicColor::resolve(&k_scrollbar_color().into_any(), app, context);
        let text_direction = Directionality::of(app, context);
        let thickness = self.thickness(app);
        let radius = self.radius(app);
        let padding = MediaQuery::padding_of(app, context);
        let (main_axis_margin, scrollbar_orientation) = {
            let widget = self.widget(app);
            (
                widget.raw_scrollbar.main_axis_margin,
                widget.raw_scrollbar.scrollbar_orientation,
            )
        };

        let painter = RawScrollbarState::scrollbar_painter(self, app);
        painter.set_color(app, color.color());
        painter.set_text_direction(app, Some(text_direction));
        painter.set_thickness(app, thickness);
        painter.set_main_axis_margin(app, main_axis_margin);
        painter.set_cross_axis_margin(app, K_SCROLLBAR_CROSS_AXIS_MARGIN);
        painter.set_radius(app, Some(radius));
        painter.set_padding(app, EdgeInsetsGeometry::from(padding));
        painter.set_min_length(app, K_SCROLLBAR_MIN_LENGTH);
        painter.set_min_overscroll_length(app, K_SCROLLBAR_MIN_OVERSCROLL_LENGTH);
        painter.set_scrollbar_orientation(app, scrollbar_orientation);
    }

    // Drag event callbacks handle the gesture where the user presses on the scrollbar thumb
    // and then drags the scrollbar without releasing.

    fn handle_thumb_press(self: Handle<Self>, app: &mut App) {
        if self.get_scrollbar_direction(app).is_none() {
            return;
        }
        RawScrollbarState::handle_thumb_press(self, app);
        self.thickness_animation_controller(app).forward(app, None);
    }

    fn handle_thumb_press_end(
        self: Handle<Self>,
        app: &mut App,
        local_position: Offset,
        velocity: Velocity,
    ) {
        if self.get_scrollbar_direction(app).is_none() {
            return;
        }
        self.thickness_animation_controller(app).reverse(app, None);
        RawScrollbarState::handle_thumb_press_end(self, app, local_position, velocity);
    }

    fn handle_track_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        // On iOS, tapping the track does not page towards the position of the tap.
        let context = self.context(app);
        if ScrollConfiguration::of(app, context).get_platform(app, context) != TargetPlatform::IOS {
            RawScrollbarState::handle_track_tap_down(self, app, details);
        }
    }
}

impl TickerProviderObject for CupertinoScrollbarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for CupertinoScrollbarState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl Debug for CupertinoScrollbarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CupertinoScrollbarState")
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use reveal_embedder::{Size, TextDirection};
    use reveal_widgets::{
        Directionality, GlobalKey, ListView, MediaQueryData, ScrollController,
        ScrollControllerLeaf, SizedBox,
    };

    use super::*;
    use crate::test_support::{build, pump, test_cell};

    const ITEM_EXTENT: f64 = 50.0;

    #[test]
    fn a_cupertino_scrollbar_thickens_while_its_thumb_is_pressed() {
        let cell = test_cell();
        let mut app = cell.borrow_mut();
        let key = Rc::new(GlobalKey::new());
        let controller = ScrollController::default(&mut app);
        let list = ListView::new()
            .controller(controller.as_controller())
            .children((0..40).map(|_| SizedBox::new().height(ITEM_EXTENT).into_widget()));
        let scrollbar = CupertinoScrollbar::new(list)
            .key(key.clone())
            .controller(controller.as_controller());
        let tree: WidgetRef = MediaQuery::new(
            MediaQueryData::new().size(Size::new(400.0, 300.0)),
            scrollbar,
        )
        .into_widget();
        drop(app);
        build(
            &cell,
            Directionality::new(TextDirection::Ltr, tree).into_widget(),
        );
        let mut app = cell.borrow_mut();
        // The metrics notification is dispatched from a microtask after layout, so the
        // painter only knows the axis from the second frame on.
        pump(&mut app, Duration::from_millis(16));

        let state = key
            .current_state::<CupertinoScrollbarState>(&mut app)
            .expect("the scrollbar is mounted");
        let painter = RawScrollbarState::scrollbar_painter(state, &app);
        assert_eq!(
            painter.thickness(&app),
            CupertinoScrollbar::DEFAULT_THICKNESS,
            "the resting thickness"
        );

        state.handle_thumb_press(&mut app);
        for step in 1..10 {
            pump(&mut app, Duration::from_millis(16 + step * 20));
        }
        assert_eq!(
            painter.thickness(&app),
            CupertinoScrollbar::DEFAULT_THICKNESS_WHILE_DRAGGING,
            "a pressed thumb thickens"
        );
        assert_eq!(
            painter.radius(&app),
            Some(CupertinoScrollbar::DEFAULT_RADIUS_WHILE_DRAGGING),
            "and takes the dragging radius"
        );
    }
}
