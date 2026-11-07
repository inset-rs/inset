//! A video's frames drawn in the widget tree: Android's `TextureView` beside `SurfaceView`,
//! Flutter's `Texture`, over [`RenderVideoFrameView`].
//!
//! The framework draws the frames as part of its picture, so they clip, turn, fade and take
//! effects like anything painted, where a `PlayerView` has the system show a player on a
//! composition layer of its own. Pick this one for what a layer cannot take, glass over the video,
//! a fade into what is painted beneath it, a shader, or on a host with no compositor; the
//! `PlayerView` otherwise, the cheaper way and the only one for protected and HDR video. The
//! widget fills the box it is given; keeping the video's aspect is its parent's. The reader is the
//! widget's to read: a reader is read by one consumer, and the host reads it when it draws.

use std::fmt::{self, Debug};
use std::sync::Arc;

use waft_embedder::VideoFrameReader;
use waft_foundation::App;
use waft_rendering::{AnyRenderObject, RenderBox, RenderHandle, RenderVideoFrameView};

use crate::framework::{BuildContext, KeyRef, LeafRenderObjectWidget, RenderObjectWidget};

/// Draws a video's newest frame where it is placed.
#[derive(Clone)]
pub struct VideoFrameView {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The frames drawn: a reader the app made and gave a player or a camera session.
    pub reader: Arc<dyn VideoFrameReader>,
}

impl VideoFrameView {
    pub fn new(reader: Arc<dyn VideoFrameReader>) -> VideoFrameView {
        VideoFrameView { key: None, reader }
    }

    pub fn key(mut self, key: KeyRef) -> VideoFrameView {
        self.key = Some(key);
        self
    }
}

impl Debug for VideoFrameView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoFrameView")
            .field("reader", &Arc::as_ptr(&self.reader).cast::<()>())
            .finish()
    }
}

impl RenderObjectWidget for VideoFrameView {
    type RenderObject = RenderVideoFrameView;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderVideoFrameView::new(app, Arc::clone(&self.reader)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderVideoFrameView>,
    ) {
        render_object.set_reader(app, Arc::clone(&self.reader));
    }
}

impl LeafRenderObjectWidget for VideoFrameView {}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Layer, Rect, TargetPlatform, VideoFrame};
    use waft_foundation::AppCell;
    use waft_test::{TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::framework::IntoWidget;
    use crate::test_harness::pump_every_view;

    /// A reader with no frame yet.
    struct NoFrames;

    impl VideoFrameReader for NoFrames {
        fn frame_for(&self, _display_time: std::time::Instant) -> Option<Arc<dyn VideoFrame>> {
            None
        }
    }

    /// The widget puts the reader itself in the frame's scene, at the box it fills, for the host
    /// to take the frame from as it draws.
    #[test]
    fn a_video_frame_view_puts_its_reader_in_the_scene_at_its_box() {
        let view = Rc::new(TestView::new(800.0, 600.0));
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::clone(&view) as waft_embedder::ViewRef);
        let cell = AppCell::with_platform(Rc::new(platform));
        let reader: Arc<dyn VideoFrameReader> = Arc::new(NoFrames);
        run_app(
            &mut cell.borrow_mut(),
            crate::Center::new()
                .child(
                    crate::SizedBox::new()
                        .width(320.0)
                        .height(180.0)
                        .child(VideoFrameView::new(Arc::clone(&reader))),
                )
                .into_widget(),
        );
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);

        let scene = view.last_scene().expect("a frame was presented");
        let shown = std::cell::RefCell::new(Vec::new());
        waft_embedder::slices(&scene, |asked_for| {
            shown.borrow_mut().push(Arc::clone(asked_for));
            None
        });
        let shown = shown.into_inner();
        assert_eq!(shown.len(), 1);
        assert!(Arc::ptr_eq(&shown[0], &reader));
        let video = find_video_frames(&scene.root.children).expect("the reader's layer");
        assert_eq!(
            video.rect.size(),
            Rect::from_ltwh(0.0, 0.0, 320.0, 180.0).size()
        );
    }

    fn find_video_frames(layers: &[Layer]) -> Option<waft_embedder::VideoLayer> {
        layers.iter().find_map(|layer| match layer {
            Layer::Video(video) => Some(video.clone()),
            Layer::Container(container) => find_video_frames(&container.children),
            _ => None,
        })
    }
}
