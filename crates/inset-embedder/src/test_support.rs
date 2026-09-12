//! Images for tests, without a host.
//!
//! An [`Image`](crate::Image) only exists once pixels have been uploaded to a GPU, so a test of
//! anything above the host — a cache, a layout, a widget — would otherwise have no image to work
//! with. This makes one directly, on a device of its own that no window is attached to.
//!
//! Behind the `test-support` feature, so an application never builds it. A test crate asks for
//! it as a dev-dependency:
//!
//! ```toml
//! [dev-dependencies]
//! inset-embedder = { workspace = true, features = ["test-support"] }
//! ```

use std::sync::{Mutex, OnceLock};

use valo::{Context, ImageDesc};

use crate::Image;

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
    let mut context = GPU
        .get_or_init(|| Mutex::new(headless_context()))
        .lock()
        .expect("the test GPU device");
    context.upload_image(
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
