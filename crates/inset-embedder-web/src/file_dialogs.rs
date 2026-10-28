//! The browser's file dialogs. A page has no paths, so every pick is a handle.
//!
//! Where the browser has the File System Access API (`showOpenFilePicker`, `showSaveFilePicker`;
//! Chrome and Edge), a pick is the browser's `FileSystemFileHandle`: read through `File.slice`, and
//! written through `createWritable({ keepExistingData: true })`, writing at positions into a copy
//! the browser replaces the file with only when the writer finishes, and drops when it is dropped
//! before. An open pick reads only, unless the app asks to write, when the browser is asked for
//! write permission as the pick is answered, which it may refuse; a save pick writes.
//!
//! Elsewhere (Safari, Firefox), opening falls back to an `<input type="file">`, whose files are
//! read-only handles; a save fails, as the browser has no way to write a file where the user
//! chooses, and a download would give the app nothing it could write again.
//!
//! A folder pick is the browser's `FileSystemDirectoryHandle` (`showDirectoryPicker`, in
//! `readwrite` mode when the app asks to write, which the browser asks the user about), listed
//! through its `values()`, opening its files and folders by name, and making them when writable.
//! Elsewhere a folder pick fails: Safari's and Firefox's `webkitdirectory` input gives a flat list
//! of files, which no folder could be written through.
//!
//! A dialog asks for the user's gesture: the app opens one while it handles the user's click or
//! key, as the browser tells a page's own dialog from one a script forced. The initial folder and
//! the confirming button's label have no counterpart in the browser's pickers.

use std::cell::Cell;

use inset_embedder::{
    EntryKind, FileDialogError, FileDialogFuture, FileDialogs, FileFinishFuture, FileHandle,
    FileOpenFuture, FileReadFuture, FileSizeFuture, FileType, FileWriteFuture, FileWriter,
    FolderEntriesFuture, FolderEntry, FolderHandle, FolderOptions, OpenOptions, PickedFile,
    PickedFolder, SaveOptions, ViewId,
};
use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, HtmlInputElement};

/// The browser's pickers.
pub(crate) struct BrowserFileDialogs;

impl FileDialogs for BrowserFileDialogs {
    fn open_file(
        &self,
        _parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let picked = open(options, false);
        Box::pin(async move { Ok(picked.await?.into_iter().next()) })
    }

    fn open_files(
        &self,
        _parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        Box::pin(open(options, true))
    }

    fn open_folder(
        &self,
        _parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>> {
        let Some(show) = picker("showDirectoryPicker") else {
            return failed("this browser cannot pick a folder");
        };
        let asked = Object::new();
        let mode = if options.writable {
            "readwrite"
        } else {
            "read"
        };
        set(&asked, "mode", &JsValue::from_str(mode));
        let shown = call(&show, &asked);
        Box::pin(async move {
            let Some(handle) = answered(shown).await? else {
                return Ok(None);
            };
            let folder = SystemFolder::new(handle, options.writable);
            Ok(Some(PickedFolder::Handle(Box::new(folder))))
        })
    }

    fn save_file(
        &self,
        _parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let Some(show) = picker("showSaveFilePicker") else {
            return failed("this browser cannot save a file where the user chooses");
        };
        let asked = Object::new();
        set(&asked, "types", &picker_types(&options.types));
        if let Some(name) = &options.suggested_name {
            set(&asked, "suggestedName", &JsValue::from_str(name));
        }
        let shown = call(&show, &asked);
        Box::pin(async move {
            let Some(handle) = answered(shown).await? else {
                return Ok(None);
            };
            let file = SystemFile::new(handle, true);
            Ok(Some(PickedFile::Handle(Box::new(file))))
        })
    }
}

fn failed<T: 'static>(reason: &str) -> FileDialogFuture<T> {
    Box::pin(std::future::ready(Err(FileDialogError::Failed(
        reason.to_owned(),
    ))))
}

