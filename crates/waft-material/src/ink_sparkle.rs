//! Flutter counterpart: material_ui's `ink_sparkle.dart`.
//!
//! Dart's file reads the deprecated `Color.alpha` and its channels, and so does this one.
#![allow(deprecated)]

use std::f64::consts::PI;
use std::hash::{BuildHasher, RandomState};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationLocalListenersMixin,
    AnimationLocalStatusListenersMixin, AnimationRef, AnimationStatus, AnimationStatusListener,
    ConstantTween, CurveTween, Curves, Tween, TweenSequence, TweenSequenceItem,
};
use waft_embedder::{
    Canvas, ClipOp, Color, FragmentProgram, FragmentShader, Matrix4, Offset, Paint, Shader,
    TextDirection, UniformFloatSlot, UniformVec2Slot, UniformVec4Slot,
};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref};
use waft_painting::{BorderRadius, ShapeBorder, ShapeBorderRef, get_as_translation};
use waft_rendering::AnyRenderBox;

use crate::ink_well::{
    InteractiveInkFeatureData, InteractiveInkFeatureFactory, InteractiveInkFeatureFactoryRef,
    InteractiveInkFeatureLeaf, InteractiveInkFeatureLeafData, InteractiveInkFeatureRef, clip_rrect,
    rrect_from_rect_and_corners,
};
use crate::material::{
    AnyMaterialInkController, InkFeature, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData,
    RectCallback,
};

/// Begin a Material 3 ink sparkle ripple, centered at the tap or click position
/// relative to the `reference_box`.
///
/// To use this effect, pass an instance of [`InkSparkle::splash_factory`] to the
/// `splash_factory` parameter of either the Material [`ThemeData`](crate::ThemeData) or any
/// component that has a `splash_factory` parameter, such as buttons:
///  - `ElevatedButton`
///  - `TextButton`
///  - `OutlinedButton`
///
/// The `controller` argument is typically obtained via
/// `Material::of(app, context)`.
///
/// If `contained_ink_well` is true, then the effect will be sized to fit
/// the well rectangle, and clipped to it when drawn. The well
/// rectangle is the box returned by `rect_callback`, if provided, or
/// otherwise is the bounds of the `reference_box`.
///
/// If `contained_ink_well` is false, then `rect_callback` should be null.
/// The ink ripple is clipped only to the edges of the [`Material`](crate::Material).
/// This is the default.
///
/// When the ripple is removed, `on_removed` will be called.
///
/// For typical use, pass the [`InkSparkle::splash_factory`] to the `splash_factory`
/// parameter of a button style or [`ThemeData`](crate::ThemeData).
pub struct InkSparkle {
    ink_feature: InkFeatureData,
    interactive_ink_feature: InteractiveInkFeatureData,

    animation_controller: Option<Ref<AnimationController>>,

    // The Android 12 version has these values calculated in the GLSL. They are
    // constant for every pixel in the animation, so the Flutter implementation
    // computes these animation values in software in order to simplify the shader
    // implementation and provide better performance on most devices.
    center: Option<AnimationRef<Offset>>,
    radius_scale: Option<AnimationRef<f64>>,
    alpha: Option<AnimationRef<f64>>,
    sparkle_alpha: Option<AnimationRef<f64>>,

    turbulence_seed: f64,

    color: Color,
    border_radius: BorderRadius,
    target_radius: f64,
    clip_callback: Option<RectCallback>,
    text_direction: TextDirection,

    fragment_shader: Option<FragmentShader>,
    uniforms: Option<InkSparkleUniforms>,
}

impl InkSparkle {
    const ANIMATION_DURATION: Duration = Duration::from_millis(617);
    const TARGET_RADIUS_MULTIPLIER: f64 = 2.3;
    const ROTATE_RIGHT: f64 = PI * 0.0078125;
    const ROTATE_LEFT: f64 = -Self::ROTATE_RIGHT;
    const NOISE_DENSITY: f64 = 2.1;

