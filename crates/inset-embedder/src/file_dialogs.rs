//! The system's file dialogs: open a file, open files, open a folder, save a file, each the
//! system's own panel. Not a dart:ui file: Flutter reaches them through its `file_selector`
//! package, whose functions these follow, one per kind (`openFile`, `openFiles`,
//! `getDirectoryPath`, `getSaveLocation`), each with options of its own and answering what its
//! kind means.
//!
//! A pick answers a path where the system gives one (macOS, iOS, Windows, Linux) and the
//! system's own handle where it gives none, as Flutter's `XFile` carries a path or a blob: the
//! web's file handle, and Android's document, open as a handle where it seeks and as a stream
//! where it is a pipe ([`PickedFile`]). A pick grants reading only, unless the app asks to write
//! ([`OpenOptions::writable`]); a save grants writing to the one file chosen, which may not exist
//! yet. A folder pick answers a path where the system gives one, and the system's folder handle
//! where it gives none, the web's directory handle and Android's document tree, which lists the
//! folder and opens and, when writable, makes what is in it ([`PickedFolder`]).
//!
//! The platform answers `None` where its system has no file dialogs, and a dialog the system
//! could not show is answered as a [`FileDialogError`]. Every answer comes through a future, once
//! the user has chosen or cancelled: a cancelled dialog answers `None`, or no files.

use std::fmt;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use crate::ViewId;

/// The system's file dialogs. `parent` is the view whose window a dialog belongs to, which the
/// system shows it on, as a sheet where it has them (macOS); without one it stands alone.
pub trait FileDialogs {
    /// Asks for one file to open; `None` if the user cancelled.
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>>;

    /// Asks for files to open; none if the user cancelled.
    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>>;

    /// Asks for a folder; `None` if the user cancelled.
    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>>;

    /// Asks where to save a file, which the pick may then write; `None` if the user cancelled.
    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>>;
}

/// What a file dialog answers, once the user has chosen or cancelled.
pub type FileDialogFuture<T> = Pin<Box<dyn Future<Output = Result<T, FileDialogError>>>>;

/// Why a file dialog answered nothing the user chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDialogError {
    /// The system did not show the dialog, or failed to answer it.
    Failed(String),
}

impl fmt::Display for FileDialogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileDialogError::Failed(reason) => write!(f, "the file dialog failed: {reason}"),
        }
    }
}

impl std::error::Error for FileDialogError {}

/// The kinds of file a dialog offers, as `file_selector`'s `XTypeGroup`: a system takes the
/// extensions or the MIME types it filters by, and shows the label where it shows one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileType {
    pub label: String,
    /// Without the dot: `txt`.
    pub extensions: Vec<String>,
    pub mime_types: Vec<String>,
}

/// How to ask for files to open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenOptions {
    /// The kinds the dialog offers; any file when there are none.
    pub types: Vec<FileType>,
    /// Where the dialog starts, where the system takes a folder.
    pub initial_folder: Option<PathBuf>,
    /// The confirming button's label, where the system takes one.
    pub confirm_label: Option<String>,
    /// Whether the app may write the files picked. Most systems grant writing to every pick, so
    /// this only matters where a pick grants reading alone (Android, the web).
    pub writable: bool,
}

/// How to ask for a folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FolderOptions {
    pub initial_folder: Option<PathBuf>,
    pub confirm_label: Option<String>,
    /// Whether the app may write in the folder picked, where a pick grants reading alone.
    pub writable: bool,
}

/// How to ask where to save a file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveOptions {
    pub types: Vec<FileType>,
    pub initial_folder: Option<PathBuf>,
    /// The file name the dialog suggests.
    pub suggested_name: Option<String>,
    pub confirm_label: Option<String>,
}

/// A file the user picked, as the system gives it.
pub enum PickedFile {
    /// macOS, iOS, Windows and Linux: a path the app opens as any file.
    Path(PathBuf),
    /// The web, and Android where the document seeks, as it always does when writable.
    Handle(Box<dyn FileHandle>),
    /// Android where the document is a pipe: read once, start to end. Read-only picks only.
    Stream(Box<dyn FileStream>),
}

impl fmt::Debug for PickedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PickedFile::Path(path) => f.debug_tuple("Path").field(path).finish(),
            PickedFile::Handle(handle) => f.debug_tuple("Handle").field(&handle.name()).finish(),
            PickedFile::Stream(stream) => f.debug_tuple("Stream").field(&stream.name()).finish(),
        }
    }
}

/// A folder the user picked, as the system gives it.
pub enum PickedFolder {
    /// macOS, iOS, Windows and Linux: a path the app reads and writes in as any folder.
    Path(PathBuf),
    /// The web's directory handle, and Android's document tree.
    Handle(Box<dyn FolderHandle>),
}

impl fmt::Debug for PickedFolder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PickedFolder::Path(path) => f.debug_tuple("Path").field(path).finish(),
            PickedFolder::Handle(handle) => f.debug_tuple("Handle").field(&handle.name()).finish(),
        }
    }
}

/// A picked folder the system gives no path for: what is in it, opened by name, and, when the
/// pick is writable, files and folders made in it. What it opens is writable when it is.
pub trait FolderHandle {
    /// The folder's name, which the user sees.
    fn name(&self) -> &str;

    /// What is in the folder, in no particular order.
    fn entries(&self) -> FolderEntriesFuture;

    /// The file `name` in the folder.
    fn open_file(&self, name: &str) -> FileOpenFuture<PickedFile>;

    /// The folder `name` in the folder.
    fn open_folder(&self, name: &str) -> FileOpenFuture<PickedFolder>;

    /// Makes an empty file `name` in the folder; `None` unless the pick is writable.
    fn create_file(&self, name: &str) -> Option<FileOpenFuture<PickedFile>>;

