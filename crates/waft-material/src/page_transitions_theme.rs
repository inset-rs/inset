//! Port of material_ui's `page_transitions_theme.dart`.
//!
//! Flutter rasterises the entering and exiting pages of the zoom and fade-forwards transitions
//! through `SnapshotWidget`; Waft's `SnapshotWidget` paints the child live (`OffsetLayer`
//! `.toImageSync` is deferred), so the zoom painters' `paint` is what draws, and their
//! `paint_snapshot` waits for the raster.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;
use waft_animation::{
    Animatable, AnimationRef, AnimationStatus, AnimationStatusListener, Cubic, Curve, Curves,
    Interval, ReverseAnimation, TweenSequence, TweenSequenceItem, ValueCurveTween, ValueTween,
    k_always_complete_animation,
};

use indexmap::IndexMap;
use waft_cupertino::CupertinoPageTransitionsBuilder;
use waft_embedder::valo::Sampling;
use waft_embedder::{Color, Image, Matrix4, Offset, Paint, Rect, Size, TargetPlatform};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, Listener, OptionRetained, Ref,
};
use waft_painting::ClipContext;
use waft_rendering::{LayerHandle, OpacityLayer, PaintingContext, TransformLayer};
use waft_widgets::{
    AnyModalRoute, AnyPageRoute, AnySnapshotPainter, BuildContext, ColoredBox,
    DelegatedTransitionBuilder, DualTransitionBuilder, FadeTransition, IgnorePointer, IntoWidget,
    PageTransitionsBuilder, PageTransitionsBuilderRef, SlideTransition, SnapshotController,
    SnapshotMode, SnapshotPainter, SnapshotWidget, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef,
};

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::predictive_back_page_transitions_builder::PredictiveBackPageTransitionsBuilder;
use crate::theme::Theme;

// Zooms and fades a new page in, zooming out the previous page. This transition is designed to
// match the Android Q activity transition.
#[derive(Debug)]
pub(crate) struct ZoomPageTransition {
    /// The animation that drives the `child`'s entrance and exit.
    animation: AnimationRef<f64>,

    /// The animation that transitions `child` when new content is pushed on top of it.
    secondary_animation: AnimationRef<f64>,

    /// Whether the [`SnapshotWidget`] will be used.
    ///
    /// When this value is true, performance is improved by disabling animations on both the
    /// outgoing and incoming route. This also implies that ink-splashes or similar animations
    /// will not animate during the transition.
    allow_snapshotting: bool,

    /// The color of the scrim (background) that fades in and out during the transition.
    ///
    /// If not provided, defaults to current theme's `ColorScheme::surface` color.
    background_color: Option<Color>,

    /// The widget below this widget in the tree.
    child: Option<WidgetRef>,

    /// Whether to enable snapshotting on the entering route during the transition animation.
    ///
    /// If false, the route snapshotting will not be applied to the route being animating into,
    /// e.g. when transitioning from route A to route B, B will not be snapshotted.
    allow_enter_route_snapshotting: bool,
}

impl ZoomPageTransition {
    // A curve sequence that is similar to the 'fastOutExtraSlowIn' curve used in the native
    // transition.
    fn fast_out_extra_slow_in_tween_sequence_items() -> Vec<TweenSequenceItem<f64>> {
        vec![
            TweenSequenceItem::new(
                Rc::new(
                    ValueTween::new(0.0, 0.4).chain(ValueCurveTween::new(Rc::new(Cubic::new(
                        0.05, 0.0, 0.133333, 0.06,
                    )))),
                ),
                0.166666,
            ),
            TweenSequenceItem::new(
                Rc::new(
                    ValueTween::new(0.4, 1.0).chain(ValueCurveTween::new(Rc::new(Cubic::new(
                        0.208333, 0.82, 0.25, 1.0,
                    )))),
                ),
                1.0 - 0.166666,
            ),
        ]
    }

    fn scale_curve_sequence() -> TweenSequence<f64> {
        TweenSequence::new(ZoomPageTransition::fast_out_extra_slow_in_tween_sequence_items())
    }
}

impl StatelessWidget for ZoomPageTransition {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let enter_transition_background_color = self
            .background_color
            .unwrap_or_else(|| Theme::of(app, context).color_scheme.surface);
        let allow_snapshotting = self.allow_snapshotting;
        let allow_enter_route_snapshotting = self.allow_enter_route_snapshotting;
        let mut transition = DualTransitionBuilder::new(
            self.animation.clone(),
            move |_app, _context, animation, child| {
                ZoomEnterTransition {
                    animation,
                    reverse: false,
                    allow_snapshotting: allow_snapshotting && allow_enter_route_snapshotting,
                    background_color: enter_transition_background_color,
                    child: child.cloned(),
                }
                .into_widget()
            },
            move |_app, _context, animation, child| {
                ZoomExitTransition {
                    animation,
                    allow_snapshotting,
                    reverse: true,
                    child: child.cloned(),
                }
                .into_widget()
            },
        );
        transition.child = Some(
            ZoomPageTransitionsBuilder::snapshot_aware_delegated_transition(
                app,
                context,
                self.animation.clone(),
                self.secondary_animation.clone(),
                self.child.clone(),
                allow_snapshotting,
                allow_enter_route_snapshotting,
                Some(enter_transition_background_color),
            ),
        );
        transition.into_widget()
    }
}

#[derive(Debug)]
pub(crate) struct ZoomEnterTransition {
    pub(crate) animation: AnimationRef<f64>,
    child: Option<WidgetRef>,
    allow_snapshotting: bool,
    pub(crate) reverse: bool,
    background_color: Color,
}

impl StatefulWidget for ZoomEnterTransition {
    type State = ZoomEnterTransitionState;

    fn create_state(&self) -> ZoomEnterTransitionState {
        ZoomEnterTransitionState {
            state: StateData::new(),
            zoom_transition_base: ZoomTransitionBaseData::new(),
            delegate: None,
        }
    }
}

/// Dart's `_ZoomEnterTransitionState`.
pub(crate) struct ZoomEnterTransitionState {
    state: StateData<ZoomEnterTransition>,
    zoom_transition_base: ZoomTransitionBaseData,
    delegate: Option<Ref<ZoomEnterTransitionPainter>>,
}

impl ZoomEnterTransitionState {
    fn fade_in_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(0.0, 1.00).chain(ValueCurveTween::new(Rc::new(Interval::new(
            0.125,
            0.250,
            Curves::linear(),
        ))))
    }

    fn scale_down_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(1.10, 1.00).chain(ZoomPageTransition::scale_curve_sequence())
    }

    fn scale_up_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(0.85, 1.00).chain(ZoomPageTransition::scale_curve_sequence())
    }

    fn scrim_opacity_tween() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(0.0, 0.60).chain(ValueCurveTween::new(Rc::new(Interval::new(
            0.2075,
            0.4175,
            Curves::linear(),
        ))))
    }

    fn update_animations(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (reverse, animation) = (widget.reverse, widget.animation.clone());
        let fade_transition = if reverse {
            k_always_complete_animation(app)
        } else {
            ZoomEnterTransitionState::fade_in_transition().animate(app, animation.clone())
        };
        let scale_transition = if reverse {
            ZoomEnterTransitionState::scale_down_transition().animate(app, animation.clone())
        } else {
            ZoomEnterTransitionState::scale_up_transition().animate(app, animation.clone())
        };
        let base = self.zoom_transition_base_data_mut(app);
        base.fade_transition = Some(fade_transition);
        base.scale_transition = Some(scale_transition);

        animation.add_listener(
            app,
            Listener::handle_method(self, Self::on_animation_value_change),
        );
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
        );
    }

    fn new_delegate(self: Handle<Self>, app: &mut App) -> Ref<ZoomEnterTransitionPainter> {
        let widget = self.widget(app);
        let (reverse, animation, background_color) = (
            widget.reverse,
            widget.animation.clone(),
            widget.background_color,
        );
        let fade = self.fade_transition(app);
        let scale = self.scale_transition(app);
        ZoomEnterTransitionPainter::new(app, reverse, scale, fade, animation, background_color)
    }

    fn delegate(self: Handle<Self>, app: &App) -> Ref<ZoomEnterTransitionPainter> {
        app.get(self)
            .delegate
            .clone()
            .expect("created in init_state")
    }

    /// Dart's `delegate.dispose()`.
    fn dispose_delegate(self: Handle<Self>, app: &mut App) {
        let delegate = self.delegate(app);
        delegate.dispose(app);
    }
}

impl ZoomTransitionBase for ZoomEnterTransitionState {
    fn zoom_transition_base_data(self: Handle<Self>, app: &App) -> &ZoomTransitionBaseData {
        &app.get(self).zoom_transition_base
    }

    fn zoom_transition_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ZoomTransitionBaseData {
        &mut app.get_mut(self).zoom_transition_base
    }

    // See SnapshotWidget doc comment, this is disabled on web because the canvaskit backend uses
    // a single thread for UI and raster work which diminishes the impact of this performance
    // improvement.
    fn use_snapshot(self: Handle<Self>, app: &App) -> bool {
        !K_IS_WEB && self.widget(app).allow_snapshotting
    }
}

impl State for ZoomEnterTransitionState {
    type Widget = ZoomEnterTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_zoom_transition_base(app);
        self.update_animations(app);
        let delegate = self.new_delegate(app);
        app.get_mut(self).delegate = Some(delegate);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ZoomEnterTransition) {
        let widget = self.widget(app);
        if old_widget.reverse != widget.reverse || old_widget.animation != widget.animation {
            old_widget.animation.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_animation_value_change),
            );
            old_widget.animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
            );
            self.update_animations(app);
            self.dispose_delegate(app);
            let delegate = self.new_delegate(app);
            app.get_mut(self).delegate = Some(delegate);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        animation.remove_listener(
            app,
            &Listener::handle_method(self, Self::on_animation_value_change),
        );
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
        );
        self.dispose_delegate(app);
        ZoomTransitionBase::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        SnapshotWidget::new(self.controller(app), self.widget(app).child.clone())
            .painter(self.delegate(app).into_erased::<AnySnapshotPainter>())
            .mode(SnapshotMode::Permissive)
            .autoresize(true)
            .into_widget()
    }
}

#[derive(Debug)]
struct ZoomExitTransition {
    animation: AnimationRef<f64>,
    allow_snapshotting: bool,
    reverse: bool,
    child: Option<WidgetRef>,
}

impl StatefulWidget for ZoomExitTransition {
    type State = ZoomExitTransitionState;

    fn create_state(&self) -> ZoomExitTransitionState {
        ZoomExitTransitionState {
            state: StateData::new(),
            zoom_transition_base: ZoomTransitionBaseData::new(),
            delegate: None,
        }
    }
}

/// Dart's `_ZoomExitTransitionState`.
struct ZoomExitTransitionState {
    state: StateData<ZoomExitTransition>,
    zoom_transition_base: ZoomTransitionBaseData,
    delegate: Option<Ref<ZoomExitTransitionPainter>>,
}

impl ZoomExitTransitionState {
    fn fade_out_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(1.0, 0.0).chain(ValueCurveTween::new(Rc::new(Interval::new(
            0.0825,
            0.2075,
            Curves::linear(),
        ))))
    }

    fn scale_up_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(1.00, 1.05).chain(ZoomPageTransition::scale_curve_sequence())
    }

    fn scale_down_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(1.00, 0.90).chain(ZoomPageTransition::scale_curve_sequence())
    }

    fn update_animations(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (reverse, animation) = (widget.reverse, widget.animation.clone());
        let fade_transition = if reverse {
            ZoomExitTransitionState::fade_out_transition().animate(app, animation.clone())
        } else {
            k_always_complete_animation(app)
        };
        let scale_transition = if reverse {
            ZoomExitTransitionState::scale_down_transition().animate(app, animation.clone())
        } else {
            ZoomExitTransitionState::scale_up_transition().animate(app, animation.clone())
        };
        let base = self.zoom_transition_base_data_mut(app);
        base.fade_transition = Some(fade_transition);
        base.scale_transition = Some(scale_transition);

        animation.add_listener(
            app,
            Listener::handle_method(self, Self::on_animation_value_change),
        );
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
        );
    }

    fn new_delegate(self: Handle<Self>, app: &mut App) -> Ref<ZoomExitTransitionPainter> {
        let widget = self.widget(app);
        let (reverse, animation) = (widget.reverse, widget.animation.clone());
        let fade = self.fade_transition(app);
        let scale = self.scale_transition(app);
        ZoomExitTransitionPainter::new(app, reverse, scale, fade, animation)
    }

    fn delegate(self: Handle<Self>, app: &App) -> Ref<ZoomExitTransitionPainter> {
        app.get(self)
            .delegate
            .clone()
            .expect("created in init_state")
    }

    /// See [`ZoomEnterTransitionState::dispose_delegate`].
    fn dispose_delegate(self: Handle<Self>, app: &mut App) {
        let delegate = self.delegate(app);
        delegate.dispose(app);
    }
}