    /// Begin a sparkly ripple effect, centered at `position` relative to
    /// `reference_box`.
    ///
    /// The `color` defines the color of the splash itself. The sparkles are
    /// always white.
    ///
    /// The `controller` argument is typically obtained via
    /// `Material::of(app, context)`.
    ///
    /// `text_direction` is used by `custom_border` if it is non-null. This allows
    /// the `custom_border`'s path to be properly defined if it was the path was
    /// expressed in terms of "start" and "end" instead of
    /// "left" and "right".
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
    /// Clipping can happen in 3 different ways:
    ///  1. If `custom_border` is provided, it is used to determine the path for
    ///     clipping.
    ///  2. If `custom_border` is null, and `border_radius` is provided, then the
    ///     canvas is clipped by an `RRect` created from `border_radius`.
    ///  3. If `border_radius` is the default [`BorderRadius::ZERO`], then the canvas
    ///     is clipped with `rect_callback`.
    ///
    /// When the ripple is removed, `on_removed` will be called.
    ///
    /// `turbulence_seed` can be passed if a non random seed should be used for
    /// the turbulence and sparkles. By default, the seed is a random number
    /// between 0.0 and 1000.0.
    ///
    /// Turbulence is an input to the shader and helps to provides a more natural,
    ///  non-circular, "splash" effect.
    ///
    /// Sparkle randomization is also driven by the `turbulence_seed`. Sparkles are
    /// identified in the shader as "noise", and the sparkles are derived from
    /// pseudorandom triangular noise.
    ///
    /// Dart's default `contained_ink_well` is true.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        color: Color,
        position: Offset,
        text_direction: TextDirection,
        contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<ShapeBorderRef>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
        turbulence_seed: Option<f64>,
    ) -> Ref<InkSparkle> {
        debug_assert!(contained_ink_well || rect_callback.is_none());
        let target_radius = radius.unwrap_or_else(|| {
            get_target_radius(
                app,
                reference_box,
                contained_ink_well,
                rect_callback.as_ref(),
                position,
            )
        }) * Self::TARGET_RADIUS_MULTIPLIER;
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        let this = app.create(InkSparkle {
            ink_feature,
            interactive_ink_feature: InteractiveInkFeatureData::new(color, custom_border),
            animation_controller: None,
            center: None,
            radius_scale: None,
            alpha: None,
            sparkle_alpha: None,
            turbulence_seed: 0.0,
            color,
            border_radius: border_radius.unwrap_or(BorderRadius::ZERO),
            target_radius,
            clip_callback: get_clip_callback(reference_box, contained_ink_well, rect_callback),
            text_direction,
            fragment_shader: None,
            uniforms: None,
        });
        // InkSparkle will not be painted until the compilation completes.
        InkSparkleFactory::initialize_shader();
        controller.add_ink_feature(app, this.erase());
        let handle = this.handle();

        // Immediately begin animating the ink.
        let vsync = controller.vsync(app);
        let animation_controller = AnimationController::new(
            app,
            None,
            Some(Self::ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        );
        animation_controller.add_listener(app, controller.mark_needs_paint_listener());
        animation_controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                handle.handle_status_changed(app, status)
            }),
        );
        animation_controller.forward(app, None);
        let animation: AnimationRef<f64> = animation_controller.erase();

        let fast_out_slow_in = CurveTween::new(app, Curves::fast_out_slow_in());
        let one = ConstantTween::new(app, 1.0);
        let radius_scale = TweenSequence::new(vec![
            TweenSequenceItem::new(Rc::new(fast_out_slow_in.clone()), 75.0),
            TweenSequenceItem::new(Rc::new(one.clone()), 25.0),
        ])
        .animate(app, animation.clone());

        // Functionally equivalent to Android 12's SkSL:
        //`return mix(u_touch, u_resolution, saturate(in_radius_scale * 2.0))`
        let size = reference_box.size(app);
        let center_tween = Tween::new(
            app,
            Some(Offset::new(position.dx(), position.dy())),
            Some(Offset::new(size.width() / 2.0, size.height() / 2.0)),
        );
        let progress = Tween::new(app, Some(0.0), Some(1.0));
        let center_progress = TweenSequence::new(vec![
            TweenSequenceItem::new(Rc::new(progress.clone()), 50.0),
            TweenSequenceItem::new(Rc::new(one.clone()), 50.0),
        ])
        .animate(app, radius_scale.clone());
        let center = center_tween.animate(app, center_progress);

        let fade_in = Tween::new(app, Some(0.0), Some(1.0));
        let fade_out = Tween::new(app, Some(1.0), Some(0.0));
        let alpha = TweenSequence::new(vec![
            TweenSequenceItem::new(Rc::new(fade_in.clone()), 13.0),
            TweenSequenceItem::new(Rc::new(one.clone()), 27.0),
            TweenSequenceItem::new(Rc::new(fade_out.clone()), 60.0),
        ])
        .animate(app, animation.clone());

        let sparkle_alpha = TweenSequence::new(vec![
            TweenSequenceItem::new(Rc::new(fade_in.clone()), 13.0),
            TweenSequenceItem::new(Rc::new(one.clone()), 27.0),
            TweenSequenceItem::new(Rc::new(fade_out.clone()), 50.0),
        ])
        .animate(app, animation.clone());

        // Creates an element of randomness so that ink emanating from the same
        // pixel have slightly different rings and sparkles.
        let mut turbulence_seed = turbulence_seed;
        if cfg!(debug_assertions) {
            // In tests, randomness can cause flakes. So if a seed has not
            // already been specified (i.e. for the purpose of the test), set it to
            // the constant turbulence seed.
            turbulence_seed.get_or_insert(InkSparkleFactory::CONSTANT_SEED);
        }
        let turbulence_seed = turbulence_seed.unwrap_or_else(|| random_double() * 1000.0);

        let sparkle = app.get_mut(handle);
        sparkle.animation_controller = Some(animation_controller);
        sparkle.radius_scale = Some(radius_scale);
        sparkle.center = Some(center);
        sparkle.alpha = Some(alpha);
        sparkle.sparkle_alpha = Some(sparkle_alpha);
        sparkle.turbulence_seed = turbulence_seed;
        this
    }

    fn handle_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_completed() {
            InkFeatureLeaf::dispose(self, app);
        }
    }

    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .handle()
            .expect("created in new")
    }

    /// Used to specify this type of ink splash for an [`InkWell`](crate::InkWell),
    /// [`InkResponse`](crate::InkResponse), material `Theme`, or `ButtonStyle`.
    ///
    /// Since no `turbulence_seed` is passed, the effect will be random for
    /// subsequent presses in the same position.
    ///
    /// Dart's `static const`: every call hands out the same factory, so they compare equal.
    pub fn splash_factory() -> InteractiveInkFeatureFactoryRef {
        thread_local! {
            static FACTORY: InteractiveInkFeatureFactoryRef =
                Rc::new(InkSparkleFactory::new());
        }
        FACTORY.with(Rc::clone)
    }

    /// Used to specify this type of ink splash for an [`InkWell`](crate::InkWell),
    /// [`InkResponse`](crate::InkResponse), material `Theme`, or `ButtonStyle`.
    ///
    /// Since a `turbulence_seed` is passed, the effect will not be random for
    /// subsequent presses in the same position. This can be used for testing.
    ///
    /// Dart's `static const`: every call hands out the same factory, so they compare equal.
    pub fn constant_turbulence_seed_splash_factory() -> InteractiveInkFeatureFactoryRef {
        thread_local! {
            static FACTORY: InteractiveInkFeatureFactoryRef =
                Rc::new(InkSparkleFactory::constant_turbulence_seed());
        }
        FACTORY.with(Rc::clone)
    }

    fn width(self: Handle<Self>, app: &App) -> f64 {
        InkFeature::reference_box(self, app).size(app).width()
    }

    fn height(self: Handle<Self>, app: &App) -> f64 {
        InkFeature::reference_box(self, app).size(app).height()
    }

    /// All double values for uniforms come from the Android 12 ripple
    /// implementation from the following files:
    /// - <https://cs.android.com/android/platform/superproject/+/main:frameworks/base/graphics/java/android/graphics/drawable/RippleShader.java>
    /// - <https://cs.android.com/android/platform/superproject/+/main:frameworks/base/graphics/java/android/graphics/drawable/RippleDrawable.java>
    /// - <https://cs.android.com/android/platform/superproject/+/main:frameworks/base/graphics/java/android/graphics/drawable/RippleAnimationSession.java>
    fn update_fragment_shader(self: Handle<Self>, app: &mut App) {
        const TURBULENCE_SCALE: f64 = 1.5;
        let sparkle = app.get(self);
        let radius_scale = sparkle
            .radius_scale
            .as_ref()
            .expect("created in new")
            .value(app);
        let alpha = sparkle.alpha.as_ref().expect("created in new").value(app);
        let sparkle_alpha = sparkle
            .sparkle_alpha
            .as_ref()
            .expect("created in new")
            .value(app);
        let center = sparkle.center.as_ref().expect("created in new").value(app);
        let color = sparkle.color;
        let target_radius = sparkle.target_radius;
        let turbulence_phase = sparkle.turbulence_seed + radius_scale;
        let noise_phase = turbulence_phase;
        let rotation1 = turbulence_phase * Self::ROTATE_RIGHT + 1.7 * PI;
        let rotation2 = turbulence_phase * Self::ROTATE_LEFT + 2.0 * PI;
        let rotation3 = turbulence_phase * Self::ROTATE_RIGHT + 2.75 * PI;
        let width = self.width(app);
        let height = self.height(app);

        let sparkle = app.get_mut(self);
        let uniforms = sparkle.uniforms.expect("made with the shader");
        let shader = sparkle
            .fragment_shader
            .as_mut()
            .expect("made in paint_feature");
        uniforms.color.set(
            shader,
            f64::from(color.red()) / 255.0,
            f64::from(color.green()) / 255.0,
            f64::from(color.blue()) / 255.0,
            f64::from(color.alpha()) / 255.0,
        );
        // u_composite_1 packs u_alpha, u_sparkle_alpha, u_blur and u_radius_scale.
        uniforms
            .composite1
            .set(shader, alpha, sparkle_alpha, 1.0, radius_scale);
        uniforms.center.set(shader, center.dx(), center.dy());
        uniforms.max_radius.set(shader, target_radius);
        uniforms
            .resolution_scale
            .set(shader, 1.0 / width, 1.0 / height);
        uniforms.noise_scale.set(
            shader,
            Self::NOISE_DENSITY / width,
            Self::NOISE_DENSITY / height,
        );
        uniforms.noise_phase.set(shader, noise_phase / 1000.0);
        uniforms.circle1.set(
            shader,
            TURBULENCE_SCALE * 0.5 + (turbulence_phase * 0.01 * (TURBULENCE_SCALE * 0.55).cos()),
            TURBULENCE_SCALE * 0.5 + (turbulence_phase * 0.01 * (TURBULENCE_SCALE * 0.55).sin()),
        );
        uniforms.circle2.set(
            shader,
            TURBULENCE_SCALE * 0.2 + (turbulence_phase * -0.0066 * (TURBULENCE_SCALE * 0.45).cos()),
            TURBULENCE_SCALE * 0.2 + (turbulence_phase * -0.0066 * (TURBULENCE_SCALE * 0.45).sin()),
        );
        uniforms.circle3.set(
            shader,
            TURBULENCE_SCALE + (turbulence_phase * -0.0066 * (TURBULENCE_SCALE * 0.35).cos()),
            TURBULENCE_SCALE + (turbulence_phase * -0.0066 * (TURBULENCE_SCALE * 0.35).sin()),
        );
        uniforms
            .rotation1
            .set(shader, rotation1.cos(), rotation1.sin());
        uniforms
            .rotation2
            .set(shader, rotation2.cos(), rotation2.sin());
        uniforms
            .rotation3
            .set(shader, rotation3.cos(), rotation3.sin());
    }

    /// Transforms the canvas for an ink feature to be painted on the `canvas`.
    ///
    /// This should be called before painting ink features that do not use
    /// `paint_ink_circle`.
    ///
    /// The `transform` argument is the [`Matrix4`] transform that typically
    /// shifts the coordinate space of the canvas to the space in which
    /// the ink feature is to be painted.
    ///
    /// For examples on how the function is used, see [`InkSparkle`] and `paint_ink_circle`.
    fn transform_canvas(canvas: &mut Canvas, transform: &Matrix4) {
        match get_as_translation(*transform) {
            None => canvas.concat(transform),
            Some(origin_offset) => {
                canvas.translate(origin_offset.dx() as f32, origin_offset.dy() as f32)
            }
        }
    }

    /// Clips the canvas for an ink feature to be painted on the `canvas`.
    ///
    /// This should be called before painting ink features with `paint_feature`
    /// that do not use `paint_ink_circle`.
    ///
    /// The `clip_callback` is the callback used to obtain the `Rect` used for clipping
    /// the ink effect.
    ///
    /// If `clip_callback` is null, no clipping is performed on the ink circle.
    ///
    /// The `text_direction` is used by `custom_border` if it is non-null. This
    /// allows the `custom_border`'s path to be properly defined if the path was
    /// expressed in terms of "start" and "end" instead of "left" and "right".
    ///
    /// For examples on how the function is used, see [`InkSparkle`].
    fn clip_canvas(
        app: &App,
        canvas: &mut Canvas,
        clip_callback: &RectCallback,
        text_direction: Option<TextDirection>,
        custom_border: Option<&dyn ShapeBorder>,
        border_radius: BorderRadius,
    ) {
        let rect = clip_callback(app);
        if let Some(custom_border) = custom_border {
            let path = custom_border.get_outer_path(rect, text_direction);
            canvas.clip_path(path.as_valo(), path.fill_type, ClipOp::Intersect, true);
        } else if border_radius != BorderRadius::ZERO {
            clip_rrect(canvas, rrect_from_rect_and_corners(rect, border_radius));
        } else {
            canvas.clip_rect(rect, ClipOp::Intersect, true);
        }
    }
}

