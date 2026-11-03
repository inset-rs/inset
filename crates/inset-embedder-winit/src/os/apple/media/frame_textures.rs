//! A video frame's texture on the app's GPU, with no copy: a Metal texture CoreVideo's texture
//! cache makes on the frame's `IOSurface`, made a wgpu texture by valo. Apple documents the cache
//! for a BGRA pixel buffer in CoreVideo's header (`CVMetalTextureCache.h`, at
//! `CVMetalTextureCacheCreateTextureFromImage`: "Mapping a BGRA buffer"), and Flutter's macOS
//! embedder draws an external texture so.
//!
//! A cache is made on one Metal device, so each Metal device a frame is made a texture on has its
//! own, kept for the process's life. Every wgpu device on a GPU is made on Metal's one device
//! object for that GPU, so the caches are as few as the GPUs.
//!
//! The invariant a later editor must keep: the texture holds the frame until the GPU is done with
//! it. A CoreVideo pool hands a frame's surface out again once nothing holds the frame, whatever
//! still reads the surface (`experiments/2026-11-01-corevideo-pool-reuse`), so the release valo
//! hands wgpu, which wgpu runs once the GPU has finished with the texture, is what lets the frame
//! go.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Mutex, PoisonError};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CFRetained;
use objc2_core_video::{
    CVMetalTexture, CVMetalTextureCache, CVMetalTextureGetTexture, CVPixelBuffer,
    CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
    kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_metal::{MTLDevice, MTLPixelFormat};

/// CoreVideo's texture cache on each Metal device a frame was made a texture on.
static TEXTURE_CACHES: TextureCaches = TextureCaches(Mutex::new(Vec::new()));

/// The texture of `frame`, a BGRA pixel buffer backed by an `IOSurface` as the readers ask their
/// pictures in, on `device`, holding the frame until wgpu frees it.
///
/// # Panics
///
/// On a device that is not Metal's, and where CoreVideo makes no texture of the frame, as wgpu's
/// default error handler panics on a texture wgpu cannot make.
pub(super) fn texture_of(
    frame: &CFRetained<CVPixelBuffer>,
    device: &wgpu::Device,
) -> wgpu::Texture {
    let texture = TEXTURE_CACHES.texture_of(frame, metal_device_of(device));
    import(
        device,
        HeldFrame {
            _frame: frame.clone(),
            texture,
        },
    )
}

/// The Metal device `device` was made on.
fn metal_device_of(device: &wgpu::Device) -> Retained<ProtocolObject<dyn MTLDevice>> {
    // SAFETY: the device is only read, to name the Metal device behind it.
    let metal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }
        .expect("a video frame's texture is made on a Metal device");
    metal.raw_device().clone()
}

/// The Metal texture `held` holds, as a wgpu texture on `device` that holds `held` until wgpu
/// frees it.
fn import(device: &wgpu::Device, held: HeldFrame) -> wgpu::Texture {
    let metal = CVMetalTextureGetTexture(&held.texture.0).expect("CoreVideo's Metal texture");
    let raw = NonNull::from(&*metal).cast::<c_void>();
    // SAFETY: a live texture on the device's Metal device, whose pixels the decoder finished
    // writing before it handed the frame out, and which stay as they are while `held` holds the
    // frame: its pool does not hand out the surface of a frame something holds
    // (`experiments/2026-11-01-corevideo-pool-reuse`).
    unsafe { valo::import_metal_texture(device, raw, Box::new(move || drop(held))) }
        .expect("valo samples a frame's BGRA texture")
}

/// CoreVideo's texture caches, one for each Metal device.
struct TextureCaches(Mutex<Vec<DeviceCache>>);

impl TextureCaches {
    /// CoreVideo's texture of `frame` on `device`, through the device's cache, which is made now
    /// if no frame was made a texture on the device before.
    fn texture_of(
        &self,
        frame: &CVPixelBuffer,
        device: Retained<ProtocolObject<dyn MTLDevice>>,
    ) -> CoreVideoTexture {
        let mut caches = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let index = match caches.iter().position(|cache| cache.is_on(&device)) {
            Some(index) => index,
            None => {
                caches.push(DeviceCache::on(device));
                caches.len() - 1
            }
        };
        caches[index].texture_of(frame)
    }
}

/// CoreVideo's texture cache on one Metal device.
struct DeviceCache {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    cache: CFRetained<CVMetalTextureCache>,
}

