//! Images: which decoders this build carries and where they run, the loader the platform decodes
//! with, started at the first image, and valo's codecs as the framework's.

use std::cell::{Cell, OnceCell};
use std::sync::Arc;

use inset_embedder::{
    ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame, ImageFrameFuture, ImageRepetition,
};
use valo::ImageContext;
use valo_codec::{Decoder, ImageLoader};

use crate::gpu::LazyGpu;
use crate::os::{ImageDecoding, ThisOs};

/// Selects where the built-in image decoders run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecodeExecution {
    /// Polls decoding on the caller's executor and starts no threads.
    #[cfg_attr(
        not(all(feature = "image-worker", not(target_arch = "wasm32"))),
        default
    )]
    Local,

    /// Owns decoder state on one native worker, waking the event loop for results.
    #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
    #[default]
    Worker,
}

/// `create_image_loader` registers the decoders this build carries, in preference order.
///
/// The platform's own codec comes first where it was compiled in: it reads formats no portable
/// decoder carries, and hands back frames the GPU can sample without an upload. The software
/// decoder takes whatever that one declines. The loader behaves the same either side of
/// [`DecodeExecution`].
pub fn create_image_loader(
    images: ImageContext,
    execution: DecodeExecution,
) -> std::io::Result<ImageLoader> {
    let decoders: Vec<Box<dyn Decoder>> = ThisOs::system_decoders()
        .into_iter()
        .chain(portable_decoders())
        .collect();
    match execution {
        DecodeExecution::Local => Ok(ImageLoader::new(images, decoders)),
        #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
        DecodeExecution::Worker => ImageLoader::with_worker(images, decoders),
    }
}

/// The decoders that run on every system, as this build carries them.
fn portable_decoders() -> Vec<Box<dyn Decoder>> {
    vec![
        #[cfg(feature = "image-software")]
        Box::new(valo_codec_software::SoftwareDecoder),
    ]
}

/// Makes the application's own image loader on the renderer's image store.
pub(crate) type CreateImageLoader = Box<dyn FnOnce(ImageContext) -> ImageLoader>;

/// The images the platform answers from: the loader that decodes for `open_image_codec`, and the
/// renderer's image store, for images the app brings its own pixels or textures for. Both are made
/// at the first image the app decodes or brings, on the device every view draws with, which that
/// makes if no view has drawn yet (`gpu.rs`).
pub(crate) struct Images {
    /// The device the images belong to, which every view draws with.
    gpu: LazyGpu,
    /// The application's loader, made once the store exists; `None` for the built-in one.
    create_loader: Cell<Option<CreateImageLoader>>,
    started: OnceCell<StartedImages>,
}

/// The image store and the loader that decodes into it. Decoded images belong to the device and
/// do not retain a window surface.
struct StartedImages {
    store: ImageContext,
    loader: ImageLoader,
}

impl Images {
    /// The images on `gpu`'s device, decoded by the loader `create_loader` makes, or by the
    /// built-in one.
    pub(crate) fn new(gpu: LazyGpu, create_loader: Option<CreateImageLoader>) -> Images {
        Images {
            gpu,
            create_loader: Cell::new(create_loader),
            started: OnceCell::new(),
        }
    }

    /// The renderer's image store.
    pub(crate) fn store(&self) -> ImageContext {
        self.started().store.clone()
    }

    /// Opens a codec for an encoded image, decoded with mip levels.
    pub(crate) fn open_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        if bytes.is_empty() {
            return Box::pin(std::future::ready(Err(ImageDecodeError::Empty)));
        }
        let loader = self.started().loader.clone();
        Box::pin(async move {
            let codec = loader
                .open(
                    bytes,
                    valo_codec::DecodeOptions {
                        // Pictures are routinely drawn smaller than they decode, and mip
                        // levels are the difference between a smooth downscale and a
                        // shimmering one.
                        mipmaps: true,
                        ..Default::default()
                    },
                )
                .await
                .map_err(as_decode_error)?;
            Ok(Box::new(WinitImageCodec { codec }) as Box<dyn ImageCodec>)
        })
    }

    fn started(&self) -> &StartedImages {
        self.started.get_or_init(|| self.start())
    }

    /// The image store on the device, and the loader decoding into it.
    fn start(&self) -> StartedImages {
        let gpu = self.gpu.get();
        let store = ImageContext::new(gpu.device.clone(), gpu.queue.clone());
        let loader = match self.create_loader.take() {
            Some(create_loader) => create_loader(store.clone()),
            None => create_image_loader(store.clone(), DecodeExecution::default())
                .expect("start the image decode worker"),
        };
        StartedImages { store, loader }
    }
}

/// Adapts Valo's drawable frames to the framework's codec contract.
struct WinitImageCodec {
    codec: valo_codec::Codec,
}

impl ImageCodec for WinitImageCodec {
    fn frame_count(&self) -> u32 {
        self.codec.info().frame_count
    }

    fn repetition(&self) -> ImageRepetition {
        match self.codec.info().repetition {
            valo_codec::Repetition::Once => ImageRepetition::Once,
            valo_codec::Repetition::Times(times) => ImageRepetition::Times(times),
            valo_codec::Repetition::Forever => ImageRepetition::Forever,
        }
    }

    fn next_frame(&mut self) -> ImageFrameFuture<'_> {
        let frame = self.codec.next_frame();
        Box::pin(async move {
            let frame = frame.await.map_err(as_decode_error)?;
            Ok(ImageFrame {
                image: frame.image,
                duration: frame.duration,
            })
        })
    }
}

/// Preserves decoder diagnostics at the framework's existing error boundary.
fn as_decode_error(error: valo_codec::DecodeError) -> ImageDecodeError {
    match error {
        valo_codec::DecodeError::NoDecoder => ImageDecodeError::NoDecoder,
        valo_codec::DecodeError::Unsupported(_) => ImageDecodeError::UnknownFormat,
        valo_codec::DecodeError::InvalidData(reason) => ImageDecodeError::Damaged(reason),
        other => ImageDecodeError::Failed(other.to_string()),
    }
}
