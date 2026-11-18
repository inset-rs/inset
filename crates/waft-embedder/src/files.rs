//! A host's file system: a path read whole, written whole, opened to read and write at an
//! offset and streamed, its metadata, a folder listed, and paths renamed and deleted. The shape
//! is dart:io's `File` and `RandomAccessFile`, with Rust names: a path names a file here, and an
//! opened one is a [`File`] the app reads through a future, as a picked file's handle is read.
//!
//! A platform answers [`Platform::files`](crate::Platform::files) with its file system, or `None`
//! where it has none, as a web page has none and a WebAssembly component has the one its host
//! grants. Apple's hosts run `std` on their own threads; a platform whose system has asynchronous
//! file IO awaits the system's.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;

pub use crate::file_dialogs::{FileReadFuture, FileSizeFuture, FileWriteFuture};

/// The host's file system.
pub trait Files {
    /// The whole file at `path`, as Flutter's `XFile.readAsBytes` reads it.
    fn read(&self, path: &Path) -> ReadFileFuture;

    /// Writes `bytes` as the whole file at `path`, making it or replacing it.
    fn write(&self, path: &Path, bytes: &[u8]) -> WriteFileFuture;

    /// Opens `path` as `mode` says; the file reads and writes at an offset and streams.
    fn open(&self, path: &Path, mode: OpenMode) -> OpenFileFuture;

    /// What the file or folder at `path` is.
    fn metadata(&self, path: &Path) -> FileMetadataFuture;

    /// What is in the folder at `path`, in no particular order.
    fn list(&self, path: &Path) -> ListFilesFuture;

    /// Renames `from` to `to`, which may be in another folder.
    fn rename(&self, from: &Path, to: &Path) -> FileOperationFuture;

    /// Deletes the file or folder at `path`. A folder is deleted with what is in it, as Dart's
    /// `Directory.delete(recursive: true)` on macOS and Linux does; a system that refuses a
    /// non-empty folder answers that refusal.
    fn delete(&self, path: &Path) -> FileOperationFuture;
}

/// A file opened through [`Files::open`], read and written at an offset and streamed.
///
/// The cursor methods advance where the file reads and writes next; the offset methods leave it
/// alone. A platform whose file system has no cursor of its own keeps one here.
pub trait File {
    /// The file's length in bytes.
    fn size(&self) -> FileSizeFuture;

    /// Reads into `buf` from the cursor, advancing it by what it read; answers how many bytes,
    /// `0` at the end.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> FileReadFuture<'a>;

    /// Reads into `buf` from `offset`, leaving the cursor alone.
    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a>;

    /// Writes `bytes` at the cursor, advancing it by their length.
    fn write<'a>(&'a mut self, bytes: &'a [u8]) -> FileWriteFuture<'a>;

    /// Writes `bytes` at `offset`, leaving the cursor alone.
    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a>;

    /// Cuts the file short at `len`, or fills it with zeroes to that length.
    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_>;

    /// Hands what was written to the system, so that it survives the process that wrote it.
    fn flush(&mut self) -> FileWriteFuture<'_>;
}

/// Dart `FileMode`: what an opened file may do, and where it starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OpenMode {
    /// Read only.
    #[default]
    Read,
    /// Read and write, cut short at the start.
    Write,
    /// Read and write, starting at the end.
    Append,
    /// Write only, cut short at the start.
    WriteOnly,
    /// Write only, starting at the end.
    WriteOnlyAppend,
}

impl OpenMode {
    /// Whether the file may be read.
    pub fn readable(self) -> bool {
        matches!(self, OpenMode::Read | OpenMode::Write | OpenMode::Append)
    }

    /// Whether the file may be written.
    pub fn writable(self) -> bool {
        !matches!(self, OpenMode::Read)
    }

    /// Whether the file starts at its end, so that the first write appends.
    pub fn appends(self) -> bool {
        matches!(self, OpenMode::Append | OpenMode::WriteOnlyAppend)
    }

    /// Whether opening cuts the file short.
    pub fn truncates(self) -> bool {
        matches!(self, OpenMode::Write | OpenMode::WriteOnly)
    }
}

/// What a path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// A file.
    File,
    /// A folder.
    Folder,
    /// A symbolic link, which a listing reports as itself where the system does.
    Link,
    /// A device, a socket, a pipe, or something else this interface does not name.
    Other,
}

/// What the file or folder at a path is, as Dart's `FileStat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// A file, a folder, a link or something else.
    pub kind: FileKind,
    /// The file's length in bytes; zero for a folder or a link.
    pub size: u64,
    /// When the system last changed it, where the system says.
    pub modified: Option<std::time::SystemTime>,
}

/// One thing in a folder, as Dart's `FileSystemEntity`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedEntry {
    /// The entry's full path, as the folder's was given, joined with its name.
    pub path: PathBuf,
    /// The entry's name alone.
    pub name: String,
    /// A file, a folder, a link or something else.
    pub kind: FileKind,
}

/// A whole file read.
pub type ReadFileFuture = Pin<Box<dyn Future<Output = io::Result<Vec<u8>>>>>;

/// A whole file written.
pub type WriteFileFuture = Pin<Box<dyn Future<Output = io::Result<()>>>>;

/// A file opened.
pub type OpenFileFuture = Pin<Box<dyn Future<Output = io::Result<Box<dyn File>>>>>;

/// What a path is.
pub type FileMetadataFuture = Pin<Box<dyn Future<Output = io::Result<Metadata>>>>;

/// What a folder holds.
pub type ListFilesFuture = Pin<Box<dyn Future<Output = io::Result<Vec<ListedEntry>>>>>;

/// A rename or a delete.
pub type FileOperationFuture = Pin<Box<dyn Future<Output = io::Result<()>>>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_says_what_an_opened_file_may_do() {
        assert!(OpenMode::Read.readable());
        assert!(!OpenMode::Read.writable());
        assert!(!OpenMode::Read.truncates());
        assert!(OpenMode::Write.readable());
        assert!(OpenMode::Write.writable());
        assert!(OpenMode::Write.truncates());
        assert!(!OpenMode::Write.appends());
        assert!(OpenMode::Append.appends());
        assert!(!OpenMode::Append.truncates());
        assert!(!OpenMode::WriteOnly.readable());
        assert!(OpenMode::WriteOnlyAppend.appends());
        assert_eq!(OpenMode::default(), OpenMode::Read);
    }

    #[test]
    fn metadata_of_a_file_names_it() {
        let metadata = Metadata {
            kind: FileKind::File,
            size: 12,
            modified: None,
        };
        assert_eq!(metadata.kind, FileKind::File);
        assert_eq!(metadata.size, 12);
    }
}