/// Opens files with the browser's picker where it has one, and its file input otherwise.
async fn open(options: OpenOptions, multiple: bool) -> Result<Vec<PickedFile>, FileDialogError> {
    match picker("showOpenFilePicker") {
        Some(show) => open_with_picker(&show, &options, multiple).await,
        None => open_with_input(&options, multiple).await,
    }
}

async fn open_with_picker(
    show: &Function,
    options: &OpenOptions,
    multiple: bool,
) -> Result<Vec<PickedFile>, FileDialogError> {
    let asked = Object::new();
    set(&asked, "multiple", &JsValue::from_bool(multiple));
    set(&asked, "types", &picker_types(&options.types));
    let Some(handles) = answered(call(show, &asked)).await? else {
        return Ok(Vec::new());
    };
    let mut picked = Vec::new();
    for handle in Array::from(&handles).iter() {
        let writable = options.writable && write_granted(&handle).await;
        picked.push(PickedFile::Handle(Box::new(SystemFile::new(
            handle, writable,
        ))));
    }
    Ok(picked)
}

/// The browser's picker function `name` on the window, where it has it.
fn picker(name: &str) -> Option<Function> {
    let window = web_sys::window()?;
    Reflect::get(&window, &JsValue::from_str(name))
        .ok()?
        .dyn_into()
        .ok()
}

/// Calls the picker `show` with `options` on the window.
fn call(show: &Function, options: &Object) -> Result<Promise, FileDialogError> {
    let window = web_sys::window().ok_or_else(|| failure("the page has no window"))?;
    show.call1(&window, options)
        .map_err(|error| failure(&describe(&error)))?
        .dyn_into()
        .map_err(|_| failure("the picker answered no promise"))
}

/// What a picker's promise resolves to; `None` once the user cancelled, which the browser rejects
/// with an `AbortError`.
async fn answered(
    shown: Result<Promise, FileDialogError>,
) -> Result<Option<JsValue>, FileDialogError> {
    match JsFuture::from(shown?).await {
        Ok(answer) => Ok(Some(answer)),
        Err(error) if error_name(&error) == "AbortError" => Ok(None),
        Err(error) => Err(failure(&describe(&error))),
    }
}

/// Asks the browser for write permission on `handle`; whether it granted it.
async fn write_granted(handle: &JsValue) -> bool {
    let asked = Object::new();
    set(&asked, "mode", &JsValue::from_str("readwrite"));
    let Ok(request) = method(handle, "requestPermission", &[asked.into()]) else {
        return false;
    };
    let Ok(answer) = JsFuture::from(request).await else {
        return false;
    };
    answer.as_string().as_deref() == Some("granted")
}

/// `types` as the pickers' `types`: one entry a type, its MIME types each accepting its
/// extensions; a type with extensions alone accepts them as any bytes.
fn picker_types(types: &[FileType]) -> Array {
    types
        .iter()
        .map(|kind| {
            let extensions: Array = kind
                .extensions
                .iter()
                .map(|extension| {
                    JsValue::from_str(&format!(".{}", extension.trim_start_matches('.')))
                })
                .collect();
            let accept = Object::new();
            match kind.mime_types.split_first() {
                Some((first, rest)) => {
                    set(&accept, first, &extensions);
                    for mime in rest {
                        set(&accept, mime, &Array::new());
                    }
                }
                None => set(&accept, "application/octet-stream", &extensions),
            }
            let entry = Object::new();
            set(&entry, "description", &JsValue::from_str(&kind.label));
            set(&entry, "accept", &accept);
            JsValue::from(entry)
        })
        .collect()
}

