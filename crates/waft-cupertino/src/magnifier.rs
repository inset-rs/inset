//! Flutter counterpart: `cupertino/magnifier.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, Curve, CurvedAnimation,
    Curves, Tween,
};
use waft_embedder::{BlurStyle, Clip, Color, Offset, Radius, Rect, Size};
use waft_foundation::{App, Handle, Listenable, Listener, OptionRetained, Ref, ValueNotifier};
use waft_painting::{BorderRadius, BorderSide, BorderStyle, BoxShadow, RoundedRectangleBorder};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    AnimatedPositioned, BuildContext, IntoWidget, KeyRef, MagnifierController, MagnifierDecoration,
    MagnifierInfo, MediaQuery, RawMagnifier, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, State, StateData, StatefulWidget, StatelessWidget,
    Transform, WidgetRef,
};

use crate::theme::CupertinoTheme;

/// A [`CupertinoMagnifier`] used for magnifying text in cases where a user's
/// finger may be blocking the point of interest, like a selection handle.
///
/// Delegates styling to [`CupertinoMagnifier`] with its position depending on
/// [`magnifier_info`](Self::magnifier_info).
///
/// Specifically, the [`CupertinoTextMagnifier`] follows the following rules.
/// [`CupertinoTextMagnifier`]:
/// - is positioned horizontally inside the screen width, with
///   [`horizontal_screen_edge_padding`](Self::horizontal_screen_edge_padding) padding.
/// - is hidden if a gesture is detected [`hide_below_threshold`](Self::hide_below_threshold)
///   units below the line that the magnifier is on, shown otherwise.
/// - follows the x coordinate of the gesture directly (with respect to rule 1).
/// - has some vertical drag resistance; i.e. if a gesture is detected k units below the field,
///   then has vertical offset [`drag_resistance`](Self::drag_resistance) * k.
pub struct CupertinoTextMagnifier {
    pub key: Option<KeyRef>,
    /// The curve used for the in / out animations.
    pub animation_curve: Rc<dyn Curve>,
    /// This magnifier's controller.
    ///
    /// The [`CupertinoTextMagnifier`] requires a [`MagnifierController`]
    /// in order to show / hide itself without removing itself from the
    /// overlay.
    pub controller: Ref<MagnifierController>,
    /// A drag resistance on the downward Y position of the lens.
    pub drag_resistance: f64,
    /// The difference in Y between the gesture position and the caret center
    /// so that the magnifier hides itself.
    pub hide_below_threshold: f64,
    /// The padding on either edge of the screen that any part of the magnifier
    /// cannot exist past.
    ///
    /// This includes any part of the magnifier, not just the center; for example,
    /// the left edge of the magnifier cannot be outside the
    /// [`horizontal_screen_edge_padding`](Self::horizontal_screen_edge_padding).v
    ///
    /// If the screen has width w, then the magnifier is bound to
    /// `_kHorizontalScreenEdgePadding, w - _kHorizontalScreenEdgePadding`.
    pub horizontal_screen_edge_padding: f64,
    /// [`CupertinoTextMagnifier`] will determine its own positioning
    /// based on the [`MagnifierInfo`] of this notifier.
    pub magnifier_info: Ref<ValueNotifier<MagnifierInfo>>,
}

impl CupertinoTextMagnifier {
    /// Constructs a [`RawMagnifier`] in the Cupertino style, positioning with respect to
    /// [`magnifier_info`](Self::magnifier_info).
    ///
    /// The default constructor parameters and constants were eyeballed on
    /// an iPhone XR iOS v15.5.
    pub fn new(
        controller: Ref<MagnifierController>,
        magnifier_info: Ref<ValueNotifier<MagnifierInfo>>,
    ) -> CupertinoTextMagnifier {
        CupertinoTextMagnifier {
            key: None,
            animation_curve: Curves::ease_out(),
            controller,
            drag_resistance: 10.0,
            hide_below_threshold: 48.0,
            horizontal_screen_edge_padding: 10.0,
            magnifier_info,
        }
    }

    /// Dart `CupertinoTextMagnifier(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoTextMagnifier {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoTextMagnifier(animationCurve:)`.
    pub fn animation_curve(mut self, animation_curve: Rc<dyn Curve>) -> CupertinoTextMagnifier {
        self.animation_curve = animation_curve;
        self
    }

    /// Dart `CupertinoTextMagnifier(dragResistance:)`.
    pub fn drag_resistance(mut self, drag_resistance: f64) -> CupertinoTextMagnifier {
        self.drag_resistance = drag_resistance;
        self
    }

