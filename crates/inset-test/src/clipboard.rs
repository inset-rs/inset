//! A clipboard a test fills and reads back: the `MockClipboard` of Flutter's widget tests
//! (`test/widgets/clipboard_utils.dart`), as a host's [`Clipboard`] rather than a handler of
//! platform-channel calls, and holding every form a system clipboard holds: items in several
//! types, promises produced at each read, files and promised files, and how the last copy is to
//! be treated.

use std::cell::RefCell;
use std::fs;
use std::future::{Future, ready};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;

use inset_embedder::{
    Clipboard, ClipboardError, ClipboardItem, ClipboardItemsFuture, ClipboardReadFuture,
    ClipboardWriteFuture, Content, ContentFuture, CopyOptions, FileSink, PastedItem, URI_LIST,
    WriteFuture,
};

/// A clipboard held in memory, which answers every call at once but a promise's, which answers
/// once its producer does. Every clone is the same clipboard.
#[derive(Clone)]
pub struct MockClipboard(Rc<RefCell<Contents>>);

/// What is on the clipboard, and since when.
struct Contents {
    items: Vec<ClipboardItem>,
    options: CopyOptions,
    /// How many times the clipboard changed, as `NSPasteboard.changeCount` counts.
    change_count: u64,
    /// Where a promised file is written when it is read.
    promised_files: PathBuf,
}

impl Default for MockClipboard {
    fn default() -> MockClipboard {
        MockClipboard::new()
    }
}

impl MockClipboard {
    /// An empty clipboard, which writes a promised file read from it into the system's
    /// temporary folder.
    pub fn new() -> MockClipboard {
        MockClipboard(Rc::new(RefCell::new(Contents {
            items: Vec::new(),
            options: CopyOptions::new(),
            change_count: 0,
            promised_files: std::env::temp_dir(),
        })))
    }

    /// The same clipboard, writing a promised file read from it into `folder`.
    pub fn with_promised_files_folder(self, folder: impl Into<PathBuf>) -> MockClipboard {
        self.0.borrow_mut().promised_files = folder.into();
        self
    }

    /// The text on the clipboard: the first item's `text/plain`, as given, which the framework's
    /// last write left or the test set.
    pub fn clipboard_data(&self) -> Option<String> {
        let contents = self.0.borrow();
        contents.items.iter().find_map(|item| {
            item.representations
                .iter()
                .find_map(|(mime, content)| match (mime.as_str(), content) {
                    ("text/plain", Content::Data(bytes)) => {
                        Some(String::from_utf8_lossy(bytes).into_owned())
                    }
                    _ => None,
                })
        })
    }

    /// Puts `text` on the clipboard as one item, or empties it with `None`.
    pub fn set_clipboard_data(&self, text: Option<String>) {
        let items = text
            .into_iter()
            .map(|text| ClipboardItem {
                representations: vec![("text/plain".to_owned(), Content::Data(text.into_bytes()))],
            })
            .collect();
        self.replace(items, CopyOptions::new());
    }

    /// How the copy on the clipboard is to be treated, as its writer said.
    pub fn options(&self) -> CopyOptions {
        self.0.borrow().options
    }

    /// How many times the clipboard changed.
    pub fn change_count(&self) -> u64 {
        self.0.borrow().change_count
    }

    fn replace(&self, items: Vec<ClipboardItem>, options: CopyOptions) {
        let mut contents = self.0.borrow_mut();
        contents.items = items;
        contents.options = options;
        contents.change_count += 1;
    }
}

impl Clipboard for MockClipboard {
    fn items(&self) -> ClipboardItemsFuture {
        let contents = self.0.borrow();
        let items = contents
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                Box::new(MockPastedItem {
                    clipboard: self.clone(),
                    change_count: contents.change_count,
                    index,
                    types: types_offered(item),
                }) as Box<dyn PastedItem>
            })
            .collect();
        Box::pin(std::future::ready(items))
    }

    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        self.replace(items, options);
        Box::pin(std::future::ready(Ok(())))
    }
}

/// The types `item` is offered in, in its writer's order: a file's as [`URI_LIST`], once.
fn types_offered(item: &ClipboardItem) -> Vec<String> {
    let mut types: Vec<String> = Vec::new();
    for (mime, content) in &item.representations {
        let offered = match content {
            Content::Data(_) | Content::Promise(_) => mime.as_str(),
            Content::File(_) | Content::FilePromise { .. } => URI_LIST,
        };
        if !types.iter().any(|known| known == offered) {
            types.push(offered.to_owned());
        }
    }
    types
}

