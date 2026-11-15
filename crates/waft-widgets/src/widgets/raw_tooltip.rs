//! Flutter counterpart: `widgets/raw_tooltip.dart`.

use std::any::Any;
use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStatusListener, AnimationStyle, Curve, CurvedAnimation, Curves,
};
use waft_embedder::{Matrix4, Offset, PointerDeviceKind, Size, TextBaseline};
use waft_foundation::{App, Handle, Listener, OptionRetained, Timer};
use waft_gestures::{
    GestureBinding, LongPressGestureRecognizer, PointerDownEvent, PointerEnterEvent,
    PointerEnterEventListener, PointerEvent, PointerExitEvent, PointerExitEventListener,
    PointerRoute, PrimaryPointerLeafData, TapGestureRecognizer,
};
use waft_painting::{position_dependent_box, transform_point};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestEntry, BoxHitTestResult,
    HitTestBehavior, PaintingContext, PipelineOwner, RenderBox, RenderBoxData, RenderHandle,
    RenderMouseRegionBase, RenderMouseRegionData, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    RenderProxyBoxWithHitTestBehavior, SingleChildLayoutDelegate,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseTrackerAnnotation;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, State,
    StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::{
    CustomSingleChildLayout, IgnorePointer, Listener as PointerListener, MouseRegion, Positioned,
    Semantics, SizedBox,
};
use crate::widgets::debug::debug_check_has_overlay;
use crate::widgets::feedback::Feedback;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::overlay::{OverlayChildLayoutInfo, OverlayPortal, OverlayPortalController};
use crate::widgets::ticker_provider::{
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
};
use waft_foundation::Ref;

/// Dart's `_kDefaultAnimationStyle`; a function so its curve is the one shared
/// `Curves::fast_out_slow_in()`, which `AnimationStyle`'s `==` compares by identity.
fn default_animation_style() -> AnimationStyle {
    AnimationStyle::new(
        Some(Curves::fast_out_slow_in()),
        Some(Duration::from_millis(150)),
        None,
        Some(Duration::from_millis(75)),
    )
}

/// Signature for building the tooltip overlay child.
///
/// The animation property exposes the underlying tooltip overlay child show
/// and hide animation. This can be used to drive animations that sync up with
/// the tooltip overlay child show/hide animation, for example to fade the
/// tooltip in and out.
pub type TooltipComponentBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef>;

/// Signature for computing the position of a tooltip.
///
/// The [`TooltipPositionContext`] contains all the necessary information for
/// positioning the tooltip, including the target location, sizes, offset, and
/// positioning preference.
///
/// Returns the offset from the top left of the overlay to the top left of the tooltip.
///
/// See also:
///
///  * [`TooltipPositionContext`], which contains the positioning parameters.
pub type TooltipPositionDelegate = Rc<dyn Fn(TooltipPositionContext) -> Offset>;

/// Contextual information for positioning a tooltip.
///
/// This immutable data class contains all the necessary information for computing
/// the position of a tooltip relative to its target widget.
///
/// See also:
///
///  * [`TooltipPositionDelegate`], which uses this context to compute tooltip positions.
// TODO(victorsanni): Consider removing preferBelow and verticalOffset since
// they are already available in the context of RawTooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipPositionContext {
    /// The center point of the target widget in the global coordinate system.
    pub target: Offset,

    /// The size of the target widget that triggers the tooltip.
    pub target_size: Size,

    /// The size of the tooltip itself.
    pub tooltip_size: Size,

    /// The configured vertical offset.
    pub vertical_offset: f64,

    /// Whether the tooltip prefers to be positioned below the target.
    pub prefer_below: bool,

    /// The size of the overlay within which the tooltip is displayed.
    pub overlay_size: Size,
}

impl TooltipPositionContext {
    /// Creates a tooltip position context.
    pub fn new(
        target: Offset,
        target_size: Size,
        tooltip_size: Size,
        vertical_offset: f64,
    ) -> TooltipPositionContext {
        TooltipPositionContext {
            target,
            target_size,
            tooltip_size,
            vertical_offset,
            prefer_below: true,
            overlay_size: Size::INFINITE,
        }
    }

    /// Dart `TooltipPositionContext(preferBelow:)`.
    pub fn prefer_below(mut self, prefer_below: bool) -> TooltipPositionContext {
        self.prefer_below = prefer_below;
        self
    }

    /// Dart `TooltipPositionContext(overlaySize:)`.
    pub fn overlay_size(mut self, overlay_size: Size) -> TooltipPositionContext {
        self.overlay_size = overlay_size;
        self
    }
}

/// How touch events should trigger a tooltip.
///
/// When using a pointer device like a mouse, a tooltip is shown as soon as
/// a pointer hovers over the widget, regardless of the value of
/// [`RawTooltip::trigger_mode`].
///
/// A tooltip might also be triggered through other means regardless of this
/// option, such as by calling `ensure_tooltip_visible`.
///
/// See also:
///
///  * [`RawTooltip::trigger_mode`], which uses this enum.
///
/// See also:
///
///   * [`RawTooltip::hover_delay`], which defines the length of time that
///     a pointer must hover over a tooltip's widget before the tooltip
///     will be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipTriggerMode {
    /// Tooltip will not be triggered by touch events.
    Manual,

    /// Tooltip will be shown after a long press.
    ///
    /// See also:
    ///
    ///   * `GestureDetector.on_long_press`, the event that is used for trigger.
    ///   * [`Feedback::for_long_press`], the feedback method called when feedback is enabled.
    LongPress,

    /// Tooltip will be shown after a single tap.
    ///
    /// See also:
    ///
    ///   * `GestureDetector.on_tap`, the event that is used for trigger.
    ///   * [`Feedback::for_tap`], the feedback method called when feedback is enabled.
    Tap,
}

/// Signature for when a tooltip is triggered.
pub type TooltipTriggeredCallback = Listener;

/// A special [`MouseRegion`] that when nested, only the first [`ExclusiveMouseRegion`]
/// to be hit in hit-testing order will be added to the [`BoxHitTestResult`] (i.e.,
/// child over parent, last sibling over first sibling).
///
/// The `on_enter` method will be called when a mouse pointer enters this
/// [`MouseRegion`], and there is no other [`ExclusiveMouseRegion`]s obstructing
/// this [`ExclusiveMouseRegion`] from receiving the events. This includes the
/// case where the mouse cursor stays within the paint bounds of an outer
/// [`ExclusiveMouseRegion`], but moves outside of the bounds of the inner
/// [`ExclusiveMouseRegion`] that was initially blocking the outer widget.
///
/// Likewise, `on_exit` is called when the a mouse pointer moves out of the paint
/// bounds of this widget, or moves into another [`ExclusiveMouseRegion`] that
/// overlaps this widget in hit-testing order.
///
/// This widget doesn't affect [`MouseRegion`]s that aren't [`ExclusiveMouseRegion`]s,
/// or other `HitTestTarget`s in the tree.
///
/// Dart's `_ExclusiveMouseRegion extends MouseRegion`: the inherited fields are the
/// [`MouseRegion`] it holds.
#[derive(Debug)]
struct ExclusiveMouseRegion {
    mouse_region: MouseRegion,
}

impl ExclusiveMouseRegion {
    fn new<K>(
        on_enter: PointerEnterEventListener,
        on_exit: PointerExitEventListener,
        child: impl IntoWidget<K>,
    ) -> ExclusiveMouseRegion {
        ExclusiveMouseRegion {
            mouse_region: MouseRegion::new()
                .on_enter(on_enter)
                .on_exit(on_exit)
                .child(child),
        }
    }
}

impl RenderObjectWidget for ExclusiveMouseRegion {
    type RenderObject = RenderExclusiveMouseRegion;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let render_object = RenderExclusiveMouseRegion::new(app);
        render_object.set_on_enter(app, self.mouse_region.on_enter.clone());
        render_object.set_on_exit(app, self.mouse_region.on_exit.clone());
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderExclusiveMouseRegion>,
    ) {
        self.mouse_region.apply_configuration(app, render_object);
    }
}

impl SingleChildRenderObjectWidget for ExclusiveMouseRegion {
    fn child(&self) -> Option<&WidgetRef> {
        self.mouse_region.child.as_ref()
    }
}

/// Dart's statics `_RenderExclusiveMouseRegion.isOutermostMouseRegion` and
/// `foundInnermostMouseRegion`, kept per [`App`].
struct ExclusiveMouseRegionHitTest {
    is_outermost_mouse_region: bool,
    found_innermost_mouse_region: bool,
}

impl Default for ExclusiveMouseRegionHitTest {
    fn default() -> ExclusiveMouseRegionHitTest {
        ExclusiveMouseRegionHitTest {
            is_outermost_mouse_region: true,
            found_innermost_mouse_region: false,
        }
    }
}

/// Dart's `_RenderExclusiveMouseRegion extends RenderMouseRegion`: a
/// [`RenderMouseRegionBase`] leaf that overrides only `hit_test`.
struct RenderExclusiveMouseRegion {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    mouse_region: RenderMouseRegionData,
}

impl RenderExclusiveMouseRegion {
    fn new(app: &mut App) -> RenderHandle<RenderExclusiveMouseRegion> {
        RenderHandle::new_box(
            app,
            RenderExclusiveMouseRegion {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                mouse_region: RenderMouseRegionData::new(true),
            },
        )
    }
}

impl RenderMouseRegionBase for RenderExclusiveMouseRegion {
    fn mouse_region_data(self: RenderHandle<Self>, app: &App) -> &RenderMouseRegionData {
        &self.get(app).mouse_region
    }

    fn mouse_region_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderMouseRegionData {
        &mut self.get_mut(app).mouse_region
    }
}

impl RenderObjectWithChildMixin for RenderExclusiveMouseRegion {
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

impl RenderProxyBoxMixin for RenderExclusiveMouseRegion {
    fn compute_size_for_no_child(
        self: RenderHandle<Self>,
        _app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }
}

impl RenderProxyBoxWithHitTestBehavior for RenderExclusiveMouseRegion {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.mouse_region_data(app).behavior()
    }
}