impl ZoomTransitionBase for ZoomExitTransitionState {
    fn zoom_transition_base_data(self: Handle<Self>, app: &App) -> &ZoomTransitionBaseData {
        &app.get(self).zoom_transition_base
    }

    fn zoom_transition_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ZoomTransitionBaseData {
        &mut app.get_mut(self).zoom_transition_base
    }

    // See SnapshotWidget doc comment, this is disabled on web because the canvaskit backend uses
    // a single thread for UI and raster work which diminishes the impact of this performance
    // improvement.
    fn use_snapshot(self: Handle<Self>, app: &App) -> bool {
        !K_IS_WEB && self.widget(app).allow_snapshotting
    }
}

impl State for ZoomExitTransitionState {
    type Widget = ZoomExitTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_zoom_transition_base(app);
        self.update_animations(app);
        let delegate = self.new_delegate(app);
        app.get_mut(self).delegate = Some(delegate);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ZoomExitTransition) {
        let widget = self.widget(app);
        if old_widget.reverse != widget.reverse || old_widget.animation != widget.animation {
            old_widget.animation.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_animation_value_change),
            );
            old_widget.animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
            );
            self.update_animations(app);
            self.dispose_delegate(app);
            let delegate = self.new_delegate(app);
            app.get_mut(self).delegate = Some(delegate);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        animation.remove_listener(
            app,
            &Listener::handle_method(self, Self::on_animation_value_change),
        );
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::on_animation_status_change),
        );
        self.dispose_delegate(app);
        ZoomTransitionBase::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        SnapshotWidget::new(self.controller(app), self.widget(app).child.clone())
            .painter(self.delegate(app).into_erased::<AnySnapshotPainter>())
            .mode(SnapshotMode::Permissive)
            .autoresize(true)
            .into_widget()
    }
}

// This transition slides a new page in from right to left while fading it in, and simultaneously
// slides the previous page out to the left while fading it out. This transition is designed to
// match the Android U activity transition.
#[derive(Debug)]
pub(crate) struct FadeForwardsPageTransition {
    animation: AnimationRef<f64>,

    secondary_animation: AnimationRef<f64>,

    /// Whether the [`SnapshotWidget`] will be used.
    ///
    /// When this value is true, performance is improved by disabling animations on both the
    /// outgoing and incoming route. This also implies that ink-splashes or similar animations
    /// will not animate during the transition.
    allow_snapshotting: bool,

    background_color: Option<Color>,

    child: Option<WidgetRef>,
}

impl FadeForwardsPageTransition {
    // The new page slides in from right to left.
    fn forward_translation_tween() -> impl Animatable<Offset> + Clone + 'static {
        ValueTween::new(Offset::new(0.25, 0.0), Offset::ZERO).chain(ValueCurveTween::new(
            FadeForwardsPageTransitionsBuilder::transition_curve(),
        ))
    }

    // The old page slides back from left to right.
    fn backward_translation_tween() -> impl Animatable<Offset> + Clone + 'static {
        ValueTween::new(Offset::ZERO, Offset::new(0.25, 0.0)).chain(ValueCurveTween::new(
            FadeForwardsPageTransitionsBuilder::transition_curve(),
        ))
    }
}

impl StatelessWidget for FadeForwardsPageTransition {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let allow_snapshotting = self.allow_snapshotting;
        let mut transition = DualTransitionBuilder::new(
            self.animation.clone(),
            move |app, _context, animation, child| {
                let opacity = FadeForwardsPageTransitionsBuilder::fade_in_transition()
                    .animate(app, animation.clone());
                let position = FadeForwardsPageTransition::forward_translation_tween()
                    .animate(app, animation.clone());

                (FadeTransition::new(opacity)
                    .child(SlideTransition::new(position).child(FadeForwardsSnapshot {
                        animation,
                        allow_snapshotting,
                        child: child.cloned(),
                    }))
                    .into_widget()) as _
            },
            move |app, _context, animation, child| {
                let opacity = FadeForwardsPageTransitionsBuilder::fade_out_transition()
                    .animate(app, animation.clone());
                let position = FadeForwardsPageTransition::backward_translation_tween()
                    .animate(app, animation.clone());

                (IgnorePointer::new()
                    .ignoring(animation.status(app) == AnimationStatus::Forward)
                    .child(FadeTransition::new(opacity).child(
                        SlideTransition::new(position).child(FadeForwardsSnapshot {
                            animation,
                            allow_snapshotting,
                            child: child.cloned(),
                        }),
                    ))
                    .into_widget()) as _
            },
        );
        transition.child = Some(
            FadeForwardsPageTransitionsBuilder::snapshot_aware_delegated_transition(
                app,
                context,
                self.secondary_animation.clone(),
                self.background_color,
                allow_snapshotting,
                self.child.clone(),
            ),
        );
        transition.into_widget()
    }
}

// Paints a snapshot of its child instead of the live child while `animation` is running, for
// performance purposes.
//
// The child is rasterized once at the start of the animation. On each following frame, the
// enclosing FadeTransition and SlideTransition only have to composite a single image, and
// because a single image draw can absorb the opacity of the parent OpacityLayer, the engine can
// skip the offscreen saveLayer that a partially transparent live page would otherwise require.
//
// This means that animations inside of the child (e.g. ink splashes or progress indicators) are
// frozen for the duration of the transition, which is the same tradeoff made by
// [`ZoomPageTransitionsBuilder`]. If the child contains a platform view, the live child is
// painted instead (see [`SnapshotMode::Permissive`]).
#[derive(Debug)]
pub(crate) struct FadeForwardsSnapshot {
    animation: AnimationRef<f64>,

    allow_snapshotting: bool,

    child: Option<WidgetRef>,
}

impl StatefulWidget for FadeForwardsSnapshot {
    type State = FadeForwardsSnapshotState;

    fn create_state(&self) -> FadeForwardsSnapshotState {
        FadeForwardsSnapshotState {
            state: StateData::new(),
            controller: None,
        }
    }
}

/// Dart's `_FadeForwardsSnapshotState`. The controller is Dart's field initialiser; it needs the
/// `App`, so `init_state` makes it before anything reads it.
pub(crate) struct FadeForwardsSnapshotState {
    state: StateData<FadeForwardsSnapshot>,
    controller: Option<Ref<SnapshotController>>,
}

impl FadeForwardsSnapshotState {
    /// Dart's `_controller`.
    pub(crate) fn controller(self: Handle<Self>, app: &App) -> Handle<SnapshotController> {
        app.get(self)
            .controller
            .handle()
            .expect("created in init_state")
    }

    // See SnapshotWidget doc comment, this is disabled on web because the canvaskit backend uses
    // a single thread for UI and raster work which diminishes the impact of this performance
    // improvement.
    fn use_snapshot(self: Handle<Self>, app: &App) -> bool {
        !K_IS_WEB && self.widget(app).allow_snapshotting
    }

    // Listen to value changes as well as status changes. DualTransitionBuilder may replace its
    // proxy animation's parent without a status notification, so a status listener alone can
    // miss the start of an animation.
    fn add_listeners(self: Handle<Self>, app: &mut App, animation: AnimationRef<f64>) {
        animation.add_listener(
            app,
            Listener::handle_method(self, Self::update_snapshotting),
        );
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::handle_status_change),
        );
    }

    fn remove_listeners(self: Handle<Self>, app: &mut App, animation: AnimationRef<f64>) {
        animation.remove_listener(
            app,
            &Listener::handle_method(self, Self::update_snapshotting),
        );
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::handle_status_change),
        );
    }

    fn handle_status_change(self: Handle<Self>, app: &mut App, _status: AnimationStatus) {
        self.update_snapshotting(app);
    }

    fn update_snapshotting(self: Handle<Self>, app: &mut App) {
        let allow_snapshotting =
            self.use_snapshot(app) && self.widget(app).animation.is_animating(app);
        self.controller(app)
            .set_allow_snapshotting(app, allow_snapshotting);
    }
}

impl State for FadeForwardsSnapshotState {
    type Widget = FadeForwardsSnapshot;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = SnapshotController::new(app, false);
        app.get_mut(self).controller = Some(controller);
        let animation = self.widget(app).animation.clone();
        self.add_listeners(app, animation);
        self.update_snapshotting(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &FadeForwardsSnapshot) {
        let animation = self.widget(app).animation.clone();
        if old_widget.animation != animation {
            self.remove_listeners(app, old_widget.animation.clone());
            self.add_listeners(app, animation);
        }
        self.update_snapshotting(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        self.remove_listeners(app, animation);
        self.controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let controller = app
            .get(self)
            .controller
            .clone()
            .expect("created in init_state");
        SnapshotWidget::new(controller, self.widget(app).child.clone())
            .mode(SnapshotMode::Permissive)
            .autoresize(true)
            .into_widget()
    }
}

/// Used by [`PageTransitionsTheme`] to define a horizontal `MaterialPageRoute` page transition
/// animation that looks like the default page transition used on Android U.
///
/// Use `allow_snapshotting` to enable snapshotting via [`SnapshotWidget`] for all routes. When
/// enabled, the child will be rasterized once at the start of the animation so that the
/// transitions only have to composite a single image. This often improves performance at the
/// expense of freezing any animations that exist within the child for the duration of the
/// animation. If the child contains a platform view, the live child is painted instead (see
/// [`SnapshotMode::Permissive`]).
///
/// See also:
///
///  * `FadeUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android O.
///  * `OpenUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android P.
///  * [`ZoomPageTransitionsBuilder`], which defines the default page transition that's similar
///    to the one provided in Android Q.
///  * [`CupertinoPageTransitionsBuilder`], which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * [`PredictiveBackPageTransitionsBuilder`], which defines a page transition that allows
///    peeking behind the current route on Android.
///  * `PageRoute::allow_snapshotting`, which enables or disables snapshotting on a per route
///    basis similar to the `allow_snapshotting` parameter.
///  * [`ZoomPageTransitionsBuilder::allow_snapshotting`], which is similar to the
///    `allow_snapshotting` parameter.
#[derive(Clone, Copy, Debug)]
pub struct FadeForwardsPageTransitionsBuilder {
    /// Whether FadeForwards page transitions will prefer to animate a snapshot of the entering
    /// and exiting routes.
    ///
    /// If not specified, defaults to true.
    ///
    /// When this value is true, FadeForwards page transitions will snapshot the entering and
    /// exiting routes. These snapshots are then animated in place of the underlying widgets to
    /// improve performance of the transition.
    ///
    /// Generally this means that animations that occur on the entering/exiting route while the
    /// route animation plays may appear frozen - unless they are a hero animation or something
    /// that is drawn in a separate overlay.
    pub allow_snapshotting: bool,

    /// The background color during transition between two routes.
    ///
    /// When a new page fades in and the old page fades out, this background color helps avoid a
    /// black background between two page.
    ///
    /// Defaults to `ColorScheme::surface`.
    pub background_color: Option<Color>,
}

impl Default for FadeForwardsPageTransitionsBuilder {
    fn default() -> FadeForwardsPageTransitionsBuilder {
        FadeForwardsPageTransitionsBuilder::new()
    }
}

impl FadeForwardsPageTransitionsBuilder {
    /// Constructs a page transition animation that matches the transition used on Android U.
    pub const fn new() -> FadeForwardsPageTransitionsBuilder {
        FadeForwardsPageTransitionsBuilder {
            allow_snapshotting: true,
            background_color: None,
        }
    }

