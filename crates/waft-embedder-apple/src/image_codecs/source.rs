//! An image Image I/O opened: its `CGImageSource`, on the image's bytes or on its file's URL, and
//! what the source's properties say of it, read as it opens: its format; how its pictures go; and
//! its primary picture's size upright, EXIF orientation and metadata (`exif`).
//!
//! Image I/O gives every animation a loop count in its format's dictionary of the whole file, a
//! GIF without a NETSCAPE extension included, which plays once: a count of plays, zero meaning
//! forever. A file of several pictures with no loop count is a document of pages, as a TIFF and a
//! HEIF collection are, each page with its own size and orientation, and one named its primary.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFData, CFDictionary, CFRetained, CFString, CFURL};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use objc2_image_io::{
    CGImageSource, CGImageSourceStatus, kCGImagePropertyAPNGDelayTime,
    kCGImagePropertyAPNGLoopCount, kCGImagePropertyAPNGUnclampedDelayTime,
    kCGImagePropertyGIFDelayTime, kCGImagePropertyGIFDictionary, kCGImagePropertyGIFLoopCount,
    kCGImagePropertyGIFUnclampedDelayTime, kCGImagePropertyHEICSDelayTime,
    kCGImagePropertyHEICSDictionary, kCGImagePropertyHEICSLoopCount,
    kCGImagePropertyHEICSUnclampedDelayTime, kCGImagePropertyOrientation,
    kCGImagePropertyPNGDictionary, kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth,
    kCGImagePropertyWebPDelayTime, kCGImagePropertyWebPDictionary, kCGImagePropertyWebPLoopCount,
    kCGImagePropertyWebPUnclampedDelayTime,
};
use waft_embedder::Size;
use waft_embedder::image_codecs::{
    AnimationFrame, AnimationPlays, ImageAnimation, ImageError, ImageErrorKind, ImageFuture,
    ImageMetadata, ImageOrientation, ImagePage, ImagePages, ImagePictures, ImageRotation,
    ImageSource, PixelBuffer,
};

use super::decoding::Decoding;
use super::{exif, formats, in_background};
use crate::media::as_ns_string;

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
    pictures: ImagePictures,
    orientation: ImageOrientation,
    metadata: ImageMetadata,
}

impl ImageIoImage {
    /// The work of decoding picture `index` at `max_size`; why not, for a picture it does not
    /// have or a size of no pixels.
    pub(super) fn decoding(
        &self,
        index: u32,
        max_size: Option<Size>,
    ) -> Result<Decoding, ImageError> {
        if index as usize >= self.pictures.count() {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                format!("the image has no picture {index}"),
            ));
        }
        let size = self.size_of_picture(index);
        Decoding::new(Arc::clone(&self.source), index, size, max_size)
    }

    /// The size picture `index` decodes at, upright: a page's own, and the image's otherwise.
    fn size_of_picture(&self, index: u32) -> Size {
        match &self.pictures {
            ImagePictures::Pages(pages) => pages.pages[index as usize].size,
            ImagePictures::Still | ImagePictures::Animation(_) => self.size,
        }
    }
}

