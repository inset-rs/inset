//! Flutter counterpart: material_ui's `ink_splash.dart`.
//!
//! Dart's file reads the deprecated `Color.alpha` and its channels, and so does this one.
#![allow(deprecated)]

use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationLocalListenersMixin,
    AnimationLocalStatusListenersMixin, AnimationRef, AnimationStatus, AnimationStatusListener,
    Drive, IntTween, Tween,
};
use waft_embedder::{Canvas, Color, Matrix4, Offset, Paint, Size, TextDirection};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref};
use waft_painting::{BorderRadius, ShapeBorderRef};
use waft_rendering::AnyRenderBox;

use crate::ink_well::{
    InteractiveInkFeature, InteractiveInkFeatureData, InteractiveInkFeatureFactory,
    InteractiveInkFeatureFactoryRef, InteractiveInkFeatureLeaf, InteractiveInkFeatureLeafData,
    InteractiveInkFeatureRef,
};
use crate::material::{
    AnyMaterialInkController, InkFeature, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData,
    Material, RectCallback,
};

const K_UNCONFIRMED_SPLASH_DURATION: Duration = Duration::from_secs(1);
const K_SPLASH_FADE_DURATION: Duration = Duration::from_millis(200);

const K_SPLASH_INITIAL_SIZE: f64 = 0.0; // logical pixels
const K_SPLASH_CONFIRMED_VELOCITY: f64 = 1.0; // logical pixels per millisecond

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
    contained_ink_well: bool,
    rect_callback: Option<&RectCallback>,
    position: Offset,
) -> f64 {
    if contained_ink_well {
        let size = match rect_callback {
            Some(rect_callback) => rect_callback(app).size(),
            None => reference_box.size(app),
        };
        return get_splash_radius_for_position_in_size(size, position);
    }
    Material::DEFAULT_SPLASH_RADIUS
}

fn get_splash_radius_for_position_in_size(bounds: Size, position: Offset) -> f64 {
    let d1 = (position - bounds.top_left(Offset::ZERO)).distance();
    let d2 = (position - bounds.top_right(Offset::ZERO)).distance();
    let d3 = (position - bounds.bottom_left(Offset::ZERO)).distance();
    let d4 = (position - bounds.bottom_right(Offset::ZERO)).distance();
    d1.max(d2).max(d3.max(d4)).ceil()
}

/// Dart's `_InkSplashFactory`.
#[derive(Debug)]
struct InkSplashFactory;

impl InteractiveInkFeatureFactory for InkSplashFactory {
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
        custom_border: Option<ShapeBorderRef>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> InteractiveInkFeatureRef {
        InkSplash::new(
            app,
            controller,
            reference_box,
            text_direction,
            Some(position),
            color,
            contained_ink_well,
            rect_callback,
            border_radius,
            custom_border,
            radius,
            on_removed,
        )
        .into_erased()
    }
}

/// A visual reaction on a piece of [`Material`] to user input.
///
/// A circular ink feature whose origin starts at the input touch point
/// and whose radius expands from zero.
///
/// This object is rarely created directly. Instead of creating an ink splash
/// directly, consider using an [`InkResponse`](crate::InkResponse) or
/// [`InkWell`](crate::InkWell) widget, which uses gestures (such as tap and long-press) to
/// trigger ink splashes.
///
/// See also:
///
///  * [`InkRipple`](crate::InkRipple), which is an ink splash feature that expands more
///    aggressively than this class does.
///  * [`InkResponse`](crate::InkResponse), which uses gestures to trigger ink highlights and
///    ink splashes in the parent [`Material`].
///  * [`InkWell`](crate::InkWell), which is a rectangular [`InkResponse`](crate::InkResponse)
///    (the most common type of ink response).
///  * [`Material`], which is the widget on which the ink splash is painted.
///  * [`InkHighlight`](crate::InkHighlight), which is an ink feature that emphasizes a part of
///    a [`Material`].
///  * [`Ink`](crate::Ink), a convenience widget for drawing images and other decorations on
///    Material widgets.
pub struct InkSplash {
    ink_feature: InkFeatureData,
    interactive_ink_feature: InteractiveInkFeatureData,
    position: Option<Offset>,
    border_radius: BorderRadius,
    target_radius: f64,
    clip_callback: Option<RectCallback>,
    reposition_to_reference_box: bool,
    text_direction: TextDirection,
    radius: Option<AnimationRef<f64>>,
    radius_controller: Option<Ref<AnimationController>>,
    alpha: Option<AnimationRef<i64>>,
    alpha_controller: Option<Ref<AnimationController>>,
}

