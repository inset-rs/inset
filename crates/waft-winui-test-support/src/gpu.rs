//! Headless GPU and PNG helpers copied from valo-harness (MIT); see ../VALO_LICENSE.
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use waft_embedder::valo::Context;

/// The one device the binary's tests render with, made at the first test that needs it.
static DEVICE: OnceLock<Option<(wgpu::Device, wgpu::Queue)>> = OnceLock::new();

/// The one renderer the binary's tests draw with.
///
/// Valo keeps pipelines, glyph rasters and uploaded images in a context, so tests that share it
/// pay for them once. The lock is also what keeps a software Vulkan driver to one drawing test
/// at a time; more than one device in flight crashed it.
static CONTEXT: OnceLock<Mutex<Context>> = OnceLock::new();

/// Runs `f` with the renderer the binary's tests share. `None` when the adapter offers no device.
pub(crate) fn with_context<T>(f: impl FnOnce(&mut Context) -> T) -> Option<T> {
    let (device, queue) = DEVICE.get_or_init(headless_device).as_ref()?;
    let context = CONTEXT.get_or_init(|| Mutex::new(Context::new(device.clone(), queue.clone())));
    let mut context = context
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Some(f(&mut context))
}

/// Creates the same headless device used by Valo's GPU tests.
pub fn headless_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok()?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("valo.harness"),
                // GPU timing when the adapter offers it (stats.gpu_ms).
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            })
            .await
            .ok()?;
        Some((device, queue))
    })
}

/// `render_to_rgba` draws `picture` into `size` pixels over `clear` and waits for them, as
/// valo-harness's helper does: the request, the device waited on, its answer. The rows are packed
/// RGBA8 with straight alpha.
pub fn render_to_rgba(
    context: &mut waft_embedder::valo::Context,
    picture: waft_embedder::valo::DisplayList,
    size: [u32; 2],
    clear: Option<waft_embedder::valo::Color>,
) -> Vec<u8> {
    let request = context.request_pixels(&std::sync::Arc::new(picture), size, clear);
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the test GPU device answers");
    pollster::block_on(request)
        .expect("the picture reads back")
        .into_bytes()
}

/// Writes a captured RGBA image.
pub fn write_png(path: &Path, size: [u32; 2], rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size[0], size[1]);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}