    /// Dart `FadeForwardsPageTransitionsBuilder(allowSnapshotting:)`.
    pub const fn allow_snapshotting(
        mut self,
        allow_snapshotting: bool,
    ) -> FadeForwardsPageTransitionsBuilder {
        self.allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `FadeForwardsPageTransitionsBuilder(backgroundColor:)`.
    pub const fn background_color(
        mut self,
        background_color: Color,
    ) -> FadeForwardsPageTransitionsBuilder {
        self.background_color = Some(background_color);
        self
    }

    /// The value of `transition_duration` in milliseconds.
    ///
    /// Eyeballed on a physical Pixel 9 running Android 16. This does not match the actual value
    /// used by native Android, which is 800ms, because native Android is using Material 3
    /// Expressive springs that are not currently supported by Flutter. So for now at least,
    /// this is an approximation.
    pub const K_TRANSITION_MILLISECONDS: u64 = 450;

    // Used by all of the sliding transition animations.
    fn transition_curve() -> Rc<dyn Curve> {
        Curves::ease_in_out_cubic_emphasized()
    }

    // The previous page slides from right to left as the current page appears.
    fn secondary_backward_translation_tween() -> impl Animatable<Offset> + Clone + 'static {
        ValueTween::new(Offset::ZERO, Offset::new(-0.25, 0.0)).chain(ValueCurveTween::new(
            FadeForwardsPageTransitionsBuilder::transition_curve(),
        ))
    }

    // The previous page slides from left to right as the current page disappears.
    fn secondary_forward_translation_tween() -> impl Animatable<Offset> + Clone + 'static {
        ValueTween::new(Offset::new(-0.25, 0.0), Offset::ZERO).chain(ValueCurveTween::new(
            FadeForwardsPageTransitionsBuilder::transition_curve(),
        ))
    }

    // The fade in transition when the new page appears.
    fn fade_in_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(0.0, 1.0).chain(ValueCurveTween::new(Rc::new(Interval::new(
            0.0,
            0.75,
            Curves::linear(),
        ))))
    }

    // The fade out transition of the old page when the new page appears.
    fn fade_out_transition() -> impl Animatable<f64> + Clone + 'static {
        ValueTween::new(1.0, 0.0).chain(ValueCurveTween::new(Rc::new(Interval::new(
            0.0,
            0.25,
            Curves::linear(),
        ))))
    }

    // A transition builder that takes into account the snapshotting properties of
    // FadeForwardsPageTransitionsBuilder.
    fn snapshot_aware_delegated_transition(
        app: &mut App,
        context: BuildContext,
        secondary_animation: AnimationRef<f64>,
        background_color: Option<Color>,
        allow_snapshotting: bool,
        child: Option<WidgetRef>,
    ) -> WidgetRef {
        let reversed = ReverseAnimation::new(app, secondary_animation.clone()).into_erased();
        let mut builder = DualTransitionBuilder::new(
            reversed,
            move |app, _context, animation, child| {
                let opacity = FadeForwardsPageTransitionsBuilder::fade_in_transition()
                    .animate(app, animation.clone());
                let position =
                    FadeForwardsPageTransitionsBuilder::secondary_forward_translation_tween()
                        .animate(app, animation.clone());

                (FadeTransition::new(opacity)
                    .child(SlideTransition::new(position).child(FadeForwardsSnapshot {
                        animation,
                        allow_snapshotting,
                        child: child.cloned(),
                    }))
                    .into_widget()) as _
            },
            move |app, _context, animation, child| {
                let opacity = FadeForwardsPageTransitionsBuilder::fade_out_transition()
                    .animate(app, animation.clone());
                let position =
                    FadeForwardsPageTransitionsBuilder::secondary_backward_translation_tween()
                        .animate(app, animation.clone());

                (FadeTransition::new(opacity)
                    .child(SlideTransition::new(position).child(FadeForwardsSnapshot {
                        animation,
                        allow_snapshotting,
                        child: child.cloned(),
                    }))
                    .into_widget()) as _
            },
        );
        builder.child = child;
        let builder = builder.into_widget();

        let is_opaque = AnyModalRoute::opaque_of(app, context).unwrap_or(true);

        if !is_opaque {
            return builder;
        }

        let color = if secondary_animation.is_animating(app) {
            background_color.unwrap_or_else(|| ColorScheme::of(app, context).surface)
        } else {
            Colors::TRANSPARENT
        };
        ColoredBox::new(color).child(builder).into_widget()
    }
}

impl PageTransitionsBuilder for FadeForwardsPageTransitionsBuilder {
    fn transition_duration(&self) -> Duration {
        Duration::from_millis(FadeForwardsPageTransitionsBuilder::K_TRANSITION_MILLISECONDS)
    }

    fn delegated_transition(&self) -> Option<DelegatedTransitionBuilder> {
        let this = *self;
        Some(DelegatedTransitionBuilder::new(
            move |app, context, _animation, secondary_animation, allow_snapshotting, child| {
                Some(
                    FadeForwardsPageTransitionsBuilder::snapshot_aware_delegated_transition(
                        app,
                        context,
                        secondary_animation,
                        this.background_color,
                        !ZoomPageTransitionsBuilder::K_PROFILE_FORCE_DISABLE_SNAPSHOTTING
                            && allow_snapshotting
                            && this.allow_snapshotting,
                        child.cloned(),
                    ),
                )
            },
        ))
    }

    fn build_transitions(
        &self,
        app: &mut App,
        route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        FadeForwardsPageTransition {
            animation,
            secondary_animation,
            allow_snapshotting: !ZoomPageTransitionsBuilder::K_PROFILE_FORCE_DISABLE_SNAPSHOTTING
                && self.allow_snapshotting
                && route
                    .as_modal_route()
                    .as_transition_route()
                    .allow_snapshotting(app),
            background_color: self.background_color,
            child: Some(child),
        }
        .into_widget()
    }
}

/// Used by [`PageTransitionsTheme`] to define a zooming `MaterialPageRoute` page transition
/// animation that looks like the default page transition used on Android Q.
///
/// See also:
///
///  * `FadeUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android O.
///  * `OpenUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android P.
///  * [`CupertinoPageTransitionsBuilder`], which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * [`PredictiveBackPageTransitionsBuilder`], which defines a page transition that allows
///    peeking behind the current route on Android.
///  * [`FadeForwardsPageTransitionsBuilder`], which defines a page transition that's similar to
///    the one provided by Android U.
#[derive(Clone, Copy, Debug)]
pub struct ZoomPageTransitionsBuilder {
    /// Whether zoom page transitions will prefer to animate a snapshot of the entering and
    /// exiting routes.
    ///
    /// If not specified, defaults to true.
    ///
    /// When this value is true, zoom page transitions will snapshot the entering and exiting
    /// routes. These snapshots are then animated in place of the underlying widgets to improve
    /// performance of the transition.
    ///
    /// Generally this means that animations that occur on the entering/exiting route while the
    /// route animation plays may appear frozen - unless they are a hero animation or something
    /// that is drawn in a separate overlay.
    ///
    /// See also:
    ///
    ///  * `PageRoute::allow_snapshotting`, which enables or disables snapshotting on a per route
    ///    basis.
    pub allow_snapshotting: bool,

    /// Whether to enable snapshotting on the entering route during the transition animation.
    ///
    /// If not specified, defaults to true. If false, the route snapshotting will not be applied
    /// to the route being animating into, e.g. when transitioning from route A to route B, B
    /// will not be snapshotted.
    pub allow_enter_route_snapshotting: bool,

    /// The color of the scrim (background) that fades in and out during the transition.
    ///
    /// If not provided, defaults to current theme's `ColorScheme::surface` color.
    pub background_color: Option<Color>,
}

impl Default for ZoomPageTransitionsBuilder {
    fn default() -> ZoomPageTransitionsBuilder {
        ZoomPageTransitionsBuilder::new()
    }
}

impl ZoomPageTransitionsBuilder {
    /// Constructs a page transition animation that matches the transition used on Android Q.
    pub const fn new() -> ZoomPageTransitionsBuilder {
        ZoomPageTransitionsBuilder {
            allow_snapshotting: true,
            allow_enter_route_snapshotting: true,
            background_color: None,
        }
    }

    /// Dart `ZoomPageTransitionsBuilder(allowSnapshotting:)`.
    pub const fn allow_snapshotting(
        mut self,
        allow_snapshotting: bool,
    ) -> ZoomPageTransitionsBuilder {
        self.allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `ZoomPageTransitionsBuilder(allowEnterRouteSnapshotting:)`.
    pub const fn allow_enter_route_snapshotting(
        mut self,
        allow_enter_route_snapshotting: bool,
    ) -> ZoomPageTransitionsBuilder {
        self.allow_enter_route_snapshotting = allow_enter_route_snapshotting;
        self
    }

    /// Dart `ZoomPageTransitionsBuilder(backgroundColor:)`.
    pub const fn background_color(mut self, background_color: Color) -> ZoomPageTransitionsBuilder {
        self.background_color = Some(background_color);
        self
    }

    // Allows devicelab benchmarks to force disable the snapshotting. This is intended to allow us
    // to profile and fix the underlying performance issues for the Impeller backend. Dart reads
    // it from the define `flutter.benchmarks.force_disable_snapshot`; a build of Waft has no
    // such define, so it is false, as in every Flutter build that does not pass it.
    const K_PROFILE_FORCE_DISABLE_SNAPSHOTTING: bool = false;

    // A transition builder that takes into account the snapshotting properties of
    // ZoomPageTransitionsBuilder.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's positional arguments, in Dart's order"
    )]
    fn snapshot_aware_delegated_transition(
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: Option<WidgetRef>,
        allow_snapshotting: bool,
        allow_enter_route_snapshotting: bool,
        background_color: Option<Color>,
    ) -> WidgetRef {
        let _ = animation;
        let enter_transition_background_color =
            background_color.unwrap_or_else(|| Theme::of(app, context).color_scheme.surface);
        let reversed = ReverseAnimation::new(app, secondary_animation.clone()).into_erased();
        let mut builder = DualTransitionBuilder::new(
            reversed,
            move |_app, _context, animation, child| {
                ZoomEnterTransition {
                    animation,
                    allow_snapshotting: allow_snapshotting && allow_enter_route_snapshotting,
                    reverse: true,
                    background_color: enter_transition_background_color,
                    child: child.cloned(),
                }
                .into_widget()
            },
            move |_app, _context, animation, child| {
                ZoomExitTransition {
                    animation,
                    allow_snapshotting,
                    reverse: false,
                    child: child.cloned(),
                }
                .into_widget()
            },
        );
        builder.child = child;
        builder.into_widget()
    }
}

impl PageTransitionsBuilder for ZoomPageTransitionsBuilder {
    fn delegated_transition(&self) -> Option<DelegatedTransitionBuilder> {
        let this = *self;
        Some(DelegatedTransitionBuilder::new(
            move |app, context, animation, secondary_animation, allow_snapshotting, child| {
                Some(
                    ZoomPageTransitionsBuilder::snapshot_aware_delegated_transition(
                        app,
                        context,
                        animation,
                        secondary_animation,
                        child.cloned(),
                        allow_snapshotting && this.allow_snapshotting,
                        this.allow_enter_route_snapshotting,
                        this.background_color,
                    ),
                )
            },
        ))
    }

    fn build_transitions(
        &self,
        app: &mut App,
        route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        ZoomPageTransition {
            animation,
            secondary_animation,
            allow_snapshotting: self.allow_snapshotting
                && route
                    .as_modal_route()
                    .as_transition_route()
                    .allow_snapshotting(app),
            allow_enter_route_snapshotting: self.allow_enter_route_snapshotting,
            background_color: self.background_color,
            child: Some(child),
        }
        .into_widget()
    }
}

/// Dart's `TargetPlatform.values`, in Dart's order.
const TARGET_PLATFORM_VALUES: [TargetPlatform; 6] = [
    TargetPlatform::Android,
    TargetPlatform::Fuchsia,
    TargetPlatform::IOS,
    TargetPlatform::Linux,
    TargetPlatform::MacOS,
    TargetPlatform::Windows,
];

/// Defines the page transition animations used by `MaterialPageRoute` for different
/// [`TargetPlatform`]s.
///
/// The `MaterialPageRoute::build_transitions` method looks up the current
/// [`PageTransitionsTheme`] with `Theme::of(app, context).page_transitions_theme` and delegates
/// to [`build_transitions`](Self::build_transitions).
///
/// If a builder with a matching platform is not found, then the [`ZoomPageTransitionsBuilder`]
/// is used.
///
/// The builders are shared: a theme and its copies hold one map, and the default theme's map is
/// one per thread, as Dart's `const` map is one object, so two default themes compare equal.
///
/// See also:
///
///  * `ThemeData::page_transitions_theme`, which defines the default page transitions for the
///    overall theme.
///  * `FadeUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android O.
///  * `OpenUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android P.
///  * [`ZoomPageTransitionsBuilder`], which defines the default page transition that's similar
///    to the one provided by Android Q.
///  * [`FadeForwardsPageTransitionsBuilder`], which defines the default page transition that's
///    similar to the one provided by Android U.
///  * [`CupertinoPageTransitionsBuilder`], which defines a horizontal page transition that
///    matches native iOS page transitions.
#[derive(Clone)]
pub struct PageTransitionsTheme {
    builders: Rc<IndexMap<TargetPlatform, PageTransitionsBuilderRef>>,
}

