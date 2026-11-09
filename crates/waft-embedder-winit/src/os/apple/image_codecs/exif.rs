//! What an image's file says of how it was taken, from the properties Image I/O reads of its first
//! frame: EXIF's dictionary, TIFF's for the camera's maker and model, and GPS's for where. EXIF
//! writes a date as `YYYY:MM:DD HH:MM:SS` and its offset from UTC as `+HH:MM`, and GPS a position
//! as degrees with the hemisphere apart, which Image I/O hands over as they are.

use objc2_image_io::{
    kCGImagePropertyExifDateTimeOriginal, kCGImagePropertyExifDictionary,
    kCGImagePropertyExifExposureBiasValue, kCGImagePropertyExifExposureTime,
    kCGImagePropertyExifFNumber, kCGImagePropertyExifFlash, kCGImagePropertyExifFocalLenIn35mmFilm,
    kCGImagePropertyExifFocalLength, kCGImagePropertyExifISOSpeedRatings,
    kCGImagePropertyExifLensModel, kCGImagePropertyExifOffsetTimeOriginal,
    kCGImagePropertyGPSAltitude, kCGImagePropertyGPSAltitudeRef, kCGImagePropertyGPSDictionary,
    kCGImagePropertyGPSLatitude, kCGImagePropertyGPSLatitudeRef, kCGImagePropertyGPSLongitude,
    kCGImagePropertyGPSLongitudeRef, kCGImagePropertyTIFFDictionary, kCGImagePropertyTIFFMake,
    kCGImagePropertyTIFFModel,
};
use waft_embedder::image_codecs::{DateTime, ImageMetadata, Location};

use super::source::{Properties, dictionary_in, first_number_in, number_in, string_in};

/// What `first`, the properties of an image's first frame, say of how it was taken.
pub(super) fn metadata_of(first: &Properties) -> ImageMetadata {
    // SAFETY: constants Image I/O exports for the life of the process.
    let (exif, tiff, gps) = unsafe {
        (
            dictionary_in(first, kCGImagePropertyExifDictionary),
            dictionary_in(first, kCGImagePropertyTIFFDictionary),
            dictionary_in(first, kCGImagePropertyGPSDictionary),
        )
    };
    let mut metadata = ImageMetadata {
        location: gps.as_deref().and_then(location_of),
        ..ImageMetadata::default()
    };
    if let Some(tiff) = tiff.as_deref() {
        // SAFETY: as above.
        unsafe {
            metadata.camera_make = string_in(tiff, kCGImagePropertyTIFFMake);
            metadata.camera_model = string_in(tiff, kCGImagePropertyTIFFModel);
        }
    }
    if let Some(exif) = exif.as_deref() {
        read_exif(exif, &mut metadata);
    }
    metadata
}

/// EXIF's own fields, from its dictionary.
fn read_exif(exif: &Properties, metadata: &mut ImageMetadata) {
    // SAFETY: constants Image I/O exports for the life of the process.
    unsafe {
        metadata.taken = string_in(exif, kCGImagePropertyExifDateTimeOriginal).and_then(|taken| {
            let offset = string_in(exif, kCGImagePropertyExifOffsetTimeOriginal);
            date_time_of(&taken, offset.as_deref())
        });
        metadata.lens_model = string_in(exif, kCGImagePropertyExifLensModel);
        metadata.exposure_time = number_in(exif, kCGImagePropertyExifExposureTime);
        metadata.f_number = number_in(exif, kCGImagePropertyExifFNumber);
        metadata.iso =
            first_number_in(exif, kCGImagePropertyExifISOSpeedRatings).map(|iso| iso as u32);
        metadata.focal_length = number_in(exif, kCGImagePropertyExifFocalLength);
        metadata.equivalent_focal_length = number_in(exif, kCGImagePropertyExifFocalLenIn35mmFilm);
        metadata.exposure_bias = number_in(exif, kCGImagePropertyExifExposureBiasValue);
        // EXIF's flash is a set of bits, the lowest whether it fired.
        metadata.flash_fired =
            number_in(exif, kCGImagePropertyExifFlash).map(|flash| flash as u32 & 1 == 1);
    }
}

/// Where GPS's dictionary says the picture was taken: its degrees signed by their hemispheres,
/// and its altitude by whether it is below sea level, `1` in EXIF.
fn location_of(gps: &Properties) -> Option<Location> {
    // SAFETY: constants Image I/O exports for the life of the process.
    unsafe {
        let latitude = number_in(gps, kCGImagePropertyGPSLatitude)?;
        let longitude = number_in(gps, kCGImagePropertyGPSLongitude)?;
        let south = string_in(gps, kCGImagePropertyGPSLatitudeRef).as_deref() == Some("S");
        let west = string_in(gps, kCGImagePropertyGPSLongitudeRef).as_deref() == Some("W");
        let below = number_in(gps, kCGImagePropertyGPSAltitudeRef) == Some(1.0);
        Some(Location {
            latitude: if south { -latitude } else { latitude },
            longitude: if west { -longitude } else { longitude },
            altitude: number_in(gps, kCGImagePropertyGPSAltitude)
                .map(|altitude| if below { -altitude } else { altitude }),
        })
    }
}

/// EXIF's `YYYY:MM:DD HH:MM:SS`, with its offset from UTC, `+HH:MM` or `-HH:MM`, where given;
/// none for text that is not a date so, as a camera whose clock was never set writes blanks.
pub(super) fn date_time_of(taken: &str, offset: Option<&str>) -> Option<DateTime> {
    let (date, time) = taken.trim().split_once(' ')?;
    let mut date = date.split(':').map(str::parse::<u32>);
    let mut time = time.split(':').map(str::parse::<u32>);
    let [year, month, day] = [date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?];
    let [hour, minute, second] = [time.next()?.ok()?, time.next()?.ok()?, time.next()?.ok()?];
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    Some(DateTime {
        year: year as i32,
        month: month as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second.min(60) as u8,
        utc_offset: offset.and_then(utc_offset_of),
    })
}

/// EXIF's `+HH:MM` or `-HH:MM`, in minutes ahead of UTC.
fn utc_offset_of(offset: &str) -> Option<i16> {
    let offset = offset.trim();
    let (sign, rest) = match offset.as_bytes().first()? {
        b'+' => (1, &offset[1..]),
        b'-' => (-1, &offset[1..]),
        _ => return None,
    };
    let (hours, minutes) = rest.split_once(':')?;
    let (hours, minutes) = (hours.parse::<i16>().ok()?, minutes.parse::<i16>().ok()?);
    (hours <= 14 && minutes < 60).then_some(sign * (hours * 60 + minutes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_taken_reads_with_its_offset_from_utc() {
        assert_eq!(
            date_time_of("2024:05:01 10:20:30", Some("+05:30")),
            Some(DateTime {
                year: 2024,
                month: 5,
                day: 1,
                hour: 10,
                minute: 20,
                second: 30,
                utc_offset: Some(330),
            })
        );
        let west = date_time_of("2024:05:01 10:20:30", Some("-07:00")).unwrap();
        assert_eq!(west.utc_offset, Some(-420));
        let local = date_time_of("2024:05:01 10:20:30", None).unwrap();
        assert_eq!(local.utc_offset, None, "the camera's local time");
        assert_eq!(
            date_time_of("    :  :     :  :  ", None),
            None,
            "an unset clock"
        );
        assert_eq!(date_time_of("2024:13:01 10:20:30", None), None);
    }
}
