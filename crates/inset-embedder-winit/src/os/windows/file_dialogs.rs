//! The file dialogs: the shell's `IFileOpenDialog` and `IFileSaveDialog`, owned by the parent
//! view's window where there is one, each answering paths once the user closes it. A dialog runs
//! its own modal loop, so it shows from the hidden window's procedure once the framework's call
//! has returned (`hidden_window`).
//!
//! A type's extensions become the dialog's filters, under its label; the shell filters by no MIME
//! type, so a type with none of its extensions filters nothing here. Every pick is a path the app
//! may read and write, as the shell grants, so `writable` asks nothing more here.

use std::path::PathBuf;
use std::rc::Rc;

use inset_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FileType, FolderOptions, OpenOptions,
    PickedFile, SaveOptions, ViewId,
};
use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
    CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, FileSaveDialog,
    IFileDialog, IFileOpenDialog, IFileSaveDialog, IShellItem, SHCreateItemFromParsingName,
    SIGDN_FILESYSPATH,
};
use windows::core::{HRESULT, HSTRING, PCWSTR};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::Windows;
use super::hidden_window::run_soon;
use crate::file_dialogs::{ParentWindows, dialog_answer, extension_patterns};
use crate::os::SystemFileDialogs;

impl SystemFileDialogs for Windows {
    fn file_dialogs(windows: ParentWindows) -> Option<Rc<dyn FileDialogs>> {
        Some(Rc::new(ShellDialogs { windows }))
    }
}

/// The shell's open and save dialogs.
struct ShellDialogs {
    windows: ParentWindows,
}

/// A dialog asked for, as it shows.
enum Request {
    Open {
        types: Vec<FileType>,
        initial_folder: Option<PathBuf>,
        confirm_label: Option<String>,
        choose: Choose,
    },
    Save(SaveOptions),
}

#[derive(Clone, Copy)]
enum Choose {
    Files { multiple: bool },
    Folder,
}

impl FileDialogs for ShellDialogs {
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let chosen = self.show(parent, open(options, Choose::Files { multiple: false }));
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }

    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        let chosen = self.show(parent, open(options, Choose::Files { multiple: true }));
        Box::pin(async move { Ok(chosen.await?.into_iter().map(PickedFile::Path).collect()) })
    }

    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PathBuf>> {
        let request = Request::Open {
            types: Vec::new(),
            initial_folder: options.initial_folder,
            confirm_label: options.confirm_label,
            choose: Choose::Folder,
        };
        let chosen = self.show(parent, request);
        Box::pin(async move { Ok(chosen.await?.into_iter().next()) })
    }

    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let chosen = self.show(parent, Request::Save(options));
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }
}

fn open(options: OpenOptions, choose: Choose) -> Request {
    Request::Open {
        types: options.types,
        initial_folder: options.initial_folder,
        confirm_label: options.confirm_label,
        choose,
    }
}

impl ShellDialogs {
    /// Shows the dialog `request` asks for once the current call has returned, owned by
    /// `parent`'s window, and answers the paths chosen, none if the user cancelled.
    fn show(&self, parent: Option<ViewId>, request: Request) -> FileDialogFuture<Vec<PathBuf>> {
        let owner = self.windows.window(parent).and_then(|window| hwnd(&window));
        let (answer, answered) = dialog_answer();
        run_soon(Box::new(move || {
            let _apartment = Apartment::enter();
            let shown = match &request {
                Request::Open {
                    types,
                    initial_folder,
                    confirm_label,
                    choose,
                } => show_open(owner, types, initial_folder, confirm_label, *choose),
                Request::Save(options) => show_save(owner, options),
            };
            answer.give(match shown {
                Ok(chosen) => Ok(chosen),
                Err(error) if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => {
                    Ok(Vec::new())
                }
                Err(error) => Err(FileDialogError::Failed(error.message())),
            });
        }));
        answered
    }
}

