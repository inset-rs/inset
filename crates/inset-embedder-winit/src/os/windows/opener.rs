//! Opening: the shell's. A link or a file opens through `ShellExecuteW`, in the app the
//! registry associates with its scheme or its kind; a file is revealed in Explorer, selected in
//! its folder (`SHOpenFolderAndSelectItems`). Each runs on a thread of its own, in a COM apartment
//! of its own, as the shell asks of a caller it may keep waiting, and answers once the shell has
//! handed it on. Whether some app opens a link is the registry's association of its scheme
//! (`AssocQueryStringW`).

use std::path::Path;
use std::ptr;
use std::rc::Rc;

use inset_embedder::{OpenError, OpenFuture, Opener};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::UI::Shell::{
    ASSOCF_INIT_IGNOREUNKNOWN, ASSOCSTR_EXECUTABLE, AssocQueryStringW, ILCreateFromPathW, ILFree,
    SE_ERR_NOASSOC, SHOpenFolderAndSelectItems, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::Windows;
use super::hidden_window::wide;
use crate::answer::answer;
use crate::file_dialogs::ParentWindows;
use crate::os::SystemOpener;

impl SystemOpener for Windows {
    fn opener(_windows: ParentWindows) -> Option<Rc<dyn Opener>> {
        Some(Rc::new(Shell))
    }
}

/// The shell.
struct Shell;

impl Opener for Shell {
    fn open_link(&self, link: &str) -> OpenFuture {
        let link = wide(link);
        on_a_thread(move || shell_execute(&link))
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        if !path.exists() {
            return failed(format!("{} is not there", path.display()));
        }
        let path = wide_path(path);
        on_a_thread(move || shell_execute(&path))
    }

    fn reveal_file(&self, path: &Path) -> OpenFuture {
        if !path.exists() {
            return failed(format!("{} is not there", path.display()));
        }
        let path = wide_path(path);
        on_a_thread(move || reveal(&path))
    }

    fn can_open_link(&self, link: &str) -> Option<bool> {
        let (scheme, _) = link.split_once(':')?;
        let scheme = wide(scheme);
        let mut length = 0;
        // SAFETY: a NUL-terminated scheme; with no buffer the call only answers the length.
        let found = unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_EXECUTABLE,
                scheme.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut length,
            )
        };
        Some(found >= 0 && length > 0)
    }
}

fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain([0]).collect()
}

fn failed(reason: String) -> OpenFuture {
    Box::pin(std::future::ready(Err(OpenError::Failed(reason))))
}

/// Runs `open` on a thread of its own, in a COM apartment of its own, and answers what it does.
fn on_a_thread(open: impl FnOnce() -> Result<(), OpenError> + Send + 'static) -> OpenFuture {
    let (answer, answered) = answer(Err(OpenError::Failed(
        "the shell ended without an answer".to_owned(),
    )));
    let started = std::thread::Builder::new()
        .name("inset-opener".to_owned())
        .spawn(move || {
            // SAFETY: COM entered on this thread for the shell, left before the thread ends.
            let entered = unsafe {
                CoInitializeEx(
                    ptr::null(),
                    (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
                )
            };
            answer.give(open());
            if entered >= 0 {
                // SAFETY: balances the `CoInitializeEx` that succeeded above.
                unsafe { CoUninitialize() };
            }
        });
    if let Err(error) = started {
        return failed(error.to_string());
    }
    Box::pin(answered)
}

/// `ShellExecuteW`'s `open` of `target`, a link or a path.
fn shell_execute(target: &[u16]) -> Result<(), OpenError> {
    let verb = wide("open");
    // SAFETY: NUL-terminated strings that outlive the call; no window owns what it opens.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    } as usize;
    match result {
        code if code > 32 => Ok(()),
        code if code == SE_ERR_NOASSOC as usize => Err(OpenError::NoApp),
        code => Err(OpenError::Failed(format!("the shell answered {code}"))),
    }
}

/// Explorer's window of the file's folder, the file selected.
fn reveal(path: &[u16]) -> Result<(), OpenError> {
    // SAFETY: a NUL-terminated path; the list the call answers is freed below.
    let item = unsafe { ILCreateFromPathW(path.as_ptr()) };
    if item.is_null() {
        return Err(OpenError::Failed(
            "the shell does not know the file".to_owned(),
        ));
    }
    // SAFETY: the item list made above, standing for the file; selecting it in its own folder.
    let shown = unsafe { SHOpenFolderAndSelectItems(item, 0, ptr::null(), 0) };
    // SAFETY: the list `ILCreateFromPathW` made, freed once.
    unsafe { ILFree(item) };
    if shown >= 0 {
        Ok(())
    } else {
        Err(OpenError::Failed(format!("the shell answered {shown:#x}")))
    }
}
