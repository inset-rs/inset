//! The NDK's encoder (`AndroidBitmap_compress`, `android/bitmap.h`), which `ndk-sys` binds; `ndk`
//! wraps it too, but sets no alpha flags, so straight alpha could not be said. Linked from
//! `libjnigraphics`: the call is of API 30.
//!
//! The invariant: the pixels are read only during the call, and every chunk Android writes is
//! copied out before the call returns.

// The NDK's C API is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::c_void;

use ndk_sys as ffi;
use waft_embedder::image_codecs::{AlphaType, ImageError, ImageErrorKind};

use super::formats::Compression;

/// Rows of RGBA, 8 bits a channel, in sRGB: `size[1]` rows of `stride` bytes, each starting with
/// `size[0]` pixels.
pub(super) struct RgbaRows<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) size: [u32; 2],
    pub(super) stride: u32,
    pub(super) alpha: AlphaType,
}

/// `rows` compressed as `compression` at `quality`, from 0 to 100.
pub(super) fn compressed(
    rows: &RgbaRows<'_>,
    compression: Compression,
    quality: i32,
) -> Result<Vec<u8>, ImageError> {
    let length = u64::from(rows.stride) * u64::from(rows.size[1]);
    if rows.stride < rows.size[0].saturating_mul(4) || (rows.bytes.len() as u64) < length {
        return Err(ImageError::new(
            ImageErrorKind::Unsupported,
            "pixels whose rows do not hold them",
        ));
    }
    let info = ffi::AndroidBitmapInfo {
        width: rows.size[0],
        height: rows.size[1],
        stride: rows.stride,
        format: ffi::AndroidBitmapFormat::ANDROID_BITMAP_FORMAT_RGBA_8888.0 as i32,
        flags: match rows.alpha {
            AlphaType::Premultiplied => ffi::ANDROID_BITMAP_FLAGS_ALPHA_PREMUL,
            AlphaType::Straight => ffi::ANDROID_BITMAP_FLAGS_ALPHA_UNPREMUL,
        },
    };
    let mut written: Vec<u8> = Vec::new();
    // SAFETY: the info describes the rows, which `bytes` holds, as checked above; the context is
    // `written`, which outlives the call, and the callback is called on this thread only during
    // it.
    let result = unsafe {
        ffi::AndroidBitmap_compress(
            &info,
            ffi::ADataSpace::ADATASPACE_SRGB.0,
            rows.bytes.as_ptr().cast(),
            format_of(compression),
            quality,
            (&raw mut written).cast(),
            Some(write_chunk),
        )
    };
    match result {
        ffi::ANDROID_BITMAP_RESULT_SUCCESS => Ok(written),
        ffi::ANDROID_BITMAP_RESULT_ALLOCATION_FAILED => Err(ImageError::new(
            ImageErrorKind::OutOfMemory,
            "AndroidBitmap_compress could not allocate",
        )),
        result => Err(ImageError::new(
            ImageErrorKind::Other,
            format!("AndroidBitmap_compress failed with {result}"),
        )),
    }
}

fn format_of(compression: Compression) -> i32 {
    let format = match compression {
        Compression::Jpeg => ffi::AndroidBitmapCompressFormat::ANDROID_BITMAP_COMPRESS_FORMAT_JPEG,
        Compression::Png => ffi::AndroidBitmapCompressFormat::ANDROID_BITMAP_COMPRESS_FORMAT_PNG,
        Compression::WebpLossy => {
            ffi::AndroidBitmapCompressFormat::ANDROID_BITMAP_COMPRESS_FORMAT_WEBP_LOSSY
        }
    };
    format.0 as i32
}

/// Appends a chunk Android wrote to the bytes `context` points at.
unsafe extern "C" fn write_chunk(context: *mut c_void, data: *const c_void, size: usize) -> bool {
    if size == 0 {
        return true;
    }
    // SAFETY: `context` is the `Vec` `compressed` handed over, and `data` holds `size` bytes for
    // the length of this call.
    unsafe {
        let written = &mut *context.cast::<Vec<u8>>();
        written.extend_from_slice(std::slice::from_raw_parts(data.cast::<u8>(), size));
    }
    true
}
