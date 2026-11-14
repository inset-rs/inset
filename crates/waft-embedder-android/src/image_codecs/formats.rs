//! The formats Android's decoder reads and its encoder writes, by the MIME types `AImageDecoder`
//! names them by. Every device decodes what Android's compatibility definition requires of it,
//! JPEG, GIF, PNG, BMP, WebP and raw photos (DNG), and Android's decoder reads ICO and WBMP too;
//! HEIF and AVIF are decoded where the device's codecs do, which only decoding one tells
//! (`PROBED`). Whether the decoder turns a format's pictures by their orientation itself is told
//! the same way (`ORIENTATION_PROBED`). Every device encodes JPEG, PNG and WebP, which is all
//! `AndroidBitmap_compress` writes.
//!
//! Android takes a quality for every encoding and has no default of its own, so where the app
//! gives none the host asks for [`QUALITY_WHERE_NONE`].

use waft_embedder::image_codecs::{ImageError, ImageErrorKind};

/// What every device decodes, as `AImageDecoder` names each.
pub(super) const DECODED: [&str; 8] = [
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/x-ico",
    "image/vnd.wap.wbmp",
    "image/x-adobe-dng",
];

/// What a device decodes where its codecs do, each with a picture that decodes on a device that
/// has it.
pub(super) const PROBED: [(&str, &[u8]); 2] = [
    ("image/heif", include_bytes!("probes/mirrored.heic")),
    ("image/avif", include_bytes!("probes/mirrored.avif")),
];

/// The formats whose files may say how their picture is turned, each with a picture of 16 by 8,
/// red on its left half and blue on its right as stored, saying to mirror it (EXIF 2): where it
/// decodes blue on the left, the decoder turns that format's pictures itself.
pub(super) const ORIENTATION_PROBED: [(&str, &[u8]); 5] = [
    ("image/jpeg", include_bytes!("probes/mirrored.jpg")),
    ("image/png", include_bytes!("probes/mirrored.png")),
    ("image/webp", include_bytes!("probes/mirrored.webp")),
    ("image/heif", include_bytes!("probes/mirrored.heic")),
    ("image/avif", include_bytes!("probes/mirrored.avif")),
];

/// The quality the host asks for where the app gives none, from 0 to 100.
pub(super) const QUALITY_WHERE_NONE: i32 = 90;

/// How `AndroidBitmap_compress` writes a format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Compression {
    Jpeg,
    Png,
    WebpLossy,
}

/// The formats it encodes, by MIME type.
pub(super) fn encoded() -> Vec<String> {
    ["image/jpeg", "image/png", "image/webp"]
        .map(str::to_owned)
        .to_vec()
}

/// How `mime` is written: [`ImageErrorKind::Unsupported`] for a format Android does not encode.
pub(super) fn compression_of(mime: &str) -> Result<Compression, ImageError> {
    match mime {
        "image/jpeg" => Ok(Compression::Jpeg),
        "image/png" => Ok(Compression::Png),
        "image/webp" => Ok(Compression::WebpLossy),
        _ => Err(ImageError::new(
            ImageErrorKind::Unsupported,
            format!("Android does not encode {mime}"),
        )),
    }
}

/// The trait's quality, from 0 to 1, as Android's, from 0 to 100.
pub(super) fn quality_of(quality: Option<f32>) -> Result<i32, ImageError> {
    match quality {
        None => Ok(QUALITY_WHERE_NONE),
        Some(quality) if (0.0..=1.0).contains(&quality) => Ok((quality * 100.0).round() as i32),
        Some(quality) => Err(ImageError::new(
            ImageErrorKind::Other,
            format!("a quality of {quality}, outside 0 to 1"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_android_encodes_is_named_both_ways() {
        for mime in encoded() {
            assert!(compression_of(&mime).is_ok(), "{mime}");
        }
        assert_eq!(
            compression_of("image/webp").unwrap(),
            Compression::WebpLossy
        );
        assert_eq!(
            compression_of("image/heic").unwrap_err().kind,
            ImageErrorKind::Unsupported
        );
        for (mime, sample) in PROBED {
            assert!(!DECODED.contains(&mime), "{mime} is probed, not given");
            assert!(!sample.is_empty());
        }
        for (mime, _) in ORIENTATION_PROBED {
            let decoded =
                DECODED.contains(&mime) || PROBED.iter().any(|(probed, _)| *probed == mime);
            assert!(decoded, "{mime} is a format Android decodes");
        }
    }

    #[test]
    fn a_quality_from_zero_to_one_is_androids_from_zero_to_a_hundred() {
        assert_eq!(quality_of(Some(0.0)).unwrap(), 0);
        assert_eq!(quality_of(Some(0.304)).unwrap(), 30);
        assert_eq!(quality_of(Some(1.0)).unwrap(), 100);
        assert_eq!(quality_of(None).unwrap(), QUALITY_WHERE_NONE);
        assert_eq!(
            quality_of(Some(1.5)).unwrap_err().kind,
            ImageErrorKind::Other
        );
        assert_eq!(
            quality_of(Some(f32::NAN)).unwrap_err().kind,
            ImageErrorKind::Other
        );
    }
}
