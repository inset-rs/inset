//! The system's file dialogs in memory, as a host shows none: each answers at once what the test
//! gave, and keeps what it was asked; and a file in memory, given as a handle, for a pick the
//! system gives no path for.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use inset_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FileFinishFuture, FileHandle, FileOpenFuture,
    FileReadFuture, FileSizeFuture, FileWriteFuture, FileWriter, FolderOptions, OpenOptions,
    PickedFile, SaveOptions, ViewId,
};

/// File dialogs that answer what the test gave, and keep each dialog asked for. Until the test
/// gives an answer, every dialog is cancelled. Every clone is the same dialogs.
#[derive(Clone, Default)]
pub struct MockFileDialogs(Rc<DialogsState>);

#[derive(Default)]
struct DialogsState {
    asked: RefCell<Vec<AskedDialog>>,
    answer: RefCell<Answer>,
}

#[derive(Default)]
enum Answer {
    #[default]
    Cancelled,
    Paths(Vec<PathBuf>),
    Picks(Rc<dyn Fn() -> Vec<PickedFile>>),
    Failure(String),
}

/// A dialog the app asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AskedDialog {
    OpenFile(Option<ViewId>, OpenOptions),
    OpenFiles(Option<ViewId>, OpenOptions),
    OpenFolder(Option<ViewId>, FolderOptions),
    SaveFile(Option<ViewId>, SaveOptions),
}

impl MockFileDialogs {
    pub fn new() -> MockFileDialogs {
        MockFileDialogs::default()
    }

    /// Answers every dialog with `paths`: one file or folder the first of them, files all.
    pub fn answering_paths(self, paths: Vec<PathBuf>) -> MockFileDialogs {
        *self.0.answer.borrow_mut() = Answer::Paths(paths);
        self
    }

    /// Answers every file dialog with what `picks` makes: one file the first; a folder dialog is
    /// cancelled.
    pub fn answering(self, picks: impl Fn() -> Vec<PickedFile> + 'static) -> MockFileDialogs {
        *self.0.answer.borrow_mut() = Answer::Picks(Rc::new(picks));
        self
    }

    /// Fails every dialog with `reason`, as a system that did not show it.
    pub fn failing(self, reason: impl Into<String>) -> MockFileDialogs {
        *self.0.answer.borrow_mut() = Answer::Failure(reason.into());
        self
    }

    /// The dialogs asked for, in order.
    pub fn asked(&self) -> Vec<AskedDialog> {
        self.0.asked.borrow().clone()
    }

    /// Keeps `asked`, and answers the files the test gave.
    fn files(&self, asked: AskedDialog) -> Result<Vec<PickedFile>, FileDialogError> {
        self.0.asked.borrow_mut().push(asked);
        match &*self.0.answer.borrow() {
            Answer::Cancelled => Ok(Vec::new()),
            Answer::Paths(paths) => Ok(paths.iter().cloned().map(PickedFile::Path).collect()),
            Answer::Picks(picks) => Ok(picks()),
            Answer::Failure(reason) => Err(FileDialogError::Failed(reason.clone())),
        }
    }
}

fn answered<T: 'static>(answer: Result<T, FileDialogError>) -> FileDialogFuture<T> {
    Box::pin(std::future::ready(answer))
}

impl FileDialogs for MockFileDialogs {
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let files = self.files(AskedDialog::OpenFile(parent, options));
        answered(files.map(|files| files.into_iter().next()))
    }

    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        answered(self.files(AskedDialog::OpenFiles(parent, options)))
    }

    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PathBuf>> {
        let files = self.files(AskedDialog::OpenFolder(parent, options));
        answered(files.map(|files| {
            files.into_iter().find_map(|file| match file {
                PickedFile::Path(path) => Some(path),
                PickedFile::Handle(_) | PickedFile::Stream(_) => None,
            })
        }))
    }

    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let files = self.files(AskedDialog::SaveFile(parent, options));
        answered(files.map(|files| files.into_iter().next()))
    }
}

/// A file in memory, given as a handle. When writable, a writer's writes land only once it
/// finishes, and a writer dropped before abandons them, as the web's do. Every clone is the same
/// file.
#[derive(Clone)]
pub struct MemoryFile(Rc<MemoryFileState>);

struct MemoryFileState {
    name: String,
    bytes: RefCell<Vec<u8>>,
    writable: Cell<bool>,
}

