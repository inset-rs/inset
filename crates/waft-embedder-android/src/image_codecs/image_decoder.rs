//! The NDK's image decoder (`android/imagedecoder.h`), which `ndk-sys` binds and `ndk` does not
//! wrap: an `AImageDecoder` over an image's bytes or over its file, which it reads only as far as
//! it is asked, what its header says, and its frames, one after another, each decoded into memory
//! the caller holds. Linked from `libjnigraphics`: the calls are of API 30 and 31, and Android 12
//! (API 31) is the least the host runs on.
//!
//! The invariant: a decoder holds what it reads, the bytes or the open file, and lets it go only
//! after the decoder is deleted, since `AImageDecoder` reads them for as long as it lives.

// The NDK's C API is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_int};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::ptr::{self, NonNull};
use std::sync::Arc;
use std::time::Duration;

use ndk_sys as ffi;
use waft_embedder::image_codecs::{ImageError, ImageErrorKind};

/// What a decoder reads, held for as long as it reads it.
#[allow(
    dead_code,
    reason = "held for the decoder, which reads it through its own pointer"
)]
enum Input {
    Bytes(Arc<[u8]>),
    File(File),
}

/// An `AImageDecoder`, deleted when dropped.
pub(super) struct ImageDecoder {
    decoder: NonNull<ffi::AImageDecoder>,
    _input: Input,
}

// SAFETY: an `AImageDecoder` is not thread-safe and is never shared: each call that reaches it
// takes the wrapper by `&mut`, or reads its header, which it no longer changes once made, so one
// thread at a time uses it, whichever thread that is.
unsafe impl Send for ImageDecoder {}

/// What a decoder's header says of its image.
pub(super) struct Header {
    /// Its width and height in pixels, as the decoder decodes it at full size.
    pub(super) size: [i32; 2],
    /// Its format, as Android names it.
    pub(super) mime_type: String,
}

impl ImageDecoder {
    /// A decoder over `bytes`, an encoded image.
    pub(super) fn over_bytes(bytes: Arc<[u8]>) -> Result<ImageDecoder, ImageError> {
        let mut decoder = ptr::null_mut();
        // SAFETY: the bytes stay where they are, held by the decoder, until it is deleted.
        let result = unsafe {
            ffi::AImageDecoder_createFromBuffer(bytes.as_ptr().cast(), bytes.len(), &mut decoder)
        };
        Self::made(result, decoder, Input::Bytes(bytes))
    }

    /// A decoder over `file`, open at its start.
    pub(super) fn over_file(file: File) -> Result<ImageDecoder, ImageError> {
        let mut decoder = ptr::null_mut();
        // SAFETY: the descriptor stays open, held by the decoder, until it is deleted.
        let result = unsafe { ffi::AImageDecoder_createFromFd(file.as_raw_fd(), &mut decoder) };
        Self::made(result, decoder, Input::File(file))
    }

    fn made(
        result: c_int,
        decoder: *mut ffi::AImageDecoder,
        input: Input,
    ) -> Result<ImageDecoder, ImageError> {
        checked(result)?;
        let decoder = NonNull::new(decoder)
            .ok_or_else(|| failure(ffi::ANDROID_IMAGE_DECODER_INTERNAL_ERROR))?;
        Ok(ImageDecoder {
            decoder,
            _input: input,
        })
    }

    pub(super) fn header(&self) -> Header {
        // SAFETY: the header is the decoder's, read while it lives, and its MIME type a string
        // literal.
        unsafe {
            let header = ffi::AImageDecoder_getHeaderInfo(self.decoder.as_ptr());
            let mime_type = CStr::from_ptr(ffi::AImageDecoderHeaderInfo_getMimeType(header));
            Header {
                size: [
                    ffi::AImageDecoderHeaderInfo_getWidth(header),
                    ffi::AImageDecoderHeaderInfo_getHeight(header),
                ],
                mime_type: mime_type.to_string_lossy().into_owned(),
            }
        }
    }

    /// Whether the image has more than one frame, which may read past the first.
    pub(super) fn is_animated(&mut self) -> bool {
        // SAFETY: a decoder that lives.
        unsafe { ffi::AImageDecoder_isAnimated(self.decoder.as_ptr()) }
    }

    /// Decodes into RGBA, 8 bits a channel, premultiplied, the decoder's default, in sRGB; set
    /// while the decoder is on its first frame.
    pub(super) fn set_rgba8_srgb(&mut self) -> Result<(), ImageError> {
        let format = ffi::AndroidBitmapFormat::ANDROID_BITMAP_FORMAT_RGBA_8888.0 as i32;
        // SAFETY: a decoder that lives, and values of the NDK's enums.
        unsafe {
            checked(ffi::AImageDecoder_setAndroidBitmapFormat(
                self.decoder.as_ptr(),
                format,
            ))?;
            checked(ffi::AImageDecoder_setDataSpace(
                self.decoder.as_ptr(),
                ffi::ADataSpace::ADATASPACE_SRGB.0,
            ))
        }
    }

