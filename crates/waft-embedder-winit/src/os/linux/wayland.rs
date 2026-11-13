//! Waft's own share of the Wayland connection winit opened: the globals composition layers need
//! (`wl_compositor`, `wl_subcompositor`, `wp_viewporter`, `zwp_linux_dmabuf_v1`), bound on an event
//! queue of Waft's, which a thread of its own reads, as Mesa's Vulkan WSI shares the connection on
//! a queue of its own. A dmabuf becomes a `wl_buffer` here, and its releases come back here.
//!
//! The invariants a later editor must keep: nothing here makes a request that can fail with a
//! protocol error, since one ends the whole connection, winit's windows with it: a `wl_buffer` is
//! made with the asynchronous `zwp_linux_buffer_params_v1.create`, whose `failed` is answered as
//! the buffer not being shown, as GTK's graphics offload does, never with `create_immed`; and every
//! object made here is on Waft's queue, so its events never reach winit's.
//!
//! wayland-client's foreign display and foreign proxies wrap libwayland's pointers, which is why
//! this module allows `unsafe`.
#![allow(unsafe_code)]

use std::collections::VecDeque;
use std::os::fd::AsFd;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_region, wl_registry, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

use waft_embedder::HardwareBuffer;

use super::dmabuf::{ARGB8888, DmabufBuffer, MOD_INVALID, MOD_LINEAR};

/// How long a `wl_buffer`'s making is waited for before the buffer is taken as refused.
const BUFFER_WAIT: Duration = Duration::from_secs(2);

/// The display winit connected to, where it is Wayland's, set once the event loop is built.
static DISPLAY: OnceLock<usize> = OnceLock::new();

/// Waft's share of the connection, made at its first use; `None` on X11, and where the compositor
/// lacks a global composition layers need.
static WAYLAND: OnceLock<Option<Arc<Wayland>>> = OnceLock::new();

/// Remembers the `wl_display` winit connected to, for [`Wayland::get`].
pub(crate) fn set_display(display: *mut std::ffi::c_void) {
    let _ = DISPLAY.set(display as usize);
}

pub(crate) struct Wayland {
    connection: Connection,
    queue: QueueHandle<Events>,
    pub(crate) compositor: wl_compositor::WlCompositor,
    pub(crate) subcompositor: wl_subcompositor::WlSubcompositor,
    pub(crate) viewporter: wp_viewporter::WpViewporter,
    linux_dmabuf: zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
    /// The modifiers the compositor takes ARGB8888 in.
    modifiers: Arc<Mutex<Vec<u64>>>,
}

/// What a `wl_buffer` carries: the producers' releases, one for each present not yet released, in
/// the order presented.
#[derive(Default)]
pub(crate) struct BufferData {
    releases: Mutex<VecDeque<Box<dyn FnOnce() + Send + Sync>>>,
}

impl BufferData {
    /// `released` runs at the compositor's next `wl_buffer.release` of this buffer not already
    /// owed to an earlier present.
    pub(crate) fn release_with(&self, released: Box<dyn FnOnce() + Send + Sync>) {
        self.releases.lock().unwrap().push_back(released);
    }
}

/// The events of Waft's queue, read on its thread.
pub(crate) struct Events;

/// What a `zwp_linux_buffer_params_v1` waits for: whether the compositor made the buffer.
struct Asked(Mutex<Option<SyncSender<Option<wl_buffer::WlBuffer>>>>);

impl Wayland {
    /// Waft's share of the connection, made at the first call; `None` on X11 and where the
    /// compositor lacks a global composition layers need, which is logged once.
    pub(crate) fn get() -> Option<Arc<Wayland>> {
        WAYLAND
            .get_or_init(|| {
                let display = *DISPLAY.get()?;
                match Wayland::connect(display as *mut _) {
                    Ok(wayland) => Some(Arc::new(wayland)),
                    Err(missing) => {
                        eprintln!("waft: composition layers are not shown: {missing}");
                        None
                    }
                }
            })
            .clone()
    }

