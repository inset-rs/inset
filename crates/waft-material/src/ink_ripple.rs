//! Flutter counterpart: material_ui's `ink_ripple.dart`.
//!
//! Dart's file reads the deprecated `Color.alpha` and its channels, and so does this one.
#![allow(deprecated)]

use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationLocalListenersMixin,
    AnimationLocalStatusListenersMixin, AnimationStatus, AnimationStatusListener, AnyAnimation,
    CurveTween, Curves, IntTween, Interval, Tween,
};
use waft_embedder::{Canvas, Color, Matrix4, Offset, Paint, TextDirection};
use waft_foundation::{App, Handle, Listener};
use waft_painting::{BorderRadius, ShapeBorder};
use waft_rendering::AnyRenderBox;

use crate::ink_well::{
    AnyInteractiveInkFeature, InteractiveInkFeature, InteractiveInkFeatureData,
    InteractiveInkFeatureFactory, InteractiveInkFeatureFactoryRef, InteractiveInkFeatureLeaf,
    InteractiveInkFeatureLeafData, InteractiveInkFeatureObject,
};
use crate::material::{
    AnyMaterialInkController, InkFeature, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData,
    InkFeatureObject, RectCallback, destroy_lingering,
};

const K_UNCONFIRMED_RIPPLE_DURATION: Duration = Duration::from_secs(1);
const K_FADE_IN_DURATION: Duration = Duration::from_millis(75);
const K_RADIUS_DURATION: Duration = Duration::from_millis(225);
const K_FADE_OUT_DURATION: Duration = Duration::from_millis(375);
const K_CANCEL_DURATION: Duration = Duration::from_millis(75);

// The fade out begins 225ms after the _fadeOutController starts. See confirm().
const K_FADE_OUT_INTERVAL_START: f64 = 225.0 / 375.0;

fn get_clip_callback(
    reference_box: AnyRenderBox,
    contained_ink_well: bool,
    rect_callback: Option<RectCallback>,
) -> Option<RectCallback> {
    if let Some(rect_callback) = rect_callback {
        debug_assert!(contained_ink_well);
        return Some(rect_callback);
    }
    if contained_ink_well {
        return Some(Rc::new(move |app| Offset::ZERO & reference_box.size(app)));
    }
    None
}

fn get_target_radius(
    app: &App,
    reference_box: AnyRenderBox,
    _contained_ink_well: bool,
    rect_callback: Option<&RectCallback>,
    _position: Offset,
) -> f64 {
    let size = match rect_callback {
        Some(rect_callback) => rect_callback(app).size(),
        None => reference_box.size(app),
    };
    let d1 = size.bottom_right(Offset::ZERO).distance();
    let d2 = (size.top_right(Offset::ZERO) - size.bottom_left(Offset::ZERO)).distance();
    d1.max(d2) / 2.0
}

/// Dart's `_InkRippleFactory`.
#[derive(Debug)]
struct InkRippleFactory;

