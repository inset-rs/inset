//! The wgpu handles the host owns: valo requires the host to bring the device.
//!
//! The adapter comes from Vulkan, and from OpenGL ES only where there is no Vulkan one:
//! wgpu ranks adapters by device type across backends, which on an emulator puts the GLES
//! translator ahead of the Vulkan device, and GLES is the lesser host — no compute, and a
//! surface valo cannot copy for its blend snapshots. One instance per attempt, over its
//! backends alone, because wgpu makes a window's surface on every backend its instance
//! carries and Android connects a window to one graphics API at a time.

/// The device every view draws with.
#[derive(Clone)]
pub(crate) struct Gpu {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl Gpu {
    pub(crate) fn acquire() -> Gpu {
        pollster::block_on(async {
            let (instance, adapter) = match adapter_on(wgpu::Backends::VULKAN).await {
                Some(vulkan) => vulkan,
                None => adapter_on(wgpu::Backends::GL)
                    .await
                    .expect("request wgpu adapter"),
            };
            // Primary buffers the host writes to directly, where the memory is unified:
            // valo then uploads its draws without a copy, which a phone's memory allows.
            let required_features = adapter.features() & wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("inset.android"),
                    required_features,
                    // What this adapter offers, rather than WebGPU's defaults: valo's
                    // passes ask for nothing beyond them, and an adapter below them still
                    // draws them. An emulator's GLES reports no compute at all.
                    required_limits: adapter.limits(),
                    ..Default::default()
                })
                .await
                .expect("request wgpu device");
            Gpu {
                instance,
                adapter,
                device,
                queue,
            }
        })
    }
}

async fn adapter_on(backends: wgpu::Backends) -> Option<(wgpu::Instance, wgpu::Adapter)> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .ok()?;
    Some((instance, adapter))
}
