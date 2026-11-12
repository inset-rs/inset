//! A hardware buffer's image handed to Android's compositor and taken back, as Vulkan asks of
//! memory another user reads: released to `VK_QUEUE_FAMILY_FOREIGN_EXT` once the work that drew
//! it is submitted, and acquired back from it before it is drawn into again. Without the release
//! the compositor may not see what was written: on the emulator a buffer whose last write was a
//! copy showed black until it was released (`experiments/2026-11-11-android-surface-control`,
//! question 8). wgpu-core has no call for either, so each is a raw barrier in a command encoder of
//! its own, on wgpu's queue in order with the frame's work: the acquire submitted here, before the
//! frame is drawn, and the release handed back as commands for the view to submit as the frame's
//! last submission, which signals the fence the compositor waits on (`frame_fence`).
//!
//! The image keeps one layout across the handover: wgpu is first told to bring the texture to the
//! colour attachment's state, and both barriers keep its layout, so what wgpu tracks for the
//! texture stays true and the next draw into it finds what wgpu thinks is there. The invariants a
//! later editor must keep: between a release and its acquire nothing on Waft's device touches the
//! texture, wgpu's tracking of it included; a texture is acquired only after a release, never
//! when new; and both barriers keep `HANDED_OVER_LAYOUT`, which must stay the layout wgpu-hal
//! gives `HANDED_OVER_STATE`.

// Recording raw barriers into wgpu-hal's command encoder needs `unsafe`; this module writes it
// for them.
#![allow(unsafe_code)]

use ash::vk;
use wgpu::hal::api::Vulkan;
use wgpu::wgt::TextureUses;

/// The state wgpu tracks for a texture while the compositor holds its image, and the layout
/// wgpu-hal gives that state, which both barriers keep: a colour attachment's, which the next
/// draw into the buffer needs anyway.
const HANDED_OVER_STATE: TextureUses = TextureUses::COLOR_TARGET;
const HANDED_OVER_LAYOUT: vk::ImageLayout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;

/// The commands that hand `texture`'s image to the compositor once the work submitted before them
/// is done, to be submitted together: wgpu brings it to the handed-over state, then a barrier
/// releases it to the foreign queue family. `None`, reported, on a device not Vulkan's.
pub(crate) fn release_to_compositor(
    device: &wgpu::Device,
    texture: &wgpu::Texture,
) -> Option<[wgpu::CommandBuffer; 2]> {
    let mut to_handed_over_state = device.create_command_encoder(&Default::default());
    to_handed_over_state.transition_resources(
        std::iter::empty(),
        std::iter::once(wgpu::wgt::TextureTransition {
            texture,
            selector: None,
            state: HANDED_OVER_STATE,
        }),
    );
    let release = barrier_encoder(device, texture, Handover::Release)?;
    Some([to_handed_over_state.finish(), release.finish()])
}

/// Takes `texture`'s image back from the compositor, which released the buffer, before the work
/// submitted on `queue` after this draws into it. Reported and left undone on a device not
/// Vulkan's.
pub(crate) fn acquire_from_compositor(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) {
    let Some(acquire) = barrier_encoder(device, texture, Handover::Acquire) else {
        return;
    };
    queue.submit([acquire.finish()]);
}

/// Which way an image goes between Waft's queue family and the compositor's.
#[derive(Clone, Copy)]
enum Handover {
    Release,
    Acquire,
}

/// A command encoder holding only the barrier that moves `texture`'s image `handover`'s way;
/// `None`, reported, on a device not Vulkan's.
fn barrier_encoder(
    device: &wgpu::Device,
    texture: &wgpu::Texture,
    handover: Handover,
) -> Option<wgpu::CommandEncoder> {
    let Some((raw_device, queue_family, image)) = vulkan_handles(device, texture) else {
        crate::log::warn_to_log!("a buffer's image is not Vulkan's; not handed over");
        return None;
    };
    let barrier = image_barrier(image, queue_family, handover);
    let mut encoder = device.create_command_encoder(&Default::default());
    // SAFETY: the encoder is new and holds only this barrier, recorded into the command buffer
    // wgpu-hal opened, which it ends and submits itself; the image is the texture's, live while
    // the texture is, and the barrier keeps the layout wgpu tracks for it.
    unsafe {
        encoder.as_hal_mut::<Vulkan, _, _>(|hal| {
            let Some(hal) = hal else {
                crate::log::warn_to_log!("no Vulkan command buffer for a buffer's handover");
                return;
            };
            let (source_stage, destination_stage) = stages(handover);
            raw_device.cmd_pipeline_barrier(
                hal.raw_handle(),
                source_stage,
                destination_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        });
    }
    Some(encoder)
}

/// Vulkan's device, the family of the queue wgpu submits on, and `texture`'s image; `None` where
/// either is not Vulkan's.
fn vulkan_handles(
    device: &wgpu::Device,
    texture: &wgpu::Texture,
) -> Option<(ash::Device, u32, vk::Image)> {
    // SAFETY: the device and texture are only read, and let go before this returns.
    let hal_device = unsafe { device.as_hal::<Vulkan>() }?;
    // SAFETY: as above; the image is not destroyed through its handle.
    let image = unsafe { texture.as_hal::<Vulkan>()?.raw_handle() };
    Some((
        hal_device.raw_device().clone(),
        hal_device.queue_family_index(),
        image,
    ))
}

/// The barrier on the whole of `image` that moves it between `queue_family` and the foreign one,
/// `handover`'s way, its layout kept: a release makes the writes before it available, an acquire
/// makes the compositor's done before what follows.
fn image_barrier(
    image: vk::Image,
    queue_family: u32,
    handover: Handover,
) -> vk::ImageMemoryBarrier<'static> {
    let (source_family, destination_family, source_access, destination_access) = match handover {
        Handover::Release => (
            queue_family,
            vk::QUEUE_FAMILY_FOREIGN_EXT,
            vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::empty(),
        ),
        Handover::Acquire => (
            vk::QUEUE_FAMILY_FOREIGN_EXT,
            queue_family,
            vk::AccessFlags::empty(),
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        ),
    };
    vk::ImageMemoryBarrier::default()
        .src_access_mask(source_access)
        .dst_access_mask(destination_access)
        .old_layout(HANDED_OVER_LAYOUT)
        .new_layout(HANDED_OVER_LAYOUT)
        .src_queue_family_index(source_family)
        .dst_queue_family_index(destination_family)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
}

/// The stages a barrier `handover`'s way waits on and holds back: a release waits on everything
/// before it, an acquire holds back everything after it.
fn stages(handover: Handover) -> (vk::PipelineStageFlags, vk::PipelineStageFlags) {
    match handover {
        Handover::Release => (
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        ),
        Handover::Acquire => (
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_COMMANDS,
        ),
    }
}
