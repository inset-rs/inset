//! Images, surfaces and drawn pixels for tests, without a host.
//!
//! An [`Image`](crate::Image) only exists once pixels have been uploaded to a GPU, so a test of
//! anything above the host — a cache, a layout, a widget — would otherwise have no image to work
//! with. This makes one directly, on a device of its own that no window is attached to, and
//! draws a [`Picture`](crate::Picture) there to read back what it shows. A
//! [`Surface`](crate::Surface) only exists where a host composites, so this makes one nobody
//! feeds.
//!
//! Behind the `test-support` feature, so an application never builds it. A test crate asks for
//! it as a dev-dependency:
//!
//! ```toml
//! [dev-dependencies]
//! inset-embedder = { workspace = true, features = ["test-support"] }
//! ```

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use valo::{Context, ImageDesc};

use crate::{Buffer, Image, Picture, Surface, SurfacePlacement};

/// One device for the whole test process, created on first use.
///
/// Shared rather than made per test: a device costs driver set-up that dwarfs anything a test
/// does with it. It also never runs a destructor, which matters — a graphics device torn down
/// while the thread it belongs to is already unwinding its own thread-locals aborts the process.
static GPU: OnceLock<Mutex<Context>> = OnceLock::new();

/// A solid image of `size`, every pixel `color` as straight red, green, blue and alpha.
///
/// # Panics
///
/// Panics if no GPU adapter is available, which on a machine that can run the renderer at all
/// means the test environment has no graphics device.
pub fn solid_image(size: [u32; 2], color: [u8; 4]) -> Image {
    let [width, height] = size;
    let pixels: Vec<u8> = color
        .iter()
        .copied()
        .cycle()
        .take(width as usize * height as usize * 4)
        .collect();
    image_from_pixels(size, &pixels)
}

/// An image of `size` from straight-alpha RGBA samples, four bytes a pixel.
///
/// # Panics
///
/// Panics if `pixels` is not exactly four bytes per pixel, or if no GPU adapter is available.
pub fn image_from_pixels(size: [u32; 2], pixels: &[u8]) -> Image {
    let [width, height] = size;
    assert_eq!(
        pixels.len(),
        width as usize * height as usize * 4,
        "an image needs four bytes a pixel"
    );
    gpu().upload_image(
        ImageDesc {
            size,
            premultiplied: false,
            // No mips: a test never draws these small enough to need them, and building a chain
            // would only slow every image a test makes.
            mips: false,
        },
        pixels,
    )
}

/// What `picture` shows drawn over a transparent area of `size`, as straight-alpha RGBA
/// samples, four bytes a pixel and row after row.
///
/// # Panics
///
/// Panics if no GPU adapter is available.
pub fn drawn_pixels(picture: &Picture, size: [u32; 2]) -> Vec<u8> {
    gpu().render_to_rgba(picture, size, Some(valo::Color::TRANSPARENT))
}

fn gpu() -> MutexGuard<'static, Context> {
    GPU.get_or_init(|| Mutex::new(headless_context()))
        .lock()
        .expect("the test GPU device")
}

fn headless_context() -> Context {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("a GPU adapter for tests");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("a GPU device for tests");
    let mut context = Context::new(device, queue);
    context.set_hide_missing_glyphs(true);
    context
}

/// A surface no producer feeds, as the handle a compositor hands out. Enough to place one in
/// a scene and compare the placement.
pub fn idle_surface() -> Arc<dyn Surface> {
    Arc::new(IdleSurface)
}

/// What [`idle_surface`] hands out: presents go nowhere, and children are idle too.
struct IdleSurface;

impl Surface for IdleSurface {
    fn present(
        &self,
        _buffer: Buffer,
        _children: &[SurfacePlacement],
        _released: Box<dyn FnOnce() + Send + Sync>,
    ) {
    }

    fn open_child(&self, _opaque: bool) -> Arc<dyn Surface> {
        idle_surface()
    }
}
