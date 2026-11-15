//! Flutter counterpart: the engine's `SnapshotDelegate::MakeRasterSnapshotSync`
//! (`lib/ui/snapshot_delegate.h`), which the rasterizer implements and `Picture.toImageSync`
//! draws through: what a host answers [`Platform::picture_to_image_sync`](crate::Platform) with.
//!
//! The invariant a later editor must keep: the picture is drawn with a renderer of the delegate's
//! own, on the device the host's images belong to, so an image it makes draws in every view and
//! no view's renderer is borrowed while the framework records a frame.

use std::cell::RefCell;

use crate::painting::{Image, Picture, valo};

/// Draws pictures into images on the device of a host's image store, with a renderer made at
/// the first picture.
pub struct SnapshotDelegate {
    images: valo::ImageContext,
    context: RefCell<Option<valo::Context>>,
}

impl SnapshotDelegate {
    /// A delegate drawing on the device `images` belongs to.
    pub fn new(images: valo::ImageContext) -> SnapshotDelegate {
        SnapshotDelegate {
            images,
            context: RefCell::new(None),
        }
    }

    /// `picture` drawn into a new image `width` by `height` pixels large over transparent black,
    /// what lies past those bounds clipped: valo's `display_list_to_image`, Impeller's
    /// `DisplayListToTexture`. The drawing is submitted now, and anything drawn with the image
    /// waits for it on the GPU.
    ///
    /// # Panics
    ///
    /// Where `width` or `height` is zero, as Dart throws "Invalid image dimensions.".
    pub fn make_raster_snapshot_sync(&self, picture: &Picture, width: u32, height: u32) -> Image {
        assert!(width > 0 && height > 0, "Invalid image dimensions.");
        let mut context = self.context.borrow_mut();
        let context = context.get_or_insert_with(|| {
            let mut context =
                valo::Context::new(self.images.device().clone(), self.images.queue().clone());
            context.set_hide_missing_glyphs(true);
            context
        });
        context.display_list_to_image(picture, [width, height])
    }
}

impl std::fmt::Debug for SnapshotDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotDelegate").finish_non_exhaustive()
    }
}