impl ImageSource for ImageIoImage {
    fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    fn size(&self) -> Size {
        self.size
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
        let decoding = self.decoding(index, max_size);
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
    let pictures = pictures_of(&source)?;
    let primary = properties_at(&source, primary_of(&pictures))
        .ok_or_else(|| damaged("the image has no picture"))?;
    let orientation = orientation_in(&primary);
    let size = upright_size(&primary, orientation)?;
    Ok(ImageIoImage {
        pictures,
        metadata: exif::metadata_of(&primary),
        source: Arc::new(Source::of(source)),
        mime_type,
        size,
        orientation,
    })
}

/// How `source`'s pictures go: an animation where its format's dictionary of the whole file has
/// a loop count, a document of pages where it has several pictures and none, and one still
/// picture otherwise.
fn pictures_of(source: &CGImageSource) -> Result<ImagePictures, ImageError> {
    // SAFETY: a plain query of a source made just now.
    let count = unsafe { source.count() };
    if count <= 1 {
        return Ok(ImagePictures::Still);
    }
    Ok(match loop_count_of(source) {
        Some(loop_count) => ImagePictures::Animation(ImageAnimation {
            frames: frames_of(source, count),
            plays: plays_of(loop_count),
        }),
        None => ImagePictures::Pages(pages_of(source, count)?),
    })
}

/// The index of the picture the image's size, orientation and metadata are read from: a
/// document's primary page, and the first picture otherwise.
fn primary_of(pictures: &ImagePictures) -> usize {
    match pictures {
        ImagePictures::Pages(pages) => pages.primary as usize,
        ImagePictures::Still | ImagePictures::Animation(_) => 0,
    }
}

/// The loop count of `source`'s animation, from its format's dictionary of the whole file; none
/// where it is not an animation.
fn loop_count_of(source: &CGImageSource) -> Option<f64> {
    // SAFETY: no options; the answer is a CF dictionary of CF strings to property lists.
    let properties = properties_of(unsafe { source.properties(None) }?);
    // SAFETY: constants Image I/O exports for the life of the process.
    let formats = unsafe {
        [
            (kCGImagePropertyGIFDictionary, kCGImagePropertyGIFLoopCount),
            (kCGImagePropertyPNGDictionary, kCGImagePropertyAPNGLoopCount),
            (
                kCGImagePropertyHEICSDictionary,
                kCGImagePropertyHEICSLoopCount,
            ),
            (
                kCGImagePropertyWebPDictionary,
                kCGImagePropertyWebPLoopCount,
            ),
        ]
    };
    formats.into_iter().find_map(|(dictionary, loop_count)| {
        number_in(&*dictionary_in(&properties, dictionary)?, loop_count)
    })
}

/// How many times an animation with Image I/O's loop count plays: the count is of plays, and zero
/// is forever.
fn plays_of(loop_count: f64) -> AnimationPlays {
    if loop_count >= 1.0 {
        AnimationPlays::Times(loop_count as u32)
    } else {
        AnimationPlays::Forever
    }
}

/// The document's `count` pages, each with its own size and orientation, and the one Image I/O
/// names its primary.
fn pages_of(source: &CGImageSource, count: usize) -> Result<ImagePages, ImageError> {
    let pages = (0..count)
        .map(|index| {
            let page = properties_at(source, index)
                .ok_or_else(|| damaged(&format!("the image says nothing of page {index}")))?;
            let orientation = orientation_in(&page);
            let size = upright_size(&page, orientation)?;
            Ok(ImagePage { size, orientation })
        })
        .collect::<Result<Vec<_>, ImageError>>()?;
    // SAFETY: a plain query; the first for every format but HEIF.
    let primary = unsafe { source.primary_image_index() }.min(count - 1);
    Ok(ImagePages {
        pages,
        primary: primary as u32,
    })
}

/// The orientation `picture`'s properties give it.
fn orientation_in(picture: &Properties) -> ImageOrientation {
    // SAFETY: a constant Image I/O exports for the life of the process.
    orientation_of(number_in(picture, unsafe { kCGImagePropertyOrientation }))
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

/// A picture's size upright, from its pixels and its orientation.
fn upright_size(picture: &Properties, orientation: ImageOrientation) -> Result<Size, ImageError> {
    // SAFETY: constants Image I/O exports for the life of the process.
    let (width, height) = unsafe {
        (
            number_in(picture, kCGImagePropertyPixelWidth),
            number_in(picture, kCGImagePropertyPixelHeight),
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

/// The animation's `count` frames, each with its delay where its file gives one.
fn frames_of(source: &CGImageSource, count: usize) -> Vec<AnimationFrame> {
    (0..count)
        .map(|index| AnimationFrame {
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

/// The properties of the source's picture `index`; none where it has none.
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
    fn a_loop_count_is_of_plays_and_zero_is_forever() {
        assert_eq!(plays_of(0.0), AnimationPlays::Forever);
        assert_eq!(plays_of(1.0), AnimationPlays::Times(1));
        assert_eq!(plays_of(4.0), AnimationPlays::Times(4));
    }

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
