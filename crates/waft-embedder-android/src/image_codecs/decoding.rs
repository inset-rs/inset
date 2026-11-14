//! A frame decoded into the app's memory: a decoder made over the image's bytes or its file, set
//! to RGBA, premultiplied, in sRGB, at the size asked, decoding the frame into rows the app then
//! holds. `AImageDecoder` turns the picture upright by the file's orientation as it decodes, for
//! the formats it does so for; for the others the host turns the decoded pixels (`orienting`).
//!
//! An animation's frames decode in order, each over the one before in the same memory, since a
//! frame may change only part of the picture: frame `n` is reached by decoding those before it.
//! The last frame decoded is kept, with its decoder and its memory, so an app that asks for the
//! frames in order at one size, as a player does, decodes each once; the app is handed a copy.
//!
//! The invariant: a kept frame is the one its memory holds, decoded at the size it was kept at;
//! one that failed halfway is not kept.

use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use waft_embedder::image_codecs::{
    AlphaType, ImageError, ImageErrorKind, ImageOrientation, PixelBuffer, PixelFormat, PixelLayout,
};

use super::image_decoder::ImageDecoder;
use super::orienting;

/// What an image was opened from, which each decoder reads again.
pub(super) enum Encoded {
    Bytes(Arc<[u8]>),
    File(PathBuf),
}

impl Encoded {
    /// A new decoder over the image: [`ImageErrorKind::NotFound`] for a path with no file.
    pub(super) fn decoder(&self) -> Result<ImageDecoder, ImageError> {
        match self {
            Encoded::Bytes(bytes) => ImageDecoder::over_bytes(Arc::clone(bytes)),
            Encoded::File(path) => {
                let file = File::open(path).map_err(|error| {
                    let kind = match error.kind() {
                        std::io::ErrorKind::NotFound => ImageErrorKind::NotFound,
                        _ => ImageErrorKind::Other,
                    };
                    ImageError::new(kind, format!("{}: {error}", path.display()))
                })?;
                ImageDecoder::over_file(file)
            }
        }
    }
}

/// The work of decoding an image's frames.
pub(super) struct Decoding {
    encoded: Encoded,
    /// Its size as the decoder decodes it at full size.
    size: [u32; 2],
    animated: bool,
    /// How the decoder's pixels are turned from upright: the file's orientation where the
    /// decoder leaves it to the host, upright where it applies it.
    left_turned: ImageOrientation,
    /// The last frame of an animation decoded, if it is not being decoded on from now.
    kept: Mutex<Option<Canvas>>,
}

impl Decoding {
    pub(super) fn new(
        encoded: Encoded,
        size: [u32; 2],
        animated: bool,
        left_turned: ImageOrientation,
    ) -> Decoding {
        Decoding {
            encoded,
            size,
            animated,
            left_turned,
            kept: Mutex::new(None),
        }
    }

    /// Frame `frame`, which the image has, upright at `size`.
    pub(super) fn frame(&self, frame: u32, size: [u32; 2]) -> Result<PixelBuffer, ImageError> {
        if self.left_turned == ImageOrientation::default() {
            return self.decoded_frame(frame, size);
        }
        let decoded = self.decoded_frame(frame, orienting::stored_size(size, self.left_turned))?;
        orienting::upright(&decoded, self.left_turned)
    }

    /// Frame `frame` as the decoder decodes it, at `size`.
    fn decoded_frame(&self, frame: u32, size: [u32; 2]) -> Result<PixelBuffer, ImageError> {
        if !self.animated {
            return self.first_frame(size)?.into_pixels();
        }
        let mut canvas = match self.kept_before(frame, size) {
            Some(canvas) => canvas,
            None => self.first_frame(size)?,
        };
        while canvas.frame < frame {
            canvas.decode_next()?;
        }
        let pixels = canvas.copied_pixels();
        *self.kept.lock().unwrap_or_else(PoisonError::into_inner) = Some(canvas);
        pixels
    }

    /// The kept frame, where it is `frame` or one before it, at `size`.
    fn kept_before(&self, frame: u32, size: [u32; 2]) -> Option<Canvas> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.take()
            .filter(|canvas| canvas.size == size && canvas.frame <= frame)
    }

    /// The first frame decoded at `size` by a decoder of its own.
    fn first_frame(&self, size: [u32; 2]) -> Result<Canvas, ImageError> {
        let mut decoder = self.encoded.decoder()?;
        decoder.set_rgba8_srgb()?;
        if size != self.size {
            decoder.set_target_size(size)?;
        }
        let stride = decoder.minimum_stride();
        let mut pixels = zeroed(stride, size)?;
        decoder.decode_into(&mut pixels, stride)?;
        Ok(Canvas {
            decoder,
            size,
            stride,
            frame: 0,
            pixels,
        })
    }
}

/// The first frame of the image `encoded` holds, at full size, as the decoder decodes it; none
/// where it does not decode.
pub(super) fn first_frame_of(encoded: Encoded) -> Option<PixelBuffer> {
    let size = encoded
        .decoder()
        .ok()?
        .header()
        .size
        .map(|side| u32::try_from(side).unwrap_or(0));
    if size.contains(&0) {
        return None;
    }
    let decoding = Decoding::new(encoded, size, false, ImageOrientation::default());
    decoding.frame(0, size).ok()
}

/// A frame decoded, in the memory the next frame decodes over.
struct Canvas {
    decoder: ImageDecoder,
    size: [u32; 2],
    stride: usize,
    frame: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn decode_next(&mut self) -> Result<(), ImageError> {
        if !self.decoder.advance_frame()? {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                format!("the image has no frame after {}", self.frame),
            ));
        }
        self.decoder.decode_into(&mut self.pixels, self.stride)?;
        self.frame += 1;
        Ok(())
    }

    fn layout(&self) -> PixelLayout {
        PixelLayout {
            size: self.size,
            row_bytes: self.stride as u32,
            format: PixelFormat::Rgba8,
            alpha: AlphaType::Premultiplied,
        }
    }

    fn into_pixels(self) -> Result<PixelBuffer, ImageError> {
        let layout = self.layout();
        pixel_buffer(layout, self.pixels)
    }

    fn copied_pixels(&self) -> Result<PixelBuffer, ImageError> {
        let mut copy = zeroed(self.stride, self.size)?;
        copy.copy_from_slice(&self.pixels);
        pixel_buffer(self.layout(), copy)
    }
}

/// Memory for `size[1]` rows of `stride` bytes: [`ImageErrorKind::TooLarge`] where no buffer
/// holds them, [`ImageErrorKind::OutOfMemory`] where the process cannot have them.
fn zeroed(stride: usize, size: [u32; 2]) -> Result<Vec<u8>, ImageError> {
    let length = stride
        .checked_mul(size[1] as usize)
        .filter(|_| u32::try_from(stride).is_ok())
        .ok_or_else(|| {
            ImageError::new(
                ImageErrorKind::TooLarge,
                format!(
                    "{} by {} is larger than a pixel buffer holds",
                    size[0], size[1]
                ),
            )
        })?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(length).map_err(|_| {
        ImageError::new(
            ImageErrorKind::OutOfMemory,
            format!("no memory for {length} bytes of pixels"),
        )
    })?;
    pixels.resize(length, 0);
    Ok(pixels)
}

fn pixel_buffer(layout: PixelLayout, pixels: Vec<u8>) -> Result<PixelBuffer, ImageError> {
    PixelBuffer::new(layout, pixels)
        .map_err(|error| ImageError::new(ImageErrorKind::Other, format!("{error:?}")))
}