    /// Dart `CupertinoTextMagnifier(hideBelowThreshold:)`.
    pub fn hide_below_threshold(mut self, hide_below_threshold: f64) -> CupertinoTextMagnifier {
        self.hide_below_threshold = hide_below_threshold;
        self
    }

    /// Dart `CupertinoTextMagnifier(horizontalScreenEdgePadding:)`.
    pub fn horizontal_screen_edge_padding(
        mut self,
        horizontal_screen_edge_padding: f64,
    ) -> CupertinoTextMagnifier {
        self.horizontal_screen_edge_padding = horizontal_screen_edge_padding;
        self
    }

    /// The duration that the magnifier drags behind its final position.
    const K_DRAG_ANIMATION_DURATION: Duration = Duration::from_millis(45);
}

impl Debug for CupertinoTextMagnifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoTextMagnifier")
            .field("key", &self.key)
            .field("controller", &self.controller)
            .field("drag_resistance", &self.drag_resistance)
            .field("hide_below_threshold", &self.hide_below_threshold)
            .field(
                "horizontal_screen_edge_padding",
                &self.horizontal_screen_edge_padding,
            )
            .field("magnifier_info", &self.magnifier_info)
            .finish()
    }
}

impl StatefulWidget for CupertinoTextMagnifier {
    type State = CupertinoTextMagnifierState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoTextMagnifierState {
        CupertinoTextMagnifierState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            current_adjusted_magnifier_position: Offset::ZERO,
            vertical_focal_point_adjustment: 0.0,
            io_animation_controller: None,
            io_animation: None,
            io_curved_animation: None,
        }
    }
}

/// Dart's `_CupertinoTextMagnifierState`.
pub struct CupertinoTextMagnifierState {
    state: StateData<CupertinoTextMagnifier>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    // Initialize to dummy values for the event that the initial call to
    // _determineMagnifierPositionAndFocalPoint calls hide, and thus does not
    // set these values.
    current_adjusted_magnifier_position: Offset,
    vertical_focal_point_adjustment: f64,
    io_animation_controller: Option<Ref<AnimationController>>,
    io_animation: Option<AnimationRef<f64>>,
    io_curved_animation: Option<Ref<CurvedAnimation>>,
}

impl CupertinoTextMagnifierState {
    fn io_animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .io_animation_controller
            .handle()
            .expect("created in init_state")
    }

    fn determine_magnifier_position_and_focal_point(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let magnifier_info = widget.magnifier_info.handle();
        let controller = widget.controller.handle();
        let (hide_below_threshold, drag_resistance, horizontal_screen_edge_padding) = (
            widget.hide_below_threshold,
            widget.drag_resistance,
            widget.horizontal_screen_edge_padding,
        );
        let text_editing_context = *magnifier_info.value(app);

        // The exact Y of the center of the current line.
        let vertical_center_of_current_line = text_editing_context.caret_rect.center().dy();

        // If the magnifier is currently showing, but we have dragged out of threshold,
        // we should hide it.
        if vertical_center_of_current_line - text_editing_context.global_gesture_position.dy()
            < -hide_below_threshold
        {
            // Only signal a hide if we are currently showing.
            if controller.shown(app) {
                controller.hide(app, false);
            }
            return;
        }

        // If we are gone, but got to this point, we shouldn't be: show.
        if !controller.shown(app) {
            self.io_animation_controller(app).forward(app, None);
        }

        // Never go above the center of the line, but have some resistance
        // going downward if the drag goes too far.
        let vertical_position_of_lens = vertical_center_of_current_line.max(
            vertical_center_of_current_line
                - (vertical_center_of_current_line
                    - text_editing_context.global_gesture_position.dy())
                    / drag_resistance,
        );

        // The raw position, tracking the gesture directly.
        let raw_magnifier_position = Offset::new(
            text_editing_context.global_gesture_position.dx()
                - CupertinoMagnifier::K_DEFAULT_SIZE.width() / 2.0,
            vertical_position_of_lens
                - (CupertinoMagnifier::K_DEFAULT_SIZE.height()
                    - CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT),
        );

        let context = self.context(app);
        let screen_rect = Offset::ZERO & MediaQuery::size_of(app, context);

        // Adjust the magnifier position so that it never exists outside the horizontal
        // padding.
        let adjusted_magnifier_position = MagnifierController::shift_within_bounds(
            raw_magnifier_position & CupertinoMagnifier::K_DEFAULT_SIZE,
            Rect::from_ltrb(
                screen_rect.left + horizontal_screen_edge_padding,
                // iOS doesn't reposition for Y, so we should expand the threshold
                // so we can send the whole magnifier out of bounds if need be.
                screen_rect.top
                    - (CupertinoMagnifier::K_DEFAULT_SIZE.height()
                        + CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT),
                screen_rect.right - horizontal_screen_edge_padding,
                screen_rect.bottom
                    + (CupertinoMagnifier::K_DEFAULT_SIZE.height()
                        + CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT),
            ),
        )
        .top_left();

        self.set_state(app, |state| {
            state.current_adjusted_magnifier_position = adjusted_magnifier_position;
            // The lens should always point to the center of the line.
            state.vertical_focal_point_adjustment =
                vertical_center_of_current_line - vertical_position_of_lens;
        });
    }
}

