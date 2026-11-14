//! The NDK's surface controls and their transactions (`android/surface_control.h`), which neither
//! `ndk` nor `ndk-sys` binds: a surface control is a node of SurfaceFlinger's tree, made from a
//! window or under another, and a transaction changes any number of them, applied atomically.
//! Linked directly: the calls are of API 29 and 31, and Android 12 (API 31) is the least the host
//! runs on.
//!
//! The invariant: a surface control here is a reference the wrapper owns and releases, and a
//! release takes it off nothing: a surface control is on screen while its parent is and until a
//! transaction reparents it to none, which is what takes it down.

// The NDK's C API is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_void};
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

use ndk::hardware_buffer::HardwareBufferRef;
use ndk::native_window::NativeWindow;
use ndk_sys::{AHardwareBuffer, ANativeWindow, ARect};

use crate::buffer_transform::BufferTransform;

#[repr(C)]
struct ASurfaceControl {
    _opaque: [u8; 0],
}

#[repr(C)]
struct ASurfaceTransaction {
    _opaque: [u8; 0],
}

#[repr(C)]
struct ASurfaceTransactionStats {
    _opaque: [u8; 0],
}

type OnComplete = unsafe extern "C" fn(context: *mut c_void, stats: *mut ASurfaceTransactionStats);

#[link(name = "android")]
unsafe extern "C" {
    fn ASurfaceControl_createFromWindow(
        parent: *mut ANativeWindow,
        debug_name: *const c_char,
    ) -> *mut ASurfaceControl;
    fn ASurfaceControl_create(
        parent: *mut ASurfaceControl,
        debug_name: *const c_char,
    ) -> *mut ASurfaceControl;
    fn ASurfaceControl_release(surface_control: *mut ASurfaceControl);
    fn ASurfaceTransaction_create() -> *mut ASurfaceTransaction;
    fn ASurfaceTransaction_delete(transaction: *mut ASurfaceTransaction);
    fn ASurfaceTransaction_apply(transaction: *mut ASurfaceTransaction);
    fn ASurfaceTransaction_setOnComplete(
        transaction: *mut ASurfaceTransaction,
        context: *mut c_void,
        func: OnComplete,
    );
    fn ASurfaceTransaction_reparent(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        new_parent: *mut ASurfaceControl,
    );
    fn ASurfaceTransaction_setVisibility(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        visibility: i8,
    );
    fn ASurfaceTransaction_setZOrder(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        z_order: i32,
    );
    fn ASurfaceTransaction_setBuffer(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        buffer: *mut AHardwareBuffer,
        acquire_fence_fd: i32,
    );
    fn ASurfaceTransaction_setBufferTransparency(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        transparency: i8,
    );
    fn ASurfaceTransaction_setPosition(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        x: i32,
        y: i32,
    );
    fn ASurfaceTransaction_setScale(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        x_scale: f32,
        y_scale: f32,
    );
    fn ASurfaceTransaction_setBufferTransform(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        transform: i32,
    );
    fn ASurfaceTransaction_setCrop(
        transaction: *mut ASurfaceTransaction,
        surface_control: *mut ASurfaceControl,
        crop: *const ARect,
    );
    fn ASurfaceTransactionStats_getASurfaceControls(
        stats: *mut ASurfaceTransactionStats,
        out_surface_controls: *mut *mut *mut ASurfaceControl,
        out_count: *mut usize,
    );
    fn ASurfaceTransactionStats_releaseASurfaceControls(
        surface_controls: *mut *mut ASurfaceControl,
    );
    fn ASurfaceTransactionStats_getPreviousReleaseFenceFd(
        stats: *mut ASurfaceTransactionStats,
        surface_control: *mut ASurfaceControl,
    ) -> i32;
}

/// `ASURFACE_TRANSACTION_VISIBILITY_HIDE` and `_SHOW`.
const HIDE: i8 = 0;
const SHOW: i8 = 1;
/// `ASURFACE_TRANSACTION_TRANSPARENCY_TRANSLUCENT` and `_OPAQUE`.
const TRANSLUCENT: i8 = 1;
const OPAQUE: i8 = 2;
/// The acquire fence of a buffer whose GPU work is done.
const NO_FENCE: i32 = -1;

/// A surface control, released when dropped, and a number no other surface control made in this
/// process has, which tells one parent from another where the system's pointer could be reused.
pub(crate) struct SurfaceControl {
    raw: *mut ASurfaceControl,
    number: u64,
}

// SAFETY: a surface control is reference counted by the system and changed only through
// transactions, which any thread may build and apply.
unsafe impl Send for SurfaceControl {}
unsafe impl Sync for SurfaceControl {}

