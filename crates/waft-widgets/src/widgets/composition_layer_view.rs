//! A composition layer the host composites, in the widget tree: Android's `SurfaceView`, over
//! [`RenderCompositionLayerView`].
//!
//! A composition layer carries pixels only: it takes no input, focus or accessibility, so it is
//! not a native view such as a web view or a map, which Flutter's platform views embed. The widget
//! fills the box it is given and tells nobody its size: whoever holds the layer knows the layout
//! and tells the producer. Put a `Listener` or a `MouseRegion` around it for the input it takes
//! none of itself.

use std::fmt::{self, Debug};
use std::sync::Arc;

use waft_embedder::CompositionLayer;
use waft_foundation::App;
use waft_rendering::{AnyRenderObject, RenderBox, RenderCompositionLayerView, RenderHandle};

use crate::framework::{BuildContext, KeyRef, LeafRenderObjectWidget, RenderObjectWidget};

/// Shows a composition layer where it is placed.
#[derive(Clone)]
pub struct CompositionLayerView {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The layer a producer feeds (`Compositor::make_layer`), which the frames of the view the
    /// widget is in place.
    pub composition_layer: Arc<dyn CompositionLayer>,
}

impl CompositionLayerView {
    pub fn new(composition_layer: Arc<dyn CompositionLayer>) -> CompositionLayerView {
        CompositionLayerView {
            key: None,
            composition_layer,
        }
    }

    pub fn key(mut self, key: KeyRef) -> CompositionLayerView {
        self.key = Some(key);
        self
    }
}

impl Debug for CompositionLayerView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompositionLayerView")
            .field(
                "composition_layer",
                &Arc::as_ptr(&self.composition_layer).cast::<()>(),
            )
            .finish()
    }
}

impl RenderObjectWidget for CompositionLayerView {
    type RenderObject = RenderCompositionLayerView;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCompositionLayerView::new(app, Arc::clone(&self.composition_layer)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCompositionLayerView>,
    ) {
        render_object.set_composition_layer(app, Arc::clone(&self.composition_layer));
    }
}

impl LeafRenderObjectWidget for CompositionLayerView {}
