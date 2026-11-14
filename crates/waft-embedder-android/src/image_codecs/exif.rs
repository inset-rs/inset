//! What an image's file says of how it was taken and how its picture is turned, from the EXIF
//! tags Android's `ExifInterface` reads (`exif_interface`), each by the tag's name and as the text
//! `getAttribute` answers: a rational as `numerator/denominator`, the few tags it keeps as decimals
//! for compatibility (`ExposureTime`, `FNumber`) as a decimal, a list as its values between
//! commas, a date as `YYYY:MM:DD HH:MM:SS` and its offset from UTC as `+HH:MM`.
//!
//! Each field of the trait's record is read from the tag Apple's host reads it from through
//! Image I/O (`waft-embedder-winit`'s `os/apple/image_codecs/exif.rs`), so the two hosts answer
//! the same for the same file, but for one: the framework's `ExifInterface` knows no `LensModel`
//! (AndroidX's does), so the lens is `None` on Android while it does not, though asked for.

use std::collections::HashMap;

use waft_embedder::image_codecs::{
    DateTime, ImageMetadata, ImageOrientation, ImageRotation, Location,
};

/// The tags read, by the names `ExifInterface` gives them.
pub(super) const TAGS: [&str; 20] = [
    ORIENTATION,
    DATE_TIME_ORIGINAL,
    OFFSET_TIME_ORIGINAL,
    MAKE,
    MODEL,
    LENS_MODEL,
    EXPOSURE_TIME,
    F_NUMBER,
    PHOTOGRAPHIC_SENSITIVITY,
    ISO_SPEED_RATINGS,
    FOCAL_LENGTH,
    FOCAL_LENGTH_IN_35MM_FILM,
    EXPOSURE_BIAS_VALUE,
    FLASH,
    GPS_LATITUDE,
    GPS_LATITUDE_REF,
    GPS_LONGITUDE,
    GPS_LONGITUDE_REF,
    GPS_ALTITUDE,
    GPS_ALTITUDE_REF,
];

const ORIENTATION: &str = "Orientation";
const DATE_TIME_ORIGINAL: &str = "DateTimeOriginal";
const OFFSET_TIME_ORIGINAL: &str = "OffsetTimeOriginal";
const MAKE: &str = "Make";
const MODEL: &str = "Model";
const LENS_MODEL: &str = "LensModel";
const EXPOSURE_TIME: &str = "ExposureTime";
const F_NUMBER: &str = "FNumber";
/// EXIF 2.3's name of tag 0x8827, which Image I/O calls `ISOSpeedRatings`; read first.
const PHOTOGRAPHIC_SENSITIVITY: &str = "PhotographicSensitivity";
/// The older name of the same tag, which `ExifInterface` keeps too.
const ISO_SPEED_RATINGS: &str = "ISOSpeedRatings";
const FOCAL_LENGTH: &str = "FocalLength";
const FOCAL_LENGTH_IN_35MM_FILM: &str = "FocalLengthIn35mmFilm";
const EXPOSURE_BIAS_VALUE: &str = "ExposureBiasValue";
const FLASH: &str = "Flash";
const GPS_LATITUDE: &str = "GPSLatitude";
const GPS_LATITUDE_REF: &str = "GPSLatitudeRef";
const GPS_LONGITUDE: &str = "GPSLongitude";
const GPS_LONGITUDE_REF: &str = "GPSLongitudeRef";
const GPS_ALTITUDE: &str = "GPSAltitude";
const GPS_ALTITUDE_REF: &str = "GPSAltitudeRef";

/// The tags a file has, each tag's text as `ExifInterface` answers it; a tag the file does not
/// have is not there.
pub(super) type ExifTags = HashMap<&'static str, String>;

/// How the file's picture is turned: EXIF's `Orientation`, upright where it says none.
pub(super) fn orientation_of(tags: &ExifTags) -> ImageOrientation {
    orientation_from(first_number(tags, ORIENTATION))
}

