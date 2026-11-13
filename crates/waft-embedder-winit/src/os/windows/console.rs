//! The console of the process that started the app. Windows starts a GUI program, one linked for
//! its `windows` subsystem (`#![windows_subsystem = "windows"]`), with no console, so it opens no
//! console window of its own; attached to its parent's console, the terminal a person ran it from,
//! what it writes to its standard output and error shows there.
//!
//! It is not a console program's terminal, as Flutter's Windows runner, which attaches the same
//! way, is not (`runner/main.cpp` of its app template):
//! - cmd and PowerShell do not wait for a GUI program, so the prompt comes back at once and what
//!   the app writes lands over it and whatever is typed next. A parent that waits, as
//!   `cargo waft run` does, keeps the terminal the app's.
//! - Ctrl-C in that console, at the prompt too, and closing the terminal end the app through the
//!   console's default handler, without Waft's quit flow: the process ends there, the app never
//!   asked whether to exit.
//!
//! The invariants a later editor will break:
//! - A stream the parent redirected, to a file or a pipe, stays redirected. Attaching points all
//!   three standard handles at the console, the redirected ones too, so each that was usable is
//!   set back; and with both output and error redirected the app is not attached at all, since
//!   an attached app also hears the console's Ctrl-C and its closing.
//! - A program started with no console in its parent, from Explorer or a shortcut, stays with
//!   none, opening no window.

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_UNKNOWN, GetFileType};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
};

/// Attaches the app to its parent's console, where the parent has one, for the standard streams
/// the parent left without a file or a pipe. The attaching fails, changing nothing, where the
/// parent has no console or the app has one already, as a console program has.
pub(super) fn attach_to_parent_console() {
    let streams = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE];
    let given = streams.map(|stream| (stream, usable_handle(stream)));
    if given[1].1.is_some() && given[2].1.is_some() {
        return;
    }
    // SAFETY: the call takes a process id and touches only this process's console.
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
        return;
    }
    for (stream, handle) in given {
        if let Some(handle) = handle {
            // SAFETY: a handle this process was started with, which it still holds.
            unsafe { SetStdHandle(stream, handle) };
        }
    }
}

/// The handle of the standard stream `stream`, where the process was given one that is open: a
/// file, a pipe or a console's.
fn usable_handle(stream: STD_HANDLE) -> Option<HANDLE> {
    // SAFETY: the call reads this process's standard handle.
    let handle = unsafe { GetStdHandle(stream) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: the type of a handle the process holds; one that is not open answers unknown.
    let open = unsafe { GetFileType(handle) } != FILE_TYPE_UNKNOWN;
    open.then_some(handle)
}
