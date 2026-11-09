//! Image codecs in memory: each answers what the test set, at once, and keeps what the app asked
//! of it, the bytes and files opened, the frames decoded and the images encoded.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use waft_embedder::image_codecs::{
    AlphaType, HardwareImageCodecs, ImageCodecs, ImageError, ImageErrorKind, ImageFrame,
    ImageFuture, ImageMetadata, ImageOrientation, ImageSource, PixelBuffer, PixelFormat,
    PixelLayout,
};
use waft_embedder::{HardwareBuffer, HardwareBufferUsage, Size};

use crate::MockHardwareBuffer;

/// Image codecs that answer what the test set, at once, and keep what the app asked of them,
/// with hardware codecs where the test gives them. Every clone is the same codecs.
#[derive(Clone)]
pub struct MockImageCodecs(Rc<CodecsShared>);

struct CodecsShared {
    answers: RefCell<ImageAnswers>,
    asked: RefCell<Vec<CodecCall>>,
    hardware: Option<MockHardwareImageCodecs>,
}

/// What every image and every encoding answers.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageAnswers {
    pub decoded_formats: Vec<String>,
    pub encoded_formats: Vec<String>,
    /// What an opened image says of itself, or why it did not open.
    pub opened: Result<ImageFacts, ImageError>,
    /// The colour of every decoded pixel, as the bytes of an RGBA pixel, premultiplied.
    pub pixels: Result<[u8; 4], ImageError>,
    /// What every encoding answers.
    pub encoded: Result<Vec<u8>, ImageError>,
}

/// What an image says of itself.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageFacts {
    pub mime_type: String,
    pub size: Size,
    pub frames: Vec<ImageFrame>,
    pub orientation: ImageOrientation,
    pub metadata: ImageMetadata,
}

impl ImageFacts {
    /// A still image of `size` in `mime_type`, upright, its file saying nothing of how it was
    /// taken.
    pub fn still(mime_type: &str, size: Size) -> ImageFacts {
        ImageFacts {
            mime_type: mime_type.to_owned(),
            size,
            frames: vec![ImageFrame::default()],
            orientation: ImageOrientation::default(),
            metadata: ImageMetadata::default(),
        }
    }
}

/// What the app asked of the codecs, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum CodecCall {
    Open(Vec<u8>),
    OpenFile(PathBuf),
    /// A frame decoded into the app's memory, or into a hardware buffer.
    Decode {
        frame: u32,
        max_size: Option<Size>,
        hardware: bool,
    },
    /// Pixels of a size, or a hardware buffer's, encoded.
    Encode {
        width: u32,
        height: u32,
        format: String,
        quality: Option<f32>,
        hardware: bool,
    },
}

impl MockImageCodecs {
    /// Codecs that answer `answers`, with no hardware codecs.
    pub fn new(answers: ImageAnswers) -> MockImageCodecs {
        MockImageCodecs::of(answers, false)
    }

    /// Codecs that answer `answers`, with hardware codecs whose buffers are
    /// [`MockHardwareBuffer`]s.
    pub fn with_hardware(answers: ImageAnswers) -> MockImageCodecs {
        MockImageCodecs::of(answers, true)
    }

    fn of(answers: ImageAnswers, hardware: bool) -> MockImageCodecs {
        MockImageCodecs(Rc::new_cyclic(|shared| CodecsShared {
            answers: RefCell::new(answers),
            asked: RefCell::default(),
            hardware: hardware.then(|| MockHardwareImageCodecs(shared.clone())),
        }))
    }

    /// Each call answers `answers` from now on.
    pub fn set_answers(&self, answers: ImageAnswers) {
        *self.0.answers.borrow_mut() = answers;
    }

    /// What the app asked, in order.
    pub fn asked(&self) -> Vec<CodecCall> {
        self.0.asked.borrow().clone()
    }
}

impl CodecsShared {
    fn answers(&self) -> ImageAnswers {
        self.answers.borrow().clone()
    }

    fn ask(&self, call: CodecCall) {
        self.asked.borrow_mut().push(call);
    }

    /// An image of the facts the test set, or why it did not open.
    fn opened(self: &Rc<Self>) -> ImageFuture<Box<dyn ImageSource>> {
        let opened = self.answers().opened.map(|facts| {
            Box::new(MockImage {
                codecs: Rc::clone(self),
                facts,
            }) as Box<dyn ImageSource>
        });
        Box::pin(std::future::ready(opened))
    }

