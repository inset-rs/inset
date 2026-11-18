//! A file system in memory: files and folders a test puts there, read whole and through an opened
//! file, listed, renamed and deleted, each answer ready at once. Every clone is the same tree.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use waft_embedder::{
    File, FileKind, FileMetadataFuture, FileOperationFuture, FileReadFuture, FileSizeFuture,
    FileWriteFuture, Files, ListFilesFuture, ListedEntry, Metadata, OpenFileFuture, OpenMode,
    ReadFileFuture, WriteFileFuture,
};

/// A file system in memory, rooted at `/`.
#[derive(Clone, Default)]
pub struct MemoryFiles(Rc<RefCell<Tree>>);

#[derive(Default)]
struct Tree {
    entries: BTreeMap<PathBuf, Entry>,
}

struct Entry {
    kind: FileKind,
    bytes: Vec<u8>,
}

impl MemoryFiles {
    /// An empty tree with a root folder.
    pub fn new() -> MemoryFiles {
        let files = MemoryFiles::default();
        files.0.borrow_mut().entries.insert(
            PathBuf::from("/"),
            Entry {
                kind: FileKind::Folder,
                bytes: Vec::new(),
            },
        );
        files
    }

    /// The same tree, holding `bytes` at `path`, and the folders above it where they lack.
    pub fn with_file(self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> MemoryFiles {
        self.put_file(path.into(), bytes.into());
        self
    }

    /// The same tree, holding an empty folder at `path`.
    pub fn with_folder(self, path: impl Into<PathBuf>) -> MemoryFiles {
        self.make_folder(&path.into());
        self
    }

    /// The bytes at `path`, or `None` where nothing is.
    pub fn file(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        let tree = self.0.borrow();
        let entry = tree.entries.get(path.as_ref())?;
        (entry.kind == FileKind::File).then(|| entry.bytes.clone())
    }

    /// What is at `path`, as the tree holds it.
    pub fn kind(&self, path: impl AsRef<Path>) -> Option<FileKind> {
        self.0
            .borrow()
            .entries
            .get(path.as_ref())
            .map(|entry| entry.kind)
    }

    /// Puts `bytes` at `path`, making it or replacing it.
    pub fn set_file(&self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) {
        self.put_file(path.into(), bytes.into());
    }

    fn put_file(&self, path: PathBuf, bytes: Vec<u8>) {
        self.make_parents(&path);
        self.0.borrow_mut().entries.insert(
            path,
            Entry {
                kind: FileKind::File,
                bytes,
            },
        );
    }

    fn make_folder(&self, path: &Path) {
        self.make_parents(path);
        self.0.borrow_mut().entries.insert(
            path.to_owned(),
            Entry {
                kind: FileKind::Folder,
                bytes: Vec::new(),
            },
        );
    }

    fn make_parents(&self, path: &Path) {
        let mut tree = self.0.borrow_mut();
        for parent in path.ancestors().skip(1) {
            if parent.as_os_str().is_empty() {
                continue;
            }
            tree.entries.entry(parent.to_owned()).or_insert(Entry {
                kind: FileKind::Folder,
                bytes: Vec::new(),
            });
        }
    }

    fn bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        let tree = self.0.borrow();
        match tree.entries.get(path) {
            Some(entry) if entry.kind == FileKind::File => Ok(entry.bytes.clone()),
            Some(_) => Err(io::Error::new(
                io::ErrorKind::IsADirectory,
                "it is a folder",
            )),
            None => Err(not_found(path)),
        }
    }
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{} does not exist", path.display()),
    )
}