impl RenderObject for RenderExclusiveMouseRegion {
    waft_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        RenderMouseRegionBase::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderMouseRegionBase::did_detach(self, app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn mouse_tracker_annotation(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<MouseTrackerAnnotation> {
        RenderMouseRegionBase::mouse_tracker_annotation(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderExclusiveMouseRegion {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let statics = app.singleton::<ExclusiveMouseRegionHitTest>();
        let mut is_hit = false;
        let outermost = app.get(statics).is_outermost_mouse_region;
        app.get_mut(statics).is_outermost_mouse_region = false;
        if self.size(app).contains(position) {
            is_hit = RenderBox::hit_test_children(self, app, result, position)
                || RenderBox::hit_test_self(self, app, position);
            if (is_hit || self.behavior(app) == HitTestBehavior::Translucent)
                && !app.get(statics).found_innermost_mouse_region
            {
                app.get_mut(statics).found_innermost_mouse_region = true;
                result.add(BoxHitTestEntry::new(self.as_box(), position).into());
            }
        }

        if outermost {
            // The outermost region resets the global states.
            let statics = app.get_mut(statics);
            statics.is_outermost_mouse_region = true;
            statics.found_innermost_mouse_region = false;
        }
        is_hit
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        _entry: &BoxHitTestEntry,
    ) {
        RenderMouseRegionBase::handle_event(self, app, event);
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// A widget that wraps a child to display an informative overlay in response to
/// user interactions, such as hovering or long-pressing.
///
/// Tooltips provide essential context by displaying text labels or brief
/// descriptions that explain the function of a button or other user interface
/// elements.
///
/// The tooltip can be triggered in several ways:
///
///  * By hovering a mouse pointer over the widget.
///  * By touch interactions, such as a long press or a tap, depending on the
///    configuration of [`trigger_mode`](Self::trigger_mode).
///  * Manually, by calling [`RawTooltipState::ensure_tooltip_visible`].
///
/// See also:
///
///   * `Tooltip`, a Material-themed [`RawTooltip`].
pub struct RawTooltip {
    pub key: Option<KeyRef>,

    /// The text to display in the tooltip's semantics announcement.
    ///
    /// This string is used by assistive technologies, most notably screen readers
    /// like TalkBack and VoiceOver, to describe the tooltip's purpose.
    ///
    /// Providing a non-empty string adds a `Semantics` tooltip string for
    /// assistive technologies. If the tooltip should not have a semantic
    /// description, this property must be explicitly set to `None`.
    pub semantics_tooltip: Option<String>,

    /// Builds the widget that will be displayed in the tooltip's overlay.
    ///
    /// The `animation` parameter is an [`Animation`] that maps to the tooltip's
    /// show and hide animation. Its value goes from 0.0 to 1.0 when the tooltip
    /// is shown, and from 1.0 to 0.0 when it is hidden.
    ///
    /// This animation can be used to create custom transitions for the tooltip,
    /// such as fading or scaling, by wrapping the tooltip's content in a
    /// `FadeTransition` or `ScaleTransition` and using the provided `animation`.
    ///
    /// The characteristics of the animation, such as its duration and curve, can
    /// be customized using the [`animation_style`](Self::animation_style) property.
    pub tooltip_builder: TooltipComponentBuilder,

    /// The length of time that a pointer must hover over a tooltip's widget
    /// before the tooltip will be shown.
    ///
    /// Defaults to 0 milliseconds (the tooltip is shown immediately upon hover).
    pub hover_delay: Duration,

    /// The length of time that the tooltip will be shown after a long press is
    /// released (if trigger_mode is [`TooltipTriggerMode::LongPress`]) or a tap is
    /// released (if trigger_mode is [`TooltipTriggerMode::Tap`]).
    ///
    /// This property does not affect mouse pointer devices.
    ///
    /// Defaults to 1.5 seconds (the tooltip is shown 1.5 seconds after a tap or
    /// long press is released).
    ///
    /// See also:
    ///
    ///  * [`dismiss_delay`](Self::dismiss_delay), which allows configuring the time until a
    ///    pointer disappears when hovering.
    pub touch_delay: Duration,

    /// The length of time that a pointer must have stopped hovering over a
    /// tooltip's widget before the tooltip will be hidden.
    ///
    /// Defaults to 100 milliseconds.
    ///
    /// See also:
    ///
    ///  * [`touch_delay`](Self::touch_delay), which allows configuring the length of time that
    ///    a tooltip will be visible after touch events are released.
    pub dismiss_delay: Duration,

    /// Whether the tooltip can be dismissed by tap.
    ///
    /// The default value is true.
    pub enable_tap_to_dismiss: bool,

    /// The [`TooltipTriggerMode`] that will show the tooltip.
    ///
    /// This property does not affect mouse devices. Setting [`trigger_mode`](Self::trigger_mode)
    /// to [`TooltipTriggerMode::Manual`] will not prevent the tooltip from showing when
    /// the mouse cursor hovers over it.
    pub trigger_mode: TooltipTriggerMode,

    /// Whether the tooltip should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// When null, the default value is true.
    ///
    /// See also:
    ///
    ///  * [`Feedback`], for providing platform-specific feedback to certain
    ///    actions.
    pub enable_feedback: bool,

    /// Called when the [`RawTooltip`] is triggered programmatically, on tap, or on
    /// long press.
    ///
    /// The tooltip is triggered after a tap when [`trigger_mode`](Self::trigger_mode) is
    /// [`TooltipTriggerMode::Tap`] or after a long press when
    /// [`trigger_mode`](Self::trigger_mode) is [`TooltipTriggerMode::LongPress`].
    ///
    /// Hovering over the tooltip with a mouse pointer does not trigger this
    /// callback.
    ///
    /// See also:
    ///
    /// * [`TooltipTriggerMode`], which defines the different ways a tooltip can be
    ///   triggered.
    pub on_triggered: Option<TooltipTriggeredCallback>,

    /// Used to override the curve and duration of the animation that shows and
    /// hides the tooltip.
    ///
    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the show tooltip animation duration. If it is null, defaults to 150ms.
    ///
    /// If [`AnimationStyle::curve`] is provided, it will be used to override
    /// the show tooltip animation curve. If it is null, defaults to
    /// `Curves::fast_out_slow_in`.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the hide tooltip animation duration. If it is null, defaults to
    /// 75ms.
    ///
    /// If [`AnimationStyle::reverse_curve`] is provided, it will be used to
    /// override the hide tooltip animation curve. If it is null, the same curve will be
    /// used as for the show tooltip animation. The reverse curve only takes
    /// effect when the tooltip starts hiding from the fully shown state; a
    /// tooltip dismissed while it is still animating in continues to use the
    /// show curve, to avoid a visual discontinuity. See
    /// [`CurvedAnimation::reverse_curve`].
    ///
    /// To disable the tooltip show/hide animation, use
    /// [`AnimationStyle::NO_ANIMATION`].
    // TODO(victorsanni): Add an example of chaining a physics-based animation to
    // the tooltip's underlying animation.
    pub animation_style: AnimationStyle,

    /// A custom position delegate function for computing where the tooltip should
    /// be positioned.
    ///
    /// If provided, this function will be called with a [`TooltipPositionContext`]
    /// containing all the necessary information for positioning the tooltip. The
    /// function should return an [`Offset`] indicating where to place the tooltip
    /// in the closest [`Overlay`](crate::widgets::overlay::Overlay).
    ///
    /// This allows for custom positioning such as left/right positioning, or any
    /// other arbitrary positioning logic.
    ///
    /// For example, if the `Overlay` takes up the entire screen, returning
    /// [`Offset::ZERO`] will position the tooltip at the top-left corner of the
    /// screen.
    ///
    /// The [`TooltipPositionContext`] provides information that can be used to
    /// position the tooltip relative to the target/child.
    ///
    /// If null, defaults to positioning the tooltip center-aligned and below the
    /// target using [`position_dependent_box`].
    ///
    /// See also:
    ///
    ///  * [`TooltipPositionContext`], which contains the positioning parameters.
    ///  * [`TooltipPositionDelegate`], the function signature for custom
    ///    positioning.
    pub position_delegate: Option<TooltipPositionDelegate>,

    /// Whether the tooltip should be invisible to hit testing.
    ///
    /// Defaults to false.
    ///
    /// See also:
    ///
    /// * [`IgnorePointer`], for more information about how pointer events are
    ///   handled or ignored.
    pub ignore_pointer: bool,

    /// The widget below this widget in the tree.
    ///
    /// The widget returned in [`tooltip_builder`](Self::tooltip_builder) will be shown when
    /// the user hovers over this child widget.
    pub child: WidgetRef,
}

/// Dart's static `RawTooltip._openedTooltips`, kept per [`App`].
#[derive(Default)]
struct OpenedTooltips {
    opened_tooltips: Vec<Handle<RawTooltipState>>,
}

impl OpenedTooltips {
    fn instance(app: &mut App) -> Handle<OpenedTooltips> {
        app.singleton::<OpenedTooltips>()
    }

    fn add(app: &mut App, tooltip: Handle<RawTooltipState>) {
        let this = OpenedTooltips::instance(app);
        app.get_mut(this).opened_tooltips.push(tooltip);
    }

    /// Dart's `List.remove`: the first occurrence only.
    fn remove(app: &mut App, tooltip: Handle<RawTooltipState>) {
        let this = OpenedTooltips::instance(app);
        let opened_tooltips = &mut app.get_mut(this).opened_tooltips;
        if let Some(index) = opened_tooltips.iter().position(|opened| *opened == tooltip) {
            opened_tooltips.remove(index);
        }
    }

    fn to_list(app: &mut App) -> Vec<Handle<RawTooltipState>> {
        let this = OpenedTooltips::instance(app);
        app.get(this).opened_tooltips.clone()
    }
}

impl RawTooltip {
    /// Creates a raw tooltip.
    ///
    /// The `semantics_tooltip`, `tooltip_builder`, and `child` arguments are
    /// required.
    pub fn new<K>(
        semantics_tooltip: Option<String>,
        tooltip_builder: impl Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef + 'static,
        child: impl IntoWidget<K>,
    ) -> RawTooltip {
        RawTooltip {
            key: None,
            semantics_tooltip,
            tooltip_builder: Rc::new(tooltip_builder),
            hover_delay: Duration::ZERO,
            touch_delay: Duration::from_millis(1500),
            dismiss_delay: Duration::from_millis(100),
            enable_tap_to_dismiss: true,
            trigger_mode: TooltipTriggerMode::LongPress,
            enable_feedback: true,
            on_triggered: None,
            animation_style: default_animation_style(),
            position_delegate: None,
            ignore_pointer: false,
            child: child.into_widget(),
        }
    }

    /// Dart `RawTooltip(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawTooltip {
        self.key = Some(key);
        self
    }

    /// Dart `RawTooltip(hoverDelay:)`.
    pub fn hover_delay(mut self, hover_delay: Duration) -> RawTooltip {
        self.hover_delay = hover_delay;
        self
    }

    /// Dart `RawTooltip(touchDelay:)`.
    pub fn touch_delay(mut self, touch_delay: Duration) -> RawTooltip {
        self.touch_delay = touch_delay;
        self
    }

    /// Dart `RawTooltip(dismissDelay:)`.
    pub fn dismiss_delay(mut self, dismiss_delay: Duration) -> RawTooltip {
        self.dismiss_delay = dismiss_delay;
        self
    }

    /// Dart `RawTooltip(enableTapToDismiss:)`.
    pub fn enable_tap_to_dismiss(mut self, enable_tap_to_dismiss: bool) -> RawTooltip {
        self.enable_tap_to_dismiss = enable_tap_to_dismiss;
        self
    }

    /// Dart `RawTooltip(triggerMode:)`.
    pub fn trigger_mode(mut self, trigger_mode: TooltipTriggerMode) -> RawTooltip {
        self.trigger_mode = trigger_mode;
        self
    }

    /// Dart `RawTooltip(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> RawTooltip {
        self.enable_feedback = enable_feedback;
        self
    }

    /// Dart `RawTooltip(onTriggered:)`.
    pub fn on_triggered(mut self, on_triggered: TooltipTriggeredCallback) -> RawTooltip {
        self.on_triggered = Some(on_triggered);
        self
    }

    /// Dart `RawTooltip(animationStyle:)`.
    pub fn animation_style(mut self, animation_style: AnimationStyle) -> RawTooltip {
        self.animation_style = animation_style;
        self
    }

    /// Dart `RawTooltip(positionDelegate:)`.
    pub fn position_delegate(mut self, position_delegate: TooltipPositionDelegate) -> RawTooltip {
        self.position_delegate = Some(position_delegate);
        self
    }

    /// Dart `RawTooltip(ignorePointer:)`.
    pub fn ignore_pointer(mut self, ignore_pointer: bool) -> RawTooltip {
        self.ignore_pointer = ignore_pointer;
        self
    }

    /// Dismiss all of the tooltips that are currently shown on the screen,
    /// including those with mouse cursors currently hovering over them.
    ///
    /// This method returns true if it successfully dismisses at least one tooltip
    /// and returns false if there is no tooltip currently displayed.
    pub fn dismiss_all_tool_tips(app: &mut App) -> bool {
        // Avoid concurrent modification.
        let opened_tooltips = OpenedTooltips::to_list(app);
        if opened_tooltips.is_empty() {
            return false;
        }
        for state in opened_tooltips {
            debug_assert!(state.mounted(app));
            state.schedule_dismiss_tooltip(app, Duration::ZERO);
        }
        true
    }
}

impl Debug for RawTooltip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawTooltip")
            .field("key", &self.key)
            .field("semantics", &self.semantics_tooltip)
            .field("hover delay", &self.hover_delay)
            .field("touch delay", &self.touch_delay)
            .field("dismiss delay", &self.dismiss_delay)
            .field("triggerMode", &self.trigger_mode)
            .field("enableFeedback", &self.enable_feedback)
            .field("positionDelegate", &self.position_delegate.is_some())
            .field("animationStyle", &self.animation_style)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RawTooltip {
    type State = RawTooltipState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawTooltipState {
        RawTooltipState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            overlay_controller: None,
            timer: None,
            backing_controller: None,
            backing_overlay_animation: None,
            long_press_recognizer: None,
            tap_recognizer: None,
            active_hovering_pointer_devices: HashSet::new(),
            animation_status: AnimationStatus::Dismissed,
        }
    }
}

/// Contains the state for a [`RawTooltip`].
///
/// This class can be used to programmatically show the [`RawTooltip`]. See the
/// [`ensure_tooltip_visible`](Self::ensure_tooltip_visible) method.
pub struct RawTooltipState {
    state: StateData<RawTooltip>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because a state is created without the [`App`].
    overlay_controller: Option<Ref<OverlayPortalController>>,

    timer: Option<Timer>,
    backing_controller: Option<Ref<AnimationController>>,
    backing_overlay_animation: Option<Ref<CurvedAnimation>>,

    long_press_recognizer: Option<Ref<LongPressGestureRecognizer>>,
    tap_recognizer: Option<Ref<TapGestureRecognizer>>,

    // The ids of mouse devices that are keeping the tooltip from being dismissed.
    //
    // Device ids are added to this set in handle_mouse_enter, and removed in
    // handle_mouse_exit. The set is cleared in handle_tap_to_dismiss, typically when
    // a PointerDown event interacts with some other UI component.
    active_hovering_pointer_devices: HashSet<i64>,

    animation_status: AnimationStatus,
}

impl RawTooltipState {
    fn overlay_controller(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self)
            .overlay_controller
            .handle()
            .expect("created in init_state")
    }

    fn show_duration(self: Handle<Self>, app: &App) -> Duration {
        self.widget(app)
            .animation_style
            .duration
            .or(default_animation_style().duration)
            .expect("the default style has a duration")
    }

    fn hide_duration(self: Handle<Self>, app: &App) -> Duration {
        self.widget(app)
            .animation_style
            .reverse_duration
            .or(default_animation_style().reverse_duration)
            .expect("the default style has a reverse duration")
    }

    fn show_curve(self: Handle<Self>, app: &App) -> Rc<dyn Curve> {
        self.widget(app)
            .animation_style
            .curve
            .clone()
            .or(default_animation_style().curve)
            .expect("the default style has a curve")
    }

    fn hide_curve(self: Handle<Self>, app: &App) -> Option<Rc<dyn Curve>> {
        self.widget(app).animation_style.reverse_curve.clone()
    }

    fn controller(self: Handle<Self>, app: &mut App) -> Handle<AnimationController> {
        if let Some(controller) = app.get(self).backing_controller.handle() {
            return controller;
        }
        let show_duration = self.show_duration(app);
        let hide_duration = self.hide_duration(app);
        let controller = AnimationController::create(
            app,
            None,
            Some(show_duration),
            Some(hide_duration),
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::handle_status_changed),
        );
        let handle = controller.handle();
        app.get_mut(self).backing_controller = Some(controller);
        handle
    }

    fn overlay_animation(self: Handle<Self>, app: &mut App) -> Handle<CurvedAnimation> {
        if let Some(overlay_animation) = app.get(self).backing_overlay_animation.handle() {
            return overlay_animation;
        }
        self.controller(app);
        let parent: AnimationRef<f64> = app
            .get(self)
            .backing_controller
            .as_ref()
            .expect("made above")
            .erase();
        let show_curve = self.show_curve(app);
        let hide_curve = self.hide_curve(app);
        let overlay_animation = CurvedAnimation::create(app, parent, show_curve, hide_curve);
        let handle = overlay_animation.handle();
        app.get_mut(self).backing_overlay_animation = Some(overlay_animation);
        handle
    }

    fn handle_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        debug_assert!(self.mounted(app));
        match (
            app.get(self).animation_status.is_dismissed(),
            status.is_dismissed(),
        ) {
            (false, true) => {
                OpenedTooltips::remove(app, self);
                self.overlay_controller(app).hide(app);
            }
            (true, false) => {
                self.overlay_controller(app).show(app);
                OpenedTooltips::add(app, self);
            }
            (true, true) | (false, false) => {}
        }
        app.get_mut(self).animation_status = status;
    }

