//! A video's frame as an image valo draws, with no copy: a Metal texture made on the frame's
//! `IOSurface` through CoreVideo's texture cache, wrapped as a wgpu texture, as Apple's samples
//! and Flutter's macOS embedder draw an external texture.
//!
//! The invariant a later editor must keep: the image holds the frame until the GPU is done with
//! it. A CoreVideo pool hands a frame's surface out again once nothing holds the frame, whatever
//! still reads the surface (`experiments/2026-11-01-corevideo-pool-reuse`), so the wgpu texture's
//! drop callback, which wgpu runs once the GPU has finished with it, is what lets the frame go.

use std::ptr::NonNull;

use inset_embedder::{Image, VideoFrame};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CFRetained;
use objc2_core_video::{
    CVMetalTexture, CVMetalTextureCache, CVMetalTextureGetTexture, CVPixelBufferGetHeight,
    CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth, kCVPixelFormatType_32BGRA,
    kCVPixelFormatType_32RGBA, kCVReturnSuccess,
};
use objc2_metal::{MTLPixelFormat, MTLTexture, MTLTextureType};

use crate::gpu::Gpu;
use crate::os::capabilities::VideoFrameImages;

/// CoreVideo's texture cache on the device valo draws with.
pub(crate) struct CoreVideoFrameImages {
    cache: CFRetained<CVMetalTextureCache>,
    device: wgpu::Device,
    images: valo::ImageContext,
}

impl VideoFrameImages for CoreVideoFrameImages {
    fn on(gpu: &Gpu) -> Option<CoreVideoFrameImages> {
        // SAFETY: the device is only read, to make the cache on it.
        let metal = unsafe { gpu.device.as_hal::<wgpu::hal::api::Metal>() }?;
        let mut cache: *mut CVMetalTextureCache = std::ptr::null_mut();
        // SAFETY: no attributes, and `cache` a valid place for the answer.
        let status = unsafe {
            CVMetalTextureCache::create(
                None,
                None,
                metal.raw_device(),
                None,
                NonNull::from(&mut cache),
            )
        };
        let cache = NonNull::new(cache).filter(|_| status == kCVReturnSuccess)?;
        Some(CoreVideoFrameImages {
            // SAFETY: a cache made just now, handed over with its one retain.
            cache: unsafe { CFRetained::from_raw(cache) },
            device: gpu.device.clone(),
            images: valo::ImageContext::new(gpu.device.clone(), gpu.queue.clone()),
        })
    }

    /// `None` for a frame in a format valo does not sample, 8-bit BGRA and RGBA being the ones it
    /// does, or one CoreVideo makes no texture of.
    fn image_of(&self, frame: &VideoFrame) -> Option<Image> {
        let (metal_format, format) = formats_of(frame)?;
        let size = [
            CVPixelBufferGetWidth(frame) as u32,
            CVPixelBufferGetHeight(frame) as u32,
        ];
        let texture = self.metal_texture(frame, metal_format, size)?;
        let raw = CVMetalTextureGetTexture(&texture.0)?;
        let held = HeldFrame {
            _frame: frame.clone(),
            _texture: texture,
        };
        let wgpu_texture = self.wrap(raw, format, size, held);
        self.cache.flush(0);
        self.images.import_texture(wgpu_texture, false).ok()
    }
}

impl CoreVideoFrameImages {
    /// The frame's Metal texture, which CoreVideo keeps on the frame's surface.
    fn metal_texture(
        &self,
        frame: &VideoFrame,
        format: MTLPixelFormat,
        [width, height]: [u32; 2],
    ) -> Option<CoreVideoTexture> {
        let mut texture: *mut CVMetalTexture = std::ptr::null_mut();
        // SAFETY: a frame CoreVideo made, its whole first plane, `texture` a valid place for the
        // answer.
        let status = unsafe {
            CVMetalTextureCache::create_texture_from_image(
                None,
                &self.cache,
                frame,
                None,
                format,
                width as usize,
                height as usize,
                0,
                NonNull::from(&mut texture),
            )
        };
        let texture = NonNull::new(texture).filter(|_| status == kCVReturnSuccess)?;
        // SAFETY: a texture made just now, handed over with its one retain.
        Some(CoreVideoTexture(unsafe { CFRetained::from_raw(texture) }))
    }