impl SingleTickerProviderStateMixin for CupertinoTextMagnifierState {
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
impl TickerProviderObject for CupertinoTextMagnifierState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoTextMagnifierState {
    type Widget = CupertinoTextMagnifier;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let io_animation_controller = AnimationController::new(
            app,
            Some(0.0),
            Some(CupertinoMagnifier::K_IN_OUT_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        io_animation_controller
            .add_listener(app, Listener::new(move |app| self.set_state(app, |_| {})));
        app.get_mut(self).io_animation_controller = Some(io_animation_controller.clone());

        let widget = self.widget(app);
        let (controller, magnifier_info, animation_curve) = (
            widget.controller.handle(),
            widget.magnifier_info.clone(),
            widget.animation_curve.clone(),
        );
        app.get_mut(controller).animation_controller = Some(io_animation_controller.clone());
        magnifier_info.add_listener(
            app,
            Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
        );
        let io_curved_animation =
            CurvedAnimation::new(app, io_animation_controller.erase(), animation_curve, None);
        let io_tween = Tween::new(app, Some(0.0), Some(1.0));
        let io_animation = io_tween.animate(app, io_curved_animation.erase());
        let state = app.get_mut(self);
        state.io_curved_animation = Some(io_curved_animation);
        state.io_animation = Some(io_animation);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (controller, magnifier_info) =
            (widget.controller.handle(), widget.magnifier_info.clone());
        app.get_mut(controller).animation_controller = None;
        let io_animation_controller = self.io_animation_controller(app);
        io_animation_controller.dispose(app);
        let io_curved_animation = app
            .get(self)
            .io_curved_animation
            .handle()
            .expect("created in init_state");
        io_curved_animation.dispose(app);
        magnifier_info.remove_listener(
            app,
            &Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
        );
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoTextMagnifier) {
        let magnifier_info = self.widget(app).magnifier_info.clone();
        if old_widget.magnifier_info != magnifier_info {
            old_widget.magnifier_info.remove_listener(
                app,
                &Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
            );
            magnifier_info.add_listener(
                app,
                Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
            );
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.determine_magnifier_position_and_focal_point(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme_data = CupertinoTheme::of(app, context);
        let state = app.get(self);
        let (position, vertical_focal_point_adjustment, io_animation) = (
            state.current_adjusted_magnifier_position,
            state.vertical_focal_point_adjustment,
            state.io_animation.clone().expect("created in init_state"),
        );
        AnimatedPositioned::new(
            CupertinoMagnifier::new()
                .in_out_animation(io_animation)
                .additional_focal_point_offset(Offset::new(0.0, vertical_focal_point_adjustment))
                .border_side(BorderSide::new(
                    theme_data.primary_color().color(),
                    2.0,
                    BorderStyle::Solid,
                    BorderSide::STROKE_ALIGN_INSIDE,
                )),
            CupertinoTextMagnifier::K_DRAG_ANIMATION_DURATION,
        )
        .curve(self.widget(app).animation_curve.clone())
        .left(position.dx())
        .top(position.dy())
        .into_widget()
    }
}

/// A [`RawMagnifier`] used for magnifying text in cases where a user's
/// finger may be blocking the point of interest, like a selection handle.
///
/// [`CupertinoMagnifier`] is a wrapper around [`RawMagnifier`] that handles styling
/// and transitions.
///
/// This magnifying glass is useful for scenarios on mobile devices where
/// the user's finger may be covering part of the screen where a granular
/// action is being performed, such as navigating a small cursor with a drag
/// gesture, on an image or text.
///
/// See also:
///
/// * [`RawMagnifier`], the backing implementation.
/// * [`CupertinoTextMagnifier`], a widget that positions [`CupertinoMagnifier`] based on
///   [`MagnifierInfo`].
/// * [`MagnifierController`], the controller for this magnifier.
#[derive(Debug)]
pub struct CupertinoMagnifier {
    pub key: Option<KeyRef>,
    /// A list of shadows cast by the `Magnifier`.
    ///
    /// If the shadows use a [`BlurStyle`] that paints inside the shape, or if they
    /// are offset, then a [`clip_behavior`](Self::clip_behavior) that enables clipping (such
    /// as [`Clip::HardEdge`]) is recommended, otherwise the shadow will occlude the
    /// magnifier (the shadow is drawn above the magnifier so as to not be
    /// included in the magnified image).
    ///
    /// A shadow that uses [`BlurStyle::Outer`] and is not offset does not need
    /// clipping.
    ///
    /// By default, the [`shadows`](Self::shadows) are not offset and use
    /// [`BlurStyle::Outer`], and correspondingly the default
    /// [`clip_behavior`](Self::clip_behavior) is [`Clip::None`].
    pub shadows: Vec<BoxShadow>,
    /// Whether and how to clip the [`shadows`](Self::shadows) that render inside the loupe.
    ///
    /// Defaults to [`Clip::None`], which is useful if the shadow will not paint
    /// where the magnified image appears, or if doing so is intentional (e.g. to
    /// blur the edges of the magnified image).
    ///
    /// The default configuration of [`CupertinoMagnifier`] does not render inside
    /// the loupe (the shadows are not offset and use [`BlurStyle::Outer`]).
    ///
    /// Other values (e.g. [`Clip::HardEdge`]) are recommended when the
    /// [`shadows`](Self::shadows) have an offset.
    ///
    /// See the discussion at [`shadows`](Self::shadows).
    pub clip_behavior: Clip,
    /// The border, or "rim", of this magnifier.
    ///
    /// This border is drawn on a [`RoundedRectangleBorder`] with radius
    /// [`border_radius`](Self::border_radius), and increases the [`size`](Self::size) of the
    /// magnifier by the [`BorderSide::width`].
    pub border_side: BorderSide,
    /// The size of this magnifier.
    ///
    /// The size does not include the [`border_side`](Self::border_side) or
    /// [`shadows`](Self::shadows).
    pub size: Size,
    /// The border radius of this magnifier.
    ///
    /// The magnifier's shape is a [`RoundedRectangleBorder`] with this radius.
    pub border_radius: BorderRadius,
    /// This [`RawMagnifier`]'s controller.
    ///
    /// Since [`CupertinoMagnifier`] has no knowledge of shown / hidden state,
    /// this animation should be driven by an external actor.
    pub in_out_animation: Option<AnimationRef<f64>>,
    /// Any additional focal point offset, applied over the regular focal
    /// point offset defined in [`K_MAGNIFIER_ABOVE_FOCAL_POINT`](Self::K_MAGNIFIER_ABOVE_FOCAL_POINT).
    pub additional_focal_point_offset: Offset,
    /// The magnification scale for the magnifier.
    ///
    /// Defaults to 1.0, which indicates that the magnifier does not apply any magnification.
    pub magnification_scale: f64,
}

impl CupertinoMagnifier {
    /// Creates a [`RawMagnifier`] in the Cupertino style.
    ///
    /// The default constructor parameters and constants were eyeballed on
    /// an iPhone 16 iOS v18.1.
    pub fn new() -> CupertinoMagnifier {
        CupertinoMagnifier::default()
    }

    /// Dart `CupertinoMagnifier(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoMagnifier {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoMagnifier(size:)`.
    pub fn size(mut self, size: Size) -> CupertinoMagnifier {
        self.size = size;
        self
    }

    /// Dart `CupertinoMagnifier(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> CupertinoMagnifier {
        self.border_radius = border_radius;
        self
    }

    /// Dart `CupertinoMagnifier(additionalFocalPointOffset:)`.
    pub fn additional_focal_point_offset(
        mut self,
        additional_focal_point_offset: Offset,
    ) -> CupertinoMagnifier {
        self.additional_focal_point_offset = additional_focal_point_offset;
        self
    }

    /// Dart `CupertinoMagnifier(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> CupertinoMagnifier {
        self.shadows = shadows;
        self
    }

    /// Dart `CupertinoMagnifier(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CupertinoMagnifier {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `CupertinoMagnifier(borderSide:)`.
    pub fn border_side(mut self, border_side: BorderSide) -> CupertinoMagnifier {
        self.border_side = border_side;
        self
    }

    /// Dart `CupertinoMagnifier(inOutAnimation:)`.
    pub fn in_out_animation(mut self, in_out_animation: AnimationRef<f64>) -> CupertinoMagnifier {
        self.in_out_animation = Some(in_out_animation);
        self
    }

    /// Dart `CupertinoMagnifier(magnificationScale:)`.
    pub fn magnification_scale(mut self, magnification_scale: f64) -> CupertinoMagnifier {
        debug_assert!(
            magnification_scale > 0.0,
            "The magnification scale should be greater than zero."
        );
        self.magnification_scale = magnification_scale;
        self
    }

    /// The vertical offset that the magnifier is along the Y axis above
    /// the focal point.
    pub const K_MAGNIFIER_ABOVE_FOCAL_POINT: f64 = -26.0;

    /// The default size of the magnifier.
    ///
    /// This is public so that positioners can choose to depend on it, although
    /// it is overridable.
    pub const K_DEFAULT_SIZE: Size = Size::new(80.0, 47.5);

    /// The duration that this magnifier animates in / out for.
    ///
    /// The animation is a translation and a fade. The translation
    /// begins at the focal point, and ends at
    /// [`K_MAGNIFIER_ABOVE_FOCAL_POINT`](Self::K_MAGNIFIER_ABOVE_FOCAL_POINT).
    /// The opacity begins at 0 and ends at 1.
    const K_IN_OUT_ANIMATION_DURATION: Duration = Duration::from_millis(150);
}

impl Default for CupertinoMagnifier {
    fn default() -> CupertinoMagnifier {
        CupertinoMagnifier {
            key: None,
            shadows: vec![BoxShadow::new(
                Color::from_argb(25, 0, 0, 0),
                Offset::ZERO,
                11.0,
                0.2,
                BlurStyle::Outer,
            )],
            clip_behavior: Clip::None,
            border_side: BorderSide::new(
                Color::from_argb(255, 0, 124, 255),
                2.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ),
            size: CupertinoMagnifier::K_DEFAULT_SIZE,
            border_radius: BorderRadius::all(Radius::elliptical(60.0, 50.0)),
            in_out_animation: None,
            additional_focal_point_offset: Offset::ZERO,
            magnification_scale: 1.0,
        }
    }
}

impl StatelessWidget for CupertinoMagnifier {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation_value = self
            .in_out_animation
            .as_ref()
            .map_or(1.0, |in_out_animation| in_out_animation.value(app));
        let default_y_offset =
            (self.size.height() / 2.0) - CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT;

