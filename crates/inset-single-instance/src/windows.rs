//! Windows: a named mutex, which a later launch finds already made, and a named pipe the first
//! instance listens on, which a later launch writes its arguments and folder to before it quits.
//! Both are named for the app and the user's session, so two sessions' instances never meet; the
//! pipe takes no client from another machine, and the first instance makes each of its pipes as
//! the pipe's first, so it never listens on one another process made under that name.
//!
//! A message is the arguments, then the folder, each as its count of UTF-16 units, little-endian
//! 32 bits, and the units, after their number.

// The mutex and the pipe are system calls, which Rust reaches through `unsafe`; this module is the
// one place in the crate that writes it.
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{ptr, thread};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_WRITE,
    GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_FIRST_PIPE_INSTANCE, OPEN_EXISTING,
    PIPE_ACCESS_INBOUND, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    WaitNamedPipeW,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentProcessId};

use crate::{Arrivals, FirstInstance, SingleInstance, this_launch};

/// How long a later launch tries to reach the first instance's pipe, which a first instance just
/// started may not have opened yet.
const PATIENCE: Duration = Duration::from_secs(3);

pub(crate) fn claim(id: &str) -> io::Result<SingleInstance> {
    let name = format!("inset-single-instance-{}-{}", sanitized(id), session());
    let mutex_name = wide(&format!("Local\\{name}"));
    // SAFETY: a NUL-terminated name that outlives the call, and default security.
    let mutex = unsafe { CreateMutexW(ptr::null(), 0, mutex_name.as_ptr()) };
    if mutex.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: read right after the call it reports on.
    let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    let pipe = wide(&format!(r"\\.\pipe\{name}"));
    if existed {
        // SAFETY: the mutex handle `CreateMutexW` answered, closed once.
        unsafe { CloseHandle(mutex) };
        hand_over(&pipe)?;
        return Ok(SingleInstance::Second);
    }
    let arrivals = Arc::new(Arrivals::default());
    let first = FirstInstance::new(Arc::clone(&arrivals), Box::new(Owned(mutex)));
    thread::Builder::new()
        .name("inset-single-instance".to_owned())
        .spawn(move || listen(&pipe, &arrivals))?;
    Ok(SingleInstance::First(first))
}

/// A handle of this process's own, closed when dropped.
struct Owned(HANDLE);

// SAFETY: a kernel handle may be held and closed on any thread.
unsafe impl Send for Owned {}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle is this process's own, closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// The app's id as a name may carry it: letters, digits, dots and dashes.
fn sanitized(id: &str) -> String {
    id.chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() || matches!(letter, '.' | '-') {
                letter
            } else {
                '_'
            }
        })
        .collect()
}

/// The user's session, which the names are made per.
fn session() -> u32 {
    let mut session = 0;
    // SAFETY: the out-pointer is the call's own; a failure leaves session 0.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
    session
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Writes this launch's arguments and folder to the first instance's pipe.
fn hand_over(pipe: &[u16]) -> io::Result<()> {
    let (arguments, current_dir) = this_launch();
    let message = encode(&arguments, &current_dir.into_os_string());
    let deadline = Instant::now() + PATIENCE;
    let handle = loop {
        // SAFETY: a NUL-terminated name, opened to write; the handle is closed below.
        let handle = unsafe {
            CreateFileW(
                pipe.as_ptr(),
                GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            break Owned(handle);
        }
        // SAFETY: read right after the call it reports on.
        let error = unsafe { GetLastError() };
        if Instant::now() >= deadline {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        if error == ERROR_PIPE_BUSY {
            // SAFETY: a NUL-terminated name.
            unsafe { WaitNamedPipeW(pipe.as_ptr(), 500) };
        } else {
            thread::sleep(Duration::from_millis(50));
        }
    };
    let mut written = 0;
    while written < message.len() {
        let mut wrote = 0;
        let rest = &message[written..];
        // SAFETY: the bytes outlive the call, which writes at most their length.
        let ok = unsafe {
            WriteFile(
                handle.0,
                rest.as_ptr(),
                rest.len() as u32,
                &mut wrote,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        written += wrote as usize;
    }
    Ok(())
}

/// Listens for later launches on the pipe, one after another, for as long as the process runs; each
/// pipe is closed once read, so the next is again the name's first.
fn listen(pipe: &[u16], arrivals: &Arc<Arrivals>) {
    loop {
        // SAFETY: a NUL-terminated name; the instance is closed once read.
        let instance = unsafe {
            CreateNamedPipeW(
                pipe.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                0,
                64 * 1024,
                0,
                ptr::null(),
            )
        };
        if instance == INVALID_HANDLE_VALUE {
            return;
        }
        let instance = Owned(instance);
        // SAFETY: the pipe instance made above, waited on without overlap.
        let connected = unsafe { ConnectNamedPipe(instance.0, ptr::null_mut()) } != 0
            // SAFETY: read right after the call it reports on.
            || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if connected {
            if let Some((arguments, current_dir)) = decode(&read_all(&instance)) {
                arrivals.arrive(arguments, PathBuf::from(current_dir));
            }
            // SAFETY: the connected instance, which is closed next.
            unsafe { DisconnectNamedPipe(instance.0) };
        }
    }
}

/// Everything the client writes, until it closes its end.
fn read_all(instance: &Owned) -> Vec<u8> {
    let mut message = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let mut read = 0;
        // SAFETY: the chunk outlives the call, which reads at most its length.
        let ok = unsafe {
            ReadFile(
                instance.0,
                chunk.as_mut_ptr(),
                chunk.len() as u32,
                &mut read,
                ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            return message;
        }
        message.extend_from_slice(&chunk[..read as usize]);
    }
}

/// `arguments`, then `current_dir`, as the pipe carries them.
fn encode(arguments: &[OsString], current_dir: &OsString) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&(arguments.len() as u32).to_le_bytes());
    for text in arguments.iter().chain([current_dir]) {
        let units: Vec<u16> = text.encode_wide().collect();
        message.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            message.extend_from_slice(&unit.to_le_bytes());
        }
    }
    message
}

/// The arguments and the folder a message carries; `None` for one cut short.
fn decode(message: &[u8]) -> Option<(Vec<OsString>, OsString)> {
    let mut rest = message;
    let number = |rest: &mut &[u8]| -> Option<u32> {
        let (bytes, after) = rest.split_first_chunk::<4>()?;
        *rest = after;
        Some(u32::from_le_bytes(*bytes))
    };
    let count = number(&mut rest)?;
    let mut texts = Vec::new();
    for _ in 0..=count {
        let units = number(&mut rest)? as usize;
        let (bytes, after) = rest.split_at_checked(units * 2)?;
        rest = after;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        texts.push(OsString::from_wide(&units));
    }
    let current_dir = texts.pop()?;
    Some((texts, current_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_carries_the_arguments_and_the_folder() {
        let arguments = vec![
            OsString::from("notes.txt"),
            OsString::from("https://example.org"),
        ];
        let folder = OsString::from(r"C:\Users\me");
        let message = encode(&arguments, &folder);
        assert_eq!(decode(&message), Some((arguments, folder)));
        assert_eq!(decode(&message[..message.len() - 1]), None);
    }
}
