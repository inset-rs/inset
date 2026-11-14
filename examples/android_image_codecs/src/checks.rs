//! Each check of the codecs: what it found, line by line, whether all of it is what the picture
//! holds, and the pixels it decoded, to show.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use waft_embedder::Size;
use waft_embedder::image_codecs::{
    AlphaType, DateTime, ImageCodecs, ImageErrorKind, ImageMetadata, ImageRotation, ImageSource,
    PixelBuffer, PixelFormat,
};

/// The folder Android gives the app's files, where the photo is written to open it as a file.
pub(crate) static FILES: OnceLock<PathBuf> = OnceLock::new();

const PHOTO: &[u8] = include_bytes!("../media/photo.jpg");
const ALPHA: &[u8] = include_bytes!("../media/alpha.png");
const GIF: &[u8] = include_bytes!("../media/two-frames.gif");
const WEBP: &[u8] = include_bytes!("../media/two-frames.webp");
const MIRRORED: &[u8] = include_bytes!("../media/mirrored.jpg");
const TURNED_PNG: &[u8] = include_bytes!("../media/turned.png");
const TURNED_WEBP: &[u8] = include_bytes!("../media/turned.webp");
const TURNED_HEIF: &[u8] = include_bytes!("../media/turned.heic");
const TURNED_AVIF: &[u8] = include_bytes!("../media/turned.avif");

/// What one check found.
pub(crate) struct Report {
    pub(crate) title: &'static str,
    pub(crate) lines: Vec<String>,
    pub(crate) passed: bool,
    pub(crate) pictures: Vec<PixelBuffer>,
}

impl Report {
    fn new(title: &'static str) -> Report {
        Report {
            title,
            lines: Vec::new(),
            passed: true,
            pictures: Vec::new(),
        }
    }

    /// Notes `line`, marked as what was expected or not.
    fn expect(&mut self, held: bool, line: String) {
        self.passed &= held;
        self.lines
            .push(format!("{} {line}", if held { "ok" } else { "WRONG" }));
    }

    /// Notes `line`, which says what the system has rather than what is right.
    fn note(&mut self, line: String) {
        self.lines.push(format!("-- {line}"));
    }
}

/// Every check, in order.
pub(crate) async fn all(codecs: &dyn ImageCodecs) -> Vec<Report> {
    vec![
        formats(codecs),
        oriented_photo(codecs).await,
        photo_file(codecs).await,
        mirrored_photo(codecs).await,
        turned(
            codecs,
            "turned.png, EXIF orientation 6",
            TURNED_PNG,
            "image/png",
        )
        .await,
        turned(
            codecs,
            "turned.webp, EXIF orientation 6",
            TURNED_WEBP,
            "image/webp",
        )
        .await,
        turned(
            codecs,
            "turned.heic, from photo.jpg",
            TURNED_HEIF,
            "image/heif",
        )
        .await,
        turned(
            codecs,
            "turned.avif, EXIF orientation 6",
            TURNED_AVIF,
            "image/avif",
        )
        .await,
        png_with_alpha(codecs).await,
        animation(
            codecs,
            "two-frames.gif",
            GIF,
            [200, 500],
            [[255, 0, 0], [0, 0, 255]],
        )
        .await,
        animation(
            codecs,
            "two-frames.webp",
            WEBP,
            [300, 700],
            [[0, 255, 0], [255, 255, 0]],
        )
        .await,
        encoded_and_back(codecs).await,
        failures(codecs).await,
    ]
}

fn formats(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("formats");
    let started = Instant::now();
    let decoded = codecs.decoded_formats();
    report.note(format!(
        "decoded, found in {:?}: {}",
        started.elapsed(),
        decoded.join(" ")
    ));
    let encoded = codecs.encoded_formats();
    report.note(format!("encoded: {}", encoded.join(" ")));
    for mime in ["image/jpeg", "image/png", "image/gif", "image/webp"] {
        report.expect(
            decoded.iter().any(|read| read == mime),
            format!("decodes {mime}"),
        );
    }
    report.expect(
        codecs.hardware_image_codecs().is_none(),
        "no hardware codecs".to_owned(),
    );
    report
}