/// Dart's random `math.Random().nextDouble()`, from the entropy `RandomState` holds.
fn random_double() -> f64 {
    let bits = RandomState::new().hash_one(0u8) >> 11;
    bits as f64 / (1u64 << 53) as f64
}

impl InkFeatureLeafData for InkSparkle {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InteractiveInkFeatureLeafData for InkSparkle {
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData {
        &self.interactive_ink_feature
    }

    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData {
        &mut self.interactive_ink_feature
    }
}

impl InteractiveInkFeatureLeaf for InkSparkle {}

impl InkFeatureLeaf for InkSparkle {
    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation_controller = self.animation_controller(app);
        animation_controller.stop(app, true);
        animation_controller.dispose(app);
        app.get_mut(self).fragment_shader = None;
        InkFeature::dispose(self, app);
    }

    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4) {
        debug_assert!(self.animation_controller(app).is_animating(app));

        // InkSparkle can only paint if its shader has been compiled.
        let Some(program) = InkSparkleFactory::program() else {
            // Skipping paintFeature because the shader it relies on is not ready to
            // be used. InkSparkleFactory.initializeShader must complete
            // before InkSparkle can paint.
            return;
        };

        if app.get(self).fragment_shader.is_none() {
            let fragment_shader = program.fragment_shader();
            let uniforms = InkSparkleUniforms::new(&fragment_shader);
            let sparkle = app.get_mut(self);
            sparkle.fragment_shader = Some(fragment_shader);
            sparkle.uniforms = Some(uniforms);
        }

        canvas.save();
        InkSparkle::transform_canvas(canvas, transform);
        let sparkle = app.get(self);
        if let Some(clip_callback) = &sparkle.clip_callback {
            InkSparkle::clip_canvas(
                app,
                canvas,
                clip_callback,
                Some(sparkle.text_direction),
                sparkle.interactive_ink_feature.custom_border_ref(),
                sparkle.border_radius,
            );
        }

        self.update_fragment_shader(app);

        let sparkle = app.get(self);
        let paint = Paint::from_shader(Shader::from(
            sparkle.fragment_shader.as_ref().expect("made above"),
        ));
        match &sparkle.clip_callback {
            Some(clip_callback) => canvas.draw_rect(clip_callback(app), &paint),
            None => canvas.draw_paint(&paint),
        }
        canvas.restore();
    }
}

