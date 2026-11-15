//! Flutter counterpart: `cupertino/refresh.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::{Curve, Curves, Interval};
use waft_embedder::{Clip, Matrix4, Offset, clamp_double};
use waft_foundation::{App, Handle, Task};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, PaintingContext, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderSliver, RenderSliverData,
    SliverGeometry,
};
use waft_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};
use waft_services::HapticFeedback;
use waft_widgets::{
    BuildContext, Center, IntoWidget, KeyRef, LayoutBuilder, LimitedBox, Opacity, Positioned,
    RenderObjectWidget, SingleChildRenderObjectWidget, SizedBox, Stack, State, StateData,
    StatefulWidget, WidgetRef,
};

use crate::activity_indicator::CupertinoActivityIndicator;

const K_ACTIVITY_INDICATOR_RADIUS: f64 = 14.0;
const K_ACTIVITY_INDICATOR_MARGIN: f64 = 16.0;

/// Dart's `_CupertinoSliverRefresh`.
#[derive(Debug, Default)]
struct CupertinoSliverRefresh {
    // The amount of space the indicator should occupy in the sliver in a
    // resting state when in the refreshing mode.
    refresh_indicator_layout_extent: f64,
    // RenderCupertinoSliverRefresh will paint the child in the available
    // space either way but this instructs the RenderCupertinoSliverRefresh
    // on whether to also occupy any layout_extent space or not.
    has_layout_extent: bool,
    child: Option<WidgetRef>,
}

impl CupertinoSliverRefresh {
    fn new() -> CupertinoSliverRefresh {
        CupertinoSliverRefresh::default()
    }

    fn refresh_indicator_layout_extent(
        mut self,
        refresh_indicator_layout_extent: f64,
    ) -> CupertinoSliverRefresh {
        debug_assert!(refresh_indicator_layout_extent >= 0.0);
        self.refresh_indicator_layout_extent = refresh_indicator_layout_extent;
        self
    }

    fn has_layout_extent(mut self, has_layout_extent: bool) -> CupertinoSliverRefresh {
        self.has_layout_extent = has_layout_extent;
        self
    }

