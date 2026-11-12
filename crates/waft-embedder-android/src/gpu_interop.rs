//! A hardware buffer imported as a texture on a GPU device, on Android: a `VkImage` bound to the
//! `AHardwareBuffer`'s memory, imported through `VK_ANDROID_external_memory_android_hardware_buffer`
//! with a dedicated allocation, made a wgpu texture through wgpu-hal, for what the buffer's usage
//! allows. Only a device opened with the import's extensions takes one (`vulkan_imports`),
//! which Waft's own is where Vulkan can; a device the app opened itself imports none.
//!
//! The invariant: a texture of a buffer holds the buffer, and the image and memory over it, until
//! wgpu frees the texture, once the GPU is done with it, and its drop callback destroys the image
//! and frees the memory then.

// Importing memory into Vulkan and wrapping the image in a wgpu texture need `unsafe`; this module
// writes it for them.
#![allow(unsafe_code)]

use std::any::Any;

use ash::vk;
use waft_embedder::{HardwareBuffer, HardwareBufferUsage};
use wgpu::hal::api::Vulkan;

use crate::hardware_buffers::AndroidHardwareBuffer;

/// The one format a buffer is allocated in, and so the one its image and texture have.
const FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

/// `buffer` as a `VkImage` over its memory on `device`, made a wgpu texture that holds the buffer
/// until wgpu frees it; `None` for a buffer another host handed out, on a device not opened to
/// import hardware buffers, and where Vulkan makes no image of the buffer.
pub(crate) fn import_texture(
    buffer: &dyn HardwareBuffer,
    device: &wgpu::Device,
) -> Option<wgpu::Texture> {
    let buffer = (buffer as &dyn Any).downcast_ref::<AndroidHardwareBuffer>()?;
    if !crate::vulkan_imports::imports(device) {
        return None;
    }
    let size = wgpu::Extent3d {
        width: buffer.width(),
        height: buffer.height(),
        depth_or_array_layers: 1,
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("waft hardware buffer"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: texture_usages(buffer.usage()),
        view_formats: &[],
    };
    // SAFETY: the device is only read, and is let go before the texture it makes is handed to
    // wgpu.
    let hal = unsafe { device.as_hal::<Vulkan>() }?;
    let image = ImportedImage::of(buffer, &hal, descriptor.usage)
        .inspect_err(|error| crate::log::warn_to_log!("no image of a hardware buffer: {error}"))
        .ok()?;
    let hal_descriptor = wgpu::hal::TextureDescriptor {
        label: descriptor.label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: hal_uses(descriptor.usage),
        memory_flags: wgpu::hal::MemoryFlags::empty(),
        view_formats: Vec::new(),
    };
    let raw = image.image;
    // SAFETY: `raw` is a live 2D RGBA8 image of exactly this size, with one mip level and one
    // layer, made on the device's Vulkan device with every usage the descriptor names, its memory
    // bound; the drop callback destroys both, so wgpu-hal destroys neither.
    let texture = unsafe {
        hal.texture_from_raw(
            raw,
            &hal_descriptor,
            Some(Box::new(move || drop(image))),
            wgpu::hal::vulkan::TextureMemory::External,
        )
    };
    drop(hal);
    // SAFETY: the texture was made on `device` and matches the descriptor. It is new, its layout
    // undefined, which `UNINITIALIZED` says; wgpu-core takes its pixels as written, as the buffer
    // holds them.
    Some(unsafe {
        device.create_texture_from_hal::<Vulkan>(
            texture,
            &descriptor,
            wgpu::wgt::TextureUses::UNINITIALIZED,
        )
    })
}

/// A `VkImage` bound to a hardware buffer's imported memory, and the buffer, held until the
/// image and the memory are let go.
struct ImportedImage {
    device: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    _buffer: AndroidHardwareBuffer,
}

impl ImportedImage {
    /// The image of `buffer` on `device`, for what `usages` names; why Vulkan made none
    /// otherwise.
    fn of(
        buffer: &AndroidHardwareBuffer,
        device: &wgpu::hal::vulkan::Device,
        usages: wgpu::TextureUsages,
    ) -> Result<ImportedImage, String> {
        let raw = device.raw_device();
        let imports = ash::android::external_memory_android_hardware_buffer::Device::new(
            device.shared_instance().raw_instance(),
            raw,
        );
        let ahb = buffer.buffer().as_ptr().cast::<vk::AHardwareBuffer>();
        let mut format = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut properties =
            vk::AndroidHardwareBufferPropertiesANDROID::default().push_next(&mut format);
        // SAFETY: a live device opened with the extension and a live buffer, the out structures
        // live across the call.
        unsafe { imports.get_android_hardware_buffer_properties(ahb, &mut properties) }
            .map_err(|error| format!("vkGetAndroidHardwareBufferPropertiesANDROID: {error}"))?;
        let (allocation_size, memory_types) =
            (properties.allocation_size, properties.memory_type_bits);
        if format.format != FORMAT {
            return Err(format!(
                "the buffer's memory is in {:?}, not RGBA8",
                format.format
            ));
        }
        let image = create_image(raw, buffer, image_usage(usages))?;
        let mut import = vk::ImportAndroidHardwareBufferInfoANDROID::default().buffer(ahb);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let allocation = vk::MemoryAllocateInfo::default()
            .push_next(&mut import)
            .push_next(&mut dedicated)
            .allocation_size(allocation_size)
            .memory_type_index(memory_types.trailing_zeros());
        // SAFETY: a live device; the allocation imports the buffer for the image it is dedicated
        // to, as the extension asks, in a memory type the buffer's properties allow.
        let memory = match unsafe { raw.allocate_memory(&allocation, None) } {
            Ok(memory) => memory,
            Err(error) => {
                // SAFETY: the image made above, bound to nothing and used by nothing.
                unsafe { raw.destroy_image(image, None) };
                return Err(format!("vkAllocateMemory with the buffer: {error}"));
            }
        };
        let imported = ImportedImage {
            device: raw.clone(),
            image,
            memory,
            _buffer: buffer.clone(),
        };
        // SAFETY: the image and its dedicated memory, at offset 0, as the extension asks.
        unsafe { raw.bind_image_memory(image, memory, 0) }
            .map_err(|error| format!("vkBindImageMemory: {error}"))?;
        Ok(imported)
    }
}

impl Drop for ImportedImage {
    fn drop(&mut self) {
        // SAFETY: the image and memory this owns, which wgpu calls this for once the GPU is done
        // with the texture over them.
        unsafe {
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

/// An image over memory of the external kind a hardware buffer is, of `buffer`'s size, for
/// `usage`.
fn create_image(
    device: &ash::Device,
    buffer: &AndroidHardwareBuffer,
    usage: vk::ImageUsageFlags,
) -> Result<vk::Image, String> {
    let mut external = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
    let create = vk::ImageCreateInfo::default()
        .push_next(&mut external)
        .image_type(vk::ImageType::TYPE_2D)
        .format(FORMAT)
        .extent(vk::Extent3D {
            width: buffer.width(),
            height: buffer.height(),
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    // SAFETY: a live device; the create info and what it points to live across the call.
    unsafe { device.create_image(&create, None) }.map_err(|error| format!("vkCreateImage: {error}"))
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

/// wgpu-hal's name for what a texture of `usages` is for.
fn hal_uses(usages: wgpu::TextureUsages) -> wgpu::wgt::TextureUses {
    use wgpu::wgt::TextureUses;
    let uses = [
        (wgpu::TextureUsages::COPY_SRC, TextureUses::COPY_SRC),
        (wgpu::TextureUsages::COPY_DST, TextureUses::COPY_DST),
        (wgpu::TextureUsages::TEXTURE_BINDING, TextureUses::RESOURCE),
        (
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            TextureUses::COLOR_TARGET,
        ),
    ];
    uses.into_iter()
        .filter(|(texture, _)| usages.contains(*texture))
        .fold(TextureUses::empty(), |all, (_, hal)| all | hal)
}

/// Vulkan's usage for an image a texture of `usages` is over: copies are allowed any image over a
/// hardware buffer, sampling one allocated `GPU_SAMPLED_IMAGE` and drawing one allocated
/// `GPU_COLOR_OUTPUT`, as the extension's table of equivalent usages says.
fn image_usage(usages: wgpu::TextureUsages) -> vk::ImageUsageFlags {
    let flags = [
        (
            wgpu::TextureUsages::COPY_SRC,
            vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (
            wgpu::TextureUsages::COPY_DST,
            vk::ImageUsageFlags::TRANSFER_DST,
        ),
        (
            wgpu::TextureUsages::TEXTURE_BINDING,
            vk::ImageUsageFlags::SAMPLED,
        ),
        (
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
        ),
    ];
    flags
        .into_iter()
        .filter(|(texture, _)| usages.contains(*texture))
        .fold(vk::ImageUsageFlags::empty(), |all, (_, image)| all | image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_buffers_textures_copy_from_it_whatever_its_usage() {
        assert_eq!(
            texture_usages(HardwareBufferUsage::default()),
            wgpu::TextureUsages::COPY_SRC
        );
        assert_eq!(
            image_usage(texture_usages(HardwareBufferUsage::GPU_SAMPLED)),
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED
        );
    }

    #[test]
    fn a_buffer_drawn_into_is_copied_into_too() {
        let usage = HardwareBufferUsage {
            gpu_color_output: true,
            ..HardwareBufferUsage::default()
        };
        assert_eq!(
            image_usage(texture_usages(usage)),
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::COLOR_ATTACHMENT
        );
    }
}