/// Opens files with a file input: the files the user chose, read-only, once its `change` comes,
/// and none once its `cancel` comes.
async fn open_with_input(
    options: &OpenOptions,
    multiple: bool,
) -> Result<Vec<PickedFile>, FileDialogError> {
    let input: HtmlInputElement = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.create_element("input").ok())
        .and_then(|element| element.dyn_into().ok())
        .ok_or_else(|| failure("the page cannot make a file input"))?;
    input.set_type("file");
    input.set_multiple(multiple);
    input.set_accept(&input_accept(&options.types));
    let chosen = Promise::new(&mut |resolve, _reject| {
        let on_change = Closure::once_into_js({
            let resolve = resolve.clone();
            move || {
                let _ = resolve.call1(&JsValue::NULL, &JsValue::TRUE);
            }
        });
        let on_cancel = Closure::once_into_js(move || {
            let _ = resolve.call1(&JsValue::NULL, &JsValue::FALSE);
        });
        let _ = input.add_event_listener_with_callback("change", on_change.unchecked_ref());
        let _ = input.add_event_listener_with_callback("cancel", on_cancel.unchecked_ref());
    });
    input.click();
    let confirmed = JsFuture::from(chosen)
        .await
        .map_err(|error| failure(&describe(&error)))?;
    if !confirmed.is_truthy() {
        return Ok(Vec::new());
    }
    let Some(files) = input.files() else {
        return Ok(Vec::new());
    };
    Ok((0..files.length())
        .filter_map(|index| files.get(index))
        .map(|file| PickedFile::Handle(Box::new(InputFile::new(file))))
        .collect())
}

/// `types` as a file input's `accept`: their extensions and MIME types, comma-separated.
fn input_accept(types: &[FileType]) -> String {
    let extensions = types
        .iter()
        .flat_map(|kind| &kind.extensions)
        .map(|extension| format!(".{}", extension.trim_start_matches('.')));
    let mime_types = types
        .iter()
        .flat_map(|kind| kind.mime_types.iter().cloned());
    extensions.chain(mime_types).collect::<Vec<_>>().join(",")
}

/// A file the browser's picker or a launch gave: its handle, and whether the app may write it.
pub(crate) struct SystemFile {
    handle: JsValue,
    name: String,
    writable: bool,
}

impl SystemFile {
    pub(crate) fn new(handle: JsValue, writable: bool) -> SystemFile {
        let name = Reflect::get(&handle, &JsValue::from_str("name"))
            .ok()
            .and_then(|name| name.as_string())
            .unwrap_or_default();
        SystemFile {
            handle,
            name,
            writable,
        }
    }

    /// The file as it is now, which the browser reads afresh at each call.
    async fn file(handle: &JsValue) -> std::io::Result<File> {
        let file = JsFuture::from(method(handle, "getFile", &[]).map_err(io_error)?)
            .await
            .map_err(|error| io_error(failure(&describe(&error))))?;
        file.dyn_into()
            .map_err(|_| io_error(failure("getFile answered no file")))
    }
}

impl FileHandle for SystemFile {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> FileSizeFuture {
        let handle = self.handle.clone();
        Box::pin(async move { Ok(SystemFile::file(&handle).await?.size() as u64) })
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        Box::pin(async move {
            let file = SystemFile::file(&self.handle).await?;
            read_blob(&file, offset, buf).await
        })
    }

    fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>> {
        if !self.writable {
            return None;
        }
        let asked = Object::new();
        set(&asked, "keepExistingData", &JsValue::TRUE);
        let created = method(&self.handle, "createWritable", &[asked.into()]);
        Some(Box::pin(async move {
            let stream = JsFuture::from(created.map_err(io_error)?)
                .await
                .map_err(|error| io_error(failure(&describe(&error))))?;
            Ok(Box::new(SystemWriter {
                stream,
                finished: Cell::new(false),
            }) as Box<dyn FileWriter>)
        }))
    }
}

/// A folder the browser's picker gave: its handle, and whether the app may write in it, which
/// what it opens inherits.
struct SystemFolder {
    handle: JsValue,
    name: String,
    writable: bool,
}