impl InteractiveInkFeatureFactory for InkRippleFactory {
    fn create(
        &self,
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        position: Offset,
        color: Color,
        text_direction: TextDirection,
        contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<Box<dyn ShapeBorder>>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> AnyInteractiveInkFeature {
        InkRipple::new(
            app,
            controller,
            reference_box,
            position,
            color,
            text_direction,
            contained_ink_well,
            rect_callback,
            border_radius,
            custom_border,
            radius,
            on_removed,
        )
        .as_interactive_ink_feature()
    }
}

/// A visual reaction on a piece of [`Material`](crate::Material) to user input.
///
/// A circular ink feature whose origin starts at the input touch point and
/// whose radius expands from 60% of the final radius. The splash origin
/// animates to the center of its `reference_box`.
///
/// This object is rarely created directly. Instead of creating an ink ripple,
/// consider using an [`InkResponse`](crate::InkResponse) or [`InkWell`](crate::InkWell)
/// widget, which uses gestures (such as tap and long-press) to trigger ink splashes. This
/// class is used when the [`ThemeData`](crate::ThemeData)'s splash factory is
/// [`InkRipple::splash_factory`].
///
/// See also:
///
///  * [`InkSplash`](crate::InkSplash), which is an ink splash feature that expands less
///    aggressively than the ripple.
///  * [`InkResponse`](crate::InkResponse), which uses gestures to trigger ink highlights and
///    ink splashes in the parent [`Material`](crate::Material).
///  * [`InkWell`](crate::InkWell), which is a rectangular [`InkResponse`](crate::InkResponse)
///    (the most common type of ink response).
///  * [`Material`](crate::Material), which is the widget on which the ink splash is painted.
///  * [`InkHighlight`](crate::InkHighlight), which is an ink feature that emphasizes a part of
///    a [`Material`](crate::Material).
pub struct InkRipple {
    ink_feature: InkFeatureData,
    interactive_ink_feature: InteractiveInkFeatureData,
    position: Offset,
    border_radius: BorderRadius,
    clip_callback: Option<RectCallback>,
    text_direction: TextDirection,
    radius: Option<AnyAnimation<f64>>,
    radius_controller: Option<Handle<AnimationController>>,
    fade_in: Option<AnyAnimation<i64>>,
    fade_in_controller: Option<Handle<AnimationController>>,
    fade_out: Option<AnyAnimation<i64>>,
    fade_out_controller: Option<Handle<AnimationController>>,
    /// The tweens and driven animations this ripple made, which its dispose frees.
    owned: Vec<waft_foundation::HandleId>,
}

impl InkRipple {
    /// Begin a ripple, centered at `position` relative to `reference_box`.
    ///
    /// The `controller` argument is typically obtained via `Material::of(app, context)`.
    ///
    /// If `contained_ink_well` is true, then the ripple will be sized to fit
    /// the well rectangle, then clipped to it when drawn. The well
    /// rectangle is the box returned by `rect_callback`, if provided, or
    /// otherwise is the bounds of the `reference_box`.
    ///
    /// If `contained_ink_well` is false, then `rect_callback` should be null.
    /// The ink ripple is clipped only to the edges of the [`Material`](crate::Material).
    /// This is the default.
    ///
    /// When the ripple is removed, `on_removed` will be called.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        position: Offset,
        color: Color,
        text_direction: TextDirection,
        contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<Box<dyn ShapeBorder>>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> Handle<InkRipple> {
        let target_radius = radius.unwrap_or_else(|| {
            get_target_radius(
                app,
                reference_box,
                contained_ink_well,
                rect_callback.as_ref(),
                position,
            )
        });
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        let this = app.create(InkRipple {
            ink_feature,
            interactive_ink_feature: InteractiveInkFeatureData::new(color, custom_border),
            position,
            border_radius: border_radius.unwrap_or(BorderRadius::ZERO),
            clip_callback: get_clip_callback(reference_box, contained_ink_well, rect_callback),
            text_direction,
            radius: None,
            radius_controller: None,
            fade_in: None,
            fade_in_controller: None,
            fade_out: None,
            fade_out_controller: None,
            owned: Vec::new(),
        });
        let vsync = controller.vsync(app);
        let create_controller = |app: &mut App, duration: Duration| {
            AnimationController::create(
                app,
                None,
                Some(duration),
                None,
                0.0,
                1.0,
                AnimationBehavior::Normal,
                Rc::clone(&vsync),
            )
        };

        // Immediately begin fading-in the initial splash.
        let fade_in_controller = create_controller(app, K_FADE_IN_DURATION);
        fade_in_controller.add_listener(app, controller.mark_needs_paint_listener());
        fade_in_controller.forward(app, None);
        let fade_in_tween = IntTween::new(app, Some(0), Some(i64::from(color.alpha())));
        let fade_in = fade_in_controller.drive(app, fade_in_tween);

        // Controls the splash radius and its center. Starts upon confirm.
        let radius_controller = create_controller(app, K_UNCONFIRMED_RIPPLE_DURATION);
        radius_controller.add_listener(app, controller.mark_needs_paint_listener());
        radius_controller.forward(app, None);
        // Initial splash diameter is 60% of the target diameter, final
        // diameter is 10dps larger than the target diameter.
        let radius_tween = Tween::new(app, Some(target_radius * 0.30), Some(target_radius + 5.0));
        let ease_curve_tween = InkRipple::ease_curve_tween(app);
        let radius = radius_controller.drive(app, radius_tween.chain(ease_curve_tween));

        // Controls the splash radius and its center. Starts upon confirm however its
        // Interval delays changes until the radius expansion has completed.
        let fade_out_controller = create_controller(app, K_FADE_OUT_DURATION);
        fade_out_controller.add_listener(app, controller.mark_needs_paint_listener());
        fade_out_controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                this.handle_alpha_status_changed(app, status)
            }),
        );
        let fade_out_tween = IntTween::new(app, Some(i64::from(color.alpha())), Some(0));
        let fade_out_interval_tween = InkRipple::fade_out_interval_tween(app);
        let fade_out =
            fade_out_controller.drive(app, fade_out_tween.chain(fade_out_interval_tween));

        let ripple = app.get_mut(this);
        ripple.fade_in_controller = Some(fade_in_controller);
        ripple.fade_in = Some(fade_in);
        ripple.radius_controller = Some(radius_controller);
        ripple.radius = Some(radius);
        ripple.fade_out_controller = Some(fade_out_controller);
        ripple.fade_out = Some(fade_out);
        ripple.owned = vec![
            fade_in.id(),
            radius.id(),
            fade_out.id(),
            fade_in_tween.id(),
            radius_tween.id(),
            ease_curve_tween.id(),
            fade_out_tween.id(),
            fade_out_interval_tween.id(),
        ];

        controller.add_ink_feature(app, this.as_ink_feature());
        this
    }

    /// Used to specify this type of ink splash for an [`InkWell`](crate::InkWell),
    /// [`InkResponse`](crate::InkResponse), material `Theme`, or `ButtonStyle`.
    ///
    /// Dart's `static const`: every call hands out the same factory, so they compare equal.
    pub fn splash_factory() -> InteractiveInkFeatureFactoryRef {
        thread_local! {
            static FACTORY: InteractiveInkFeatureFactoryRef = Rc::new(InkRippleFactory);
        }
        FACTORY.with(Rc::clone)
    }

    /// Dart's `static final Animatable<double> _easeCurveTween`; one per ripple, in the
    /// arena of the ripple's app.
    fn ease_curve_tween(app: &mut App) -> Handle<CurveTween> {
        CurveTween::new(app, Curves::ease())
    }

    /// Dart's `static final Animatable<double> _fadeOutIntervalTween`; one per ripple, in the
    /// arena of the ripple's app.
    fn fade_out_interval_tween(app: &mut App) -> Handle<CurveTween> {
        CurveTween::new(
            app,
            Rc::new(Interval::new(
                K_FADE_OUT_INTERVAL_START,
                1.0,
                Curves::linear(),
            )),
        )
    }

    /// The alpha, centre and radius [`paint_feature`](InkFeatureLeaf::paint_feature) draws
    /// the ripple's circle with now.
    pub(crate) fn circle(self: Handle<Self>, app: &mut App) -> (i64, Offset, f64) {
        let ripple = app.get(self);
        let fade_in_controller = InkRipple::controller(ripple.fade_in_controller);
        let alpha = if fade_in_controller.is_animating(app) {
            app.get(self).fade_in.expect("created in new").value(app)
        } else {
            app.get(self).fade_out.expect("created in new").value(app)
        };
        let ripple = app.get(self);
        let rect = ripple
            .clip_callback
            .as_ref()
            .map(|clip_callback| clip_callback(app));
        // Splash moves to the center of the reference box.
        let center = Offset::lerp(
            Some(ripple.position),
            Some(match rect {
                Some(rect) => rect.center(),
                None => InkFeature::reference_box(self, app)
                    .size(app)
                    .center(Offset::ZERO),
            }),
            Curves::ease().transform(InkRipple::controller(ripple.radius_controller).value(app)),
        )
        .expect("both ends are given");
        let radius = ripple.radius.expect("created in new").value(app);
        (alpha, center, radius)
    }

    fn controller(field: Option<Handle<AnimationController>>) -> Handle<AnimationController> {
        field.expect("created in new")
    }

    fn handle_alpha_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_completed() {
            InkFeatureLeaf::dispose(self, app);
        }
    }
}