/// The photo opens upright, 32 by 48, and decodes, scaled down to fit 16 by 16, red above blue.
async fn oriented_photo(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("photo.jpg, EXIF orientation 6");
    let photo = match codecs.open(Arc::from(PHOTO)).await {
        Ok(photo) => photo,
        Err(error) => return failed(report, &error.to_string()),
    };
    says_it_is_the_upright_photo(&mut report, photo.as_ref());
    is_turned(
        &mut report,
        photo.as_ref(),
        false,
        ImageRotation::QuarterTurn,
    );
    says_how_it_was_taken(&mut report, &photo.metadata());
    match photo.decode_pixels(0, Some(Size::new(16.0, 16.0))).await {
        Ok(pixels) => {
            report.expect(
                close(pixels.layout().size, [11, 16]),
                format!("decoded to fit 16 by 16: {:?}", pixels.layout().size),
            );
            is_red_above_blue(&mut report, &pixels);
        }
        Err(error) => report.expect(false, format!("decoding: {error}")),
    }
    if let Ok(whole) = photo.decode_pixels(0, None).await {
        report.pictures.push(whole);
    }
    report
}

/// The orientation `image` answers is `mirrored` and turned by `rotation`.
fn is_turned(
    report: &mut Report,
    image: &dyn ImageSource,
    mirrored: bool,
    rotation: ImageRotation,
) {
    let orientation = image.orientation();
    report.expect(
        (orientation.mirrored, orientation.rotation) == (mirrored, rotation),
        format!(
            "orientation: mirrored {}, {:?}",
            orientation.mirrored, orientation.rotation
        ),
    );
}

/// The photo's metadata is what Apple's host's test reads of the same file
/// (`waft-embedder-winit/tests/an_image_decodes_and_encodes_through_the_systems_codecs.rs`).
fn says_how_it_was_taken(report: &mut Report, metadata: &ImageMetadata) {
    let taken = DateTime {
        year: 2024,
        month: 5,
        day: 1,
        hour: 10,
        minute: 20,
        second: 30,
        utc_offset: Some(120),
    };
    report.expect(
        metadata.taken == Some(taken),
        format!("taken: {:?}", metadata.taken),
    );
    report.expect(
        metadata.camera_make.as_deref() == Some("Waft")
            && metadata.camera_model.as_deref() == Some("Test Camera"),
        format!(
            "camera: {:?} {:?}",
            metadata.camera_make, metadata.camera_model
        ),
    );
    // The framework's `ExifInterface` reads no `LensModel`; Apple's host reads
    // "Test Lens 4.2mm f/1.8" here.
    report.note(format!("lens: {:?}", metadata.lens_model));
    let near = |value: Option<f64>, expected: f64| {
        value.is_some_and(|value| (value - expected).abs() < 1e-6)
    };
    report.expect(
        near(metadata.exposure_time, 1.0 / 120.0)
            && near(metadata.f_number, 1.8)
            && metadata.iso == Some(200)
            && near(metadata.focal_length, 4.2)
            && near(metadata.equivalent_focal_length, 26.0)
            && near(metadata.exposure_bias, -1.0 / 3.0)
            && metadata.flash_fired == Some(true),
        format!(
            "exposure {:?} s, f/{:?}, ISO {:?}, {:?} mm ({:?} mm), bias {:?}, flash {:?}",
            metadata.exposure_time,
            metadata.f_number,
            metadata.iso,
            metadata.focal_length,
            metadata.equivalent_focal_length,
            metadata.exposure_bias,
            metadata.flash_fired
        ),
    );
    let location = metadata.location;
    report.expect(
        location.is_some_and(|location| {
            (location.latitude - (48.0 + 51.0 / 60.0 + 3.0 / 3600.0)).abs() < 1e-6
                && (location.longitude + (2.0 + 17.0 / 60.0 + 42.0 / 3600.0)).abs() < 1e-6
                && near(location.altitude, 35.0)
        }),
        format!("location: {location:?}"),
    );
}

