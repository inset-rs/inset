//! Files on macOS and iOS: `std`'s file system, run on the platform's [`Dispatcher`] threads so
//! that a read of a large file never stops the app's frame. Every operation answers a future
//! the app awaits; the work runs where the host runs work off the main thread.
//!
//! Paths are the system's own, and a path a pick gave is a path like any other.

use std::fs::{self, File as StdFile, OpenOptions};
use std::future::Future;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use waft_embedder::{
    Dispatcher, File, FileKind, FileMetadataFuture, FileOperationFuture, FileReadFuture,
    FileSizeFuture, FileWriteFuture, Files, ListFilesFuture, ListedEntry, Metadata, OpenFileFuture,
    OpenMode, ReadFileFuture, WriteFileFuture,
};

/// The system's file system, on `dispatcher`'s threads: a read, a write, a listing and the rest
/// each run there and answer the app with a future.
pub struct AppleFiles {
    dispatcher: Arc<dyn Dispatcher>,
}

impl AppleFiles {
    /// Files whose blocking work runs on `dispatcher`'s threads.
    pub fn new(dispatcher: Arc<dyn Dispatcher>) -> AppleFiles {
        AppleFiles { dispatcher }
    }
}

impl Files for AppleFiles {
    fn read(&self, path: &Path) -> ReadFileFuture {
        let path = path.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || fs::read(path))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> WriteFileFuture {
        let path = path.to_owned();
        let bytes = bytes.to_vec();
        off_main(Arc::clone(&self.dispatcher), move || fs::write(path, bytes))
    }

    fn open(&self, path: &Path, mode: OpenMode) -> OpenFileFuture {
        let path = path.to_owned();
        let dispatcher = Arc::clone(&self.dispatcher);
        Box::pin(async move {
            let state = off_main(dispatcher.clone(), move || open_file(&path, mode)).await?;
            Ok(Box::new(OpenedFile {
                state: Arc::new(Mutex::new(state)),
                dispatcher,
            }) as Box<dyn File>)
        })
    }

    fn metadata(&self, path: &Path) -> FileMetadataFuture {
        let path = path.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || metadata_of(&path))
    }

    fn list(&self, path: &Path) -> ListFilesFuture {
        let path = path.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || list_in(&path))
    }

    fn create_folder(&self, path: &Path) -> FileOperationFuture {
        let path = path.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || {
            fs::create_dir_all(path)
        })
    }

    fn rename(&self, from: &Path, to: &Path) -> FileOperationFuture {
        let from = from.to_owned();
        let to = to.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || fs::rename(from, to))
    }

    fn delete(&self, path: &Path) -> FileOperationFuture {
        let path = path.to_owned();
        off_main(Arc::clone(&self.dispatcher), move || delete_at(&path))
    }
}

/// An opened file on the system: its handle, and where its cursor is.
struct OpenedFile {
    state: Arc<Mutex<OpenedState>>,
    dispatcher: Arc<dyn Dispatcher>,
}

struct OpenedState {
    file: StdFile,
    cursor: u64,
}

fn lock(state: &Mutex<OpenedState>) -> MutexGuard<'_, OpenedState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl File for OpenedFile {
    fn size(&self) -> FileSizeFuture {
        let state = Arc::clone(&self.state);
        off_main(Arc::clone(&self.dispatcher), move || {
            let state = lock(&state);
            Ok(state.file.metadata()?.len())
        })
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        let state = Arc::clone(&self.state);
        let dispatcher = Arc::clone(&self.dispatcher);
        let offset = lock(&self.state).cursor;
        Box::pin(async move {
            let read = read_from(dispatcher, Arc::clone(&state), offset, buf.len()).await?;
            buf[..read.len()].copy_from_slice(&read);
            lock(&self.state).cursor = offset + read.len() as u64;
            Ok(read.len())
        })
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        let dispatcher = Arc::clone(&self.dispatcher);
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let read = read_from(dispatcher, state, offset, buf.len()).await?;
            buf[..read.len()].copy_from_slice(&read);
            Ok(read.len())
        })
    }

    fn write<'a>(&'a mut self, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        let dispatcher = Arc::clone(&self.dispatcher);
        let state = Arc::clone(&self.state);
        let offset = lock(&self.state).cursor;
        let bytes = bytes.to_vec();
        let len = bytes.len() as u64;
        Box::pin(async move {
            write_to(dispatcher, Arc::clone(&state), offset, bytes).await?;
            lock(&self.state).cursor = offset + len;
            Ok(())
        })
    }

    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        write_to(
            Arc::clone(&self.dispatcher),
            Arc::clone(&self.state),
            offset,
            bytes.to_vec(),
        )
    }

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_> {
        let state = Arc::clone(&self.state);
        off_main(Arc::clone(&self.dispatcher), move || {
            lock(&state).file.set_len(len)
        })
    }

    fn flush(&mut self) -> FileWriteFuture<'_> {
        let state = Arc::clone(&self.state);
        off_main(Arc::clone(&self.dispatcher), move || {
            lock(&state).file.sync_all()
        })
    }
}

