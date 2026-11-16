//! A render object painted on its own, at the origin, into a layer of its own, and the pictures
//! and ops it records there: what a case hands the [`paints`](super::paints) matcher.

use std::sync::Arc;

use waft_embedder::{Offset, Picture, valo::Op};
use waft_foundation::App;

use crate::{
    AnyContainerLayer, AnyRenderObject, ContainerLayer, OffsetLayer, PaintingContext, PictureLayer,
};

/// The pictures `render` and its subtree record, painted at the origin: the top-level pictures
/// only, not those of the layers `render` pushes.
pub fn pictures(app: &mut App, render: AnyRenderObject) -> Vec<Arc<Picture>> {
    painted_pictures(app, render, false)
}

/// Every picture `render` records and every picture of the layers it pushes (repaint boundaries,
/// transforms, clips and the rest), depth first in paint order: what Dart's `paints` reads from
/// a `TestRecordingCanvas`, which paints a child that is a repaint boundary inline. Coordinates
/// are those of each picture's own layer.
pub fn pictures_deep(app: &mut App, render: AnyRenderObject) -> Vec<Arc<Picture>> {
    painted_pictures(app, render, true)
}

/// The ops of [`pictures`], flattened.
pub fn painted_ops(app: &mut App, render: AnyRenderObject) -> Vec<Op> {
    flattened(&pictures(app, render))
}

/// The ops of [`pictures_deep`], flattened.
pub fn painted_ops_deep(app: &mut App, render: AnyRenderObject) -> Vec<Op> {
    flattened(&pictures_deep(app, render))
}

fn flattened(pictures: &[Arc<Picture>]) -> Vec<Op> {
    pictures
        .iter()
        .flat_map(|picture| waft_embedder::flattened_ops(picture))
        .collect()
}

/// Paints `render` at the origin into a layer of its own and collects the pictures under it,
/// descending into pushed layers when `deep`.
fn painted_pictures(app: &mut App, render: AnyRenderObject, deep: bool) -> Vec<Arc<Picture>> {
    assert!(
        !render.is_repaint_boundary(app),
        "a repaint boundary would move its layer out of the scene"
    );
    let layer = OffsetLayer::new(app, Offset::ZERO);
    let container = layer.as_container_layer();
    let bounds = render.paint_bounds(app);
    let mut context = PaintingContext::new(container, bounds);
    context.paint_child(app, render, Offset::ZERO);
    context.stop_recording_if_needed(app);
    let mut pictures = Vec::new();
    collect_pictures(app, container, deep, &mut pictures);
    pictures
}

/// Adds the pictures of the layers under `container` to `pictures`, in order, and those of the
/// layers nested under them, depth first, when `deep`.
fn collect_pictures(
    app: &App,
    container: AnyContainerLayer,
    deep: bool,
    pictures: &mut Vec<Arc<Picture>>,
) {
    let mut child = container.first_child(app);
    while let Some(layer) = child {
        if let Some(picture) = app
            .handle::<PictureLayer>(layer.id())
            .and_then(|picture| picture.picture(app))
        {
            pictures.push(Arc::clone(picture));
        }
        if deep && let Some(nested) = layer.as_container_layer() {
            collect_pictures(app, nested, deep, pictures);
        }
        child = layer.next_sibling(app);
    }
}
