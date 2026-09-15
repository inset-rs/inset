//! Browser `createImageBitmap` into a valo image.

use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Poll, Waker};

use inset_embedder::{
    Image, ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame, ImageFrameFuture,
    ImageRepetition,
};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, ImageBitmap};

use crate::platform::OnSchedule;

/// Opens `bytes` with the browser decoder and uploads into `images`.
pub fn open_codec(
    bytes: Arc<[u8]>,
    images: valo::ImageContext,
    frame_requested: Rc<Cell<bool>>,
    on_schedule: OnSchedule,
) -> ImageCodecFuture {
    let slot: Rc<RefCell<Slot>> = Rc::new(RefCell::new(Slot {
        result: None,
        waker: None,
    }));
    let pending = Rc::clone(&slot);
    wasm_bindgen_futures::spawn_local(async move {
        let result = decode(bytes, images).await;
        let waker = {
            let mut slot = pending.borrow_mut();
            slot.result = Some(result);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        if !frame_requested.replace(true)
            && let Some(schedule) = on_schedule.borrow().as_ref()
        {
            schedule();
        }
    });
    Box::pin(poll_fn(move |cx| {
        let mut slot = slot.borrow_mut();
        if let Some(result) = slot.result.take() {
            Poll::Ready(result)
        } else {
            slot.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }))
}

struct Slot {
    result: Option<Result<Box<dyn ImageCodec>, ImageDecodeError>>,
    waker: Option<Waker>,
}

async fn decode(
    bytes: Arc<[u8]>,
    images: valo::ImageContext,
) -> Result<Box<dyn ImageCodec>, ImageDecodeError> {
    if bytes.is_empty() {
        return Err(ImageDecodeError::Empty);
    }
    let bitmap = bitmap_from_bytes(&bytes).await?;
    let image = upload_bitmap(&images, &bitmap)?;
    bitmap.close();
    Ok(Box::new(StillCodec { image }) as Box<dyn ImageCodec>)
}

async fn bitmap_from_bytes(bytes: &[u8]) -> Result<ImageBitmap, ImageDecodeError> {
    let window = web_sys::window().ok_or(ImageDecodeError::NoDecoder)?;
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let blob = Blob::new_with_u8_array_sequence(&parts)
        .map_err(|error| ImageDecodeError::Failed(format!("{error:?}")))?;
    let promise = window
        .create_image_bitmap_with_blob(&blob)
        .map_err(|error| ImageDecodeError::Failed(format!("{error:?}")))?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|error| ImageDecodeError::Damaged(format!("{error:?}")))?;
    value
        .dyn_into::<ImageBitmap>()
        .map_err(|_| ImageDecodeError::Failed("createImageBitmap did not return a bitmap".into()))
}

fn upload_bitmap(
    images: &valo::ImageContext,
    bitmap: &ImageBitmap,
) -> Result<Image, ImageDecodeError> {
    let size = [bitmap.width(), bitmap.height()];
    if size.contains(&0) {
        return Err(ImageDecodeError::Empty);
    }
    images.check_size(size).map_err(|error| {
        ImageDecodeError::Failed(format!("image {size:?} is not uploadable: {error:?}"))
    })?;
    let texture = images.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("inset.web.image"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    images.queue().copy_external_image_to_texture(
        &wgpu::CopyExternalImageSourceInfo {
            source: wgpu::ExternalImageSource::ImageBitmap(bitmap.clone()),
            origin: wgpu::Origin2d::ZERO,
            flip_y: false,
        },
        wgpu::CopyExternalImageDestInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
            color_space: wgpu::PredefinedColorSpace::Srgb,
            premultiplied_alpha: true,
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
    images
        .import_texture(texture, true)
        .map_err(|error| ImageDecodeError::Failed(format!("{error:?}")))
}

struct StillCodec {
    image: Image,
}

impl ImageCodec for StillCodec {
    fn frame_count(&self) -> u32 {
        1
    }

    fn repetition(&self) -> ImageRepetition {
        ImageRepetition::Once
    }

    fn next_frame(&mut self) -> ImageFrameFuture<'_> {
        let image = self.image.clone();
        Box::pin(async move {
            Ok(ImageFrame {
                image,
                duration: std::time::Duration::ZERO,
            })
        })
    }
}
