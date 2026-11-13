//! Composition layers on Windows, through DirectComposition on every Windows that composes, as
//! Chromium shows a video where it cannot hand DirectComposition the decoder's texture: a layer is
//! a visual holding its children's visuals, then on top a content visual whose content is a
//! composition swapchain of the layer's own, a wgpu surface on a device of the compositor's own on
//! Waft's adapter. A present copies the producer's buffer, a texture shared by NT handle
//! (`shared_textures`), into the swapchain's next image on that device's queue and presents it;
//! the buffer goes back to its producer once the copy is done on the GPU, which a thread of the
//! compositor waits for. A window is a DirectComposition
//! target whose root visual holds the layers its view's scenes place, then on top the visual of
//! the view's own picture, wgpu's composition swapchain (`WindowComposition::make_surface`): a
//! window shows its own content beneath every visual, so the picture is a visual itself, and the
//! window has no redirection surface (`with_no_redirection_bitmap`).
//!
//! A scene's placements are committed right before the view's picture presents. The two are not
//! one step: DirectComposition applies a commit at its next frame, and a swapchain's present
//! shows a frame later, so a layer that moves or resizes is a frame away from the room the
//! picture makes for it, a sliver while it moves (`experiments/2026-11-13-windows-composition`,
//! row 2), as Chromium accepts between its root swapchain and its overlays. Composition textures
//! (Windows 11 24H2) would show the producer's texture itself in the same commit as the
//! placements, with no copy and no sliver.
//!
//! The invariants a later editor must keep: every call to DirectComposition, wgpu's configuring a
//! surface on a visual included, is made under the device's lock, which is taken last, inside a
//! layer's own lock and never around one; a layer has one placer at a time and shows where its
//! placer's last placement or present put it; a producer's release runs once, when the copy of its
//! buffer is done, or at once where nothing was copied.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak, mpsc};

use waft_embedder::{
    CompositionLayer, Compositor, HardwareBuffer, LayerPicture, Placement, ViewId,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDesktopDevice, IDCompositionTarget,
    IDCompositionVisual2,
};
use windows::core::Interface as _;
use winit::raw_window_handle::RawWindowHandle;
use winit::window::Window;

use super::shared_textures::{FORMAT, SharedTexture};
use crate::gpu::Gpu;
use crate::os::window_handle_layer::WindowComposition;

/// A release a producer handed with its buffer.
type Released = Box<dyn FnOnce() + Send + Sync>;

/// The process's DirectComposition device, every call to which, and to its visuals and targets,
/// is made holding it.
struct Device(IDCompositionDesktopDevice);

/// A visual or a target of the device, touched only under the device's lock.
struct Shared<T>(T);

// SAFETY: DirectComposition's objects are free-threaded COM objects, not bound to the thread that
// made them; every call to one is made under the device's lock, so no two threads call into the
// device at once.
unsafe impl Send for Device {}
unsafe impl<T> Send for Shared<T> {}
unsafe impl<T> Sync for Shared<T> {}

static DEVICE: OnceLock<Option<Mutex<Device>>> = OnceLock::new();

/// The device, locked, made at the first call; `None` where Windows makes none.
fn device() -> Option<MutexGuard<'static, Device>> {
    let device = DEVICE.get_or_init(|| {
        // SAFETY: no rendering device: the device makes visuals and targets only.
        let made = unsafe { DCompositionCreateDevice2::<_, IDCompositionDesktopDevice>(None) };
        made.map_err(|error| eprintln!("waft: no DirectComposition device: {error}"))
            .ok()
            .map(|device| Mutex::new(Device(device)))
    });
    device.as_ref().map(|device| device.lock().unwrap())
}

impl Device {
    fn visual(&self) -> Option<Shared<IDCompositionVisual2>> {
        // SAFETY: a call on the device, under its lock.
        unsafe { self.0.CreateVisual() }
            .map_err(|error| eprintln!("waft: no visual: {error}"))
            .ok()
            .map(Shared)
    }

    fn commit(&self) {
        // SAFETY: a call on the device, under its lock.
        if let Err(error) = unsafe { self.0.Commit() } {
            eprintln!("waft: a composition is not committed: {error}");
        }
    }