impl Default for PageTransitionsTheme {
    fn default() -> PageTransitionsTheme {
        PageTransitionsTheme::new()
    }
}

impl PageTransitionsTheme {
    /// Constructs an object that selects a transition based on the platform.
    ///
    /// By default the list of builders is: [`PredictiveBackPageTransitionsBuilder`] for
    /// [`TargetPlatform::Android`], [`ZoomPageTransitionsBuilder`] for
    /// [`TargetPlatform::Windows`] and [`TargetPlatform::Linux`], and
    /// [`CupertinoPageTransitionsBuilder`] for [`TargetPlatform::IOS`] and
    /// [`TargetPlatform::MacOS`].
    pub fn new() -> PageTransitionsTheme {
        PageTransitionsTheme {
            builders: PageTransitionsTheme::default_builders(),
        }
    }

    /// Dart `PageTransitionsTheme(builders:)`.
    pub fn with_builders(
        mut self,
        builders: IndexMap<TargetPlatform, PageTransitionsBuilderRef>,
    ) -> PageTransitionsTheme {
        self.builders = Rc::new(builders);
        self
    }

    /// Dart's `_defaultBuilders`: one map per thread, whose builders are one per thread too,
    /// as each of Dart's `const` builders is one object.
    fn default_builders() -> Rc<IndexMap<TargetPlatform, PageTransitionsBuilderRef>> {
        thread_local! {
            static DEFAULT_BUILDERS: Rc<IndexMap<TargetPlatform, PageTransitionsBuilderRef>> = {
                let predictive_back =
                    PageTransitionsBuilderRef::new(PredictiveBackPageTransitionsBuilder::new());
                let cupertino =
                    PageTransitionsBuilderRef::new(CupertinoPageTransitionsBuilder::new());
                let zoom = PageTransitionsBuilderRef::new(ZoomPageTransitionsBuilder::new());
                Rc::new(IndexMap::from([
                    (TargetPlatform::Android, predictive_back),
                    (TargetPlatform::IOS, cupertino.clone()),
                    (TargetPlatform::MacOS, cupertino),
                    (TargetPlatform::Windows, zoom.clone()),
                    (TargetPlatform::Linux, zoom),
                ]))
            };
        }
        DEFAULT_BUILDERS.with(Rc::clone)
    }

    /// The [`PageTransitionsBuilder`]s supported by this theme.
    pub fn builders(&self) -> &IndexMap<TargetPlatform, PageTransitionsBuilderRef> {
        &self.builders
    }

    /// Delegates to the builder for the current `ThemeData::platform`. If a builder for the
    /// current platform is not found, then the [`ZoomPageTransitionsBuilder`] is used.
    ///
    /// `MaterialPageRoute::build_transitions` delegates to this method.
    pub fn build_transitions(
        &self,
        app: &mut App,
        route: AnyPageRoute,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = (app, context);
        PageTransitionsThemeTransitions {
            builders: Rc::clone(&self.builders),
            route,
            animation,
            secondary_animation,
            child,
        }
        .into_widget()
    }

    /// Provides the delegate transition for the target platform.
    ///
    /// See `ModalRoute::delegated_transition`.
    pub fn delegated_transition(
        &self,
        platform: TargetPlatform,
    ) -> Option<DelegatedTransitionBuilder> {
        match self.builders.get(&platform) {
            Some(matching_builder) => matching_builder.delegated_transition(),
            None => ZoomPageTransitionsBuilder::new().delegated_transition(),
        }
    }

    // Map the builders to a list with one PageTransitionsBuilder per platform for the operator
    // == overload.
    fn all(
        builders: &IndexMap<TargetPlatform, PageTransitionsBuilderRef>,
    ) -> Vec<Option<PageTransitionsBuilderRef>> {
        TARGET_PLATFORM_VALUES
            .iter()
            .map(|platform| builders.get(platform).cloned())
            .collect()
    }
}

impl PartialEq for PageTransitionsTheme {
    fn eq(&self, other: &PageTransitionsTheme) -> bool {
        if Rc::ptr_eq(&self.builders, &other.builders) {
            return true;
        }
        PageTransitionsTheme::all(&other.builders) == PageTransitionsTheme::all(&self.builders)
    }
}

impl Debug for PageTransitionsTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTransitionsTheme")
            .field("builders", &*self.builders)
            .finish()
    }
}

pub(crate) struct PageTransitionsThemeTransitions {
    builders: Rc<IndexMap<TargetPlatform, PageTransitionsBuilderRef>>,
    route: AnyPageRoute,
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    child: WidgetRef,
}

impl Debug for PageTransitionsThemeTransitions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_PageTransitionsThemeTransitions")
            .field("route", &self.route)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PageTransitionsThemeTransitions {
    type State = PageTransitionsThemeTransitionsState;

    fn create_state(&self) -> PageTransitionsThemeTransitionsState {
        PageTransitionsThemeTransitionsState {
            state: StateData::new(),
            transition_platform: None,
        }
    }
}

/// Dart's `_PageTransitionsThemeTransitionsState`.
pub(crate) struct PageTransitionsThemeTransitionsState {
    state: StateData<PageTransitionsThemeTransitions>,
    transition_platform: Option<TargetPlatform>,
}

impl State for PageTransitionsThemeTransitionsState {
    type Widget = PageTransitionsThemeTransitions;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut platform = Theme::of(app, context).platform;

        // If the theme platform is changed in the middle of a pop gesture, keep the transition
        // that the gesture began with until the gesture is finished.
        let route = self.widget(app).route;
        if route.as_modal_route().pop_gesture_in_progress(app) {
            let transition_platform = *app
                .get_mut(self)
                .transition_platform
                .get_or_insert(platform);
            platform = transition_platform;
        } else {
            app.get_mut(self).transition_platform = None;
        }

        let widget = self.widget(app);
        let (animation, secondary_animation, child) = (
            widget.animation.clone(),
            widget.secondary_animation.clone(),
            widget.child.clone(),
        );
        match widget.builders.get(&platform).cloned() {
            Some(matching_builder) => matching_builder.build_transitions(
                app,
                route,
                context,
                animation,
                secondary_animation,
                child,
            ),
            None => match platform {
                TargetPlatform::IOS => CupertinoPageTransitionsBuilder::new().build_transitions(
                    app,
                    route,
                    context,
                    animation,
                    secondary_animation,
                    child,
                ),
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Windows
                | TargetPlatform::MacOS
                | TargetPlatform::Linux => ZoomPageTransitionsBuilder::new().build_transitions(
                    app,
                    route,
                    context,
                    animation,
                    secondary_animation,
                    child,
                ),
            },
        }
    }
}

// Take an image and draw it centered and scaled. The image is already scaled by the
// `pixel_ratio`.
fn draw_image_scaled_and_centered(
    context: &mut PaintingContext,
    image: &Image,
    scale: f64,
    opacity: f64,
    pixel_ratio: f64,
) {
    if scale <= 0.0 || opacity <= 0.0 {
        return;
    }
    // Valo's default sampling, linear with linear mipmaps, is Dart's `FilterQuality.medium`.
    let sampling = Sampling::default();
    let paint = Paint::from_color(Color::from_rgbo(0, 0, 0, opacity).into());
    let logical_width = f64::from(image.width()) / pixel_ratio;
    let logical_height = f64::from(image.height()) / pixel_ratio;
    let scaled_logical_width = logical_width * scale;
    let scaled_logical_height = logical_height * scale;
    let left = (logical_width - scaled_logical_width) / 2.0;
    let top = (logical_height - scaled_logical_height) / 2.0;
    let dst = Rect::from_ltwh(left, top, scaled_logical_width, scaled_logical_height);
    context.canvas().draw_image_rect(
        image,
        Rect::from_ltwh(
            0.0,
            0.0,
            f64::from(image.width()),
            f64::from(image.height()),
        )
        .into(),
        dst.into(),
        sampling,
        &paint,
    );
}

fn update_scaled_transform(transform: &mut Matrix4, scale: f64, size: Size) {
    *transform = Matrix4::IDENTITY;
    if scale == 1.0 {
        return;
    }
    let s = scale as f32;
    *transform = transform.then(&Matrix4::from_flutter_array(&[
        s, 0.0, 0.0, 0.0, //
        0.0, s, 0.0, 0.0, //
        0.0, 0.0, s, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]));
    let dx = ((size.width() * scale) - size.width()) / 2.0;
    let dy = ((size.height() * scale) - size.height()) / 2.0;
    *transform = transform.then(&Matrix4::translation(-dx as f32, -dy as f32));
}

/// The fields Dart's `_ZoomTransitionBase` mixin declares; both zoom transition states carry
/// this bag under the field `zoom_transition_base`.
struct ZoomTransitionBaseData {
    // Don't rasterize if:
    // 1. Rasterization is disabled by the platform.
    // 2. The animation is paused/stopped.
    // 3. The values of the scale/fade transition do not
    //    benefit from rasterization.
    controller: Option<Ref<SnapshotController>>,

    fade_transition: Option<AnimationRef<f64>>,
    scale_transition: Option<AnimationRef<f64>>,
}

impl ZoomTransitionBaseData {
    fn new() -> ZoomTransitionBaseData {
        ZoomTransitionBaseData {
            controller: None,
            fade_transition: None,
            scale_transition: None,
        }
    }
}

/// Dart's `_ZoomTransitionBase` mixin on `State`.
trait ZoomTransitionBase: State {
    /// The mixin's fields, held under the field `zoom_transition_base`.
    fn zoom_transition_base_data(self: Handle<Self>, app: &App) -> &ZoomTransitionBaseData;

    /// See [`zoom_transition_base_data`](Self::zoom_transition_base_data).
    fn zoom_transition_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ZoomTransitionBaseData;

    fn use_snapshot(self: Handle<Self>, app: &App) -> bool;

    /// Dart's field initialiser of `controller`; it needs the `App`, so the state's
    /// `init_state` runs it first.
    fn init_zoom_transition_base(self: Handle<Self>, app: &mut App) {
        let controller = SnapshotController::new(app, false);
        self.zoom_transition_base_data_mut(app).controller = Some(controller);
    }

    fn controller(self: Handle<Self>, app: &App) -> Ref<SnapshotController> {
        self.zoom_transition_base_data(app)
            .controller
            .clone()
            .expect("created in init_state")
    }

    fn fade_transition(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        self.zoom_transition_base_data(app)
            .fade_transition
            .clone()
            .expect("set in init_state")
    }

    fn scale_transition(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        self.zoom_transition_base_data(app)
            .scale_transition
            .clone()
            .expect("set in init_state")
    }

    fn on_animation_value_change(self: Handle<Self>, app: &mut App) {
        let scale = self.scale_transition(app).value(app);
        let fade = self.fade_transition(app).value(app);
        let controller = self.controller(app);
        if scale == 1.0 && (fade == 0.0 || fade == 1.0) {
            controller.set_allow_snapshotting(app, false);
        } else {
            let use_snapshot = self.use_snapshot(app);
            controller.set_allow_snapshotting(app, use_snapshot);
        }
    }

    fn on_animation_status_change(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        let allow_snapshotting = status.is_animating() && self.use_snapshot(app);
        self.controller(app)
            .set_allow_snapshotting(app, allow_snapshotting);
    }

    /// Dart's `_ZoomTransitionBase.dispose`, which the state's `dispose` runs last.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.controller(app).dispose(app);
    }
}

struct ZoomEnterTransitionPainter {
    change_notifier: ChangeNotifierData,
    reverse: bool,
    animation: AnimationRef<f64>,
    scale: AnimationRef<f64>,
    fade: AnimationRef<f64>,
    background_color: Color,
    transform: Matrix4,
    opacity_handle: LayerHandle<Handle<OpacityLayer>>,
    transform_handler: LayerHandle<Handle<TransformLayer>>,
}