// SAFETY: the cache is reached through the caches' lock, from one thread at a time, and its retain
// count is atomic, as every CF object's; the Metal device is `Send` and `Sync` of itself. The
// binding cannot say so of the cache.
unsafe impl Send for DeviceCache {}

impl DeviceCache {
    /// A new cache on `device`.
    fn on(device: Retained<ProtocolObject<dyn MTLDevice>>) -> DeviceCache {
        let mut cache: *mut CVMetalTextureCache = std::ptr::null_mut();
        // SAFETY: no attributes, and `cache` a valid place for the answer.
        let status = unsafe {
            CVMetalTextureCache::create(None, None, &device, None, NonNull::from(&mut cache))
        };
        let cache = NonNull::new(cache)
            .filter(|_| status == kCVReturnSuccess)
            .expect("CoreVideo's texture cache on the Metal device");
        DeviceCache {
            device,
            // SAFETY: a cache made just now, handed over with its one retain.
            cache: unsafe { CFRetained::from_raw(cache) },
        }
    }

    fn is_on(&self, device: &ProtocolObject<dyn MTLDevice>) -> bool {
        std::ptr::eq(&*self.device, device)
    }

    /// The texture of `frame`'s whole picture. The cache is flushed after, as CoreVideo's header
    /// asks to be done periodically.
    fn texture_of(&self, frame: &CVPixelBuffer) -> CoreVideoTexture {
        debug_assert_eq!(
            CVPixelBufferGetPixelFormatType(frame),
            kCVPixelFormatType_32BGRA,
            "the readers ask their pictures in BGRA"
        );
        let mut texture: *mut CVMetalTexture = std::ptr::null_mut();
        // SAFETY: a frame CoreVideo made, its whole first plane in BGRA, `texture` a valid place
        // for the answer.
        let status = unsafe {
            CVMetalTextureCache::create_texture_from_image(
                None,
                &self.cache,
                frame,
                None,
                MTLPixelFormat::BGRA8Unorm,
                CVPixelBufferGetWidth(frame),
                CVPixelBufferGetHeight(frame),
                0,
                NonNull::from(&mut texture),
            )
        };
        self.cache.flush(0);
        let texture = NonNull::new(texture)
            .filter(|_| status == kCVReturnSuccess)
            .expect("CoreVideo's texture of the frame");
        // SAFETY: a texture made just now, handed over with its one retain.
        CoreVideoTexture(unsafe { CFRetained::from_raw(texture) })
    }
}

/// CoreVideo's texture on a frame.
struct CoreVideoTexture(CFRetained<CVMetalTexture>);

/// The frame and CoreVideo's texture on it, held by the wgpu texture until wgpu frees it.
struct HeldFrame {
    _frame: CFRetained<CVPixelBuffer>,
    texture: CoreVideoTexture,
}

// SAFETY: a pixel buffer and a CoreVideo texture are reference counted atomically and only
// released here, from whichever thread wgpu frees the texture on. The bindings cannot say so.
unsafe impl Send for HeldFrame {}
unsafe impl Sync for HeldFrame {}

/// A 64 by 48 frame as a frame reader hands one out: BGRA, on an `IOSurface`.
#[cfg(test)]
pub(super) fn test_frame() -> CFRetained<CVPixelBuffer> {
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

#[cfg(test)]
mod tests {
    use objc2_core_video::CVPixelBufferGetIOSurface;

    use super::*;
    use crate::gpu::LazyGpu;

    /// The texture keeps the frame, and with it the frame's surface in use, so its pool cannot
    /// hand the surface out again while the texture may be drawn; once wgpu frees the texture,
    /// the frame goes.
    #[test]
    fn a_frames_texture_holds_the_frame_until_wgpu_frees_it() {
        let gpu = LazyGpu::default();
        let device = &gpu.get().device;
        let frame = test_frame();
        let surface = CVPixelBufferGetIOSurface(Some(&frame)).expect("the frame's surface");
        let texture = texture_of(&frame, device);
        assert_eq!([texture.width(), texture.height()], [64, 48]);
        assert_eq!(texture.format(), wgpu::TextureFormat::Bgra8Unorm);

        drop(frame);
        assert!(surface.is_in_use(), "the texture holds the frame");
        drop(texture);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive");
        assert!(!surface.is_in_use(), "the texture freed, the frame goes");
    }
}
