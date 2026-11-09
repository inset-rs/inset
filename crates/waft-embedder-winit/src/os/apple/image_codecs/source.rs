//! An image Image I/O opened: its `CGImageSource`, on the image's bytes or on its file's URL, and
//! what the source's properties say of it, read as it opens: its format, its size upright, its
//! frames with an animation's delays, its EXIF orientation and its metadata (`exif`).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFData, CFDictionary, CFRetained, CFString, CFURL};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use objc2_image_io::{
    CGImageSource, CGImageSourceStatus, kCGImagePropertyAPNGDelayTime,
    kCGImagePropertyAPNGUnclampedDelayTime, kCGImagePropertyGIFDelayTime,
    kCGImagePropertyGIFDictionary, kCGImagePropertyGIFUnclampedDelayTime,
    kCGImagePropertyHEICSDelayTime, kCGImagePropertyHEICSDictionary,
    kCGImagePropertyHEICSUnclampedDelayTime, kCGImagePropertyOrientation,
    kCGImagePropertyPNGDictionary, kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth,
    kCGImagePropertyWebPDelayTime, kCGImagePropertyWebPDictionary,
    kCGImagePropertyWebPUnclampedDelayTime,
};
use waft_embedder::Size;
use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageFrame, ImageFuture, ImageMetadata, ImageOrientation,
    ImageRotation, ImageSource, PixelBuffer,
};

use super::decoding::Decoding;
use super::{exif, formats, in_background};
use crate::os::apple::media::as_ns_string;

/// A `CGImageSource`, which any thread reads: Image I/O's sources are safe to share between
/// threads.
pub(super) struct Source(CFRetained<CGImageSource>);

// SAFETY: Image I/O's functions are thread-safe, and a source is only read here, from whichever
// thread decodes it. The binding cannot say so.
unsafe impl Send for Source {}
unsafe impl Sync for Source {}

impl Source {
    pub(super) fn of(source: CFRetained<CGImageSource>) -> Source {
        Source(source)
    }

    pub(super) fn get(&self) -> &CGImageSource {
        &self.0
    }
}

/// An image Image I/O opened, with what it says of itself.
pub(super) struct ImageIoImage {
    source: Arc<Source>,
    mime_type: String,
    size: Size,
    frames: Vec<ImageFrame>,
    orientation: ImageOrientation,
    metadata: ImageMetadata,
}

impl ImageIoImage {
    /// The work of decoding frame `frame` at `max_size`; why not, for a frame it does not have or
    /// a size of no pixels.
    pub(super) fn decoding(
        &self,
        frame: u32,
        max_size: Option<Size>,
    ) -> Result<Decoding, ImageError> {
        if frame as usize >= self.frames.len() {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                format!("the image has no frame {frame}"),
            ));
        }
        Decoding::new(Arc::clone(&self.source), frame, self.size, max_size)
    }
}

impl ImageSource for ImageIoImage {
    fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    fn size(&self) -> Size {
        self.size
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
        let decoding = self.decoding(frame, max_size);
        in_background(move || decoding?.pixels())
    }
}

/// The image `data` encodes.
pub(super) fn open_data(data: &[u8]) -> Result<ImageIoImage, ImageError> {
    let data = CFData::from_bytes(data);
    // SAFETY: the source takes its own reference to the data, and no options.
    let source = unsafe { CGImageSource::with_data(&data, None) };
    read(source)
}

/// The image in the file at `path`, read only as far as it is asked.
pub(super) fn open_file(path: &Path) -> Result<ImageIoImage, ImageError> {
    if !path.is_file() {
        return Err(ImageError::new(
            ImageErrorKind::NotFound,
            format!("no file at {}", path.display()),
        ));
    }
    let url = CFURL::from_file_path(path).ok_or_else(|| {
        ImageError::new(
            ImageErrorKind::NotFound,
            format!("no URL names {}", path.display()),
        )
    })?;
    // SAFETY: a file URL, and no options.
    let source = unsafe { CGImageSource::with_url(&url, None) };
    read(source)
}