/// The uniforms declared in `shaders/ink_sparkle.wgsl`, Dart's `lib/shaders/ink_sparkle.frag`, looked up by name once
/// so that [`InkSparkle`] only has to set their values on each frame.
#[derive(Clone, Copy)]
struct InkSparkleUniforms {
    color: UniformVec4Slot,
    composite1: UniformVec4Slot,
    center: UniformVec2Slot,
    max_radius: UniformFloatSlot,
    resolution_scale: UniformVec2Slot,
    noise_scale: UniformVec2Slot,
    noise_phase: UniformFloatSlot,
    circle1: UniformVec2Slot,
    circle2: UniformVec2Slot,
    circle3: UniformVec2Slot,
    rotation1: UniformVec2Slot,
    rotation2: UniformVec2Slot,
    rotation3: UniformVec2Slot,
}

impl InkSparkleUniforms {
    fn new(shader: &FragmentShader) -> InkSparkleUniforms {
        let vec4 = |name: &str| {
            shader
                .get_uniform_vec4(name)
                .expect("ink_sparkle.wgsl declares it")
        };
        let vec2 = |name: &str| {
            shader
                .get_uniform_vec2(name)
                .expect("ink_sparkle.wgsl declares it")
        };
        let float = |name: &str| {
            shader
                .get_uniform_float(name, None)
                .expect("ink_sparkle.wgsl declares it")
        };
        InkSparkleUniforms {
            color: vec4("u_color"),
            composite1: vec4("u_composite_1"),
            center: vec2("u_center"),
            max_radius: float("u_max_radius"),
            resolution_scale: vec2("u_resolution_scale"),
            noise_scale: vec2("u_noise_scale"),
            noise_phase: float("u_noise_phase"),
            circle1: vec2("u_circle1"),
            circle2: vec2("u_circle2"),
            circle3: vec2("u_circle3"),
            rotation1: vec2("u_rotation1"),
            rotation2: vec2("u_rotation2"),
            rotation3: vec2("u_rotation3"),
        }
    }
}

