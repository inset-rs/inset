//! IOSurfaces a producer draws into on the CPU: allocated as BGRA, filled a row at a time.

// Allocating an IOSurface and writing its pixels is what this module is for; the rest of the
// example has no `unsafe`.
#![allow(unsafe_code)]

use std::ptr::NonNull;

use inset_embedder::Buffer;
use objc2_core_foundation::{CFDictionary, CFNumber, CFString};
use objc2_io_surface::{
    IOSurfaceLockOptions, IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow,
    kIOSurfaceHeight, kIOSurfacePixelFormat, kIOSurfaceWidth,
};

/// The `OSType` for 32-bit BGRA.
const BGRA: i32 = 0x4247_5241;
const BYTES_PER_PIXEL: usize = 4;

/// The frame's colours, as BGRA.
const BACKGROUND: [u8; 4] = [120, 60, 30, 255];
const STRIPE: [u8; 4] = [240, 240, 240, 255];
/// The stripe's width and how far it moves each frame, in pixels.
const STRIPE_WIDTH: usize = 48;
const STRIPE_STEP: u64 = 8;

/// A BGRA surface of `width` by `height` pixels.
pub(crate) fn allocate(width: usize, height: usize) -> Buffer {
    // SAFETY: the keys are constants IOSurface exports for the life of the process.
    let keys: [&CFString; 5] = unsafe {
        [
            kIOSurfaceWidth,
            kIOSurfaceHeight,
            kIOSurfaceBytesPerElement,
            kIOSurfaceBytesPerRow,
            kIOSurfacePixelFormat,
        ]
    };
    let bytes_per_row = IOSurfaceRef::align_property(keys[3], width * BYTES_PER_PIXEL);
    let numbers = [
        CFNumber::new_isize(width as isize),
        CFNumber::new_isize(height as isize),
        CFNumber::new_isize(BYTES_PER_PIXEL as isize),
        CFNumber::new_isize(bytes_per_row as isize),
        CFNumber::new_i32(BGRA),
    ];
    let values: [&CFNumber; 5] = numbers.each_ref().map(|number| &**number);
    let properties = CFDictionary::from_slices(&keys, &values);
    // SAFETY: every value is of the type its key names.
    unsafe { IOSurfaceRef::new(properties.as_opaque()) }.expect("an IOSurface")
}

/// Draws frame `frame` into `surface`: the background, with the stripe where it has moved to.
pub(crate) fn fill(surface: &IOSurfaceRef, frame: u64) {
    let width = surface.width();
    let row = row_of(width, frame);
    with_pixels(surface, |base, bytes_per_row| {
        for y in 0..surface.height() {
            // SAFETY: row `y` of a locked surface is `bytes_per_row` bytes at this offset, of
            // which the first `width` pixels are the image.
            let destination = unsafe {
                std::slice::from_raw_parts_mut(base.as_ptr().add(y * bytes_per_row), row.len())
            };
            destination.copy_from_slice(&row);
        }
    });
}

/// One row of frame `frame`, which every row repeats.
fn row_of(width: usize, frame: u64) -> Vec<u8> {
    let stripe_left = (frame * STRIPE_STEP) as usize % width.max(1);
    (0..width)
        .flat_map(|x| {
            let in_stripe = x >= stripe_left && x < stripe_left + STRIPE_WIDTH;
            if in_stripe { STRIPE } else { BACKGROUND }
        })
        .collect()
}

/// Runs `write` with the surface locked for the CPU: its base address and row stride.
fn with_pixels(surface: &IOSurfaceRef, write: impl FnOnce(NonNull<u8>, usize)) {
    // SAFETY: a null seed is allowed; the lock is released below before returning.
    let locked = unsafe { surface.lock(IOSurfaceLockOptions::empty(), std::ptr::null_mut()) };
    assert_eq!(locked, 0, "locking an IOSurface for the CPU");
    write(surface.base_address().cast(), surface.bytes_per_row());
    // SAFETY: as above.
    unsafe { surface.unlock(IOSurfaceLockOptions::empty(), std::ptr::null_mut()) };
}