    /// Takes each layer of `unplaced` out of the visual it was a child of.
    fn take_out(&self, unplaced: &[(Arc<WindowsLayer>, Arc<Shared<IDCompositionVisual2>>)]) {
        for (layer, parent) in unplaced {
            // SAFETY: a call on the device's visual, under its lock.
            unsafe {
                let _ = parent.0.RemoveVisual(&layer.container.0);
            }
        }
    }

    /// Makes `children` the visuals of `parent`, bottom-most first, then `top` above them.
    fn restack(
        &self,
        parent: &Shared<IDCompositionVisual2>,
        children: &[Arc<WindowsLayer>],
        top: &Shared<IDCompositionVisual2>,
    ) {
        // SAFETY: calls on the device's visuals, under its lock; each child is taken off its
        // parent before it is added, as a visual has one parent.
        unsafe {
            let _ = parent.0.RemoveAllVisuals();
            for child in children {
                let _ = parent.0.AddVisual(&child.container.0, false, None);
            }
            let _ = parent.0.AddVisual(&top.0, false, None);
        }
    }
}

/// What a layer's present uses of the GPU: a device of the compositor's own on Waft's adapter, and
/// its queue, which copy and present, apart from Waft's device, since configuring a surface fails
/// while another thread submits on its device, and producers present from threads of their own
/// while Waft's main thread resizes its views; the lock that keeps the compositor's own configures
/// and submissions apart; the instance its surfaces are made on; and the thread its releases wait
/// on.
struct LayerGpu {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    submissions: Mutex<()>,
    releases: Mutex<mpsc::Sender<(wgpu::SubmissionIndex, Released)>>,
}

static GPU: OnceLock<Option<LayerGpu>> = OnceLock::new();

/// The compositor's device, once Waft's is made.
fn layer_gpu() -> Option<&'static LayerGpu> {
    GPU.get()?.as_ref()
}

/// Waft's device is made: the compositor makes its own on the same adapter, which a layer copies
/// and presents on from now on; none, said so, where the adapter gives no second device.
pub(crate) fn device_made(gpu: &Gpu) {
    GPU.get_or_init(|| {
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("waft composition"),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(gpu.adapter.request_device(&descriptor))
            .map_err(|error| eprintln!("waft: no device for composition layers: {error}"))
            .ok()?;
        let (releases, waits) = mpsc::channel::<(wgpu::SubmissionIndex, Released)>();
        let waiting = device.clone();
        let spawned = std::thread::Builder::new()
            .name("waft layer releases".to_owned())
            .spawn(move || {
                for (submission, released) in waits {
                    let _ = waiting.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    });
                    released();
                }
            });
        if let Err(error) = spawned {
            eprintln!("waft: no thread for the layers' releases: {error}");
        }
        Some(LayerGpu {
            instance: gpu.instance.clone(),
            device,
            queue,
            submissions: Mutex::new(()),
            releases: Mutex::new(releases),
        })
    });
}

/// The compositor on Windows, where DirectComposition makes a device.
pub(crate) struct WindowsCompositor;

impl WindowsCompositor {
    pub(crate) fn new() -> Option<WindowsCompositor> {
        device().map(|_| WindowsCompositor)
    }
}

impl Compositor for WindowsCompositor {
    fn make_layer(&self, opaque: bool) -> Arc<dyn CompositionLayer> {
        Arc::new_cyclic(|me| WindowsLayer::new(me.clone(), opaque))
    }
}

/// One composition layer: its container visual, which its placement places, holding its
/// children's visuals and its content visual, which shows its swapchain.
pub(crate) struct WindowsLayer {
    me: Weak<WindowsLayer>,
    opaque: bool,
    container: Shared<IDCompositionVisual2>,
    content: Shared<IDCompositionVisual2>,
    state: Mutex<LayerState>,
}