/// What the file says of how it was taken.
pub(super) fn metadata_of(tags: &ExifTags) -> ImageMetadata {
    let text = |tag| tags.get(tag).map(|value| value.trim().to_owned());
    ImageMetadata {
        taken: text(DATE_TIME_ORIGINAL)
            .and_then(|taken| date_time_of(&taken, text(OFFSET_TIME_ORIGINAL).as_deref())),
        camera_make: text(MAKE),
        camera_model: text(MODEL),
        lens_model: text(LENS_MODEL),
        exposure_time: first_number(tags, EXPOSURE_TIME),
        f_number: first_number(tags, F_NUMBER),
        iso: first_number(tags, PHOTOGRAPHIC_SENSITIVITY)
            .or_else(|| first_number(tags, ISO_SPEED_RATINGS))
            .map(|iso| iso as u32),
        focal_length: first_number(tags, FOCAL_LENGTH),
        equivalent_focal_length: first_number(tags, FOCAL_LENGTH_IN_35MM_FILM),
        exposure_bias: first_number(tags, EXPOSURE_BIAS_VALUE),
        // EXIF's flash is a set of bits, the lowest whether it fired.
        flash_fired: first_number(tags, FLASH).map(|flash| flash as u32 & 1 == 1),
        location: location_of(tags),
    }
}