        // Cancel the downward translation below so the lens keeps showing the focal
        // point during the in/out animation, matching the native iOS loupe.
        let focal_point_offset = Offset::new(
            0.0,
            default_y_offset
                + CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT * (1.0 - animation_value),
        ) + self.additional_focal_point_offset;

        Transform::translate(
            Offset::lerp(
                Some(Offset::new(
                    0.0,
                    -CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT,
                )),
                Some(Offset::ZERO),
                animation_value,
            )
            .expect("both ends are given"),
        )
        .child(
            RawMagnifier::new(self.size)
                .focal_point_offset(focal_point_offset)
                .decoration(
                    MagnifierDecoration::new()
                        .opacity(animation_value)
                        .shape(RoundedRectangleBorder::new(
                            self.border_side,
                            self.border_radius.into(),
                        ))
                        .shadows(self.shadows.clone()),
                )
                .clip_behavior(self.clip_behavior)
                .magnification_scale(self.magnification_scale),
        )
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_widgets::testing::{TestVSync, WidgetTester, test_widgets};
    use waft_widgets::{Center, GlobalKey, ListenableBuilder, SizedBox};

    use super::*;
    use crate::app::CupertinoApp;
    use crate::colors::CupertinoColors;
    use crate::page_scaffold::CupertinoPageScaffold;
    use crate::theme::CupertinoThemeData;