impl InkSplash {
    /// Begin a splash, centered at position relative to `reference_box`.
    ///
    /// The `controller` argument is typically obtained via `Material::of(app, context)`.
    ///
    /// If `contained_ink_well` is true, then the splash will be sized to fit
    /// the well rectangle, then clipped to it when drawn. The well
    /// rectangle is the box returned by `rect_callback`, if provided, or
    /// otherwise is the bounds of the `reference_box`.
    ///
    /// If `contained_ink_well` is false, then `rect_callback` should be null.
    /// The ink splash is clipped only to the edges of the [`Material`].
    /// This is the default.
    ///
    /// When the splash is removed, `on_removed` will be called.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        text_direction: TextDirection,
        position: Option<Offset>,
        color: Color,
        contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<ShapeBorderRef>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> Ref<InkSplash> {
        let target_radius = radius.unwrap_or_else(|| {
            get_target_radius(
                app,
                reference_box,
                contained_ink_well,
                rect_callback.as_ref(),
                position.expect("a splash without a radius has a position"),
            )
        });
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        let this = app.create(InkSplash {
            ink_feature,
            interactive_ink_feature: InteractiveInkFeatureData::new(color, custom_border),
            position,
            border_radius: border_radius.unwrap_or(BorderRadius::ZERO),
            target_radius,
            clip_callback: get_clip_callback(reference_box, contained_ink_well, rect_callback),
            reposition_to_reference_box: !contained_ink_well,
            text_direction,
            radius: None,
            radius_controller: None,
            alpha: None,
            alpha_controller: None,
        });
        let handle = this.handle();
        let vsync = controller.vsync(app);
        let radius_controller = AnimationController::new(
            app,
            None,
            Some(K_UNCONFIRMED_SPLASH_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            Rc::clone(&vsync),
        );
        radius_controller.add_listener(app, controller.mark_needs_paint_listener());
        radius_controller.forward(app, None);
        let radius_tween = Tween::new(app, Some(K_SPLASH_INITIAL_SIZE), Some(target_radius));
        let radius = radius_controller.drive(app, radius_tween);
        let alpha_controller = AnimationController::new(
            app,
            None,
            Some(K_SPLASH_FADE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        );
        alpha_controller.add_listener(app, controller.mark_needs_paint_listener());
        alpha_controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                handle.handle_alpha_status_changed(app, status)
            }),
        );
        let alpha_tween = IntTween::new(app, Some(i64::from(color.alpha())), Some(0));
        let alpha = alpha_controller.drive(app, alpha_tween);
        let splash = app.get_mut(handle);
        splash.radius_controller = Some(radius_controller);
        splash.radius = Some(radius);
        splash.alpha_controller = Some(alpha_controller);
        splash.alpha = Some(alpha);

        controller.add_ink_feature(app, this.erase());
        this
    }

    /// Used to specify this type of ink splash for an [`InkWell`](crate::InkWell),
    /// [`InkResponse`](crate::InkResponse), material `Theme`, or `ButtonStyle`.
    ///
    /// Dart's `static const`: every call hands out the same factory, so they compare equal.
    pub fn splash_factory() -> InteractiveInkFeatureFactoryRef {
        thread_local! {
            static FACTORY: InteractiveInkFeatureFactoryRef = Rc::new(InkSplashFactory);
        }
        FACTORY.with(Rc::clone)
    }

    fn radius_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .radius_controller
            .handle()
            .expect("created in new")
    }

    fn handle_alpha_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_completed() {
            InkFeatureLeaf::dispose(self, app);
        }
    }
}

