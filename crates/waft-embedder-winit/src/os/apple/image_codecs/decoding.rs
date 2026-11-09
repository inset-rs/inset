//! A frame decoded: Image I/O's thumbnail of it, made from the image itself rather than a small
//! picture the file carries, with the file's orientation applied, and no larger than the size
//! asked, which Image I/O takes as the length of the longer side. The thumbnail is drawn into the
//! app's memory, in RGBA, premultiplied, in sRGB; or into an `IOSurface` of the app's own, a
//! hardware buffer that holds no decoder's frame (`media::drawn_frames`), refused where it would
//! be larger than an `IOSurface` the system makes.

use std::sync::Arc;

use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFDictionary, CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use objc2_image_io::{
    kCGImageSourceCreateThumbnailFromImageAlways, kCGImageSourceCreateThumbnailWithTransform,
    kCGImageSourceShouldCacheImmediately, kCGImageSourceThumbnailMaxPixelSize,
};
use objc2_io_surface::{IOSurfaceRef, kIOSurfaceHeight, kIOSurfaceWidth};
use waft_embedder::Size;
use waft_embedder::image_codecs::{
    AlphaType, ImageError, ImageErrorKind, PixelBuffer, PixelFormat, PixelLayout,
};

use super::source::Source;
use crate::os::apple::hardware_buffers::SurfaceBuffer;
use crate::os::apple::media::as_ns_string;
use crate::os::apple::media::drawn_frames::buffer_of;

/// The work of decoding one frame at a size.
pub(super) struct Decoding {
    source: Arc<Source>,
    frame: u32,
    /// The size it decodes at, upright.
    size: [usize; 2],
    /// The longest side Image I/O is asked for; none for the full size.
    max_pixel_size: Option<usize>,
}

impl Decoding {
    /// Frame `frame` of `source`, whose image is `size` upright, scaled down to fit `max_size`;
    /// why not, for a size of no pixels.
    pub(super) fn new(
        source: Arc<Source>,
        frame: u32,
        size: Size,
        max_size: Option<Size>,
    ) -> Result<Decoding, ImageError> {
        let scale = match max_size {
            Some(max) if !(max.width() >= 1.0 && max.height() >= 1.0) => {
                return Err(ImageError::new(
                    ImageErrorKind::Other,
                    format!("no image fits in {max:?}"),
                ));
            }
            Some(max) => (max.width() / size.width())
                .min(max.height() / size.height())
                .min(1.0),
            None => 1.0,
        };
        let side = |length: f64| ((length * scale).round() as usize).max(1);
        let longer = size.width().max(size.height());
        Ok(Decoding {
            source,
            frame,
            size: [side(size.width()), side(size.height())],
            max_pixel_size: (scale < 1.0).then(|| side(longer)),
        })
    }

    /// The frame in the app's memory, in RGBA, premultiplied.
    pub(super) fn pixels(self) -> Result<PixelBuffer, ImageError> {
        let image = self.thumbnail()?;
        pixels_of(&image)
    }

    /// The frame in a hardware buffer of the app's own, an `IOSurface` in BGRA, premultiplied:
    /// [`ImageErrorKind::TooLarge`] where it is larger than an `IOSurface` the system makes.
    pub(super) fn buffer(self) -> Result<SurfaceBuffer, ImageError> {
        // SAFETY: constants IOSurface exports for the life of the process.
        let largest = unsafe {
            [
                IOSurfaceRef::property_maximum(kIOSurfaceWidth),
                IOSurfaceRef::property_maximum(kIOSurfaceHeight),
            ]
        };
        if self.size[0] > largest[0] || self.size[1] > largest[1] {
            return Err(ImageError::new(
                ImageErrorKind::TooLarge,
                format!(
                    "{} by {} is larger than an IOSurface, at most {} by {}",
                    self.size[0], self.size[1], largest[0], largest[1]
                ),
            ));
        }
        let image = self.thumbnail()?;
        buffer_of(&image).map_err(|message| ImageError::new(ImageErrorKind::Other, message))
    }

    /// Image I/O's thumbnail of the frame, at the size asked, upright.
    fn thumbnail(&self) -> Result<CFRetained<CGImage>, ImageError> {
        let options = self.thumbnail_options();
        let options: *const _ = &*options;
        // SAFETY: an NSDictionary is the CFDictionary it is toll-free bridged to.
        let options = unsafe { &*options.cast::<CFDictionary>() };
        // SAFETY: options of the types Image I/O's keys name, on a source of at least this many
        // frames.
        let image = unsafe {
            self.source
                .get()
                .thumbnail_at_index(self.frame as usize, Some(options))
        };
        image.ok_or_else(|| {
            ImageError::new(
                ImageErrorKind::Damaged,
                format!("Image I/O decoded no picture of frame {}", self.frame),
            )
        })
    }