impl SurfaceControl {
    /// A surface control under `window`'s own, the root of what the app shows in it; `None` where
    /// the system makes none.
    pub(crate) fn from_window(window: &NativeWindow, name: &CStr) -> Option<SurfaceControl> {
        // SAFETY: a live window and a NUL-terminated name.
        let raw = unsafe { ASurfaceControl_createFromWindow(window.ptr().as_ptr(), name.as_ptr()) };
        SurfaceControl::owning(raw)
    }

    /// A surface control under `parent`, shown from the transaction that first changes it on;
    /// `None` where the system makes none.
    pub(crate) fn under(parent: &SurfaceControl, name: &CStr) -> Option<SurfaceControl> {
        // SAFETY: a live parent and a NUL-terminated name.
        let raw = unsafe { ASurfaceControl_create(parent.raw, name.as_ptr()) };
        SurfaceControl::owning(raw)
    }

    fn owning(raw: *mut ASurfaceControl) -> Option<SurfaceControl> {
        static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);
        (!raw.is_null()).then(|| SurfaceControl {
            raw,
            number: NEXT_NUMBER.fetch_add(1, Ordering::Relaxed),
        })
    }

    /// What tells this surface control from every other one made in the process.
    pub(crate) fn number(&self) -> u64 {
        self.number
    }
}

impl Drop for SurfaceControl {
    fn drop(&mut self) {
        // SAFETY: the reference this owns; the system keeps the surface control for as long as a
        // transaction or its parent needs it.
        unsafe { ASurfaceControl_release(self.raw) };
    }
}

/// Where a surface control lies in its parent: its own pixels in `crop`, scaled by `scale` about
/// its origin, the origin at `position`, in the parent's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Geometry {
    pub(crate) position: [i32; 2],
    pub(crate) scale: f32,
    /// `[left, top, right, bottom]`, in the surface control's own pixels.
    pub(crate) crop: [i32; 4],
}

/// One transaction: what it changes, applied atomically when it is, and what it runs once its
/// frame is on screen.
pub(crate) struct Transaction {
    raw: *mut ASurfaceTransaction,
}

// SAFETY: a transaction is plain data until applied, and is built and applied by one thread at a
// time, which owning it here makes sure of.
unsafe impl Send for Transaction {}

impl Transaction {
    pub(crate) fn new() -> Transaction {
        // SAFETY: no arguments; the transaction is deleted when this drops.
        Transaction {
            raw: unsafe { ASurfaceTransaction_create() },
        }
    }

    /// Makes `parent` the parent of `child`, or none, which takes it and its children off screen.
    pub(crate) fn reparent(&mut self, child: &SurfaceControl, parent: Option<&SurfaceControl>) {
        let parent = parent.map_or(ptr::null_mut(), |parent| parent.raw);
        // SAFETY: a live transaction and surface controls; a null parent is allowed.
        unsafe { ASurfaceTransaction_reparent(self.raw, child.raw, parent) };
    }

    pub(crate) fn set_visible(&mut self, surface: &SurfaceControl, visible: bool) {
        let visibility = if visible { SHOW } else { HIDE };
        // SAFETY: a live transaction and surface control.
        unsafe { ASurfaceTransaction_setVisibility(self.raw, surface.raw, visibility) };
    }

    /// Orders `surface` among its siblings, the higher above.
    pub(crate) fn set_z_order(&mut self, surface: &SurfaceControl, z: i32) {
        // SAFETY: a live transaction and surface control.
        unsafe { ASurfaceTransaction_setZOrder(self.raw, surface.raw, z) };
    }

    /// Shows `buffer` on `surface` once `acquire_fence` signals, at once where there is none,
    /// as for a buffer whose GPU work is done; the transaction owns the fence from here on.
    pub(crate) fn set_buffer(
        &mut self,
        surface: &SurfaceControl,
        buffer: &HardwareBufferRef,
        acquire_fence: Option<OwnedFd>,
    ) {
        let fence = acquire_fence.map_or(NO_FENCE, IntoRawFd::into_raw_fd);
        // SAFETY: a live transaction, surface control and buffer, which the transaction takes a
        // reference on; the fence is a descriptor handed over, which the system closes, as the
        // header says.
        unsafe { ASurfaceTransaction_setBuffer(self.raw, surface.raw, buffer.as_ptr(), fence) };
    }

