//! Flutter counterpart: material_ui's `ink_highlight.dart`.
//!
//! Dart's file reads the deprecated `Color.alpha` and its channels, and so does this one.
#![allow(deprecated)]

use std::time::Duration;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationLocalListenersMixin,
    AnimationLocalStatusListenersMixin, AnimationStatus, AnimationStatusListener, AnyAnimation,
    IntTween,
};
use waft_embedder::{Canvas, Color, Matrix4, Offset, Paint, Rect, TextDirection};
use waft_foundation::{App, Handle, Listener};
use waft_painting::{BorderRadius, BoxShape, ShapeBorder, draw_rrect, get_as_translation};
use waft_rendering::AnyRenderBox;

use crate::ink_well::{
    InteractiveInkFeature, InteractiveInkFeatureData, InteractiveInkFeatureLeaf,
    InteractiveInkFeatureLeafData, rrect_from_rect_and_corners,
};
use crate::material::{
    AnyMaterialInkController, InkFeature, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData,
    InkFeatureObject, Material, RectCallback, destroy_lingering,
};

const K_DEFAULT_HIGHLIGHT_FADE_DURATION: Duration = Duration::from_millis(200);

/// A visual emphasis on a part of a [`Material`] receiving user interaction.
///
/// This object is rarely created directly. Instead of creating an ink highlight
/// directly, consider using an [`InkResponse`](crate::InkResponse) or
/// [`InkWell`](crate::InkWell) widget, which uses gestures (such as tap and long-press) to
/// trigger ink highlights.
///
/// See also:
///
///  * [`InkResponse`](crate::InkResponse), which uses gestures to trigger ink highlights and
///    ink splashes in the parent [`Material`].
///  * [`InkWell`](crate::InkWell), which is a rectangular [`InkResponse`](crate::InkResponse)
///    (the most common type of ink response).
///  * [`Material`], which is the widget on which the ink highlight is painted.
///  * [`InkSplash`](crate::InkSplash), which is an ink feature that shows a reaction to user
///    input on a [`Material`].
///  * [`Ink`](crate::Ink), a convenience widget for drawing images and other decorations on
///    Material widgets.
pub struct InkHighlight {
    ink_feature: InkFeatureData,
    interactive_ink_feature: InteractiveInkFeatureData,
    shape: BoxShape,
    radius: Option<f64>,
    border_radius: BorderRadius,
    rect_callback: Option<RectCallback>,
    text_direction: TextDirection,
    alpha: Option<AnyAnimation<i64>>,
    alpha_controller: Option<Handle<AnimationController>>,
    alpha_tween: Option<Handle<IntTween>>,
    active: bool,
}

impl InkHighlight {
    /// The default fade duration of [`InkHighlight::new`]: Dart's
    /// `_kDefaultHighlightFadeDuration`.
    pub const DEFAULT_FADE_DURATION: Duration = K_DEFAULT_HIGHLIGHT_FADE_DURATION;