    fn connect(display: *mut std::ffi::c_void) -> Result<Wayland, String> {
        // SAFETY: winit's live `wl_display`, which outlives the app's windows and so every use.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, mut queue) =
            registry_queue_init::<Events>(&connection).map_err(|error| error.to_string())?;
        let handle = queue.handle();
        let bind_error = |name: &str| format!("the compositor offers no {name}");
        let compositor = globals
            .bind(&handle, 4..=6, ())
            .map_err(|_| bind_error("wl_compositor"))?;
        let subcompositor = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| bind_error("wl_subcompositor"))?;
        let viewporter = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| bind_error("wp_viewporter"))?;
        let modifiers = Arc::new(Mutex::new(Vec::new()));
        // Version 3 lists the formats and modifiers as events of the global itself.
        let linux_dmabuf = globals
            .bind(&handle, 3..=3, Arc::clone(&modifiers))
            .map_err(|_| bind_error("zwp_linux_dmabuf_v1 at version 3"))?;
        queue
            .roundtrip(&mut Events)
            .map_err(|error| error.to_string())?;
        std::thread::Builder::new()
            .name("waft wayland".into())
            .spawn(
                move || {
                    while queue.blocking_dispatch(&mut Events).is_ok() {}
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(Wayland {
            connection,
            queue: handle,
            compositor,
            subcompositor,
            viewporter,
            linux_dmabuf,
            modifiers,
        })
    }

    pub(crate) fn queue(&self) -> &QueueHandle<Events> {
        &self.queue
    }

    /// Sends what was asked so far.
    pub(crate) fn flush(&self) {
        let _ = self.connection.flush();
    }

    /// The window's own `wl_surface`, winit's, as a proxy Waft makes requests on; its events stay
    /// winit's.
    pub(crate) fn foreign_surface(
        &self,
        surface: *mut std::ffi::c_void,
    ) -> Option<wl_surface::WlSurface> {
        // SAFETY: a live `wl_surface` of winit's window, of this interface, on this connection.
        let id = unsafe { ObjectId::from_ptr(wl_surface::WlSurface::interface(), surface.cast()) }
            .ok()?;
        wl_surface::WlSurface::from_id(&self.connection, id).ok()
    }

    /// The modifier the compositor is told `buffer` has: its own where the compositor takes it,
    /// and for a linear buffer the implicit modifier where that alone is offered, as a driver's
    /// implicit layout is linear for a buffer allocated linear. `None` where it takes neither.
    fn announced_modifier(&self, buffer: &DmabufBuffer) -> Option<u64> {
        let offered = self.modifiers.lock().unwrap();
        if offered.contains(&buffer.modifier()) {
            Some(buffer.modifier())
        } else if buffer.modifier() == MOD_LINEAR && offered.contains(&MOD_INVALID) {
            Some(MOD_INVALID)
        } else {
            None
        }
    }

    /// `buffer` as a `wl_buffer`, made by the compositor and waited for; `None`, said why, where
    /// the compositor refuses it or does not answer.
    pub(crate) fn wl_buffer(&self, buffer: &DmabufBuffer) -> Option<wl_buffer::WlBuffer> {
        let Some(modifier) = self.announced_modifier(buffer) else {
            eprintln!(
                "waft: the compositor takes ARGB8888 in no modifier this buffer ({:#x}) has: it is not shown",
                buffer.modifier()
            );
            return None;
        };
        let (answer, answered) = sync_channel(1);
        let params = self
            .linux_dmabuf
            .create_params(&self.queue, Asked(Mutex::new(Some(answer))));
        params.add(
            buffer.fd().as_fd(),
            0,
            buffer.offset(),
            buffer.stride(),
            (modifier >> 32) as u32,
            modifier as u32,
        );
        params.create(
            buffer.width() as i32,
            buffer.height() as i32,
            ARGB8888,
            zwp_linux_buffer_params_v1::Flags::empty(),
        );
        self.flush();
        let made = answered.recv_timeout(BUFFER_WAIT).ok().flatten();
        params.destroy();
        if made.is_none() {
            eprintln!("waft: the compositor refused a dmabuf: its layer is not shown");
        }
        made
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Events {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, Arc<Mutex<Vec<u64>>>> for Events {
    fn event(
        _: &mut Self,
        _: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        event: zwp_linux_dmabuf_v1::Event,
        modifiers: &Arc<Mutex<Vec<u64>>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_linux_dmabuf_v1::Event::Modifier {
            format,
            modifier_hi,
            modifier_lo,
        } = event
            && format == ARGB8888
        {
            let modifier = (u64::from(modifier_hi) << 32) | u64::from(modifier_lo);
            modifiers.lock().unwrap().push(modifier);
        }
    }
}

impl Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, Asked> for Events {
    fn event(
        _: &mut Self,
        _: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        event: zwp_linux_buffer_params_v1::Event,
        asked: &Asked,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let made = match event {
            zwp_linux_buffer_params_v1::Event::Created { buffer } => Some(buffer),
            zwp_linux_buffer_params_v1::Event::Failed => None,
            _ => return,
        };
        if let Some(answer) = asked.0.lock().unwrap().take() {
            let _ = answer.send(made);
        }
    }

    event_created_child!(Events, zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, [
        zwp_linux_buffer_params_v1::EVT_CREATED_OPCODE => (wl_buffer::WlBuffer, Arc::new(BufferData::default())),
    ]);
}

impl Dispatch<wl_buffer::WlBuffer, Arc<BufferData>> for Events {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        data: &Arc<BufferData>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            let released = data.releases.lock().unwrap().pop_front();
            if let Some(released) = released {
                released();
            }
        }
    }
}

delegate_noop!(Events: ignore wl_compositor::WlCompositor);
delegate_noop!(Events: ignore wl_subcompositor::WlSubcompositor);
delegate_noop!(Events: ignore wl_surface::WlSurface);
delegate_noop!(Events: ignore wl_subsurface::WlSubsurface);
delegate_noop!(Events: ignore wl_region::WlRegion);
delegate_noop!(Events: ignore wp_viewporter::WpViewporter);
delegate_noop!(Events: ignore wp_viewport::WpViewport);