/// The mirrored photo answers a mirroring and no turn, and decodes as it shows: 48 by 32, blue on
/// the left and red on the right, the stored picture flipped.
async fn mirrored_photo(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("mirrored.jpg, EXIF orientation 2");
    let photo = match codecs.open(Arc::from(MIRRORED)).await {
        Ok(photo) => photo,
        Err(error) => return failed(report, &error.to_string()),
    };
    report.expect(
        photo.size() == Size::new(48.0, 32.0),
        format!("size {:?}", photo.size()),
    );
    is_turned(&mut report, photo.as_ref(), true, ImageRotation::None);
    report.expect(
        photo.metadata() == ImageMetadata::default(),
        "no metadata, as the file has none".to_owned(),
    );
    match photo.decode_pixels(0, None).await {
        Ok(pixels) => {
            let [red, _, blue, _] = pixel_at(&pixels, 4, 16);
            report.expect(
                red < 60 && blue > 200,
                format!("blue on the left: {:?}", [red, blue]),
            );
            let [red, _, blue, _] = pixel_at(&pixels, 43, 16);
            report.expect(
                red > 200 && blue < 60,
                format!("red on the right: {:?}", [red, blue]),
            );
            report.pictures.push(pixels);
        }
        Err(error) => report.expect(false, format!("decoding: {error}")),
    }
    report
}

/// A picture of another format stored as the photo is, 48 by 32 red left and blue right, saying
/// to turn it a quarter clockwise: its orientation as answered, and whether its size and pixels
/// are upright, red above blue.
async fn turned(
    codecs: &dyn ImageCodecs,
    title: &'static str,
    bytes: &'static [u8],
    mime: &str,
) -> Report {
    let mut report = Report::new(title);
    let image = match codecs.open(Arc::from(bytes)).await {
        Ok(image) => image,
        Err(error) => return failed(report, &error.to_string()),
    };
    report.expect(
        image.mime_type() == mime,
        format!("MIME type {}", image.mime_type()),
    );
    is_turned(
        &mut report,
        image.as_ref(),
        false,
        ImageRotation::QuarterTurn,
    );
    report.expect(
        image.size() == Size::new(32.0, 48.0),
        format!("size upright {:?}", image.size()),
    );
    match image.decode_pixels(0, None).await {
        Ok(pixels) => {
            is_red_above_blue(&mut report, &pixels);
            report.pictures.push(pixels);
        }
        Err(error) => report.expect(false, format!("decoding: {error}")),
    }
    report
}

/// The photo, written to a file, opens from it as it did from its bytes.
async fn photo_file(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("photo.jpg, opened as a file");
    let Some(path) = FILES.get().map(|files| files.join("photo.jpg")) else {
        return failed(report, "Android gave no folder for the app's files");
    };
    if let Err(error) = std::fs::write(&path, PHOTO) {
        return failed(report, &format!("writing {}: {error}", path.display()));
    }
    match codecs.open_file(&path).await {
        Ok(photo) => {
            says_it_is_the_upright_photo(&mut report, photo.as_ref());
            is_turned(
                &mut report,
                photo.as_ref(),
                false,
                ImageRotation::QuarterTurn,
            );
            says_how_it_was_taken(&mut report, &photo.metadata());
            match photo.decode_pixels(0, None).await {
                Ok(pixels) => is_red_above_blue(&mut report, &pixels),
                Err(error) => report.expect(false, format!("decoding: {error}")),
            }
        }
        Err(error) => report.expect(false, format!("opening {}: {error}", path.display())),
    }
    report
}

fn says_it_is_the_upright_photo(report: &mut Report, photo: &dyn ImageSource) {
    report.expect(
        photo.mime_type() == "image/jpeg",
        format!("MIME type {}", photo.mime_type()),
    );
    report.expect(
        photo.size() == Size::new(32.0, 48.0),
        format!("size upright {:?}", photo.size()),
    );
    let frames = photo.frames();
    report.expect(
        frames.len() == 1 && frames[0].duration.is_none(),
        format!("one still frame: {frames:?}"),
    );
}

/// Red near the top of `pixels`, blue near the bottom, as JPEG keeps colours.
fn is_red_above_blue(report: &mut Report, pixels: &PixelBuffer) {
    let height = pixels.layout().size[1];
    let [red, green, blue, alpha] = middle_of_row(pixels, 1);
    report.expect(
        red > 200 && green < 60 && blue < 60 && alpha == 255,
        format!("red above: {:?}", [red, green, blue, alpha]),
    );
    let [red, green, blue, _] = middle_of_row(pixels, height - 2);
    report.expect(
        red < 60 && green < 60 && blue > 200,
        format!("blue below: {:?}", [red, green, blue]),
    );
}