    fn child<K>(mut self, child: impl IntoWidget<K>) -> CupertinoSliverRefresh {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for CupertinoSliverRefresh {
    type RenderObject = RenderCupertinoSliverRefresh;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCupertinoSliverRefresh::new(
            app,
            self.refresh_indicator_layout_extent,
            self.has_layout_extent,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCupertinoSliverRefresh>,
    ) {
        render_object
            .set_refresh_indicator_layout_extent(app, self.refresh_indicator_layout_extent);
        render_object.set_has_layout_extent(app, self.has_layout_extent);
    }
}

impl SingleChildRenderObjectWidget for CupertinoSliverRefresh {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderCupertinoSliverRefresh`.
///
/// RenderSliver object that gives its child RenderBox object space to paint in the
/// overscrolled gap and may or may not hold that overscrolled gap around the RenderBox
/// depending on whether `layout_extent` is set.
///
/// The [`layout_extent_offset_compensation`](Self::layout_extent_offset_compensation) field
/// keeps internal accounting to prevent scroll position jumps as the `layout_extent` is set
/// and unset.
struct RenderCupertinoSliverRefresh {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    refresh_indicator_extent: f64,
    has_layout_extent: bool,
    // This keeps track of the previously applied scroll offsets to the scrollable
    // so that when `refresh_indicator_layout_extent` or `has_layout_extent` changes,
    // the appropriate delta can be applied to keep everything in the same place
    // visually.
    layout_extent_offset_compensation: f64,
}

impl RenderCupertinoSliverRefresh {
    fn new(
        app: &mut App,
        refresh_indicator_extent: f64,
        has_layout_extent: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderCupertinoSliverRefresh> {
        debug_assert!(refresh_indicator_extent >= 0.0);
        let this = RenderHandle::new_sliver(
            app,
            RenderCupertinoSliverRefresh {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                refresh_indicator_extent,
                has_layout_extent,
                layout_extent_offset_compensation: 0.0,
            },
        );
        this.set_child(app, child);
        this
    }

    // The amount of layout space the indicator should occupy in the sliver in a
    // resting state when in the refreshing mode.
    fn refresh_indicator_layout_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).refresh_indicator_extent
    }

    fn set_refresh_indicator_layout_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!(value >= 0.0);
        if value == self.refresh_indicator_layout_extent(app) {
            return;
        }
        self.get_mut(app).refresh_indicator_extent = value;
        self.mark_needs_layout(app);
    }

    // The child box will be laid out and painted in the available space either
    // way but this determines whether to also occupy any
    // `SliverGeometry::layout_extent` space or not.
    fn has_layout_extent(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_layout_extent
    }

    fn set_has_layout_extent(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.has_layout_extent(app) {
            return;
        }
        self.get_mut(app).has_layout_extent = value;
        self.mark_needs_layout(app);
    }
}

impl Debug for RenderCupertinoSliverRefresh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_RenderCupertinoSliverRefresh")
            .field("refresh_indicator_extent", &self.refresh_indicator_extent)
            .field("has_layout_extent", &self.has_layout_extent)
            .finish()
    }
}

impl RenderObjectWithChildMixin for RenderCupertinoSliverRefresh {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderObject for RenderCupertinoSliverRefresh {
    waft_rendering::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        // Only pulling to refresh from the top is currently supported.
        debug_assert_eq!(
            constraints.axis_direction,
            waft_painting::AxisDirection::Down
        );
        debug_assert_eq!(
            constraints.growth_direction,
            waft_rendering::GrowthDirection::Forward
        );

        let this = self.get(app);
        // The new layout extent this sliver should now have.
        let layout_extent =
            (if this.has_layout_extent { 1.0 } else { 0.0 }) * this.refresh_indicator_extent;
        // If the new layout_extent instructive changed, the SliverGeometry's
        // layout_extent will take that value (on the next perform_layout run). Shift
        // the scroll offset first so it doesn't make the scroll position suddenly jump.
        if layout_extent != this.layout_extent_offset_compensation {
            let correction = layout_extent - this.layout_extent_offset_compensation;
            self.set_geometry(
                app,
                SliverGeometry::builder()
                    .scroll_offset_correction(correction)
                    .build(),
            );
            self.get_mut(app).layout_extent_offset_compensation = layout_extent;
            // Return so we don't have to do temporary accounting and adjusting the
            // child's constraints accounting for this one transient frame using a
            // combination of existing layout extent, new layout extent change and
            // the overlap.
            return;
        }

        let active = constraints.overlap < 0.0 || layout_extent > 0.0;
        let overscrolled_extent = if constraints.overlap < 0.0 {
            constraints.overlap.abs()
        } else {
            0.0
        };
        let child = self
            .child(app)
            .expect("a CupertinoSliverRefresh always has its LayoutBuilder child");
        // Layout the child giving it the space of the currently dragged overscroll
        // which may or may not include a sliver layout extent space that it will
        // keep after the user lets go during the refresh process.
        child.layout(
            app,
            constraints.as_box_constraints(
                0.0,
                layout_extent
                    // Plus only the overscrolled portion immediately preceding this
                    // sliver.
                    + overscrolled_extent,
                None,
            ),
            true,
        );
        if active {
            let child_height = child.size(app).height();
            self.set_geometry(
                app,
                SliverGeometry::builder()
                    .scroll_extent(layout_extent)
                    .paint_origin(-overscrolled_extent - constraints.scroll_offset)
                    .paint_extent(f64::max(
                        // Check child size (which can come from overscroll) because
                        // layout_extent may be zero. Check layout_extent also since even
                        // with a layout_extent, the indicator builder may decide to not
                        // build anything.
                        f64::max(child_height, layout_extent) - constraints.scroll_offset,
                        0.0,
                    ))
                    .max_paint_extent(f64::max(
                        f64::max(child_height, layout_extent) - constraints.scroll_offset,
                        0.0,
                    ))
                    .layout_extent(f64::max(layout_extent - constraints.scroll_offset, 0.0))
                    .build(),
            );
        } else {
            // If we never started overscrolling, return no geometry.
            self.set_geometry(app, SliverGeometry::ZERO);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        paint_context: &mut PaintingContext,
        offset: Offset,
    ) {
        let constraints = self.constraints(app);
        let child = self
            .child(app)
            .expect("a CupertinoSliverRefresh always has its LayoutBuilder child");
        if constraints.overlap < 0.0 || constraints.scroll_offset + child.size(app).height() > 0.0 {
            paint_context.paint_child(app, child.as_object(), offset);
        }
    }

    // Nothing special done here because this sliver always paints its child
    // exactly between paint_origin and paint_extent.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        _app: &App,
        _child: AnyRenderObject,
        _transform: &mut Matrix4,
    ) {
        let _ = self;
    }
}

impl RenderSliver for RenderCupertinoSliverRefresh {
    waft_rendering::render_sliver_accessors!();
}

/// The current state of the refresh control.
///
/// Passed into the [`RefreshControlIndicatorBuilder`] builder function so users can show
/// different UI in different modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshIndicatorMode {
    /// Initial state, when not being overscrolled into, or after the overscroll is canceled or
    /// after done and the sliver retracted away.
    Inactive,

    /// While being overscrolled but not far enough yet to trigger the refresh.
    Drag,

    /// Dragged far enough that the onRefresh callback will run and the dragged displacement is
    /// not yet at the final refresh resting state.
    Armed,

    /// While the onRefresh task is running.
    Refresh,

    /// While the indicator is animating away after refreshing.
    Done,
}

/// Signature for a builder that can create a different widget to show in the refresh
/// indicator space depending on the current state of the refresh control and the space
/// available.
///
/// The `refresh_trigger_pull_distance` and `refresh_indicator_extent` parameters are the same
/// values passed into the [`CupertinoSliverRefreshControl`].
///
/// The `pulled_extent` parameter is the currently available space either from overscrolling
/// or as held by the sliver during refresh.
pub type RefreshControlIndicatorBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        RefreshIndicatorMode, // refresh_state
        f64,                  // pulled_extent
        f64,                  // refresh_trigger_pull_distance
        f64,                  // refresh_indicator_extent
    ) -> WidgetRef,
>;

/// A callback function that's invoked when the [`CupertinoSliverRefreshControl`] is pulled a
/// `refresh_trigger_pull_distance`. Must return a [`Task`]. Upon completion of the [`Task`],
/// the [`CupertinoSliverRefreshControl`] enters the [`RefreshIndicatorMode::Done`] state and
/// will start to go away.
pub type RefreshCallback = Rc<dyn Fn(&mut App) -> Task<()>>;

/// A sliver widget implementing the iOS-style pull to refresh content control.
///
/// When inserted as the first sliver in a scroll view or behind other slivers that still lets
/// the scrollable overscroll in front of this sliver (such as the `CupertinoSliverNavigationBar`,
/// this widget will:
///
///  * Let the user draw inside the overscrolled area via the passed in
///    [`builder`](Self::builder).
///  * Trigger the provided [`on_refresh`](Self::on_refresh) function when overscrolled far
///    enough to pass [`refresh_trigger_pull_distance`](Self::refresh_trigger_pull_distance).
///  * Continue to hold [`refresh_indicator_extent`](Self::refresh_indicator_extent) amount of
///    space for the [`builder`](Self::builder) to keep drawing inside of as the [`Task`]
///    returned by [`on_refresh`](Self::on_refresh) processes.
///  * Scroll away once the [`on_refresh`](Self::on_refresh) [`Task`] completes.
///
/// The [`builder`](Self::builder) function will be informed of the current
/// [`RefreshIndicatorMode`] when invoking it, except in the [`RefreshIndicatorMode::Inactive`]
/// state when no space is available and nothing needs to be built. The
/// [`builder`](Self::builder) function will otherwise be continuously invoked as the amount of
/// space available changes from overscroll, as the sliver scrolls away after the
/// [`on_refresh`](Self::on_refresh) task is done, etc.
///
/// Only one refresh can be triggered until the previous refresh has completed and the
/// indicator sliver has retracted at least 90% of the way back.
///
/// Can only be used in downward-scrolling vertical lists that overscrolls. In other words,
/// refreshes can't be triggered with `Scrollable`s using `ClampingScrollPhysics` which is the
/// default on Android. To allow overscroll on Android, use an overscrolling physics such as
/// `BouncingScrollPhysics`. This can be done via:
///
///  * Providing a `BouncingScrollPhysics` (possibly in combination with a
///    `AlwaysScrollableScrollPhysics`) while constructing the scrollable.
///  * By inserting a `ScrollConfiguration` with `BouncingScrollPhysics` above the scrollable.
///  * By using `CupertinoApp`, which always uses a `ScrollConfiguration` with
///    `BouncingScrollPhysics` regardless of platform.
///
/// In a typical application, this sliver should be inserted between the app bar sliver such as
/// `CupertinoSliverNavigationBar` and your main scrollable content's sliver.
///
/// See also:
///
///  * `CustomScrollView`, a typical sliver holding scroll view this control should go into.
///  * <https://developer.apple.com/ios/human-interface-guidelines/controls/refresh-content-controls/>
///  * `RefreshIndicator`, a Material Design version of the pull-to-refresh paradigm. This
///    widget works differently than `RefreshIndicator` because instead of being an overlay on
///    top of the scrollable, the [`CupertinoSliverRefreshControl`] is part of the scrollable
///    and actively occupies scrollable space.
pub struct CupertinoSliverRefreshControl {
    pub key: Option<KeyRef>,
    /// The amount of overscroll the scrollable must be dragged to trigger a reload.
    ///
    /// Must be larger than zero and larger than
    /// [`refresh_indicator_extent`](Self::refresh_indicator_extent). Defaults to 100 pixels
    /// when not specified.
    ///
    /// When overscrolled past this distance, [`on_refresh`](Self::on_refresh) will be called if
    /// not null and the [`builder`](Self::builder) will build in the
    /// [`RefreshIndicatorMode::Armed`] state.
    pub refresh_trigger_pull_distance: f64,
    /// The amount of space the refresh indicator sliver will keep holding while
    /// [`on_refresh`](Self::on_refresh)'s [`Task`] is still running.
    ///
    /// Must be a positive number, but can be zero, in which case the sliver will start
    /// retracting back to zero as soon as the refresh is started. Defaults to 60 pixels when
    /// not specified.
    ///
    /// Must be smaller than
    /// [`refresh_trigger_pull_distance`](Self::refresh_trigger_pull_distance), since the sliver
    /// shouldn't grow further after triggering the refresh.
    pub refresh_indicator_extent: f64,
    /// A builder that's called as this sliver's size changes, and as the state changes.
    ///
    /// Can be set to null, in which case nothing will be drawn in the overscrolled space.
    ///
    /// Will not be called when the available space is zero such as before any overscroll.
    pub builder: Option<RefreshControlIndicatorBuilder>,
    /// Callback invoked when pulled by
    /// [`refresh_trigger_pull_distance`](Self::refresh_trigger_pull_distance).
    ///
    /// If provided, must return a [`Task`] which will keep the indicator in the
    /// [`RefreshIndicatorMode::Refresh`] state until the [`Task`] completes.
    ///
    /// Can be null, in which case a single frame of [`RefreshIndicatorMode::Armed`] state will
    /// be drawn before going immediately to the [`RefreshIndicatorMode::Done`] where the sliver
    /// will start retracting.
    pub on_refresh: Option<RefreshCallback>,
}

impl CupertinoSliverRefreshControl {
    const DEFAULT_REFRESH_TRIGGER_PULL_DISTANCE: f64 = 100.0;
    const DEFAULT_REFRESH_INDICATOR_EXTENT: f64 = 60.0;

    /// Create a new refresh control for inserting into a list of slivers.
    ///
    /// The [`refresh_trigger_pull_distance`](Self::refresh_trigger_pull_distance) and
    /// [`refresh_indicator_extent`](Self::refresh_indicator_extent) arguments must be greater
    /// than or equal to 0.
    ///
    /// The [`builder`](Self::builder) argument may be null, in which case no indicator UI will
    /// be shown but the [`on_refresh`](Self::on_refresh) will still be invoked. By default,
    /// [`builder`](Self::builder) shows a [`CupertinoActivityIndicator`].
    ///
    /// The [`on_refresh`](Self::on_refresh) argument will be called when pulled far enough to
    /// trigger a refresh.
    pub fn new() -> CupertinoSliverRefreshControl {
        CupertinoSliverRefreshControl::default()
    }

    /// Dart `CupertinoSliverRefreshControl(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSliverRefreshControl {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSliverRefreshControl(refreshTriggerPullDistance:)`.
    pub fn refresh_trigger_pull_distance(
        mut self,
        refresh_trigger_pull_distance: f64,
    ) -> CupertinoSliverRefreshControl {
        debug_assert!(refresh_trigger_pull_distance > 0.0);
        self.refresh_trigger_pull_distance = refresh_trigger_pull_distance;
        self
    }

    /// Dart `CupertinoSliverRefreshControl(refreshIndicatorExtent:)`.
    pub fn refresh_indicator_extent(
        mut self,
        refresh_indicator_extent: f64,
    ) -> CupertinoSliverRefreshControl {
        debug_assert!(refresh_indicator_extent >= 0.0);
        self.refresh_indicator_extent = refresh_indicator_extent;
        self
    }

    /// Dart `CupertinoSliverRefreshControl(builder:)`.
    pub fn builder(
        mut self,
        builder: Option<RefreshControlIndicatorBuilder>,
    ) -> CupertinoSliverRefreshControl {
        self.builder = builder;
        self
    }

    /// Dart `CupertinoSliverRefreshControl(onRefresh:)`.
    pub fn on_refresh(mut self, on_refresh: RefreshCallback) -> CupertinoSliverRefreshControl {
        self.on_refresh = Some(on_refresh);
        self
    }

    /// Retrieve the current state of the CupertinoSliverRefreshControl. The same as the state
    /// that gets passed into the [`builder`](Self::builder) function. Used for testing.
    pub fn state(app: &App, context: BuildContext) -> RefreshIndicatorMode {
        let state = context
            .find_ancestor_state_of_type::<CupertinoSliverRefreshControlState>(app)
            .expect("the context is below a CupertinoSliverRefreshControl");
        app.get(state).refresh_state
    }

    /// Builds a refresh indicator that reflects the standard iOS pull-to-refresh behavior.
    /// Specifically, this entails presenting an activity indicator that changes depending on
    /// the current refresh_state. As the user initially drags down, the indicator will
    /// gradually reveal individual ticks until the refresh becomes armed. At this point, the
    /// animated activity indicator will begin rotating. Once the refresh has completed, the
    /// activity indicator shrinks away as the space allocation animates back to closed.
    pub fn build_refresh_indicator(
        _app: &mut App,
        _context: BuildContext,
        refresh_state: RefreshIndicatorMode,
        pulled_extent: f64,
        refresh_trigger_pull_distance: f64,
        _refresh_indicator_extent: f64,
    ) -> WidgetRef {
        let percentage_complete =
            clamp_double(pulled_extent / refresh_trigger_pull_distance, 0.0, 1.0);

        // Place the indicator at the top of the sliver that opens up. We're using a
        // Stack/Positioned widget because the CupertinoActivityIndicator does some
        // internal translations based on the current size (which grows as the user drags)
        // that makes Padding calculations difficult. Rather than be reliant on the
        // internal implementation of the activity indicator, the Positioned widget allows
        // us to be explicit where the widget gets placed. The indicator should appear
        // over the top of the dragged widget, hence the use of Clip::None.
        Center::new()
            .child(
                Stack::new()
                    .clip_behavior(Clip::None)
                    .children([Positioned::new(Self::build_indicator_for_refresh_state(
                        refresh_state,
                        K_ACTIVITY_INDICATOR_RADIUS,
                        percentage_complete,
                    ))
                    .top(K_ACTIVITY_INDICATOR_MARGIN)
                    .left(0.0)
                    .right(0.0)
                    .into_widget()]),
            )
            .into_widget()
    }

    fn build_indicator_for_refresh_state(
        refresh_state: RefreshIndicatorMode,
        radius: f64,
        percentage_complete: f64,
    ) -> WidgetRef {
        match refresh_state {
            RefreshIndicatorMode::Drag => {
                // While we're dragging, we draw individual ticks of the spinner while
                // simultaneously easing the opacity in. The opacity curve values here were
                // derived using Xcode through inspecting a native app running on iOS 13.5.
                let opacity_curve = Interval::new(0.0, 0.35, Curves::ease_in_out());
                Opacity::new(opacity_curve.transform(percentage_complete))
                    .child(
                        CupertinoActivityIndicator::partially_revealed()
                            .radius(radius)
                            .progress(percentage_complete),
                    )
                    .into_widget()
            }
            RefreshIndicatorMode::Armed | RefreshIndicatorMode::Refresh => {
                // Once we're armed or performing the refresh, we just show the normal spinner.
                CupertinoActivityIndicator::new()
                    .radius(radius)
                    .into_widget()
            }
            RefreshIndicatorMode::Done => {
                // When the user lets go, the standard transition is to shrink the spinner.
                CupertinoActivityIndicator::new()
                    .radius(radius * percentage_complete)
                    .into_widget()
            }
            RefreshIndicatorMode::Inactive => {
                // Anything else doesn't show anything.
                SizedBox::shrink().into_widget()
            }
        }
    }
}

impl Default for CupertinoSliverRefreshControl {
    fn default() -> CupertinoSliverRefreshControl {
        CupertinoSliverRefreshControl {
            key: None,
            refresh_trigger_pull_distance: Self::DEFAULT_REFRESH_TRIGGER_PULL_DISTANCE,
            refresh_indicator_extent: Self::DEFAULT_REFRESH_INDICATOR_EXTENT,
            builder: Some(Rc::new(Self::build_refresh_indicator)),
            on_refresh: None,
        }
    }
}

impl Debug for CupertinoSliverRefreshControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoSliverRefreshControl")
            .field(
                "refresh_trigger_pull_distance",
                &self.refresh_trigger_pull_distance,
            )
            .field("refresh_indicator_extent", &self.refresh_indicator_extent)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoSliverRefreshControl {
    type State = CupertinoSliverRefreshControlState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.refresh_trigger_pull_distance >= self.refresh_indicator_extent,
            "The refresh indicator cannot take more space in its final state than the amount \
             initially created by overscrolling."
        );
    }

    fn create_state(&self) -> CupertinoSliverRefreshControlState {
        CupertinoSliverRefreshControlState {
            state: StateData::new(),
            refresh_state: RefreshIndicatorMode::Inactive,
            refresh_task: None,
            latest_indicator_box_extent: 0.0,
            has_sliver_layout_extent: false,
        }
    }
}

/// Dart's `_CupertinoSliverRefreshControlState`.
pub struct CupertinoSliverRefreshControlState {
    state: StateData<CupertinoSliverRefreshControl>,
    refresh_state: RefreshIndicatorMode,
    /// What awaits the [`Task`] returned by the widget's `on_refresh`, and runs Dart's
    /// `whenComplete` body once it has completed; `Some` exactly while a refresh is running.
    refresh_task: Option<Task<()>>,
    // The amount of space available from the inner indicator box's perspective.
    //
    // The value is the sum of the sliver's layout extent and the overscroll
    // (which partially gets transferred into the layout extent when the refresh
    // triggers).
    //
    // The value of latest_indicator_box_extent doesn't change when the sliver scrolls
    // away without retracting; it is independent from the sliver's scroll_offset.
    latest_indicator_box_extent: f64,
    has_sliver_layout_extent: bool,
}

impl CupertinoSliverRefreshControlState {
    // Reset the state from done to inactive when only this fraction of the
    // original `refresh_trigger_pull_distance` is left.
    const INACTIVE_RESET_OVERSCROLL_FRACTION: f64 = 0.1;

    // A state machine transition calculator. Multiple states can be transitioned
    // through per single call.
    // Dart's switch assigns `nextState` on paths that fall through to a case which returns or
    // reassigns it; the stores stay, so the arms read as Dart's.
    #[allow(unused_assignments)]
    fn transition_next_state(self: Handle<Self>, app: &mut App) -> RefreshIndicatorMode {
        let mut next_state: RefreshIndicatorMode;

        // Dart's `goToDone` assigns `nextState`; this one returns what it assigns, as Rust
        // lends no `&mut` of a binding that may be unassigned.
        fn go_to_done(
            this: Handle<CupertinoSliverRefreshControlState>,
            app: &mut App,
        ) -> RefreshIndicatorMode {
            let next_state = RefreshIndicatorMode::Done;
            // Either schedule the RenderSliver to re-layout on the next frame
            // when not currently in a frame or schedule it on the next frame.
            if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::Idle {
                this.set_state(app, |state| state.has_sliver_layout_extent = false);
            } else {
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _timestamp| {
                        this.set_state(app, |state| state.has_sliver_layout_extent = false);
                    }),
                    None,
                );
            }
            next_state
        }

        let latest_indicator_box_extent = app.get(self).latest_indicator_box_extent;
        let widget = self.widget(app);
        let refresh_trigger_pull_distance = widget.refresh_trigger_pull_distance;
        let refresh_indicator_extent = widget.refresh_indicator_extent;

        // Dart's `switch` with `continue` to a labelled case: each arm names the case it
        // falls through to.
        let mut case = app.get(self).refresh_state;
        loop {
            match case {
                RefreshIndicatorMode::Inactive => {
                    if latest_indicator_box_extent <= 0.0 {
                        return RefreshIndicatorMode::Inactive;
                    } else {
                        next_state = RefreshIndicatorMode::Drag;
                    }
                    case = RefreshIndicatorMode::Drag;
                }
                RefreshIndicatorMode::Drag => {
                    if latest_indicator_box_extent == 0.0 {
                        return RefreshIndicatorMode::Inactive;
                    } else if latest_indicator_box_extent < refresh_trigger_pull_distance {
                        return RefreshIndicatorMode::Drag;
                    } else {
                        if self.widget(app).on_refresh.is_some() {
                            HapticFeedback::medium_impact(app);
                            // Call on_refresh after this frame finished since the function is
                            // user supplied and we're always here in the middle of the sliver's
                            // perform_layout.
                            SchedulerBinding::add_post_frame_callback(
                                app,
                                FrameCallback::new(move |app, _timestamp| {
                                    self.start_refresh_task(app);
                                    self.set_state(app, |state| {
                                        state.has_sliver_layout_extent = true;
                                    });
                                }),
                                None,
                            );
                        }
                        return RefreshIndicatorMode::Armed;
                    }
                }
                RefreshIndicatorMode::Armed => {
                    if app.get(self).refresh_state == RefreshIndicatorMode::Armed
                        && app.get(self).refresh_task.is_none()
                    {
                        next_state = go_to_done(self, app);
                        case = RefreshIndicatorMode::Done;
                        continue;
                    }

                    if latest_indicator_box_extent > refresh_indicator_extent {
                        return RefreshIndicatorMode::Armed;
                    } else {
                        next_state = RefreshIndicatorMode::Refresh;
                    }
                    case = RefreshIndicatorMode::Refresh;
                }
                RefreshIndicatorMode::Refresh => {
                    if app.get(self).refresh_task.is_some() {
                        return RefreshIndicatorMode::Refresh;
                    } else {
                        next_state = go_to_done(self, app);
                    }
                    case = RefreshIndicatorMode::Done;
                }
                RefreshIndicatorMode::Done => {
                    // Let the transition back to inactive trigger before strictly going
                    // to 0.0 since the last bit of the animation can take some time and
                    // can feel sluggish if not going all the way back to 0.0 prevented
                    // a subsequent pull-to-refresh from starting.
                    if latest_indicator_box_extent
                        > refresh_trigger_pull_distance * Self::INACTIVE_RESET_OVERSCROLL_FRACTION
                    {
                        return RefreshIndicatorMode::Done;
                    } else {
                        next_state = RefreshIndicatorMode::Inactive;
                    }
                    break;
                }
            }
        }

        next_state
    }

    /// Dart's `refreshTask = widget.onRefresh!()..whenComplete(..)`.
    fn start_refresh_task(self: Handle<Self>, app: &mut App) {
        let on_refresh = self
            .widget(app)
            .on_refresh
            .clone()
            .expect("scheduled only while the widget has an on_refresh");
        let refresh = on_refresh(app);
        let refresh_task = app.spawn(async move |cx| {
            refresh.await;
            cx.update(|app| {
                if self.mounted(app) {
                    self.set_state(app, |state| state.refresh_task = None);
                    // Trigger one more transition because by this time, BoxConstraint's
                    // max_height might already be resting at 0 in which case no
                    // calls to transition_next_state will occur anymore and the
                    // state may be stuck in a non-inactive state.
                    let refresh_state = self.transition_next_state(app);
                    app.get_mut(self).refresh_state = refresh_state;
                }
            });
        });
        app.get_mut(self).refresh_task = Some(refresh_task);
    }
}

impl State for CupertinoSliverRefreshControlState {
    type Widget = CupertinoSliverRefreshControl;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).refresh_state = RefreshIndicatorMode::Inactive;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        CupertinoSliverRefresh::new()
            .refresh_indicator_layout_extent(widget.refresh_indicator_extent)
            .has_layout_extent(app.get(self).has_sliver_layout_extent)
            // A LayoutBuilder lets the sliver's layout changes be fed back out to
            // its owner to trigger state changes.
            .child(
                LayoutBuilder::new(move |app, context, constraints| {
                    app.get_mut(self).latest_indicator_box_extent = constraints.max_height;
                    let refresh_state = self.transition_next_state(app);
                    app.get_mut(self).refresh_state = refresh_state;
                    let latest_indicator_box_extent = app.get(self).latest_indicator_box_extent;
                    let widget = self.widget(app);
                    if let Some(builder) = widget.builder.clone()
                        && latest_indicator_box_extent > 0.0
                    {
                        let (refresh_trigger_pull_distance, refresh_indicator_extent) = (
                            widget.refresh_trigger_pull_distance,
                            widget.refresh_indicator_extent,
                        );
                        return builder(
                            app,
                            context,
                            refresh_state,
                            latest_indicator_box_extent,
                            refresh_trigger_pull_distance,
                            refresh_indicator_extent,
                        );
                    }
                    LimitedBox::new()
                        .max_width(0.0)
                        .max_height(0.0)
                        .child(SizedBox::expand())
                        .into_widget()
                })
                .into_widget(),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use waft_embedder::{
        HapticFeedbackType, Haptics, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind, Rect, TargetPlatform, TextDirection,
    };
    use waft_foundation::{AppCell, Completer};
    use waft_gestures::GestureBinding;
    use waft_test::{TestPlatform, TestView};
    use waft_widgets::{
        AnyElement, BouncingScrollPhysics, Builder, Container, CustomScrollView, Directionality,
        GlobalKey, MediaQuery, MediaQueryData, SliverList, Text, WidgetsBinding, downcast_widget,
    };

    use super::*;
    use crate::app::CupertinoApp;
    use crate::nav_bar::CupertinoSliverNavigationBar;
    use crate::test_support::pump;

    /// Dart's `precisionErrorTolerance`, which `moreOrLessEquals` defaults to.
    const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

    /// The tolerance of a position read through `local_to_global`: `Matrix4` is valo's f32
    /// matrix (waft-gestures' PORTING.md), so a transformed position is exact only to f32.
    const POSITION_TOLERANCE: f64 = 1e-4;

    /// Dart's `kDragSlopDefault`, the slop `tester.drag` moves through first.
    const K_DRAG_SLOP_DEFAULT: f64 = 20.0;

    /// The platforms Dart's `TargetPlatformVariant` names for most cases.
    const APPLE: [TargetPlatform; 2] = [TargetPlatform::IOS, TargetPlatform::MacOS];

    /// The length of Dart's `buildAListOfStuff`. Dart's file means its first groups to run on
    /// ten items and its last on two, but every case reads `testListLength` when it runs,
    /// after `main` has set it to two: each group runs on two items, so the cases run once
    /// here, on two.
    const TEST_LIST_LENGTH: i32 = 2;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Invocation {
        Builder {
            refresh_state: RefreshIndicatorMode,
            pulled_extent: f64,
            refresh_trigger_pull_distance: f64,
            refresh_indicator_extent: f64,
        },
        RefreshTask,
    }

    /// What Dart's matchers say of `pulledExtent`.
    #[derive(Debug, Clone, Copy)]
    enum Extent {
        Exactly(f64),
        /// `moreOrLessEquals`.
        About(f64),
        /// `moreOrLessEquals` of a position read through the f32 `Matrix4`.
        AboutPosition(f64),
        /// `inExclusiveRange`.
        Between(f64, f64),
    }

    impl Extent {
        fn matches(self, value: f64) -> bool {
            match self {
                Extent::Exactly(expected) => value == expected,
                Extent::About(expected) => (value - expected).abs() <= PRECISION_ERROR_TOLERANCE,
                Extent::AboutPosition(expected) => (value - expected).abs() <= POSITION_TOLERANCE,
                Extent::Between(low, high) => value > low && value < high,
            }
        }
    }

    /// Dart's `matchesBuilder` with the default distances, 100 and 60.
    #[derive(Debug, Clone, Copy)]
    struct MatchesBuilder {
        refresh_state: RefreshIndicatorMode,
        pulled_extent: Extent,
    }

    fn matches_builder(
        refresh_state: RefreshIndicatorMode,
        pulled_extent: Extent,
    ) -> MatchesBuilder {
        MatchesBuilder {
            refresh_state,
            pulled_extent,
        }
    }

    impl MatchesBuilder {
        fn matches(self, invocation: &Invocation) -> bool {
            match *invocation {
                Invocation::Builder {
                    refresh_state,
                    pulled_extent,
                    refresh_trigger_pull_distance,
                    refresh_indicator_extent,
                } => {
                    refresh_state == self.refresh_state
                        && self.pulled_extent.matches(pulled_extent)
                        && refresh_trigger_pull_distance == 100.0
                        && refresh_indicator_extent == 60.0
                }
                Invocation::RefreshTask => false,
            }
        }
    }

    /// Dart's `FakeBuilder`.
    struct FakeBuilder {
        refresh_completer: Completer<()>,
        invocations: RefCell<Vec<Invocation>>,
        refresh_indicator: RefCell<WidgetRef>,
    }

    impl FakeBuilder {
        fn new() -> Rc<FakeBuilder> {
            Rc::new(FakeBuilder {
                refresh_completer: Completer::new(),
                invocations: RefCell::new(Vec::new()),
                refresh_indicator: RefCell::new(Container::new().into_widget()),
            })
        }

        fn builder(self: &Rc<Self>) -> Option<RefreshControlIndicatorBuilder> {
            let this = Rc::clone(self);
            Some(Rc::new(
                move |_app: &mut App,
                      _context: BuildContext,
                      refresh_state: RefreshIndicatorMode,
                      pulled_extent: f64,
                      refresh_trigger_pull_distance: f64,
                      refresh_indicator_extent: f64| {
                    assert!(
                        pulled_extent >= 0.0,
                        "The pulledExtent should never be less than 0.0"
                    );
                    assert!(
                        refresh_trigger_pull_distance >= 0.0,
                        "The refreshTriggerPullDistance should never be less than 0.0"
                    );
                    assert!(
                        refresh_indicator_extent >= 0.0,
                        "The refreshIndicatorExtent should never be less than 0.0"
                    );
                    this.invocations.borrow_mut().push(Invocation::Builder {
                        refresh_state,
                        pulled_extent,
                        refresh_trigger_pull_distance,
                        refresh_indicator_extent,
                    });
                    Rc::clone(&this.refresh_indicator.borrow())
                },
            ))
        }

        fn refresh_task(self: &Rc<Self>) -> RefreshCallback {
            let this = Rc::clone(self);
            Rc::new(move |app: &mut App| {
                this.invocations.borrow_mut().push(Invocation::RefreshTask);
                let future = this.refresh_completer.future();
                app.spawn(async move |_cx| future.await)
            })
        }
    }

    #[derive(Default)]
    struct RecordingHaptics(RefCell<Vec<HapticFeedbackType>>);

    impl Haptics for RecordingHaptics {
        fn feedback(&self, kind: HapticFeedbackType) {
            self.0.borrow_mut().push(kind);
        }
    }

    /// One case of the Dart file: an app on one platform with an 800x600 logical view, as
    /// Flutter's test view is, and the tester's clock, pointer and finders.
    struct Tester {
        cell: Rc<AppCell>,
        platform: TargetPlatform,
        now: Duration,
        items: Rc<Vec<GlobalKey>>,
        indicator: GlobalKey,
        haptics: Rc<RecordingHaptics>,
        mock: Rc<FakeBuilder>,
        pointer: i64,
        last_position: Offset,
    }

    impl Tester {
        fn new(platform: TargetPlatform) -> Tester {
            let haptics = Rc::new(RecordingHaptics::default());
            let test_platform = TestPlatform::new()
                .on(platform)
                .with_view(Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 1.0)))
                .with_haptics(Rc::clone(&haptics) as Rc<dyn Haptics>);
            let cell = AppCell::with_platform(Rc::new(test_platform));
            {
                let mut app = cell.borrow_mut();
                let binding = waft_painting::PaintingBinding::instance(&mut app);
                binding.install_fonts(&mut app, |fonts| {
                    fonts.add_source(valo_system_fonts::SystemFonts::load());
                });
            }
            Tester {
                cell,
                platform,
                now: Duration::ZERO,
                items: Rc::new((0..TEST_LIST_LENGTH).map(|_| GlobalKey::new()).collect()),
                indicator: GlobalKey::new(),
                haptics,
                mock: FakeBuilder::new(),
                pointer: 0,
                last_position: Offset::ZERO,
            }
        }

        fn on_mac_os(&self) -> bool {
            self.platform == TargetPlatform::MacOS
        }

        /// Dart's `mockHelper.refreshIndicator = const Center(child: Text('-1'))`.
        fn indicate_with_text(&self) {
            *self.mock.refresh_indicator.borrow_mut() = Center::new()
                .key(self.indicator.clone().into())
                .child(Text::new("-1"))
                .into_widget();
        }

        /// Dart's `buildAListOfStuff`, its items keyed so the tester can find them.
        fn list_of_stuff(&self) -> WidgetRef {
            let items = Rc::clone(&self.items);
            SliverList::builder(move |_app, _context, index| {
                Some(
                    SizedBox::new()
                        .key(items[index as usize].clone().into())
                        .height(200.0)
                        .child(Center::new().child(Text::new(index.to_string())))
                        .into_widget(),
                )
            })
            .item_count(TEST_LIST_LENGTH)
            .into_widget()
        }

        /// `tester.pumpWidget`.
        fn pump_widget(&mut self, root: WidgetRef) {
            {
                let mut app = self.cell.borrow_mut();
                let view = app.platform().views()[0].clone();
                waft_widgets::run_widget(
                    &mut app,
                    waft_widgets::View::new(view, root).into_widget(),
                );
            }
            self.cell.elapse(Duration::ZERO);
            self.pump(Duration::ZERO);
        }

        /// A `CupertinoApp` whose home scrolls `slivers`.
        fn pump_app(&mut self, slivers: Vec<WidgetRef>) {
            self.pump_widget(
                CupertinoApp::new()
                    .home(CustomScrollView::new().slivers(slivers))
                    .into_widget(),
            );
        }

        /// The usual tree: the refresh control above the list of stuff.
        fn pump_control(&mut self, control: CupertinoSliverRefreshControl) {
            let list = self.list_of_stuff();
            self.pump_app(vec![control.into_widget(), list]);
        }

        fn builder(&self) -> Option<RefreshControlIndicatorBuilder> {
            self.mock.builder()
        }

        fn refresh_task(&self) -> RefreshCallback {
            self.mock.refresh_task()
        }

        /// `tester.pump(duration)`.
        fn pump(&mut self, duration: Duration) {
            self.now += duration;
            self.cell.elapse(duration);
            pump(&mut self.cell.borrow_mut(), self.now);
            self.cell.checkpoint();
        }

        fn complete_refresh(&self) {
            self.mock
                .refresh_completer
                .complete(&mut self.cell.borrow_mut(), ());
            self.cell.checkpoint();
        }

        fn send(&mut self, change: PointerChange, position: Offset, time_stamp: Duration) {
            let delta = match change {
                PointerChange::Down => Offset::ZERO,
                _ => position - self.last_position,
            };
            self.last_position = position;
            {
                let mut app = self.cell.borrow_mut();
                GestureBinding::instance(&mut app).handle_pointer_data_packet(
                    &mut app,
                    PointerDataPacket::new(vec![PointerData {
                        change,
                        kind: PointerDeviceKind::Touch,
                        time_stamp,
                        pointer_identifier: self.pointer,
                        buttons: if matches!(change, PointerChange::Down | PointerChange::Move) {
                            1
                        } else {
                            0
                        },
                        physical_x: position.dx(),
                        physical_y: position.dy(),
                        physical_delta_x: delta.dx(),
                        physical_delta_y: delta.dy(),
                        ..PointerData::default()
                    }]),
                );
                app.drain_microtasks();
            }
            self.cell.checkpoint();
        }

        /// `tester.startGesture`: every event of a `TestGesture` is at time zero.
        fn start_gesture(&mut self, position: Offset) {
            self.pointer += 1;
            self.send(PointerChange::Down, position, Duration::ZERO);
        }

        fn move_by(&mut self, offset: Offset) {
            let position = self.last_position + offset;
            self.send(PointerChange::Move, position, Duration::ZERO);
        }

        fn up(&mut self) {
            let position = self.last_position;
            self.send(PointerChange::Up, position, Duration::ZERO);
        }

        /// `tester.dragFrom` for a vertical `offset`.
        fn drag_from(&mut self, start: Offset, offset: Offset, touch_slop_y: f64) {
            assert_eq!(offset.dx(), 0.0, "the refresh cases drag vertically");
            self.start_gesture(start);
            if offset.dy().abs() > touch_slop_y && touch_slop_y > 0.0 {
                let signed_slop_y = touch_slop_y * offset.dy().signum();
                self.move_by(Offset::new(0.0, signed_slop_y));
                self.move_by(Offset::new(0.0, offset.dy() - signed_slop_y));
            } else {
                self.move_by(offset);
            }
            self.up();
        }

        /// `tester.drag(find.text('$index'), ..)`.
        fn drag_item(&mut self, index: usize, offset: Offset, touch_slop_y: f64) {
            let start = self.item_rect(index).center();
            self.drag_from(start, offset, touch_slop_y);
        }

        /// `tester.flingFrom`.
        fn fling_from(&mut self, start: Offset, offset: Offset, speed: f64) {
            const K_MOVE_COUNT: i32 = 50;
            let frame_interval = Duration::from_millis(16).as_micros() as f64;
            let time_stamp_delta = 1_000_000.0 * offset.distance() / (K_MOVE_COUNT as f64 * speed);
            let mut time_stamp = 0.0_f64;
            let mut last_time_stamp = time_stamp;
            self.pointer += 1;
            self.send(PointerChange::Down, start, Duration::ZERO);
            for i in 0..=K_MOVE_COUNT {
                let location = start + offset * (i as f64 / K_MOVE_COUNT as f64);
                self.send(
                    PointerChange::Move,
                    location,
                    Duration::from_micros(time_stamp.round() as u64),
                );
                time_stamp += time_stamp_delta;
                if time_stamp - last_time_stamp > frame_interval {
                    self.pump(Duration::from_micros(
                        (time_stamp - last_time_stamp).trunc() as u64,
                    ));
                    last_time_stamp = time_stamp;
                }
            }
            let position = self.last_position;
            self.send(
                PointerChange::Up,
                position,
                Duration::from_micros(time_stamp.round() as u64),
            );
        }

        fn rect_of(&self, key: &GlobalKey) -> Option<Rect> {
            let mut app = self.cell.borrow_mut();
            let object = key
                .current_context(&mut app)?
                .find_render_object(&app)?
                .as_box()?;
            Some(object.local_to_global(&app, Offset::ZERO, None) & object.size(&app))
        }

        /// `tester.getRect(find.widgetWithText(SizedBox, '$index'))`: the `Center` inside is
        /// the same rect.
        fn item_rect(&self, index: usize) -> Rect {
            self.rect_of(&self.items[index])
                .unwrap_or_else(|| panic!("item {index} is built"))
        }

        fn item_top(&self, index: usize) -> f64 {
            self.item_rect(index).top
        }

        /// `tester.getRect(find.widgetWithText(Center, '-1'))`.
        fn indicator_rect(&self) -> Rect {
            self.rect_of(&self.indicator)
                .expect("the indicator is built")
        }

        /// `find.text('-1')` finds a widget.
        fn indicator_shown(&self) -> bool {
            self.indicator
                .current_context(&mut self.cell.borrow_mut())
                .is_some()
        }

        fn elements(&self) -> Vec<AnyElement> {
            let mut app = self.cell.borrow_mut();
            let root = WidgetsBinding::instance(&mut app)
                .root_element(&app)
                .expect("a mounted app");
            let mut all = vec![root];
            let mut visited = 0;
            while visited < all.len() {
                let element = all[visited];
                element.visit_children(&app, &mut |child| all.push(child));
                visited += 1;
            }
            all
        }

        /// `CupertinoSliverRefreshControl.state(tester.element(find.byType(LayoutBuilder)))`.
        fn state(&self) -> RefreshIndicatorMode {
            let layout_builder = self
                .elements()
                .into_iter()
                .find(|element| {
                    downcast_widget::<LayoutBuilder>(&**element.widget(&self.cell.borrow()))
                        .is_some()
                })
                .expect("the refresh control's LayoutBuilder is mounted");
            CupertinoSliverRefreshControl::state(&self.cell.borrow(), layout_builder)
        }

        /// `tester.widget<CupertinoActivityIndicator>(find.byType(..))`'s progress and radius.
        fn activity_indicator(&self) -> (f64, f64) {
            let elements = self.elements();
            let app = self.cell.borrow();
            elements
                .into_iter()
                .find_map(|element| {
                    downcast_widget::<CupertinoActivityIndicator>(&**element.widget(&app))
                        .map(|indicator| (indicator.progress, indicator.radius))
                })
                .expect("a CupertinoActivityIndicator is built")
        }

        fn invocations(&self) -> Vec<Invocation> {
            self.mock.invocations.borrow().clone()
        }

        fn contains(&self, expected: MatchesBuilder) -> bool {
            self.invocations()
                .iter()
                .any(|invocation| expected.matches(invocation))
        }

        fn contains_refresh_task(&self) -> bool {
            self.invocations().contains(&Invocation::RefreshTask)
        }

        /// `containsAllInOrder`, where `None` stands for `RefreshTaskInvocation()`.
        fn contains_all_in_order(&self, expected: &[Option<MatchesBuilder>]) -> bool {
            let invocations = self.invocations();
            let mut remaining = invocations.iter();
            expected.iter().all(|expected| {
                remaining.any(|invocation| match expected {
                    Some(builder) => builder.matches(invocation),
                    None => *invocation == Invocation::RefreshTask,
                })
            })
        }

        fn pulled_extent(&self, index: usize) -> f64 {
            match self.invocations()[index] {
                Invocation::Builder { pulled_extent, .. } => pulled_extent,
                Invocation::RefreshTask => panic!("invocation {index} is the refresh task"),
            }
        }
    }

    use RefreshIndicatorMode::{Armed, Done, Drag, Inactive, Refresh};

    fn about(value: f64) -> Extent {
        Extent::About(value)
    }

    fn about_position(value: f64) -> Extent {
        Extent::AboutPosition(value)
    }

    fn exactly(value: f64) -> Extent {
        Extent::Exactly(value)
    }

    fn between(low: f64, high: f64) -> Extent {
        Extent::Between(low, high)
    }

    /// Runs `case` on each of `platforms`, as Dart's `TargetPlatformVariant` does.
    fn for_each_variant(platforms: &[TargetPlatform], case: impl Fn(&mut Tester)) {
        for &platform in platforms {
            let mut tester = Tester::new(platform);
            case(&mut tester);
        }
    }

    /// Dart's `moreOrLessEquals` of a position.
    fn assert_about(actual: f64, expected: f64, what: &str) {
        assert!(
            (actual - expected).abs() <= POSITION_TOLERANCE,
            "{what}: {actual} is not about {expected}"
        );
    }

    // UI tests.

    #[test]
    fn does_not_invoke_anything_without_user_interaction() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            assert!(t.invocations().is_empty());
            assert_eq!(t.item_rect(0).top_left(), Offset::ZERO);
        });
    }

    #[test]
    fn calls_the_indicator_builder_when_starting_to_overscroll() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            // Drag down but not enough to trigger the refresh.
            t.drag_item(0, Offset::new(0.0, 50.0), 0.0);
            t.pump(Duration::ZERO);

            // The function is referenced once while passing into CupertinoSliverRefreshControl
            // and is called.
            assert!(matches_builder(Drag, exactly(50.0)).matches(&t.invocations()[0]));
            assert_eq!(t.invocations().len(), 1);

            assert_eq!(t.item_rect(0).top_left(), Offset::new(0.0, 50.0));
        });
    }

    #[test]
    fn does_not_call_the_builder_if_overscroll_does_not_move_slivers_like_on_android() {
        let mut t = Tester::new(TargetPlatform::Android);
        let control = CupertinoSliverRefreshControl::new().builder(t.builder());
        let list = t.list_of_stuff();
        t.pump_widget(
            Directionality::new(
                TextDirection::Ltr,
                MediaQuery::new(
                    MediaQueryData::new(),
                    CustomScrollView::new().slivers([control.into_widget(), list]),
                )
                .into_widget(),
            )
            .into_widget(),
        );

        // Drag down but not enough to trigger the refresh.
        t.drag_item(0, Offset::new(0.0, 50.0), K_DRAG_SLOP_DEFAULT);
        t.pump(Duration::ZERO);

        assert!(t.invocations().is_empty());
        assert_eq!(t.item_rect(0).top_left(), Offset::ZERO);
    }

    #[test]
    fn lets_the_builder_update_as_a_canceled_drag_scrolls_away() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            // Drag down but not enough to trigger the refresh.
            t.drag_item(0, Offset::new(0.0, 50.0), 0.0);
            t.pump(Duration::ZERO);
            t.pump(Duration::from_millis(20));
            t.pump(Duration::from_millis(20));
            t.pump(Duration::from_secs(3));

            assert!(t.contains_all_in_order(&[
                Some(matches_builder(Drag, exactly(50.0))),
                Some(matches_builder(Drag, between(0.0, 50.0))),
                Some(matches_builder(Drag, between(0.0, t.pulled_extent(1)))),
            ]));
            // The builder isn't called again when the sliver completely goes away.
            assert_eq!(t.invocations().len(), 3);

            assert_eq!(t.item_rect(0).top_left(), Offset::ZERO);
        });
    }

    #[test]
    fn a_drag_past_the_threshold_triggers_the_refresh_task() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.start_gesture(Offset::ZERO);
            t.move_by(Offset::new(0.0, 99.0));
            t.pump(Duration::ZERO);
            if t.on_mac_os() {
                t.move_by(Offset::new(0.0, -3.0));
            } else {
                t.move_by(Offset::new(0.0, -30.0));
            }
            t.pump(Duration::ZERO);
            if t.on_mac_os() {
                t.move_by(Offset::new(0.0, 90.0));
            } else {
                t.move_by(Offset::new(0.0, 50.0));
            }
            t.pump(Duration::ZERO);

            let (dragged_back, armed) = if t.on_mac_os() {
                (96.0, 112.51104)
            } else {
                (86.78169, 105.80452021305739)
            };
            assert!(
                t.contains_all_in_order(&[
                    Some(matches_builder(Drag, exactly(99.0))),
                    Some(matches_builder(Drag, about(dragged_back))),
                    Some(matches_builder(Armed, about(armed))),
                ]),
                "on {:?}: {:?}",
                t.platform,
                t.invocations()
            );
            // The refresh callback is triggered after the frame.
            assert_eq!(t.invocations().last(), Some(&Invocation::RefreshTask));
            assert_eq!(t.invocations().len(), 4);

            assert_eq!(
                t.haptics.0.borrow().last(),
                Some(&HapticFeedbackType::MediumImpact)
            );
        });
    }

    #[test]
    fn a_refreshing_task_keeps_the_sliver_expanded_forever_until_done() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);
            // Let it start snapping back.
            t.pump(Duration::from_millis(50));

            assert!(t.contains_all_in_order(&[
                Some(matches_builder(Armed, exactly(150.0))),
                None,
                Some(matches_builder(Armed, between(60.0, 150.0))),
            ]));

            // Reaches refresh state and sliver's at 60.0 in height after a while.
            t.pump(Duration::from_secs(1));

            assert!(t.contains(matches_builder(Refresh, exactly(60.0))));

            // Stays in that state forever until future completes.
            t.pump(Duration::from_secs(1000));
            assert_eq!(t.item_rect(0).top_left(), Offset::new(0.0, 60.0));

            t.complete_refresh();
            t.pump(Duration::ZERO);

            assert!(t.contains(matches_builder(Done, exactly(60.0))));
            assert_eq!(t.invocations().len(), 5);
        });
    }

    #[test]
    fn an_expanded_refreshing_sliver_scrolls_normally() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);

            assert!(t.contains(matches_builder(Armed, exactly(150.0))));

            // Given a box constraint of 150, the Center will occupy all that height.
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 150.0));

            // Hits the list.
            t.drag_item(0, Offset::new(0.0, -300.0), 0.0);
            t.pump(Duration::ZERO);

            // Refresh indicator still being told to layout the same way.
            assert!(t.contains(matches_builder(Refresh, exactly(60.0))));

            // Now the sliver is scrolled off screen.
            let (indicator_top, indicator_bottom) = if t.on_mac_os() {
                (-210.0, -150.0)
            } else {
                (-175.38461538461536, -115.38461538461536)
            };
            assert_about(t.indicator_rect().top, indicator_top, "the indicator's top");
            assert_about(
                t.indicator_rect().bottom,
                indicator_bottom,
                "the indicator's bottom",
            );
            assert_about(t.item_top(0), indicator_bottom, "the first item's top");

            // Scroll the top of the refresh indicator back to overscroll, it will
            // snap to the size of the refresh indicator and stay there.
            t.drag_item(1, Offset::new(0.0, 200.0), K_DRAG_SLOP_DEFAULT);
            t.pump(Duration::ZERO);
            t.pump(Duration::from_secs(2));
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 60.0));
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 60.0, 800.0, 260.0));
        });
    }

    #[test]
    fn an_expanded_refreshing_sliver_goes_away_when_done() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);
            assert!(t.contains(matches_builder(Armed, exactly(150.0))));
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 150.0));
            assert!(t.contains_refresh_task());

            // Rebuilds the sliver with a layout extent now.
            t.pump(Duration::ZERO);
            // Let it snap back to occupy the indicator's final sliver space only.
            t.pump(Duration::from_secs(2));

            assert!(t.contains(matches_builder(Refresh, exactly(60.0))));
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 60.0));
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 60.0, 800.0, 260.0));

            t.complete_refresh();
            t.pump(Duration::ZERO);
            assert!(t.contains(matches_builder(Done, exactly(60.0))));

            t.pump(Duration::from_secs(5));
            assert!(!t.indicator_shown());
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));
        });
    }

    #[test]
    fn the_builder_is_still_called_when_the_sliver_snapped_back_more_than_90_percent() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);
            assert!(t.contains(matches_builder(Armed, exactly(150.0))));
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 150.0));
            assert!(t.contains_refresh_task());

            // Rebuilds the sliver with a layout extent now.
            t.pump(Duration::ZERO);
            // Let it snap back to occupy the indicator's final sliver space only.
            t.pump(Duration::from_secs(2));
            assert!(t.contains(matches_builder(Refresh, exactly(60.0))));
            assert_eq!(t.indicator_rect(), Rect::from_ltrb(0.0, 0.0, 800.0, 60.0));
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 60.0, 800.0, 260.0));

            t.complete_refresh();
            t.pump(Duration::ZERO);

            assert!(t.contains(matches_builder(Done, exactly(60.0))));

            // Waiting for refresh control to reach approximately 5% of height
            t.pump(Duration::from_millis(400));

            let top = t.item_top(0);
            assert!(top > 0.0 && top < 6.0, "the first item's top: {top}");
            assert_about(t.indicator_rect().height(), top, "the indicator's height");
            assert!(t.contains(matches_builder(Inactive, about_position(top))));
            assert!(t.indicator_shown());
        });
    }

    #[test]
    fn a_retracting_sliver_during_done_cannot_be_pulled_to_refresh_again_until_fully_retracted() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);
            assert!(t.contains_refresh_task());

            t.complete_refresh();
            t.pump(Duration::ZERO);
            // Still overscrolled here.
            assert!(t.contains(matches_builder(Done, exactly(150.0))));

            // Let it start going away but not fully.
            t.pump(Duration::from_millis(100));
            // The refresh indicator is still building.
            let retracting_dy = t.indicator_rect().bottom;
            assert!(retracting_dy > 0.0 && retracting_dy < 150.0);
            assert!(t.contains(matches_builder(Done, about_position(retracting_dy))));

            // Start another drag by an amount that would have been enough to
            // trigger another refresh if it were in the right state.
            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);

            // Instead, it's still in the done state because the sliver never
            // fully retracted.
            let dragged_dy = t.indicator_rect().bottom;
            assert!(dragged_dy > retracting_dy && dragged_dy < 150.0 + retracting_dy);
            assert!(t.contains(matches_builder(Done, about_position(dragged_dy))));

            // Now let it fully go away.
            t.pump(Duration::from_secs(5));
            assert!(!t.indicator_shown());
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));

            // Start another drag. It's now in drag mode.
            t.drag_item(0, Offset::new(0.0, 40.0), 0.0);
            t.pump(Duration::ZERO);
            assert!(t.contains(matches_builder(Drag, exactly(40.0))));
        });
    }

    #[test]
    fn a_sliver_held_in_overscroll_when_the_task_finishes_completes_normally() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.start_gesture(Offset::ZERO);
            // Start a refresh.
            t.move_by(Offset::new(0.0, 150.0));
            t.pump(Duration::ZERO);
            assert!(t.contains_refresh_task());

            // Complete the task while held down.
            t.complete_refresh();
            t.pump(Duration::ZERO);

            // Still overscrolled here.
            assert!(t.contains(matches_builder(Done, exactly(150.0))));
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 150.0, 800.0, 350.0));

            t.up();
            t.pump(Duration::ZERO);
            t.pump(Duration::from_secs(5));
            assert!(!t.indicator_shown());
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));
        });
    }

    #[test]
    fn does_nothing_unless_it_can_be_overscrolled_at_the_start_of_the_list() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            let first_list = t.list_of_stuff();
            // Unkeyed, since the first list holds the keys.
            let second_list = SliverList::builder(|_app, _context, index| {
                Some(
                    SizedBox::new()
                        .height(200.0)
                        .child(Center::new().child(Text::new(index.to_string())))
                        .into_widget(),
                )
            })
            .item_count(TEST_LIST_LENGTH)
            .into_widget();
            t.pump_app(vec![
                first_list,
                // It's in the middle now.
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task())
                    .into_widget(),
                second_list,
            ]);

            let first_sized_box = t.item_rect(0).center();
            t.fling_from(first_sized_box, Offset::new(0.0, 200.0), 2000.0);
            // IgnorePointer is enabled while scroll is ballistic.
            let first_sized_box = t.item_rect(0).center();
            t.fling_from(first_sized_box, Offset::new(0.0, -200.0), 3000.0);

            assert!(t.invocations().is_empty());
        });
    }

    #[test]
    fn without_an_on_refresh_the_builder_is_called_with_armed_for_one_frame_then_the_sliver_goes_away()
     {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            t.drag_item(0, Offset::new(0.0, 150.0), 0.0);
            t.pump(Duration::ZERO);

            assert!(matches_builder(Armed, exactly(150.0)).matches(&t.invocations()[0]));

            t.pump(Duration::from_millis(10));

            assert!(
                matches_builder(Done, between(0.0, 150.0))
                    .matches(t.invocations().last().expect("the builder was called"))
            );

            t.pump(Duration::from_secs(5));
            assert!(!t.indicator_shown());
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));
        });
    }

    #[test]
    fn does_not_panic_when_dragged() {
        for_each_variant(&APPLE, |t| {
            // Dart's `Future<void>.delayed(const Duration(days: 2000))`: a refresh that does
            // not end within the case.
            let list = CustomScrollView::new()
                .physics(Rc::new(BouncingScrollPhysics::new()))
                .slivers([CupertinoSliverRefreshControl::new()
                    .on_refresh(Rc::new(|app: &mut App| {
                        let never: Completer<()> = Completer::new();
                        let future = never.future();
                        app.spawn(async move |_cx| future.await)
                    }))
                    .into_widget()]);
            t.pump_widget(CupertinoApp::new().home(list).into_widget());

            t.drag_from(Offset::new(100.0, 10.0), Offset::new(0.0, 50.0), 0.0);
            t.pump(Duration::ZERO);

            t.drag_from(Offset::new(100.0, 10.0), Offset::new(0.0, 500.0), 0.0);
            t.pump(Duration::ZERO);
        });
    }

    #[test]
    fn displays_properly_when_the_refresh_sliver_is_behind_the_large_title_nav_bar_sliver() {
        // https://github.com/flutter/flutter/issues/74516: the refresh sliver's overscroll is
        // not eaten by the nav bar sliver.
        for_each_variant(&APPLE, |t| {
            let list = t.list_of_stuff();
            t.pump_app(vec![
                CupertinoSliverNavigationBar::new()
                    .large_title(Text::new("Title"))
                    .into_widget(),
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .into_widget(),
                list,
            ]);

            let initial_first_cell_y = t.item_top(0);

            // Drag down but not enough to trigger the refresh.
            t.drag_item(0, Offset::new(0.0, 50.0), 0.0);
            t.pump(Duration::ZERO);

            assert!(matches_builder(Drag, exactly(50.0)).matches(&t.invocations()[0]));
            assert_eq!(t.invocations().len(), 1);

            assert_eq!(t.item_top(0), initial_first_cell_y + 50.0);
        });
    }

    // State machine tests.

    #[test]
    fn starts_in_the_inactive_state() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            assert_eq!(t.state(), Inactive);
        });
    }

    #[test]
    fn goes_to_drag_and_returns_to_inactive_in_a_small_drag() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(CupertinoSliverRefreshControl::new().builder(t.builder()));

            t.drag_item(0, Offset::new(0.0, 20.0), K_DRAG_SLOP_DEFAULT);
            t.pump(Duration::ZERO);

            assert_eq!(t.state(), Drag);

            t.pump(Duration::from_secs(2));

            assert_eq!(t.state(), Inactive);
        });
    }

    #[test]
    fn goes_to_armed_the_frame_it_passes_the_threshold() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .refresh_trigger_pull_distance(80.0),
            );

            t.start_gesture(Offset::ZERO);
            t.move_by(Offset::new(0.0, 79.0));
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Drag);
            // Overscrolling, need to move more than 1px.
            if t.on_mac_os() {
                t.move_by(Offset::new(0.0, 20.0));
            } else {
                t.move_by(Offset::new(0.0, 3.0));
            }
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);
        });
    }

    #[test]
    fn goes_to_refresh_the_frame_it_crossed_back_the_refresh_threshold() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task())
                    .refresh_trigger_pull_distance(90.0)
                    .refresh_indicator_extent(50.0),
            );

            t.start_gesture(Offset::ZERO);
            // Arm it.
            t.move_by(Offset::new(0.0, 90.0));
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);

            // Overscrolling, need to move more than -40.
            t.move_by(Offset::new(0.0, -80.0));
            t.pump(Duration::ZERO);
            // Below 50 now.
            let expected = if t.on_mac_os() {
                10.0
            } else {
                49.775111111111116
            };
            assert_about(t.item_top(0), expected, "the first item's top");
            assert_eq!(t.state(), Refresh);
        });
    }

    #[test]
    fn goes_to_done_internally_as_soon_as_the_task_finishes() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.drag_item(0, Offset::new(0.0, 100.0), 0.0);
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);
            // The sliver scroll offset correction is applied on the next frame.
            t.pump(Duration::ZERO);

            t.pump(Duration::from_secs(2));
            assert_eq!(t.state(), Refresh);
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 60.0, 800.0, 260.0));

            t.complete_refresh();
            // The task completed between frames. The internal state goes to done
            // right away even though the sliver gets a new offset correction the
            // next frame.
            assert_eq!(t.state(), Done);
        });
    }

    #[test]
    fn goes_back_to_inactive_when_retracting_back_past_10_percent_of_the_arming_distance() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.start_gesture(Offset::ZERO);
            t.move_by(Offset::new(0.0, 150.0));
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);

            t.complete_refresh();
            assert_eq!(t.state(), Done);
            t.pump(Duration::ZERO);

            // Now back in overscroll mode.
            if t.on_mac_os() {
                t.move_by(Offset::new(0.0, -125.0));
            } else {
                t.move_by(Offset::new(0.0, -200.0));
            }
            t.pump(Duration::ZERO);
            let expected = if t.on_mac_os() {
                25.0
            } else {
                27.944444444444457
            };
            assert_about(t.item_top(0), expected, "the first item's top");
            // Need to bring it to 100 * 0.1 to reset to inactive.
            assert_eq!(t.state(), Done);

            if t.on_mac_os() {
                t.move_by(Offset::new(0.0, -16.0));
            } else {
                t.move_by(Offset::new(0.0, -35.0));
            }
            t.pump(Duration::ZERO);
            let expected = if t.on_mac_os() {
                9.0
            } else {
                9.313890708161875
            };
            assert_about(t.item_top(0), expected, "the first item's top");
            assert_eq!(t.state(), Inactive);
        });
    }

    #[test]
    fn goes_back_to_inactive_if_already_scrolled_away_when_the_task_completes() {
        for_each_variant(&APPLE, |t| {
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(t.builder())
                    .on_refresh(t.refresh_task()),
            );

            t.start_gesture(Offset::ZERO);
            t.move_by(Offset::new(0.0, 150.0));
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);
            // Sliver scroll offset correction is applied one frame later.
            t.pump(Duration::ZERO);

            t.move_by(Offset::new(0.0, -300.0));
            let indicator_destination_position = if t.on_mac_os() {
                -150.0
            } else {
                -145.0332383665717
            };
            t.pump(Duration::ZERO);
            // The refresh indicator is offscreen now.
            assert_about(
                t.item_top(0),
                indicator_destination_position,
                "the first item's top",
            );
            assert_eq!(t.state(), Refresh);

            t.complete_refresh();
            // The sliver layout extent is removed on next frame.
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Inactive);
            // Nothing moved.
            assert_about(
                t.item_top(0),
                indicator_destination_position,
                "the first item's top",
            );
            t.pump(Duration::from_secs(2));
            // Everything stayed as is.
            assert_about(
                t.item_top(0),
                indicator_destination_position,
                "the first item's top",
            );
        });
    }

    #[test]
    fn does_not_have_to_build_any_indicators_or_occupy_space_during_refresh() {
        for_each_variant(&APPLE, |t| {
            t.indicate_with_text();
            t.pump_control(
                CupertinoSliverRefreshControl::new()
                    .builder(None)
                    .on_refresh(t.refresh_task())
                    .refresh_indicator_extent(0.0),
            );

            t.drag_item(0, Offset::new(0.0, 150.0), K_DRAG_SLOP_DEFAULT);
            t.pump(Duration::ZERO);
            assert_eq!(t.state(), Armed);

            t.pump(Duration::ZERO);
            t.pump(Duration::from_secs(5));
            // In refresh mode but has no UI.
            assert_eq!(t.state(), Refresh);
            assert_eq!(t.item_rect(0), Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));

            t.complete_refresh();
            t.pump(Duration::ZERO);
            // Goes to inactive right away since the sliver is already collapsed.
            assert_eq!(t.state(), Inactive);
        });
    }

    #[test]
    fn build_refresh_indicator_reveals_ticks_with_the_progress() {
        // Dart pumps the three trees one after another; each is a tree of its own here.
        for pulled_extent in [10.0, 26.0, 100.0] {
            let mut t = Tester::new(TargetPlatform::IOS);
            t.pump_widget(
                CupertinoApp::new()
                    .home(Builder::new(move |app, context| {
                        CupertinoSliverRefreshControl::build_refresh_indicator(
                            app,
                            context,
                            Drag,
                            pulled_extent,
                            100.0,
                            10.0,
                        )
                    }))
                    .into_widget(),
            );
            assert_eq!(t.activity_indicator().0, pulled_extent / 100.0);
        }
    }

    #[test]
    fn the_indicator_does_not_become_larger_when_overscrolled() {
        // https://github.com/flutter/flutter/issues/79841
        let mut t = Tester::new(TargetPlatform::IOS);
        t.pump_widget(
            Directionality::new(
                TextDirection::Ltr,
                Builder::new(|app, context| {
                    CupertinoSliverRefreshControl::build_refresh_indicator(
                        app, context, Done, 120.0, 100.0, 10.0,
                    )
                }),
            )
            .into_widget(),
        );

        assert_eq!(t.activity_indicator().1, 14.0);
    }

    #[test]
    fn does_not_panic_when_the_paint_extent_exceeds_the_remaining_paint_extent() {
        // https://github.com/flutter/flutter/issues/46871
        let mut t = Tester::new(TargetPlatform::IOS);
        t.pump_widget(
            CupertinoApp::new()
                .home(
                    CustomScrollView::new()
                        .physics(Rc::new(BouncingScrollPhysics::new()))
                        .slivers([
                            CupertinoSliverRefreshControl::new().into_widget(),
                            SliverList::builder(|_app, _context, _index| {
                                Some(SizedBox::new().height(100.0).into_widget())
                            })
                            .item_count(20)
                            .into_widget(),
                        ]),
                )
                .into_widget(),
        );

        // Drag the content down far enough so that
        // geometry.paint_extent > constraints.max_paint_extent
        t.drag_from(
            Offset::new(10.0, 10.0),
            Offset::new(0.0, 500.0),
            K_DRAG_SLOP_DEFAULT,
        );
        t.pump(Duration::ZERO);
    }
}