    /// Makes a folder `name` in the folder; `None` unless the pick is writable.
    fn create_folder(&self, name: &str) -> Option<FileOpenFuture<PickedFolder>>;
}

/// One thing in a picked folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderEntry {
    pub name: String,
    pub kind: EntryKind,
}

/// Whether a folder's entry is a file or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Folder,
}

/// What a folder's listing answers.
pub type FolderEntriesFuture = Pin<Box<dyn Future<Output = io::Result<Vec<FolderEntry>>>>>;

/// A picked file the system gives no path for, read and written at any offset.
pub trait FileHandle {
    /// The file's name, which the user sees.
    fn name(&self) -> &str;

    fn size(&self) -> FileSizeFuture;

    /// Reads into `buf` from `offset`; answers how many bytes it read, `0` at the end.
    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a>;

    /// Opens the file to write; `None` unless the pick is writable.
    fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>>;
}

/// Writing to a picked file.
pub trait FileWriter {
    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a>;

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_>;

    /// Ends the writing: on the web the file is replaced only now, and a writer dropped before
    /// abandons it; on Android the writes landed in place as they were made.
    fn finish(self: Box<Self>) -> FileFinishFuture;
}

/// A picked file that can only be read through, once, start to end.
pub trait FileStream {
    fn name(&self) -> &str;

    /// Reads the next bytes into `buf`; answers how many, `0` at the end.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> FileReadFuture<'a>;
}

pub type FileSizeFuture = Pin<Box<dyn Future<Output = io::Result<u64>>>>;
pub type FileReadFuture<'a> = Pin<Box<dyn Future<Output = io::Result<usize>> + 'a>>;
pub type FileOpenFuture<T> = Pin<Box<dyn Future<Output = io::Result<T>>>>;
pub type FileWriteFuture<'a> = Pin<Box<dyn Future<Output = io::Result<()>> + 'a>>;
pub type FileFinishFuture = Pin<Box<dyn Future<Output = io::Result<()>>>>;

/// How much a whole read asks for at a time.
const READ_CHUNK: usize = 64 * 1024;

impl PickedFile {
    /// The whole file, as Flutter's `XFile.readAsBytes` reads it: a path is read on a thread of
    /// its own, a handle from its start to its end, and a stream through, which uses it up.
    pub fn read_all(&mut self) -> Pin<Box<dyn Future<Output = io::Result<Vec<u8>>> + '_>> {
        match self {
            PickedFile::Path(path) => Box::pin(read_path(path.clone())),
            PickedFile::Handle(handle) => Box::pin(read_handle(&**handle)),
            PickedFile::Stream(stream) => Box::pin(read_stream(&mut **stream)),
        }
    }
}

async fn read_path(path: PathBuf) -> io::Result<Vec<u8>> {
    let (answer, read) = one_shot();
    std::thread::Builder::new()
        .name("inset-read-picked-file".to_owned())
        .spawn(move || answer.give(std::fs::read(path)))?;
    read.await
}

async fn read_handle(handle: &dyn FileHandle) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(usize::try_from(handle.size().await?).unwrap_or(0));
    let mut chunk = vec![0; READ_CHUNK];
    loop {
        let read = handle.read_at(bytes.len() as u64, &mut chunk).await?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

async fn read_stream(stream: &mut dyn FileStream) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = vec![0; READ_CHUNK];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

/// An answer given once, from any thread, and the future that waits for it.
fn one_shot<T>() -> (Answer<T>, Answered<T>) {
    let shared = Arc::new(Mutex::new((None, None)));
    (Answer(Arc::clone(&shared)), Answered(shared))
}

type Shared<T> = Arc<Mutex<(Option<io::Result<T>>, Option<Waker>)>>;

struct Answer<T>(Shared<T>);

impl<T> Answer<T> {
    fn give(self, value: io::Result<T>) {
        let waker = {
            let mut shared = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            shared.0 = Some(value);
            shared.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct Answered<T>(Shared<T>);

impl<T> Future for Answered<T> {
    type Output = io::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<T>> {
        let mut shared = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match shared.0.take() {
            Some(value) => Poll::Ready(value),
            None if Arc::strong_count(&self.0) == 1 => {
                Poll::Ready(Err(io::Error::other("the read ended without an answer")))
            }
            None => {
                shared.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// A handle over bytes in memory, read a few at a time.
    struct Bytes(RefCell<Vec<u8>>);

    impl FileHandle for Bytes {
        fn name(&self) -> &str {
            "bytes"
        }

        fn size(&self) -> FileSizeFuture {
            let size = self.0.borrow().len() as u64;
            Box::pin(async move { Ok(size) })
        }

        fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
            let bytes = self.0.borrow();
            let start = (offset as usize).min(bytes.len());
            let read = (bytes.len() - start).min(buf.len()).min(3);
            buf[..read].copy_from_slice(&bytes[start..start + read]);
            Box::pin(async move { Ok(read) })
        }

        fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>> {
            None
        }
    }

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_handle_is_read_whole_in_pieces() {
        let mut picked = PickedFile::Handle(Box::new(Bytes(RefCell::new(b"hello, file".to_vec()))));
        assert_eq!(now(picked.read_all()).unwrap(), b"hello, file");
    }

    #[test]
    fn a_path_is_read_whole_on_a_thread() {
        let path = std::env::temp_dir().join(format!("inset-read-all-{}", std::process::id()));
        std::fs::write(&path, b"on disk").unwrap();
        let mut picked = PickedFile::Path(path.clone());
        assert_eq!(now(picked.read_all()).unwrap(), b"on disk");
        std::fs::remove_file(path).unwrap();
        let mut missing = PickedFile::Path(PathBuf::from("/no/such/inset/file"));
        assert!(now(missing.read_all()).is_err());
    }
}