/// The PNG decodes premultiplied: its left half opaque green, its right half red at half alpha,
/// so red at half strength.
async fn png_with_alpha(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("alpha.png");
    let image = match codecs.open(Arc::from(ALPHA)).await {
        Ok(image) => image,
        Err(error) => return failed(report, &error.to_string()),
    };
    report.expect(
        image.mime_type() == "image/png" && image.size() == Size::new(32.0, 32.0),
        format!("{} {:?}", image.mime_type(), image.size()),
    );
    match image.decode_pixels(0, None).await {
        Ok(pixels) => {
            let layout = pixels.layout();
            report.expect(
                (layout.format, layout.alpha) == (PixelFormat::Rgba8, AlphaType::Premultiplied),
                format!("{:?}, {:?}", layout.format, layout.alpha),
            );
            let left = pixel_at(&pixels, 4, 16);
            report.expect(
                left == [0, 255, 0, 255],
                format!("left, opaque green: {left:?}"),
            );
            let right = pixel_at(&pixels, 28, 16);
            report.expect(
                right[0].abs_diff(128) <= 2 && right[1] == 0 && right[3].abs_diff(128) <= 1,
                format!("right, red at half alpha, premultiplied: {right:?}"),
            );
            report.pictures.push(pixels);
        }
        Err(error) => report.expect(false, format!("decoding: {error}")),
    }
    report
}

/// The animation's two frames, with their durations and colours, the second decoded before the
/// first and after it.
async fn animation(
    codecs: &dyn ImageCodecs,
    title: &'static str,
    bytes: &'static [u8],
    durations: [u64; 2],
    colours: [[u8; 3]; 2],
) -> Report {
    let mut report = Report::new(title);
    let image = match codecs.open(Arc::from(bytes)).await {
        Ok(image) => image,
        Err(error) => return failed(report, &error.to_string()),
    };
    report.note(format!("{} {:?}", image.mime_type(), image.size()));
    let found: Vec<_> = image.frames().iter().map(|frame| frame.duration).collect();
    let expected = durations.map(|milliseconds| Some(Duration::from_millis(milliseconds)));
    report.expect(
        found == expected,
        format!("frames and durations: {found:?}"),
    );
    for frame in [1, 0, 1] {
        match image.decode_pixels(frame, None).await {
            Ok(pixels) => {
                let [red, green, blue, _] = pixel_at(&pixels, 4, 4);
                report.expect(
                    [red, green, blue] == colours[frame as usize],
                    format!("frame {frame}: {:?}", [red, green, blue]),
                );
                if report.pictures.len() < 2 {
                    report.pictures.push(pixels);
                }
            }
            Err(error) => report.expect(false, format!("frame {frame}: {error}")),
        }
    }
    let past = image.decode_pixels(2, None).await;
    report.expect(
        past.as_ref().err().map(|error| error.kind) == Some(ImageErrorKind::Other),
        "no frame 2".to_owned(),
    );
    report
}

/// The PNG's pixels, encoded as PNG, lossy WebP and JPEG, each opened and decoded again.
async fn encoded_and_back(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("encoded and decoded back");
    let pixels = match decoded(codecs, ALPHA).await {
        Ok(pixels) => pixels,
        Err(error) => return failed(report, &error),
    };
    for (format, quality) in [
        ("image/png", None),
        ("image/webp", Some(0.8)),
        ("image/jpeg", Some(0.3)),
    ] {
        let copy = PixelBuffer::new(pixels.layout(), pixels.bytes().to_vec()).expect("a copy");
        let bytes = match codecs.encode_pixels(copy, format, quality).await {
            Ok(bytes) => bytes,
            Err(error) => {
                report.expect(false, format!("{format}: {error}"));
                continue;
            }
        };
        let length = bytes.len();
        match decoded(codecs, &bytes).await {
            Ok(back) => {
                let left = pixel_at(&back, 4, 16);
                report.expect(
                    back.layout().size == [32, 32] && left[1] > 230 && left[0] < 30,
                    format!("{format} at {quality:?}, {length} bytes, decodes back: {left:?}"),
                );
                if format == "image/png" {
                    report.expect(back.bytes() == pixels.bytes(), "PNG is lossless".to_owned());
                    report.pictures.push(back);
                }
            }
            Err(error) => report.expect(false, format!("{format} back: {error}")),
        }
    }
    photo_encoded_again(codecs, &mut report).await;
    let unknown = codecs.encode_pixels(pixels, "image/heic", None).await;
    report.expect(
        unknown.err().map(|error| error.kind) == Some(ImageErrorKind::Unsupported),
        "HEIC is not encoded".to_owned(),
    );
    report
}

