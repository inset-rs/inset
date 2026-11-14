//! An image `AImageDecoder` opened, on its bytes or on its file, and what it says of itself, read
//! as it opens: from the decoder's header its format and its size, upright, as the decoder turns
//! the picture by the file's orientation; its frames, with how long each of an animation's shows,
//! read by moving through them without decoding any; and from `ExifInterface`, which reads the
//! same file, the orientation the decoder applies and the metadata (`exif_interface`, `exif`).

use std::sync::Arc;

use waft_embedder::Size;
use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageFrame, ImageFuture, ImageMetadata, ImageOrientation,
    ImageSource, PixelBuffer,
};

use super::abilities::Abilities;
use super::decoding::{Decoding, Encoded};
use super::exif::{metadata_of, orientation_of};
use super::exif_interface::tags_of;
use super::image_decoder::ImageDecoder;
use super::{failed, in_background, orienting, target_size};
use crate::application::Application;

/// An image `AImageDecoder` opened, with what it says of itself.
pub(super) struct AndroidImage {
    mime_type: String,
    size: [u32; 2],
    frames: Vec<ImageFrame>,
    orientation: ImageOrientation,
    metadata: ImageMetadata,
    decoding: Arc<Decoding>,
}

impl ImageSource for AndroidImage {
    fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    fn size(&self) -> Size {
        Size::new(f64::from(self.size[0]), f64::from(self.size[1]))
    }

    fn frames(&self) -> Vec<ImageFrame> {
        self.frames.clone()
    }

    fn orientation(&self) -> ImageOrientation {
        self.orientation
    }

    fn metadata(&self) -> ImageMetadata {
        self.metadata.clone()
    }

    fn decode_pixels(&self, frame: u32, max_size: Option<Size>) -> ImageFuture<PixelBuffer> {
        if frame as usize >= self.frames.len() {
            return failed(ImageError::new(
                ImageErrorKind::Other,
                format!("the image has no frame {frame}"),
            ));
        }
        let size = match target_size::fitted(self.size, max_size) {
            Ok(size) => size,
            Err(error) => return failed(error),
        };
        let decoding = Arc::clone(&self.decoding);
        in_background(move || decoding.frame(frame, size))
    }
}

/// The image `encoded` holds, as its header, its frames and its EXIF say, the EXIF read through
/// `application`; upright, turned by the host where the decoder's `abilities` leave its format
/// unturned.
pub(super) fn open(
    encoded: Encoded,
    application: &Application,
    abilities: &Abilities,
) -> Result<AndroidImage, ImageError> {
    let mut decoder = encoded.decoder()?;
    let header = decoder.header();
    let decoded_size = header.size.map(|side| u32::try_from(side).unwrap_or(0));
    if decoded_size.contains(&0) {
        return Err(ImageError::new(
            ImageErrorKind::Damaged,
            format!("the image says a size of {:?}", header.size),
        ));
    }
    let animated = decoder.is_animated();
    let frames = if animated {
        frames_of(&mut decoder)?
    } else {
        vec![ImageFrame::default()]
    };
    let tags = tags_of(application, &encoded)?;
    let orientation = orientation_of(&tags);
    let left_turned = if abilities.leaves_unturned(&header.mime_type) {
        orientation
    } else {
        ImageOrientation::default()
    };
    // The decoder's size is as stored where it leaves the picture so; upright is its sides swapped
    // back for a quarter turn, as storing it swapped them.
    let size = orienting::stored_size(decoded_size, left_turned);
    Ok(AndroidImage {
        mime_type: header.mime_type,
        orientation,
        metadata: metadata_of(&tags),
        size,
        frames,
        decoding: Arc::new(Decoding::new(encoded, decoded_size, animated, left_turned)),
    })
}

/// An animation's frames, each with how long it shows, from its first to its last.
fn frames_of(decoder: &mut ImageDecoder) -> Result<Vec<ImageFrame>, ImageError> {
    let mut frames = Vec::new();
    loop {
        frames.push(ImageFrame {
            duration: decoder.frame_duration()?,
        });
        if !decoder.advance_frame()? {
            return Ok(frames);
        }
    }
}
