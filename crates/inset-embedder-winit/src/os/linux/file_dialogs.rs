//! The file dialogs: the desktop portal's `FileChooser`, which the desktop shows, over the session
//! bus, answering the `file://` URIs it gives as paths. Each dialog asks on a thread of its own,
//! which waits for the portal's `Response` for as long as the dialog shows, so the app goes on
//! meanwhile. The dialogs are `None` where the session bus has no portal, running or one it can
//! start.
//!
//! A type becomes the portal's filter, by its extensions' globs and its MIME types, under its
//! label. A dialog is modal to its parent's window on X11, which the portal names by the window's
//! id; on Wayland naming it would need the window exported, which winit does not, so the dialog
//! stands alone there. The portal grants reading and writing to every pick, so `writable` asks
//! nothing more here.

use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use inset_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FolderOptions, OpenOptions, PickedFile,
    PickedFolder, SaveOptions, ViewId, file_uri_path,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
use zbus::zvariant::OwnedValue;

use super::Linux;
use super::portal::{self, owned};
use crate::file_dialogs::{ParentWindows, dialog_answer, portal_filters};
use crate::os::SystemFileDialogs;

const FILE_CHOOSER: &str = "org.freedesktop.portal.FileChooser";

impl SystemFileDialogs for Linux {
    fn file_dialogs(windows: ParentWindows) -> Option<Rc<dyn FileDialogs>> {
        portal::is_on_the_bus().then(|| Rc::new(PortalDialogs { windows }) as Rc<dyn FileDialogs>)
    }
}

/// The portal's file dialogs.
struct PortalDialogs {
    windows: ParentWindows,
}

/// What a dialog asks the portal, which a thread of its own takes.
struct Ask {
    method: &'static str,
    title: &'static str,
    parent_window: String,
    options: HashMap<&'static str, OwnedValue>,
}

impl FileDialogs for PortalDialogs {
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let chosen = self.ask(
            parent,
            "OpenFile",
            "Open File",
            open_options(&options, false),
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }

    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        let chosen = self.ask(
            parent,
            "OpenFile",
            "Open Files",
            open_options(&options, true),
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().map(PickedFile::Path).collect()) })
    }

    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>> {
        let mut asked = common_options(&options.initial_folder, &options.confirm_label);
        asked.insert("directory", owned(true));
        let chosen = self.ask(parent, "OpenFile", "Open Folder", asked);
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFolder::Path)) })
    }

    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let mut asked = common_options(&options.initial_folder, &options.confirm_label);
        if !options.types.is_empty() {
            asked.insert("filters", owned(portal_filters(&options.types)));
        }
        if let Some(name) = options.suggested_name {
            asked.insert("current_name", owned(name));
        }
        let chosen = self.ask(parent, "SaveFile", "Save File", asked);
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }
}

fn open_options(options: &OpenOptions, multiple: bool) -> HashMap<&'static str, OwnedValue> {
    let mut asked = common_options(&options.initial_folder, &options.confirm_label);
    asked.insert("multiple", owned(multiple));
    if !options.types.is_empty() {
        asked.insert("filters", owned(portal_filters(&options.types)));
    }
    asked
}

/// What every dialog takes from its options: the folder it starts in, as the portal's
/// NUL-terminated bytes, and its confirming button's label.
fn common_options(
    initial_folder: &Option<PathBuf>,
    confirm_label: &Option<String>,
) -> HashMap<&'static str, OwnedValue> {
    let mut asked = HashMap::new();
    if let Some(folder) = initial_folder {
        asked.insert("current_folder", owned(nul_terminated(folder)));
    }
    if let Some(label) = confirm_label {
        asked.insert("accept_label", owned(label.clone()));
    }
    asked
}

fn nul_terminated(path: &Path) -> Vec<u8> {
    path.as_os_str()
        .as_bytes()
        .iter()
        .copied()
        .chain([0])
        .collect()
}

impl PortalDialogs {
    /// Asks the portal's `method` on a thread of its own, and answers the paths it gives.
    fn ask(
        &self,
        parent: Option<ViewId>,
        method: &'static str,
        title: &'static str,
        options: HashMap<&'static str, OwnedValue>,
    ) -> FileDialogFuture<Vec<PathBuf>> {
        let ask = Ask {
            method,
            title,
            parent_window: self
                .windows
                .window(parent)
                .and_then(|window| parent_window(&window))
                .unwrap_or_default(),
            options,
        };
        let (answer, answered) = dialog_answer();
        let asking = std::thread::Builder::new()
            .name("inset-file-dialog".to_owned())
            .spawn(move || answer.give(ask_portal(ask)));
        if let Err(error) = asking {
            let (failed, answered) = dialog_answer();
            failed.give(Err(FileDialogError::Failed(error.to_string())));
            return answered;
        }
        answered
    }
}

/// The portal's name for `window`: its id on X11, and none elsewhere.
fn parent_window(window: &Window) -> Option<String> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Xlib(xlib) => Some(format!("x11:{:x}", xlib.window)),
        RawWindowHandle::Xcb(xcb) => Some(format!("x11:{:x}", xcb.window.get())),
        _ => None,
    }
}

/// Asks the portal and waits for its answer: the paths chosen, none if the user cancelled.
fn ask_portal(ask: Ask) -> Result<Vec<PathBuf>, FileDialogError> {
    let Ask {
        method,
        title,
        parent_window,
        options,
    } = ask;
    let response = portal::request(FILE_CHOOSER, method, options, |options| {
        (parent_window, title, options)
    })
    .map_err(|error| FileDialogError::Failed(error.to_string()))?;
    match response.code {
        0 => Ok(uris(&response.results)
            .iter()
            .filter_map(|uri| file_uri_path(uri))
            .collect()),
        1 => Ok(Vec::new()),
        _ => Err(FileDialogError::Failed(
            "the portal ended the dialog".to_owned(),
        )),
    }
}

/// The URIs a response's results give.
fn uris(results: &HashMap<String, OwnedValue>) -> Vec<String> {
    results
        .get("uris")
        .and_then(|uris| Vec::<String>::try_from(uris.try_clone().ok()?).ok())
        .unwrap_or_default()
}