struct LayerState {
    /// Who placed the layer, and the visual it is a child of there.
    placed: Option<(Placer, Arc<Shared<IDCompositionVisual2>>)>,
    /// The pixels a producer draws to one unit of a layer, the view's device pixel ratio.
    pixel_ratio: f64,
    /// The swapchain the content visual shows, and its size.
    surface: Option<(wgpu::Surface<'static>, [u32; 2])>,
    buffer: Option<Arc<dyn HardwareBuffer>>,
    children: Vec<Arc<WindowsLayer>>,
}

/// Who placed a layer.
#[derive(Clone)]
enum Placer {
    View(ViewId),
    Layer(Weak<WindowsLayer>),
}

impl Placer {
    fn is(&self, other: &Placer) -> bool {
        match (self, other) {
            (Placer::View(a), Placer::View(b)) => a == b,
            (Placer::Layer(a), Placer::Layer(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

/// Where a layer goes in its parent's pixels: moved, scaled, and cropped in its own pixels.
#[derive(Debug, PartialEq)]
struct Geometry {
    position: [f32; 2],
    scale: f32,
    /// What of the layer shows, in its own pixels: left, top, right, bottom.
    crop: [f32; 4],
}

/// Where `placement`, in its parent's pixels, puts a layer whose buffers are drawn `pixel_ratio`
/// pixels to its unit, as Android places a surface control (`layer_geometry` there): its box under
/// every transform above it, their scale, and what of the box every clip above it leaves. `None`
/// where nothing of it shows.
fn geometry(placement: &Placement, pixel_ratio: f64) -> Option<Geometry> {
    let scale = placement.scale() / pixel_ratio;
    let visible = placement.visible_bounds();
    if placement.opacity() <= 0.0 || !(scale.is_finite() && scale > 0.0) || visible.is_empty() {
        return None;
    }
    let bounds = placement.bounds();
    let position = [bounds.left.round(), bounds.top.round()];
    let local = |parent: f64, origin: f64| (parent - origin) / scale;
    Some(Geometry {
        position: position.map(|side| side as f32),
        scale: scale as f32,
        crop: [
            local(visible.left, position[0]).floor() as f32,
            local(visible.top, position[1]).floor() as f32,
            local(visible.right, position[0]).ceil() as f32,
            local(visible.bottom, position[1]).ceil() as f32,
        ],
    })
}

impl WindowsLayer {
    fn new(me: Weak<WindowsLayer>, opaque: bool) -> WindowsLayer {
        let device = device().expect("a compositor has a device");
        let container = device.visual().expect("a visual for a layer");
        let content = device.visual().expect("a visual for a layer's content");
        // SAFETY: calls on the device's visuals, under its lock.
        unsafe {
            let _ = container.0.AddVisual(&content.0, false, None);
        }
        drop(device);
        WindowsLayer {
            me,
            opaque,
            container,
            content,
            state: Mutex::new(LayerState {
                placed: None,
                pixel_ratio: 1.0,
                surface: None,
                buffer: None,
                children: Vec::new(),
            }),
        }
    }

    /// Takes the layer as `placer`'s child of `parent`, at `pixel_ratio`; false where another
    /// placer holds it.
    fn take_placer(
        &self,
        parent: &Arc<Shared<IDCompositionVisual2>>,
        placer: Placer,
        pixel_ratio: f64,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        if let Some((held, _)) = &state.placed
            && !held.is(&placer)
        {
            return false;
        }
        state.placed = Some((placer, Arc::clone(parent)));
        state.pixel_ratio = pixel_ratio;
        true
    }

    /// Moves, scales and crops the container as `geometry` says. Under the device's lock.
    fn set_geometry(&self, _device: &Device, geometry: &Geometry) {
        let [x, y] = geometry.position;
        let matrix = windows_numerics::Matrix3x2 {
            M11: geometry.scale,
            M12: 0.0,
            M21: 0.0,
            M22: geometry.scale,
            M31: x,
            M32: y,
        };
        let [left, top, right, bottom] = geometry.crop;
        let crop = D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        };
        // SAFETY: calls on the device's visual, under its lock; the clip is in the container's
        // own pixels, which its transform carries.
        unsafe {
            let _ = self.container.0.SetTransform2(&matrix);
            let _ = self.container.0.SetClip2(&crop);
        }
    }

    /// Takes the layer off its placer, which shows it no more: answers the visual it is to be
    /// taken out of, under the device's lock ([`Device::take_out`]).
    fn unplace(self: &Arc<Self>) -> Option<(Arc<WindowsLayer>, Arc<Shared<IDCompositionVisual2>>)> {
        let placed = self.state.lock().unwrap().placed.take()?;
        Some((Arc::clone(self), placed.1))
    }

    /// The children `children` asks for placed beneath this layer's content, and the ones it
    /// no longer names let go.
    fn place_children(self: &Arc<Self>, children: Vec<(Arc<dyn CompositionLayer>, Placement)>) {
        let pixel_ratio = self.state.lock().unwrap().pixel_ratio;
        let parent = Arc::new(Shared(self.container.0.clone()));
        let placer = Placer::Layer(Arc::downgrade(self));
        let mut placed = Vec::new();
        let mut shown = Vec::new();
        let mut hidden = Vec::new();
        for (child, placement) in children {
            let Some(child) = as_windows(child) else {
                eprintln!("waft: a layer of another host is not shown as a child");
                continue;
            };
            match geometry(&placement, pixel_ratio) {
                Some(geometry) if child.take_placer(&parent, placer.clone(), pixel_ratio) => {
                    shown.push((Arc::clone(&child), geometry));
                    placed.push(child);
                }
                Some(_) => eprintln!("waft: a layer another placer holds is not shown here"),
                None => hidden.push(child),
            }
        }
        let previous = std::mem::replace(&mut self.state.lock().unwrap().children, placed.clone());
        let unplaced = unplaced(&previous, &placed, &hidden);
        let Some(device) = device() else {
            return;
        };
        for (child, geometry) in &shown {
            child.set_geometry(&device, geometry);
        }
        device.take_out(&unplaced);
        device.restack(&self.container, &placed, &self.content);
        drop(device);
    }

    /// The swapchain shown, configured for buffers of `size`, made at the first present.
    fn surface<'a>(
        &self,
        state: &'a mut LayerState,
        gpu: &LayerGpu,
        size: [u32; 2],
    ) -> Option<&'a wgpu::Surface<'static>> {
        if state
            .surface
            .as_ref()
            .is_some_and(|(_, made)| *made == size)
        {
            return state.surface.as_ref().map(|(surface, _)| surface);
        }
        let device = device()?;
        if state.surface.is_none() {
            // SAFETY: the content visual lives as long as the layer, and so as long as the surface
            // made on it, which the layer's state holds.
            let target = wgpu::SurfaceTargetUnsafe::CompositionVisual(self.content.0.as_raw());
            let made = unsafe { gpu.instance.create_surface_unsafe(target) };
            match made {
                Ok(surface) => state.surface = Some((surface, [0, 0])),
                Err(error) => {
                    eprintln!("waft: no swapchain for a layer: {error}");
                    return None;
                }
            }
        }
        let (surface, made) = state.surface.as_mut().expect("made above");
        // Configuring a surface on a visual sets the visual's content: a call into DirectComposition,
        // under the device's lock; and it waits for the device to be idle, which it is only while
        // no other layer submits.
        let _submissions = gpu.submissions.lock().unwrap();
        surface.configure(
            &gpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::COPY_DST,
                format: FORMAT,
                width: size[0],
                height: size[1],
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: if self.opaque {
                    wgpu::CompositeAlphaMode::Opaque
                } else {
                    wgpu::CompositeAlphaMode::PreMultiplied
                },
                view_formats: vec![],
                color_space: wgpu::SurfaceColorSpace::Auto,
            },
        );
        drop(device);
        *made = size;
        state.surface.as_ref().map(|(surface, _)| surface)
    }

    /// Copies `texture`, `size` pixels, into the swapchain's next image and presents it, then
    /// hands `released` to the releases' thread for when the copy is done; answers `released`
    /// back where nothing was copied.
    fn copy_and_present(
        &self,
        gpu: &LayerGpu,
        texture: &wgpu::Texture,
        size: [u32; 2],
        released: Released,
    ) -> Result<(), Released> {
        let mut state = self.state.lock().unwrap();
        let Some(surface) = self.surface(&mut state, gpu, size) else {
            return Err(released);
        };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return Err(released),
        };
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            texture.as_image_copy(),
            frame.texture.as_image_copy(),
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
        // Another device draws into the buffer next, from the common state.
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture,
                selector: None,
                state: wgpu::TextureUses::PRESENT,
            }),
        );
        let submissions = gpu.submissions.lock().unwrap();
        let submission = gpu.queue.submit([encoder.finish()]);
        gpu.queue.present(frame);
        drop(submissions);
        drop(state);
        let sent = gpu.releases.lock().unwrap().send((submission, released));
        sent.map_err(|mpsc::SendError((_, released))| released)
    }
}

