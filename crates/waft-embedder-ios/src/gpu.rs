//! The one device every view draws with and every image belongs to: Metal's, made as the app
//! launches.

#[derive(Clone)]
pub(crate) struct Gpu {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl Gpu {
    /// The device every view draws with: valo's full feature level, or its compatibility level on
    /// a GPU that lacks it.
    ///
    /// # Panics
    ///
    /// Where Metal gives no device, saying why.
    pub(crate) fn acquire() -> Gpu {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            ..Default::default()
        };
        let (adapter, device, queue) = pollster::block_on(async {
            match valo::request_device(&instance, &options, valo::FeatureLevel::Core).await {
                Ok(device) => Ok(device),
                Err(_) => {
                    valo::request_device(&instance, &options, valo::FeatureLevel::Compatibility)
                        .await
                }
            }
        })
        .unwrap_or_else(|no_device| panic!("{no_device}"));
        Gpu {
            instance,
            adapter,
            device,
            queue,
        }
    }
}
