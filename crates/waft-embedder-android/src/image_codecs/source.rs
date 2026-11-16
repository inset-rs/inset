//! An image `AImageDecoder` opened, on its bytes or on its file, and what it says of itself, read
//! as it opens: from the decoder's header its format and its size, upright, as the decoder turns
//! the picture by the file's orientation; whether it is an animation, and an animation's frames,
//! with how long each shows, read by moving through them without decoding any, and how many times
//! it plays; and from `ExifInterface`, which reads the same file, the orientation the decoder
//! applies and the metadata (`exif_interface`, `exif`). The decoder reads one picture of a file of
//! several that does not animate, so an image here is never a document of pages.

use std::sync::Arc;

use waft_embedder::Size;
use waft_embedder::image_codecs::{
    AnimationFrame, ImageAnimation, ImageError, ImageErrorKind, ImageFuture, ImageMetadata,
    ImageOrientation, ImagePictures, ImageSource, PixelBuffer,
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
    pictures: ImagePictures,
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

    fn pictures(&self) -> ImagePictures {
        self.pictures.clone()
    }

    fn orientation(&self) -> ImageOrientation {
        self.orientation
    }

    fn metadata(&self) -> ImageMetadata {
        self.metadata.clone()
    }

    fn decode_pixels(&self, index: u32, max_size: Option<Size>) -> ImageFuture<PixelBuffer> {
        if index as usize >= self.pictures.count() {
            return failed(ImageError::new(
                ImageErrorKind::Other,
                format!("the image has no picture {index}"),
            ));
        }
        let size = match target_size::fitted(self.size, max_size) {
            Ok(size) => size,
            Err(error) => return failed(error),
        };
        let decoding = Arc::clone(&self.decoding);
        in_background(move || decoding.frame(index, size))
    }
}

/// The image `encoded` holds, as its header, its animation and its EXIF say, the EXIF read through
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
    let pictures = if animated {
        ImagePictures::Animation(animation_of(&mut decoder)?)
    } else {
        ImagePictures::Still
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
        pictures,
        decoding: Arc::new(Decoding::new(encoded, decoded_size, animated, left_turned)),
    })
}

/// An animation's frames and how many times it plays, read while the decoder is on its first
/// frame, which may need it to seek.
fn animation_of(decoder: &mut ImageDecoder) -> Result<ImageAnimation, ImageError> {
    let plays = decoder.plays()?;
    Ok(ImageAnimation {
        frames: frames_of(decoder)?,
        plays,
    })
}

/// An animation's frames, each with how long it shows, from its first to its last.
fn frames_of(decoder: &mut ImageDecoder) -> Result<Vec<AnimationFrame>, ImageError> {
    let mut frames = Vec::new();
    loop {
        frames.push(AnimationFrame {
            duration: decoder.frame_duration()?,
        });
        if !decoder.advance_frame()? {
            return Ok(frames);
        }
    }
}