/// One item of a [`MockClipboard`], as it was listed.
struct MockPastedItem {
    clipboard: MockClipboard,
    change_count: u64,
    index: usize,
    types: Vec<String>,
}

/// One form of an item, taken out of the clipboard so that what makes it runs with the
/// clipboard let go.
enum Form {
    Data(Vec<u8>),
    Promise(Rc<dyn Fn() -> ContentFuture>),
    Files(Vec<File>),
}

/// A file an item carries.
enum File {
    Given(PathBuf),
    Promised {
        name: String,
        produce: Rc<dyn Fn(Box<dyn FileSink>) -> WriteFuture>,
    },
}

impl MockPastedItem {
    /// The item's form in `mime`, while the clipboard still holds the item.
    fn form(&self, mime: &str) -> Result<Form, ClipboardError> {
        let contents = self.clipboard.0.borrow();
        if contents.change_count != self.change_count {
            return Err(ClipboardError::Changed);
        }
        let item = &contents.items[self.index];
        if mime == URI_LIST {
            let files: Vec<File> = item
                .representations
                .iter()
                .filter_map(|(_, content)| match content {
                    Content::File(path) => Some(File::Given(path.clone())),
                    Content::FilePromise { name, produce } => Some(File::Promised {
                        name: name.clone(),
                        produce: Rc::clone(produce),
                    }),
                    Content::Data(_) | Content::Promise(_) => None,
                })
                .collect();
            if !files.is_empty() {
                return Ok(Form::Files(files));
            }
        }
        item.representations
            .iter()
            .filter(|(offered, _)| offered == mime)
            .find_map(|(_, content)| match content {
                Content::Data(bytes) => Some(Form::Data(bytes.clone())),
                Content::Promise(produce) => Some(Form::Promise(Rc::clone(produce))),
                Content::File(_) | Content::FilePromise { .. } => None,
            })
            .ok_or(ClipboardError::NotFound)
    }
}

impl PastedItem for MockPastedItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        let form = match self.form(mime) {
            Ok(form) => form,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let folder = self.clipboard.0.borrow().promised_files.clone();
        Box::pin(async move {
            match form {
                Form::Data(bytes) => Ok(bytes),
                Form::Promise(produce) => produce().await.ok_or_else(|| {
                    ClipboardError::Failed("the promised form was not produced".to_owned())
                }),
                Form::Files(files) => uri_list(files, &folder).await,
            }
        })
    }
}

/// The `file:` URL of each file, one per line, a promised one written into `folder` first.
async fn uri_list(files: Vec<File>, folder: &Path) -> Result<Vec<u8>, ClipboardError> {
    let mut list = String::new();
    for file in files {
        let path = match file {
            File::Given(path) => path,
            File::Promised { name, produce } => {
                let path = folder.join(name);
                written_into(&path, &produce)
                    .await
                    .map_err(|error| ClipboardError::Failed(error.to_string()))?;
                path
            }
        };
        list.push_str(&file_url(&path));
        list.push_str("\r\n");
    }
    Ok(list.into_bytes())
}

/// The file at `path`, written with what `produce` makes, as a host writes a promised file where
/// the receiver chose; removed when the producer fails.
async fn written_into(
    path: &Path,
    produce: &Rc<dyn Fn(Box<dyn FileSink>) -> WriteFuture>,
) -> io::Result<()> {
    let file = fs::File::create(path)?;
    let produced = produce(Box::new(FileWritten(file))).await;
    if produced.is_err() {
        let _ = fs::remove_file(path);
    }
    produced
}

/// A promised file's sink: the file it is written into, as the bytes come.
struct FileWritten(fs::File);

impl FileSink for FileWritten {
    fn write<'a>(
        &'a mut self,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + 'a>> {
        Box::pin(ready(self.0.write_all(bytes)))
    }
}

