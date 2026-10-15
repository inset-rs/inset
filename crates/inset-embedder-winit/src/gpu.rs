//! The wgpu handles the embedder owns: valo requires the embedder to bring the device.
//!
//! valo chooses the adapter (`valo::request_device`), high performance first: a fully
//! WebGPU-compliant one where the machine has one, and otherwise any adapter valo draws with. A
//! virtual machine ranks its GL driver above a compliant software adapter, lavapipe or WARP; the
//! GL driver has no compute and refuses WebGPU's limits, so the software adapter is taken.

#[derive(Clone)]
pub(crate) struct Gpu {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl Gpu {
    /// The device every view draws with.
    ///
    /// # Panics
    ///
    /// When no adapter gives a device, saying why each refused.
    pub(crate) fn acquire() -> Gpu {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (adapter, device, queue) = pollster::block_on(request_device(&instance))
            .unwrap_or_else(|no_device| panic!("{no_device}"));
        Gpu {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

/// A device on a fully WebGPU-compliant adapter, or, where none gives one, on any adapter valo
/// draws with.
async fn request_device(
    instance: &wgpu::Instance,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), valo::NoDevice> {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        // The device comes with the first view, which is made before its window: there is no
        // surface yet for the adapter to present to.
        compatible_surface: None,
        ..Default::default()
    };
    match valo::request_device(instance, &options, valo::FeatureLevel::Core).await {
        Ok(device) => Ok(device),
        Err(_) => valo::request_device(instance, &options, valo::FeatureLevel::Compatibility).await,
    }
}