impl InkFeatureLeafData for InkSplash {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InteractiveInkFeatureLeafData for InkSplash {
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData {
        &self.interactive_ink_feature
    }

    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData {
        &mut self.interactive_ink_feature
    }
}

impl InteractiveInkFeatureLeaf for InkSplash {
    fn confirm(self: Handle<Self>, app: &mut App) {
        let duration = (app.get(self).target_radius / K_SPLASH_CONFIRMED_VELOCITY).floor() as u64;
        let radius_controller = self.radius_controller(app);
        app.get_mut(radius_controller).duration = Some(Duration::from_millis(duration));
        radius_controller.forward(app, None);
        app.get(self)
            .alpha_controller
            .as_ref()
            .expect("not disposed")
            .forward(app, None);
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        if let Some(alpha_controller) = app.get(self).alpha_controller.handle() {
            alpha_controller.forward(app, None);
        }
    }
}

impl InkFeatureLeaf for InkSplash {
    fn dispose(self: Handle<Self>, app: &mut App) {
        let radius_controller = self.radius_controller(app);
        radius_controller.dispose(app);
        let alpha_controller = app
            .get_mut(self)
            .alpha_controller
            .take()
            .expect("disposed once");
        alpha_controller.dispose(app);
        InkFeature::dispose(self, app);
    }

    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4) {
        let splash = app.get(self);
        let alpha = splash.alpha.as_ref().expect("created in new").value(app);
        let radius = splash.radius.as_ref().expect("created in new").value(app);
        let color = InteractiveInkFeature::color(self, app);
        let paint = Paint::from_color(color.with_alpha(alpha as i32).into());
        let mut center = splash.position;
        if splash.reposition_to_reference_box {
            let reference_box = InkFeature::reference_box(self, app);
            center = Offset::lerp(
                center,
                Some(reference_box.size(app).center(Offset::ZERO)),
                self.radius_controller(app).value(app),
            );
        }
        InteractiveInkFeature::paint_ink_circle(
            app,
            canvas,
            transform,
            &paint,
            center.expect("a splash has a center"),
            radius,
            Some(splash.text_direction),
            splash.interactive_ink_feature.custom_border_ref(),
            splash.border_radius,
            splash.clip_callback.as_ref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use waft_foundation::K_IS_WEB;
    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};
    use waft_widgets::{Center, Directionality, IntoWidget, SizedBox, WidgetRef};

    use super::*;
    use crate::ink_well::InkWell;
    use crate::no_splash::NoSplash;
    use crate::test_support::{Drawn, PaintInspection};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    fn with_theme(tester: &WidgetTester, use_material3: bool, child: WidgetRef) -> WidgetRef {
        let data = ThemeData::builder()
            .use_material3(use_material3)
            .build(&tester.cell.borrow());
        Theme::new(data, Directionality::new(TextDirection::Ltr, child)).into_widget()
    }

    #[test]
    fn material2_ink_well_widget_renders_an_ink_splash() {
        test_widgets(|tester| {
            let splash_color = Color::new(0xAA0000FF);
            let border_radius = BorderRadius::all(waft_embedder::Radius::circular(6.0));
            let widget = with_theme(
                tester,
                false,
                Material::new()
                    .child(
                        Center::new().child(
                            SizedBox::new().width(200.0).height(60.0).child(
                                InkWell::new()
                                    .border_radius(border_radius)
                                    .splash_color(splash_color)
                                    .on_tap(Listener::new(|_app| {})),
                            ),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);

            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, WidgetTester::SIZE.center(Offset::ZERO));
            tester.pump(); // start gesture
            tester.pump_by(Duration::from_millis(200)); // wait for splash to be well under way

            let drawn = tester.ink_drawn();
            let circle = drawn
                .iter()
                .find_map(|drawn| match drawn {
                    Drawn::Path(color, bounds) if *color == splash_color.value() => Some(*bounds),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("a splash circle in {drawn:?}"));
            assert_eq!(circle.center(), Offset::new(100.0, 30.0));
            assert!((circle.width() / 2.0 - 21.0).abs() < 1e-3, "{circle:?}");
            let all = Drawn::all(&tester.ink_list());
            assert!(all.contains(&Drawn::Clip), "the rounded clip: {all:?}");

            tester.up(&gesture);
        });
    }

    /// Dart's `InkWell with NoSplash splashFactory paints nothing`, Material 2 and 3. Valo
    /// records no draw of a circle of radius 0 or a transparent highlight, which are what
    /// the frame after the press holds, so one more frame runs before each count.
    fn press_and_count(
        tester: &mut WidgetTester,
        use_material3: bool,
        splash_factory: Option<InteractiveInkFeatureFactoryRef>,
    ) -> (usize, usize) {
        let mut ink_well = InkWell::new()
            .on_tap(Listener::new(|_app| {}))
            .child(SizedBox::new().width(40.0).height(20.0));
        if let Some(splash_factory) = splash_factory {
            ink_well = ink_well.splash_factory(splash_factory);
        }
        let widget = with_theme(
            tester,
            use_material3,
            Center::new()
                .child(Material::new().child(ink_well))
                .into_widget(),
        );
        tester.pump_widget(widget);
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, WidgetTester::SIZE.center(Offset::ZERO));
        tester.pump_by(Duration::from_millis(200));
        tester.pump_by(Duration::from_millis(16));
        let drawn = tester.ink_drawn();
        let rects = drawn
            .iter()
            .filter(|drawn| matches!(drawn, Drawn::Rect(..)))
            .count();
        let circles = drawn
            .iter()
            .filter(|drawn| matches!(drawn, Drawn::Path(..)))
            .count();
        tester.up(&gesture);
        tester.pump_and_settle();
        (rects, circles)
    }

    #[test]
    fn material2_ink_well_with_no_splash_splash_factory_paints_nothing() {
        test_widgets(|tester| {
            // NoSplash.splashFactory, no splash circles drawn
            let (_, circles) = press_and_count(tester, false, Some(NoSplash::splash_factory()));
            assert_eq!(circles, 0);

            // Default splashFactory (from Theme.of().splashFactory), one splash circle drawn.
            let (_, circles) = press_and_count(tester, false, None);
            assert_eq!(circles, 1);
        });
    }

    #[test]
    fn material3_ink_well_with_no_splash_splash_factory_paints_nothing() {
        test_widgets(|tester| {
            // NoSplash.splashFactory, one rect is drawn for the highlight.
            let (rects, circles) = press_and_count(tester, true, Some(NoSplash::splash_factory()));
            assert_eq!((rects, circles), (1, 0));

            // Default splashFactory (from Theme.of().splashFactory), two rects are drawn for the
            // splash and highlight.
            let (rects, _) = press_and_count(tester, true, None);
            assert_eq!(rects, if K_IS_WEB { 1 } else { 2 });
        });
    }
}