impl SystemFolder {
    fn new(handle: JsValue, writable: bool) -> SystemFolder {
        let name = Reflect::get(&handle, &JsValue::from_str("name"))
            .ok()
            .and_then(|name| name.as_string())
            .unwrap_or_default();
        SystemFolder {
            handle,
            name,
            writable,
        }
    }

    /// The handle `getter` answers for `name`, made first when `create` is set.
    fn child(&self, getter: &str, name: &str, create: bool) -> FileOpenFuture<JsValue> {
        let asked = Object::new();
        set(&asked, "create", &JsValue::from_bool(create));
        let got = method(
            &self.handle,
            getter,
            &[JsValue::from_str(name), asked.into()],
        );
        Box::pin(async move {
            JsFuture::from(got.map_err(io_error)?)
                .await
                .map_err(|error| {
                    if error_name(&error) == "NotFoundError" {
                        std::io::Error::new(std::io::ErrorKind::NotFound, describe(&error))
                    } else {
                        io_error(failure(&describe(&error)))
                    }
                })
        })
    }

    fn file(&self, name: &str, create: bool) -> FileOpenFuture<PickedFile> {
        let child = self.child("getFileHandle", name, create);
        let writable = self.writable;
        Box::pin(async move {
            let file = SystemFile::new(child.await?, writable);
            Ok(PickedFile::Handle(Box::new(file)))
        })
    }

    fn folder(&self, name: &str, create: bool) -> FileOpenFuture<PickedFolder> {
        let child = self.child("getDirectoryHandle", name, create);
        let writable = self.writable;
        Box::pin(async move {
            let folder = SystemFolder::new(child.await?, writable);
            Ok(PickedFolder::Handle(Box::new(folder)))
        })
    }
}

impl FolderHandle for SystemFolder {
    fn name(&self) -> &str {
        &self.name
    }

    /// The folder's `values()`, an asynchronous iterator, read to its end.
    fn entries(&self) -> FolderEntriesFuture {
        let handle = self.handle.clone();
        Box::pin(async move {
            let values: Function = Reflect::get(&handle, &JsValue::from_str("values"))
                .ok()
                .and_then(|values| values.dyn_into().ok())
                .ok_or_else(|| io_error(failure("the folder lists nothing")))?;
            let iterator = values
                .call0(&handle)
                .map_err(|error| io_error(failure(&describe(&error))))?;
            let mut entries = Vec::new();
            loop {
                let step = JsFuture::from(method(&iterator, "next", &[]).map_err(io_error)?)
                    .await
                    .map_err(|error| io_error(failure(&describe(&error))))?;
                if Reflect::get(&step, &JsValue::from_str("done"))
                    .is_ok_and(|done| done.is_truthy())
                {
                    return Ok(entries);
                }
                let entry = Reflect::get(&step, &JsValue::from_str("value"))
                    .map_err(|error| io_error(failure(&describe(&error))))?;
                let text = |key: &str| {
                    Reflect::get(&entry, &JsValue::from_str(key))
                        .ok()
                        .and_then(|value| value.as_string())
                        .unwrap_or_default()
                };
                let kind = if text("kind") == "directory" {
                    EntryKind::Folder
                } else {
                    EntryKind::File
                };
                entries.push(FolderEntry {
                    name: text("name"),
                    kind,
                });
            }
        })
    }

    fn open_file(&self, name: &str) -> FileOpenFuture<PickedFile> {
        self.file(name, false)
    }

    fn open_folder(&self, name: &str) -> FileOpenFuture<PickedFolder> {
        self.folder(name, false)
    }

    fn create_file(&self, name: &str) -> Option<FileOpenFuture<PickedFile>> {
        self.writable.then(|| self.file(name, true))
    }

    fn create_folder(&self, name: &str) -> Option<FileOpenFuture<PickedFolder>> {
        self.writable.then(|| self.folder(name, true))
    }
}