    /// `raw` as a wgpu texture valo samples, holding `frame` until wgpu has freed it.
    fn wrap(
        &self,
        raw: Retained<ProtocolObject<dyn MTLTexture>>,
        format: wgpu::TextureFormat,
        [width, height]: [u32; 2],
        frame: HeldFrame,
    ) -> wgpu::Texture {
        let extent = wgpu::hal::CopyExtent {
            width,
            height,
            depth: 1,
        };
        // SAFETY: a single-plane 2D texture CoreVideo made on this device, described as it is.
        let hal = unsafe {
            wgpu::hal::metal::Device::texture_from_raw(
                raw,
                format,
                MTLTextureType::Type2D,
                1,
                1,
                extent,
                Some(Box::new(move || drop(frame))),
            )
        };
        let descriptor = wgpu::TextureDescriptor {
            label: Some("video frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        // SAFETY: the texture is initialised by its decoder, so wgpu takes it as sampled, not
        // as one to clear.
        unsafe {
            self.device
                .create_texture_from_hal::<wgpu::hal::api::Metal>(
                    hal,
                    &descriptor,
                    wgpu::TextureUses::RESOURCE,
                )
        }
    }
}

/// The Metal and wgpu formats of a frame valo samples.
fn formats_of(frame: &VideoFrame) -> Option<(MTLPixelFormat, wgpu::TextureFormat)> {
    match CVPixelBufferGetPixelFormatType(frame) {
        format if format == kCVPixelFormatType_32BGRA => {
            Some((MTLPixelFormat::BGRA8Unorm, wgpu::TextureFormat::Bgra8Unorm))
        }
        format if format == kCVPixelFormatType_32RGBA => {
            Some((MTLPixelFormat::RGBA8Unorm, wgpu::TextureFormat::Rgba8Unorm))
        }
        _ => None,
    }
}

/// CoreVideo's texture on a frame.
struct CoreVideoTexture(CFRetained<CVMetalTexture>);

/// The frame and CoreVideo's texture on it, held by the wgpu texture until wgpu frees it.
struct HeldFrame {
    _frame: VideoFrame,
    _texture: CoreVideoTexture,
}

// SAFETY: a pixel buffer and a CoreVideo texture are reference counted atomically and only
// released here, from whichever thread wgpu frees the texture on. The bindings cannot say so.
unsafe impl Send for HeldFrame {}
unsafe impl Sync for HeldFrame {}

/// A 64 by 48 frame as a frame reader hands one out: BGRA, on an `IOSurface`.
#[cfg(test)]
fn test_frame() -> VideoFrame {
    use objc2_core_foundation::CFDictionary;
    use objc2_core_video::CVPixelBufferCreate;

    let attributes = super::frames::pixel_buffer_attributes();
    let attributes: *const CFDictionary = Retained::as_ptr(&attributes).cast();
    let mut buffer = std::ptr::null_mut();
    // SAFETY: the readers' own attributes, an NSDictionary bridged to the CFDictionary it is, and
    // `buffer` a valid place for the answer.
    let status = unsafe {
        CVPixelBufferCreate(
            None,
            64,
            48,
            kCVPixelFormatType_32BGRA,
            Some(&*attributes),
            NonNull::from(&mut buffer),
        )
    };
    assert_eq!(status, kCVReturnSuccess);
    // SAFETY: a buffer made just now, handed over with its one retain.
    unsafe { CFRetained::from_raw(NonNull::new(buffer).expect("a pixel buffer")) }
}

/// A reader handing out a new test frame each time a test puts one in.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct TestFrames(std::sync::Mutex<Option<TestFrame>>);

/// A frame handed between a test and its reader.
#[cfg(test)]
struct TestFrame(VideoFrame);

// SAFETY: a pixel buffer's retain count is atomic, and the test only moves it.
#[cfg(test)]
unsafe impl Send for TestFrame {}

#[cfg(test)]
impl TestFrames {
    /// A new frame for the reader's next read.
    pub(crate) fn put_new(&self) {
        *self.0.lock().unwrap() = Some(TestFrame(test_frame()));
    }
}

#[cfg(test)]
impl inset_embedder::VideoFrameReader for TestFrames {
    fn frame_for(&self, _display_time: std::time::Instant) -> Option<VideoFrame> {
        self.0.lock().unwrap().take().map(|frame| frame.0)
    }
}

#[cfg(test)]
mod tests {
    use objc2_core_video::CVPixelBufferGetIOSurface;

    use super::*;
    use crate::gpu::LazyGpu;

    /// The image keeps the frame, and with it the frame's surface in use, so its pool cannot hand
    /// the surface out again while the image may be drawn; once wgpu frees the image's texture,
    /// the frame goes.
    #[test]
    fn a_frames_image_holds_the_frame_until_wgpu_frees_it() {
        let gpu = LazyGpu::default();
        let images = CoreVideoFrameImages::on(gpu.get()).expect("CoreVideo's texture cache");
        let frame = test_frame();
        let surface = CVPixelBufferGetIOSurface(Some(&frame)).expect("the frame's surface");
        let image = images.image_of(&frame).expect("an image of the frame");
        assert_eq!(image.size(), [64, 48]);

        drop(frame);
        assert!(surface.is_in_use(), "the image holds the frame");
        drop(image);
        gpu.get()
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive");
        assert!(!surface.is_in_use(), "the image freed, the frame goes");
    }
}