impl Drop for WindowsLayer {
    fn drop(&mut self) {
        let placed = self.state.get_mut().unwrap().placed.take();
        if let (Some((_, parent)), Some(device)) = (placed, device()) {
            // SAFETY: a call on the device's visual, under its lock.
            unsafe {
                let _ = parent.0.RemoveVisual(&self.container.0);
            }
            device.commit();
        }
    }
}

/// The layers no longer placed, `previous` ones `placed` leaves out and the `hidden` ones, taken off
/// their placer, with the visual each is to be taken out of.
fn unplaced(
    previous: &[Arc<WindowsLayer>],
    placed: &[Arc<WindowsLayer>],
    hidden: &[Arc<WindowsLayer>],
) -> Vec<(Arc<WindowsLayer>, Arc<Shared<IDCompositionVisual2>>)> {
    let gone = previous
        .iter()
        .filter(|layer| !placed.iter().any(|kept| Arc::ptr_eq(kept, layer)));
    hidden
        .iter()
        .chain(gone)
        .filter_map(WindowsLayer::unplace)
        .collect()
}

fn as_windows(layer: Arc<dyn CompositionLayer>) -> Option<Arc<WindowsLayer>> {
    let any: Arc<dyn std::any::Any + Send + Sync> = layer;
    any.downcast::<WindowsLayer>().ok()
}