/// Dart's `static bool _initCalled` and `static ui.FragmentProgram? _program`, one value here: the
/// program is made in the first [`InkSparkleFactory::initialize_shader`] call, so the call having
/// run and the program being there are one state. One program serves every thread, as the
/// engine's registry caches one per asset.
static PROGRAM: OnceLock<Arc<FragmentProgram>> = OnceLock::new();

/// Dart's `_InkSparkleFactory`.
#[derive(Debug)]
struct InkSparkleFactory {
    turbulence_seed: Option<f64>,
}

impl InkSparkleFactory {
    const CONSTANT_SEED: f64 = 1337.0;

    const fn new() -> InkSparkleFactory {
        InkSparkleFactory {
            turbulence_seed: None,
        }
    }

    const fn constant_turbulence_seed() -> InkSparkleFactory {
        InkSparkleFactory {
            turbulence_seed: Some(InkSparkleFactory::CONSTANT_SEED),
        }
    }

    /// Dart's `initializeShader`: makes the program from `ink_sparkle.wgsl`, Dart's
    /// `packages/material_ui/shaders/ink_sparkle.frag`, on the first call.
    fn initialize_shader() {
        PROGRAM.get_or_init(|| {
            FragmentProgram::from_wgsl(include_str!("shaders/ink_sparkle.wgsl"))
                .expect("ink_sparkle.wgsl is a valid program")
        });
    }