    fn schedule_show_tooltip(
        self: Handle<Self>,
        app: &mut App,
        with_delay: Duration,
        touch_delay: Option<Duration>,
    ) {
        debug_assert!(self.mounted(app));
        fn show(this: Handle<RawTooltipState>, app: &mut App, touch_delay: Option<Duration>) {
            debug_assert!(this.mounted(app));

            this.controller(app).forward(app, None);
            let controller = app
                .get(this)
                .backing_controller
                .clone()
                .expect("made above");
            if let Some(timer) = app.get(this).timer {
                timer.cancel(app);
            }
            let timer = touch_delay.map(|touch_delay| {
                Timer::new(
                    app,
                    touch_delay,
                    Listener::new(move |app| {
                        controller.reverse(app, None);
                    }),
                )
            });
            app.get_mut(this).timer = timer;
        }

        debug_assert!(
            !app.get(self)
                .timer
                .is_some_and(|timer| timer.is_active(app))
                || app
                    .get(self)
                    .backing_controller
                    .as_ref()
                    .is_none_or(|controller| controller.status(app) != AnimationStatus::Reverse),
            "timer must not be active when the tooltip is animating out"
        );
        let controller = self.controller(app);
        if controller.is_dismissed(app) && !with_delay.is_zero() {
            if let Some(timer) = app.get(self).timer {
                timer.cancel(app);
            }
            let timer = Timer::new(
                app,
                with_delay,
                Listener::new(move |app| show(self, app, touch_delay)),
            );
            app.get_mut(self).timer = Some(timer);
        } else {
            // If the tooltip is already animating in or fully visible, skip
            // the animation and show the tooltip immediately.
            show(self, app, touch_delay);
        }
    }

    fn schedule_dismiss_tooltip(self: Handle<Self>, app: &mut App, with_delay: Duration) {
        debug_assert!(self.mounted(app));
        debug_assert!(
            !app.get(self)
                .timer
                .is_some_and(|timer| timer.is_active(app))
                || app
                    .get(self)
                    .backing_controller
                    .as_ref()
                    .is_none_or(|controller| controller.status(app) != AnimationStatus::Reverse),
            "timer must not be active when the tooltip is animating out"
        );

        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        // Use backing_controller instead of controller to prevent the lazy getter
        // from instantiating an AnimationController unnecessarily.
        if app
            .get(self)
            .backing_controller
            .as_ref()
            .is_some_and(|controller| controller.is_forward_or_completed(app))
        {
            let controller = self.controller(app);
            // Dismiss when the tooltip is animating in: if there's a dismiss delay
            // we'll allow the animation to continue until the delay timer fires.
            if !with_delay.is_zero() {
                let timer = Timer::new(
                    app,
                    with_delay,
                    Listener::new(move |app| {
                        controller.reverse(app, None);
                    }),
                );
                app.get_mut(self).timer = Some(timer);
            } else {
                controller.reverse(app, None);
            }
        }
    }

    fn handle_pointer_down(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        // PointerDeviceKinds that don't support hovering.
        let trigger_mode_device_kinds = [
            PointerDeviceKind::InvertedStylus,
            PointerDeviceKind::Stylus,
            PointerDeviceKind::Touch,
            PointerDeviceKind::Unknown,
            // MouseRegion only tracks PointerDeviceKind == mouse.
            PointerDeviceKind::Trackpad,
        ];
        match self.widget(app).trigger_mode {
            TooltipTriggerMode::LongPress => {
                let recognizer = match app.get(self).long_press_recognizer.handle() {
                    Some(recognizer) => recognizer,
                    None => {
                        let recognizer = LongPressGestureRecognizer::new(app)
                            .supported_devices(app, trigger_mode_device_kinds);
                        let handle = recognizer.handle();
                        app.get_mut(self).long_press_recognizer = Some(recognizer);
                        handle
                    }
                };
                recognizer.set_on_long_press_cancel(
                    app,
                    Some(Listener::handle_method(self, Self::handle_tap_to_dismiss)),
                );
                recognizer.set_on_long_press(
                    app,
                    Some(Listener::handle_method(self, Self::handle_long_press)),
                );
                recognizer.set_on_long_press_up(
                    app,
                    Some(Listener::handle_method(self, Self::handle_press_up)),
                );
                recognizer.add_pointer(app, event);
            }
            TooltipTriggerMode::Tap => {
                let recognizer = match app.get(self).tap_recognizer.handle() {
                    Some(recognizer) => recognizer,
                    None => {
                        let recognizer = TapGestureRecognizer::new(app)
                            .supported_devices(app, trigger_mode_device_kinds);
                        let handle = recognizer.handle();
                        app.get_mut(self).tap_recognizer = Some(recognizer);
                        handle
                    }
                };
                recognizer.set_on_tap_cancel(
                    app,
                    Some(Listener::handle_method(self, Self::handle_tap_to_dismiss)),
                );
                recognizer.set_on_tap(app, Some(Listener::handle_method(self, Self::handle_tap)));
                recognizer.add_pointer(app, event);
            }
            TooltipTriggerMode::Manual => {}
        }
    }

    // For PointerDownEvents, this method will be called after handle_pointer_down.
    fn handle_global_pointer_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        debug_assert!(self.mounted(app));
        let pointer = Some(event.pointer());
        let tap_primary_pointer = app
            .get(self)
            .tap_recognizer
            .as_ref()
            .and_then(|recognizer| app.get(recognizer).primary().primary_pointer());
        let long_press_primary_pointer = app
            .get(self)
            .long_press_recognizer
            .as_ref()
            .and_then(|recognizer| app.get(recognizer).primary().primary_pointer());
        if tap_primary_pointer == pointer || long_press_primary_pointer == pointer {
            // This is a pointer of interest specified by the trigger mode, since it's
            // picked up by the recognizer.
            //
            // The recognizer will later determine if this is indeed a "trigger"
            // gesture and dismiss the tooltip if that's not the case. However there's
            // still a chance that the PointerEvent was cancelled before the gesture
            // recognizer gets to emit a tap/longPress down, in which case the onCancel
            // callback (handle_tap_to_dismiss) will not be called.
            return;
        }
        if (app.get(self).timer.is_none() && self.controller(app).is_dismissed(app))
            || !matches!(event, PointerEvent::Down(_))
        {
            return;
        }
        self.handle_tap_to_dismiss(app);
    }

    // The primary pointer is not part of a "trigger" gesture so the tooltip
    // should be dismissed.
    fn handle_tap_to_dismiss(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).enable_tap_to_dismiss {
            return;
        }
        self.schedule_dismiss_tooltip(app, Duration::ZERO);
        app.get_mut(self).active_hovering_pointer_devices.clear();
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        let tooltip_created = self.controller(app).is_dismissed(app);
        if tooltip_created && self.widget(app).enable_feedback {
            debug_assert!(self.widget(app).trigger_mode == TooltipTriggerMode::Tap);
            Feedback::for_tap(app, self.context(app));
        }
        if let Some(on_triggered) = self.widget(app).on_triggered.clone() {
            on_triggered.call(app);
        }
        let touch_delay = if app.get(self).active_hovering_pointer_devices.is_empty() {
            Some(self.widget(app).touch_delay)
        } else {
            // active_hovering_pointer_devices keep the tooltip visible.
            None
        };
        self.schedule_show_tooltip(app, Duration::ZERO, touch_delay);
    }

    // When a "trigger" gesture is recognized and the pointer down even is a part
    // of it.
    fn handle_long_press(self: Handle<Self>, app: &mut App) {
        let tooltip_created = self.controller(app).is_dismissed(app);
        if tooltip_created && self.widget(app).enable_feedback {
            debug_assert!(self.widget(app).trigger_mode == TooltipTriggerMode::LongPress);
            Feedback::for_long_press(app, self.context(app));
        }
        if let Some(on_triggered) = self.widget(app).on_triggered.clone() {
            on_triggered.call(app);
        }
        self.schedule_show_tooltip(app, Duration::ZERO, None);
    }

    fn handle_press_up(self: Handle<Self>, app: &mut App) {
        if !app.get(self).active_hovering_pointer_devices.is_empty() {
            return;
        }
        let touch_delay = self.widget(app).touch_delay;
        self.schedule_dismiss_tooltip(app, touch_delay);
    }

    // # Current Hovering Behavior:
    // 1. Hovered tooltips don't show more than one at a time, for each mouse
    //    device. For example, a chip with a delete icon typically shouldn't show
    //    both the delete icon tooltip and the chip tooltip at the same time.
    // 2. Hovered tooltips are dismissed when:
    //    i. [dismiss_all_tool_tips] is called, even these tooltips are still hovered
    //    ii. a unrecognized PointerDownEvent occurred within the application
    //    (even these tooltips are still hovered),
    //    iii. The last hovering device leaves the tooltip.
    fn handle_mouse_enter(self: Handle<Self>, app: &mut App, event: PointerEnterEvent) {
        // handle_mouse_enter is only called when the mouse starts to hover over this
        // tooltip (including the actual tooltip it shows on the overlay), and this
        // tooltip is the first to be hit in the widget tree's hit testing order.
        // See also ExclusiveMouseRegion for the exact behavior.
        app.get_mut(self)
            .active_hovering_pointer_devices
            .insert(event.device);
        // Dismiss other open tooltips unless they're kept visible by other mice.
        // The mouse tracker implementation always dispatches all `onExit` events
        // before dispatching any `onEnter` events, so `event.device` must have
        // already been removed from active_hovering_pointer_devices of the tooltips
        // that are no longer being hovered over.
        let tooltips_to_dismiss: Vec<Handle<RawTooltipState>> = OpenedTooltips::to_list(app)
            .into_iter()
            .filter(|tooltip| app.get(*tooltip).active_hovering_pointer_devices.is_empty())
            .collect();
        for tooltip in &tooltips_to_dismiss {
            debug_assert!(tooltip.mounted(app));
            tooltip.schedule_dismiss_tooltip(app, Duration::ZERO);
        }
        let with_delay = if tooltips_to_dismiss.is_empty() {
            self.widget(app).hover_delay
        } else {
            Duration::ZERO
        };
        self.schedule_show_tooltip(app, with_delay, None);
    }

    fn handle_mouse_exit(self: Handle<Self>, app: &mut App, event: PointerExitEvent) {
        if app.get(self).active_hovering_pointer_devices.is_empty() {
            return;
        }
        app.get_mut(self)
            .active_hovering_pointer_devices
            .remove(&event.device);
        if app.get(self).active_hovering_pointer_devices.is_empty() {
            let dismiss_delay = self.widget(app).dismiss_delay;
            self.schedule_dismiss_tooltip(app, dismiss_delay);
        }
    }

    /// Shows the tooltip if it is not already visible.
    ///
    /// After made visible by this method, the tooltip does not automatically
    /// dismiss after [`RawTooltip::hover_delay`] until the user dismisses/re-triggers it, or
    /// [`RawTooltip::dismiss_all_tool_tips`] is called.
    ///
    /// Returns `false` when the tooltip shouldn't be shown or when the tooltip
    /// was already visible.
    pub fn ensure_tooltip_visible(self: Handle<Self>, app: &mut App) -> bool {
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        if self.controller(app).is_forward_or_completed(app) {
            return false;
        }
        self.schedule_show_tooltip(app, Duration::ZERO, None);
        true
    }

    fn build_tooltip_overlay(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        layout_info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        if layout_info.child_paint_transform().determinant() == 0.0 {
            // The child is not visible.
            return SizedBox::shrink().into_widget();
        }
        let target = transform_point(
            &layout_info.child_paint_transform(),
            layout_info.child_size().center(Offset::ZERO),
        );

        // Keep the tooltip visible while the overlay child is hovered.
        let tooltip_builder = Rc::clone(&self.widget(app).tooltip_builder);
        self.overlay_animation(app);
        let overlay_animation: AnimationRef<f64> = app
            .get(self)
            .backing_overlay_animation
            .as_ref()
            .expect("made above")
            .erase();
        let tooltip = IgnorePointer::new()
            .ignoring(self.widget(app).ignore_pointer)
            .child(ExclusiveMouseRegion::new(
                Rc::new(move |app, event| self.handle_mouse_enter(app, event)),
                Rc::new(move |app, event| self.handle_mouse_exit(app, event)),
                tooltip_builder(app, context, overlay_animation),
            ));

        let bottom =
            MediaQuery::maybe_view_insets_of(app, context).map_or(0.0, |insets| insets.bottom);
        let overlay_child = Positioned::fill(
            CustomSingleChildLayout::new(Rc::new(TooltipPositionLayoutDelegate {
                target,
                target_size: layout_info.child_size(),
                position_delegate: self.widget(app).position_delegate.clone(),
            }))
            .child(tooltip),
        )
        .bottom(bottom)
        .into_widget();

        match selection_container_maybe_of(app, context) {
            None => overlay_child,
            Some(registrar) => match registrar {},
        }
    }
}

/// Dart's `SelectionContainer.maybeOf(context)`. `SelectionContainer` is not ported, so no
/// registrar exists and the overlay child is never wrapped in `SelectionContainer.disabled`.
fn selection_container_maybe_of(_app: &mut App, _context: BuildContext) -> Option<Infallible> {
    None
}

