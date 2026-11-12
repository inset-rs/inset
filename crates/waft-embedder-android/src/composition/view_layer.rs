//! The view's own picture as a composition layer: drawn into hardware buffers of its own through
//! textures imported on Waft's device, and shown as the content of a layer placed filling a
//! surface control made from the window, the composition layers the view's scene places beneath
//! it as the layer's children. A frame is one transaction: the picture's buffer, the layer filling
//! the window at its size, and every placement beneath it, so the picture and its layers land
//! together, where a swapchain on the window could not join the transaction its layers are placed
//! in. The window's own surface holds only a buffer of the clear colour at the window's size,
//! which Android sizes the window and its input by (`window_surface`).
//!
//! The view's buffers, at the window's size, are a pool as Impeller's hardware-buffer swapchain
//! keeps one (`AHBTexturePoolVK`): a frame takes the buffer the compositor released first, or a
//! new one where none is free, and a buffer goes back to the pool once the compositor has released
//! it, about a refresh after the present that replaced it
//! (`experiments/2026-11-11-android-surface-control`, question 5). What bounds the pool is the
//! frames in flight: before a frame is drawn, the loop waits for the frame two before it to be
//! done on the GPU (`frames_in_flight`), so it holds the buffers of at most two pending frames
//! beside those the compositor holds.
//!
//! A frame goes, in order on wgpu's queue: the buffer's image acquired back from the compositor's
//! queue family where the compositor had it, the picture drawn, and the image released to that
//! family (`queue_family_handover`) as the frame's last submission, which signals the fence the
//! transaction hands the compositor with the buffer (`frame_fence`). No thread waits for the
//! frame's GPU work: SurfaceFlinger waits on the fence before it reads the buffer.
//!
//! The view holds buffers only while it has a window. When the window goes, so do the pool and the
//! layer, with its surface controls and the buffers presented to it, which the compositor would
//! otherwise keep, a screen's worth for each view in the background; the layers the scene placed
//! beneath it are let go, to be placed again. A window that arrives is given a new layer, and the
//! next draw makes buffers of its size.
//!
//! The invariant: a buffer is drawn into only after its release has run and its image has been
//! acquired back, and is presented only with the fence of the submission that released its image;
//! and the layer is placed in the window that is there, which goes and comes back with the
//! activity's, and whose own surface holds a buffer of its size. A later editor must not keep the
//! pool or the layer across a window's going: nothing bounds the views in the background.

use std::collections::VecDeque;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ndk::native_window::NativeWindow;
use waft_embedder::{HardwareBufferAllocator, HardwareBufferUsage, Picture, Slices, ViewId};

use crate::frame_fence::{FrameFences, SignalledFrame};
use crate::gpu::Gpu;
use crate::hardware_buffers::{AndroidHardwareBuffer, GrallocAllocator};
use crate::queue_family_handover;
use crate::surface_control::SurfaceControl;

use super::frames_in_flight::FramesInFlight;
use super::releases::OnReleased;
use super::window_surface::WindowSurface;
use super::{SurfaceControlCompositor, SurfaceControlLayer, layer_geometry, own_layers};

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
    /// The view, which the layer's placement in a window names.
    view: ViewId,
    layer: Arc<SurfaceControlLayer>,
    /// The window, while there is one.
    window: Option<ViewWindow>,
    /// The window's size, in pixels.
    size: [u32; 2],
    buffers: Option<ViewBuffers>,
    fences: FrameFences,
    in_flight: FramesInFlight,
    gpu: Gpu,
    context: valo::Context,
}