    /// Begin a highlight animation.
    ///
    /// The `controller` argument is typically obtained via
    /// `Material::of(app, context)`.
    ///
    /// If a `rect_callback` is given, then it provides the highlight rectangle,
    /// otherwise, the highlight rectangle is coincident with the `reference_box`.
    ///
    /// When the highlight is removed, `on_removed` will be called.
    ///
    /// Dart's defaults: `shape` [`BoxShape::Rectangle`], `border_radius`
    /// [`BorderRadius::ZERO`], `fade_duration` [`InkHighlight::DEFAULT_FADE_DURATION`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        color: Color,
        text_direction: TextDirection,
        shape: BoxShape,
        radius: Option<f64>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<Box<dyn ShapeBorder>>,
        rect_callback: Option<RectCallback>,
        on_removed: Option<Listener>,
        fade_duration: Duration,
    ) -> Handle<InkHighlight> {
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        let this = app.create(InkHighlight {
            ink_feature,
            interactive_ink_feature: InteractiveInkFeatureData::new(color, custom_border),
            shape,
            radius,
            border_radius: border_radius.unwrap_or(BorderRadius::ZERO),
            rect_callback,
            text_direction,
            alpha: None,
            alpha_controller: None,
            alpha_tween: None,
            active: true,
        });
        let vsync = controller.vsync(app);
        let alpha_controller = AnimationController::create(
            app,
            None,
            Some(fade_duration),
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
                this.handle_alpha_status_changed(app, status)
            }),
        );
        alpha_controller.forward(app, None);
        let alpha_tween = IntTween::new(app, Some(0), Some(i64::from(color.alpha())));
        let alpha = alpha_controller.drive(app, alpha_tween);
        let highlight = app.get_mut(this);
        highlight.alpha_controller = Some(alpha_controller);
        highlight.alpha_tween = Some(alpha_tween);
        highlight.alpha = Some(alpha);

        controller.add_ink_feature(app, this.as_ink_feature());
        this
    }

    fn alpha_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).alpha_controller.expect("created in new")
    }

    /// Whether this part of the material is being visually emphasized.
    pub fn active(self: Handle<Self>, app: &App) -> bool {
        app.get(self).active
    }

    /// Start visually emphasizing this part of the material.
    pub fn activate(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).active = true;
        self.alpha_controller(app).forward(app, None);
    }

    /// Stop visually emphasizing this part of the material.
    pub fn deactivate(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).active = false;
        self.alpha_controller(app).reverse(app, None);
    }

    /// The color [`paint_feature`](InkFeatureLeaf::paint_feature) paints with now; a
    /// transparent one draws nothing a test can record.
    #[cfg(test)]
    pub(crate) fn debug_paint_color(self: Handle<Self>, app: &App) -> Color {
        let alpha = app.get(self).alpha.expect("created in new").value(app);
        InteractiveInkFeature::color(self, app).with_alpha(alpha as i32)
    }

    fn handle_alpha_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_dismissed() && !app.get(self).active {
            InkFeatureLeaf::dispose(self, app);
        }
    }

    fn paint_highlight(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
    ) {
        let highlight = app.get(self);
        canvas.save();
        if let Some(custom_border) = highlight.interactive_ink_feature().custom_border_ref() {
            canvas.clip_path(
                &custom_border.get_outer_path(rect, Some(highlight.text_direction)),
                waft_embedder::FillRule::NonZero,
                waft_embedder::ClipOp::Intersect,
                true,
            );
        }
        match highlight.shape {
            BoxShape::Circle => {
                canvas.draw_circle(
                    rect.center(),
                    highlight.radius.unwrap_or(Material::DEFAULT_SPLASH_RADIUS) as f32,
                    paint,
                );
            }
            BoxShape::Rectangle => {
                if highlight.border_radius != BorderRadius::ZERO {
                    let clip_rrect = rrect_from_rect_and_corners(rect, highlight.border_radius);
                    draw_rrect(canvas, clip_rrect, paint);
                } else {
                    canvas.draw_rect(rect, paint);
                }
            }
        }
        canvas.restore();
    }
}

impl InkFeatureLeafData for InkHighlight {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InteractiveInkFeatureLeafData for InkHighlight {
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData {
        &self.interactive_ink_feature
    }

    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData {
        &mut self.interactive_ink_feature
    }
}

impl InkFeatureLeaf for InkHighlight {
    fn dispose(self: Handle<Self>, app: &mut App) {
        let alpha_controller = self.alpha_controller(app);
        alpha_controller.dispose(app);
        destroy_lingering(app, alpha_controller);
        if let Some(alpha_tween) = app.get_mut(self).alpha_tween.take() {
            destroy_lingering(app, alpha_tween);
        }
        if let Some(alpha) = app.get_mut(self).alpha.take() {
            destroy_lingering(app, alpha);
        }
        InkFeature::dispose(self, app);
    }

    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4) {
        let alpha = app.get(self).alpha.expect("created in new").value(app);
        let color = InteractiveInkFeature::color(self, app);
        let paint = Paint::from_color(color.with_alpha(alpha as i32).into());
        let origin_offset = get_as_translation(*transform);
        let rect = match &app.get(self).rect_callback {
            Some(rect_callback) => rect_callback(app),
            None => Offset::ZERO & InkFeature::reference_box(self, app).size(app),
        };
        match origin_offset {
            None => {
                canvas.save();
                canvas.concat(transform);
                self.paint_highlight(app, canvas, rect, &paint);
                canvas.restore();
            }
            Some(origin_offset) => {
                self.paint_highlight(app, canvas, rect.shift(origin_offset), &paint);
            }
        }
    }
}

impl InteractiveInkFeatureLeaf for InkHighlight {}
