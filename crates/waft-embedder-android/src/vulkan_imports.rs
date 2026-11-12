//! A Vulkan device that imports hardware buffers and hands frames to the compositor: opened
//! through wgpu-hal with `VK_ANDROID_external_memory_android_hardware_buffer`, the
//! `VK_EXT_queue_family_foreign` it depends on, and `VK_KHR_external_semaphore_fd` added to the
//! extensions wgpu enables, which wgpu-hal never adds itself; and whether a device was opened so.
//! Every other part of the device is what wgpu-hal opens, so wgpu-core's device over it is any
//! wgpu device.
//!
//! The invariant: a device answers that it imports only when it was opened with all three
//! extensions, since `vkGetAndroidHardwareBufferPropertiesANDROID` and an import into a device
//! without them are undefined, and the composition path it answers for needs the other two.
//! `VK_EXT_queue_family_foreign` is what lets the view hand its buffers' images to the compositor
//! and take them back (`queue_family_handover`); `VK_KHR_external_semaphore_fd` is what gives the
//! compositor the fence each frame hands over with its buffer (`frame_fence`), its dependency
//! `VK_KHR_external_semaphore` being Vulkan 1.1's own.

// Opening a device through wgpu-hal and reading what a device enabled need `unsafe`; this module
// writes it for them.
#![allow(unsafe_code)]

use std::ffi::CStr;

use wgpu::hal::api::Vulkan;

/// What a device must enable to import hardware buffers and hand frames to the compositor.
const COMPOSITION_EXTENSIONS: [&CStr; 3] = [
    ash::android::external_memory_android_hardware_buffer::NAME,
    ash::ext::queue_family_foreign::NAME,
    ash::khr::external_semaphore_fd::NAME,
];

/// The features a device is asked for where the adapter offers them, as valo's own
/// `request_device` asks: timestamps for the frame's GPU time, and mappable primary buffers, which
/// valo writes each frame's uniforms and vertices into directly.
const WANTED_FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS);

/// A device and its queue on the first of `adapters`, Vulkan's, that imports hardware buffers,
/// hardware ones before software ones; `None` where none can.
pub(crate) fn open_on_first(
    mut adapters: Vec<wgpu::Adapter>,
) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    adapters.sort_by_key(|adapter| adapter.get_info().device_type == wgpu::DeviceType::Cpu);
    adapters.into_iter().find_map(|adapter| {
        if !can_import(&adapter) {
            return None;
        }
        match open(&adapter) {
            Ok((device, queue)) => Some((adapter, device, queue)),
            Err(error) => {
                crate::log::warn_to_log!(
                    "{} gave no device that imports hardware buffers: {error}",
                    adapter.get_info().name
                );
                None
            }
        }
    })
}

/// Whether `device` was opened with the extensions that import a hardware buffer's memory and
/// hand a frame to the compositor.
pub(crate) fn imports(device: &wgpu::Device) -> bool {
    // SAFETY: the device is only read, and let go before this returns.
    let Some(hal) = (unsafe { device.as_hal::<Vulkan>() }) else {
        return false;
    };
    let enabled = hal.enabled_device_extensions();
    COMPOSITION_EXTENSIONS
        .iter()
        .all(|extension| enabled.contains(extension))
}

/// Whether `adapter` is Vulkan's, at 1.1 or later, whose dedicated allocations an import takes,
/// and offers the three extensions.
fn can_import(adapter: &wgpu::Adapter) -> bool {
    // SAFETY: the adapter is only read, and let go before this returns.
    let Some(hal) = (unsafe { adapter.as_hal::<Vulkan>() }) else {
        return false;
    };
    let capabilities = hal.physical_device_capabilities();
    capabilities.properties().api_version >= ash::vk::API_VERSION_1_1
        && COMPOSITION_EXTENSIONS
            .iter()
            .all(|extension| capabilities.supports_extension(extension))
}

/// The size of the blocks the device takes memory from the system in: what Impeller asks of its
/// Vulkan allocator (`allocator_vk.cc`, `preferredLargeHeapBlockSize`).
const MEMORY_BLOCK_BYTES: u64 = 4 * 1024 * 1024;

/// How the device takes memory from the system: in blocks of one small size, and a block of its
/// own for anything larger, so a texture that comes costs an allocation of its size. wgpu's
/// default takes 128 to 256 MB at a time and gives a block back as it empties: where a frame's
/// lasting textures happened to fill the first block exactly, the full-size texture that comes
/// and goes with a backdrop blur cost a Pixel 2 XL a 256 MB allocation, 134 to 225 ms, each time
/// (measured on a WaOS program's device, which opened the same way).
const MEMORY_HINTS: wgpu::MemoryHints = wgpu::MemoryHints::Manual {
    suballocated_device_memory_block_size: MEMORY_BLOCK_BYTES..MEMORY_BLOCK_BYTES,
};

/// A device on `adapter` with its own limits, the features valo uses that it offers, and the
/// import's extensions.
fn open(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), String> {
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("waft"),
        required_features: adapter.features() & WANTED_FEATURES,
        required_limits: adapter.limits(),
        memory_hints: MEMORY_HINTS,
        ..Default::default()
    };
    // SAFETY: the features and limits are the adapter's own, the callback only adds extensions the
    // adapter offers (`can_import`), and the hal adapter is let go before wgpu takes the device
    // it opened.
    let opened = unsafe {
        let hal = adapter
            .as_hal::<Vulkan>()
            .ok_or("the adapter is not Vulkan's")?;
        hal.open_with_callback(
            descriptor.required_features,
            &descriptor.required_limits,
            &descriptor.memory_hints,
            Some(Box::new(|arguments| {
                for extension in COMPOSITION_EXTENSIONS {
                    if !arguments.extensions.contains(&extension) {
                        arguments.extensions.push(extension);
                    }
                }
            })),
        )
    }
    .map_err(|error| error.to_string())?;
    // SAFETY: the device was opened on this adapter's physical device, with the descriptor's
    // features and limits.
    unsafe { adapter.create_device_from_hal(opened, &descriptor) }
        .map_err(|error| error.to_string())
}