impl SingleTickerProviderStateMixin for RawTooltipState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for RawTooltipState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for RawTooltipState {
    type Widget = RawTooltip;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).overlay_controller = Some(OverlayPortalController::new(app, None));
        // Listen to global pointer events so that we can hide a tooltip immediately
        // if some other control is clicked on. Pointer events are dispatched to
        // global routes **after** other routes.
        let router = GestureBinding::instance(app).pointer_router(app);
        router.add_global_route(
            app,
            PointerRoute::handle_method(self, Self::handle_global_pointer_event),
            None,
        );
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawTooltip) {
        if self.widget(app).animation_style != old_widget.animation_style {
            if let Some(controller) = app.get(self).backing_controller.handle() {
                let show_duration = self.show_duration(app);
                let hide_duration = self.hide_duration(app);
                let controller = app.get_mut(controller);
                controller.duration = Some(show_duration);
                controller.reverse_duration = Some(hide_duration);
            }
            if let Some(overlay_animation) = app.get(self).backing_overlay_animation.handle() {
                let show_curve = self.show_curve(app);
                let hide_curve = self.hide_curve(app);
                let overlay_animation = app.get_mut(overlay_animation);
                overlay_animation.curve = show_curve;
                overlay_animation.reverse_curve = hide_curve;
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let router = GestureBinding::instance(app).pointer_router(app);
        router.remove_global_route(
            app,
            &PointerRoute::handle_method(self, Self::handle_global_pointer_event),
        );
        OpenedTooltips::remove(app, self);
        // long_press_recognizer.dispose() and tap_recognizer.dispose() may call
        // their registered on_cancel callbacks if there's a gesture in progress.
        // Remove the on_cancel callbacks to prevent the registered callbacks from
        // triggering unnecessary side effects (such as animations).
        if let Some(recognizer) = app.get_mut(self).long_press_recognizer.take() {
            recognizer.set_on_long_press_cancel(app, None);
            recognizer.dispose(app);
        }
        if let Some(recognizer) = app.get_mut(self).tap_recognizer.take() {
            recognizer.set_on_tap_cancel(app, None);
            recognizer.dispose(app);
        }
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        if let Some(controller) = app.get(self).backing_controller.handle() {
            controller.dispose(app);
        }
        if let Some(overlay_animation) = app.get(self).backing_overlay_animation.handle() {
            overlay_animation.dispose(app);
        }
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // If message is empty then no need to create a tooltip overlay to show
        // the empty black container so just return the wrapped child as is or
        // empty container if child is not specified.
        if self
            .widget(app)
            .semantics_tooltip
            .as_ref()
            .is_some_and(|semantics_tooltip| semantics_tooltip.is_empty())
        {
            return self.widget(app).child.clone();
        }
        debug_assert!(debug_check_has_overlay(app, context));
        let semantics_tooltip = self.widget(app).semantics_tooltip.clone();
        let exclude_from_semantics = semantics_tooltip
            .as_ref()
            .is_none_or(|semantics_tooltip| semantics_tooltip.is_empty());
        let mut semantics = Semantics::new();
        if !exclude_from_semantics && let Some(semantics_tooltip) = semantics_tooltip {
            semantics = semantics.tooltip(semantics_tooltip);
        }
        let result = semantics.child(self.widget(app).child.clone());

        // Only check for gestures if tooltip should be visible.
        let result = ExclusiveMouseRegion::new(
            Rc::new(move |app, event| self.handle_mouse_enter(app, event)),
            Rc::new(move |app, event| self.handle_mouse_exit(app, event)),
            PointerListener::new()
                .on_pointer_down(Rc::new(move |app, event| {
                    self.handle_pointer_down(app, event)
                }))
                .behavior(HitTestBehavior::Opaque)
                .child(result),
        );

        OverlayPortal::overlay_child_layout_builder(
            app.get(self)
                .overlay_controller
                .clone()
                .expect("created in init_state"),
            move |app, context, layout_info| self.build_tooltip_overlay(app, context, layout_info),
        )
        .child(result)
        .into_widget()
    }
}

/// A delegate for computing the layout of a tooltip to be displayed above or
/// below a target specified in the global coordinate system.
///
/// Dart's private `_TooltipPositionDelegate`, renamed because Rust has one namespace for it
/// and the public [`TooltipPositionDelegate`].
struct TooltipPositionLayoutDelegate {
    /// The offset of the target the tooltip is positioned near in the global
    /// coordinate system.
    target: Offset,

    /// The size of the target widget that triggers the tooltip.
    target_size: Size,

    /// A custom position delegate function for computing where the tooltip should be positioned.
    ///
    /// If provided, this function will be called with a [`TooltipPositionContext`] instead
    /// of the default positioning logic.
    position_delegate: Option<TooltipPositionDelegate>,
}

impl Debug for TooltipPositionLayoutDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TooltipPositionLayoutDelegate")
            .field("target", &self.target)
            .field("target_size", &self.target_size)
            .field("position_delegate", &self.position_delegate.is_some())
            .finish()
    }
}

impl SingleChildLayoutDelegate for TooltipPositionLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        if let Some(position_delegate) = &self.position_delegate {
            return position_delegate(
                TooltipPositionContext::new(self.target, self.target_size, child_size, 0.0)
                    .overlay_size(size),
            );
        }
        position_dependent_box(size, child_size, self.target, true, 0.0, 10.0)
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<TooltipPositionLayoutDelegate>()
            .is_none_or(|old_delegate| {
                self.target != old_delegate.target
                    || self.target_size != old_delegate.target_size
                    || !same_position_delegate(
                        self.position_delegate.as_ref(),
                        old_delegate.position_delegate.as_ref(),
                    )
            })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `positionDelegate != oldDelegate.positionDelegate` on two function values: the same