impl Files for MemoryFiles {
    fn read(&self, path: &Path) -> ReadFileFuture {
        let bytes = self.bytes(path);
        Box::pin(std::future::ready(bytes))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> WriteFileFuture {
        self.put_file(path.to_owned(), bytes.to_vec());
        Box::pin(std::future::ready(Ok(())))
    }

    fn open(&self, path: &Path, mode: OpenMode) -> OpenFileFuture {
        let mut tree = self.0.borrow_mut();
        let exists = tree
            .entries
            .get(path)
            .is_some_and(|entry| entry.kind == FileKind::File);
        if !exists && !mode.writable() {
            return Box::pin(std::future::ready(Err(not_found(path))));
        }
        if !exists {
            drop(tree);
            self.make_parents(path);
            tree = self.0.borrow_mut();
            tree.entries.insert(
                path.to_owned(),
                Entry {
                    kind: FileKind::File,
                    bytes: Vec::new(),
                },
            );
        }
        if mode.truncates() {
            tree.entries.get_mut(path).unwrap().bytes.clear();
        }
        let cursor = if mode.appends() {
            tree.entries.get(path).unwrap().bytes.len() as u64
        } else {
            0
        };
        drop(tree);
        let file = MemoryFile {
            tree: Rc::clone(&self.0),
            path: path.to_owned(),
            mode,
            cursor,
        };
        Box::pin(std::future::ready(Ok(Box::new(file) as Box<dyn File>)))
    }

    fn metadata(&self, path: &Path) -> FileMetadataFuture {
        let tree = self.0.borrow();
        let metadata = tree.entries.get(path).map(|entry| Metadata {
            kind: entry.kind,
            size: entry.bytes.len() as u64,
            modified: None,
        });
        Box::pin(std::future::ready(metadata.ok_or_else(|| not_found(path))))
    }

    fn list(&self, path: &Path) -> ListFilesFuture {
        let tree = self.0.borrow();
        match tree.entries.get(path) {
            None => return Box::pin(std::future::ready(Err(not_found(path)))),
            Some(entry) if entry.kind != FileKind::Folder => {
                return Box::pin(std::future::ready(Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "it is not a folder",
                ))));
            }
            Some(_) => {}
        }
        let mut listed: Vec<ListedEntry> = tree
            .entries
            .iter()
            .filter(|(entry_path, _)| entry_path.parent() == Some(path))
            .map(|(entry_path, entry)| ListedEntry {
                path: entry_path.clone(),
                name: entry_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: entry.kind,
            })
            .collect();
        listed.sort_by(|left, right| left.name.cmp(&right.name));
        Box::pin(std::future::ready(Ok(listed)))
    }

    fn create_folder(&self, path: &Path) -> FileOperationFuture {
        let tree = self.0.borrow();
        match tree.entries.get(path) {
            Some(entry) if entry.kind != FileKind::Folder => {
                return Box::pin(std::future::ready(Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "a file is there",
                ))));
            }
            Some(_) => return Box::pin(std::future::ready(Ok(()))),
            None => {}
        }
        drop(tree);
        self.make_folder(path);
        Box::pin(std::future::ready(Ok(())))
    }

    fn rename(&self, from: &Path, to: &Path) -> FileOperationFuture {
        let mut tree = self.0.borrow_mut();
        if !tree.entries.contains_key(from) {
            return Box::pin(std::future::ready(Err(not_found(from))));
        }
        let moved: Vec<(PathBuf, PathBuf)> = tree
            .entries
            .keys()
            .filter(|entry_path| entry_path.starts_with(from))
            .map(|entry_path| {
                let rest = entry_path.strip_prefix(from).expect("a path under `from`");
                (entry_path.clone(), to.join(rest))
            })
            .collect();
        for (old, new) in moved {
            let entry = tree.entries.remove(&old).expect("the entry just seen");
            tree.entries.insert(new, entry);
        }
        Box::pin(std::future::ready(Ok(())))
    }

    fn delete(&self, path: &Path) -> FileOperationFuture {
        let mut tree = self.0.borrow_mut();
        if !tree.entries.contains_key(path) {
            return Box::pin(std::future::ready(Err(not_found(path))));
        }
        tree.entries
            .retain(|entry, _| entry.as_path() != path && !entry.starts_with(path));
        Box::pin(std::future::ready(Ok(())))
    }
}

/// A file opened on a [`MemoryFiles`] tree, read and written where the tree holds it.
struct MemoryFile {
    tree: Rc<RefCell<Tree>>,
    path: PathBuf,
    mode: OpenMode,
    cursor: u64,
}

impl MemoryFile {
    fn bytes(&self) -> Vec<u8> {
        self.tree
            .borrow()
            .entries
            .get(&self.path)
            .map(|entry| entry.bytes.clone())
            .unwrap_or_default()
    }

    fn update(&mut self, f: impl FnOnce(&mut Vec<u8>)) {
        if let Some(entry) = self.tree.borrow_mut().entries.get_mut(&self.path) {
            f(&mut entry.bytes);
        }
    }
}

fn refused(mode: OpenMode, reading: bool) -> io::Result<()> {
    if (reading && !mode.readable()) || (!reading && !mode.writable()) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the file was opened for the other use",
        ));
    }
    Ok(())
}

impl File for MemoryFile {
    fn size(&self) -> FileSizeFuture {
        let size = self.bytes().len() as u64;
        Box::pin(std::future::ready(Ok(size)))
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        if let Err(error) = refused(self.mode, true) {
            return Box::pin(std::future::ready(Err(error)));
        }
        let bytes = self.bytes();
        let start = usize::try_from(self.cursor)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let read = (bytes.len() - start).min(buf.len());
        buf[..read].copy_from_slice(&bytes[start..start + read]);
        self.cursor += read as u64;
        Box::pin(std::future::ready(Ok(read)))
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        if let Err(error) = refused(self.mode, true) {
            return Box::pin(std::future::ready(Err(error)));
        }
        let bytes = self.bytes();
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let read = (bytes.len() - start).min(buf.len());
        buf[..read].copy_from_slice(&bytes[start..start + read]);
        Box::pin(std::future::ready(Ok(read)))
    }

