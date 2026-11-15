//! The formats Image I/O reads and writes, named by MIME type for the app and by UTI for Image I/O,
//! as `UTType` maps the two. A UTI with no MIME type is not offered.

use objc2::rc::Retained;
use objc2_core_foundation::CFArray;
use objc2_foundation::{NSArray, NSString};
use objc2_image_io::{CGImageDestination, CGImageSource};
use objc2_uniform_type_identifiers::UTType;
use waft_embedder::image_codecs::{ImageError, ImageErrorKind};

/// The MIME types of the formats Image I/O decodes, in its order.
pub(super) fn decoded() -> Vec<String> {
    // SAFETY: a plain query of the types Image I/O reads.
    let read = unsafe { CGImageSource::type_identifiers() };
    mime_types_of(&read)
}

/// The MIME types of the formats Image I/O encodes, in its order.
pub(super) fn encoded() -> Vec<String> {
    // SAFETY: a plain query of the types Image I/O writes.
    let written = unsafe { CGImageDestination::type_identifiers() };
    mime_types_of(&written)
}

/// The UTI Image I/O writes `mime` as: [`ImageErrorKind::Unsupported`] for a MIME type it does not
/// encode.
pub(super) fn encoded_uti(mime: &str) -> Result<Retained<NSString>, ImageError> {
    let uti = UTType::typeWithMIMEType(&NSString::from_str(mime)).map(|uti| uti.identifier());
    // SAFETY: a plain query of the types Image I/O writes.
    let encoded = unsafe { CGImageDestination::type_identifiers() };
    uti.filter(|uti| {
        utis_of(&encoded)
            .iter()
            .any(|written| written.isEqualToString(uti))
    })
    .ok_or_else(|| {
        ImageError::new(
            ImageErrorKind::Unsupported,
            format!("Image I/O does not encode {mime}"),
        )
    })
}

/// The MIME type of `uti`, the type Image I/O read an image as; none for a UTI with none.
pub(super) fn mime_type_of(uti: &NSString) -> Option<String> {
    UTType::typeWithIdentifier(uti)?
        .preferredMIMEType()
        .map(|mime| mime.to_string())
}

/// The MIME types of `utis`, each once, in order.
fn mime_types_of(utis: &CFArray) -> Vec<String> {
    let mut mime_types: Vec<String> = Vec::new();
    for mime in utis_of(utis).iter().filter_map(|uti| mime_type_of(&uti)) {
        if !mime_types.contains(&mime) {
            mime_types.push(mime);
        }
    }
    mime_types
}

/// `utis`, a CF array of CF strings, as the NSArray it is toll-free bridged to.
fn utis_of(utis: &CFArray) -> &NSArray<NSString> {
    let pointer: *const CFArray = utis;
    // SAFETY: a CFArray is an NSArray, and Image I/O's lists of types hold CF strings, which are
    // NSStrings.
    unsafe { &*pointer.cast::<NSArray<NSString>>() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_formats_are_named_by_their_mime_types_both_ways() {
        let decoded = decoded();
        for mime in ["image/jpeg", "image/png", "image/heic", "image/gif"] {
            assert!(
                decoded.iter().any(|read| read == mime),
                "{mime}: {decoded:?}"
            );
        }
        let encoded = encoded();
        for mime in ["image/jpeg", "image/png"] {
            assert!(
                encoded.iter().any(|written| written == mime),
                "{mime}: {encoded:?}"
            );
            assert!(encoded_uti(mime).is_ok(), "{mime}");
        }
        let unknown = encoded_uti("image/x-nothing").err().unwrap();
        assert_eq!(unknown.kind, ImageErrorKind::Unsupported);
        assert_eq!(
            mime_type_of(&NSString::from_str("public.png")).as_deref(),
            Some("image/png")
        );
    }
}
