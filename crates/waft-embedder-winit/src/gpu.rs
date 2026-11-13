//! The wgpu handles the embedder owns: valo requires the embedder to bring the device. One device
//! serves every view and every image, and it is made when something first needs it ([`LazyGpu`]).
//!
//! The adapter is a hardware one wherever the machine has one, at the best level valo draws at
//! on it, `Core` where it is fully WebGPU-compliant and `Compatibility` otherwise, before any
//! software adapter, which only an app that asks for it takes first (`force_fallback_adapter`,
//! WebGPU's `forceFallbackAdapter`). Waft draws with neither compute nor storage buffers, so a
//! downlevel GPU, such as a virtual machine's GL driver, serves it better than lavapipe or WARP. A
//! software adapter is one wgpu reports as `DeviceType::Cpu`, as Vulkan's and EGL's drivers say of
//! lavapipe and llvmpipe and D3D12's of WARP; valo ranks those last itself, and keeps only them
//! where `RequestAdapterOptions::force_fallback_adapter` is set.
//!
//! The instance is made with the event loop's display, as wgpu asks for GL on Wayland and X11:
//! without it wgpu's EGL is on its surfaceless platform, whose GL adapter cannot present to a
//! window (`../experiments/2026-11-13-wgpu-gl-in-vm`).

use std::cell::OnceCell;
use std::rc::Rc;

use winit::event_loop::OwnedDisplayHandle;

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
    fn acquire(request: &GpuRequest) -> Gpu {
        let instance = wgpu::Instance::new(instance_descriptor(request.display.clone()));
        let (adapter, device, queue) =
            pollster::block_on(request_device(&instance, request.force_fallback_adapter))
                .unwrap_or_else(|no_device| panic!("{no_device}"));
        Gpu {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

/// What the instance is made with: the event loop's display where there is one.
fn instance_descriptor(display: Option<OwnedDisplayHandle>) -> wgpu::InstanceDescriptor {
    match display {
        Some(display) => wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display)),
        None => wgpu::InstanceDescriptor::new_without_display_handle(),
    }
}

/// How the device is asked for: on an instance that knows the event loop's display, and on the
/// system's software adapter only where the app forces it.
#[derive(Clone, Default)]
pub(crate) struct GpuRequest {
    pub(crate) display: Option<OwnedDisplayHandle>,
    pub(crate) force_fallback_adapter: bool,
}

/// The one device every view draws with and every image belongs to, made when something first
/// needs it: a view's first draw (`rasterizer.rs`), or the first image the app decodes or brings
/// (`images.rs`), as `request` asks. Clones share it.
#[derive(Clone, Default)]
pub(crate) struct LazyGpu {
    gpu: Rc<OnceCell<Gpu>>,
    request: GpuRequest,
}

impl LazyGpu {
    /// The device to be made as `request` asks.
    pub(crate) fn new(request: GpuRequest) -> LazyGpu {
        LazyGpu {
            gpu: Rc::default(),
            request,
        }
    }

    /// The device, made now if nothing has needed it before.
    ///
    /// # Panics
    ///
    /// When no adapter gives a device, saying why each refused.
    pub(crate) fn get(&self) -> &Gpu {
        self.gpu.get_or_init(|| Gpu::acquire(&self.request))
    }
}

/// A device on a hardware adapter at the best level valo draws at on it, `Core` where one is fully
/// WebGPU-compliant; where there is none, on the software adapter; and with
/// `force_fallback_adapter`, on the software adapter alone.
async fn request_device(
    instance: &wgpu::Instance,
    force_fallback_adapter: bool,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), valo::NoDevice> {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter,
        // The device serves every window, and may be made before any has a surface: there is no
        // surface for the adapter to present to.
        compatible_surface: None,
        ..Default::default()
    };
    let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
    let level = best_level(&adapters, force_fallback_adapter);
    match valo::request_device(instance, &options, level).await {
        Ok(device) => Ok(device),
        Err(_) if level == valo::FeatureLevel::Core => {
            valo::request_device(instance, &options, valo::FeatureLevel::Compatibility).await
        }
        Err(no_device) => Err(no_device),
    }
}

/// The level to ask valo for: `Core` where an adapter of the kind wanted, hardware or, forced, the
/// software one, is fully WebGPU-compliant; `Compatibility` otherwise, at which valo still ranks
/// hardware adapters before software ones.
fn best_level(adapters: &[wgpu::Adapter], force_fallback_adapter: bool) -> valo::FeatureLevel {
    let wanted =
        |adapter: &&wgpu::Adapter| is_software(&adapter.get_info()) == force_fallback_adapter;
    if adapters
        .iter()
        .filter(wanted)
        .any(|adapter| adapter.get_downlevel_capabilities().is_webgpu_compliant())
    {
        valo::FeatureLevel::Core
    } else {
        valo::FeatureLevel::Compatibility
    }
}

/// Whether `adapter` draws on the CPU: lavapipe, llvmpipe, WARP.
fn is_software(adapter: &wgpu::AdapterInfo) -> bool {
    adapter.device_type == wgpu::DeviceType::Cpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instance_without_an_event_loop_has_no_display() {
        assert!(instance_descriptor(None).display.is_none());
    }

    #[test]
    fn a_software_adapter_is_one_that_draws_on_the_cpu() {
        let info = |device_type| wgpu::AdapterInfo::new(device_type, wgpu::Backend::Vulkan);
        assert!(is_software(&info(wgpu::DeviceType::Cpu)));
        assert!(!is_software(&info(wgpu::DeviceType::VirtualGpu)));
        assert!(!is_software(&info(wgpu::DeviceType::Other)));
    }
}