    /// From the image itself, upright, decoded at once, and no longer on its longer side than
    /// asked.
    fn thumbnail_options(&self) -> objc2::rc::Retained<NSDictionary<NSString, AnyObject>> {
        let yes = NSNumber::new_bool(true);
        // SAFETY: constants Image I/O exports for the life of the process, CF strings.
        let (always, transform, immediately, longest) = unsafe {
            (
                as_ns_string(kCGImageSourceCreateThumbnailFromImageAlways),
                as_ns_string(kCGImageSourceCreateThumbnailWithTransform),
                as_ns_string(kCGImageSourceShouldCacheImmediately),
                as_ns_string(kCGImageSourceThumbnailMaxPixelSize),
            )
        };
        let mut keys = vec![always, transform, immediately];
        let mut values: Vec<&AnyObject> = vec![yes.as_ref(), yes.as_ref(), yes.as_ref()];
        let side = self.max_pixel_size.map(NSNumber::new_usize);
        if let Some(side) = &side {
            keys.push(longest);
            values.push(side.as_ref());
        }
        NSDictionary::from_slices(&keys, &values)
    }
}

/// `image` drawn into the app's memory at its own size, in RGBA, premultiplied, in sRGB.
fn pixels_of(image: &CGImage) -> Result<PixelBuffer, ImageError> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    let too_large = || {
        ImageError::new(
            ImageErrorKind::TooLarge,
            format!("{width} by {height} is larger than a pixel buffer holds"),
        )
    };
    let size = [
        u32::try_from(width).map_err(|_| too_large())?,
        u32::try_from(height).map_err(|_| too_large())?,
    ];
    let layout = PixelLayout::packed(size, PixelFormat::Rgba8, AlphaType::Premultiplied);
    let length = layout.byte_len().map_err(|_| too_large())?;
    let mut bytes = vec![0; length];
    // SAFETY: a constant Core Graphics exports for the life of the process.
    let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    // RGBA in memory: 32-bit words in big-endian order, alpha last and premultiplied.
    let bitmap = CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0;
    // SAFETY: `bytes` holds `height` rows of `width` pixels of four bytes, and outlives the
    // context, which is dropped at the end of this function.
    let context = unsafe {
        CGBitmapContextCreate(
            bytes.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            srgb.as_deref(),
            bitmap,
        )
    };
    let context = context.ok_or_else(|| {
        ImageError::new(
            ImageErrorKind::Other,
            "the system made no context to draw the picture",
        )
    })?;
    let whole = CGRect::new(CGPoint::ZERO, CGSize::new(width as f64, height as f64));
    CGContext::draw_image(Some(&context), whole, Some(image));
    drop(context);
    PixelBuffer::new(layout, bytes)
        .map_err(|error| ImageError::new(ImageErrorKind::Other, format!("{error:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source no test decodes: the sizes are worked out before any.
    fn decoding(size: Size, max_size: Option<Size>) -> Result<Decoding, ImageError> {
        let data = objc2_core_foundation::CFData::from_bytes(b"not an image");
        // SAFETY: no options.
        let source = unsafe { objc2_image_io::CGImageSource::with_data(&data, None) }.unwrap();
        Decoding::new(Arc::new(Source::of(source)), 0, size, max_size)
    }

    #[test]
    fn an_image_scales_down_to_fit_keeping_its_shape_and_never_up() {
        let fitted = decoding(Size::new(4000.0, 3000.0), Some(Size::new(400.0, 400.0))).unwrap();
        assert_eq!(fitted.size, [400, 300]);
        assert_eq!(fitted.max_pixel_size, Some(400));
        let tall = decoding(Size::new(3000.0, 4000.0), Some(Size::new(400.0, 200.0))).unwrap();
        assert_eq!(tall.size, [150, 200]);
        assert_eq!(tall.max_pixel_size, Some(200));
        let small = decoding(Size::new(64.0, 48.0), Some(Size::new(400.0, 400.0))).unwrap();
        assert_eq!((small.size, small.max_pixel_size), ([64, 48], None));
        let whole = decoding(Size::new(64.0, 48.0), None).unwrap();
        assert_eq!((whole.size, whole.max_pixel_size), ([64, 48], None));
        let nothing = decoding(Size::new(64.0, 48.0), Some(Size::new(0.0, 10.0)));
        assert_eq!(nothing.err().unwrap().kind, ImageErrorKind::Other);
    }
}