    /// Dart's `basicOffset`: from the magnifier's top left to the point it magnifies.
    const BASIC_OFFSET: Offset = Offset::new(
        CupertinoMagnifier::K_DEFAULT_SIZE.width() / 2.0,
        CupertinoMagnifier::K_DEFAULT_SIZE.height()
            - CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT,
    );
    const REASONABLE_TEXT_FIELD: Rect = Rect::from_ltrb(0.0, 100.0, 200.0, 200.0);

    /// `tester.widget<W>(find.byType(W))`, read through `read`.
    fn read_widget<W: 'static, R>(tester: &WidgetTester, read: impl FnOnce(&W) -> R) -> R {
        let found = *tester.find_by_type::<W>().first().expect("a mounted W");
        tester.widget(found, read)
    }

    fn magnifier_position(tester: &WidgetTester) -> Offset {
        read_widget::<AnimatedPositioned, _>(tester, |positioned| {
            Offset::new(
                positioned.left.unwrap_or(0.0),
                positioned.top.unwrap_or(0.0),
            )
        })
    }

    /// Dart's `CupertinoApp(home: Placeholder())`; the home's context is the magnifier's.
    fn mount_app(tester: &mut WidgetTester, home: GlobalKey) {
        tester.pump_widget(CupertinoApp::new().home(SizedBox::new().key(home.into())));
    }

