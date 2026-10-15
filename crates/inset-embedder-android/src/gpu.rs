//! The wgpu handles the host owns: valo requires the host to bring the device.
//!
//! The adapter comes from Vulkan, and from OpenGL ES only where Vulkan gives no device: wgpu
//! ranks adapters by device type across backends, which on an emulator puts the GLES translator
//! ahead of the Vulkan device, and GLES is the lesser host — no compute, and a surface valo
//! cannot copy for its blend snapshots. One instance per attempt, over its backends alone,
//! because wgpu makes a window's surface on every backend its instance carries and Android
//! connects a window to one graphics API at a time.

/// The device every view draws with.
#[derive(Clone)]
pub(crate) struct Gpu {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl Gpu {
    /// The device on Vulkan, or on OpenGL ES where Vulkan gives none.
    ///
    /// # Panics
    ///
    /// When neither Vulkan nor OpenGL ES gives a device, saying why each adapter refused.
    pub(crate) fn acquire() -> Gpu {
        pollster::block_on(async {
            match Gpu::on(wgpu::Backends::VULKAN).await {
                Ok(gpu) => gpu,
                Err(no_vulkan) => Gpu::on(wgpu::Backends::GL)
                    .await
                    .unwrap_or_else(|no_gles| panic!("Vulkan: {no_vulkan}; OpenGL ES: {no_gles}")),
            }
        })
    }

    /// A device on `backends` alone, on any adapter valo draws with (`valo::request_device`)
    /// rather than on WebGPU-compliant ones only: an emulator's GLES reports no compute at all.
    async fn on(backends: wgpu::Backends) -> Result<Gpu, valo::NoDevice> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(descriptor);
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            // The device is acquired before the system gives the activity a window.
            compatible_surface: None,
            ..Default::default()
        };
        let (adapter, device, queue) =
            valo::request_device(&instance, &options, valo::FeatureLevel::Compatibility).await?;
        Ok(Gpu {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