/// A file an `<input type="file">` gave, which the page may only read.
struct InputFile {
    file: File,
    name: String,
}

impl InputFile {
    fn new(file: File) -> InputFile {
        let name = file.name();
        InputFile { file, name }
    }
}

impl FileHandle for InputFile {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> FileSizeFuture {
        let size = self.file.size() as u64;
        Box::pin(std::future::ready(Ok(size)))
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        Box::pin(read_blob(&self.file, offset, buf))
    }

    fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>> {
        None
    }
}

/// Reads `blob` into `buf` from `offset`: the slice of it that fits.
async fn read_blob(blob: &Blob, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
    let size = blob.size() as u64;
    let start = offset.min(size);
    let end = start.saturating_add(buf.len() as u64).min(size);
    let slice = blob
        .slice_with_f64_and_f64(start as f64, end as f64)
        .map_err(|error| io_error(failure(&describe(&error))))?;
    let bytes = JsFuture::from(slice.array_buffer())
        .await
        .map_err(|error| io_error(failure(&describe(&error))))?;
    let bytes = Uint8Array::new(&bytes);
    let read = bytes.length() as usize;
    bytes.copy_to(&mut buf[..read]);
    Ok(read)
}

/// Writing to a picked file through the browser's writable stream, which replaces the file only
/// when it closes, and drops its writes when aborted.
struct SystemWriter {
    stream: JsValue,
    finished: Cell<bool>,
}

impl SystemWriter {
    async fn ask(&self, name: &str, arguments: &[JsValue]) -> std::io::Result<()> {
        JsFuture::from(method(&self.stream, name, arguments).map_err(io_error)?)
            .await
            .map(|_| ())
            .map_err(|error| io_error(failure(&describe(&error))))
    }
}

impl FileWriter for SystemWriter {
    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        let command = Object::new();
        set(&command, "type", &JsValue::from_str("write"));
        set(&command, "position", &JsValue::from_f64(offset as f64));
        set(&command, "data", &Uint8Array::from(bytes));
        Box::pin(async move { self.ask("write", &[command.into()]).await })
    }

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_> {
        Box::pin(async move { self.ask("truncate", &[JsValue::from_f64(len as f64)]).await })
    }

    fn finish(self: Box<Self>) -> FileFinishFuture {
        self.finished.set(true);
        Box::pin(async move { self.ask("close", &[]).await })
    }
}

impl Drop for SystemWriter {
    /// A writer dropped before it finished abandons its writes.
    fn drop(&mut self) {
        if !self.finished.get() {
            let _ = method(&self.stream, "abort", &[]);
        }
    }
}

/// Calls `target.name(arguments)`, which answers a promise.
fn method(target: &JsValue, name: &str, arguments: &[JsValue]) -> Result<Promise, FileDialogError> {
    let function: Function = Reflect::get(target, &JsValue::from_str(name))
        .map_err(|error| failure(&describe(&error)))?
        .dyn_into()
        .map_err(|_| failure(&format!("no {name} here")))?;
    let arguments: Array = arguments.iter().collect();
    function
        .apply(target, &arguments)
        .map_err(|error| failure(&describe(&error)))?
        .dyn_into()
        .map_err(|_| failure(&format!("{name} answered no promise")))
}

fn set(target: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(target, &JsValue::from_str(key), value);
}

fn error_name(error: &JsValue) -> String {
    Reflect::get(error, &JsValue::from_str("name"))
        .ok()
        .and_then(|name| name.as_string())
        .unwrap_or_default()
}

/// What the browser says went wrong: an error's message, or what it threw.
pub(crate) fn describe(error: &JsValue) -> String {
    Reflect::get(error, &JsValue::from_str("message"))
        .ok()
        .and_then(|message| message.as_string())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "the browser refused".to_owned())
}

fn failure(reason: &str) -> FileDialogError {
    FileDialogError::Failed(reason.to_owned())
}

fn io_error(error: FileDialogError) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