    /// Dart's `showCupertinoMagnifier`: shows the magnifier and runs it to the end of its
    /// entry animation.
    fn show_cupertino_magnifier(
        tester: &mut WidgetTester,
        context: BuildContext,
        magnifier_controller: Ref<MagnifierController>,
        magnifier_info: Ref<ValueNotifier<MagnifierInfo>>,
    ) {
        let shown_controller = magnifier_controller.clone();
        tester.update(|app| {
            magnifier_controller.show(
                app,
                context,
                Rc::new(move |_app: &mut App, _context: BuildContext| {
                    CupertinoTextMagnifier::new(shown_controller.clone(), magnifier_info.clone())
                        .into_widget()
                }),
                None,
            )
        });
        tester.pump();
        tester.pump_by(Duration::from_secs(2));
    }

    /// Dart's `tearDown`: `magnifierController.removeFromOverlay()`.
    fn tear_down(tester: &WidgetTester, magnifier_controller: &Ref<MagnifierController>) {
        tester.update(|app| magnifier_controller.remove_from_overlay(app));
    }

    fn info(global_gesture_position: Offset) -> MagnifierInfo {
        MagnifierInfo::new(
            global_gesture_position,
            REASONABLE_TEXT_FIELD,
            REASONABLE_TEXT_FIELD,
            REASONABLE_TEXT_FIELD,
        )
    }

    #[test]
    fn magnifier_border_color_inherits_from_parent_cupertino_theme() {
        test_widgets(|tester| {
            let fake_text_field = GlobalKey::new();
            let placeholder = GlobalKey::new();
            tester.pump_widget(
                CupertinoApp::new().home(
                    SizedBox::square(Some(10.0))
                        .key(fake_text_field.clone().into())
                        .child(CupertinoTheme::new(
                            CupertinoThemeData::new()
                                .with_primary_color(CupertinoColors::ACTIVE_GREEN),
                            SizedBox::new().key(placeholder.clone().into()),
                        )),
                ),
            );
            let context = placeholder
                .current_context(&tester.cell.borrow())
                .expect("mounted");
            let fake_text_field_rect = tester.global_rect(
                tester
                    .render_object(&fake_text_field)
                    .as_box()
                    .expect("laid out"),
            );
            let magnifier = tester.update(|app| {
                app.create(ValueNotifier::new(MagnifierInfo::new(
                    fake_text_field_rect.center(),
                    fake_text_field_rect,
                    fake_text_field_rect,
                    fake_text_field_rect,
                )))
            });
            let magnifier_controller = tester.update(MagnifierController::new);

            show_cupertino_magnifier(
                tester,
                context,
                magnifier_controller.clone(),
                magnifier.clone(),
            );

            // Magnifier border color should inherit from CupertinoTheme.of(context).primaryColor.
            let magnifier_border_color =
                read_widget::<CupertinoMagnifier, _>(tester, |magnifier| {
                    magnifier.border_side.color
                });
            assert_eq!(
                magnifier_border_color,
                CupertinoColors::ACTIVE_GREEN.color()
            );
            tear_down(tester, &magnifier_controller);
        });
    }

    #[test]
    fn should_never_horizontally_be_outside_of_screen_padding() {
        test_widgets(|tester| {
            let home = GlobalKey::new();
            mount_app(tester, home.clone());
            let context = home
                .current_context(&tester.cell.borrow())
                .expect("mounted");
            let width = tester.update(|app| MediaQuery::size_of(app, context).width());
            // The tap position is far out of the right side of the app.
            let magnifier_info = tester.update(|app| {
                app.create(ValueNotifier::new(info(Offset::new(width + 100.0, 0.0))))
            });
            let magnifier_controller = tester.update(MagnifierController::new);

            show_cupertino_magnifier(
                tester,
                context,
                magnifier_controller.clone(),
                magnifier_info.clone(),
            );

            // Should be less than the right edge, since we have padding.
            assert!(magnifier_position(tester).dx() < width);
            tear_down(tester, &magnifier_controller);
        });
    }

