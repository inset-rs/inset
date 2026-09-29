//! A native surface the host composites, in the widget tree: Android's `SurfaceView`,
//! Flutter's platform view, over [`RenderSurfaceView`].
//!
//! The widget fills the box it is given and tells nobody its size: whoever holds the surface
//! knows the layout and tells the producer. Put a `Listener` or a `MouseRegion` around it for
//! the input a surface takes none of itself.

use std::fmt::{self, Debug};
use std::sync::Arc;

use inset_embedder::Surface;
use inset_foundation::App;
use inset_rendering::{AnyRenderObject, RenderBox, RenderHandle, RenderSurfaceView};

use crate::framework::{BuildContext, KeyRef, LeafRenderObjectWidget, RenderObjectWidget};

/// Shows a native surface where it is placed.
#[derive(Clone)]
pub struct SurfaceView {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The surface a producer feeds, opened on the host's compositor.
    pub surface: Arc<dyn Surface>,
}

impl SurfaceView {
    pub fn new(surface: Arc<dyn Surface>) -> SurfaceView {
        SurfaceView { key: None, surface }
    }

    pub fn key(mut self, key: KeyRef) -> SurfaceView {
        self.key = Some(key);
        self
    }
}

impl Debug for SurfaceView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceView")
            .field("surface", &Arc::as_ptr(&self.surface).cast::<()>())
            .finish()
    }
}

impl RenderObjectWidget for SurfaceView {
    type RenderObject = RenderSurfaceView;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSurfaceView::new(app, Arc::clone(&self.surface)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSurfaceView>,
    ) {
        render_object.set_surface(app, Arc::clone(&self.surface));
    }
}

impl LeafRenderObjectWidget for SurfaceView {}
