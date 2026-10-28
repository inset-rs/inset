//! The embedder's hidden window, which the notification area tells what happens to the app's icons
//! (`menus/notify_icons`), and from whose procedure every system loop the app asks for runs, on a
//! message posted for it ([`run_soon`]): a menu's (`menus/popup_menu`) and a file dialog's
//! (`file_dialogs`), each of which runs its own modal loop. Run from there, a loop is outside
//! every call into the app, while Windows goes on dispatching winit's window messages from it, as
//! it does while a window is moved.
//!
//! It is a top-level window that never shows rather than a message-only one, so it hears
//! Explorer's `TaskbarCreated`, broadcast when the taskbar comes back after Explorer restarted,
//! and so a menu at an icon can make it the foreground window, as Windows asks. It is made on the
//! main thread, the thread of winit's event loop, which dispatches its messages, when it is first
//! needed, and lives as long as the process.

use std::cell::{OnceCell, RefCell};
use std::collections::VecDeque;
use std::ptr;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, PostMessageW, RegisterClassExW, RegisterWindowMessageW,
    WM_APP, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::menus;

/// What the notification area sends about an icon, as the icons ask it to.
pub(super) const ICON_NOTIFICATION: u32 = WM_APP + 1;
/// The work posted is to run.
const RUN_POSTED: u32 = WM_APP + 2;

/// Work for a later turn of the main thread.
pub(super) type Posted = Box<dyn FnOnce()>;

thread_local! {
    /// The window, once made; `None` if Windows refused it. Main thread.
    static HIDDEN_WINDOW: OnceCell<Option<HWND>> = const { OnceCell::new() };
    /// The number Windows gives `TaskbarCreated` in this session.
    static TASKBAR_CREATED: OnceCell<u32> = const { OnceCell::new() };
    /// The work posted, in order. Main thread.
    static POSTED: RefCell<VecDeque<Posted>> = RefCell::default();
}

/// The hidden window, made now if it is not yet; `None` where Windows refuses it.
pub(super) fn hidden_window() -> Option<HWND> {
    HIDDEN_WINDOW.with(|window| *window.get_or_init(make_window))
}

/// Runs `work` from the window's procedure once the current call has returned, in the order
/// posted; drops it where Windows takes no message for the window.
pub(super) fn run_soon(work: Posted) {
    let Some(window) = hidden_window() else {
        return;
    };
    POSTED.with_borrow_mut(|posted| posted.push_back(work));
    // SAFETY: the window is the embedder's own, alive for the process; the message carries
    // nothing.
    if unsafe { PostMessageW(window, RUN_POSTED, 0, 0) } == 0 {
        POSTED.with_borrow_mut(VecDeque::clear);
    }
}

/// Runs the work posted, one after the other.
fn run_posted() {
    while let Some(work) = POSTED.with_borrow_mut(VecDeque::pop_front) {
        work();
    }
}

fn make_window() -> Option<HWND> {
    let class_name = wide("InsetHiddenWindow");
    // SAFETY: a null name answers the module of the process's executable.
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_procedure),
        hInstance: instance,
        lpszClassName: class_name.as_ptr(),
        ..WNDCLASSEXW::default()
    };
    // SAFETY: the class names a procedure that answers every message, and its name outlives the
    // call, which copies it. A second registration fails harmlessly; the window is made once.
    unsafe { RegisterClassExW(&class) };
    // SAFETY: the class is registered above; the window has no parent and no menu, and is never
    // shown.
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };
    (!window.is_null()).then_some(window)
}

/// The number of `TaskbarCreated` in this session.
fn taskbar_created() -> u32 {
    TASKBAR_CREATED.with(|number| {
        *number.get_or_init(|| {
            let name = wide("TaskbarCreated");
            // SAFETY: the name is a NUL-terminated wide string that outlives the call.
            unsafe { RegisterWindowMessageW(name.as_ptr()) }
        })
    })
}

/// What the window does with its messages. A panic here aborts the process, as Rust does at an
/// `extern "system"` boundary.
unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        ICON_NOTIFICATION => menus::icon_notified(wparam, lparam),
        RUN_POSTED => run_posted(),
        message if message == taskbar_created() => menus::taskbar_created(),
        // SAFETY: Windows' own answer to a message the window leaves, with what it was sent.
        _ => return unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
    0
}

/// `text` as a NUL-terminated wide string.
pub(super) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}