impl InkFeatureLeafData for InkRipple {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InteractiveInkFeatureLeafData for InkRipple {
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData {
        &self.interactive_ink_feature
    }

    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData {
        &mut self.interactive_ink_feature
    }
}

impl InteractiveInkFeatureLeaf for InkRipple {
    fn confirm(self: Handle<Self>, app: &mut App) {
        let ripple = app.get(self);
        let radius_controller = InkRipple::controller(ripple.radius_controller);
        let fade_in_controller = InkRipple::controller(ripple.fade_in_controller);
        let fade_out_controller = InkRipple::controller(ripple.fade_out_controller);
        app.get_mut(radius_controller).duration = Some(K_RADIUS_DURATION);
        radius_controller.forward(app, None);
        // This confirm may have been preceded by a cancel.
        fade_in_controller.forward(app, None);
        fade_out_controller.animate_to(app, 1.0, Some(K_FADE_OUT_DURATION), Curves::linear());
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        let ripple = app.get(self);
        let fade_in_controller = InkRipple::controller(ripple.fade_in_controller);
        let fade_out_controller = InkRipple::controller(ripple.fade_out_controller);
        fade_in_controller.stop(app, true);
        // Watch out: setting _fadeOutController's value to 1.0 will
        // trigger a call to _handleAlphaStatusChanged() which will
        // dispose _fadeOutController.
        let fade_out_value = 1.0 - fade_in_controller.value(app);
        fade_out_controller.set_value(app, fade_out_value);
        if fade_out_value < 1.0 {
            fade_out_controller.animate_to(app, 1.0, Some(K_CANCEL_DURATION), Curves::linear());
        }
    }
}

impl InkFeatureLeaf for InkRipple {
    fn dispose(self: Handle<Self>, app: &mut App) {
        let ripple = app.get(self);
        let controllers = [
            InkRipple::controller(ripple.radius_controller),
            InkRipple::controller(ripple.fade_in_controller),
            InkRipple::controller(ripple.fade_out_controller),
        ];
        for controller in controllers {
            controller.dispose(app);
        }
        for controller in controllers {
            destroy_lingering(app, controller);
        }
        for owned in std::mem::take(&mut app.get_mut(self).owned) {
            destroy_lingering(app, owned);
        }
        InkFeature::dispose(self, app);
    }

    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4) {
        let (alpha, center, radius) = self.circle(app);
        let ripple = app.get(self);
        let color = InteractiveInkFeature::color(self, app);
        let paint = Paint::from_color(color.with_alpha(alpha as i32).into());
        InteractiveInkFeature::paint_ink_circle(
            app,
            canvas,
            transform,
            &paint,
            center,
            radius,
            Some(ripple.text_direction),
            ripple.interactive_ink_feature.custom_border_ref(),
            ripple.border_radius,
            ripple.clip_callback.as_ref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Size, TargetPlatform};
    use waft_painting::BoxShape;
    use waft_widgets::{
        Actions, ActivateIntent, BuildContext, Builder, Center, Directionality,
        FocusHighlightStrategy, FocusManager, IntoWidget, SizedBox, WidgetRef,
    };

    use super::*;
    use crate::ink_well::{InkResponse, InkWell};
    use crate::material::{Material, RenderInkFeatures};
    use crate::test_support::{Drawn, WidgetTester, with_main_thread_stack};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    const HIGHLIGHT_COLOR: Color = Color::new(0xAAFF0000);
    const SPLASH_COLOR: Color = Color::new(0xB40000FF);

    /// The square ink well of Dart's ripple cases, at the centre of the view.
    fn ripple_frame(child: Option<WidgetRef>) -> WidgetRef {
        let mut ink_well = InkWell::new()
            .border_radius(BorderRadius::all(waft_embedder::Radius::circular(6.0)))
            .highlight_color(HIGHLIGHT_COLOR)
            .splash_color(SPLASH_COLOR)
            .on_tap(Listener::new(|_app| {}))
            .radius(100.0)
            .splash_factory(InkRipple::splash_factory());
        if let Some(child) = child {
            ink_well = ink_well.child(child);
        }
        Directionality::new(
            TextDirection::Ltr,
            Material::new()
                .child(Center::new().child(SizedBox::square(Some(100.0)).child(ink_well))),
        )
        .into_widget()
    }

    const TOP_LEFT: Offset = Offset::new(350.0, 250.0);
    const INK_WELL_CENTER: Offset = Offset::new(400.0, 300.0);

    fn ripple(tester: &WidgetTester) -> Handle<InkRipple> {
        let render = tester.first::<RenderInkFeatures>();
        let app = tester.cell.borrow();
        render
            .debug_ink_features(&app)
            .unwrap_or_default()
            .into_iter()
            .find_map(|feature| feature.downcast::<InkRipple>(&app))
            .expect("a ripple")
    }

    /// Dart's `ripplePattern`: the circle's centre, radius and alpha.
    fn assert_ripple(
        tester: &WidgetTester,
        expected_center: Offset,
        expected_radius: f64,
        expected_alpha: i64,
    ) {
        let (alpha, center, radius) = ripple(tester).circle(&mut tester.cell.borrow_mut());
        assert!(
            (center - expected_center).distance() < 1.0
                && (radius - expected_radius).abs() < 1.0
                && alpha == expected_alpha,
            "Expected: center == {expected_center:?}, radius == {expected_radius}, alpha == \
             {expected_alpha}\nFound: center == {center:?} radius == {radius} alpha == {alpha}"
        );
        if alpha > 0 {
            let drawn = tester.ink_drawn();
            assert!(
                drawn.iter().any(|drawn| matches!(
                    drawn,
                    Drawn::Path(color, bounds) if *color >> 24 == alpha as u32
                        && (bounds.center() - center).distance() < 1e-3
                )),
                "the ripple's circle is drawn: {drawn:?}"
            );
        }
    }

    #[test]
    fn the_ink_well_widget_renders_an_ink_ripple() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ripple_frame(None));
            let tap_down_offset = TOP_LEFT;
            tester.tap(tap_down_offset, 1);
            tester.pump(Duration::ZERO); // start gesture