    /// Whether SurfaceFlinger may take `surface`'s buffer as covering what lies beneath it.
    pub(crate) fn set_opaque(&mut self, surface: &SurfaceControl, opaque: bool) {
        let transparency = if opaque { OPAQUE } else { TRANSLUCENT };
        // SAFETY: a live transaction and surface control.
        unsafe { ASurfaceTransaction_setBufferTransparency(self.raw, surface.raw, transparency) };
    }

    /// Places `surface` in its parent as `geometry` says.
    pub(crate) fn set_geometry(&mut self, surface: &SurfaceControl, geometry: &Geometry) {
        let [left, top, right, bottom] = geometry.crop;
        let crop = ARect {
            left,
            top,
            right,
            bottom,
        };
        let [x, y] = geometry.position;
        // SAFETY: a live transaction and surface control; the crop lives across its call, and the
        // scale is positive, as the call asks.
        unsafe {
            ASurfaceTransaction_setPosition(self.raw, surface.raw, x, y);
            ASurfaceTransaction_setScale(self.raw, surface.raw, geometry.scale, geometry.scale);
            ASurfaceTransaction_setCrop(self.raw, surface.raw, &crop);
        }
    }

    /// Turns `surface`'s buffer by `transform` before it is shown, cropped, scaled and placed.
    pub(crate) fn set_buffer_transform(
        &mut self,
        surface: &SurfaceControl,
        transform: BufferTransform,
    ) {
        // SAFETY: a live transaction and surface control, and a transform of Android's flags.
        unsafe { ASurfaceTransaction_setBufferTransform(self.raw, surface.raw, transform.flags()) };
    }

    /// Runs `complete`, on a thread of the system's, once the transaction's frame is on screen,
    /// with what the system says of it.
    pub(crate) fn on_complete(&mut self, complete: impl FnOnce(&Completion) + Send + 'static) {
        type Complete = Box<dyn FnOnce(&Completion) + Send>;
        let complete: Box<Complete> = Box::new(Box::new(complete));
        // SAFETY: the context is a box `completed` takes back exactly once, which the system
        // calls once for an applied transaction.
        unsafe {
            ASurfaceTransaction_setOnComplete(self.raw, Box::into_raw(complete).cast(), completed);
        }
    }

    /// Applies every change at once.
    pub(crate) fn apply(self) {
        // SAFETY: a live transaction, deleted when this drops.
        unsafe { ASurfaceTransaction_apply(self.raw) };
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        // SAFETY: the transaction this owns.
        unsafe { ASurfaceTransaction_delete(self.raw) };
    }
}

/// What the system calls once an applied transaction's frame is on screen: the closure
/// `on_complete` boxed, with the stats, valid for the call alone.
unsafe extern "C" fn completed(context: *mut c_void, stats: *mut ASurfaceTransactionStats) {
    // SAFETY: the box `on_complete` handed over, taken back once.
    let complete = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce(&Completion) + Send>>()) };
    complete(&Completion { stats });
}

/// What the system says of a transaction whose frame is on screen, during `on_complete`'s call.
pub(crate) struct Completion {
    stats: *mut ASurfaceTransactionStats,
}

/// When the buffer a surface showed before a transaction set another goes back to its producer.
pub(crate) enum PreviousRelease {
    /// The system says nothing of the surface control.
    NotReported,
    /// The compositor is done with it already.
    Free,
    /// Signals once the compositor is done with it.
    Fence(OwnedFd),
}

impl Completion {
    /// The release of the buffer `surface` showed before the transaction's: the fence the system
    /// hands over for it, which signals once the compositor is done with it.
    pub(crate) fn previous_release(&self, surface: &SurfaceControl) -> PreviousRelease {
        if !self.reports(surface) {
            return PreviousRelease::NotReported;
        }
        // SAFETY: stats valid during the call, of a surface control they hold; the fd is the
        // caller's, as the header says.
        match unsafe { ASurfaceTransactionStats_getPreviousReleaseFenceFd(self.stats, surface.raw) }
        {
            fd if fd >= 0 => PreviousRelease::Fence(unsafe { OwnedFd::from_raw_fd(fd) }),
            _ => PreviousRelease::Free,
        }
    }

    /// Whether the stats hold `surface`: asking for the fence of one they do not is a fatal error.
    fn reports(&self, surface: &SurfaceControl) -> bool {
        let mut controls = ptr::null_mut();
        let mut count = 0;
        // SAFETY: stats valid during the call; the list is released as the header asks.
        unsafe {
            ASurfaceTransactionStats_getASurfaceControls(self.stats, &mut controls, &mut count);
            let held = !controls.is_null()
                && std::slice::from_raw_parts(controls, count).contains(&surface.raw);
            ASurfaceTransactionStats_releaseASurfaceControls(controls);
            held
        }
    }
}