    /// Dart's `_program`: the compiled `ink_sparkle.wgsl`, once it has loaded.
    fn program() -> Option<Arc<FragmentProgram>> {
        PROGRAM.get().cloned()
    }
}

impl InteractiveInkFeatureFactory for InkSparkleFactory {
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
        InkSparkle::new(
            app,
            controller,
            reference_box,
            color,
            position,
            text_direction,
            contained_ink_well,
            rect_callback,
            border_radius,
            custom_border,
            radius,
            on_removed,
            self.turbulence_seed,
        )
        .into_erased()
    }
}

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

#[cfg(test)]
mod tests {
    use waft_embedder::{TargetPlatform, valo};
    use waft_widgets::{Center, Directionality, IntoWidget, SizedBox, WidgetRef};

    use super::*;
    use crate::elevated_button::ElevatedButton;
    use crate::ink_well::{InkResponse, InkWell};
    use crate::material::{Material, RenderInkFeatures};
    use crate::test_support::{WidgetTester, with_main_thread_stack};

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    fn frame<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Material::new().child(Center::new().child(child)),
        )
        .into_widget()
    }

    fn ink_features(tester: &WidgetTester) -> usize {
        let render = tester.first::<RenderInkFeatures>();
        render
            .debug_ink_features(&tester.cell.borrow())
            .unwrap_or_default()
            .len()
    }

    /// The rects and the paints the material's ink draws with a fragment shader, each with that
    /// shader.
    fn shader_draws(
        tester: &WidgetTester,
    ) -> (
        Vec<(waft_embedder::Rect, valo::RuntimeShader)>,
        Vec<valo::RuntimeShader>,
    ) {
        let runtime = |paint: &valo::Paint| match &paint.shader {
            Some(valo::Shader::Runtime(shader)) => Some(shader.clone()),
            _ => None,
        };
        let list = tester.ink_list();
        let mut rects = Vec::new();
        let mut paints = Vec::new();
        for op in list.ops() {
            match op {
                valo::Op::DrawRect { rect, paint, .. } => {
                    let rect = waft_embedder::Rect::from_ltwh(
                        f64::from(rect.x),
                        f64::from(rect.y),
                        f64::from(rect.width),
                        f64::from(rect.height),
                    );
                    rects.extend(runtime(paint).map(|shader| (rect, shader)));
                }
                valo::Op::DrawPaint { paint, .. } => paints.extend(runtime(paint)),
                _ => {}
            }
        }
        (rects, paints)
    }

    /// How many rects and paints the material's ink draws, with a shader or without.
    fn draw_counts(tester: &WidgetTester) -> (usize, usize) {
        let list = tester.ink_list();
        let rects = list
            .ops()
            .iter()
            .filter(|op| matches!(op, valo::Op::DrawRect { .. }))
            .count();
        let paints = list
            .ops()
            .iter()
            .filter(|op| matches!(op, valo::Op::DrawPaint { .. }))
            .count();
        (rects, paints)
    }

    /// The `count` floats of the uniform named `name` in `shader`'s bytes, read where WGSL lays
    /// them out.
    fn uniform(shader: &valo::RuntimeShader, name: &str, count: u32) -> Vec<f32> {
        let field = shader
            .effect
            .uniforms()
            .field(name)
            .unwrap_or_else(|| panic!("ink_sparkle.wgsl declares {name}"));
        (0..count)
            .map(|component| {
                let at = (field.offset + component * 4) as usize;
                f32::from_le_bytes(shader.uniforms[at..at + 4].try_into().expect("four bytes"))
            })
            .collect()
    }

    fn assert_uniform(shader: &valo::RuntimeShader, name: &str, expected: &[f64]) {
        let actual = uniform(shader, name, expected.len() as u32);
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (f64::from(*actual) - expected).abs() <= 1e-5 * expected.abs().max(1.0),
                "{name}: {actual} is not {expected}"
            );
        }
    }

    /// Dart's `InkSparkle in a Button compiles and does not crash`.
    #[test]
    fn ink_sparkle_in_a_button_compiles_and_does_not_crash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(frame(
                ElevatedButton::new(
                    Some(Listener::new(|_app| {})),
                    Some(SizedBox::new().width(80.0).height(20.0)),
                )
                .style(
                    ElevatedButton::style_from()
                        .splash_factory(InkSparkle::splash_factory())
                        .build(),
                ),
            ));
            tester.tap(WidgetTester::SIZE.center(Offset::ZERO), 1);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert!(InkSparkleFactory::program().is_some());
        });
    }

    /// Dart's `InkSparkle default splashFactory paints with drawRect when bounded`.
    #[test]
    fn ink_sparkle_default_splash_factory_paints_with_draw_rect_when_bounded() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(frame(
                InkWell::new()
                    .splash_factory(InkSparkle::splash_factory())
                    .child(SizedBox::new().width(80.0).height(20.0))
                    .on_tap(Listener::new(|_app| {})),
            ));
            tester.tap(WidgetTester::SIZE.center(Offset::ZERO), 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(200));

            assert_eq!(draw_counts(&tester).0, 1);
            let (rects, _) = shader_draws(&tester);
            assert_eq!(
                rects.iter().map(|(rect, _)| *rect).collect::<Vec<_>>(),
                [waft_embedder::Rect::from_ltwh(0.0, 0.0, 80.0, 20.0)]
            );

            assert_eq!(ink_features(&tester), 1);

            tester.pump_and_settle();
            // ink feature is disposed.
            assert_eq!(ink_features(&tester), 0);
        });
    }

    /// Dart's `InkSparkle default splashFactory paints with drawPaint when unbounded`.
    #[test]
    fn ink_sparkle_default_splash_factory_paints_with_draw_paint_when_unbounded() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(frame(
                InkResponse::new()
                    .splash_factory(InkSparkle::splash_factory())
                    .child(SizedBox::new().width(80.0).height(20.0))
                    .on_tap(Listener::new(|_app| {})),
            ));
            tester.tap(WidgetTester::SIZE.center(Offset::ZERO), 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(200));

            assert_eq!(draw_counts(&tester).1, 1);
            assert_eq!(shader_draws(&tester).1.len(), 1);
        });
    }

    /// What Dart's goldens show, as numbers: the uniforms `_updateFragmentShader` sets at 50
    /// milliseconds into the animation of a sparkle with the constant turbulence seed, tapped
    /// off the well's centre.
    #[test]
    fn the_sparkle_sets_its_uniforms_from_the_animation_and_the_constant_turbulence_seed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let color = Color::new(0x66AA3300);
            tester.pump_widget(frame(
                InkWell::new()
                    .splash_factory(InkSparkle::constant_turbulence_seed_splash_factory())
                    .splash_color(color)
                    .child(SizedBox::new().width(80.0).height(20.0))
                    .on_tap(Listener::new(|_app| {})),
            ));
            let well_center = WidgetTester::SIZE.center(Offset::ZERO);
            tester.tap(well_center + Offset::new(-30.0, -5.0), 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            let (rects, _) = shader_draws(&tester);
            let [(_, shader)] = rects.as_slice() else {
                panic!("one sparkle rect, not {}", rects.len());
            };

            let t = 50.0 / 617.0;
            let radius_scale = Curves::fast_out_slow_in().transform(t / 0.75);
            let alpha = t / 0.13;
            let sparkle_alpha = t / (13.0 / 90.0);
            let center_progress = radius_scale / 0.5;
            let position = Offset::new(10.0, 5.0);
            let center = position + (Offset::new(40.0, 10.0) - position) * center_progress;
            let target_radius = (80.0f64.hypot(20.0) / 2.0) * 2.3;
            let turbulence_phase = 1337.0 + radius_scale;
            let rotation = |offset: f64, direction: f64| {
                let angle = turbulence_phase * direction * PI * 0.0078125 + offset * PI;
                [angle.cos(), angle.sin()]
            };
            let circle = |base: f64, speed: f64, scale: f64| {
                [
                    base + turbulence_phase * speed * scale.cos(),
                    base + turbulence_phase * speed * scale.sin(),
                ]
            };

            assert_uniform(
                shader,
                "u_color",
                &[170.0 / 255.0, 51.0 / 255.0, 0.0, f64::from(0x66) / 255.0],
            );
            assert_uniform(
                shader,
                "u_composite_1",
                &[alpha, sparkle_alpha, 1.0, radius_scale],
            );
            assert_uniform(shader, "u_center", &[center.dx(), center.dy()]);
            assert_uniform(shader, "u_max_radius", &[target_radius]);
            assert_uniform(shader, "u_resolution_scale", &[1.0 / 80.0, 1.0 / 20.0]);
            assert_uniform(shader, "u_noise_scale", &[2.1 / 80.0, 2.1 / 20.0]);
            assert_uniform(shader, "u_noise_phase", &[turbulence_phase / 1000.0]);
            assert_uniform(shader, "u_circle1", &circle(0.75, 0.01, 1.5 * 0.55));
            assert_uniform(shader, "u_circle2", &circle(0.3, -0.0066, 1.5 * 0.45));
            assert_uniform(shader, "u_circle3", &circle(1.5, -0.0066, 1.5 * 0.35));
            assert_uniform(shader, "u_rotation1", &rotation(1.7, 1.0));
            assert_uniform(shader, "u_rotation2", &rotation(2.0, -1.0));
            assert_uniform(shader, "u_rotation3", &rotation(2.75, 1.0));
        });
    }

    /// `ink_sparkle.wgsl` declares the GLSL's uniforms in its order, so each slot is at the
    /// float index Flutter's `ink_sparkle.frag` gives it.
    #[test]
    fn the_shader_keeps_the_glsl_float_index_of_every_uniform() {
        InkSparkleFactory::initialize_shader();
        let shader = InkSparkleFactory::program()
            .expect("loaded")
            .fragment_shader();
        let index = |name: &str| {
            shader
                .get_uniform_float(name, Some(0))
                .expect("ink_sparkle.wgsl declares it")
                .shader_index()
        };
        let indices: Vec<usize> = [
            "u_color",
            "u_composite_1",
            "u_center",
            "u_max_radius",
            "u_resolution_scale",
            "u_noise_scale",
            "u_noise_phase",
            "u_circle1",
            "u_circle2",
            "u_circle3",
            "u_rotation1",
            "u_rotation2",
            "u_rotation3",
        ]
        .into_iter()
        .map(index)
        .collect();
        assert_eq!(indices, [0, 4, 8, 10, 11, 13, 15, 16, 18, 20, 22, 24, 26]);
    }

    /// Dart's `_InkSparkleFactory._program`: one program, loaded once, serves every sparkle.
    #[test]
    fn every_sparkle_draws_with_the_one_program() {
        InkSparkleFactory::initialize_shader();
        let first = InkSparkleFactory::program().expect("loaded");
        InkSparkleFactory::initialize_shader();
        assert!(Arc::ptr_eq(
            &first,
            &InkSparkleFactory::program().expect("loaded")
        ));
    }

    #[test]
    fn the_splash_factories_are_constants() {
        assert!(Rc::ptr_eq(
            &InkSparkle::splash_factory(),
            &InkSparkle::splash_factory()
        ));
        assert!(Rc::ptr_eq(
            &InkSparkle::constant_turbulence_seed_splash_factory(),
            &InkSparkle::constant_turbulence_seed_splash_factory()
        ));
        assert!(!Rc::ptr_eq(
            &InkSparkle::splash_factory(),
            &InkSparkle::constant_turbulence_seed_splash_factory()
        ));
    }
}