            // Initially the ripple's center is where the tap occurred;
            // ripplePattern always add a translation of tapDownOffset.
            assert_ripple(&tester, Offset::ZERO, 30.0, 0);

            // The ripple fades in for 75ms. During that time its alpha is eased from
            // 0 to the splashColor's alpha value and its center moves towards the
            // center of the ink well.
            tester.pump(Duration::from_millis(50));
            assert_ripple(&tester, Offset::new(17.0, 17.0), 56.0, 120);

            // At 75ms the ripple has fade in: it's alpha matches the splashColor's
            // alpha and its center has moved closer to the ink well's center.
            tester.pump(Duration::from_millis(25));
            assert_ripple(&tester, Offset::new(29.0, 29.0), 73.0, 180);

            // At this point the splash radius has expanded to its limit: 5 past the
            // ink well's radius parameter. The splash center has moved to its final
            // location at the inkwell's center and the fade-out is about to start.
            // The fade-out begins at 225ms = 50ms + 25ms + 150ms.
            tester.pump(Duration::from_millis(150));
            assert_ripple(&tester, INK_WELL_CENTER - tap_down_offset, 105.0, 180);

            // After another 150ms the fade-out is complete.
            tester.pump(Duration::from_millis(150));
            assert_ripple(&tester, INK_WELL_CENTER - tap_down_offset, 105.0, 0);
        });
    }

    /// Dart sends the shortcut's key; here the intent is invoked under the focused ink well.
    #[test]
    fn the_ink_well_widget_renders_an_activate_action_induced_ink_ripple() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            {
                let mut app = tester.cell.borrow_mut();
                FocusManager::instance(&mut app)
                    .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
            }
            let inner: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let child = Builder::new({
                let inner = Rc::clone(&inner);
                move |_app, context| {
                    inner.set(Some(context));
                    SizedBox::expand().into_widget()
                }
            })
            .into_widget();
            tester.pump_widget(ripple_frame(Some(child)));
            tester.pump_and_settle();
            let inkwell_center = INK_WELL_CENTER - TOP_LEFT;

            let context = inner.get().expect("built");
            Actions::invoke(&mut tester.cell.borrow_mut(), context, &ActivateIntent);
            tester.pump(Duration::ZERO);

            // ripplePattern always add a translation of topLeft.
            assert_ripple(&tester, inkwell_center, 30.0, 0);

            // The ripple fades in for 75ms. During that time its alpha is eased from
            // 0 to the splashColor's alpha value.
            tester.pump(Duration::from_millis(50));
            assert_ripple(&tester, inkwell_center, 56.0, 120);

            // At 75ms the ripple has faded in: it's alpha matches the splashColor's
            // alpha.
            tester.pump(Duration::from_millis(25));
            assert_ripple(&tester, inkwell_center, 73.0, 180);

            // At this point the splash radius has expanded to its limit: 5 past the
            // ink well's radius parameter. The fade-out is about to start.
            // The fade-out begins at 225ms = 50ms + 25ms + 150ms.
            tester.pump(Duration::from_millis(150));
            assert_ripple(&tester, inkwell_center, 105.0, 180);

            // After another 150ms the fade-out is complete.
            tester.pump(Duration::from_millis(150));
            assert_ripple(&tester, inkwell_center, 105.0, 0);
        });
    }

    #[test]
    fn cancel_an_ink_ripple_that_was_disposed_when_its_animation_ended() {
        // Regression test for https://github.com/flutter/flutter/issues/14391
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(
                Directionality::new(
                    TextDirection::Ltr,
                    Material::new().child(
                        Center::new().child(
                            SizedBox::square(Some(100.0)).child(
                                InkWell::new()
                                    .on_tap(Listener::new(|_app| {}))
                                    .radius(100.0)
                                    .splash_factory(InkRipple::splash_factory()),
                            ),
                        ),
                    ),
                )
                .into_widget(),
            );
            tester.tap(TOP_LEFT, 1);
            tester.pump(Duration::ZERO); // start splash
            tester.pump(K_FADE_OUT_DURATION); // _kFadeOutDuration, in_ripple.dart

            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, TOP_LEFT);
            tester.pump(Duration::ZERO); // start gesture
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.up(&gesture); // generates a tap cancel
            tester.pump_and_settle();
        });
    }

    #[test]
    fn cancel_an_ink_ripple_before_it_started() {
        // Regression test for https://github.com/flutter/flutter/issues/14391
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ripple_frame(None));
            tester.tap(TOP_LEFT, 1);
            tester.pump(Duration::ZERO); // start splash
            // No delay here so _fadeInController.value=1.0 (InkRipple.dart)

            // Generate a tap cancel; Will cancel the ink splash before it started
            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, TOP_LEFT);
            tester.pump(Duration::ZERO); // start gesture
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.up(&gesture); // generates a tap cancel

            let render = tester.first::<RenderInkFeatures>();
            let app = tester.cell.borrow();
            let ripples: Vec<Handle<InkRipple>> = render
                .debug_ink_features(&app)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|feature| feature.downcast::<InkRipple>(&app))
                .collect();
            drop(app);
            for ripple in ripples {
                let (alpha, _, _) = ripple.circle(&mut tester.cell.borrow_mut());
                assert_eq!(alpha, 0, "Expected: paint.color.alpha == 0, found: {alpha}");
            }
        });
    }

    /// Dart's `_InkRippleFactory`: a ripple whose rect is the top 100x100 of the response.
    #[derive(Debug)]
    struct TopSquareRippleFactory;

    impl InteractiveInkFeatureFactory for TopSquareRippleFactory {
        fn create(
            &self,
            app: &mut App,
            controller: AnyMaterialInkController,
            reference_box: AnyRenderBox,
            position: Offset,
            color: Color,
            text_direction: TextDirection,
            contained_ink_well: bool,
            _rect_callback: Option<RectCallback>,
            border_radius: Option<BorderRadius>,
            custom_border: Option<Box<dyn ShapeBorder>>,
            radius: Option<f64>,
            on_removed: Option<Listener>,
        ) -> AnyInteractiveInkFeature {
            InkRipple::new(
                app,
                controller,
                reference_box,
                position,
                color,
                text_direction,
                contained_ink_well,
                Some(Rc::new(|_app| Offset::ZERO & Size::new(100.0, 100.0))),
                border_radius,
                custom_border,
                radius,
                on_removed,
            )
            .as_interactive_ink_feature()
        }
    }

    #[test]
    fn material2_custom_rect_callback_renders_an_ink_splash_from_its_center() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let splash_color = Color::new(0xff00ff00);
            let build_widget =
                |tester: &WidgetTester, splash_factory: Option<InteractiveInkFeatureFactoryRef>| {
                    let mut response = InkResponse::new()
                        .splash_color(splash_color)
                        .contained_ink_well(true)
                        .highlight_shape(BoxShape::Rectangle)
                        .on_tap(Listener::new(|_app| {}));
                    if let Some(splash_factory) = splash_factory {
                        response = response.splash_factory(splash_factory);
                    }
                    let data = ThemeData::builder()
                        .use_material3(false)
                        .build(&tester.cell.borrow());
                    Theme::new(
                        data,
                        Directionality::new(
                            TextDirection::Ltr,
                            Material::new().child(
                                Center::new().child(
                                    SizedBox::new().width(100.0).height(200.0).child(response),
                                ),
                            ),
                        ),
                    )
                    .into_widget()
                };
            let circle_center = |tester: &WidgetTester| {
                tester.ink_drawn().iter().find_map(|drawn| match drawn {
                    Drawn::Path(color, bounds) if *color == splash_color.value() => {
                        Some(bounds.center())
                    }
                    _ => None,
                })
            };

            let widget = build_widget(&tester, None);
            tester.pump_widget(widget);
            let center = WidgetTester::SIZE.center(Offset::ZERO);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            tester.pump(Duration::ZERO); // start gesture
            tester.pump_and_settle(); // Finish rendering ink splash.
            assert_eq!(circle_center(&tester), Some(Offset::new(50.0, 100.0)));
            tester.up(&gesture);

            let widget = build_widget(&tester, Some(Rc::new(TopSquareRippleFactory)));
            tester.pump_widget(widget);
            tester.pump_and_settle(); // Finish rendering ink splash.

            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, center);
            tester.pump(Duration::ZERO); // start gesture
            tester.pump_and_settle(); // Finish rendering ink splash.
            assert_eq!(circle_center(&tester), Some(Offset::new(50.0, 50.0)));
        });
    }
}
