//! Flutter counterpart: material_ui's `no_splash.dart`.

use std::rc::Rc;

use waft_embedder::{Canvas, Color, Matrix4, Offset, TextDirection};
use waft_foundation::{App, Handle, Listener, Ref};
use waft_painting::{BorderRadius, ShapeBorderRef};
use waft_rendering::AnyRenderBox;

use crate::ink_well::{
    InteractiveInkFeatureData, InteractiveInkFeatureFactory, InteractiveInkFeatureFactoryRef,
    InteractiveInkFeatureLeaf, InteractiveInkFeatureLeafData, InteractiveInkFeatureRef,
};
use crate::material::{
    AnyMaterialInkController, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData, RectCallback,
};

/// Dart's `_NoSplashFactory`.
#[derive(Debug)]
struct NoSplashFactory;

impl InteractiveInkFeatureFactory for NoSplashFactory {
    fn create(
        &self,
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        _position: Offset,
        color: Color,
        _text_direction: TextDirection,
        _contained_ink_well: bool,
        _rect_callback: Option<RectCallback>,
        _border_radius: Option<BorderRadius>,
        _custom_border: Option<ShapeBorderRef>,
        _radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> InteractiveInkFeatureRef {
        NoSplash::new(app, controller, reference_box, color, on_removed).into_erased()
    }
}

/// An [`InteractiveInkFeature`](crate::InteractiveInkFeature) that doesn't paint a splash.
///
/// Use [`NoSplash::splash_factory`] to defeat the default ink splash drawn by
/// an [`InkWell`](crate::InkWell) or `ButtonStyle`. For example, to create an `ElevatedButton`
/// that does not draw the default "ripple" ink splash when it's tapped:
///
/// ```text
/// ElevatedButton::new()
///     .style(ElevatedButton::style_from().splash_factory(NoSplash::splash_factory()))
///     .on_pressed(..)
///     .child(Text::new("Run"))
/// ```
pub struct NoSplash {
    ink_feature: InkFeatureData,
    interactive_ink_feature: InteractiveInkFeatureData,
}

impl NoSplash {
    /// Create an [`InteractiveInkFeature`](crate::InteractiveInkFeature) that doesn't paint a
    /// splash.
    pub fn new(
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        color: Color,
        on_removed: Option<Listener>,
    ) -> Ref<NoSplash> {
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        app.create(NoSplash {
            ink_feature,
            interactive_ink_feature: InteractiveInkFeatureData::new(color, None),
        })
    }

    /// Used to specify this type of ink splash for an [`InkWell`](crate::InkWell),
    /// [`InkResponse`](crate::InkResponse), material `Theme`, or `ButtonStyle`.
    ///
    /// Dart's `static const`: every call hands out the same factory, so they compare equal.
    pub fn splash_factory() -> InteractiveInkFeatureFactoryRef {
        thread_local! {
            static FACTORY: InteractiveInkFeatureFactoryRef = Rc::new(NoSplashFactory);
        }
        FACTORY.with(Rc::clone)
    }
}

impl InkFeatureLeafData for NoSplash {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InteractiveInkFeatureLeafData for NoSplash {
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData {
        &self.interactive_ink_feature
    }

    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData {
        &mut self.interactive_ink_feature
    }
}

impl InkFeatureLeaf for NoSplash {
    fn paint_feature(
        self: Handle<Self>,
        _app: &mut App,
        _canvas: &mut Canvas,
        _transform: &Matrix4,
    ) {
    }
}

impl InteractiveInkFeatureLeaf for NoSplash {
    fn confirm(self: Handle<Self>, app: &mut App) {
        InkFeatureLeaf::dispose(self, app);
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        InkFeatureLeaf::dispose(self, app);
    }
}
