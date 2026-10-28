//! An icon image as Windows' GDI objects: a 32-bit top-down bitmap for a menu item, its alpha
//! premultiplied, and an icon for the notification area, made from such a bitmap with its alpha as
//! it is. Each is released when its owner drops it.

use std::ptr;

use inset_embedder::IconImage;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, HBITMAP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, HICON, ICONINFO,
};

use crate::menus::pixels::{bgra, bgra_premultiplied};

/// A bitmap of the embedder's, deleted when dropped.
pub(super) struct Bitmap(HBITMAP);

impl Bitmap {
    pub(super) fn handle(&self) -> HBITMAP {
        self.0
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: the bitmap is the embedder's own, and nothing selects it into a device context.
        unsafe { DeleteObject(self.0) };
    }
}

/// An icon of the embedder's, destroyed when dropped.
pub(super) struct Icon(HICON);

impl Icon {
    pub(super) fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY: the icon is the embedder's own, made by `CreateIconIndirect`.
        unsafe { DestroyIcon(self.0) };
    }
}

/// `image` as a menu item's bitmap, at its own size in pixels.
pub(super) fn menu_item_bitmap(image: &IconImage) -> Option<Bitmap> {
    dib_section(image, &bgra_premultiplied(image))
}

/// `image` as an icon, which the notification area scales to its own size.
pub(super) fn icon(image: &IconImage) -> Option<Icon> {
    let colour = dib_section(image, &bgra(image))?;
    let mask = transparent_mask(image)?;
    let info = ICONINFO {
        fIcon: 1,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask.handle(),
        hbmColor: colour.handle(),
    };
    // SAFETY: both bitmaps are alive for the call, which copies them.
    let icon = unsafe { CreateIconIndirect(&info) };
    (!icon.is_null()).then_some(Icon(icon))
}

/// A 32-bit top-down bitmap of `image`'s size holding `pixels`, four bytes a pixel.
fn dib_section(image: &IconImage, pixels: &[u8]) -> Option<Bitmap> {
    let (width, height) = (
        i32::try_from(image.width).ok()?,
        i32::try_from(image.height).ok()?,
    );
    if width == 0 || height == 0 || pixels.len() != image.width as usize * image.height as usize * 4
    {
        return None;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let mut bits = ptr::null_mut();
    // SAFETY: the header describes a 32-bit top-down bitmap; without a device context or a section
    // the system allocates the bits and answers where.
    let bitmap = unsafe {
        CreateDIBSection(
            ptr::null_mut(),
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            ptr::null_mut(),
            0,
        )
    };
    if bitmap.is_null() || bits.is_null() {
        return None;
    }
    // SAFETY: the bits are the bitmap's, width times height pixels of four bytes, the length of
    // `pixels`, checked above.
    unsafe { ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len()) };
    Some(Bitmap(bitmap))
}

/// An icon's mask that hides nothing: Windows draws a 32-bit icon by its alpha.
fn transparent_mask(image: &IconImage) -> Option<Bitmap> {
    let (width, height) = (
        i32::try_from(image.width).ok()?,
        i32::try_from(image.height).ok()?,
    );
    // A monochrome bitmap's rows are padded to 16 bits.
    let row_bytes = image.width.div_ceil(16) as usize * 2;
    let zeros = vec![0u8; row_bytes * image.height as usize];
    // SAFETY: the bits hold `height` rows of `row_bytes` bytes, as a one-bit bitmap's rows are.
    let mask = unsafe { CreateBitmap(width, height, 1, 1, zeros.as_ptr().cast()) };
    (!mask.is_null()).then_some(Bitmap(mask))
}
