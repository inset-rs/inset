//! The frames of the videos a view shows, as its rasterizer draws them: for each reader a draw
//! asked for, the image of the frame on screen. A reader is asked for the frame of the moment the
//! picture reaches the screen when a draw first asks for it, and at each vsync after that, which
//! redraws the view when a new one came, as Apple's display link asks a player's video output for
//! the frame of each refresh's target time.
//!
//! The invariant a later editor must keep: a reader's frame is held only while the view's draws
//! ask for the reader. Once a draw no longer does, its entry goes, and with it the frame's place
//! in its decoder's pool.

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;
use std::time::Instant;

use inset_embedder::{Image, VideoFrameReader};

use crate::gpu::Gpu;
use crate::os::VideoFrameImages;
use crate::os::capabilities::VideoFrameImages as _;

/// The videos one view shows, and the frame each shows now.
#[derive(Default)]
pub(crate) struct ShownVideoFrames {
    /// What makes the images, on the device the view draws with: made when a frame first needs
    /// it, `None` on a system that hands out no frames.
    images: OnceCell<Option<VideoFrameImages>>,
    shown: RefCell<Vec<ShownVideo>>,
}

/// One reader the view's draws ask for.
struct ShownVideo {
    reader: Arc<dyn VideoFrameReader>,
    /// The image of the frame on screen; `None` before the reader's first frame.
    image: Option<Image>,
    /// Whether the draw being made asked for the reader.
    asked: bool,
}

impl ShownVideoFrames {
    /// A draw of the view begins: it asks for the readers it shows through
    /// [`image_of`](Self::image_of), and [`end_draw`](Self::end_draw) lets go of the others.
    pub(crate) fn begin_draw(&self) {
        for video in self.shown.borrow_mut().iter_mut() {
            video.asked = false;
        }
    }

    /// The image of the frame `reader` shows in the draw being made: the one on screen, or, for
    /// a reader no draw asked for before, its frame for `display_time`.
    pub(crate) fn image_of(
        &self,
        gpu: &Gpu,
        reader: &Arc<dyn VideoFrameReader>,
        display_time: Instant,
    ) -> Option<Image> {
        let mut shown = self.shown.borrow_mut();
        if let Some(video) = shown
            .iter_mut()
            .find(|video| Arc::ptr_eq(&video.reader, reader))
        {
            video.asked = true;
            return video.image.clone();
        }
        let image = self.frame_image(gpu, reader, display_time);
        shown.push(ShownVideo {
            reader: Arc::clone(reader),
            image: image.clone(),
            asked: true,
        });
        image
    }

    /// The draw is made: the readers it did not ask for are let go, with their frames.
    pub(crate) fn end_draw(&self) {
        self.shown.borrow_mut().retain(|video| video.asked);
    }

    /// Whether the view shows a video, whose new frames it takes at each vsync.
    pub(crate) fn shows_video(&self) -> bool {
        !self.shown.borrow().is_empty()
    }

    /// Takes each shown reader's frame for `display_time`, where it is a new one; whether any
    /// was, so the view is drawn again.
    pub(crate) fn take_new_frames(&self, gpu: &Gpu, display_time: Instant) -> bool {
        let mut came = false;
        for video in self.shown.borrow_mut().iter_mut() {
            if let Some(image) = self.frame_image(gpu, &video.reader, display_time) {
                video.image = Some(image);
                came = true;
            }
        }
        came
    }

    /// Lets go of every reader and its frame: the view's window closed.
    pub(crate) fn forget_all(&self) {
        self.shown.borrow_mut().clear();
    }

    /// The image of `reader`'s frame for `display_time`, if it is a new one.
    fn frame_image(
        &self,
        gpu: &Gpu,
        reader: &Arc<dyn VideoFrameReader>,
        display_time: Instant,
    ) -> Option<Image> {
        let frame = reader.frame_for(display_time)?;
        let images = self.images.get_or_init(|| VideoFrameImages::on(gpu));
        images.as_ref()?.image_of(&frame)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use inset_embedder::VideoFrame;

    use super::*;
    use crate::gpu::LazyGpu;

    /// A reader with no frame yet, which keeps the display times it is asked for.
    #[derive(Default)]
    struct NoFrames(Mutex<Vec<Instant>>);

    impl VideoFrameReader for NoFrames {
        fn frame_for(&self, display_time: Instant) -> Option<VideoFrame> {
            self.0.lock().unwrap().push(display_time);
            None
        }
    }

    #[test]
    fn a_reader_no_draw_asks_for_is_let_go() {
        let gpu = LazyGpu::default();
        let shown = ShownVideoFrames::default();
        let reader: Arc<dyn VideoFrameReader> = Arc::new(NoFrames::default());
        shown.begin_draw();
        assert!(
            shown.image_of(gpu.get(), &reader, Instant::now()).is_none(),
            "no frame yet"
        );
        shown.end_draw();
        assert!(
            shown.shows_video(),
            "asked for, it is kept to take its first frame"
        );
        shown.begin_draw();
        shown.end_draw();
        assert!(!shown.shows_video());
    }

    /// A reader is asked for the frame of the moment the picture reaches the screen, at its first
    /// draw and at each vsync after.
    #[test]
    fn a_reader_is_asked_for_the_frame_of_the_display_time() {
        let gpu = LazyGpu::default();
        let shown = ShownVideoFrames::default();
        let frames = Arc::new(NoFrames::default());
        let reader: Arc<dyn VideoFrameReader> = frames.clone();
        let first = Instant::now() + Duration::from_millis(16);
        let next = first + Duration::from_millis(16);
        shown.begin_draw();
        shown.image_of(gpu.get(), &reader, first);
        shown.end_draw();
        shown.take_new_frames(gpu.get(), next);
        assert_eq!(*frames.0.lock().unwrap(), [first, next]);
    }

    /// A reader's frame is taken at its first draw, and at a vsync only once a new one came,
    /// which is when the view is drawn again.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn a_vsync_takes_a_frame_only_when_a_new_one_came() {
        let gpu = LazyGpu::default();
        let shown = ShownVideoFrames::default();
        let frames = Arc::new(crate::os::TestFrames::default());
        let reader: Arc<dyn VideoFrameReader> = frames.clone();
        frames.put_new();
        shown.begin_draw();
        assert!(shown.image_of(gpu.get(), &reader, Instant::now()).is_some());
        shown.end_draw();
        assert!(
            !shown.take_new_frames(gpu.get(), Instant::now()),
            "nothing new"
        );
        frames.put_new();
        assert!(shown.take_new_frames(gpu.get(), Instant::now()));
    }
}