/// closure.
fn same_position_delegate(
    a: Option<&TooltipPositionDelegate>,
    b: Option<&TooltipPositionDelegate>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use waft_foundation::Ref;

    use waft_embedder::{
        Color, KeyData, KeyEventDeviceType, KeyEventType, Rect, TargetPlatform, TextDirection,
    };
    use waft_foundation::AppCell;
    use waft_gestures::{
        K_LONG_PRESS_TIMEOUT, K_PRESS_TIMEOUT, K_PRIMARY_BUTTON, PointerAddedEvent,
        PointerHoverEvent, PointerMoveEvent, PointerRemovedEvent, PointerUpEvent,
    };
    use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry};
    use waft_rendering::RenderParagraph;
    use waft_services::{KeyEventManager, LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_test::TestPlatform;

    use super::*;
    use crate::framework::GlobalKey;
    use crate::test_harness::{binding_mount, platform_view};
    use crate::widgets::app::WidgetsApp;
    use crate::widgets::basic::{
        Align, Builder, Center, ColoredBox, Column, Directionality, Padding, Spacer, Stack,
        WidgetBuilder,
    };
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::navigator::{RouteSettings, RouteSettingsRef};
    use crate::widgets::overlay::{Overlay, OverlayEntry};
    use crate::widgets::pages::{PageRouteBuilder, PageRouteRef};
    use crate::widgets::routes::RoutePageBuilder;
    use crate::widgets::text::Text;
    use crate::widgets::transitions::FadeTransition;

    const MOUSE_DEVICE: i64 = 1;
    const MOUSE_POINTER: i64 = 0;
    const ETERNITY: Duration = Duration::from_secs(9999 * 24 * 60 * 60);
    const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

    /// Flutter's `WidgetTester` in miniature, on an 800x600 view as Flutter's tests have: a
    /// logical clock that timers and frames share, pointer gestures, and the frame pumps.
    struct Tester {
        cell: Rc<AppCell>,
        now: Duration,
        next_pointer: i64,
        mouse_position: Offset,
        /// Whether the mouse's primary button is held: Dart's mouse `TestGesture` after `down`.
        mouse_is_down: bool,
        /// What the overlay's one entry builds: the tree `pump_widget` was last given.
        home: Rc<RefCell<Rc<dyn Fn() -> WidgetRef>>>,
        /// The entry `pump_widget` mounted, until `pump_root` replaces the tree.
        entry: Option<Ref<OverlayEntry>>,
    }

    impl Tester {
        fn new() -> Tester {
            Tester::with_platform(
                TestPlatform::new()
                    .on(TargetPlatform::MacOS)
                    .with_view(Rc::new(waft_test::TestView::new(800.0, 600.0))),
            )
        }

        fn with_platform(platform: TestPlatform) -> Tester {
            Tester {
                cell: AppCell::with_platform(Rc::new(platform)),
                now: Duration::ZERO,
                next_pointer: 1,
                mouse_position: Offset::ZERO,
                mouse_is_down: false,
                home: Rc::new(RefCell::new(Rc::new(|| SizedBox::shrink().into_widget()))),
                entry: None,
            }
        }

        /// Dart's `tester.pumpWidget(WidgetsApp(builder: (_, _) => Overlay(initialEntries:
        /// [entry])))`, with `child` as the one entry's widget: the first call mounts the tree,
        /// a later one rebuilds the entry with the new `child`, as `pumpWidget` updates the
        /// tree in place.
        fn pump_widget(&mut self, child: impl Fn() -> WidgetRef + 'static) {
            *self.home.borrow_mut() = Rc::new(child);
            if let Some(entry) = &self.entry {
                entry.mark_needs_build(&mut self.cell.borrow_mut());
                self.pump(Duration::ZERO);
                return;
            }
            let home = Rc::clone(&self.home);
            let entry = OverlayEntry::new(
                &mut self.cell.borrow_mut(),
                Rc::new(move |_app, _context| {
                    let build = Rc::clone(&home.borrow());
                    build()
                }),
                false,
                false,
                false,
            );
            self.pump_root(
                Directionality::new(
                    TextDirection::Ltr,
                    Overlay::new().initial_entries([entry.clone()]),
                )
                .into_widget(),
            );
            self.entry = Some(entry);
        }

        /// Dart's `tester.pumpWidget(widget)` with a tree of another shape: `widget` replaces
        /// the whole tree under the view.
        fn pump_root(&mut self, widget: WidgetRef) {
            self.entry = None;
            {
                let mut app = self.cell.borrow_mut();
                let view = platform_view(&app);
                crate::binding::run_widget(&mut app, crate::View::new(view, widget).into_widget());
            }
            self.pump(Duration::ZERO);
        }

        /// The OS fonts a `Text` needs for a size.
        fn install_fonts(&self) {
            let mut app = self.cell.borrow_mut();
            let binding = waft_painting::PaintingBinding::instance(&mut app);
            if !binding.has_fonts(&app) {
                binding.install_fonts(&mut app, |fonts| {
                    fonts.add_source(valo_system_fonts::SystemFonts::load());
                });
            }
        }

        /// Dart's `WidgetsApp(color: Color(0x00000000), pageRouteBuilder: .., home: home)`, with
        /// the OS fonts its `Text`s need.
        fn pump_app(&mut self, home: WidgetRef) {
            self.install_fonts();
            binding_mount(
                &self.cell,
                WidgetsApp::new(Color::new(0x00000000))
                    .page_route_builder(page_route)
                    .home(home)
                    .into_widget(),
            );
        }

        /// Dart's `tester.sendKeyEvent(key)`: the key down, then up.
        fn send_key_event(&mut self, physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) {
            for event_type in [KeyEventType::Down, KeyEventType::Up] {
                let mut app = self.cell.borrow_mut();
                KeyEventManager::instance(&mut app).handle_key_data(
                    &mut app,
                    KeyData {
                        time_stamp: Duration::ZERO,
                        event_type,
                        device_type: KeyEventDeviceType::Keyboard,
                        physical: physical.usb_hid_usage,
                        logical: logical.key_id,
                        character: None,
                        synthesized: false,
                    },
                );
                app.drain_microtasks();
            }
        }

        /// Dart's `tester.pump(duration)`: timers due by then fire, then a frame.
        fn pump(&mut self, duration: Duration) {
            self.cell.elapse(duration);
            self.now += duration;
            let mut app = self.cell.borrow_mut();
            let view = platform_view(&app).id();
            crate::test_harness::binding_pump_view(&mut app, view, self.now);
        }

        /// Dart's `tester.pumpAndSettle()`: 100ms frames until none is asked for.
        fn pump_and_settle(&mut self) {
            for _ in 0..1000 {
                self.pump(Duration::from_millis(100));
                let requested = self
                    .cell
                    .borrow()
                    .platform()
                    .downcast_ref::<TestPlatform>()
                    .expect("a TestPlatform")
                    .take_frame_requests();
                if requested.is_empty() {
                    return;
                }
            }
            panic!("pump_and_settle timed out");
        }

        fn dispatch(&mut self, event: PointerEvent) {
            let mut app = self.cell.borrow_mut();
            GestureBinding::instance(&mut app).handle_pointer_event(&mut app, event);
            app.drain_microtasks();
        }

        /// Dart's `tester.startGesture(position)`: a touch pointer down; returns the pointer.
        fn start_gesture(&mut self, position: Offset) -> i64 {
            let pointer = self.next_pointer;
            self.next_pointer += 1;
            self.dispatch(PointerEvent::Down(PointerDownEvent {
                pointer,
                position,
                kind: PointerDeviceKind::Touch,
                buttons: K_PRIMARY_BUTTON,
                ..PointerDownEvent::default()
            }));
            pointer
        }

        /// Dart's `gesture.up()`.
        fn up(&mut self, pointer: i64, position: Offset) {
            self.dispatch(PointerEvent::Up(PointerUpEvent {
                pointer,
                position,
                kind: PointerDeviceKind::Touch,
                ..PointerUpEvent::default()
            }));
        }

        /// Dart's `tester.tap(finder)`, at the center of what it found.
        fn tap(&mut self, position: Offset) {
            let pointer = self.start_gesture(position);
            self.up(pointer, position);
        }

        /// Dart's `tester.longPress(finder)`: down, `kLongPressTimeout` and a bit, up.
        fn long_press(&mut self, position: Offset) {
            let pointer = self.start_gesture(position);
            self.pump(K_LONG_PRESS_TIMEOUT + Duration::from_millis(100));
            self.up(pointer, position);
        }

        /// Dart's `_testGestureTap`.
        fn test_gesture_tap(&mut self, position: Offset) {
            self.tap(position);
            self.pump(Duration::from_millis(10));
        }

        /// Dart's `_testGestureLongPress`.
        fn test_gesture_long_press(&mut self, position: Offset) {
            let pointer = self.start_gesture(position);
            self.pump(Duration::ZERO);
            self.pump(K_LONG_PRESS_TIMEOUT);
            self.up(pointer, position);
            self.pump(Duration::ZERO);
        }

        /// Dart's `mouseGesture.moveTo(position)`: a hover, or a move while the button is held.
        fn mouse_move_to(&mut self, position: Offset) {
            let delta = position - self.mouse_position;
            self.mouse_position = position;
            if self.mouse_is_down {
                self.dispatch(PointerEvent::Move(PointerMoveEvent {
                    pointer: MOUSE_POINTER,
                    device: MOUSE_DEVICE,
                    position,
                    delta,
                    kind: PointerDeviceKind::Mouse,
                    buttons: K_PRIMARY_BUTTON,
                    ..PointerMoveEvent::default()
                }));
            } else {
                self.dispatch(PointerEvent::Hover(PointerHoverEvent {
                    pointer: MOUSE_POINTER,
                    device: MOUSE_DEVICE,
                    position,
                    delta,
                    kind: PointerDeviceKind::Mouse,
                    ..PointerHoverEvent::default()
                }));
            }
        }

        /// Dart's `mouseGesture.addPointer(location: position)`.
        fn mouse_add_pointer(&mut self, position: Offset) {
            self.mouse_position = position;
            self.dispatch(PointerEvent::Added(PointerAddedEvent {
                pointer: MOUSE_POINTER,
                device: MOUSE_DEVICE,
                position,
                kind: PointerDeviceKind::Mouse,
                ..PointerAddedEvent::default()
            }));
        }

        /// Dart's `mouseGesture.removePointer()`.
        fn mouse_remove_pointer(&mut self) {
            let position = self.mouse_position;
            self.dispatch(PointerEvent::Removed(PointerRemovedEvent {
                pointer: MOUSE_POINTER,
                device: MOUSE_DEVICE,
                position,
                kind: PointerDeviceKind::Mouse,
                ..PointerRemovedEvent::default()
            }));
        }

        /// Dart's `tester.startGesture(position, kind: PointerDeviceKind.mouse)`: the primary
        /// button down.
        fn mouse_down(&mut self, position: Offset) {
            self.mouse_position = position;
            self.mouse_is_down = true;
            self.dispatch(PointerEvent::Down(PointerDownEvent {
                pointer: MOUSE_POINTER,
                device: MOUSE_DEVICE,
                position,
                kind: PointerDeviceKind::Mouse,
                buttons: K_PRIMARY_BUTTON,
                ..PointerDownEvent::default()
            }));
        }

        /// Dart's `mouseGesture.up()`.
        fn mouse_up(&mut self) {
            let position = self.mouse_position;
            self.mouse_is_down = false;
            self.dispatch(PointerEvent::Up(PointerUpEvent {
                pointer: MOUSE_POINTER,
                device: MOUSE_DEVICE,
                position,
                kind: PointerDeviceKind::Mouse,
                ..PointerUpEvent::default()
            }));
        }

        /// Dart's `find.byKey(key)` finding one widget.
        fn finds(&self, key: &GlobalKey) -> bool {
            key.current_context(&self.cell.borrow_mut()).is_some()
        }

        /// The global rectangle of the box `key` is on.
        fn rect_of(&self, key: &GlobalKey) -> Rect {
            let app = self.cell.borrow();
            let render_box = key
                .current_context(&app)
                .expect("mounted")
                .find_render_object(&app)
                .and_then(|render_object| render_object.as_box())
                .expect("a box");
            let top_left = render_box.local_to_global(&app, Offset::ZERO, None);
            top_left & render_box.size(&app)
        }

        fn state(&self, key: &GlobalKey) -> Handle<RawTooltipState> {
            key.current_state::<RawTooltipState>(&self.cell.borrow_mut())
                .expect("the tooltip is mounted")
        }
    }

    /// The tests' `pageRouteBuilder`: a `PageRouteBuilder` whose page is the builder's.
    fn page_route(app: &mut App, settings: &RouteSettings, builder: WidgetBuilder) -> PageRouteRef {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        let route = PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Settings(settings.clone()));
        route.erase()
    }

    fn sized_box(width: f64, height: f64) -> SizedBox {
        SizedBox::new().width(width).height(height)
    }

    /// Dart's `Text(tooltipText)` as a tooltip: a box of Ahem's 14 pixel line height, keyed so
    /// a test can find it.
    fn tip(key: &GlobalKey) -> WidgetRef {
        sized_box(42.0, 14.0).key(key.clone().into()).into_widget()
    }

    /// The `positionDelegate` of `setWidgetForTooltipMode`.
    fn below_with_vertical_offset_24() -> TooltipPositionDelegate {
        Rc::new(|context: TooltipPositionContext| {
            position_dependent_box(
                context.overlay_size,
                context.tooltip_size,
                context.target,
                context.prefer_below,
                24.0,
                10.0,
            )
        })
    }

    fn prefer_above() -> TooltipPositionDelegate {
        Rc::new(|context: TooltipPositionContext| {
            position_dependent_box(
                context.overlay_size,
                context.tooltip_size,
                context.target,
                false,
                0.0,
                10.0,
            )
        })
    }

    const CENTER: Offset = Offset::new(400.0, 300.0);

    /// Dart's `setWidgetForTooltipMode`.
    fn set_widget_for_tooltip_mode(
        tester: &mut Tester,
        trigger_mode: TooltipTriggerMode,
        touch_delay: Option<Duration>,
        enable_tap_to_dismiss: Option<bool>,
        on_triggered: Option<Listener>,
    ) -> GlobalKey {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            let mut tooltip = RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(&key),
                sized_box(100.0, 100.0),
            )
            .trigger_mode(trigger_mode)
            .touch_delay(touch_delay.unwrap_or(Duration::from_millis(1500)))
            .enable_tap_to_dismiss(enable_tap_to_dismiss.unwrap_or(true))
            .position_delegate(below_with_vertical_offset_24());
            if let Some(on_triggered) = on_triggered.clone() {
                tooltip = tooltip.on_triggered(on_triggered);
            }
            Center::new().child(tooltip).into_widget()
        });
        tip_key
    }

    /// The tree of Dart's positioning cases: a tooltip at `(left, top)` of a `Stack`, shown
    /// by `ensureTooltipVisible` and given two seconds to fade in.
    fn position_case(
        left: f64,
        top: f64,
        tip: impl Fn() -> WidgetRef + 'static,
        position_delegate: Option<TooltipPositionDelegate>,
    ) -> Tester {
        let mut tester = Tester::new();
        let tooltip_key = GlobalKey::new();
        let key = tooltip_key.clone();
        let tip = Rc::new(tip);
        tester.pump_widget(move || {
            let tip = Rc::clone(&tip);
            let mut tooltip = RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(),
                SizedBox::shrink(),
            )
            .key(key.clone().into());
            if let Some(position_delegate) = position_delegate.clone() {
                tooltip = tooltip.position_delegate(position_delegate);
            }
            Stack::new()
                .children([Positioned::new(tooltip).left(left).top(top).into_widget()])
                .into_widget()
        });
        let state = tester.state(&tooltip_key);
        state.ensure_tooltip_visible(&mut tester.cell.borrow_mut());
        tester.pump(Duration::from_secs(2)); // faded in, show timer started (and at 0.0)
        tester
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(300.0, 0.0, move || tip(&key), None);
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.top_center().dx(), 300.0);
        assert_eq!(tip.top_center().dy(), 0.0);
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_top_left() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(
            0.0,
            0.0,
            move || {
                Padding::new(EdgeInsetsGeometry::symmetric(5.0, 0.0))
                    .key(key.clone().into())
                    .child(sized_box(42.0, 20.0))
                    .into_widget()
            },
            None,
        );
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 30.0); // 20.0 height + 5.0 padding * 2 (top, bottom)
        assert_eq!(tip.top_left(), Offset::new(10.0, 0.0));
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_above_fits() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(
            400.0,
            300.0,
            move || sized_box(42.0, 100.0).key(key.clone().into()).into_widget(),
            Some(prefer_above()),
        );
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 100.0);
        assert_eq!(tip.top, 200.0);
        assert_eq!(tip.bottom, 300.0);
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_above_does_not_fit() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(
            400.0,
            299.0,
            move || sized_box(42.0, 190.0).key(key.clone().into()).into_widget(),
            Some(prefer_above()),
        );
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 190.0);
        assert_eq!(tip.top, 109.0);
        assert_eq!(tip.bottom, 299.0);
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_below_fits() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(
            400.0,
            300.0,
            move || sized_box(42.0, 190.0).key(key.clone().into()).into_widget(),
            None,
        );
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 190.0);
        assert_eq!(tip.top, 300.0);
        assert_eq!(tip.bottom, 490.0);
    }

    fn padded_ten_high(key: &GlobalKey) -> WidgetRef {
        Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 10.0))
            .key(key.clone().into())
            .child(sized_box(42.0, 10.0))
            .into_widget()
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_way_off_to_the_right() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(1600.0, 300.0, move || padded_ten_high(&key), None);
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 20.0);
        assert_eq!(tip.top, 300.0);
        assert_eq!(tip.right, 790.0);
        assert_eq!(tip.bottom, 320.0);
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_near_the_edge() {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let tester = position_case(780.0, 300.0, move || padded_ten_high(&key), None);
        let tip = tester.rect_of(&tip_key);
        assert_eq!(tip.height(), 20.0);
        assert_eq!(tip.top, 300.0);
        assert_eq!(tip.right, 790.0);
        assert_eq!(tip.bottom, 320.0);
    }

    #[test]
    fn custom_tooltip_positioning_with_the_position_delegate() {
        let mut tester = Tester::new();
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        let target_key = GlobalKey::new();
        let target = target_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            Center::new()
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        move |_app, _context, _animation| tip(&key),
                        sized_box(50.0, 50.0).key(target.clone().into()),
                    )
                    .position_delegate(Rc::new(
                        |context: TooltipPositionContext| {
                            // Align on top right of box with bottom left of tooltip.
                            Offset::new(
                                context.target.dx() + (context.target_size.width() / 2.0),
                                context.target.dy()
                                    - (context.target_size.height() / 2.0)
                                    - context.tooltip_size.height(),
                            )
                        },
                    )),
                )
                .into_widget()
        });
        tester.long_press(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(tester.finds(&tip_key));

        let target_center = tester.rect_of(&target_key).center();
        let tooltip_position = tester.rect_of(&tip_key).top_left();
        // The tooltip should be positioned at target + (25, -25-14).
        assert!((tooltip_position.dx() - (target_center.dx() + 25.0)).abs() <= 5.0);
        assert!((tooltip_position.dy() - (target_center.dy() - 25.0 - 14.0)).abs() <= 5.0);
    }

    #[test]
    fn raw_tooltip_stays_after_long_press() {
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            None,
            None,
            None,
        );

        // long press reveals tooltip
        let pointer = tester.start_gesture(CENTER);
        tester.pump(K_LONG_PRESS_TIMEOUT);
        tester.pump(Duration::from_millis(10));
        assert!(tester.finds(&tip_key));
        tester.up(pointer, CENTER);

        // tap (down, up) gesture hides tooltip, since its not
        // a long press
        tester.tap(CENTER);
        tester.pump(Duration::ZERO);
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));

        // long press once more
        let pointer = tester.start_gesture(CENTER);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(300));
        assert!(!tester.finds(&tip_key));

        tester.pump(K_LONG_PRESS_TIMEOUT);
        tester.pump(Duration::from_millis(10));
        assert!(tester.finds(&tip_key));

        // keep holding the long press, should still show tooltip
        tester.pump(K_LONG_PRESS_TIMEOUT);
        assert!(tester.finds(&tip_key));
        tester.up(pointer, CENTER);
    }

    #[test]
    fn raw_tooltip_dismiss_countdown_begins_on_long_press_release() {
        let touch_delay = Duration::from_secs(1);
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            Some(touch_delay),
            None,
            None,
        );

        let pointer = tester.start_gesture(CENTER);
        tester.pump(K_LONG_PRESS_TIMEOUT);
        assert!(tester.finds(&tip_key));
        // Keep holding to prevent the tooltip from dismissing.
        tester.pump(ETERNITY);
        assert!(tester.finds(&tip_key));
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));

        tester.up(pointer, CENTER);
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));

        tester.pump(touch_delay);
        tester.pump(Duration::from_millis(500));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn raw_tooltip_is_dismissed_after_a_long_press_and_touch_delay_expired() {
        let touch_delay = Duration::from_secs(3);
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            Some(touch_delay),
            None,
            None,
        );

        // Long press reveals tooltip
        let pointer = tester.start_gesture(CENTER);
        tester.pump(K_LONG_PRESS_TIMEOUT);
        tester.pump(Duration::from_millis(10));
        assert!(tester.finds(&tip_key));
        tester.up(pointer, CENTER);

        // Tooltip is dismissed after touchDelay expired
        tester.pump(touch_delay);
        tester.pump(Duration::from_millis(10));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn raw_tooltip_is_dismissed_after_a_tap_and_touch_delay_expired() {
        let touch_delay = Duration::from_secs(3);
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::Tap,
            Some(touch_delay),
            None,
            None,
        );
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_tap(CENTER);
        assert!(tester.finds(&tip_key));

        // Tooltip is dismissed after touchDelay expired
        tester.pump(touch_delay);
        tester.pump(Duration::from_millis(10));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn raw_tooltip_is_dismissed_after_tap_to_dismiss_immediately() {
        let mut tester = Tester::new();
        let tip_key =
            set_widget_for_tooltip_mode(&mut tester, TooltipTriggerMode::Tap, None, None, None);
        assert!(!tester.finds(&tip_key));

        // Tap to trigger the tooltip.
        tester.test_gesture_tap(CENTER);
        assert!(tester.finds(&tip_key));

        // Tap to dismiss the tooltip. Tooltip is dismissed immediately.
        let tip = tester.rect_of(&tip_key).center();
        tester.test_gesture_tap(tip);
        tester.pump(Duration::from_millis(10));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_is_not_dismissed_after_tap_if_enable_tap_to_dismiss_is_false() {
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::Tap,
            None,
            Some(false),
            None,
        );
        assert!(!tester.finds(&tip_key));

        // Tap to trigger the tooltip.
        tester.test_gesture_tap(CENTER);
        assert!(tester.finds(&tip_key));

        // Tap the tooltip. The tooltip is not dismissed.
        let tip = tester.rect_of(&tip_key).center();
        tester.test_gesture_tap(tip);
        tester.pump(Duration::from_millis(10));
        assert!(tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_triggers_on_tap_when_trigger_mode_is_tap() {
        let mut tester = Tester::new();
        let tip_key =
            set_widget_for_tooltip_mode(&mut tester, TooltipTriggerMode::Tap, None, None, None);
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_tap(CENTER);
        assert!(tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_triggers_on_long_press_when_mode_is_long_press() {
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            None,
            None,
            None,
        );
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_tap(CENTER);
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_long_press(CENTER);
        assert!(tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_does_not_trigger_when_trigger_mode_is_manual() {
        let mut tester = Tester::new();
        let tip_key =
            set_widget_for_tooltip_mode(&mut tester, TooltipTriggerMode::Manual, None, None, None);
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_tap(CENTER);
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_long_press(CENTER);
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_on_triggered_is_called_when_tooltip_triggers() {
        let on_triggered_called = Rc::new(Cell::new(false));
        let called = Rc::clone(&on_triggered_called);
        let on_triggered = Listener::new(move |_app| called.set(true));

        let mut tester = Tester::new();
        set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            None,
            None,
            Some(on_triggered.clone()),
        );
        tester.test_gesture_long_press(CENTER);
        assert!(on_triggered_called.get());

        on_triggered_called.set(false);
        let mut tester = Tester::new();
        set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::Tap,
            None,
            None,
            Some(on_triggered),
        );
        tester.test_gesture_tap(CENTER);
        assert!(on_triggered_called.get());
    }

    /// The `Center(child: RawTooltip(child: SizedBox(100x100)))` the hover cases mount.
    fn hover_case(
        tester: &mut Tester,
        configure: impl Fn(RawTooltip) -> RawTooltip + 'static,
    ) -> GlobalKey {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            Center::new()
                .child(configure(RawTooltip::new(
                    Some("TIP".into()),
                    move |_app, _context, _animation| tip(&key),
                    sized_box(100.0, 100.0),
                )))
                .into_widget()
        });
        tip_key
    }

    #[test]
    fn tooltip_on_triggered_is_not_called_when_tooltip_is_hovered() {
        let on_triggered_called = Rc::new(Cell::new(false));
        let called = Rc::clone(&on_triggered_called);
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::ZERO);
        hover_case(&mut tester, move |tooltip| {
            let called = Rc::clone(&called);
            tooltip.on_triggered(Listener::new(move |_app| called.set(true)))
        });

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(Duration::ZERO);
        assert!(!on_triggered_called.get());
    }

    #[test]
    fn tooltip_shows_and_hides_when_hovered() {
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.mouse_move_to(Offset::ZERO);
        let tip_key = hover_case(&mut tester, |tooltip| tooltip);

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));

        // Wait a looong time to make sure that it doesn't go away if the mouse is
        // still over the widget.
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);

        // Wait for it to disappear.
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_text_is_also_hoverable() {
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.mouse_move_to(Offset::ZERO);
        let tip_key = hover_case(&mut tester, |tooltip| tooltip);

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));

        // Wait a looong time to make sure that it doesn't go away if the mouse is
        // still over the widget.
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));

        // Hover to the tooltip text and verify the tooltip doesn't go away.
        let tip = tester.rect_of(&tip_key).top_left();
        tester.mouse_move_to(tip);
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);

        // Wait for it to disappear.
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_shows_without_delay_when_the_mouse_moves_from_another_tooltip() {
        let hover_delay = Duration::from_millis(700);
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.mouse_move_to(Offset::ZERO);
        let (first_key, last_key) = (GlobalKey::new(), GlobalKey::new());
        let (first, last) = (first_key.clone(), last_key.clone());
        tester.pump_widget(move || {
            let (first, last) = (first.clone(), last.clone());
            Column::new()
                .children([
                    RawTooltip::new(
                        Some("first tooltip".into()),
                        move |_app, _context, _animation| tip(&first),
                        sized_box(100.0, 100.0),
                    )
                    .hover_delay(hover_delay)
                    .into_widget(),
                    RawTooltip::new(
                        Some("last tooltip".into()),
                        move |_app, _context, _animation| tip(&last),
                        sized_box(100.0, 100.0),
                    )
                    .hover_delay(hover_delay)
                    .into_widget(),
                ])
                .into_widget()
        });

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(Offset::new(400.0, 50.0));
        tester.pump(Duration::ZERO);
        // Wait for the first tooltip to appear.
        tester.pump(hover_delay);
        assert!(tester.finds(&first_key));
        assert!(!tester.finds(&last_key));

        // Move to the second tooltip and expect it to show up immediately.
        tester.mouse_move_to(Offset::new(400.0, 150.0));
        tester.pump(Duration::ZERO);
        assert!(!tester.finds(&first_key));
        assert!(tester.finds(&last_key));
    }

    #[test]
    fn tooltip_should_not_show_more_than_one_tooltip_when_hovered() {
        let hover_delay = Duration::from_millis(500);
        let mut tester = Tester::new();
        let (outer_tip, inner_tip) = (GlobalKey::new(), GlobalKey::new());
        let (outer, inner) = (outer_tip.clone(), inner_tip.clone());
        tester.pump_widget(move || {
            let (outer, inner) = (outer.clone(), inner.clone());
            Center::new()
                .child(RawTooltip::new(
                    Some("Outer".into()),
                    move |_app, _context, _animation| tip(&outer),
                    sized_box(100.0, 100.0).child(
                        Align::new()
                            .alignment(AlignmentGeometry::CENTER_RIGHT)
                            .child(RawTooltip::new(
                                Some("Inner".into()),
                                move |_app, _context, _animation| tip(&inner),
                                sized_box(25.0, 100.0),
                            )),
                    ),
                ))
                .into_widget()
        });
        let outer_center = CENTER;
        let inner_center = Offset::new(437.5, 300.0);

        // Both the inner and outer containers have tooltips associated with them, but only
        // the currently hovered one should appear, even though the pointer is inside both.
        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(outer_center);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(inner_center);
        tester.pump(Duration::ZERO);

        // Wait for it to appear.
        tester.pump(hover_delay);

        assert!(!tester.finds(&outer_tip));
        assert!(tester.finds(&inner_tip));
        tester.mouse_move_to(outer_center);
        tester.pump(Duration::ZERO);
        // Wait for it to switch.
        tester.pump_and_settle();
        assert!(tester.finds(&outer_tip));
        assert!(!tester.finds(&inner_tip));

        tester.mouse_move_to(Offset::ZERO);

        // Wait for all tooltips to disappear.
        tester.pump_and_settle();
        assert!(!tester.finds(&outer_tip));
        assert!(!tester.finds(&inner_tip));
    }

    #[test]
    fn dismiss_all_tool_tips_dismisses_hovered_tooltips() {
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::ZERO);
        let tip_key = hover_case(&mut tester, |tooltip| {
            tooltip.position_delegate(below_with_vertical_offset_24())
        });

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));

        assert!(RawTooltip::dismiss_all_tool_tips(
            &mut tester.cell.borrow_mut()
        ));
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_can_be_dismissed_by_escape_key() {
        let hover_delay = Duration::ZERO;
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(Offset::ZERO);

        let colored_box = GlobalKey::new();
        let key = colored_box.clone();
        tester.pump_app(
            Center::new()
                .child(RawTooltip::new(
                    Some("TIP".into()),
                    move |_app, _context, _animation| {
                        ColoredBox::new(Color::new(0xff0000ff))
                            .key(key.clone().into())
                            .into_widget()
                    },
                    Text::new("I am tool tip"),
                ))
                .into_widget(),
        );

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(hover_delay);
        assert!(tester.finds(&colored_box));

        // Try to dismiss the tooltip with the shortcut key.
        tester.send_key_event(PhysicalKeyboardKey::ESCAPE, LogicalKeyboardKey::ESCAPE);
        tester.pump_and_settle();
        assert!(!tester.finds(&colored_box));

        tester.mouse_move_to(Offset::ZERO);
        tester.pump_and_settle();
    }

    #[test]
    fn multiple_tooltips_are_dismissed_by_escape_key() {
        let hover_delay = Duration::ZERO;
        let mut tester = Tester::new();
        let (tooltip1, tooltip2) = (GlobalKey::new(), GlobalKey::new());
        let (message1, message2) = (GlobalKey::new(), GlobalKey::new());
        let message = |key: &GlobalKey, text: &'static str| {
            let key = key.clone();
            move |_app: &mut App, _context: BuildContext, _animation: AnimationRef<f64>| {
                Text::new(text).key(key.clone().into()).into_widget()
            }
        };
        tester.pump_app(
            Center::new()
                .child(
                    Column::new().children([
                        RawTooltip::new(
                            Some("tooltip1".into()),
                            message(&message1, "message1"),
                            Text::new("tooltip1"),
                        )
                        .key(tooltip1.clone().into())
                        .touch_delay(ONE_DAY)
                        .into_widget(),
                        Spacer::new().flex(2).into_widget(),
                        RawTooltip::new(
                            Some("tooltip2".into()),
                            message(&message2, "message2"),
                            Text::new("tooltip2"),
                        )
                        .key(tooltip2.clone().into())
                        .touch_delay(ONE_DAY)
                        .into_widget(),
                    ]),
                )
                .into_widget(),
        );

        let (state1, state2) = (tester.state(&tooltip1), tester.state(&tooltip2));
        state1.ensure_tooltip_visible(&mut tester.cell.borrow_mut());
        state2.ensure_tooltip_visible(&mut tester.cell.borrow_mut());
        tester.pump(Duration::ZERO);
        tester.pump(hover_delay);
        // Make sure both messages are on the screen.
        assert!(tester.finds(&message1));
        assert!(tester.finds(&message2));

        // Try to dismiss the tooltip with the shortcut key
        tester.send_key_event(PhysicalKeyboardKey::ESCAPE, LogicalKeyboardKey::ESCAPE);
        tester.pump_and_settle();
        assert!(!tester.finds(&message1));
        assert!(!tester.finds(&message2));
    }

    #[test]
    fn hovered_tooltips_do_not_dismiss_after_touch_delay() {
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::ZERO);
        let tip_key = hover_case(&mut tester, |tooltip| tooltip);

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(Duration::ZERO);
        assert!(tester.finds(&tip_key));
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));

        tester.long_press(CENTER);
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        // Still visible.
        assert!(tester.finds(&tip_key));

        // Still visible.
        tester.pump(ONE_DAY);
        tester.pump_and_settle();

        // The tooltip is no longer hovered and becomes invisible.
        tester.mouse_move_to(Offset::ZERO);
        tester.pump(ONE_DAY);
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));
    }

    /// The `Center(SizedBox.square(10, RawTooltip(child: SizedBox.expand())))` of the
    /// dismiss-delay cases.
    fn small_hover_case(tester: &mut Tester, dismiss_delay: Option<Duration>) -> GlobalKey {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            let mut tooltip = RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(&key),
                SizedBox::expand(),
            );
            if let Some(dismiss_delay) = dismiss_delay {
                tooltip = tooltip.dismiss_delay(dismiss_delay);
            }
            Center::new()
                .child(sized_box(10.0, 10.0).child(tooltip))
                .into_widget()
        });
        tip_key
    }

    #[test]
    fn hovered_tooltips_hide_after_stopping_the_hover() {
        let mut tester = Tester::new();
        let tip_key = small_hover_case(&mut tester, None);

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::from_millis(100));
        tester.pump_and_settle();
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should be hidden when no longer hovered."
        );
    }

    #[test]
    fn hovered_tooltips_hide_after_stopping_the_hover_and_dismiss_delay_expires() {
        let dismiss_delay = Duration::from_secs(1);
        let mut tester = Tester::new();
        let tip_key = small_hover_case(&mut tester, Some(dismiss_delay));

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::from_millis(100));
        tester.pump_and_settle();
        assert!(
            tester.finds(&tip_key),
            "Tooltip should wait until dismissDelay expires before being hidden"
        );

        tester.pump(Duration::from_secs(1));
        tester.pump_and_settle();
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should be hidden when no longer hovered."
        );
    }

    #[test]
    fn raw_tooltip_ignores_pointer_events_when_ignore_pointer_is_true() {
        let mut tester = Tester::new();
        tester.mouse_move_to(Offset::ZERO);
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            // Dart's `Text('Hover me')`: eight Ahem glyphs of 14 pixels.
            Center::new()
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        move |_app, _context, _animation| tip(&key),
                        sized_box(112.0, 14.0),
                    )
                    .ignore_pointer(true),
                )
                .into_widget()
        });

        // Hover over the target widget.
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);

        // Wait for the tooltip to actually appear.
        tester.pump(Duration::from_millis(100));
        assert!(tester.finds(&tip_key));

        // Move the mouse to the tooltip overlay.
        let tip = tester.rect_of(&tip_key).center();
        tester.mouse_move_to(tip);
        tester.pump_and_settle();

        // Verify the tooltip overlay is no longer displayed.
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn a_tooltip_with_an_empty_semantics_tooltip_builds_only_its_child() {
        let mut tester = Tester::new();
        let (tip_key, child_key) = (GlobalKey::new(), GlobalKey::new());
        let (key, child) = (tip_key.clone(), child_key.clone());
        tester.pump_widget(move || {
            let key = key.clone();
            RawTooltip::new(
                Some(String::new()),
                move |_app, _context, _animation| tip(&key),
                SizedBox::shrink().key(child.clone().into()),
            )
            .into_widget()
        });
        assert!(tester.finds(&child_key));
        let state = child_key
            .current_context(&tester.cell.borrow_mut())
            .expect("mounted");
        let app = tester.cell.borrow();
        assert!(
            state
                .find_ancestor_state_of_type::<RawTooltipState>(&app)
                .is_some(),
            "the tooltip is still in the tree"
        );
        drop(app);
        tester.long_press(Offset::ZERO);
        tester.pump(Duration::from_secs(1));
        assert!(!tester.finds(&tip_key));
    }

    /// The text direction of the paragraph `key` is on.
    fn text_direction_of(tester: &Tester, key: &GlobalKey) -> TextDirection {
        let app = tester.cell.borrow();
        key.current_context(&app)
            .expect("mounted")
            .find_render_object(&app)
            .and_then(|render_object| render_object.downcast::<RenderParagraph>(&app))
            .expect("a paragraph")
            .text_direction(&app)
    }

    #[test]
    fn raw_tooltip_overlay_respects_ambient_directionality() {
        // Regression test for https://github.com/flutter/flutter/issues/40702.
        let mut tester = Tester::new();
        tester.install_fonts();
        let (tooltip_key, text_key) = (GlobalKey::new(), GlobalKey::new());
        let build_app = |text_direction: TextDirection| {
            let (tooltip_key, text_key) = (tooltip_key.clone(), text_key.clone());
            move || {
                let text_key = text_key.clone();
                Directionality::new(
                    text_direction,
                    Center::new().child(
                        RawTooltip::new(
                            Some("TIP".into()),
                            move |_app, _context, _animation| {
                                Text::new("TIP").key(text_key.clone().into()).into_widget()
                            },
                            sized_box(100.0, 100.0).child(ColoredBox::new(Color::new(0xff00ff00))),
                        )
                        .key(tooltip_key.clone().into()),
                    ),
                )
                .into_widget()
            }
        };

        tester.pump_widget(build_app(TextDirection::Rtl));
        let center = tester.rect_of(&tooltip_key).center();
        tester.long_press(center);
        assert!(tester.finds(&text_key));
        assert_eq!(text_direction_of(&tester, &text_key), TextDirection::Rtl);

        tester.pump(Duration::from_secs(10));
        tester.pump_and_settle();
        tester.pump(Duration::ZERO);

        tester.pump_widget(build_app(TextDirection::Ltr));
        tester.long_press(center);
        assert!(tester.finds(&text_key));
        assert_eq!(text_direction_of(&tester, &text_key), TextDirection::Ltr);
    }

    #[test]
    fn tooltip_is_dismissed_after_a_tap_and_touch_delay_expired_when_competing_with_a_gesture_detector()
     {
        // Regression test for https://github.com/flutter/flutter/issues/98854
        let touch_delay = Duration::from_secs(3);
        let mut tester = Tester::new();
        let (tooltip_key, tip_key) = (GlobalKey::new(), GlobalKey::new());
        let (tooltip, key) = (tooltip_key.clone(), tip_key.clone());
        tester.pump_widget(move || {
            let key = key.clone();
            GestureDetector::new()
                .on_vertical_drag_start(Rc::new(|_app, _details| {
                    // Do nothing
                }))
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        move |_app, _context, _animation| tip(&key),
                        sized_box(100.0, 100.0),
                    )
                    .key(tooltip.clone().into())
                    .trigger_mode(TooltipTriggerMode::Tap)
                    .touch_delay(touch_delay),
                )
                .into_widget()
        });
        assert!(!tester.finds(&tip_key));

        let center = tester.rect_of(&tooltip_key).center();
        tester.tap(center);
        // Wait for GestureArena disambiguation, delay is kPressTimeout to disambiguate
        // between onTap and onVerticalDragStart
        tester.pump(K_PRESS_TIMEOUT);
        assert!(tester.finds(&tip_key));

        // Tooltip is dismissed after touchDelay expired
        tester.pump(touch_delay);
        tester.pump(Duration::from_millis(10));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn dispatch_the_mouse_events_before_tip_overlay_detached() {
        // Regression test for https://github.com/flutter/flutter/issues/96890
        let hover_delay = Duration::ZERO;
        let mut tester = Tester::new();
        tester.mouse_add_pointer(Offset::ZERO);
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(Offset::ZERO);

        let tip_key = hover_case(&mut tester, |tooltip| tooltip);

        // Trigger the tip overlay.
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);
        // Wait for it to appear.
        tester.pump(hover_delay);
        assert!(tester.finds(&tip_key));

        // Remove the `Tooltip` widget.
        tester.pump_root(Center::new().child(SizedBox::shrink()).into_widget());

        // The tooltip should be removed, including the overlay child.
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn calling_ensure_tooltip_visible_on_an_unmounted_raw_tooltip_state_returns_false() {
        // Regression test for https://github.com/flutter/flutter/issues/95851
        let mut tester = Tester::new();
        let tooltip_key = GlobalKey::new();
        let key = tooltip_key.clone();
        tester.pump_widget(move || {
            Center::new()
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        |_app, _context, _animation| sized_box(42.0, 14.0).into_widget(),
                        sized_box(100.0, 100.0),
                    )
                    .key(key.clone().into()),
                )
                .into_widget()
        });

        let tooltip_state = tester.state(&tooltip_key);
        assert!(tooltip_state.ensure_tooltip_visible(&mut tester.cell.borrow_mut()));

        // Remove the tooltip.
        tester.pump_root(Center::new().child(SizedBox::shrink()).into_widget());

        // The recorded divergence: Dart's `?? false` through the key, as Material's `TooltipState`
        // reaches it; the stale handle itself is gone.
        let state = tooltip_key.current_state::<RawTooltipState>(&tester.cell.borrow_mut());
        let visible =
            state.is_some_and(|state| state.ensure_tooltip_visible(&mut tester.cell.borrow_mut()));
        assert!(!visible);
        assert!(!tester.cell.borrow().contains(tooltip_state));
    }

    #[test]
    fn tooltip_shows_and_hides_when_the_mouse_hovers_and_then_exits_and_re_enters_in_quick_succession()
     {
        let hover_delay = Duration::from_millis(700);
        let mut tester = Tester::new();
        tester.mouse_add_pointer(Offset::ZERO);
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        let tip_key = hover_case(&mut tester, move |tooltip| {
            tooltip.hover_delay(hover_delay).dismiss_delay(hover_delay)
        });

        let mouse_enter_and_wait_until_visible = |tester: &mut Tester| {
            tester.mouse_move_to(CENTER);
            tester.pump(Duration::ZERO);
            tester.pump(hover_delay);
            tester.pump_and_settle();
            assert!(tester.finds(&tip_key));
        };
        let mouse_exit = |tester: &mut Tester| {
            tester.mouse_move_to(Offset::ZERO);
            tester.pump(Duration::ZERO);
        };
        let wait_for_reset = |tester: &mut Tester| {
            mouse_exit(tester);
            tester.pump(Duration::from_secs(60 * 60));
            tester.pump_and_settle();
            assert!(!tester.finds(&tip_key));
        };

        mouse_enter_and_wait_until_visible(&mut tester);
        assert!(tester.finds(&tip_key));

        wait_for_reset(&mut tester);

        mouse_enter_and_wait_until_visible(&mut tester);
        mouse_exit(&mut tester);
        mouse_enter_and_wait_until_visible(&mut tester);
        assert!(tester.finds(&tip_key));

        wait_for_reset(&mut tester);

        mouse_enter_and_wait_until_visible(&mut tester);
        mouse_exit(&mut tester);
        mouse_enter_and_wait_until_visible(&mut tester);
        mouse_exit(&mut tester);
        mouse_enter_and_wait_until_visible(&mut tester);
        assert!(tester.finds(&tip_key));

        wait_for_reset(&mut tester);
    }

    #[test]
    fn tooltip_does_not_attempt_to_show_after_unmount() {
        // Regression test for https://github.com/flutter/flutter/issues/54096.
        let hover_delay = Duration::from_secs(1);
        let mut tester = Tester::new();
        tester.mouse_add_pointer(Offset::ZERO);
        tester.mouse_move_to(Offset::new(1.0, 1.0));
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(Offset::ZERO);

        hover_case(&mut tester, move |tooltip| tooltip.hover_delay(hover_delay));

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::ZERO);

        // Pump another random widget to unmount the Tooltip widget.
        tester.pump_root(Center::new().child(SizedBox::new()).into_widget());

        // If the issue regresses, a panic will be raised while we are waiting.
        tester.pump(hover_delay);
    }

    /// Dart's `FeedbackTester.hapticCount`: the `HapticFeedback.vibrate` messages, which carry
    /// every feedback kind.
    #[derive(Default)]
    struct CountingHaptics {
        haptic_count: Cell<usize>,
    }

    impl waft_embedder::Haptics for CountingHaptics {
        fn feedback(&self, _kind: waft_embedder::HapticFeedbackType) {
            self.haptic_count.set(self.haptic_count.get() + 1);
        }
    }

    #[test]
    fn haptic_feedback() {
        let feedback = Rc::new(CountingHaptics::default());
        // Flutter's tests run on Android, where a long press vibrates.
        let mut tester = Tester::with_platform(
            TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_view(Rc::new(waft_test::TestView::new(800.0, 600.0)))
                .with_haptics(feedback.clone()),
        );
        hover_case(&mut tester, |tooltip| tooltip);

        tester.long_press(CENTER);
        tester.pump(Duration::from_secs(1));
        tester.pump_and_settle();
        assert_eq!(feedback.haptic_count.get(), 1);
    }

    #[test]
    fn tooltip_does_not_trigger_on_tap_when_trigger_mode_is_long_press() {
        let mut tester = Tester::new();
        let tip_key = set_widget_for_tooltip_mode(
            &mut tester,
            TooltipTriggerMode::LongPress,
            None,
            None,
            None,
        );
        assert!(!tester.finds(&tip_key));

        tester.test_gesture_tap(CENTER);
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn hovered_tooltips_with_touch_delay_set_do_dismiss_when_hovering_elsewhere() {
        let touch_delay = Duration::from_secs(1);
        let mut tester = Tester::new();
        let tip_key = hover_case(&mut tester, move |tooltip| tooltip.touch_delay(touch_delay));

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        tester.mouse_move_to(Offset::ZERO);
        // Set a duration equal to the default exit
        tester.pump(Duration::from_millis(100));
        tester.pump_and_settle();
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should not wait for touchDelay before it hides itself."
        );
    }

    #[test]
    fn tooltip_should_not_be_shown_with_empty_message_with_child() {
        let mut tester = Tester::new();
        let (tip_key, child_key) = (GlobalKey::new(), GlobalKey::new());
        let (key, child) = (tip_key.clone(), child_key.clone());
        tester.pump_widget(move || {
            let key = key.clone();
            RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(&key),
                tip(&child),
            )
            .into_widget()
        });
        assert!(tester.finds(&child_key));
        assert!(!tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_should_not_be_shown_with_empty_message_without_child() {
        let mut tester = Tester::new();
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(&key),
                SizedBox::shrink(),
            )
            .into_widget()
        });
        assert!(!tester.finds(&tip_key));
    }

    /// `home: RawTooltip(child: SizedBox.expand())`, filling the view.
    fn expanded_case(tester: &mut Tester) -> GlobalKey {
        let tip_key = GlobalKey::new();
        let key = tip_key.clone();
        tester.pump_widget(move || {
            let key = key.clone();
            RawTooltip::new(
                Some("TIP".into()),
                move |_app, _context, _animation| tip(&key),
                SizedBox::expand(),
            )
            .into_widget()
        });
        tip_key
    }

    #[test]
    fn tooltip_trigger_mode_ignores_mouse_events() {
        let mut tester = Tester::new();
        let tip_key = expanded_case(&mut tester);

        tester.mouse_down(CENTER);
        tester.pump(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
        tester.mouse_up();

        tester.pump(Duration::ZERO);
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));

        let touch_gesture = tester.start_gesture(CENTER);
        tester.pump(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
        tester.up(touch_gesture, CENTER);

        tester.pump(Duration::ZERO);
        tester.pump_and_settle();
        assert!(tester.finds(&tip_key));
    }

    #[test]
    fn tooltip_does_not_block_other_mouse_regions() {
        let entered = Rc::new(Cell::new(false));
        let mut tester = Tester::new();
        let sink = Rc::clone(&entered);
        tester.pump_widget(move || {
            let sink = Rc::clone(&sink);
            MouseRegion::new()
                .on_enter(Rc::new(move |_app, _event| sink.set(true)))
                .child(RawTooltip::new(
                    Some("TIP".into()),
                    |_app, _context, _animation| sized_box(42.0, 14.0).into_widget(),
                    SizedBox::expand(),
                ))
                .into_widget()
        });

        assert!(!entered.get());

        tester.mouse_add_pointer(Offset::ZERO);
        tester.mouse_move_to(CENTER);
        tester.mouse_remove_pointer();

        assert!(entered.get());
    }

    #[test]
    fn does_not_rebuild_on_mouse_connect_and_disconnect() {
        // Regression test for https://github.com/flutter/flutter/issues/117627
        let build_count = Rc::new(Cell::new(0));
        let mut tester = Tester::new();
        let count = Rc::clone(&build_count);
        tester.pump_widget(move || {
            let count = Rc::clone(&count);
            RawTooltip::new(
                Some("TIP".into()),
                |_app, _context, _animation| sized_box(42.0, 14.0).into_widget(),
                Builder::new(move |_app, _context| {
                    count.set(count.get() + 1);
                    SizedBox::expand().into_widget()
                }),
            )
            .into_widget()
        });
        assert_eq!(build_count.get(), 1);

        tester.mouse_add_pointer(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.mouse_remove_pointer();
        tester.pump(Duration::ZERO);

        assert_eq!(build_count.get(), 1);
    }

    /// `Center(SizedBox.square(10, RawTooltip(hoverDelay: 1s, child: SizedBox.expand())))`.
    fn square_hover_case(
        tester: &mut Tester,
        configure: impl Fn(RawTooltip) -> RawTooltip + 'static,
    ) -> (GlobalKey, GlobalKey) {
        let (tooltip_key, tip_key) = (GlobalKey::new(), GlobalKey::new());
        let (tooltip, key) = (tooltip_key.clone(), tip_key.clone());
        tester.pump_widget(move || {
            let key = key.clone();
            Center::new()
                .child(
                    sized_box(10.0, 10.0).child(configure(
                        RawTooltip::new(
                            Some("TIP".into()),
                            move |_app, _context, _animation| tip(&key),
                            SizedBox::expand(),
                        )
                        .key(tooltip.clone().into())
                        .hover_delay(Duration::from_secs(1)),
                    )),
                )
                .into_widget()
        });
        (tooltip_key, tip_key)
    }

    #[test]
    fn hold_mouse_button_down_and_hover_over_the_tooltip_widget() {
        let mut tester = Tester::new();
        let (_, tip_key) = square_hover_case(&mut tester, |tooltip| tooltip);

        tester.mouse_down(Offset::ZERO);
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        tester.mouse_up();
        tester.pump(Duration::from_secs(10 * 24 * 60 * 60));
        tester.pump_and_settle();
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible even when there is a PointerUp when hovered."
        );

        tester.mouse_move_to(Offset::ZERO);
        tester.pump(Duration::from_secs(1));
        tester.pump_and_settle();
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should be dismissed with no hovering mouse cursor."
        );
    }

    #[test]
    fn hovered_text_should_dismiss_when_clicked_outside() {
        let mut tester = Tester::new();
        let (_, tip_key) = square_hover_case(&mut tester, |tooltip| {
            tooltip.position_delegate(below_with_vertical_offset_24())
        });

        // Avoid using startGesture here to avoid the PointDown event from also being
        // interpreted as a PointHover event by the Tooltip.
        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        // Tapping on the Tooltip widget should dismiss the tooltip, since the
        // trigger mode is longPress.
        tester.tap(CENTER);
        tester.pump(Duration::ZERO);
        tester.pump_and_settle();
        assert!(!tester.finds(&tip_key));
        tester.mouse_remove_pointer();

        tester.mouse_move_to(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            tester.finds(&tip_key),
            "Tooltip should be visible when hovered."
        );

        tester.tap(Offset::ZERO);
        tester.pump(Duration::ZERO);
        tester.pump_and_settle();
        assert!(
            !tester.finds(&tip_key),
            "Tapping outside of the Tooltip widget should dismiss the tooltip."
        );
    }

    #[test]
    fn mouse_tap_and_hover_over_the_tooltip_widget() {
        // Regression test for https://github.com/flutter/flutter/issues/127575 .
        let mut tester = Tester::new();
        let (_, tip_key) = square_hover_case(&mut tester, |tooltip| tooltip);

        // The PointDown event is also interpreted as a PointHover event by the
        // Tooltip. This should be pretty rare but since it's more of a tap event
        // than a hover event, the tooltip shouldn't show unless the triggerMode
        // is set to tap.
        tester.mouse_down(CENTER);
        tester.pump(Duration::from_secs(1));
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should NOT be visible when hovered and tapped, when trigger mode is not tap"
        );
        tester.mouse_up();
        tester.mouse_remove_pointer();
        tester.pump(Duration::from_secs(10 * 24 * 60 * 60));
        tester.pump_and_settle();

        let (_, tip_key) = square_hover_case(&mut tester, |tooltip| {
            tooltip.trigger_mode(TooltipTriggerMode::Tap)
        });

        tester.mouse_down(CENTER);
        // The tap should be ignored, since Tooltip does not track "trigger gestures"
        // for mouse devices.
        tester.pump(Duration::from_millis(100));
        tester.mouse_up();
        tester.pump(Duration::from_secs(1));
        assert!(
            !tester.finds(&tip_key),
            "Tooltip should NOT be visible when hovered and tapped, when trigger mode is tap"
        );
    }

    #[test]
    fn tooltip_does_not_rebuild_for_show_and_hide_animation() {
        let mut tester = Tester::new();
        let tooltip_key = GlobalKey::new();
        let key = tooltip_key.clone();
        tester.pump_widget(move || {
            Center::new()
                .child(
                    sized_box(10.0, 10.0).child(
                        RawTooltip::new(
                            Some("TIP".into()),
                            |_app, _context, animation| {
                                FadeTransition::new(animation)
                                    .child(sized_box(42.0, 14.0))
                                    .into_widget()
                            },
                            SizedBox::expand(),
                        )
                        .key(key.clone().into())
                        .hover_delay(Duration::from_secs(1)),
                    ),
                )
                .into_widget()
        });
        let tooltip_state = tester.state(&tooltip_key);
        let element = tooltip_state.context(&tester.cell.borrow());
        let dirty = |tester: &Tester| element.dirty(&tester.cell.borrow());
        // The Tooltip widget itself is almost stateless thus doesn't need
        // rebuilding.
        assert!(!dirty(&tester));

        assert!(tooltip_state.ensure_tooltip_visible(&mut tester.cell.borrow_mut()));
        assert!(!dirty(&tester));
        tester.pump(Duration::from_secs(1));
        assert!(!dirty(&tester));

        assert!(RawTooltip::dismiss_all_tool_tips(
            &mut tester.cell.borrow_mut()
        ));
        assert!(!dirty(&tester));
        tester.pump(Duration::from_secs(1));
        assert!(!dirty(&tester));
    }

    /// `Center(RawTooltip(hoverDelay: 1s, child: SizedBox.square(50)))`.
    fn disposal_case(tester: &mut Tester) -> GlobalKey {
        let tooltip_key = GlobalKey::new();
        let key = tooltip_key.clone();
        tester.pump_widget(move || {
            Center::new()
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        |_app, _context, _animation| sized_box(42.0, 14.0).into_widget(),
                        sized_box(50.0, 50.0),
                    )
                    .key(key.clone().into())
                    .hover_delay(Duration::from_secs(1)),
                )
                .into_widget()
        });
        tooltip_key
    }

    #[test]
    fn tooltip_does_not_initialize_animation_controller_in_dispose_process() {
        let mut tester = Tester::new();
        disposal_case(&mut tester);

        let gesture = tester.start_gesture(CENTER);
        tester.pump_root(SizedBox::new().into_widget());

        // Finish gesture to release resources.
        tester.up(gesture, CENTER);
        tester.pump_and_settle();
    }

    #[test]
    fn tooltip_does_not_crash_when_showing_the_tooltip_but_the_overlay_portal_is_unmounted_during_dispose()
     {
        let mut tester = Tester::new();
        let tooltip_key = disposal_case(&mut tester);

        let tooltip_state = tester.state(&tooltip_key);
        let gesture = tester.start_gesture(CENTER);
        tooltip_state.ensure_tooltip_visible(&mut tester.cell.borrow_mut());
        tester.pump_root(SizedBox::new().into_widget());

        // Finish gesture to release resources.
        tester.up(gesture, CENTER);
        tester.pump_and_settle();
    }

    #[test]
    fn raw_tooltip_does_not_crash_at_zero_area() {
        let mut tester = Tester::with_platform(
            TestPlatform::new()
                .on(TargetPlatform::MacOS)
                .with_view(Rc::new(waft_test::TestView::new(0.0, 0.0))),
        );
        tester.install_fonts();
        let tooltip_key = GlobalKey::new();
        let key = tooltip_key.clone();
        tester.pump_widget(move || {
            Center::new()
                .child(
                    RawTooltip::new(
                        Some("TIP".into()),
                        |_app, _context, _animation| Text::new("Y").into_widget(),
                        Text::new("X"),
                    )
                    .key(key.clone().into()),
                )
                .into_widget()
        });
        assert_eq!(tester.rect_of(&tooltip_key).size(), Size::ZERO);
    }

    /// Dart's `moreOrLessEquals(expected)`: within `precisionErrorTolerance`.
    fn assert_more_or_less_equals(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-10,
            "{actual} is not {expected}"
        );
    }

    /// The tree of the `AnimationStyle` cases: `Center(RawTooltip(key:, animationStyle:,
    /// child: SizedBox(100x100)))`, whose tooltip builder keeps the animation it is given.
    struct AnimationStyleCase {
        tester: Tester,
        tooltip_key: GlobalKey,
        tip_key: GlobalKey,
        tooltip_animation: Rc<RefCell<Option<AnimationRef<f64>>>>,
    }

    impl AnimationStyleCase {
        fn new(animation_style: AnimationStyle) -> AnimationStyleCase {
            let mut this = AnimationStyleCase {
                tester: Tester::new(),
                tooltip_key: GlobalKey::new(),
                tip_key: GlobalKey::new(),
                tooltip_animation: Rc::default(),
            };
            this.pump_widget(animation_style);
            this
        }

        /// Dart's `tester.pumpWidget(buildApp(animationStyle))`.
        fn pump_widget(&mut self, animation_style: AnimationStyle) {
            let (tooltip_key, tip_key) = (self.tooltip_key.clone(), self.tip_key.clone());
            let tooltip_animation = Rc::clone(&self.tooltip_animation);
            self.tester.pump_widget(move || {
                let (tip_key, tooltip_animation) = (tip_key.clone(), Rc::clone(&tooltip_animation));
                Center::new()
                    .child(
                        RawTooltip::new(
                            Some("TIP".into()),
                            move |_app, _context, animation| {
                                *tooltip_animation.borrow_mut() = Some(animation);
                                tip(&tip_key)
                            },
                            sized_box(100.0, 100.0),
                        )
                        .key(tooltip_key.clone().into())
                        .animation_style(animation_style.clone()),
                    )
                    .into_widget()
            });
        }

        fn ensure_tooltip_visible(&mut self) {
            let state = self.tester.state(&self.tooltip_key);
            state.ensure_tooltip_visible(&mut self.tester.cell.borrow_mut());
        }

        fn dismiss_all_tool_tips(&mut self) {
            RawTooltip::dismiss_all_tool_tips(&mut self.tester.cell.borrow_mut());
        }

        /// `tooltipAnimation.value`.
        fn value(&self) -> f64 {
            let animation = self
                .tooltip_animation
                .borrow()
                .clone()
                .expect("the tooltip was built");
            animation.value(&self.tester.cell.borrow())
        }

        fn finds_tip(&self) -> bool {
            self.tester.finds(&self.tip_key)
        }
    }

    fn style(
        curve: Rc<dyn Curve>,
        duration: Option<Duration>,
        reverse_curve: Option<Rc<dyn Curve>>,
        reverse_duration: Option<Duration>,
    ) -> AnimationStyle {
        AnimationStyle::new(Some(curve), duration, reverse_curve, reverse_duration)
    }

    const MS_100: Option<Duration> = Some(Duration::from_millis(100));

    #[test]
    fn animation_style_reverse_curve_is_used_for_the_hide_animation() {
        let mut case = AnimationStyleCase::new(style(
            Curves::linear(),
            MS_100,
            Some(Curves::ease_in_expo()),
            MS_100,
        ));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(50));

        // Halfway through the show animation, the forward curve applies.
        assert_more_or_less_equals(case.value(), 0.5);
        // CurvedAnimation only switches to the reverse curve when the direction
        // changes from a completed state.
        case.tester.pump_and_settle();
        assert_eq!(case.value(), 1.0);

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(50));

        // Halfway through the hide animation, the reverse curve applies.
        assert_more_or_less_equals(case.value(), Curves::ease_in_expo().transform(0.5));
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn hide_animation_uses_the_forward_curve_when_animation_style_reverse_curve_is_null() {
        let mut case = AnimationStyleCase::new(style(Curves::ease_in_expo(), MS_100, None, MS_100));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump_and_settle();

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(50));

        // The hide animation follows the forward curve when no reverse curve is
        // provided.
        assert_more_or_less_equals(case.value(), Curves::ease_in_expo().transform(0.5));
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn animation_style_duration_changes_take_effect_after_the_first_show() {
        let mut case = AnimationStyleCase::new(style(Curves::linear(), MS_100, None, MS_100));

        // Show once to instantiate the lazily-created animation controller.
        case.ensure_tooltip_visible();
        case.tester.pump_and_settle();
        case.dismiss_all_tool_tips();
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());

        case.pump_widget(style(
            Curves::linear(),
            Some(Duration::from_millis(300)),
            None,
            MS_100,
        ));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(150));

        // Halfway through the updated 300ms show duration, not the original 100ms
        // duration which would have already completed the animation.
        assert_more_or_less_equals(case.value(), 0.5);
        case.tester.pump(Duration::from_millis(150));
        assert_eq!(case.value(), 1.0);

        case.dismiss_all_tool_tips();
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn null_animation_style_durations_fall_back_to_the_documented_defaults() {
        let mut case = AnimationStyleCase::new(style(Curves::linear(), None, None, None));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(75));

        // Halfway through the default 150ms show duration.
        assert_more_or_less_equals(case.value(), 0.5);
        case.tester.pump(Duration::from_millis(75));
        assert_eq!(case.value(), 1.0);

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_micros(37500));

        // Halfway through the default 75ms hide duration.
        assert_more_or_less_equals(case.value(), 0.5);
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn hide_animation_defaults_to_75ms_even_when_animation_style_duration_is_set() {
        let mut case = AnimationStyleCase::new(style(
            Curves::linear(),
            Some(Duration::from_millis(400)),
            None,
            None,
        ));

        case.ensure_tooltip_visible();
        case.tester.pump_and_settle();
        assert_eq!(case.value(), 1.0);

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_micros(37500));
        // Halfway through the default 75ms hide duration, not the 400ms duration.
        assert_more_or_less_equals(case.value(), 0.5);
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn animation_style_curve_changes_take_effect_after_the_first_show() {
        let mut case = AnimationStyleCase::new(style(Curves::linear(), MS_100, None, MS_100));

        // Show once to instantiate the lazily-created animation objects.
        case.ensure_tooltip_visible();
        case.tester.pump_and_settle();
        case.dismiss_all_tool_tips();
        case.tester.pump_and_settle();

        case.pump_widget(style(
            Curves::ease_in_expo(),
            MS_100,
            Some(Curves::bounce_out()),
            MS_100,
        ));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(50));

        // Halfway through the show animation, the updated forward curve applies.
        assert_more_or_less_equals(case.value(), Curves::ease_in_expo().transform(0.5));
        case.tester.pump_and_settle();

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(50));

        // Halfway through the hide animation, the updated reverse curve applies.
        assert_more_or_less_equals(case.value(), Curves::bounce_out().transform(0.5));
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn animation_style_changes_while_the_tooltip_is_shown_apply_to_the_next_animation() {
        let mut case = AnimationStyleCase::new(style(Curves::linear(), MS_100, None, MS_100));

        case.ensure_tooltip_visible();
        case.tester.pump_and_settle();
        assert_eq!(case.value(), 1.0);

        // Update the style while the tooltip is fully shown.
        case.pump_widget(style(
            Curves::linear(),
            MS_100,
            Some(Curves::ease_in_expo()),
            Some(Duration::from_millis(200)),
        ));
        assert!(case.finds_tip());

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(100));

        // Halfway through the updated 200ms hide duration, following the updated
        // reverse curve.
        assert_more_or_less_equals(case.value(), Curves::ease_in_expo().transform(0.5));
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn changing_animation_style_while_the_tooltip_is_animating_in_does_not_crash() {
        let mut case = AnimationStyleCase::new(style(Curves::linear(), MS_100, None, MS_100));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_millis(25));

        // Update the style while the show animation is running.
        case.pump_widget(style(
            Curves::ease_in_expo(),
            Some(Duration::from_millis(300)),
            None,
            MS_100,
        ));

        // The running show animation completes on its original 100ms timeline, of
        // which 25ms have already elapsed. Duration changes take effect the next
        // time an animation starts.
        case.tester.pump(Duration::from_millis(75));
        assert_eq!(case.value(), 1.0);
        assert!(case.finds_tip());

        case.dismiss_all_tool_tips();
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }

    #[test]
    fn zero_show_duration_shows_instantly_and_hides_with_the_default_75ms() {
        let mut case =
            AnimationStyleCase::new(style(Curves::linear(), Some(Duration::ZERO), None, None));

        case.ensure_tooltip_visible();
        case.tester.pump(Duration::ZERO);
        assert_eq!(case.value(), 1.0);
        assert!(case.finds_tip());

        case.dismiss_all_tool_tips();
        case.tester.pump(Duration::ZERO);
        case.tester.pump(Duration::from_micros(37500));

        // The hide animation uses the default 75ms reverse duration.
        assert_more_or_less_equals(case.value(), 0.5);
        case.tester.pump_and_settle();
        assert!(!case.finds_tip());
    }
}