impl MemoryFile {
    /// A read-only file named `name` holding `bytes`.
    pub fn new(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> MemoryFile {
        MemoryFile(Rc::new(MemoryFileState {
            name: name.into(),
            bytes: RefCell::new(bytes.into()),
            writable: Cell::new(false),
        }))
    }

    /// The same file, writable.
    pub fn writable(self) -> MemoryFile {
        self.0.writable.set(true);
        self
    }

    /// What the file holds now.
    pub fn contents(&self) -> Vec<u8> {
        self.0.bytes.borrow().clone()
    }

    /// The file as a pick.
    pub fn picked(&self) -> PickedFile {
        PickedFile::Handle(Box::new(self.clone()))
    }
}

impl FileHandle for MemoryFile {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn size(&self) -> FileSizeFuture {
        let size = self.0.bytes.borrow().len() as u64;
        Box::pin(std::future::ready(Ok(size)))
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        let bytes = self.0.bytes.borrow();
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let read = (bytes.len() - start).min(buf.len());
        buf[..read].copy_from_slice(&bytes[start..start + read]);
        Box::pin(std::future::ready(Ok(read)))
    }

    fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>> {
        if !self.0.writable.get() {
            return None;
        }
        let writer: Box<dyn FileWriter> = Box::new(MemoryWriter {
            file: self.clone(),
            pending: self.contents(),
        });
        Some(Box::pin(std::future::ready(Ok(writer))))
    }
}

/// Writes to a [`MemoryFile`], kept apart until finished.
struct MemoryWriter {
    file: MemoryFile,
    pending: Vec<u8>,
}

impl FileWriter for MemoryWriter {
    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let end = start.saturating_add(bytes.len());
        if self.pending.len() < end {
            self.pending.resize(end, 0);
        }
        self.pending[start..end].copy_from_slice(bytes);
        Box::pin(std::future::ready(Ok(())))
    }

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_> {
        self.pending
            .resize(usize::try_from(len).unwrap_or(usize::MAX), 0);
        Box::pin(std::future::ready(Ok(())))
    }

    fn finish(self: Box<Self>) -> FileFinishFuture {
        *self.file.0.bytes.borrow_mut() = self.pending;
        Box::pin(std::future::ready(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("answers at once"),
        }
    }

    #[test]
    fn each_dialog_answers_what_the_test_gave_and_is_kept() {
        let dialogs = MockFileDialogs::new().answering_paths(vec!["/a".into(), "/b".into()]);
        let options = OpenOptions {
            writable: true,
            ..OpenOptions::default()
        };
        let one = now(dialogs.open_file(Some(ViewId(1)), options.clone())).unwrap();
        assert!(matches!(one, Some(PickedFile::Path(path)) if path.as_os_str() == "/a"));
        assert_eq!(
            now(dialogs.open_files(None, options.clone()))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            now(dialogs.open_folder(None, FolderOptions::default())).unwrap(),
            Some(PathBuf::from("/a"))
        );
        assert_eq!(
            dialogs.asked()[0],
            AskedDialog::OpenFile(Some(ViewId(1)), options)
        );
        assert_eq!(dialogs.asked().len(), 3);

        let cancelled = MockFileDialogs::new();
        assert!(
            now(cancelled.save_file(None, SaveOptions::default()))
                .unwrap()
                .is_none()
        );
        let failing = MockFileDialogs::new().failing("no panel");
        assert_eq!(
            now(failing.open_files(None, OpenOptions::default())).err(),
            Some(FileDialogError::Failed("no panel".to_owned()))
        );
    }

    #[test]
    fn a_memory_file_reads_and_its_writes_land_when_finished() {
        let file = MemoryFile::new("notes.txt", b"hello".to_vec()).writable();
        let dialogs = MockFileDialogs::new().answering({
            let file = file.clone();
            move || vec![file.picked()]
        });
        let Some(mut picked) = now(dialogs.open_file(None, OpenOptions::default())).unwrap() else {
            panic!("a pick");
        };
        assert_eq!(now(picked.read_all()).unwrap(), b"hello");
        let PickedFile::Handle(handle) = &picked else {
            panic!("a handle");
        };
        let mut writer = now(handle.open_write().expect("writable")).unwrap();
        now(writer.write_at(5, b", file")).unwrap();
        assert_eq!(file.contents(), b"hello", "not before it finishes");
        now(writer.finish()).unwrap();
        assert_eq!(file.contents(), b"hello, file");
        assert!(
            MemoryFile::new("read only", Vec::new())
                .open_write()
                .is_none()
        );
    }
}
