//! The system's image codecs, Image I/O's: a photo opened by its file says its format, its size
//! upright, its one frame, how its picture is turned, and what its EXIF says of how it was taken,
//! its location among it; decoded into the app's memory at a size, it comes upright, scaled down
//! to fit; decoded by the hardware codecs, it is the host's own hardware buffer, an `IOSurface`,
//! whose pixels read back in BGRA; and both encode again, the app's pixels and the buffer's, into
//! an image that opens with no metadata, the buffer as HEIC too where the system encodes it, which
//! decodes back into a buffer. An animation's bytes open with each frame's delay, and bytes of no
//! image fail with the kind of their failure. All without a window.
//!
//! Its own binary, without the test harness, as the media tests are. The photo,
//! `tests/media/photo.jpg`, is 48 by 32 pixels as stored, its left half red and its right half
//! blue, with EXIF saying to turn it a quarter clockwise to show it, so upright it is 32 by 48, red
//! above and blue below; the animation, `tests/media/three-frames.gif`, three frames of 8 by 8,
//! red, green and blue, each shown a tenth of a second.

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() {
    codecs::decode_and_encode_through_image_io();
    println!("an_image_decodes_and_encodes_through_the_systems_codecs: ok");
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn main() {}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod codecs {
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use waft_embedder::image_codecs::{
        AlphaType, DateTime, ImageCodecs, ImageErrorKind, ImageRotation, ImageSource, PixelBuffer,
        PixelFormat,
    };
    use waft_embedder::{HardwareBuffer, Size};
    use waft_embedder_winit::SurfaceBuffer;

    /// Far longer than decoding a small image takes.
    const PATIENCE: Duration = Duration::from_secs(20);

    pub(crate) fn decode_and_encode_through_image_io() {
        let codecs =
            waft_embedder_winit::test_support::image_codecs().expect("the system's image codecs");
        for format in ["image/jpeg", "image/png", "image/heic", "image/gif"] {
            assert!(
                codecs.decoded_formats().iter().any(|read| read == format),
                "Image I/O decodes {format}"
            );
        }
        let photo = waited("the photo", codecs.open_file(&media("photo.jpg"))).expect("it opens");
        says_what_it_is(photo.as_ref());
        let pixels = waited(
            "its pixels",
            photo.decode_pixels(0, Some(Size::new(16.0, 16.0))),
        )
        .expect("its pixels");
        is_red_above_blue(&pixels, PixelFormat::Rgba8, [11, 16]);

        let hardware = codecs.hardware_image_codecs().expect("the hardware codecs");
        let buffer =
            waited("its buffer", hardware.decode(photo.as_ref(), 0, None)).expect("a buffer");
        let host_buffer: &dyn std::any::Any = &*buffer;
        assert!(host_buffer.is::<SurfaceBuffer>(), "the host's own buffer");
        let read = waited("the buffer's pixels", buffer.read_pixels()).expect("its pixels");
        is_red_above_blue(&read, PixelFormat::Bgra8, [32, 48]);

        encodes_again(codecs.as_ref(), pixels, &buffer);
        an_animation_opens_with_its_delays(codecs.as_ref());
        what_is_no_image_fails(codecs.as_ref());
    }

    /// The photo's format, size upright, frame, orientation and metadata.
    fn says_what_it_is(photo: &dyn ImageSource) {
        assert_eq!(photo.mime_type(), "image/jpeg");
        assert_eq!(photo.size(), Size::new(32.0, 48.0), "upright");
        assert_eq!(photo.frames().len(), 1);
        assert_eq!(photo.frames()[0].duration, None, "a still image");
        let orientation = photo.orientation();
        assert_eq!(
            (orientation.mirrored, orientation.rotation),
            (false, ImageRotation::QuarterTurn)
        );
        let metadata = photo.metadata();
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
        assert_near(metadata.exposure_time, 1.0 / 120.0);
        assert_near(metadata.f_number, 1.8);
        assert_eq!(metadata.iso, Some(200));
        assert_near(metadata.focal_length, 4.2);
        assert_near(metadata.equivalent_focal_length, 26.0);
        assert_near(metadata.exposure_bias, -1.0 / 3.0);
        assert_eq!(metadata.flash_fired, Some(true));
        let location = metadata.location.expect("where it was taken");
        assert!((location.latitude - (48.0 + 51.0 / 60.0 + 3.0 / 3600.0)).abs() < 1e-6);
        assert!(
            (location.longitude + (2.0 + 17.0 / 60.0 + 42.0 / 3600.0)).abs() < 1e-6,
            "west"
        );
        assert_near(location.altitude, 35.0);
    }

    /// `pixels` are `size`, upright, in `format` and premultiplied: red at the top, blue at the
    /// bottom, as JPEG keeps colours.
    fn is_red_above_blue(pixels: &PixelBuffer, format: PixelFormat, size: [u32; 2]) {
        let layout = pixels.layout();
        let close = |made: u32, asked: u32| made.abs_diff(asked) <= 1;
        assert!(
            close(layout.size[0], size[0]) && close(layout.size[1], size[1]),
            "scaled down to fit, upright: {:?}",
            layout.size
        );
        assert_eq!(
            (layout.format, layout.alpha),
            (format, AlphaType::Premultiplied)
        );
        let at = |row: u32| {
            let start = (row * layout.row_bytes + layout.size[0] / 2 * 4) as usize;
            let [first, green, third, alpha] = pixels.bytes()[start..start + 4] else {
                unreachable!()
            };
            match format {
                PixelFormat::Rgba8 => [first, green, third, alpha],
                PixelFormat::Bgra8 => [third, green, first, alpha],
            }
        };
        let [red, green, blue, alpha] = at(1);
        assert!(
            red > 200 && green < 60 && blue < 60 && alpha == 255,
            "red above: {:?}",
            at(1)
        );
        let bottom = layout.size[1] - 2;
        let [red, green, blue, _] = at(bottom);
        assert!(
            red < 60 && green < 60 && blue > 200,
            "blue below: {:?}",
            at(bottom)
        );
    }

    /// The pixels and the buffer encode, as PNG and as a JPEG of a low quality, into images that
    /// open at their sizes, upright, with no metadata.
    fn encodes_again(
        codecs: &dyn ImageCodecs,
        pixels: PixelBuffer,
        buffer: &Arc<dyn HardwareBuffer>,
    ) {
        assert!(
            codecs
                .encoded_formats()
                .iter()
                .any(|written| written == "image/png")
        );
        let [width, height] = pixels.layout().size;
        let png = waited(
            "the pixels encoded",
            codecs.encode_pixels(pixels, "image/png", None),
        )
        .expect("a PNG");
        let png = waited("the PNG", codecs.open(Arc::from(png))).expect("it opens");
        assert_eq!(png.mime_type(), "image/png");
        assert_eq!(png.size(), Size::new(f64::from(width), f64::from(height)));
        let hardware = codecs.hardware_image_codecs().expect("the hardware codecs");
        let jpeg = waited(
            "the buffer encoded",
            hardware.encode(buffer.as_ref(), "image/jpeg", Some(0.3)),
        )
        .expect("a JPEG");
        let jpeg = waited("the JPEG", codecs.open(Arc::from(jpeg))).expect("it opens");
        assert_eq!(jpeg.size(), Size::new(32.0, 48.0));
        assert_eq!(jpeg.orientation().rotation, ImageRotation::None);
        assert_eq!(jpeg.metadata().location, None, "no metadata is written");
        if codecs
            .encoded_formats()
            .iter()
            .any(|written| written == "image/heic")
        {
            let heic = waited(
                "the buffer encoded as HEIC",
                hardware.encode(buffer.as_ref(), "image/heic", None),
            )
            .expect("a HEIC");
            let heic = waited("the HEIC", codecs.open(Arc::from(heic))).expect("it opens");
            assert_eq!(
                (heic.mime_type().as_str(), heic.size()),
                ("image/heic", Size::new(32.0, 48.0))
            );
            let decoded = waited("the HEIC decoded", hardware.decode(heic.as_ref(), 0, None))
                .expect("a buffer");
            let read = waited("its pixels", decoded.read_pixels()).expect("its pixels");
            is_red_above_blue(&read, PixelFormat::Bgra8, [32, 48]);
        }
        let unknown = waited(
            "a format no codec writes",
            hardware.encode(buffer.as_ref(), "image/x-nothing", None),
        );
        assert_eq!(unknown.unwrap_err().kind, ImageErrorKind::Unsupported);
    }

    /// The animation's three frames, each with its delay.
    fn an_animation_opens_with_its_delays(codecs: &dyn ImageCodecs) {
        let bytes = std::fs::read(media("three-frames.gif")).expect("the animation");
        let animation = waited("the animation", codecs.open(Arc::from(bytes))).expect("it opens");
        assert_eq!(animation.mime_type(), "image/gif");
        let delays: Vec<_> = animation
            .frames()
            .iter()
            .map(|frame| frame.duration)
            .collect();
        assert_eq!(delays, [Some(Duration::from_millis(100)); 3]);
        let last = waited("its last frame", animation.decode_pixels(2, None)).expect("decoded");
        let [red, green, blue, _] = last.bytes()[..4] else {
            unreachable!()
        };
        assert!(
            blue > 200 && red < 60 && green < 60,
            "the blue frame: {:?}",
            &last.bytes()[..4]
        );
        let past = waited("a frame it has not", animation.decode_pixels(3, None));
        assert_eq!(
            past.err().map(|error| error.kind),
            Some(ImageErrorKind::Other)
        );
    }

    /// Bytes of no image, and a file that is not there, fail with their kinds.
    fn what_is_no_image_fails(codecs: &dyn ImageCodecs) {
        let words = waited("words", codecs.open(Arc::from(&b"these are words"[..])));
        assert_eq!(
            words.err().map(|error| error.kind),
            Some(ImageErrorKind::Unsupported)
        );
        let missing = waited("no file", codecs.open_file(&media("no such photo.jpg")));
        assert_eq!(
            missing.err().map(|error| error.kind),
            Some(ImageErrorKind::NotFound)
        );
    }

    fn media(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/media")
            .join(name)
    }

    fn assert_near(value: Option<f64>, expected: f64) {
        let value = value.expect("a value");
        assert!((value - expected).abs() < 1e-6, "{value} is not {expected}");
    }

    /// What `future` answers, waiting for Image I/O's queue, failing once `PATIENCE` has passed.
    fn waited<T>(what: &str, future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let started = Instant::now();
        loop {
            if let Poll::Ready(answer) = future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                return answer;
            }
            assert!(
                started.elapsed() < PATIENCE,
                "{what} took over {PATIENCE:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