impl CompositionLayer for WindowsLayer {
    fn present(
        &self,
        buffer: Arc<dyn HardwareBuffer>,
        children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        let any: &dyn std::any::Any = &*buffer;
        let Some(shared) = any.downcast_ref::<SharedTexture>() else {
            released();
            return;
        };
        let Some(gpu) = layer_gpu() else {
            eprintln!("waft: a layer is presented before Waft's device is made; it is not shown");
            released();
            return;
        };
        let Some(texture) = shared.texture_on(&gpu.device) else {
            released();
            return;
        };
        let size = [shared.width(), shared.height()];
        if let Err(released) = self.copy_and_present(gpu, &texture, size, released) {
            released();
            return;
        }
        if let Some(this) = self.me.upgrade() {
            this.place_children(children);
        }
        if let Some(device) = device() {
            device.commit();
        }
        self.state.lock().unwrap().buffer = Some(buffer);
    }

    fn current_picture(&self) -> Option<LayerPicture> {
        self.state
            .lock()
            .unwrap()
            .buffer
            .clone()
            .map(LayerPicture::Buffer)
    }
}

/// A window's share of DirectComposition: its target, whose root visual holds the layers a scene
/// of its view places, then the visual of the view's own picture on top.
pub(crate) struct DirectComposition {
    window: Arc<dyn Window>,
    tree: Option<WindowTree>,
    placed: Mutex<Vec<Arc<WindowsLayer>>>,
}

struct WindowTree {
    _target: Shared<IDCompositionTarget>,
    root: Arc<Shared<IDCompositionVisual2>>,
    picture: Shared<IDCompositionVisual2>,
}

impl WindowTree {
    /// `window`'s target and root, the picture's visual in it; `None` where DirectComposition
    /// makes none, said so.
    fn of(window: &dyn Window) -> Option<WindowTree> {
        let hwnd = hwnd_of(window)?;
        let device = device()?;
        // SAFETY: calls on the device, under its lock, for a window of this process alive while
        // the target is.
        let target = unsafe { device.0.CreateTargetForHwnd(hwnd, false) }
            .map_err(|error| eprintln!("waft: no composition target for a window: {error}"))
            .ok()?;
        let root = device.visual()?;
        let picture = device.visual()?;
        // SAFETY: calls on the device's target and visuals, under its lock.
        unsafe {
            let _ = root.0.AddVisual(&picture.0, false, None);
            let _ = target.SetRoot(&root.0);
        }
        device.commit();
        Some(WindowTree {
            _target: Shared(target),
            root: Arc::new(root),
            picture,
        })
    }
}

fn hwnd_of(window: &dyn Window) -> Option<HWND> {
    let handle = window.rwh_06_window_handle().window_handle().ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void))
}

