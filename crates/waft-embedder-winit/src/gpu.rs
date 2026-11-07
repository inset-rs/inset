//! The wgpu handles the embedder owns: valo requires the embedder to bring the device. One device
//! serves every view and every image, and it is made when something first needs it ([`LazyGpu`]).
//!
//! valo chooses the adapter (`valo::request_device`), high performance first: a fully
//! WebGPU-compliant one where the machine has one, and otherwise any adapter valo draws with. A
//! virtual machine ranks its GL driver above a compliant software adapter, lavapipe or WARP; the
//! GL driver has no compute and refuses WebGPU's limits, so the software adapter is taken.

use std::cell::OnceCell;
use std::rc::Rc;

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
    fn acquire() -> Gpu {
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

/// The one device every view draws with and every image belongs to, made when something first
/// needs it: a view's first draw (`rasterizer.rs`), or the first image the app decodes or brings
/// (`images.rs`). Clones share it.
#[derive(Clone, Default)]
pub(crate) struct LazyGpu(Rc<OnceCell<Gpu>>);

impl LazyGpu {
    /// The device, made now if nothing has needed it before.
    ///
    /// # Panics
    ///
    /// When no adapter gives a device, saying why each refused.
    pub(crate) fn get(&self) -> &Gpu {
        self.0.get_or_init(Gpu::acquire)
    }
}

/// A device on a fully WebGPU-compliant adapter, or, where none gives one, on any adapter valo
/// draws with.
async fn request_device(
    instance: &wgpu::Instance,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), valo::NoDevice> {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        // The device serves every window, and may be made before any has a surface: there is no
        // surface for the adapter to present to.
        compatible_surface: None,
        ..Default::default()
    };
    match valo::request_device(instance, &options, valo::FeatureLevel::Core).await {
        Ok(device) => Ok(device),
        Err(_) => valo::request_device(instance, &options, valo::FeatureLevel::Compatibility).await,
    }
}
