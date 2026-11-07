//! A box that draws a video's newest frame: Flutter's `TextureBox`, over the reader a player or
//! a camera hands out in place of a texture id.
//!
//! It adds a [`VideoLayer`] at its place in paint order, and whoever draws the scene takes
//! the reader's newest frame then and draws it into the box, as part of the picture: what is
//! painted above it lies above it, and the clips, transforms and opacity above it apply to it as
//! to any drawing. It fills what it is given, the frame stretched to the box; keeping the video's
//! aspect is its parent's. It always needs compositing, as Flutter's `TextureBox` does, so the
//! layer it adds sits in the scene. The invariant: the layer added is the box, at the offset it is
//! painted at, and it holds the reader, never a frame.

use std::sync::Arc;

use waft_embedder::{Offset, Size, VideoFrameReader};
use waft_foundation::App;

use crate::box_::{BoxConstraints, RenderBox, RenderBoxData};
use crate::layer::{ErasedLayer, VideoLayer};
use crate::object::{RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;

/// Draws a video's frames in the box it is given.
pub struct RenderVideoFrameView {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    reader: Arc<dyn VideoFrameReader>,
}

impl RenderVideoFrameView {
    pub fn new(
        app: &mut App,
        reader: Arc<dyn VideoFrameReader>,
    ) -> RenderHandle<RenderVideoFrameView> {
        RenderHandle::new_box(
            app,
            RenderVideoFrameView {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                reader,
            },
        )
    }

    /// The reader whose frames are drawn.
    pub fn reader(self: RenderHandle<Self>, app: &App) -> &Arc<dyn VideoFrameReader> {
        &self.get(app).reader
    }

    /// Draws `reader`'s frames instead, from the next paint.
    pub fn set_reader(self: RenderHandle<Self>, app: &mut App, reader: Arc<dyn VideoFrameReader>) {
        if Arc::ptr_eq(&self.get(app).reader, &reader) {
            return;
        }
        self.get_mut(app).reader = reader;
        self.mark_needs_paint(app);
    }
}

impl RenderObject for RenderVideoFrameView {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let size = self.constraints(app).biggest();
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let reader = Arc::clone(&self.get(app).reader);
        let layer = VideoLayer::new(app, reader, offset & self.size(app));
        context.add_layer(app, layer.as_layer());
    }
}

impl RenderBox for RenderVideoFrameView {
    crate::render_box_accessors!();

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }

    /// A video takes a hit anywhere in its box, so a listener around it hears the pointer.
    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }
}