fn open_file(path: &Path, mode: OpenMode) -> io::Result<OpenedState> {
    let mut options = OpenOptions::new();
    options.read(mode.readable()).write(mode.writable());
    options.truncate(mode.truncates());
    let file = options.open(path)?;
    let cursor = if mode.appends() {
        file.metadata()?.len()
    } else {
        0
    };
    Ok(OpenedState { file, cursor })
}

fn read_from(
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<Mutex<OpenedState>>,
    offset: u64,
    len: usize,
) -> Pin<Box<dyn Future<Output = io::Result<Vec<u8>>>>> {
    off_main(dispatcher, move || {
        let state = lock(&state);
        let mut bytes = vec![0; len];
        let read = state.file.read_at(&mut bytes, offset)?;
        bytes.truncate(read);
        Ok(bytes)
    })
}

fn write_to(
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<Mutex<OpenedState>>,
    offset: u64,
    bytes: Vec<u8>,
) -> FileWriteFuture<'static> {
    off_main(dispatcher, move || {
        lock(&state).file.write_all_at(&bytes, offset)
    })
}

fn metadata_of(path: &Path) -> io::Result<Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    let kind = if metadata.is_dir() {
        FileKind::Folder
    } else if metadata.is_file() {
        FileKind::File
    } else if metadata.file_type().is_symlink() {
        FileKind::Link
    } else {
        FileKind::Other
    };
    Ok(Metadata {
        kind,
        size: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn list_in(path: &Path) -> io::Result<Vec<ListedEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = path.join(entry.file_name());
        let kind = match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => FileKind::Folder,
            Ok(file_type) if file_type.is_file() => FileKind::File,
            Ok(file_type) if file_type.is_symlink() => FileKind::Link,
            _ => FileKind::Other,
        };
        entries.push(ListedEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path,
            kind,
        });
    }
    Ok(entries)
}

fn delete_at(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Runs `work` on `dispatcher`'s threads and answers its result, or why it never ran.
fn off_main<T: Send + 'static>(
    dispatcher: Arc<dyn Dispatcher>,
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> Pin<Box<dyn Future<Output = io::Result<T>>>> {
    Box::pin(async move {
        let (given, answered) = crate::answer::answer::<io::Result<T>>(Err(io::Error::other(
            "the file work never ran",
        )));
        dispatcher.dispatch(Box::new(move || given.give(work())));
        answered.await
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use waft_embedder::InertDispatcher;

    fn files() -> AppleFiles {
        AppleFiles::new(Arc::new(InertDispatcher))
    }

    fn now<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("waft-files-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn a_file_written_is_read_whole_and_listed() {
        let root = temp("whole");
        let file = root.join("a.png");
        let files = files();
        now(files.write(&file, b"pixels")).unwrap();
        assert_eq!(now(files.read(&file)).unwrap(), b"pixels");
        let listed = now(files.list(&root)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "a.png");
        assert_eq!(now(files.metadata(&file)).unwrap().size, 6);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_opened_file_reads_and_writes_at_an_offset() {
        let root = temp("opened");
        let path = root.join("a");
        std::fs::write(&path, b"12345").unwrap();
        let files = files();
        let mut file = now(files.open(&path, OpenMode::Append)).unwrap();
        let mut buf = [0u8; 2];
        assert_eq!(now(file.read_at(2, &mut buf)).unwrap(), 2);
        assert_eq!(&buf, b"34");
        now(file.write_at(1, b"ab")).unwrap();
        assert_eq!(now(files.read(&path)).unwrap(), b"1ab45");
        now(file.set_len(4)).unwrap();
        assert_eq!(now(files.read(&path)).unwrap(), b"1ab4");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_folder_made_with_its_parents_is_a_folder() {
        let root = temp("made");
        let folder = root.join("photos/2026");
        let files = files();
        now(files.create_folder(&folder)).unwrap();
        assert!(folder.is_dir());
        assert_eq!(now(files.metadata(&folder)).unwrap().kind, FileKind::Folder);
        now(files.create_folder(&folder)).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_folder_renamed_and_deleted_takes_what_is_in_it() {
        let root = temp("folder");
        std::fs::create_dir_all(root.join("one")).unwrap();
        std::fs::write(root.join("one/a"), b"a").unwrap();
        let files = files();
        now(files.rename(&root.join("one"), &root.join("two"))).unwrap();
        assert_eq!(now(files.read(&root.join("two/a"))).unwrap(), b"a");
        now(files.delete(&root.join("two"))).unwrap();
        assert!(!root.join("two").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