    /// What an encoding of `width` by `height` answers, kept as asked.
    fn encoded(
        &self,
        [width, height]: [u32; 2],
        format: &str,
        quality: Option<f32>,
        hardware: bool,
    ) -> ImageFuture<Vec<u8>> {
        self.ask(CodecCall::Encode {
            width,
            height,
            format: format.to_owned(),
            quality,
            hardware,
        });
        Box::pin(std::future::ready(self.answers().encoded))
    }
}

impl ImageCodecs for MockImageCodecs {
    fn decoded_formats(&self) -> Vec<String> {
        self.0.answers().decoded_formats
    }

    fn encoded_formats(&self) -> Vec<String> {
        self.0.answers().encoded_formats
    }

    fn open(&self, data: Arc<[u8]>) -> ImageFuture<Box<dyn ImageSource>> {
        self.0.ask(CodecCall::Open(data.to_vec()));
        self.0.opened()
    }

    fn open_file(&self, path: &Path) -> ImageFuture<Box<dyn ImageSource>> {
        self.0.ask(CodecCall::OpenFile(path.to_owned()));
        self.0.opened()
    }

    fn encode_pixels(
        &self,
        pixels: PixelBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>> {
        self.0.encoded(pixels.layout().size, format, quality, false)
    }

    fn hardware_image_codecs(&self) -> Option<&dyn HardwareImageCodecs> {
        self.0
            .hardware
            .as_ref()
            .map(|hardware| hardware as &dyn HardwareImageCodecs)
    }
}

/// The hardware codecs of [`MockImageCodecs::with_hardware`].
struct MockHardwareImageCodecs(std::rc::Weak<CodecsShared>);

impl HardwareImageCodecs for MockHardwareImageCodecs {
    fn decode(
        &self,
        image: &dyn ImageSource,
        frame: u32,
        max_size: Option<Size>,
    ) -> ImageFuture<Arc<dyn HardwareBuffer>> {
        let codecs = self
            .0
            .upgrade()
            .expect("the codecs hold their hardware codecs");
        codecs.ask(CodecCall::Decode {
            frame,
            max_size,
            hardware: true,
        });
        let image: &dyn std::any::Any = image;
        let decoded = match image.downcast_ref::<MockImage>() {
            Some(image) => image.checked(frame).map(|()| {
                let [width, height] = fitted(image.facts.size, max_size);
                Arc::new(MockHardwareBuffer {
                    width,
                    height,
                    usage: HardwareBufferUsage::GPU_SAMPLED,
                }) as Arc<dyn HardwareBuffer>
            }),
            None => Err(ImageError::new(
                ImageErrorKind::Unsupported,
                "an image these codecs did not open",
            )),
        };
        Box::pin(std::future::ready(decoded))
    }

    fn encode(
        &self,
        buffer: &dyn HardwareBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>> {
        let codecs = self
            .0
            .upgrade()
            .expect("the codecs hold their hardware codecs");
        codecs.encoded([buffer.width(), buffer.height()], format, quality, true)
    }
}

/// An image the app opened.
struct MockImage {
    codecs: Rc<CodecsShared>,
    facts: ImageFacts,
}

impl MockImage {
    /// Whether the image has frame `frame`.
    fn checked(&self, frame: u32) -> Result<(), ImageError> {
        if (frame as usize) < self.facts.frames.len() {
            Ok(())
        } else {
            Err(ImageError::new(
                ImageErrorKind::Other,
                format!("no frame {frame}"),
            ))
        }
    }
}

impl ImageSource for MockImage {
    fn mime_type(&self) -> String {
        self.facts.mime_type.clone()
    }

    fn size(&self) -> Size {
        self.facts.size
    }

    fn frames(&self) -> Vec<ImageFrame> {
        self.facts.frames.clone()
    }

    fn orientation(&self) -> ImageOrientation {
        self.facts.orientation
    }

    fn metadata(&self) -> ImageMetadata {
        self.facts.metadata.clone()
    }