impl ZoomEnterTransitionPainter {
    fn new(
        app: &mut App,
        reverse: bool,
        scale: AnimationRef<f64>,
        fade: AnimationRef<f64>,
        animation: AnimationRef<f64>,
        background_color: Color,
    ) -> Ref<ZoomEnterTransitionPainter> {
        let this = app.create(ZoomEnterTransitionPainter {
            change_notifier: ChangeNotifierData::new(),
            reverse,
            animation: animation.clone(),
            scale: scale.clone(),
            fade: fade.clone(),
            background_color,
            transform: Matrix4::IDENTITY,
            opacity_handle: LayerHandle::new(),
            transform_handler: LayerHandle::new(),
        });
        let handle = this.handle();
        animation.add_listener(
            app,
            Listener::handle_method(handle, Handle::notify_listeners),
        );
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(handle, Self::on_status_change),
        );
        scale.add_listener(
            app,
            Listener::handle_method(handle, Handle::notify_listeners),
        );
        fade.add_listener(
            app,
            Listener::handle_method(handle, Handle::notify_listeners),
        );
        this
    }

    fn on_status_change(self: Handle<Self>, app: &mut App, _status: AnimationStatus) {
        self.notify_listeners(app);
    }

    fn draw_scrim(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
    ) {
        let mut scrim_opacity = 0.0;
        // The transition's scrim opacity only increases on the forward transition. In the
        // reverse transition, the opacity should always be 0.0.
        //
        // Therefore, we need to only apply the scrim opacity animation when the transition is
        // running forwards.
        //
        // The reason that we check that the animation's status is not `completed` instead of
        // checking that it is `forward` is that this allows the interrupted reversal of the
        // forward transition to smoothly fade the scrim away. This prevents a disjointed removal
        // of the scrim.
        let this = app.get(self);
        let (reverse, animation, background_color) =
            (this.reverse, this.animation.handle(), this.background_color);
        if !reverse && !animation.is_completed(app) {
            scrim_opacity =
                ZoomEnterTransitionState::scrim_opacity_tween().evaluate(app, animation);
        }
        debug_assert!(!reverse || scrim_opacity == 0.0);
        if scrim_opacity > 0.0 {
            // Dart's `withOpacity`, deprecated there too: its rounding to an 8-bit alpha is part
            // of the behaviour being ported.
            #[allow(deprecated)]
            let color = background_color.with_opacity(scrim_opacity);
            context
                .canvas()
                .draw_rect(offset & size, &Paint::from_color(color.into()));
        }
    }

    /// Dart's `dispose`: lets go of the animations and the layers, then `ChangeNotifier`'s.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let this = app.get(self);
        let (animation, scale, fade) = (
            this.animation.clone(),
            this.scale.clone(),
            this.fade.clone(),
        );
        animation.remove_listener(
            app,
            &Listener::handle_method(self, Handle::notify_listeners),
        );
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::on_status_change),
        );
        scale.remove_listener(
            app,
            &Listener::handle_method(self, Handle::notify_listeners),
        );
        fade.remove_listener(
            app,
            &Listener::handle_method(self, Handle::notify_listeners),
        );
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).opacity_handle, None);
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).transform_handler, None);
        ChangeNotifierData::dispose(app, self);
    }
}

impl ChangeNotifier for ZoomEnterTransitionPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl SnapshotPainter for ZoomEnterTransitionPainter {
    fn paint(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
    ) {
        if !app.get(self).animation.is_animating(app) {
            return painter(app, context, offset);
        }

        self.draw_scrim(app, context, offset, size);
        let scale = app.get(self).scale.value(app);
        update_scaled_transform(&mut app.get_mut(self).transform, scale, size);
        let transform = app.get(self).transform;
        let old_transform = app.get(self).transform_handler.layer();
        let layer = context.push_transform(
            app,
            true,
            offset,
            transform,
            |app, context, offset| {
                let alpha = (app.get(self).fade.value(app) * 255.0).round() as i32;
                let old_opacity = app.get(self).opacity_handle.layer();
                let opacity = context.push_opacity(app, offset, alpha, painter, old_opacity);
                LayerHandle::set_layer(
                    app,
                    |app| &mut app.get_mut(self).opacity_handle,
                    Some(opacity),
                );
            },
            old_transform,
        );
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).transform_handler, layer);
    }

    fn paint_snapshot(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        image: &Image,
        _source_size: Size,
        pixel_ratio: f64,
    ) {
        self.draw_scrim(app, context, offset, size);
        let this = app.get(self);
        let (scale, fade) = (this.scale.value(app), this.fade.value(app));
        draw_image_scaled_and_centered(context, image, scale, fade, pixel_ratio);
    }

    fn should_repaint(self: Handle<Self>, app: &App, old_painter: AnySnapshotPainter) -> bool {
        let old_delegate = app.get(
            old_painter
                .downcast::<ZoomEnterTransitionPainter>(app)
                .expect("the painter it replaces is a zoom enter painter"),
        );
        let this = app.get(self);
        old_delegate.reverse != this.reverse
            || old_delegate.animation.value(app) != this.animation.value(app)
            || old_delegate.scale.value(app) != this.scale.value(app)
            || old_delegate.fade.value(app) != this.fade.value(app)
    }
}

struct ZoomExitTransitionPainter {
    change_notifier: ChangeNotifierData,
    reverse: bool,
    scale: AnimationRef<f64>,
    fade: AnimationRef<f64>,
    animation: AnimationRef<f64>,
    transform: Matrix4,
    opacity_handle: LayerHandle<Handle<OpacityLayer>>,
    transform_handler: LayerHandle<Handle<TransformLayer>>,
}

impl ZoomExitTransitionPainter {
    fn new(
        app: &mut App,
        reverse: bool,
        scale: AnimationRef<f64>,
        fade: AnimationRef<f64>,
        animation: AnimationRef<f64>,
    ) -> Ref<ZoomExitTransitionPainter> {
        let this = app.create(ZoomExitTransitionPainter {
            change_notifier: ChangeNotifierData::new(),
            reverse,
            scale: scale.clone(),
            fade: fade.clone(),
            animation: animation.clone(),
            transform: Matrix4::IDENTITY,
            opacity_handle: LayerHandle::new(),
            transform_handler: LayerHandle::new(),
        });
        let handle = this.handle();
        scale.add_listener(
            app,
            Listener::handle_method(handle, Handle::notify_listeners),
        );
        fade.add_listener(
            app,
            Listener::handle_method(handle, Handle::notify_listeners),
        );
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(handle, Self::on_status_change),
        );
        this
    }

    fn on_status_change(self: Handle<Self>, app: &mut App, _status: AnimationStatus) {
        self.notify_listeners(app);
    }

    /// Dart's `dispose`: lets go of the layers and the animations, then `ChangeNotifier`'s.
    fn dispose(self: Handle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).opacity_handle, None);
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).transform_handler, None);
        let this = app.get(self);
        let (scale, fade, animation) = (
            this.scale.clone(),
            this.fade.clone(),
            this.animation.clone(),
        );
        scale.remove_listener(
            app,
            &Listener::handle_method(self, Handle::notify_listeners),
        );
        fade.remove_listener(
            app,
            &Listener::handle_method(self, Handle::notify_listeners),
        );
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::on_status_change),
        );
        ChangeNotifierData::dispose(app, self);
    }
}

impl ChangeNotifier for ZoomExitTransitionPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl SnapshotPainter for ZoomExitTransitionPainter {
    fn paint_snapshot(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        _offset: Offset,
        _size: Size,
        image: &Image,
        _source_size: Size,
        pixel_ratio: f64,
    ) {
        let this = app.get(self);
        let (scale, fade) = (this.scale.value(app), this.fade.value(app));
        draw_image_scaled_and_centered(context, image, scale, fade, pixel_ratio);
    }

    fn paint(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
    ) {
        if !app.get(self).animation.is_animating(app) {
            return painter(app, context, offset);
        }

        let scale = app.get(self).scale.value(app);
        update_scaled_transform(&mut app.get_mut(self).transform, scale, size);
        let transform = app.get(self).transform;
        let old_transform = app.get(self).transform_handler.layer();
        let layer = context.push_transform(
            app,
            true,
            offset,
            transform,
            |app, context, offset| {
                let alpha = (app.get(self).fade.value(app) * 255.0).round() as i32;
                let old_opacity = app.get(self).opacity_handle.layer();
                let opacity = context.push_opacity(app, offset, alpha, painter, old_opacity);
                LayerHandle::set_layer(
                    app,
                    |app| &mut app.get_mut(self).opacity_handle,
                    Some(opacity),
                );
            },
            old_transform,
        );
        LayerHandle::set_layer(app, |app| &mut app.get_mut(self).transform_handler, layer);
    }

    fn should_repaint(self: Handle<Self>, app: &App, old_painter: AnySnapshotPainter) -> bool {
        let old_delegate = app.get(
            old_painter
                .downcast::<ZoomExitTransitionPainter>(app)
                .expect("the painter it replaces is a zoom exit painter"),
        );
        let this = app.get(self);
        old_delegate.reverse != this.reverse
            || old_delegate.fade.value(app) != this.fade.value(app)
            || old_delegate.scale.value(app) != this.scale.value(app)
    }
}

#[cfg(test)]
mod tests {
    use waft_animation::{AnimationBehavior, AnimationController};
    use waft_cupertino::CupertinoPageTransition;
    use waft_foundation::Listener;
    use waft_rendering::AnyLayer;
    use waft_widgets::{
        AnyElement, Builder, Navigator, NavigatorState, PageRouteBuilder, SizedBox,
        StatefulBuilder, Text, WidgetBuilder,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::material::Material;
    use crate::page::MaterialPageRoute;
    use crate::test_support::{WidgetTester, material_page_app, with_main_thread_stack};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use waft_animation::AnyAnimation;
    use waft_test::{assert_no_new_objects, object_snapshot};

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    /// `tester.pumpWidget(MaterialApp(theme: theme(app), ..))` with the pages `configure` gives.
    fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        configure: impl FnOnce(MaterialApp) -> MaterialApp,
    ) {
        let root = {
            let theme = theme(&tester.cell.borrow());
            material_page_app(theme, configure)
        };
        tester.pump_widget(root);
    }

    /// A page with a `TextButton` showing `label` that runs `on_pressed` with the page's context.
    fn button_page(
        label: &'static str,
        on_pressed: impl Fn(&mut App, BuildContext) + 'static,
    ) -> WidgetBuilder {
        let on_pressed = Rc::new(on_pressed);
        Rc::new(move |_app, context| {
            let on_pressed = Rc::clone(&on_pressed);
            Material::new()
                .child(TextButton::new(
                    Some(Listener::new(move |app| on_pressed(app, context))),
                    Some(Text::new(label)),
                ))
                .into_widget()
        })
    }

