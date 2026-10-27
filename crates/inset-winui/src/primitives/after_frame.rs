//! Work a control defers until a frame of its view has been drawn, and the frame it waits for:
//! Flutter's `addPostFrameCallback` with `ensureVisualUpdate`, for a view of its own.

use inset_embedder::ViewId;
use inset_foundation::App;
use inset_scheduler::{FrameCallback, FrameTarget, SchedulerBinding};

/// Runs `callback` at the end of the next frame of `view`, and asks for that frame. Work of no
/// view, as a control outside every view has, runs at the end of the next frame of any view, and
/// a frame of every view is asked for.
pub(crate) fn after_next_frame(app: &mut App, view: Option<ViewId>, callback: FrameCallback) {
    SchedulerBinding::add_post_frame_callback(app, callback, view);
    let target = match view {
        Some(view) => FrameTarget::View(view),
        None => FrameTarget::EveryView,
    };
    SchedulerBinding::ensure_visual_update(app, target);
}