    fn decode_pixels(&self, frame: u32, max_size: Option<Size>) -> ImageFuture<PixelBuffer> {
        self.codecs.ask(CodecCall::Decode {
            frame,
            max_size,
            hardware: false,
        });
        let decoded = self.checked(frame).and_then(|()| {
            let colour = self.codecs.answers().pixels?;
            Ok(pixels_of(
                fitted(self.facts.size, max_size),
                colour,
                PixelFormat::Rgba8,
            ))
        });
        Box::pin(std::future::ready(decoded))
    }
}

/// `size` scaled down to fit `max_size`, keeping its shape, in whole pixels, at least one each way.
fn fitted(size: Size, max_size: Option<Size>) -> [u32; 2] {
    let scale = max_size.map_or(1.0, |max| {
        (max.width() / size.width())
            .min(max.height() / size.height())
            .min(1.0)
    });
    let side = |length: f64| ((length * scale).round() as u32).max(1);
    [side(size.width()), side(size.height())]
}

/// `width` by `height` pixels of `colour`, four bytes each in `format`, premultiplied, packed.
pub(crate) fn pixels_of(
    [width, height]: [u32; 2],
    colour: [u8; 4],
    format: PixelFormat,
) -> PixelBuffer {
    let layout = PixelLayout::packed([width, height], format, AlphaType::Premultiplied);
    let bytes = colour.repeat(width as usize * height as usize);
    PixelBuffer::new(layout, bytes).expect("a packed layout of at least one pixel")
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("answers at once"),
        }
    }

    fn answers() -> ImageAnswers {
        ImageAnswers {
            decoded_formats: vec!["image/png".to_owned(), "image/heic".to_owned()],
            encoded_formats: vec!["image/png".to_owned()],
            opened: Ok(ImageFacts::still("image/heic", Size::new(400.0, 300.0))),
            pixels: Ok([255, 0, 0, 255]),
            encoded: Ok(b"encoded".to_vec()),
        }
    }

    #[test]
    fn an_image_opens_with_the_facts_the_test_set_and_decodes_to_fit() {
        let codecs = MockImageCodecs::new(answers());
        assert!(codecs.hardware_image_codecs().is_none());
        let image = now(codecs.open(Arc::from(&b"bytes"[..]))).unwrap();
        assert_eq!(image.mime_type(), "image/heic");
        assert_eq!(image.frames().len(), 1);
        let pixels = now(image.decode_pixels(0, Some(Size::new(200.0, 200.0)))).unwrap();
        assert_eq!(pixels.layout().size, [200, 150]);
        assert_eq!(&pixels.bytes()[..4], [255, 0, 0, 255]);
        let past = now(image.decode_pixels(1, None)).err().unwrap();
        assert_eq!(past.kind, ImageErrorKind::Other, "a frame it does not have");
        let encoded = now(codecs.encode_pixels(pixels, "image/png", Some(0.5)));
        assert_eq!(encoded.unwrap(), b"encoded");
        assert_eq!(
            codecs.asked(),
            [
                CodecCall::Open(b"bytes".to_vec()),
                CodecCall::Decode {
                    frame: 0,
                    max_size: Some(Size::new(200.0, 200.0)),
                    hardware: false
                },
                CodecCall::Decode {
                    frame: 1,
                    max_size: None,
                    hardware: false
                },
                CodecCall::Encode {
                    width: 200,
                    height: 150,
                    format: "image/png".to_owned(),
                    quality: Some(0.5),
                    hardware: false
                },
            ]
        );
    }

    #[test]
    fn hardware_codecs_decode_into_frames_and_encode_from_them() {
        let codecs = MockImageCodecs::with_hardware(answers());
        let hardware = codecs.hardware_image_codecs().expect("hardware codecs");
        let image = now(codecs.open_file(Path::new("/data/photo.heic"))).unwrap();
        let frame = now(hardware.decode(image.as_ref(), 0, None)).unwrap();
        assert_eq!([frame.width(), frame.height()], [400, 300]);
        let pixels = now(frame.read_pixels()).unwrap();
        assert_eq!(pixels.layout().size, [400, 300]);
        assert_eq!(
            now(hardware.encode(frame.as_ref(), "image/png", None)).unwrap(),
            b"encoded"
        );
        assert_eq!(
            codecs.asked()[0],
            CodecCall::OpenFile("/data/photo.heic".into())
        );
    }
}