    fn write<'a>(&'a mut self, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        if let Err(error) = refused(self.mode, false) {
            return Box::pin(std::future::ready(Err(error)));
        }
        let at = self.cursor;
        self.update(|whole| write_at(whole, at, bytes));
        self.cursor = at + bytes.len() as u64;
        Box::pin(std::future::ready(Ok(())))
    }

    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        if let Err(error) = refused(self.mode, false) {
            return Box::pin(std::future::ready(Err(error)));
        }
        self.update(|whole| write_at(whole, offset, bytes));
        Box::pin(std::future::ready(Ok(())))
    }

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_> {
        if let Err(error) = refused(self.mode, false) {
            return Box::pin(std::future::ready(Err(error)));
        }
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        self.update(|whole| whole.resize(len, 0));
        Box::pin(std::future::ready(Ok(())))
    }

    fn flush(&mut self) -> FileWriteFuture<'_> {
        Box::pin(std::future::ready(Ok(())))
    }
}

fn write_at(whole: &mut Vec<u8>, offset: u64, bytes: &[u8]) {
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    let end = start.saturating_add(bytes.len());
    if whole.len() < end {
        whole.resize(end, 0);
    }
    whole[start..end].copy_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    #[test]
    fn a_file_written_is_read_whole_and_listed() {
        let files = MemoryFiles::new().with_folder("/photos");
        now(files.write(Path::new("/photos/a.png"), b"pixels")).unwrap();
        assert_eq!(files.file("/photos/a.png").unwrap(), b"pixels");
        assert_eq!(
            now(files.read(Path::new("/photos/a.png"))).unwrap(),
            b"pixels"
        );
        let listed = now(files.list(Path::new("/photos"))).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "a.png");
        assert_eq!(
            now(files.metadata(Path::new("/photos/a.png")))
                .unwrap()
                .size,
            6
        );
    }

    #[test]
    fn an_opened_file_reads_and_writes_at_an_offset() {
        let files = MemoryFiles::new().with_file("/a", b"12345");
        let mut file = now(files.open(Path::new("/a"), OpenMode::Append)).unwrap();
        let mut buf = [0u8; 2];
        assert_eq!(now(file.read_at(2, &mut buf)).unwrap(), 2);
        assert_eq!(&buf, b"34");
        now(file.write_at(1, b"ab")).unwrap();
        assert_eq!(files.file("/a").unwrap(), b"1ab45");
        now(file.set_len(4)).unwrap();
        assert_eq!(files.file("/a").unwrap(), b"1ab4");
    }

    #[test]
    fn a_read_only_open_refuses_writes_and_a_missing_file_refuses_reading() {
        let files = MemoryFiles::new().with_file("/a", b"x");
        let mut file = now(files.open(Path::new("/a"), OpenMode::Read)).unwrap();
        assert!(now(file.write(b"y")).is_err());
        assert!(now(files.open(Path::new("/none"), OpenMode::Read)).is_err());
    }

    #[test]
    fn a_folder_made_with_its_parents_is_listed() {
        let files = MemoryFiles::new();
        now(files.create_folder(Path::new("/photos/2026"))).unwrap();
        assert_eq!(files.kind("/photos/2026"), Some(FileKind::Folder));
        let listed = now(files.list(Path::new("/photos"))).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "2026");
        assert_eq!(
            now(files.metadata(Path::new("/photos/2026"))).unwrap().kind,
            FileKind::Folder
        );
        now(files.create_folder(Path::new("/photos/2026"))).unwrap();
        files.set_file("/photos/2026/a.png", b"x");
        assert!(now(files.create_folder(Path::new("/photos/2026/a.png"))).is_err());
    }

    #[test]
    fn a_folder_renamed_and_deleted_takes_what_is_in_it() {
        let files = MemoryFiles::new()
            .with_file("/one/a", b"a")
            .with_file("/one/b", b"b");
        now(files.rename(Path::new("/one"), Path::new("/two"))).unwrap();
        assert_eq!(files.file("/two/a").unwrap(), b"a");
        now(files.delete(Path::new("/two"))).unwrap();
        assert_eq!(files.kind("/two"), None);
        assert_eq!(files.kind("/two/a"), None);
    }
}