fn show_open(
    owner: Option<HWND>,
    types: &[FileType],
    initial_folder: &Option<PathBuf>,
    confirm_label: &Option<String>,
    choose: Choose,
) -> windows::core::Result<Vec<PathBuf>> {
    // SAFETY: the shell's own dialog class, made on this thread, which COM was entered on.
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: plain calls on the dialog made above, with values that outlive each call.
    unsafe {
        let mut options = dialog.GetOptions()? | FOS_FORCEFILESYSTEM;
        match choose {
            Choose::Files { multiple: true } => options |= FOS_ALLOWMULTISELECT,
            Choose::Files { multiple: false } => {}
            Choose::Folder => options |= FOS_PICKFOLDERS,
        }
        dialog.SetOptions(options)?;
    }
    configure(&dialog, types, initial_folder, confirm_label)?;
    // SAFETY: the dialog runs its modal loop here, outside every call into the app; the owner is
    // a live window of this thread's, or none.
    unsafe { dialog.Show(owner)? };
    // SAFETY: the dialog has answered, so its results are there.
    let results = unsafe { dialog.GetResults()? };
    // SAFETY: plain calls on the results the dialog answered.
    let count = unsafe { results.GetCount()? };
    (0..count)
        .map(|index| {
            // SAFETY: `index` is below the count the results gave.
            let item = unsafe { results.GetItemAt(index)? };
            path_of(&item)
        })
        .collect()
}

fn show_save(owner: Option<HWND>, options: &SaveOptions) -> windows::core::Result<Vec<PathBuf>> {
    // SAFETY: the shell's own dialog class, made on this thread, which COM was entered on.
    let dialog: IFileSaveDialog =
        unsafe { CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: plain calls on the dialog made above, with values that outlive each call.
    unsafe {
        dialog.SetOptions(dialog.GetOptions()? | FOS_FORCEFILESYSTEM)?;
        if let Some(name) = &options.suggested_name {
            dialog.SetFileName(&HSTRING::from(name.as_str()))?;
        }
        let first_extension = options
            .types
            .iter()
            .flat_map(|kind| &kind.extensions)
            .next();
        if let Some(extension) = first_extension {
            dialog.SetDefaultExtension(&HSTRING::from(extension.trim_start_matches('.')))?;
        }
    }
    configure(
        &dialog,
        &options.types,
        &options.initial_folder,
        &options.confirm_label,
    )?;
    // SAFETY: as for an open dialog.
    unsafe { dialog.Show(owner)? };
    // SAFETY: the dialog has answered, so its result is there.
    let item = unsafe { dialog.GetResult()? };
    Ok(vec![path_of(&item)?])
}

/// What every dialog takes from its options: its filters, the folder it starts in, and its
/// confirming button's label.
fn configure(
    dialog: &IFileDialog,
    types: &[FileType],
    initial_folder: &Option<PathBuf>,
    confirm_label: &Option<String>,
) -> windows::core::Result<()> {
    let patterns: Vec<(HSTRING, HSTRING)> = extension_patterns(types)
        .into_iter()
        .map(|(label, spec)| (HSTRING::from(label), HSTRING::from(spec)))
        .collect();
    let filters: Vec<COMDLG_FILTERSPEC> = patterns
        .iter()
        .map(|(label, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(label.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect();
    // SAFETY: the filters' strings live in `patterns` past the calls, which copy them; the folder
    // item is the shell's own.
    unsafe {
        if !filters.is_empty() {
            dialog.SetFileTypes(&filters)?;
        }
        if let Some(folder) = initial_folder {
            let folder: IShellItem =
                SHCreateItemFromParsingName(&HSTRING::from(folder.as_os_str()), None)?;
            dialog.SetFolder(&folder)?;
        }
        if let Some(label) = confirm_label {
            dialog.SetOkButtonLabel(&HSTRING::from(label.as_str()))?;
        }
    }
    Ok(())
}

/// The file-system path of `item`.
fn path_of(item: &IShellItem) -> windows::core::Result<PathBuf> {
    // SAFETY: the item is the dialog's answer; the name it gives is the caller's to free, which it
    // is, once read.
    unsafe {
        let name = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = name.to_hstring();
        CoTaskMemFree(Some(name.as_ptr().cast()));
        Ok(PathBuf::from(path.to_os_string()))
    }
}

/// COM entered on this thread for a dialog, left again when dropped if entering it was new.
/// winit's own drag and drop has usually entered it already, as a single-threaded apartment.
struct Apartment {
    entered: bool,
}

impl Apartment {
    fn enter() -> Apartment {
        // SAFETY: entering COM on this thread, which the drop balances when it succeeded.
        let entered =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        Apartment {
            entered: entered.is_ok(),
        }
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.entered {
            // SAFETY: balances the `CoInitializeEx` that succeeded above, on the same thread.
            unsafe { CoUninitialize() };
        }
    }
}

/// The window's handle; `None` for a window winit gives none.
fn hwnd(window: &Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return None;
    };
    Some(HWND(win32.hwnd.get() as *mut core::ffi::c_void))
}