impl SurfaceControlViewLayer {
    /// The layer of the view numbered `view`, made opaque on `compositor`, drawn with `context`
    /// on `gpu`'s device, which imports hardware buffers.
    ///
    /// # Panics
    ///
    /// Where the device is not Vulkan's, which one that imports hardware buffers always is
    /// (`vulkan_imports`).
    pub(crate) fn new(
        view: ViewId,
        compositor: &SurfaceControlCompositor,
        gpu: &Gpu,
        context: valo::Context,
    ) -> SurfaceControlViewLayer {
        SurfaceControlViewLayer {
            view,
            layer: compositor.layer(true),
            window: None,
            size: [0, 0],
            buffers: None,
            fences: FrameFences::new(&gpu.device, &gpu.queue)
                .expect("a device that imports hardware buffers is Vulkan's"),
            in_flight: FramesInFlight::new(),
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

    /// The system takes the window away: the layer goes, off screen with its surface controls
    /// and the buffers presented to it, every layer beneath it let go, and the view's buffers go.
    /// A layer with nothing presented stands in for the window that comes back.
    pub(crate) fn window_gone(&mut self) {
        let Some(window) = self.window.take() else {
            return;
        };
        self.layer = self.layer.successor();
        self.buffers = None;
        drop(window);
    }

    /// The window is `size` pixels large from now on.
    pub(crate) fn resized(&mut self, size: [u32; 2]) {
        self.size = size;
    }

    /// Draws `slices` over `clear` into a free buffer, and shows it filling the window with the
    /// composition layers the slices place beneath it, laid out at `pixel_ratio`, in one
    /// transaction, the buffer with its frame's fence; the window's own surface is filled with
    /// `clear` first where its size is new. Waits first while two frames are pending on the GPU.
    /// Refused as lost while there is no window, where no buffer can be made, and where the frame
    /// gets no fence.
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
            buffers => buffers.insert(ViewBuffers::new(size)),
        };
        self.in_flight.wait_for_turn();
        let slot = buffers.take_free(&self.gpu).ok_or(valo::Refused::Lost)?;
        slot.drawn.take_back_from_compositor(&self.gpu);
        render(
            &mut self.context,
            &slices.picture_with_room(),
            &slot.drawn,
            clear,
        );
        let Some(frame) = slot.drawn.hand_to_compositor(&self.gpu, &self.fences) else {
            buffers.give_back(slot.index);
            return Err(valo::Refused::Lost);
        };
        self.in_flight.submitted(frame.submission);
        let children = own_layers(
            &slices
                .composition_layers()
                .map(|(layer, placement)| (Arc::clone(layer), placement.clone()))
                .collect::<Vec<_>>(),
        );
        let released = buffers.release_of(slot.index);
        let layer = &self.layer;
        layer.transactions.transact(|transaction| {
            if layer.adopted_by(super::Placer::Window(self.view)) {
                let filling = Some(layer_geometry::filling(size));
                layer.show_in(transaction, &window.control, 0, filling, pixel_ratio);
            }
            layer.present_in(
                transaction,
                slot.buffer,
                frame.acquire_fence,
                children,
                released,
            );
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

/// A buffer's texture on Waft's device, the view the picture is drawn through, and whether its
/// image was handed to the compositor and not yet taken back.
struct DrawnTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    with_compositor: AtomicBool,
}

impl DrawnTexture {
    /// Releases the image to the compositor as the frame's last submission, after the frame's
    /// work submitted before this, and answers it with the fence it signals; `None`, reported,
    /// where it was not submitted or gave no fence.
    fn hand_to_compositor(&self, gpu: &Gpu, fences: &FrameFences) -> Option<SignalledFrame> {
        let release = queue_family_handover::release_to_compositor(&gpu.device, &self.texture)?;
        self.with_compositor.store(true, Ordering::Relaxed);
        fences.submit_signalling(release)
    }

    /// Acquires the image back from the compositor, which released its buffer, before the work
    /// submitted after this; a new texture was never handed over, and is not.
    fn take_back_from_compositor(&self, gpu: &Gpu) {
        if self.with_compositor.swap(false, Ordering::Relaxed) {
            queue_family_handover::acquire_from_compositor(&gpu.device, &gpu.queue, &self.texture);
        }
    }
}

/// The buffers the view draws into, of one size, made as frames need them, and which of them the
/// compositor has released, in the order it released them.
struct ViewBuffers {
    size: [u32; 2],
    buffers: Vec<(AndroidHardwareBuffer, Arc<DrawnTexture>)>,
    free: Arc<Mutex<VecDeque<usize>>>,
}

/// A free buffer, taken to be drawn into and presented.
struct Slot {
    index: usize,
    buffer: AndroidHardwareBuffer,
    drawn: Arc<DrawnTexture>,
}

impl ViewBuffers {
    /// No buffers yet, of `size`.
    fn new(size: [u32; 2]) -> ViewBuffers {
        ViewBuffers {
            size,
            buffers: Vec::new(),
            free: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// The buffer the compositor released first, taken, or a new one where none is free, as
    /// Impeller's pool pops; `None` where the system allocates or imports none.
    fn take_free(&mut self, gpu: &Gpu) -> Option<Slot> {
        let released = self
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();
        let index = match released {
            Some(index) => index,
            None => self.add(gpu)?,
        };
        let (buffer, drawn) = &self.buffers[index];
        Some(Slot {
            index,
            buffer: buffer.clone(),
            drawn: Arc::clone(drawn),
        })
    }

    /// Buffer `index`, taken and not presented, is free again.
    fn give_back(&self, index: usize) {
        free_buffer(&self.free, index);
    }

    /// What frees buffer `index` once the compositor releases it.
    fn release_of(&self, index: usize) -> OnReleased {
        let free = Arc::clone(&self.free);
        Box::new(move || free_buffer(&free, index))
    }

    /// A new buffer, allocated and imported on `gpu`'s device, and its index.
    fn add(&mut self, gpu: &Gpu) -> Option<usize> {
        let buffer = drawn_buffer(gpu, self.size)?;
        self.buffers.push(buffer);
        Some(self.buffers.len() - 1)
    }
}

/// Puts buffer `index` at the back of `free`, to be taken after those released before it.
fn free_buffer(free: &Mutex<VecDeque<usize>>, index: usize) {
    free.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back(index);
}

/// A buffer of `size`, allocated to be drawn into and shown, and its texture on `gpu`'s device;
/// `None`, reported where the system allocates none, where it imports none.
fn drawn_buffer(gpu: &Gpu, size: [u32; 2]) -> Option<(AndroidHardwareBuffer, Arc<DrawnTexture>)> {
    let allocator = GrallocAllocator {
        largest_side: gpu.device.limits().max_texture_dimension_2d,
    };
    let allocated = allocator
        .allocate(
            size[0],
            size[1],
            waft_embedder::image_codecs::PixelFormat::Rgba8,
            DRAWN_AND_SHOWN,
        )
        .inspect_err(|error| crate::log::warn_to_log!("no buffer for the view: {error}"))
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
}
