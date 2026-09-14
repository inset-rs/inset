/// The wgpu handles the embedder owns: valo requires the host to bring the
/// device.
#[derive(Clone)]
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub fn acquire() -> Gpu {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                })
                .await
                .expect("request wgpu adapter");
            // Primary buffers that the host can write to directly, where memory is
            // unified: valo then uploads its draws without a copy.
            let required_features = adapter.features() & wgpu::Features::MAPPABLE_PRIMARY_BUFFERS;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("inset.winit"),
                    required_features,
                    required_limits: limits_within(&adapter.limits()),
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

/// wgpu's default limits, lowered where this adapter offers less. The iOS
/// simulator's Metal allows 15 inter-stage shader variables against the
/// default request of 16; valo's shaders use two.
fn limits_within(supported: &wgpu::Limits) -> wgpu::Limits {
    let mut limits = wgpu::Limits::default();
    limits.max_inter_stage_shader_variables = limits
        .max_inter_stage_shader_variables
        .min(supported.max_inter_stage_shader_variables);
    limits
}