/// `path`'s `file:` URL: each byte but a path's unreserved ones and its slashes
/// percent-encoded (RFC 3986).
fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::task::{Context, Poll, Waker};

    use super::*;

    /// `future`'s output, which a mock clipboard answers at once.
    fn now<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("the mock clipboard answers at once"),
        }
    }

    fn data(mime: &str, bytes: &[u8]) -> (String, Content) {
        (mime.to_owned(), Content::Data(bytes.to_vec()))
    }

    fn written(clipboard: &MockClipboard, representations: Vec<(String, Content)>) {
        let item = ClipboardItem { representations };
        now(clipboard.write(vec![item], CopyOptions::new())).expect("the mock takes every write");
    }

    #[test]
    fn items_are_listed_in_their_types_and_read_in_one() {
        let clipboard = MockClipboard::new();
        written(
            &clipboard,
            vec![data("text/html", b"<b>hi</b>"), data("text/plain", b"hi")],
        );

        let items = now(clipboard.items());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].types(), ["text/html", "text/plain"]);
        assert_eq!(now(items[0].read("text/plain")), Ok(b"hi".to_vec()));
        assert_eq!(
            now(items[0].read("image/png")),
            Err(ClipboardError::NotFound)
        );
    }

    #[test]
    fn a_read_after_the_clipboard_changed_fails() {
        let clipboard = MockClipboard::new();
        clipboard.set_clipboard_data(Some("first".to_owned()));
        let items = now(clipboard.items());
        clipboard.set_clipboard_data(Some("second".to_owned()));
        assert_eq!(
            now(items[0].read("text/plain")),
            Err(ClipboardError::Changed)
        );
        assert_eq!(clipboard.change_count(), 2);
    }

    #[test]
    fn a_promise_is_produced_at_each_read_and_not_before() {
        let produced = Rc::new(Cell::new(0));
        let counter = Rc::clone(&produced);
        let promise = Content::Promise(Rc::new(move || {
            counter.set(counter.get() + 1);
            Box::pin(std::future::ready(Some(b"made".to_vec())))
        }));
        let clipboard = MockClipboard::new();
        written(&clipboard, vec![("text/plain".to_owned(), promise)]);
        assert_eq!(produced.get(), 0);

        let items = now(clipboard.items());
        assert_eq!(now(items[0].read("text/plain")), Ok(b"made".to_vec()));
        assert_eq!(now(items[0].read("text/plain")), Ok(b"made".to_vec()));
        assert_eq!(produced.get(), 2);
    }

    #[test]
    fn a_file_is_offered_and_read_as_its_url() {
        let clipboard = MockClipboard::new();
        let file = Content::File(PathBuf::from("/photos/a b.png"));
        written(&clipboard, vec![("image/png".to_owned(), file)]);

        let items = now(clipboard.items());
        assert_eq!(items[0].types(), [URI_LIST]);
        assert_eq!(
            now(items[0].read(URI_LIST)),
            Ok(b"file:///photos/a%20b.png\r\n".to_vec())
        );
    }

    /// A promised file named `name`, whose producer writes `chunks` in order, then fails if
    /// `fails`.
    fn promised_file(name: &str, chunks: &'static [&'static [u8]], fails: bool) -> Content {
        let produce: Rc<dyn Fn(Box<dyn FileSink>) -> WriteFuture> = Rc::new(move |mut sink| {
            Box::pin(async move {
                for chunk in chunks {
                    sink.write(chunk).await?;
                }
                if fails {
                    return Err(io::Error::other("the producer broke"));
                }
                Ok(())
            })
        });
        Content::FilePromise {
            name: name.to_owned(),
            produce,
        }
    }

    fn folder(test: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "inset-test-clipboard-{}-{test}",
            std::process::id()
        ));
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_promised_file_is_written_into_the_folder_when_read() {
        let folder = folder("written");
        let clipboard = MockClipboard::new().with_promised_files_folder(&folder);
        let promise = promised_file("note.txt", &[b"prom", b"ised"], false);
        written(&clipboard, vec![("text/plain".to_owned(), promise)]);

        let items = now(clipboard.items());
        let list = now(items[0].read(URI_LIST)).expect("the file was written");
        let path = folder.join("note.txt");
        assert_eq!(list, format!("{}\r\n", file_url(&path)).into_bytes());
        assert_eq!(fs::read(&path).unwrap(), b"promised");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_promised_file_whose_producer_fails_fails_the_paste_and_is_removed() {
        let folder = folder("failed");
        let clipboard = MockClipboard::new().with_promised_files_folder(&folder);
        let promise = promised_file("half.txt", &[b"half"], true);
        written(&clipboard, vec![("text/plain".to_owned(), promise)]);

        let items = now(clipboard.items());
        let read = now(items[0].read(URI_LIST));
        assert!(matches!(read, Err(ClipboardError::Failed(_))), "{read:?}");
        assert!(
            !folder.join("half.txt").exists(),
            "what was written is removed"
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_options_of_the_last_copy_are_kept() {
        let clipboard = MockClipboard::new();
        let options = CopyOptions::new().sensitive(true);
        now(clipboard.write(Vec::new(), options)).expect("the mock takes every write");
        assert_eq!(clipboard.options(), options);
    }
}
