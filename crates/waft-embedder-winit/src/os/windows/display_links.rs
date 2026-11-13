//! A display's refreshes on Windows: a thread per display link waits on `IDXGIOutput::WaitForVBlank`
//! for the output that shows the display, as Chromium's default vsync thread does
//! (`ui/gl/vsync_thread_win_dxgi.cc`), and has the main thread called through the embedder's
//! hidden window (`hidden_window`). The output is looked for on every adapter, by the display's
//! `HMONITOR`, since the adapter that drives a display need not be the one that draws: in a
//! virtual machine without a D3D12 driver, the display is the virtual GPU's and the drawing WARP's
//! (`experiments/2026-11-13-windows-composition`, row 3).
//!
//! `WaitForVBlank` answers at once while the desktop is occluded, as Chromium notes, so a wait that
//! ends sooner than half an interval after the last vsync is followed by a sleep to the next.
//!
//! The invariant a later editor must keep: at most one call is posted for a link before the main
//! thread has made it, so a busy main thread is called once for the vsyncs it missed, not once
//! for each.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput};
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;
use winit::monitor::MonitorHandle;

use super::hidden_window::{VSYNC, hidden_window};
use crate::os::capabilities::DisplayLink;

/// The time between vsyncs assumed where the display reports no rate.
const DEFAULT_INTERVAL: Duration = Duration::from_micros(16_667);

/// One display's vsyncs, from a thread of its own, while it is kept.
pub(crate) struct VBlankLink {
    id: u64,
    state: Arc<LinkState>,
}

/// What the link's thread and the main thread share.
struct LinkState {
    /// Whether the calls are wanted; the thread waits while they are not.
    wanted: Mutex<bool>,
    wake: Condvar,
    stopped: AtomicBool,
    /// A call is posted that the main thread has not made yet.
    posted: AtomicBool,
    /// The latest vsync, and when the picture drawn at it reaches the screen.
    times: Mutex<Option<(Instant, Instant)>>,
}

/// An output, which the link's thread waits on.
struct Output(IDXGIOutput);

// SAFETY: DXGI's objects are free-threaded; the output is used by the link's thread alone once it
// is handed there.
unsafe impl Send for Output {}

/// A link's call at a vsync, and its state.
type LinkCall = (Box<dyn Fn()>, Arc<LinkState>);

thread_local! {
    /// Each link's call, by its id. Main thread.
    static CALLS: RefCell<HashMap<u64, LinkCall>> = RefCell::default();
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl DisplayLink for VBlankLink {
    /// A link for `display`, wanting its vsyncs; `None` where no adapter has an output showing it,
    /// or the embedder has no hidden window to be called through.
    fn start(display: &MonitorHandle, on_vsync: Box<dyn Fn()>) -> Option<VBlankLink> {
        let output = output_showing(HMONITOR(display.native_id() as *mut core::ffi::c_void))?;
        let window = hidden_window()? as usize;
        let interval = interval_of(display);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(LinkState {
            wanted: Mutex::new(true),
            wake: Condvar::new(),
            stopped: AtomicBool::new(false),
            posted: AtomicBool::new(false),
            times: Mutex::new(None),
        });
        let waiting = Arc::clone(&state);
        std::thread::Builder::new()
            .name("waft vblank".to_owned())
            .spawn(move || wait_for_vblanks(output, interval, &waiting, window, id))
            .map_err(|error| eprintln!("waft: no thread for a display's vsyncs: {error}"))
            .ok()?;
        CALLS.with_borrow_mut(|calls| calls.insert(id, (on_vsync, Arc::clone(&state))));
        Some(VBlankLink { id, state })
    }

    fn set_wanted(&self, wanted: bool) {
        *self.state.wanted.lock().unwrap() = wanted;
        self.state.wake.notify_one();
    }

    fn target_time(&self) -> Option<Instant> {
        self.state.times.lock().unwrap().map(|(_, target)| target)
    }

    fn vsync_time(&self) -> Option<Instant> {
        self.state.times.lock().unwrap().map(|(vsync, _)| vsync)
    }
}

impl Drop for VBlankLink {
    fn drop(&mut self) {
        self.state.stopped.store(true, Ordering::Release);
        self.set_wanted(true);
        CALLS.with_borrow_mut(|calls| calls.remove(&self.id));
    }
}

/// The main thread's call for the link `id`, posted at a vsync: made outside the table, since it
/// may start or stop links, and put back unless the link stopped meanwhile. Main thread.
pub(super) fn vsync(id: u64) {
    let Some((on_vsync, state)) = CALLS.with_borrow_mut(|calls| calls.remove(&id)) else {
        return;
    };
    state.posted.store(false, Ordering::Release);
    on_vsync();
    if !state.stopped.load(Ordering::Acquire) {
        CALLS.with_borrow_mut(|calls| {
            calls.entry(id).or_insert((on_vsync, state));
        });
    }
}

/// The link's thread: waits for each vsync while they are wanted, and posts the main thread's
/// call for it to `window`, until the link stops.
fn wait_for_vblanks(output: Output, interval: Duration, state: &LinkState, window: usize, id: u64) {
    let mut last = Instant::now();
    loop {
        {
            let wanted = state.wanted.lock().unwrap();
            let _wanted = state.wake.wait_while(wanted, |wanted| !*wanted).unwrap();
        }
        if state.stopped.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: a call on an output this thread holds.
        let waited = unsafe { output.0.WaitForVBlank() }.is_ok();
        let mut now = Instant::now();
        if !waited || now < last + interval / 2 {
            std::thread::sleep((last + interval).saturating_duration_since(now));
            now = Instant::now();
        }
        last = now;
        *state.times.lock().unwrap() = Some((now, now + interval));
        if !state.posted.swap(true, Ordering::AcqRel) {
            // SAFETY: the hidden window lives as long as the process; the message carries the
            // link's id only.
            let sent = unsafe { PostMessageW(window as _, VSYNC, id as usize, 0) };
            if sent == 0 {
                state.posted.store(false, Ordering::Release);
            }
        }
    }
}

/// The output on any adapter whose monitor is `monitor`.
fn output_showing(monitor: HMONITOR) -> Option<Output> {
    // SAFETY: DXGI's factory and its enumeration, with no arguments but indices.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut adapter_index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(adapter_index) {
            adapter_index += 1;
            let mut output_index = 0;
            while let Ok(output) = adapter.EnumOutputs(output_index) {
                output_index += 1;
                if output.GetDesc().is_ok_and(|desc| desc.Monitor == monitor) {
                    return Some(Output(output));
                }
            }
        }
    }
    None
}

/// The time between `display`'s vsyncs, from its rate as winit reports it.
fn interval_of(display: &MonitorHandle) -> Duration {
    display
        .current_video_mode()
        .and_then(|mode| mode.refresh_rate_millihertz())
        .map(|millihertz| Duration::from_secs_f64(1000.0 / f64::from(millihertz.get())))
        .unwrap_or(DEFAULT_INTERVAL)
}