    #[test]
    fn should_have_some_vertical_drag() {
        test_widgets(|tester| {
            let home = GlobalKey::new();
            mount_app(tester, home.clone());
            let context = home
                .current_context(&tester.cell.borrow())
                .expect("mounted");
            let drag_position_below_text_field = REASONABLE_TEXT_FIELD.center().dy() + 30.0;
            let width = tester.update(|app| MediaQuery::size_of(app, context).width());
            let magnifier_info = tester.update(|app| {
                app.create(ValueNotifier::new(info(Offset::new(
                    width / 2.0,
                    drag_position_below_text_field,
                ))))
            });
            let magnifier_controller = tester.update(MagnifierController::new);

            show_cupertino_magnifier(
                tester,
                context,
                magnifier_controller.clone(),
                magnifier_info.clone(),
            );

            // The magnifier Y should be greater than the text field, since we "dragged" it down.
            let magnified_y = magnifier_position(tester).dy() + BASIC_OFFSET.dy();
            assert!(magnified_y > REASONABLE_TEXT_FIELD.center().dy());
            assert!(magnified_y < drag_position_below_text_field);
            tear_down(tester, &magnifier_controller);
        });
    }

    /// Dart's 'status' group: shows the magnifier, then moves the gesture 100 below the line.
    fn shown_then_dragged_below(
        tester: &mut WidgetTester,
        context: BuildContext,
    ) -> (Ref<MagnifierController>, Ref<ValueNotifier<MagnifierInfo>>) {
        let width = tester.update(|app| MediaQuery::size_of(app, context).width());
        let magnifier_info = tester.update(|app| {
            app.create(ValueNotifier::new(info(Offset::new(
                width / 2.0,
                REASONABLE_TEXT_FIELD.top,
            ))))
        });
        let magnifier_controller = tester.update(MagnifierController::new);

        // Show the magnifier initially, so that we get it in a not hidden state.
        show_cupertino_magnifier(
            tester,
            context,
            magnifier_controller.clone(),
            magnifier_info.clone(),
        );

        // Move the gesture to one that should hide it.
        tester.update(|app| {
            let below = magnifier_info.value(app).global_gesture_position + Offset::new(0.0, 100.0);
            magnifier_info.set_value(app, info(below));
        });
        tester.pump_and_settle();

        {
            let app = tester.cell.borrow();
            assert!(!magnifier_controller.shown(&app));
            assert!(magnifier_controller.overlay_entry(&app).is_some());
        }
        (magnifier_controller, magnifier_info)
    }

    #[test]
    fn should_hide_if_gesture_is_far_below_the_text_field() {
        test_widgets(|tester| {
            let home = GlobalKey::new();
            mount_app(tester, home.clone());
            let context = home
                .current_context(&tester.cell.borrow())
                .expect("mounted");
            let (magnifier_controller, _) = shown_then_dragged_below(tester, context);
            tear_down(tester, &magnifier_controller);
        });
    }

    #[test]
    fn should_re_show_if_gesture_moves_back_up() {
        test_widgets(|tester| {
            let home = GlobalKey::new();
            mount_app(tester, home.clone());
            let context = home
                .current_context(&tester.cell.borrow())
                .expect("mounted");
            let (magnifier_controller, magnifier_info) = shown_then_dragged_below(tester, context);

            // Return the gesture to one that shows it.
            let width = tester.update(|app| MediaQuery::size_of(app, context).width());
            tester.update(|app| {
                magnifier_info.set_value(
                    app,
                    info(Offset::new(width / 2.0, REASONABLE_TEXT_FIELD.top)),
                )
            });
            tester.pump_and_settle();

            {
                let app = tester.cell.borrow();
                assert!(magnifier_controller.shown(&app));
                assert!(magnifier_controller.overlay_entry(&app).is_some());
            }
            tear_down(tester, &magnifier_controller);
        });
    }

    #[test]
    #[should_panic(expected = "The magnification scale should be greater than zero.")]
    fn throws_assertion_error_when_magnification_scale_is_zero() {
        CupertinoPageScaffold::new(CupertinoMagnifier::new().magnification_scale(0.0));
    }

    #[test]
    #[should_panic(expected = "The magnification scale should be greater than zero.")]
    fn throws_assertion_error_when_magnification_scale_is_negative() {
        CupertinoPageScaffold::new(CupertinoMagnifier::new().magnification_scale(-1.0));
    }

