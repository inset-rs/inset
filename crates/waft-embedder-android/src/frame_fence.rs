//! The fence a frame hands the compositor with its buffer, as Impeller's hardware-buffer swapchain
//! does (`SubmitSignalForPresentReady` and `ExternalSemaphoreVK`): the frame's last submission
//! signals a binary semaphore made exportable as a sync file, and the sync file exported from it
//! after that submission is the buffer's acquire fence in the transaction that presents it, so
//! SurfaceFlinger waits for the frame on the GPU and no thread waits for it on the CPU. wgpu-core
//! owns the queue's submissions; wgpu-hal's `Queue::add_signal_semaphore` makes the next one signal
//! one more semaphore, which is how the frame's own last submission signals this one.
//!
//! The invariants a later editor must keep:
//!
//! - Nothing submits on Waft's queue between the semaphore being added and the frame's last
//!   submission: an earlier submission would signal it, and Android could show the frame before
//!   its image is released to the compositor. `submit_signalling` adds it and submits with no call
//!   in between, and every submission on Waft's queue is made on the loop's thread: valo's
//!   renderer, valo's image uploads, copies and mip passes, the platform's imports into the device,
//!   and the handover; valo-codec's image worker decodes only and makes the image on the caller's
//!   thread. A submission on the queue from another thread would break this.
//! - The sync file is exported only after the submission, as Vulkan asks of one (its semaphore
//!   must have a signal pending), and a semaphore is destroyed only once the submission that
//!   signals it is done: `FrameSubmission` waits for that submission when it goes.

// Making, exporting and destroying Vulkan semaphores, and adding one to wgpu-hal's queue, need
// `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::Arc;

use ash::vk;
use wgpu::hal::api::Vulkan;

/// What makes each frame's fence.
pub(crate) struct FrameFences(Arc<FenceDevice>);

/// Waft's device and queue, and Vulkan's calls that make, export and destroy semaphores on that
/// device.
struct FenceDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    raw_device: ash::Device,
    semaphore_fd: ash::khr::external_semaphore_fd::Device,
}

/// A frame's last submission, made: what holds its semaphore until it is done, and the fence the
/// compositor waits on before it reads the frame, none where the GPU is done with it already.
pub(crate) struct SignalledFrame {
    pub(crate) submission: FrameSubmission,
    pub(crate) acquire_fence: Option<OwnedFd>,
}

/// A frame's last submission and the semaphore it signals, destroyed once it is done.
pub(crate) struct FrameSubmission {
    device: Arc<FenceDevice>,
    semaphore: vk::Semaphore,
    index: wgpu::SubmissionIndex,
}

impl FrameFences {
    /// The fences of frames submitted on `queue`, on `device`, which must have been opened with
    /// `VK_KHR_external_semaphore_fd` (`vulkan_imports`); `None` on a device not Vulkan's.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<FrameFences> {
        // SAFETY: the device is only read, and let go before this returns; the handles cloned
        // from it live while `device`, held here, does.
        let hal = unsafe { device.as_hal::<Vulkan>() }?;
        let raw_device = hal.raw_device().clone();
        let semaphore_fd = ash::khr::external_semaphore_fd::Device::new(
            hal.shared_instance().raw_instance(),
            &raw_device,
        );
        Some(FrameFences(Arc::new(FenceDevice {
            device: device.clone(),
            queue: queue.clone(),
            raw_device,
            semaphore_fd,
        })))
    }

    /// Submits `commands` as the frame's last submission, signalling a semaphore of its own, and
    /// answers the submission and the fence exported from it; `None`, reported, where Vulkan made
    /// no semaphore, the submission did not reach the queue, or no fence could be exported.
    pub(crate) fn submit_signalling(
        &self,
        commands: impl IntoIterator<Item = wgpu::CommandBuffer>,
    ) -> Option<SignalledFrame> {
        let device = &self.0;
        let semaphore = device.exportable_semaphore()?;
        let Some(index) = device.submit_with_signal(semaphore, commands) else {
            crate::log::warn_to_log!("the view's frame was not submitted; it is not shown");
            // SAFETY: no submission signals it: it was taken back from the queue unsubmitted.
            unsafe { device.raw_device.destroy_semaphore(semaphore, None) };
            return None;
        };
        let submission = FrameSubmission {
            device: Arc::clone(device),
            semaphore,
            index,
        };
        let acquire_fence = device
            .export(semaphore)
            .inspect_err(|error| {
                crate::log::warn_to_log!("no fence for the view's frame: {error}; it is not shown")
            })
            .ok()?;
        Some(SignalledFrame {
            submission,
            acquire_fence,
        })
    }
}

impl FenceDevice {
    /// A binary semaphore its payload can be exported from as a sync file.
    fn exportable_semaphore(&self) -> Option<vk::Semaphore> {
        let mut export = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        let info = vk::SemaphoreCreateInfo::default().push_next(&mut export);
        // SAFETY: a live device, opened with the extension a sync file's export needs.
        unsafe { self.raw_device.create_semaphore(&info, None) }
            .inspect_err(|error| {
                crate::log::warn_to_log!("no semaphore for the view's frame: {error}")
            })
            .ok()
    }

    /// Submits `commands` on the queue, the submission signalling `semaphore`, and answers its
    /// index; `None` where the submission did not reach the queue, the semaphore taken back.
    fn submit_with_signal(
        &self,
        semaphore: vk::Semaphore,
        commands: impl IntoIterator<Item = wgpu::CommandBuffer>,
    ) -> Option<wgpu::SubmissionIndex> {
        let commands: Vec<wgpu::CommandBuffer> = commands.into_iter().collect();
        // SAFETY: the queue is only read; the semaphore is live until its submission is done,
        // which `FrameSubmission` waits for, or until it is taken back unsubmitted below.
        let hal = unsafe { self.queue.as_hal::<Vulkan>() }?;
        hal.add_signal_semaphore(semaphore, None);
        let index = self.queue.submit(commands);
        let unsubmitted = hal.remove_signal_semaphore(semaphore);
        (!unsubmitted).then_some(index)
    }

    /// The sync file of `semaphore`'s pending signal; `None` where the GPU is done already, as
    /// Vulkan may answer for a semaphore signalled by then.
    fn export(&self, semaphore: vk::Semaphore) -> Result<Option<OwnedFd>, vk::Result> {
        let info = vk::SemaphoreGetFdInfoKHR::default()
            .semaphore(semaphore)
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        // SAFETY: a semaphore made exportable as a sync file, with its signal submitted.
        let fd = unsafe { self.semaphore_fd.get_semaphore_fd(&info) }?;
        // SAFETY: a descriptor Vulkan just handed over, which the caller owns, as the
        // extension says; -1 is none.
        Ok((fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd) }))
    }
}

impl FrameSubmission {
    /// Waits until the GPU has done the submission.
    fn wait(&self) {
        let done = self.device.device.poll(wgpu::PollType::Wait {
            submission_index: Some(self.index.clone()),
            timeout: None,
        });
        if let Err(error) = done {
            crate::log::warn_to_log!("the device did not finish a frame of the view: {error}");
        }
    }
}

impl Drop for FrameSubmission {
    /// Waits for the submission, which signals the semaphore, then destroys the semaphore.
    fn drop(&mut self) {
        self.wait();
        // SAFETY: the submission that signals it is done, and nothing else uses it.
        unsafe {
            self.device
                .raw_device
                .destroy_semaphore(self.semaphore, None)
        };
    }
}