    /// Decodes scaled to `size`; set while the decoder is on its first frame.
    pub(super) fn set_target_size(&mut self, size: [u32; 2]) -> Result<(), ImageError> {
        let [width, height] = size.map(|side| i32::try_from(side).unwrap_or(i32::MAX));
        // SAFETY: a decoder that lives.
        checked(unsafe { ffi::AImageDecoder_setTargetSize(self.decoder.as_ptr(), width, height) })
    }

    /// The bytes of a row of what it decodes, with no padding.
    pub(super) fn minimum_stride(&mut self) -> usize {
        // SAFETY: a decoder that lives.
        unsafe { ffi::AImageDecoder_getMinimumStride(self.decoder.as_ptr()) }
    }

    /// Decodes the current frame into `pixels`, rows of `stride` bytes, over what they hold: an
    /// animation's frame may change only part of the one before.
    pub(super) fn decode_into(
        &mut self,
        pixels: &mut [u8],
        stride: usize,
    ) -> Result<(), ImageError> {
        // SAFETY: the decoder checks that `pixels.len()` bytes hold its rows of `stride`, and
        // writes no further.
        checked(unsafe {
            ffi::AImageDecoder_decodeImage(
                self.decoder.as_ptr(),
                pixels.as_mut_ptr().cast(),
                stride,
                pixels.len(),
            )
        })
    }

    /// Moves on to the next frame of an animation, with no decoding; `false` past the last.
    pub(super) fn advance_frame(&mut self) -> Result<bool, ImageError> {
        // SAFETY: a decoder that lives.
        match unsafe { ffi::AImageDecoder_advanceFrame(self.decoder.as_ptr()) } {
            ffi::ANDROID_IMAGE_DECODER_FINISHED => Ok(false),
            result => checked(result).map(|()| true),
        }
    }

    /// How long the current frame shows; none where the file gives it no time.
    pub(super) fn frame_duration(&mut self) -> Result<Option<Duration>, ImageError> {
        let info = FrameInfo::new()?;
        // SAFETY: a decoder that lives, and an info it fills, deleted after it is read.
        let nanoseconds = unsafe {
            checked(ffi::AImageDecoder_getFrameInfo(
                self.decoder.as_ptr(),
                info.0.as_ptr(),
            ))?;
            ffi::AImageDecoderFrameInfo_getDuration(info.0.as_ptr())
        };
        Ok(u64::try_from(nanoseconds)
            .ok()
            .filter(|nanoseconds| *nanoseconds > 0)
            .map(Duration::from_nanos))
    }
}

impl Drop for ImageDecoder {
    fn drop(&mut self) {
        // SAFETY: the decoder is this wrapper's, deleted once; what it read is let go after.
        unsafe { ffi::AImageDecoder_delete(self.decoder.as_ptr()) }
    }
}

/// An `AImageDecoderFrameInfo`, deleted when dropped.
struct FrameInfo(NonNull<ffi::AImageDecoderFrameInfo>);

impl FrameInfo {
    fn new() -> Result<FrameInfo, ImageError> {
        // SAFETY: a plain allocation, which may fail.
        let info = unsafe { ffi::AImageDecoderFrameInfo_create() };
        NonNull::new(info)
            .map(FrameInfo)
            .ok_or_else(|| failure(ffi::ANDROID_IMAGE_DECODER_INTERNAL_ERROR))
    }
}

impl Drop for FrameInfo {
    fn drop(&mut self) {
        // SAFETY: the info is this wrapper's, deleted once.
        unsafe { ffi::AImageDecoderFrameInfo_delete(self.0.as_ptr()) }
    }
}

fn checked(result: c_int) -> Result<(), ImageError> {
    if result == ffi::ANDROID_IMAGE_DECODER_SUCCESS {
        Ok(())
    } else {
        Err(failure(result))
    }
}

/// `result`, a code `AImageDecoder` failed with, as the kind of failure it is and its name.
fn failure(result: c_int) -> ImageError {
    let kind = match result {
        ffi::ANDROID_IMAGE_DECODER_UNSUPPORTED_FORMAT
        | ffi::ANDROID_IMAGE_DECODER_INVALID_CONVERSION => ImageErrorKind::Unsupported,
        ffi::ANDROID_IMAGE_DECODER_INCOMPLETE
        | ffi::ANDROID_IMAGE_DECODER_ERROR
        | ffi::ANDROID_IMAGE_DECODER_INVALID_INPUT => ImageErrorKind::Damaged,
        _ => ImageErrorKind::Other,
    };
    // SAFETY: a plain query, answering a string literal or null for a code it does not know.
    let name = unsafe { ffi::AImageDecoder_resultToString(result) };
    let name = if name.is_null() {
        format!("result {result}")
    } else {
        // SAFETY: a string literal, which lives for the process.
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    };
    ImageError::new(kind, format!("AImageDecoder: {name}"))
}
