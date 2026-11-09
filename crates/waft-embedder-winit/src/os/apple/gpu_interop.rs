//! A hardware buffer imported as a texture on the app's GPU device, on macOS and iOS: a Metal
//! texture over the buffer's `IOSurface` (`newTextureWithDescriptor:iosurface:plane:`), made a
//! wgpu texture through wgpu-hal, for what the buffer was allocated for. A video frame's buffer is
//! its pixel buffer's surface (`media::video_frame`), imported the same way, so a frame is drawn
//! with no copy on the device the app gives.
//!
//! The invariant: a texture of a buffer holds the buffer, and the frame it is the memory of, until
//! wgpu frees it, once the GPU is done with it, since a CoreVideo pool hands a frame's surface out
//! again once nothing holds the frame, whatever still reads the surface
//! (`experiments/2026-11-01-corevideo-pool-reuse`).

// Wrapping an `IOSurface` in a Metal texture, and that in a wgpu texture, needs `unsafe`; this
// module writes it for them.
#![allow(unsafe_code)]

use std::any::Any;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_video::kCVPixelFormatType_32BGRA;
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage,
};
use waft_embedder::{HardwareBuffer, HardwareBufferUsage};

use super::hardware_buffers::SurfaceBuffer;

/// `buffer` as a Metal texture over its surface on `device`, made a wgpu texture that holds the
/// buffer until wgpu frees it; `None` for a buffer another host handed out, on a device not on
/// Metal, for a surface not in BGRA, and where Metal makes no texture of the surface.
pub(crate) fn import_texture(
    buffer: &dyn HardwareBuffer,
    device: &wgpu::Device,
) -> Option<wgpu::Texture> {
    let buffer = (buffer as &dyn Any).downcast_ref::<SurfaceBuffer>()?;
    let surface = buffer.surface();
    if surface.pixel_format() != kCVPixelFormatType_32BGRA {
        return None;
    }
    // SAFETY: the device is only read, and is let go before the texture it makes is handed to
    // wgpu.
    let hal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }?;
    let metal = metal_texture(surface, buffer.usage(), hal.raw_device())?;
    drop(hal);
    let size = wgpu::Extent3d {
        width: surface.width() as u32,
        height: surface.height() as u32,
        depth_or_array_layers: 1,
    };
    let held = buffer.clone();
    // SAFETY: `metal` is a live 2D BGRA8 texture of exactly this size, with one mip level and one
    // layer, made on the device's Metal device.
    let texture = unsafe {
        wgpu::hal::metal::Device::texture_from_raw(
            metal,
            wgpu::TextureFormat::Bgra8Unorm,
            MTLTextureType::Type2D,
            1,
            1,
            wgpu::hal::CopyExtent {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            Some(Box::new(move || drop(held))),
        )
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("waft hardware buffer"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: texture_usages(buffer.usage()),
        view_formats: &[],
    };
    // SAFETY: the texture was made on `device` and matches the descriptor, whose usage is within
    // what the Metal texture serves; its pixels are as the surface holds them, which wgpu takes as
    // written.
    Some(unsafe {
        device.create_texture_from_hal::<wgpu::hal::api::Metal>(
            texture,
            &descriptor,
            arriving_state(buffer.usage()),
        )
    })
}

/// A Metal texture over `surface` on `device`, for `usage`; `None` where Metal makes none.
fn metal_texture(
    surface: &IOSurfaceRef,
    usage: HardwareBufferUsage,
    device: &ProtocolObject<dyn MTLDevice>,
) -> Option<Retained<ProtocolObject<dyn MTLTexture>>> {
    let descriptor = MTLTextureDescriptor::new();
    descriptor.setTextureType(MTLTextureType::Type2D);
    descriptor.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    // SAFETY: both fit an `NSUInteger`, and the surface is of this size.
    unsafe {
        descriptor.setWidth(surface.width());
        descriptor.setHeight(surface.height());
    }
    // Shared is the storage mode a texture over an `IOSurface` takes: it makes the texture a view
    // of the surface rather than a private copy of it.
    descriptor.setStorageMode(MTLStorageMode::Shared);
    descriptor.setUsage(metal_usage(usage));
    device.newTextureWithDescriptor_iosurface_plane(&descriptor, surface, 0)
}

/// What a texture of a buffer of `usage` is for: sampled where the GPU samples the buffer, drawn
/// into and copied into where it draws into it, and copied from whatever its usage.
fn texture_usages(usage: HardwareBufferUsage) -> wgpu::TextureUsages {
    let mut usages = wgpu::TextureUsages::COPY_SRC;
    if usage.gpu_sampled {
        usages |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    if usage.gpu_color_output {
        usages |= wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST;
    }
    usages
}

/// What Metal is told a texture of a buffer of `usage` does: it is read by shaders, and drawn
/// into where the usage says. Copies need no usage of Metal's.
fn metal_usage(usage: HardwareBufferUsage) -> MTLTextureUsage {
    if usage.gpu_color_output {
        MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget
    } else {
        MTLTextureUsage::ShaderRead
    }
}

/// The state a texture of a buffer of `usage` arrives in, which its usage allows.
fn arriving_state(usage: HardwareBufferUsage) -> wgpu::wgt::TextureUses {
    if usage.gpu_sampled {
        wgpu::wgt::TextureUses::RESOURCE
    } else if usage.gpu_color_output {
        wgpu::wgt::TextureUses::COLOR_TARGET
    } else {
        wgpu::wgt::TextureUses::COPY_SRC
    }
}

#[cfg(test)]
mod tests {
    use objc2_core_video::CVPixelBufferGetIOSurface;
    use waft_embedder::image_codecs::PixelFormat;
    use waft_embedder::{HardwareBufferAllocator, VideoFrame};

    use super::*;
    use crate::gpu::LazyGpu;
    use crate::os::apple::hardware_buffers::SurfaceBufferAllocator;
    use crate::os::apple::media::test_frame;

    const DRAWN: HardwareBufferUsage = HardwareBufferUsage {
        gpu_sampled: true,
        gpu_color_output: true,
    };

    #[test]
    fn a_buffers_textures_copy_from_it_whatever_its_usage() {
        assert_eq!(
            texture_usages(HardwareBufferUsage::default()),
            wgpu::TextureUsages::COPY_SRC
        );
        assert_eq!(
            texture_usages(HardwareBufferUsage::GPU_SAMPLED),
            wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING
        );
    }

    #[test]
    fn a_buffer_drawn_into_is_copied_into_too() {
        let usage = HardwareBufferUsage {
            gpu_sampled: false,
            gpu_color_output: true,
        };
        assert_eq!(
            texture_usages(usage),
            wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
        );
    }

    #[test]
    fn a_buffer_allocated_is_drawn_into_and_read_back() {
        let gpu = LazyGpu::default();
        let gpu = gpu.get();
        let buffer = SurfaceBufferAllocator
            .allocate(8, 4, PixelFormat::Bgra8, DRAWN)
            .expect("an IOSurface of 8 by 4");
        let texture = import_texture(&*buffer, &gpu.device).expect("a Metal device takes it");
        assert_eq!([texture.width(), texture.height()], [8, 4]);
        assert!(
            texture
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST)
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive");
        let pixels = pollster::block_on(buffer.read_pixels()).expect("its pixels");
        assert_eq!(pixels.layout().format, PixelFormat::Bgra8);
        assert_eq!(&pixels.bytes()[..4], [0, 0, 255, 255], "red, as BGRA");
    }

    /// The texture of a frame's buffer keeps the frame, and with it the frame's surface in use, so
    /// its pool cannot hand the surface out again while the texture may be drawn; once wgpu frees
    /// the texture, the frame goes.
    #[test]
    fn a_frames_texture_holds_the_frame_until_wgpu_frees_it() {
        let gpu = LazyGpu::default();
        let device = &gpu.get().device;
        let frame = test_frame();
        let surface =
            CVPixelBufferGetIOSurface(Some(frame.pixel_buffer())).expect("the frame's surface");
        let texture =
            import_texture(&*frame.hardware_buffer(), device).expect("a Metal device takes it");
        assert_eq!([texture.width(), texture.height()], [64, 48]);
        assert_eq!(texture.format(), wgpu::TextureFormat::Bgra8Unorm);
        assert!(
            texture
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        );

        drop(frame);
        assert!(surface.is_in_use(), "the texture holds the frame");
        drop(texture);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device is alive");
        assert!(!surface.is_in_use(), "the texture freed, the frame goes");
    }

    #[test]
    fn a_buffer_another_host_handed_out_is_not_imported() {
        struct Elsewhere;

        impl HardwareBuffer for Elsewhere {
            fn width(&self) -> u32 {
                4
            }

            fn height(&self) -> u32 {
                4
            }

            fn read_pixels(
                &self,
            ) -> waft_embedder::image_codecs::ImageFuture<waft_embedder::image_codecs::PixelBuffer>
            {
                unreachable!("these tests never read it")
            }
        }

        let gpu = LazyGpu::default();
        assert!(import_texture(&Elsewhere, &gpu.get().device).is_none());
    }
}