impl WindowComposition for DirectComposition {
    fn new(window: Arc<dyn Window>) -> Self {
        let tree = WindowTree::of(&*window);
        DirectComposition {
            window,
            tree,
            placed: Mutex::new(Vec::new()),
        }
    }

    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        let Some(tree) = &self.tree else {
            return;
        };
        let pixel_ratio = self.window.scale_factor();
        let mut placed = Vec::new();
        let mut shown = Vec::new();
        let mut hidden = Vec::new();
        for (layer, placement) in placements {
            let Some(layer) = as_windows(Arc::clone(layer)) else {
                eprintln!("waft: a layer of another host is not shown");
                continue;
            };
            match geometry(placement, pixel_ratio) {
                Some(geometry)
                    if layer.take_placer(&tree.root, Placer::View(view), pixel_ratio) =>
                {
                    shown.push((Arc::clone(&layer), geometry));
                    placed.push(layer);
                }
                Some(_) => eprintln!("waft: a layer another view places is not shown here"),
                None => hidden.push(layer),
            }
        }
        let previous = std::mem::replace(&mut *self.placed.lock().unwrap(), placed.clone());
        let unplaced = unplaced(&previous, &placed, &hidden);
        let Some(device) = device() else {
            return;
        };
        for (layer, geometry) in &shown {
            layer.set_geometry(&device, geometry);
        }
        device.take_out(&unplaced);
        let restacked = previous.len() != placed.len()
            || previous
                .iter()
                .zip(&placed)
                .any(|(was, is)| !Arc::ptr_eq(was, is));
        if restacked {
            device.restack(&tree.root, &placed, &tree.picture);
        }
        drop(device);
    }

    fn make_surface(
        &self,
        gpu: &Gpu,
        size: [u32; 2],
        _alpha: valo::SurfaceAlpha,
    ) -> Option<Result<valo::Surface, String>> {
        Some(self.picture_surface(gpu, size))
    }

    fn will_present(&self) {
        if let Some(device) = device() {
            device.commit();
        }
    }
}

impl DirectComposition {
    /// The view's surface: wgpu's composition swapchain on the picture's visual, `size` pixels.
    fn picture_surface(&self, gpu: &Gpu, size: [u32; 2]) -> Result<valo::Surface, String> {
        let tree = self
            .tree
            .as_ref()
            .ok_or("the window has no composition target")?;
        let device = device().ok_or("no DirectComposition device")?;
        let target = wgpu::SurfaceTargetUnsafe::CompositionVisual(tree.picture.0.as_raw());
        // wgpu cannot change a composition swapchain's alpha once made, as it keeps the swapchain
        // and resizes it, so the picture is premultiplied from the start: an opaque view draws
        // opaque pixels, and the layers beneath show through the room a scene clears for them.
        let options = valo::SurfaceOptions::default().with_alpha(valo::SurfaceAlpha::Transparent);
        // SAFETY: the picture's visual lives as long as the window's tree, and the window's
        // composition outlives the view's surface; configuring it sets the visual's content, under
        // the device's lock.
        let surface = unsafe {
            valo::Surface::new_unsafe_with_options(
                &gpu.instance,
                &gpu.adapter,
                &gpu.device,
                target,
                size,
                options,
            )
        }
        .map_err(|error| error.to_string())?;
        device.commit();
        Ok(surface)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Matrix4, Mutator, Rect};

    use super::*;

    #[test]
    fn a_layer_drawn_at_the_views_ratio_shows_unscaled_where_its_box_lies() {
        let placement = Placement {
            rect: Rect::from_ltwh(20.0, 40.0, 320.0, 180.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(2.5, 2.5))],
        };
        let geometry = geometry(&placement, 2.5).expect("it shows");
        assert_eq!(geometry.position, [50.0, 100.0]);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.crop, [0.0, 0.0, 800.0, 450.0]);
    }

    #[test]
    fn a_clip_crops_the_layer_in_its_own_pixels() {
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 400.0, 400.0),
            mutators: vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::ClipRect(Rect::from_ltwh(50.0, 50.0, 100.0, 100.0)),
                Mutator::Transform(Matrix4::scale(0.5, 0.5)),
            ],
        };
        let geometry = geometry(&placement, 2.0).expect("it shows");
        assert_eq!(geometry.position, [0.0, 0.0]);
        assert_eq!(geometry.scale, 0.5);
        assert_eq!(geometry.crop, [200.0, 200.0, 600.0, 600.0]);
    }

    #[test]
    fn a_layer_nothing_lets_show_is_not_placed() {
        let faded = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::Opacity(0)],
        };
        assert_eq!(geometry(&faded, 1.0), None);
    }
}
