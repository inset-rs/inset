//! The view's own picture as a composition layer: drawn into hardware buffers of its own through
//! textures imported on Waft's device, and shown as the content of a layer placed filling a
//! surface control made from the window, the composition layers the view's scene places beneath
//! it as the layer's children. A frame is one transaction: the picture's buffer, the layer filling
//! the window at its size, and every placement beneath it, so the picture and its layers land
//! together, where a swapchain on the window could not join the transaction its layers are placed
//! in. The window's own surface holds only a buffer of the clear colour at the window's size,
//! which Android sizes the window and its input by (`window_surface`).
//!
//! The view keeps a few buffers at the window's size, each drawn again once the compositor has
//! released it: one replaced comes back about a refresh after the present that replaced it, so
//! three at 60 Hz are rarely all held (`experiments/2026-11-11-android-surface-control`,
//! question 5). A frame goes, in order on wgpu's queue: the buffer's image acquired back from the
//! compositor's queue family where the compositor had it, the picture drawn, the image released
//! to that family (`queue_family_handover`); then the loop's thread waits for the GPU, since
//! `present` carries no fence (about 3 ms for a clear on the emulator and on a Pixel, a known
//! gap), and the transaction presents the buffer.
//!
//! The invariant: a buffer is drawn into only after its release has run and its image has been
//! acquired back, and is presented only once its image has been released; and the layer is placed
//! in the window that is there, which goes and comes back with the activity's, and whose own
//! surface holds a buffer of its size.

use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use ndk::native_window::NativeWindow;
use waft_embedder::{HardwareBufferAllocator, HardwareBufferUsage, Picture, Slices};

use crate::gpu::Gpu;
use crate::hardware_buffers::{AndroidHardwareBuffer, GrallocAllocator};
use crate::queue_family_handover;
use crate::surface_control::SurfaceControl;

use super::window_surface::WindowSurface;
use super::{SurfaceControlCompositor, SurfaceControlLayer, layer_geometry, own_layers};

/// How many buffers the view draws into in turn.
const BUFFERS: usize = 3;

/// How long a draw waits for one of the buffers to come back before it is refused, to be drawn
/// again at the next refresh: longer than a frame held a refresh and a half, the most the
/// experiment saw.
const PATIENCE: Duration = Duration::from_millis(100);

/// What the view's buffers are allocated for: drawn into and sampled by the GPU, and shown by the
/// display as an overlay.
const DRAWN_AND_SHOWN: HardwareBufferUsage = HardwareBufferUsage {
    gpu_sampled: true,
    gpu_color_output: true,
    composer_overlay: true,
    cpu_read: false,
};

const WINDOW_NAME: &CStr = c"waft view";

/// The view's composition layer, the window it is placed in, and the buffers its picture is drawn
/// into.
pub(crate) struct SurfaceControlViewLayer {
    layer: Arc<SurfaceControlLayer>,
    /// The window, while there is one.
    window: Option<ViewWindow>,
    /// The window's size, in pixels.
    size: [u32; 2],
    buffers: Option<ViewBuffers>,
    gpu: Gpu,
    context: valo::Context,
}

impl SurfaceControlViewLayer {
    /// The view's layer, made opaque on `compositor`, drawn with `context` on `gpu`'s device,
    /// which imports hardware buffers.
    pub(crate) fn new(
        compositor: &SurfaceControlCompositor,
        gpu: &Gpu,
        context: valo::Context,
    ) -> SurfaceControlViewLayer {
        SurfaceControlViewLayer {
            layer: compositor.layer(true),
            window: None,
            size: [0, 0],
            buffers: None,
            gpu: gpu.clone(),
            context,
        }
    }

    /// The system gave the activity `window`, `size` pixels large: the layer is placed in it at
    /// the next draw. `Err` where the system makes no surface control of it.
    pub(crate) fn window_arrived(
        &mut self,
        window: &NativeWindow,
        size: [u32; 2],
    ) -> Result<(), String> {
        let control = SurfaceControl::from_window(window, WINDOW_NAME)
            .ok_or("the system made no surface control of the window")?;
        self.window = Some(ViewWindow {
            control,
            own_surface: WindowSurface::new(window),
        });
        self.size = size;
        Ok(())
    }

    /// The system takes the window away: the layer leaves it, with every layer beneath it.
    pub(crate) fn window_gone(&mut self) {
        let Some(window) = self.window.take() else {
            return;
        };
        self.layer
            .transactions
            .transact(|transaction| self.layer.let_go(transaction));
        drop(window);
    }

    /// The window is `size` pixels large from now on.
    pub(crate) fn resized(&mut self, size: [u32; 2]) {
        self.size = size;
    }

    /// Draws `slices` over `clear` into a free buffer, and shows it filling the window with the
    /// composition layers the slices place beneath it, laid out at `pixel_ratio`, in one
    /// transaction; the window's own surface is filled with `clear` first where its size is new.
    /// Refused as lost while there is no window or no buffer can be made, and as timed out while
    /// every buffer is still the compositor's.
    pub(crate) fn draw(
        &mut self,
        slices: &Slices,
        clear: valo::Color,
        pixel_ratio: f64,
    ) -> Result<(), valo::Refused> {
        let Some(window) = &mut self.window else {
            return Err(valo::Refused::Lost);
        };
        let size = self.size;
        window.own_surface.fill(size, clear);
        let buffers = match &mut self.buffers {
            Some(buffers) if buffers.size == size => buffers,
            buffers => {
                buffers.insert(ViewBuffers::new(&self.gpu, size).ok_or(valo::Refused::Lost)?)
            }
        };
        let slot = buffers.take_free()?;
        slot.drawn.take_back_from_compositor(&self.gpu);
        render(
            &mut self.context,
            &slices.picture_with_room(),
            &slot.drawn,
            clear,
        );
        slot.drawn.hand_to_compositor(&self.gpu);
        wait_for_gpu(&self.gpu);
        let children = own_layers(
            &slices
                .composition_layers()
                .map(|(layer, placement)| (Arc::clone(layer), placement.clone()))
                .collect::<Vec<_>>(),
        );
        let released = buffers.release_of(slot.index);
        let layer = &self.layer;
        layer.transactions.transact(|transaction| {
            if layer.adopted_by(super::Placer::Window) {
                let filling = Some(layer_geometry::filling(size));
                layer.show_in(transaction, &window.control, 0, filling, pixel_ratio);
            }
            layer.present_in(transaction, slot.buffer, children, released);
        });
        Ok(())
    }
}