/// The photo's upright pixels, encoded as JPEG, open upright with no metadata: encoding writes
/// none, so the location does not go with them.
async fn photo_encoded_again(codecs: &dyn ImageCodecs, report: &mut Report) {
    let encoded = match decoded(codecs, PHOTO).await {
        Ok(pixels) => codecs.encode_pixels(pixels, "image/jpeg", None).await,
        Err(error) => return report.expect(false, format!("the photo: {error}")),
    };
    let again = match encoded {
        Ok(bytes) => codecs.open(Arc::from(bytes)).await,
        Err(error) => return report.expect(false, format!("the photo encoded: {error}")),
    };
    match again {
        Ok(image) => report.expect(
            image.size() == Size::new(32.0, 48.0)
                && image.orientation().rotation == ImageRotation::None
                && image.metadata() == ImageMetadata::default(),
            format!(
                "the photo encoded again: {:?}, {:?}, metadata {}",
                image.size(),
                image.orientation().rotation,
                if image.metadata() == ImageMetadata::default() {
                    "none"
                } else {
                    "kept"
                }
            ),
        ),
        Err(error) => report.expect(false, format!("the photo encoded, opened: {error}")),
    }
}

/// Bytes of no image, damaged bytes, and a file that is not there, fail with their kinds.
async fn failures(codecs: &dyn ImageCodecs) -> Report {
    let mut report = Report::new("failures");
    let words = codecs.open(Arc::from(&b"these are words"[..])).await;
    let kind = words.err().map(|error| error.kind);
    report.expect(
        kind == Some(ImageErrorKind::Unsupported),
        format!("words: {kind:?}"),
    );
    let cut = codecs.open(Arc::from(&PHOTO[..PHOTO.len() / 2])).await;
    let kind = match cut {
        Ok(image) => image
            .decode_pixels(0, None)
            .await
            .err()
            .map(|error| error.kind),
        Err(error) => Some(error.kind),
    };
    report.expect(
        kind == Some(ImageErrorKind::Damaged),
        format!("half a JPEG: {kind:?}"),
    );
    let missing = codecs
        .open_file(&PathBuf::from("/data/no such photo.jpg"))
        .await;
    let kind = missing.err().map(|error| error.kind);
    report.expect(
        kind == Some(ImageErrorKind::NotFound),
        format!("no file: {kind:?}"),
    );
    report
}

/// `bytes` opened and their first frame decoded.
async fn decoded(codecs: &dyn ImageCodecs, bytes: &[u8]) -> Result<PixelBuffer, String> {
    let image = codecs
        .open(Arc::from(bytes))
        .await
        .map_err(|error| error.to_string())?;
    image
        .decode_pixels(0, None)
        .await
        .map_err(|error| error.to_string())
}

fn failed(mut report: Report, why: &str) -> Report {
    report.expect(false, why.to_owned());
    report
}

fn middle_of_row(pixels: &PixelBuffer, row: u32) -> [u8; 4] {
    pixel_at(pixels, pixels.layout().size[0] / 2, row)
}

/// The RGBA pixel at `x`, `y`.
fn pixel_at(pixels: &PixelBuffer, x: u32, y: u32) -> [u8; 4] {
    let layout = pixels.layout();
    let start = (y * layout.row_bytes + x * 4) as usize;
    let mut pixel = [0; 4];
    pixel.copy_from_slice(&pixels.bytes()[start..start + 4]);
    if layout.format == PixelFormat::Bgra8 {
        pixel.swap(0, 2);
    }
    pixel
}

fn close(made: [u32; 2], asked: [u32; 2]) -> bool {
    made[0].abs_diff(asked[0]) <= 1 && made[1].abs_diff(asked[1]) <= 1
}