    fn mount_magnifier(tester: &mut WidgetTester, magnifier: CupertinoMagnifier) {
        tester.pump_widget(CupertinoApp::new().home(CupertinoPageScaffold::new(magnifier)));
    }

    #[test]
    fn cupertino_magnifier_magnification_scale_defaults_to_1() {
        test_widgets(|tester| {
            mount_magnifier(tester, CupertinoMagnifier::new());
            let scale = read_widget::<RawMagnifier, _>(tester, |raw| raw.magnification_scale);
            assert_eq!(scale, 1.0);
        });
    }

    #[test]
    fn magnification_scale_argument_is_passed_to_the_raw_magnifier() {
        test_widgets(|tester| {
            mount_magnifier(tester, CupertinoMagnifier::new().magnification_scale(2.0));
            let scale = read_widget::<RawMagnifier, _>(tester, |raw| raw.magnification_scale);
            assert_eq!(scale, 2.0);
        });
    }

    #[test]
    fn cupertino_magnifier_focal_point_is_based_on_its_size() {
        test_widgets(|tester| {
            let focal_point_offset_for = |tester: &mut WidgetTester, size: Size| {
                mount_magnifier(tester, CupertinoMagnifier::new().size(size));
                read_widget::<RawMagnifier, _>(tester, |raw| raw.focal_point_offset)
            };

            assert_eq!(
                focal_point_offset_for(tester, CupertinoMagnifier::K_DEFAULT_SIZE),
                Offset::new(
                    0.0,
                    CupertinoMagnifier::K_DEFAULT_SIZE.height() / 2.0
                        - CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT
                )
            );
            assert_eq!(
                focal_point_offset_for(tester, Size::new(100.0, 200.0)),
                Offset::new(
                    0.0,
                    100.0 - CupertinoMagnifier::K_MAGNIFIER_ABOVE_FOCAL_POINT
                )
            );
        });
    }

    #[test]
    fn cupertino_magnifier_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(CupertinoApp::new().home(Center::new().child(
                SizedBox::shrink().child(CupertinoMagnifier::new().magnification_scale(2.0)),
            )));
            let magnifier = tester.find_by_type::<CupertinoMagnifier>()[0];
            let size = tester.render_box_of(magnifier).size(&tester.cell.borrow());
            assert_eq!(size, Size::ZERO);
        });
    }

    #[test]
    fn cupertino_magnifier_keeps_magnifying_the_same_point_during_in_out_animation() {
        test_widgets(|tester| {
            let controller = tester.update(|app| {
                AnimationController::new(
                    app,
                    None,
                    None,
                    None,
                    0.0,
                    1.0,
                    AnimationBehavior::Normal,
                    TestVSync,
                )
            });
            let built_with = controller.clone();
            tester.pump_widget(CupertinoApp::new().home(Center::new().child(
                // CupertinoMagnifier does not listen to the animation itself; its
                // owner (e.g. CupertinoTextMagnifier) rebuilds it on every tick.
                ListenableBuilder::new(
                    Rc::new(controller.clone()),
                    move |_app, _context, _child| {
                        CupertinoMagnifier::new()
                            .in_out_animation(built_with.erase())
                            .into_widget()
                    },
                ),
            )));
            // The global position the lens is magnifying: lens center (which includes
            // the in/out translation) plus the focal point offset.
            let magnified_point = |tester: &WidgetTester| {
                let raw = tester.find_by_type::<RawMagnifier>()[0];
                let render_box = tester.render_box_of(raw);
                let center = {
                    let app = tester.cell.borrow();
                    let size = render_box.size(&app);
                    render_box.local_to_global(
                        &app,
                        Offset::new(size.width() / 2.0, size.height() / 2.0),
                        None,
                    )
                };
                let focal_point_offset =
                    read_widget::<RawMagnifier, _>(tester, |raw| raw.focal_point_offset);
                center + focal_point_offset
            };

            tester.update(|app| controller.set_value(app, 1.0));
            tester.pump();
            let at_rest = magnified_point(tester);
            for value in [0.0, 0.25, 0.5, 0.75] {
                tester.update(|app| controller.set_value(app, value));
                tester.pump();
                let point = magnified_point(tester);
                assert!(
                    (point.dx() - at_rest.dx()).abs() < 1e-10
                        && (point.dy() - at_rest.dy()).abs() < 1e-10,
                    "value={value}: {point:?} is not {at_rest:?}"
                );
            }
        });
    }
}