/// EXIF's orientation, from 1 to 8, as how the stored picture is turned to show upright, as
/// ExifTool names each: mirrored left to right first, then turned clockwise. Upright where the
/// file says none, or a value EXIF does not have.
fn orientation_from(exif: Option<f64>) -> ImageOrientation {
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

/// Where GPS's tags say the picture was taken: degrees, minutes and seconds signed by their
/// hemispheres, and the altitude by whether it is below sea level, `1` in EXIF.
fn location_of(tags: &ExifTags) -> Option<Location> {
    let latitude = degrees_of(tags.get(GPS_LATITUDE)?)?;
    let longitude = degrees_of(tags.get(GPS_LONGITUDE)?)?;
    let is = |tag, value: &str| tags.get(tag).is_some_and(|text| text.trim() == value);
    let below = first_number(tags, GPS_ALTITUDE_REF) == Some(1.0);
    Some(Location {
        latitude: if is(GPS_LATITUDE_REF, "S") {
            -latitude
        } else {
            latitude
        },
        longitude: if is(GPS_LONGITUDE_REF, "W") {
            -longitude
        } else {
            longitude
        },
        altitude: first_number(tags, GPS_ALTITUDE)
            .map(|altitude| if below { -altitude } else { altitude }),
    })
}

/// GPS's three rationals, degrees, minutes and seconds, as degrees.
fn degrees_of(text: &str) -> Option<f64> {
    let mut parts = text.split(',').map(number_of);
    let degrees = parts.next()??;
    let minutes = parts.next().flatten().unwrap_or(0.0);
    let seconds = parts.next().flatten().unwrap_or(0.0);
    Some(degrees + minutes / 60.0 + seconds / 3600.0)
}

/// The first of the values the tag holds, as a number.
fn first_number(tags: &ExifTags, tag: &str) -> Option<f64> {
    number_of(tags.get(tag)?.split(',').next()?)
}

/// A rational, `numerator/denominator`, or a decimal; none for a denominator of zero.
fn number_of(text: &str) -> Option<f64> {
    let text = text.trim();
    match text.split_once('/') {
        Some((numerator, denominator)) => {
            let denominator = denominator.trim().parse::<f64>().ok()?;
            let numerator = numerator.trim().parse::<f64>().ok()?;
            (denominator != 0.0).then(|| numerator / denominator)
        }
        None => text.parse::<f64>().ok().filter(|number| number.is_finite()),
    }
}

/// EXIF's `YYYY:MM:DD HH:MM:SS`, with its offset from UTC, `+HH:MM` or `-HH:MM`, where given;
/// none for text that is not a date so, as a camera whose clock was never set writes blanks.
fn date_time_of(taken: &str, offset: Option<&str>) -> Option<DateTime> {
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

    /// The tags as `ExifInterface` answers them for `waft-embedder-winit`'s
    /// `tests/media/photo.jpg`, which Apple's host's test reads too.
    fn photo() -> ExifTags {
        [
            (ORIENTATION, "6"),
            (DATE_TIME_ORIGINAL, "2024:05:01 10:20:30"),
            (OFFSET_TIME_ORIGINAL, "+02:00"),
            (MAKE, "Waft"),
            (MODEL, "Test Camera"),
            (LENS_MODEL, "Test Lens 4.2mm f/1.8"),
            (EXPOSURE_TIME, "0.008333333333333333"),
            (F_NUMBER, "1.8"),
            (PHOTOGRAPHIC_SENSITIVITY, "200"),
            (FOCAL_LENGTH, "42/10"),
            (FOCAL_LENGTH_IN_35MM_FILM, "26"),
            (EXPOSURE_BIAS_VALUE, "-1/3"),
            (FLASH, "1"),
            (GPS_LATITUDE, "48/1,51/1,3/1"),
            (GPS_LATITUDE_REF, "N"),
            (GPS_LONGITUDE, "2/1,17/1,42/1"),
            (GPS_LONGITUDE_REF, "W"),
            (GPS_ALTITUDE, "35/1"),
            (GPS_ALTITUDE_REF, "0"),
        ]
        .into_iter()
        .map(|(tag, value)| (tag, value.to_owned()))
        .collect()
    }

    #[test]
    fn a_photos_tags_read_as_apples_host_reads_them() {
        let tags = photo();
        let orientation = orientation_of(&tags);
        assert_eq!(
            (orientation.mirrored, orientation.rotation),
            (false, ImageRotation::QuarterTurn)
        );
        let metadata = metadata_of(&tags);
        assert_eq!(
            metadata.taken,
            Some(DateTime {
                year: 2024,
                month: 5,
                day: 1,
                hour: 10,
                minute: 20,
                second: 30,
                utc_offset: Some(120),
            })
        );
        assert_eq!(metadata.camera_make.as_deref(), Some("Waft"));
        assert_eq!(metadata.camera_model.as_deref(), Some("Test Camera"));
        assert_eq!(
            metadata.lens_model.as_deref(),
            Some("Test Lens 4.2mm f/1.8")
        );
        let near = |value: Option<f64>, expected: f64| {
            assert!((value.unwrap() - expected).abs() < 1e-6, "{value:?}");
        };
        near(metadata.exposure_time, 1.0 / 120.0);
        near(metadata.f_number, 1.8);
        assert_eq!(metadata.iso, Some(200));
        near(metadata.focal_length, 4.2);
        near(metadata.equivalent_focal_length, 26.0);
        near(metadata.exposure_bias, -1.0 / 3.0);
        assert_eq!(metadata.flash_fired, Some(true));
        let location = metadata.location.unwrap();
        assert!((location.latitude - (48.0 + 51.0 / 60.0 + 3.0 / 3600.0)).abs() < 1e-6);
        assert!((location.longitude + (2.0 + 17.0 / 60.0 + 42.0 / 3600.0)).abs() < 1e-6);
        near(location.altitude, 35.0);
    }

    #[test]
    fn every_tag_read_is_asked_of_exif_interface() {
        for tag in photo().keys() {
            assert!(TAGS.contains(tag), "{tag}");
        }
    }

    #[test]
    fn a_file_that_says_nothing_is_upright_with_no_metadata() {
        let none = ExifTags::new();
        assert_eq!(orientation_of(&none), ImageOrientation::default());
        assert_eq!(metadata_of(&none), ImageMetadata::default());
    }

    #[test]
    fn each_exif_orientation_is_a_mirroring_and_a_turn() {
        let turned = |exif: f64| {
            let orientation = orientation_from(Some(exif));
            (orientation.mirrored, orientation.rotation)
        };
        assert_eq!(turned(1.0), (false, ImageRotation::None));
        assert_eq!(turned(2.0), (true, ImageRotation::None));
        assert_eq!(turned(3.0), (false, ImageRotation::HalfTurn));
        assert_eq!(turned(4.0), (true, ImageRotation::HalfTurn));
        assert_eq!(turned(5.0), (true, ImageRotation::ThreeQuarterTurn));
        assert_eq!(turned(6.0), (false, ImageRotation::QuarterTurn));
        assert_eq!(turned(7.0), (true, ImageRotation::QuarterTurn));
        assert_eq!(turned(8.0), (false, ImageRotation::ThreeQuarterTurn));
        assert_eq!(orientation_from(Some(9.0)), ImageOrientation::default());
    }

    #[test]
    fn numbers_read_as_rationals_or_decimals_and_dates_with_their_offsets() {
        assert_eq!(number_of("1/120"), Some(1.0 / 120.0));
        assert_eq!(number_of(" -1/3 "), Some(-1.0 / 3.0));
        assert_eq!(number_of("1/0"), None);
        assert_eq!(number_of("0.25"), Some(0.25));
        assert_eq!(number_of("NaN"), None);
        assert_eq!(degrees_of("48/1,30/1,0/1"), Some(48.5));
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