/// The window the view's layer is placed in: the surface control made from it, and its own surface.
struct ViewWindow {
    control: SurfaceControl,
    own_surface: WindowSurface,
}

/// Renders `picture` over `clear` into `texture`.
fn render(
    context: &mut valo::Context,
    picture: &Picture,
    texture: &DrawnTexture,
    clear: valo::Color,
) {
    let size = [texture.texture.width(), texture.texture.height()];
    let target = valo::RenderTarget {
        view: &texture.view,
        texture: &texture.texture,
        format: texture.texture.format(),
        size,
        clear: Some(clear),
    };
    context.render(picture, &target);
}

/// Waits for the GPU to finish the frame and its handover: the compositor is handed the buffer
/// with no fence to wait on.
fn wait_for_gpu(gpu: &Gpu) {
    if let Err(error) = gpu.device.poll(wgpu::PollType::wait_indefinitely()) {
        crate::log::warn_to_log!("the device did not finish the view's frame: {error}");
    }
}

/// A buffer's texture on Waft's device, the view the picture is drawn through, and whether its
/// image was handed to the compositor and not yet taken back.
struct DrawnTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    with_compositor: AtomicBool,
}

impl DrawnTexture {
    /// Releases the image to the compositor, after the frame's work submitted before this.
    fn hand_to_compositor(&self, gpu: &Gpu) {
        queue_family_handover::release_to_compositor(&gpu.device, &gpu.queue, &self.texture);
        self.with_compositor.store(true, Ordering::Relaxed);
    }

    /// Acquires the image back from the compositor, which released its buffer, before the work
    /// submitted after this; a new texture was never handed over, and is not.
    fn take_back_from_compositor(&self, gpu: &Gpu) {
        if self.with_compositor.swap(false, Ordering::Relaxed) {
            queue_family_handover::acquire_from_compositor(&gpu.device, &gpu.queue, &self.texture);
        }
    }
}

/// The buffers the view draws into, of one size, and which of them the compositor has released.
struct ViewBuffers {
    size: [u32; 2],
    buffers: Vec<(AndroidHardwareBuffer, Arc<DrawnTexture>)>,
    free: Arc<FreeBuffers>,
}

/// Which buffers are free to draw into, and the condition a draw waits on for one.
struct FreeBuffers {
    free: Mutex<Vec<bool>>,
    released: Condvar,
}

/// A free buffer, taken to be drawn into and presented.
struct Slot {
    index: usize,
    buffer: AndroidHardwareBuffer,
    drawn: Arc<DrawnTexture>,
}

impl ViewBuffers {
    /// Buffers of `size`, allocated and imported on `gpu`'s device; `None` where the system
    /// allocates or imports none.
    fn new(gpu: &Gpu, size: [u32; 2]) -> Option<ViewBuffers> {
        let allocator = GrallocAllocator {
            largest_side: gpu.device.limits().max_texture_dimension_2d,
        };
        let buffers = (0..BUFFERS)
            .map(|_| {
                let allocated = allocator
                    .allocate(
                        size[0],
                        size[1],
                        waft_embedder::image_codecs::PixelFormat::Rgba8,
                        DRAWN_AND_SHOWN,
                    )
                    .inspect_err(|error| {
                        crate::log::warn_to_log!("no buffer for the view: {error}")
                    })
                    .ok()?;
                let texture = crate::gpu_interop::import_texture(&*allocated, &gpu.device)?;
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let buffer = (&*allocated as &dyn std::any::Any)
                    .downcast_ref::<AndroidHardwareBuffer>()?
                    .clone();
                let drawn = DrawnTexture {
                    texture,
                    view,
                    with_compositor: AtomicBool::new(false),
                };
                Some((buffer, Arc::new(drawn)))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(ViewBuffers {
            size,
            buffers,
            free: Arc::new(FreeBuffers {
                free: Mutex::new(vec![true; BUFFERS]),
                released: Condvar::new(),
            }),
        })
    }

    /// A free buffer, taken, waiting a while for the compositor to release one; refused as timed
    /// out where none comes back in time.
    fn take_free(&self) -> Result<Slot, valo::Refused> {
        let free = self
            .free
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (mut free, _) = self
            .free
            .released
            .wait_timeout_while(free, PATIENCE, |free| !free.contains(&true))
            .unwrap_or_else(PoisonError::into_inner);
        let index = free
            .iter()
            .position(|free| *free)
            .ok_or(valo::Refused::Timeout)?;
        free[index] = false;
        let (buffer, drawn) = &self.buffers[index];
        Ok(Slot {
            index,
            buffer: buffer.clone(),
            drawn: Arc::clone(drawn),
        })
    }

    /// What frees buffer `index` once the compositor releases it.
    fn release_of(&self, index: usize) -> Box<dyn FnOnce() + Send + Sync> {
        let free = Arc::clone(&self.free);
        Box::new(move || {
            free.free.lock().unwrap_or_else(PoisonError::into_inner)[index] = true;
            free.released.notify_all();
        })
    }
}