/// What `source` says of its image; why it says nothing.
fn read(source: Option<CFRetained<CGImageSource>>) -> Result<ImageIoImage, ImageError> {
    let source = source.ok_or_else(|| unsupported("Image I/O made no source of the image"))?;
    let mime_type = mime_type_of(&source)?;
    check_status(&source)?;
    // SAFETY: plain queries of a source made just now.
    let count = unsafe { source.count() };
    let first = properties_at(&source, 0).ok_or_else(|| damaged("the image has no frame"))?;
    let orientation = orientation_of(number_in(&first, unsafe { kCGImagePropertyOrientation }));
    let size = upright_size(&first, orientation)?;
    Ok(ImageIoImage {
        frames: frames_of(&source, count),
        metadata: exif::metadata_of(&first),
        source: Arc::new(Source::of(source)),
        mime_type,
        size,
        orientation,
    })
}

/// The MIME type of the format `source` read its bytes as; [`ImageErrorKind::Unsupported`] for
/// bytes of no format Image I/O reads.
fn mime_type_of(source: &CGImageSource) -> Result<String, ImageError> {
    // SAFETY: a plain query; none where Image I/O does not know the bytes.
    let uti =
        unsafe { source.r#type() }.ok_or_else(|| unsupported("of no format Image I/O reads"))?;
    // A format Apple names no MIME type for still opens: the app can decode what it cannot name.
    Ok(formats::mime_type_of(as_ns_string(&uti))
        .unwrap_or_else(|| "application/octet-stream".to_owned()))
}

/// Whether Image I/O could read `source`'s bytes: [`ImageErrorKind::Damaged`] where they are not
/// a whole image of their format.
fn check_status(source: &CGImageSource) -> Result<(), ImageError> {
    // SAFETY: a plain query.
    match unsafe { source.status() } {
        CGImageSourceStatus::StatusComplete => Ok(()),
        CGImageSourceStatus::StatusUnknownType => Err(unsupported("of no format Image I/O reads")),
        status => Err(damaged(&format!(
            "Image I/O reads the image no further, at status {}",
            status.0
        ))),
    }
}

/// The image's size upright, from its first frame's pixels and its orientation.
fn upright_size(first: &Properties, orientation: ImageOrientation) -> Result<Size, ImageError> {
    // SAFETY: constants Image I/O exports for the life of the process.
    let (width, height) = unsafe {
        (
            number_in(first, kCGImagePropertyPixelWidth),
            number_in(first, kCGImagePropertyPixelHeight),
        )
    };
    let (Some(width), Some(height)) = (width, height) else {
        return Err(damaged("the image says no size"));
    };
    Ok(match orientation.rotation {
        ImageRotation::QuarterTurn | ImageRotation::ThreeQuarterTurn => Size::new(height, width),
        ImageRotation::None | ImageRotation::HalfTurn => Size::new(width, height),
    })
}

/// EXIF's orientation, from 1 to 8, as how the stored picture is turned to show upright, as
/// ExifTool names each: mirrored left to right first, then turned clockwise. Upright where the
/// file says none, or a value EXIF does not have.
pub(super) fn orientation_of(exif: Option<f64>) -> ImageOrientation {
    let (mirrored, rotation) = match exif.map(|value| value as u8) {
        Some(2) => (true, ImageRotation::None),
        Some(3) => (false, ImageRotation::HalfTurn),
        Some(4) => (true, ImageRotation::HalfTurn),
        Some(5) => (true, ImageRotation::ThreeQuarterTurn),
        Some(6) => (false, ImageRotation::QuarterTurn),
        Some(7) => (true, ImageRotation::QuarterTurn),
        Some(8) => (false, ImageRotation::ThreeQuarterTurn),
        _ => (false, ImageRotation::None),
    };
    ImageOrientation { mirrored, rotation }
}

/// The source's `count` frames, each with its animation's delay where it is one of several and
/// its file gives one.
fn frames_of(source: &CGImageSource, count: usize) -> Vec<ImageFrame> {
    if count <= 1 {
        return vec![ImageFrame::default()];
    }
    (0..count)
        .map(|index| ImageFrame {
            duration: properties_at(source, index).and_then(|frame| delay_of(&frame)),
        })
        .collect()
}

/// How long a frame shows, from its format's own dictionary: the unclamped delay first, since
/// the clamped one raises a short delay to a tenth of a second, a browser's rule rather than what
/// the file says.
fn delay_of(frame: &Properties) -> Option<Duration> {
    // SAFETY: constants Image I/O exports for the life of the process.
    let formats = unsafe {
        [
            (
                kCGImagePropertyGIFDictionary,
                kCGImagePropertyGIFUnclampedDelayTime,
                kCGImagePropertyGIFDelayTime,
            ),
            (
                kCGImagePropertyPNGDictionary,
                kCGImagePropertyAPNGUnclampedDelayTime,
                kCGImagePropertyAPNGDelayTime,
            ),
            (
                kCGImagePropertyHEICSDictionary,
                kCGImagePropertyHEICSUnclampedDelayTime,
                kCGImagePropertyHEICSDelayTime,
            ),
            (
                kCGImagePropertyWebPDictionary,
                kCGImagePropertyWebPUnclampedDelayTime,
                kCGImagePropertyWebPDelayTime,
            ),
        ]
    };
    formats
        .into_iter()
        .find_map(|(dictionary, unclamped, clamped)| {
            let format = dictionary_in(frame, dictionary)?;
            [unclamped, clamped]
                .into_iter()
                .find_map(|key| number_in(&format, key).filter(|seconds| *seconds > 0.0))
                .map(Duration::from_secs_f64)
        })
}

/// A dictionary of properties, as Image I/O answers them.
pub(super) type Properties = NSDictionary<NSString, AnyObject>;

/// The properties of the source's frame `index`; none where it has none.
fn properties_at(source: &CGImageSource, index: usize) -> Option<Retained<Properties>> {
    // SAFETY: no options; the answer is a CF dictionary of CF strings to property lists.
    let properties = unsafe { source.properties_at_index(index, None) }?;
    Some(properties_of(properties))
}

/// `properties`, a CF dictionary, as the NSDictionary it is toll-free bridged to.
fn properties_of(properties: CFRetained<CFDictionary>) -> Retained<Properties> {
    let pointer = CFRetained::into_raw(properties).cast::<Properties>();
    // SAFETY: a CFDictionary is an NSDictionary, handed over with its one retain.
    unsafe { Retained::from_raw(pointer.as_ptr()) }.expect("a retained dictionary")
}

/// The dictionary under `key` in `properties`.
pub(super) fn dictionary_in(
    properties: &Properties,
    key: &CFString,
) -> Option<Retained<Properties>> {
    let value = properties.objectForKey(as_ns_string(key))?;
    value.downcast::<NSDictionary>().ok().map(|dictionary| {
        // SAFETY: Image I/O's nested dictionaries are keyed by strings too.
        unsafe { Retained::cast_unchecked::<Properties>(dictionary) }
    })
}

/// The number under `key` in `properties`.
pub(super) fn number_in(properties: &Properties, key: &CFString) -> Option<f64> {
    let value = properties.objectForKey(as_ns_string(key))?;
    value.downcast_ref::<NSNumber>().map(NSNumber::doubleValue)
}

/// The text under `key` in `properties`.
pub(super) fn string_in(properties: &Properties, key: &CFString) -> Option<String> {
    let value = properties.objectForKey(as_ns_string(key))?;
    value.downcast_ref::<NSString>().map(NSString::to_string)
}

/// The first number of the list under `key` in `properties`.
pub(super) fn first_number_in(properties: &Properties, key: &CFString) -> Option<f64> {
    let value = properties.objectForKey(as_ns_string(key))?;
    let list = value.downcast_ref::<objc2_foundation::NSArray>()?;
    let first = list.firstObject()?;
    first.downcast_ref::<NSNumber>().map(NSNumber::doubleValue)
}

fn unsupported(why: &str) -> ImageError {
    ImageError::new(ImageErrorKind::Unsupported, format!("the bytes are {why}"))
}

fn damaged(why: &str) -> ImageError {
    ImageError::new(ImageErrorKind::Damaged, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_exif_orientation_is_a_mirroring_and_a_turn() {
        let turned = |exif: f64| {
            let orientation = orientation_of(Some(exif));
            (orientation.mirrored, orientation.rotation)
        };
        assert_eq!(turned(1.0), (false, ImageRotation::None));
        assert_eq!(turned(3.0), (false, ImageRotation::HalfTurn));
        assert_eq!(turned(6.0), (false, ImageRotation::QuarterTurn));
        assert_eq!(turned(8.0), (false, ImageRotation::ThreeQuarterTurn));
        assert_eq!(turned(2.0), (true, ImageRotation::None));
        assert_eq!(turned(4.0), (true, ImageRotation::HalfTurn));
        assert_eq!(turned(5.0), (true, ImageRotation::ThreeQuarterTurn));
        assert_eq!(turned(7.0), (true, ImageRotation::QuarterTurn));
        assert_eq!(orientation_of(None), ImageOrientation::default());
        assert_eq!(orientation_of(Some(9.0)), ImageOrientation::default());
    }
}
