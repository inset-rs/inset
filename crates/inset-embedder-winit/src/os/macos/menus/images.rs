//! An image a menu item or a status item shows, as AppKit's `NSImage`: the RGBA pixels copied into
//! a bitmap, drawn at a height in points the place sets, a template drawn by its alpha in the
//! colour AppKit gives the place.

use inset_embedder::IconImage;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
use objc2_foundation::NSSize;

/// The height an item's image is drawn at in a menu, in points.
pub(super) const MENU_ITEM_HEIGHT: f64 = 16.0;

/// The height a status item's image is drawn at in the menu bar, in points.
pub(super) const STATUS_ITEM_HEIGHT: f64 = 18.0;

/// `icon` as an image `height` points high, its width in proportion; none for an empty icon or
/// pixels that do not fill it.
pub(super) fn ns_image(icon: &IconImage, height: f64) -> Option<Retained<NSImage>> {
    let (width, rows) = (icon.width as usize, icon.height as usize);
    let row_bytes = width.checked_mul(4)?;
    if width == 0 || rows == 0 || icon.rgba.len() < row_bytes.checked_mul(rows)? {
        return None;
    }
    // SAFETY: no planes are handed over, so the bitmap makes its own; the colour space is a
    // constant AppKit exports for the life of the process; the sizes describe 8-bit RGBA rows of
    // `row_bytes`, as the bitmap is given them below.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width as isize,
            rows as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied,
            row_bytes as isize,
            32,
        )
    }?;
    let data = bitmap.bitmapData();
    if data.is_null() {
        return None;
    }
    // SAFETY: the bitmap made its own buffer of `rows` rows of `row_bytes`, which nothing else
    // holds yet, and `icon.rgba` holds at least as many, checked above.
    unsafe { std::ptr::copy_nonoverlapping(icon.rgba.as_ptr(), data, row_bytes * rows) };
    let size = NSSize::new(height * width as f64 / rows as f64, height);
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    image.addRepresentation(&bitmap);
    image.setTemplate(icon.template);
    Some(image)
}