    fn text_page(text: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| Text::new(text).into_widget())
    }

    /// The cases' `routes`: `/` a button 'push' that pushes `/b`, and `/b` the text 'page b'.
    fn push_routes() -> Vec<(String, WidgetBuilder)> {
        vec![
            (
                "/".to_string(),
                button_page("push", |app, context| {
                    Navigator::push_named(app, context, "/b", None);
                }),
            ),
            ("/b".to_string(), text_page("page b")),
        ]
    }

    /// `ThemeData(pageTransitionsTheme: PageTransitionsTheme(builders: {platform: builder}))`.
    fn theme_with(
        platform: TargetPlatform,
        builder: impl PageTransitionsBuilder,
    ) -> impl FnOnce(&App) -> ThemeData {
        move |app| {
            ThemeData::builder()
                .page_transitions_theme(PageTransitionsTheme::new().with_builders(IndexMap::from(
                    [(platform, PageTransitionsBuilderRef::new(builder))],
                )))
                .build(app)
        }
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = *tester
            .find_onstage_text(text)
            .first()
            .expect("the text is onstage");
        let center = tester.global_rect(tester.render_box_of(element)).center();
        tester.tap(center, 1);
    }

    fn count_onstage_text(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    /// Dart's `find.byType(W)` with its default `skipOffstage`.
    fn find_onstage_by_type<W: 'static>(tester: &WidgetTester) -> Vec<AnyElement> {
        tester
            .find_by_type::<W>()
            .into_iter()
            .filter(|element| tester.is_onstage(*element))
            .collect()
    }

    fn count_of<W: 'static>(tester: &WidgetTester) -> usize {
        find_onstage_by_type::<W>(tester).len()
    }

    fn navigator(tester: &WidgetTester) -> Handle<NavigatorState> {
        let element = *tester
            .find_by_type::<Navigator>()
            .first()
            .expect("a navigator");
        let app = tester.cell.borrow();
        element
            .state_handle::<NavigatorState>(&app)
            .expect("a navigator's state")
    }

    fn last_colored_box_color(tester: &WidgetTester) -> Color {
        let element = *find_onstage_by_type::<ColoredBox>(tester)
            .last()
            .expect("a ColoredBox");
        tester.widget::<ColoredBox, _>(element, |colored_box| colored_box.color.color())
    }

    #[test]
    fn the_default_page_transitions_theme_has_a_builder_for_every_platform_but_fuchsia() {
        let theme = PageTransitionsTheme::new();
        for platform in TARGET_PLATFORM_VALUES {
            match platform {
                TargetPlatform::Android
                | TargetPlatform::IOS
                | TargetPlatform::MacOS
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {
                    assert!(theme.builders().get(&platform).is_some(), "{platform:?}")
                }
                TargetPlatform::Fuchsia => {
                    assert!(theme.builders().get(&platform).is_none(), "{platform:?}")
                }
            }
        }
    }

    #[test]
    fn two_default_themes_are_equal_and_a_theme_with_other_builders_is_not() {
        assert_eq!(PageTransitionsTheme::new(), PageTransitionsTheme::default());
        let default_builders = PageTransitionsTheme::new().builders().clone();
        assert_eq!(
            PageTransitionsTheme::new().with_builders(default_builders.clone()),
            PageTransitionsTheme::new()
        );
        let mut other_builders = default_builders;
        other_builders.insert(
            TargetPlatform::Android,
            PageTransitionsBuilderRef::new(ZoomPageTransitionsBuilder::new()),
        );
        assert_ne!(
            PageTransitionsTheme::new().with_builders(other_builders),
            PageTransitionsTheme::new()
        );
    }

    #[test]
    fn the_default_theme_builds_a_cupertino_page_transition_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                pump_app(&mut tester, ThemeData::new, |app| app.routes(push_routes()));
                assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);

                tap_text(&mut tester, "push");
                tester.pump_and_settle();
                assert_eq!(count_onstage_text(&tester, "page b"), 1);
                assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);
            });
        }
    }

    #[test]
    fn the_default_theme_builds_a_fade_forwards_page_transition_on_android() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(&mut tester, ThemeData::new, |app| app.routes(push_routes()));
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);
        });
    }

    /// The fade-forwards cases on the background: the last `ColoredBox` 400ms into the push.
    fn fade_forwards_background(builder: FadeForwardsPageTransitionsBuilder) -> Color {
        let mut color = Colors::TRANSPARENT;
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                move |app| {
                    let color_scheme = ThemeData::new(app)
                        .color_scheme
                        .with_surface(Colors::PINK.primary());
                    ThemeData::builder()
                        .page_transitions_theme(PageTransitionsTheme::new().with_builders(
                            IndexMap::from([(
                                TargetPlatform::Android,
                                PageTransitionsBuilderRef::new(builder),
                            )]),
                        ))
                        .color_scheme(color_scheme)
                        .build(app)
                },
                |app| app.routes(push_routes()),
            );
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump(Duration::from_millis(400));
            color = last_colored_box_color(&tester);

            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);
        });
        color
    }

    #[test]
    fn fade_forwards_paints_the_color_scheme_surface_behind_the_pages_by_default() {
        let color = fade_forwards_background(FadeForwardsPageTransitionsBuilder::new());
        assert_eq!(color, Colors::PINK.primary());
    }

    #[test]
    fn fade_forwards_paints_its_background_color_behind_the_pages() {
        let color = fade_forwards_background(
            FadeForwardsPageTransitionsBuilder::new()
                .background_color(Colors::LIGHT_GREEN.primary()),
        );
        assert_eq!(color, Colors::LIGHT_GREEN.primary());
    }

    /// flutter_test's `TestVSync`: vends ordinary tickers, in no view.
    struct TestVSync;

    impl waft_scheduler::TickerProvider for TestVSync {
        fn create_ticker(
            &self,
            app: &mut App,
            on_tick: waft_scheduler::TickerCallback,
        ) -> Ref<waft_scheduler::Ticker> {
            waft_scheduler::Ticker::new(app, on_tick, None)
        }
    }

    /// The fade-forwards delegated transition over `animation` and `secondary_animation` of a
    /// 100ms controller, around a `SizedBox`; answers the controller.
    fn delegated_fade_forwards(
        tester: &mut WidgetTester,
        animation: ValueTween<f64>,
        secondary_animation: ValueTween<f64>,
    ) -> Ref<AnimationController> {
        let controller = AnimationController::new(
            &mut tester.cell.borrow_mut(),
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        );
        let (animation, secondary_animation) = {
            let mut app = tester.cell.borrow_mut();
            let parent = controller.erase::<AnyAnimation<f64>>();
            (
                animation.animate(&mut app, parent.clone()),
                secondary_animation.animate(&mut app, parent),
            )
        };
        tester.pump_widget(
            Builder::new(move |app, context| {
                FadeForwardsPageTransitionsBuilder::new()
                    .delegated_transition()
                    .expect("a delegated transition")
                    .call(
                        app,
                        context,
                        animation.clone(),
                        secondary_animation.clone(),
                        false,
                        Some(&SizedBox::new().into_widget()),
                    )
                    .expect("a widget")
            })
            .into_widget(),
        );
        controller
    }

    /// The opacity of the `FadeTransition` nearest above the `SizedBox`: Dart's
    /// `findAncestorRenderObjectOfType<RenderAnimatedOpacity>()?.opacity.value`.
    fn opacity(tester: &WidgetTester) -> f64 {
        let sized_box = *tester
            .find_by_type::<SizedBox>()
            .first()
            .expect("the SizedBox");
        let fade = tester.ancestor_of_type::<FadeTransition>(sized_box);
        let app = tester.cell.borrow();
        tester.widget::<FadeTransition, _>(fade, |fade| fade.opacity.value(&app))
    }

    #[test]
    fn the_fade_forwards_delegated_transition_fades_out_during_a_forward_secondary_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let controller = delegated_fade_forwards(
                &mut tester,
                ValueTween::new(1.0, 0.0),
                ValueTween::new(0.0, 1.0),
            );

            // Since secondary animation is forward, transition will be reverse between duration
            // 0 to 0.25.
            controller.set_value(&mut tester.cell.borrow_mut(), 0.0);
            tester.pump(Duration::ZERO);
            assert_eq!(opacity(&tester), 1.0);

            controller.set_value(&mut tester.cell.borrow_mut(), 0.25);
            tester.pump(Duration::ZERO);
            assert_eq!(opacity(&tester), 0.0);
        });
    }

    #[test]
    fn the_fade_forwards_delegated_transition_fades_in_during_a_reverse_secondary_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let controller = delegated_fade_forwards(
                &mut tester,
                ValueTween::new(0.0, 1.0),
                ValueTween::new(1.0, 0.0),
            );

            // Since secondary animation is reverse, transition will be forward between duration
            // 0.75 to 1.0.
            controller.set_value(&mut tester.cell.borrow_mut(), 0.75);
            tester.pump(Duration::ZERO);
            assert_eq!(opacity(&tester), 0.0);

            controller.set_value(&mut tester.cell.borrow_mut(), 1.0);
            tester.pump(Duration::ZERO);
            assert_eq!(opacity(&tester), 1.0);
        });
    }

    #[test]
    fn fade_forwards_paints_no_background_under_a_route_that_is_not_opaque() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let light_green = Colors::LIGHT_GREEN.primary();
            let push_b = Rc::new(|app: &mut App, context: BuildContext| {
                let route = MaterialPageRoute::new(app, text_page("page b"));
                Navigator::of(app, context, false).push(app, route.erase());
            });
            let home = button_page("push a", move |app, context| {
                let push_b = Rc::clone(&push_b);
                let page = button_page("push b", move |app, _page_context| push_b(app, context));
                let route = PageRouteBuilder::new(
                    app,
                    Rc::new(move |app, page_context, _animation, _secondary| {
                        page(app, page_context)
                    }),
                )
                .opaque(app, false);
                Navigator::of(app, context, false).push(app, route.erase());
            });
            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    FadeForwardsPageTransitionsBuilder::new().background_color(light_green),
                ),
                move |app| app.routes([("/".to_string(), home)]),
            );
            let no_light_green_box = |tester: &WidgetTester| {
                find_onstage_by_type::<ColoredBox>(tester)
                    .into_iter()
                    .all(|element| {
                        tester.widget::<ColoredBox, _>(element, |colored_box| {
                            colored_box.color.color() != light_green
                        })
                    })
            };

            tap_text(&mut tester, "push a");
            tester.pump_and_settle();

            tap_text(&mut tester, "push b");
            tester.pump(Duration::from_millis(400));

            // Check that ColoredBox is not used for non-opaque route.
            assert!(no_light_green_box(&tester));

            tester.pump_and_settle();

            let page_b = *tester.find_onstage_text("page b").first().expect("page b");
            Navigator::pop(&mut tester.cell.borrow_mut(), page_b, None);

            tester.pump_and_settle_by(Duration::from_millis(400));

            // Check that ColoredBox is not used for non-opaque route
            assert!(no_light_green_box(&tester));
        });
    }

    /// Not a Dart case: Dart's collector frees the transitions and the painter a zoom state's
    /// `_updateAnimations` replaces; here they are freed with their last reference.
    #[test]
    fn a_zoom_transition_given_a_new_animation_leaves_no_new_arena_objects() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let [first, second] = [0.25, 0.5].map(|value| {
                tester
                    .cell
                    .borrow_mut()
                    .create(waft_animation::AlwaysStoppedAnimation::new(value))
                    .into_erased::<AnyAnimation<f64>>()
            });
            let zoom = |animation: AnimationRef<f64>, reverse: bool| -> WidgetRef {
                waft_widgets::Directionality::new(
                    waft_embedder::TextDirection::Ltr,
                    waft_widgets::Column::new().children([
                        ZoomEnterTransition {
                            animation: animation.clone(),
                            child: Some(SizedBox::new().into_widget()),
                            allow_snapshotting: true,
                            reverse,
                            background_color: Colors::BLACK,
                        }
                        .into_widget(),
                        ZoomExitTransition {
                            animation: animation.clone(),
                            allow_snapshotting: true,
                            reverse,
                            child: Some(SizedBox::new().into_widget()),
                        }
                        .into_widget(),
                    ]),
                )
                .into_widget()
            };
            for reverse in [false, true] {
                tester.pump_widget(zoom(first.clone(), reverse));
                tester.pump_widget(zoom(second.clone(), reverse));
                let snapshot = object_snapshot(&tester.cell.borrow());
                tester.pump_widget(zoom(first.clone(), reverse));
                tester.pump_widget(zoom(second.clone(), reverse));
                tester.cell.checkpoint();
                assert_no_new_objects(&tester.cell.borrow(), &snapshot);
            }
        });
    }

    /// Checks that the frame `at` into a push, and the one `at` into the pop that follows, leave
    /// no new arena objects with `builder`: Dart's collector frees what each frame's build
    /// replaces. `at` lies between the bounds of the intervals the transitions fade over, so the
    /// frame adds and drops no layer.
    fn assert_mid_transition_frames_leave_nothing(
        platform: TargetPlatform,
        builder: impl PageTransitionsBuilder,
        at: Duration,
    ) {
        let mut tester = tester_on(platform);
        pump_app(&mut tester, theme_with(platform, builder), |app| {
            app.routes(push_routes())
        });
        let assert_a_frame_leaves_nothing = |tester: &mut WidgetTester| {
            tester.pump(Duration::ZERO);
            tester.pump(at);
            let snapshot = object_snapshot(&tester.cell.borrow());
            tester.pump(Duration::from_millis(16));
            tester.cell.checkpoint();
            assert_no_new_objects(&tester.cell.borrow(), &snapshot);
        };
        tap_text(&mut tester, "push");
        assert_a_frame_leaves_nothing(&mut tester);
        tester.pump_and_settle();
        let page_b = *tester.find_onstage_text("page b").first().expect("page b");
        Navigator::pop(&mut tester.cell.borrow_mut(), page_b, None);
        assert_a_frame_leaves_nothing(&mut tester);
    }

    /// Not a Dart case: see [`assert_mid_transition_frames_leave_nothing`].
    #[test]
    fn a_frame_of_a_zoom_push_or_pop_leaves_no_new_arena_objects() {
        with_main_thread_stack(|| {
            assert_mid_transition_frames_leave_nothing(
                TargetPlatform::Android,
                ZoomPageTransitionsBuilder::new(),
                Duration::from_millis(100),
            );
        });
    }

    /// Not a Dart case: see [`assert_mid_transition_frames_leave_nothing`].
    #[test]
    fn a_frame_of_a_fade_forwards_push_or_pop_leaves_no_new_arena_objects() {
        with_main_thread_stack(|| {
            assert_mid_transition_frames_leave_nothing(
                TargetPlatform::Android,
                FadeForwardsPageTransitionsBuilder::new(),
                Duration::from_millis(200),
            );
        });
    }

    #[test]
    fn the_fade_forwards_transition_lasts_450_ms() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    FadeForwardsPageTransitionsBuilder::new(),
                ),
                |app| app.routes(push_routes()),
            );
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump(Duration::from_millis(799));
            assert_eq!(count_onstage_text(&tester, "page b"), 0);
            // Color is not transparent during animation.
            assert_ne!(last_colored_box_color(&tester), Colors::TRANSPARENT);

            tester.pump(Duration::from_millis(801));
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            // Color is transparent after the animation.
            assert_eq!(last_colored_box_color(&tester), Colors::TRANSPARENT);
        });
    }

    #[test]
    fn the_cupertino_page_transitions_builder_lasts_500_ms() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::IOS);
            pump_app(
                &mut tester,
                theme_with(TargetPlatform::IOS, CupertinoPageTransitionsBuilder::new()),
                |app| app.routes(push_routes()),
            );
            assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(499));
            let running = |tester: &WidgetTester| {
                waft_scheduler::SchedulerBinding::transient_callback_count(
                    &mut tester.cell.borrow_mut(),
                ) > 0
            };
            assert!(running(&tester));

            tester.pump(Duration::from_millis(10));
            assert!(!running(&tester));
        });
    }

    #[test]
    fn the_route_durations_follow_a_change_of_the_page_transitions_builder() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let routes = || {
                vec![
                    (
                        "/".to_string(),
                        button_page("push", |app, context| {
                            Navigator::push_named(app, context, "/b", None);
                        }),
                    ),
                    (
                        "/b".to_string(),
                        Rc::new(|_app: &mut App, context: BuildContext| {
                            Material::new()
                                .child(
                                    waft_widgets::Column::new().children([
                                        TextButton::new(
                                            Some(Listener::new(move |app| {
                                                Navigator::pop(app, context, None)
                                            })),
                                            Some(Text::new("pop")),
                                        )
                                        .into_widget(),
                                        Text::new("page b").into_widget(),
                                    ]),
                                )
                                .into_widget()
                        }) as WidgetBuilder,
                    ),
                ]
            };
            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    FadeForwardsPageTransitionsBuilder::new(),
                ),
                move |app| app.routes(routes()),
            );
            assert_eq!(count_of::<FadeForwardsPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump(Duration::from_millis(799));
            assert_eq!(count_onstage_text(&tester, "page b"), 0);
            // The color is not transparent during animation.
            assert_ne!(last_colored_box_color(&tester), Colors::TRANSPARENT);

            tester.pump(Duration::from_millis(801));
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            // The color is transparent after the animation.
            assert_eq!(last_colored_box_color(&tester), Colors::TRANSPARENT);

            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    waft_widgets::FadeUpwardsPageTransitionsBuilder::new(),
                ),
                move |app| app.routes(routes()),
            );
            tester.pump_and_settle();
            assert_eq!(
                tester
                    .elements()
                    .into_iter()
                    .filter(|element| tester.is_onstage(*element))
                    .filter(
                        |element| format!("{:?}", element.widget(&tester.cell.borrow()))
                            .starts_with("_FadeUpwardsPageTransition")
                    )
                    .count(),
                1
            );
            tap_text(&mut tester, "pop");
            tester.pump(Duration::from_millis(299));
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            // ColoredBox doesn't exist in FadeUpwardsPageTransition.
            assert_eq!(count_of::<ColoredBox>(&tester), 0);

            tester.pump(Duration::from_millis(301));
            assert_eq!(count_onstage_text(&tester, "page b"), 0);
            // The first page.
            assert_eq!(count_onstage_text(&tester, "push"), 1);
            assert_eq!(count_of::<ColoredBox>(&tester), 0);
        });
    }

    #[test]
    fn a_page_transitions_theme_override_builds_a_cupertino_page_transition_on_android() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    CupertinoPageTransitionsBuilder::new(),
                ),
                |app| app.routes(push_routes()),
            );
            assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);
        });
    }

    #[test]
    fn a_cupertino_page_transition_on_android_does_not_block_gestures_after_a_back_swipe() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                theme_with(
                    TargetPlatform::Android,
                    CupertinoPageTransitionsBuilder::new(),
                ),
                |app| app.routes(push_routes()),
            );
            assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);

            tap_text(&mut tester, "push");
            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
            assert_eq!(count_of::<CupertinoPageTransition>(&tester), 1);

            tester.pump_and_settle_by(Duration::from_secs(60));

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(5.0, 100.0));
            tester.move_to(&mut gesture, Offset::new(405.0, 100.0), true);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);

            tester.pump_and_settle_by(Duration::from_secs(60));

            assert_eq!(count_onstage_text(&tester, "push"), 1);
            tap_text(&mut tester, "push");
            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
        });
    }

    /// The zoom snapshotting cases' app: page 1 and page 2, the second route with
    /// `second_route_allow_snapshotting`, under a zoom builder with `theme_allow_snapshotting`.
    fn zoom_snapshotting_app(
        tester: &mut WidgetTester,
        theme_allow_snapshotting: bool,
        second_route_allow_snapshotting: bool,
    ) {
        pump_app(
            tester,
            theme_with(
                TargetPlatform::Android,
                ZoomPageTransitionsBuilder::new().allow_snapshotting(theme_allow_snapshotting),
            ),
            move |app| {
                app.on_generate_route(move |app, settings| {
                    if settings.name.as_deref() == Some("/") {
                        return Some(
                            (MaterialPageRoute::new(
                                app,
                                Rc::new(|_app, _context| {
                                    Material::new().child(Text::new("Page 1")).into_widget()
                                }),
                            ))
                            .into_erased(),
                        );
                    }
                    Some(
                        (MaterialPageRoute::new(
                            app,
                            Rc::new(|_app, _context| {
                                Material::new().child(Text::new("Page 2")).into_widget()
                            }),
                        )
                        .allow_snapshotting(app, second_route_allow_snapshotting))
                        .into_erased(),
                    )
                })
            },
        );
    }

    /// Whether a `SnapshotWidget` above `text` is set to take a snapshot: Dart's
    /// `isTransitioningWithSnapshotting` reads that from the layers a snapshot leaves out of the
    /// route's layer tree, and Waft's `SnapshotWidget` paints the child live, so the case reads
    /// the controllers.
    fn is_transitioning_with_snapshotting(tester: &WidgetTester, text: &str) -> bool {
        is_snapshotting(tester, text)
    }

    #[test]
    fn zoom_page_transitions_snapshot_both_routes_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            zoom_snapshotting_app(&mut tester, true, true);

            // Transitioning from page 1 to page 2.
            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            // Exiting route should be snapshotted.
            assert!(is_transitioning_with_snapshotting(&tester, "Page 1"));

            // Entering route should be snapshotted.
            assert!(is_transitioning_with_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();

            // Transitioning back from page 2 to page 1.
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            // Exiting route should be snapshotted.
            assert!(is_transitioning_with_snapshotting(&tester, "Page 2"));

            // Entering route should be snapshotted.
            assert!(is_transitioning_with_snapshotting(&tester, "Page 1"));
        });
    }

    #[test]
    fn zoom_page_transitions_allow_snapshotting_false_snapshots_neither_route() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            zoom_snapshotting_app(&mut tester, false, true);

            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            assert!(!is_transitioning_with_snapshotting(&tester, "Page 1"));
            assert!(!is_transitioning_with_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();

            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            assert!(!is_transitioning_with_snapshotting(&tester, "Page 2"));
            assert!(!is_transitioning_with_snapshotting(&tester, "Page 1"));
        });
    }

    #[test]
    fn a_route_allow_snapshotting_false_overrides_the_zoom_builders_true() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            zoom_snapshotting_app(&mut tester, true, false);

            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            // First route should be snapshotted.
            assert!(is_transitioning_with_snapshotting(&tester, "Page 1"));

            // Second route should not be snapshotted.
            assert!(!is_transitioning_with_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();
        });
    }

    #[test]
    fn the_zoom_page_transition_builds_the_page_once() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let built_count = Rc::new(std::cell::Cell::new(0));
            let routes = {
                let built_count = Rc::clone(&built_count);
                vec![
                    (
                        "/".to_string(),
                        button_page("push", |app, context| {
                            Navigator::push_named(app, context, "/b", None);
                        }),
                    ),
                    (
                        "/b".to_string(),
                        Rc::new(move |_app: &mut App, _context: BuildContext| {
                            let built_count = Rc::clone(&built_count);
                            StatefulBuilder::new(move |_app, context, _set_state| {
                                // Increase the count each time the widget builds.
                                built_count.set(built_count.get() + 1);
                                TextButton::new(
                                    Some(Listener::new(move |app| {
                                        Navigator::pop(app, context, None)
                                    })),
                                    Some(Text::new("pop")),
                                )
                                .into_widget()
                            })
                            .into_widget()
                        }) as WidgetBuilder,
                    ),
                ]
            };
            pump_app(
                &mut tester,
                theme_with(TargetPlatform::Android, ZoomPageTransitionsBuilder::new()),
                move |app| app.routes(routes),
            );

            // No matter push or pop was called, the child widget should built only once.
            tap_text(&mut tester, "push");
            tester.pump_and_settle();
            assert_eq!(built_count.get(), 1);

            tap_text(&mut tester, "pop");
            tester.pump_and_settle();
            assert_eq!(built_count.get(), 1);
        });
    }

    /// The layers under the `SnapshotWidget` nearest above `text`: Dart's
    /// `tester.layerListOf(find.ancestor(of: text, matching: find.byType(SnapshotWidget)).first)`.
    fn layers_above(tester: &WidgetTester, text: &str) -> Vec<AnyLayer> {
        let text = *tester.find_text(text).first().expect("the text");
        let snapshot: AnyElement = tester.ancestor_of_type::<SnapshotWidget>(text);
        tester.layer_list_of(snapshot)
    }

    /// The scale of the first `TransformLayer` in `layers`.
    fn first_scale(tester: &WidgetTester, layers: &[AnyLayer]) -> f32 {
        let app = tester.cell.borrow();
        let transform = layers
            .iter()
            .find_map(|layer| app.handle::<TransformLayer>(layer.id()))
            .expect("a transform layer")
            .transform(&app)
            .expect("a transform");
        transform.to_flutter_array()[0]
    }

    /// The alpha of the first `OpacityLayer` in `layers`.
    fn first_alpha(tester: &WidgetTester, layers: &[AnyLayer]) -> i32 {
        let app = tester.cell.borrow();
        layers
            .iter()
            .find_map(|layer| app.handle::<OpacityLayer>(layer.id()))
            .expect("an opacity layer")
            .alpha(&app)
            .expect("an alpha")
    }

    // page_test 'test page transition (_ZoomPageTransition) without rasterization'.
    #[test]
    fn the_zoom_page_transition_scales_and_fades_the_pages_mid_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                theme_with(TargetPlatform::Android, ZoomPageTransitionsBuilder::new()),
                |app| {
                    app.on_generate_route(|app, settings| {
                        let first = settings.name.as_deref() == Some("/");
                        Some(
                            (MaterialPageRoute::new(
                                app,
                                Rc::new(move |_app, _context| {
                                    Material::new()
                                        .child(Text::new(if first { "Page 1" } else { "Page 2" }))
                                        .into_widget()
                                }),
                            )
                            .allow_snapshotting(app, false))
                            .into_erased(),
                        )
                    })
                },
            );

            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/next", None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            let page_1 = layers_above(&tester, "Page 1");
            let page_2 = layers_above(&tester, "Page 2");
            // Page 1 is enlarging, starts from 1.0.
            assert!(first_scale(&tester, &page_1) > 1.0);
            // Page 2 is enlarging from the value less than 1.0.
            assert!(first_scale(&tester, &page_2) < 1.0);
            // Page 2 is becoming none transparent.
            assert!(first_alpha(&tester, &page_2) < 255);

            tester.pump(Duration::from_millis(250));
            tester.pump(Duration::from_millis(1));

            // Page 2 covers page 1.
            assert_eq!(count_onstage_text(&tester, "Page 1"), 0);
            assert_eq!(count_onstage_text(&tester, "Page 2"), 1);

            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(100));

            let page_1 = layers_above(&tester, "Page 1");
            let page_2 = layers_above(&tester, "Page 2");
            // Page 1 is narrowing down, but still larger than 1.0.
            assert!(first_scale(&tester, &page_1) > 1.0);
            // Page 2 is smaller than 1.0.
            assert!(first_scale(&tester, &page_2) < 1.0);
            // Page 2 is becoming transparent.
            assert!(first_alpha(&tester, &page_2) < 255);

            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(1));

            assert_eq!(count_onstage_text(&tester, "Page 1"), 1);
            assert_eq!(count_onstage_text(&tester, "Page 2"), 0);
        });
    }

    /// The zoom scrim cases: the rect the entering page's zoom transition paints `at` into the
    /// push of a page whose builder is `page`, under a zoom builder of `builder` on `platform`
    /// with the colour scheme's surface `surface`.
    fn zoom_scrim_color(
        platform: TargetPlatform,
        builder: ZoomPageTransitionsBuilder,
        surface: Color,
        at: Duration,
    ) -> Vec<u32> {
        let mut drawn = Vec::new();
        with_main_thread_stack(|| {
            let mut tester = tester_on(platform);
            pump_app(
                &mut tester,
                move |app| {
                    let color_scheme = ThemeData::new(app).color_scheme.with_surface(surface);
                    ThemeData::builder()
                        .color_scheme(color_scheme)
                        .page_transitions_theme(
                            PageTransitionsTheme::new().with_builders(
                                TARGET_PLATFORM_VALUES
                                    .into_iter()
                                    .map(|platform| {
                                        (platform, PageTransitionsBuilderRef::new(builder))
                                    })
                                    .collect(),
                            ),
                        )
                        .build(app)
                },
                |app| app.routes(push_routes()),
            );

            tap_text(&mut tester, "push");
            tester.pump(Duration::ZERO);
            tester.pump(at);

            let enter = *tester
                .find_by_type::<ZoomEnterTransition>()
                .iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    tester.widget::<ZoomEnterTransition, _>(**element, |enter| {
                        !enter.reverse && enter.animation.is_animating(&app)
                    })
                })
                .expect("the entering page's zoom transition");
            let render = tester.render_box_of(enter).as_object();
            drawn = tester
                .painted(render)
                .into_iter()
                .filter_map(|drawn| match drawn {
                    crate::test_support::Drawn::Rect(color, _) => Some(color),
                    _ => None,
                })
                .collect();

            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "page b"), 1);
        });
        drawn
    }

    // 'ZoomPageTransitionsBuilder uses theme color during transition effects', on the page
    // without a scaffold.
    #[test]
    fn the_zoom_page_transition_scrim_is_the_color_scheme_surface() {
        let surface = Color::from_argb(255, 195, 255, 0);
        for platform in TARGET_PLATFORM_VALUES {
            let drawn = zoom_scrim_color(
                platform,
                ZoomPageTransitionsBuilder::new(),
                surface,
                Duration::from_millis(75),
            );
            // Expect the color to be at exactly 12.2% opacity at this time.
            #[allow(deprecated)]
            let expected = surface.with_opacity(0.122).to_argb32();
            assert!(drawn.contains(&expected), "{platform:?}: {drawn:x?}");
        }
    }

    #[test]
    fn the_zoom_page_transition_scrim_is_the_builders_background_color() {
        let red = Colors::RED.primary();
        for platform in TARGET_PLATFORM_VALUES {
            let drawn = zoom_scrim_color(
                platform,
                ZoomPageTransitionsBuilder::new().background_color(red),
                Colors::BLUE.primary(),
                Duration::from_millis(125),
            );
            // Expect the color to be at exactly 59.6% opacity at this time.
            #[allow(deprecated)]
            let expected = red.with_opacity(0.596).to_argb32();
            assert!(drawn.contains(&expected), "{platform:?}: {drawn:x?}");
        }
    }

    #[test]
    fn the_incoming_route_takes_input_during_a_fade_forwards_back_navigation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                move |app| {
                    let color_scheme = ThemeData::new(app)
                        .color_scheme
                        .with_surface(Colors::PINK.primary());
                    ThemeData::builder()
                        .page_transitions_theme(PageTransitionsTheme::new().with_builders(
                            IndexMap::from([(
                                TargetPlatform::Android,
                                PageTransitionsBuilderRef::new(
                                    FadeForwardsPageTransitionsBuilder::new(),
                                ),
                            )]),
                        ))
                        .color_scheme(color_scheme)
                        .build(app)
                },
                |app| {
                    app.routes([
                        (
                            "/".to_string(),
                            button_page("push", |app, context| {
                                Navigator::push_named(app, context, "/b", None);
                            }),
                        ),
                        (
                            "/b".to_string(),
                            button_page("go back", |app, context| {
                                Navigator::pop(app, context, None)
                            }),
                        ),
                    ])
                },
            );

            assert_eq!(count_onstage_text(&tester, "push"), 1);
            assert_eq!(count_onstage_text(&tester, "go back"), 0);

            // Go to the second route.
            tap_text(&mut tester, "push");
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(801));

            assert_eq!(count_onstage_text(&tester, "push"), 0);
            assert_eq!(count_onstage_text(&tester, "go back"), 1);

            // Tap to go back to the first route.
            tap_text(&mut tester, "go back");
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(400));

            assert_eq!(count_onstage_text(&tester, "push"), 1);
            assert_eq!(count_onstage_text(&tester, "go back"), 1);

            // In the middle of the transition, tap to go back to the second route.
            tap_text(&mut tester, "push");

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(401));

            assert_eq!(count_onstage_text(&tester, "push"), 1);
            assert_eq!(count_onstage_text(&tester, "go back"), 1);

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(400));

            assert_eq!(count_onstage_text(&tester, "push"), 0);
            assert_eq!(count_onstage_text(&tester, "go back"), 1);
        });
    }

    /// Whether any `SnapshotWidget` above `text` is set to take a snapshot.
    fn is_snapshotting(tester: &WidgetTester, text: &str) -> bool {
        let text = *tester.find_text(text).first().expect("the text");
        let ancestors = tester.ancestors(text);
        let app = tester.cell.borrow();
        ancestors.into_iter().any(|ancestor| {
            waft_widgets::downcast_widget::<SnapshotWidget>(&**ancestor.widget(&app))
                .is_some_and(|snapshot| snapshot.controller.allow_snapshotting(&app))
        })
    }

    fn fade_forwards_snapshotting_app(
        tester: &mut WidgetTester,
        builder_allow_snapshotting: bool,
        second_route_allow_snapshotting: bool,
    ) {
        pump_app(
            tester,
            theme_with(
                TargetPlatform::Android,
                FadeForwardsPageTransitionsBuilder::new()
                    .allow_snapshotting(builder_allow_snapshotting),
            ),
            move |app| {
                app.on_generate_route(move |app, settings| {
                    if settings.name.as_deref() == Some("/") {
                        return Some(
                            (MaterialPageRoute::new(
                                app,
                                Rc::new(|_app, _context| {
                                    Material::new().child(Text::new("Page 1")).into_widget()
                                }),
                            ))
                            .into_erased(),
                        );
                    }
                    Some(
                        (MaterialPageRoute::new(
                            app,
                            Rc::new(|_app, _context| {
                                Material::new().child(Text::new("Page 2")).into_widget()
                            }),
                        )
                        .allow_snapshotting(app, second_route_allow_snapshotting))
                        .into_erased(),
                    )
                })
            },
        );
    }

    /// A tenth of the fade-forwards transition.
    const A_TENTH_OF_THE_TRANSITION: Duration =
        Duration::from_millis(FadeForwardsPageTransitionsBuilder::K_TRANSITION_MILLISECONDS / 10);

    #[test]
    fn fade_forwards_snapshots_the_entering_and_exiting_routes_only_while_animating() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            fade_forwards_snapshotting_app(&mut tester, true, true);
            assert!(!is_snapshotting(&tester, "Page 1"));

            // Push. Part way into the transition, both pages are partially transparent.
            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(A_TENTH_OF_THE_TRANSITION);
            assert!(is_snapshotting(&tester, "Page 1"));
            assert!(is_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();
            assert!(!is_snapshotting(&tester, "Page 2"));

            // Pop.
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(A_TENTH_OF_THE_TRANSITION);
            assert!(is_snapshotting(&tester, "Page 1"));
            assert!(is_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();
            assert_eq!(count_onstage_text(&tester, "Page 2"), 0);
            assert!(!is_snapshotting(&tester, "Page 1"));
        });
    }

    #[test]
    fn a_route_allow_snapshotting_false_disables_fade_forwards_snapshotting_for_that_route_only() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            fade_forwards_snapshotting_app(&mut tester, true, false);

            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(A_TENTH_OF_THE_TRANSITION);
            assert!(is_snapshotting(&tester, "Page 1"));
            assert!(!is_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();
        });
    }

    #[test]
    fn fade_forwards_allow_snapshotting_false_disables_snapshotting_for_all_routes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            fade_forwards_snapshotting_app(&mut tester, false, true);

            // Push.
            let navigator = navigator(&tester);
            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(A_TENTH_OF_THE_TRANSITION);
            assert!(!is_snapshotting(&tester, "Page 1"));
            assert!(!is_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();

            // Pop.
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(A_TENTH_OF_THE_TRANSITION);
            assert!(!is_snapshotting(&tester, "Page 1"));
            assert!(!is_snapshotting(&tester, "Page 2"));

            tester.pump_and_settle();
        });
    }

    #[test]
    fn the_delegated_transition_of_the_theme_is_its_platform_builders() {
        let theme = PageTransitionsTheme::new();
        // The Cupertino builder's is a function, equal across calls.
        assert_eq!(
            theme.delegated_transition(TargetPlatform::IOS),
            CupertinoPageTransitionsBuilder::new().delegated_transition()
        );
        // The zoom and fade-forwards builders hand out a new closure on every call, as Dart's
        // getters do.
        let zoom = ZoomPageTransitionsBuilder::new();
        assert_ne!(zoom.delegated_transition(), zoom.delegated_transition());
        assert!(
            theme
                .delegated_transition(TargetPlatform::Fuchsia)
                .is_some()
        );
        assert!(
            PredictiveBackPageTransitionsBuilder::new()
                .delegated_transition()
                .is_none()
        );
    }

    #[test]
    fn the_zoom_painters_compare_their_animation_values_to_repaint() {
        let cell = crate::test_support::test_cell();
        let mut app = cell.borrow_mut();
        let controller = AnimationController::new(
            &mut app,
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        );
        let animation = controller.erase::<AnyAnimation<f64>>();
        let complete = k_always_complete_animation(&mut app);
        let first = ZoomExitTransitionPainter::new(
            &mut app,
            false,
            animation.clone(),
            complete.clone(),
            animation.clone(),
        );
        let second = ZoomExitTransitionPainter::new(
            &mut app,
            false,
            animation.clone(),
            complete.clone(),
            animation.clone(),
        );
        assert!(!second.should_repaint(&app, first.erase::<AnySnapshotPainter>().handle()));
        let reversed =
            ZoomExitTransitionPainter::new(&mut app, true, animation.clone(), complete, animation);
        assert!(reversed.should_repaint(&app, first.erase::<AnySnapshotPainter>().handle()));
    }

    #[test]
    fn a_zoom_enter_transition_that_flips_its_direction_repaints_with_a_new_painter() {
        let mut tester = tester_on(TargetPlatform::Android);
        let animation = {
            let mut app = tester.cell.borrow_mut();
            AnimationController::new(
                &mut app,
                None,
                Some(Duration::from_millis(100)),
                None,
                0.5,
                1.0,
                AnimationBehavior::Normal,
                TestVSync,
            )
            .into_erased::<AnyAnimation<f64>>()
        };
        let transition = |reverse| ZoomEnterTransition {
            animation: animation.clone(),
            child: Some(SizedBox::new().width(10.0).height(10.0).into_widget()),
            allow_snapshotting: true,
            reverse,
            background_color: Color::from_argb(0xFF, 0, 0, 0),
        };
        tester.pump_widget(transition(false).into_widget());
        tester.pump_widget(transition(true).into_widget());
    }
}
