//! The file dialogs: UIKit's `UIDocumentPickerViewController`, presented by the parent view's view
//! controller, or that of a view in sight (`Views::presenter`), answering the paths of the
//! documents picked. A future dropped before the picker answers dismisses it.
//!
//! A document outside the app's sandbox is reached only while its security-scoped access lasts,
//! between `startAccessingSecurityScopedResource` and its stop. A path carries no lifetime the app
//! could end it by, so the access starts when the pick is answered and is kept for the app's life:
//! the URLs are kept here, never stopped, and the system ends their access when the app quits.
//! Each holds a sandbox extension of the kernel's, which an app picking without end would run
//! out of; an app picks a few files a session.
//!
//! iOS has no dialog that answers a file yet to be written: its picker exports a file that
//! exists. A save makes an empty file under the suggested name, in a folder of its own in the
//! temporary folder, and has the picker move it where the user chooses; the URL it answers is the
//! file there, which the app then writes, its access kept as an open's is. A folder pick opens
//! the folder type. A type's extensions and MIME types become the uniform types the picker offers;
//! iOS shows no type's label and no confirming label of the app's. Every pick may be read and
//! written, as the picker opens in place, so `writable` asks nothing more here.

// UIKit's classes need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{UIDocumentPickerDelegate, UIDocumentPickerViewController};
use objc2_uniform_type_identifiers::{UTType, UTTypeFolder, UTTypeItem};
use waft_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FileType, FolderOptions, OpenOptions,
    PickedFile, PickedFolder, SaveOptions, ViewId,
};
use waft_embedder_apple::answer::{Answer, Answered, answer};

use crate::views::Views;

thread_local! {
    /// The pickers' delegates until they answer, which UIKit holds only weakly. Main thread.
    static ANSWERING: RefCell<Vec<Retained<PickerDelegate>>> = RefCell::default();
    /// The documents whose access is kept for the app's life. Main thread.
    static ACCESSED: RefCell<Vec<Retained<NSURL>>> = RefCell::default();
}

/// UIKit's document pickers, presented from the app's views.
pub(crate) struct DocumentPickers {
    views: Rc<Views>,
}

impl DocumentPickers {
    pub(crate) fn new(views: Rc<Views>) -> DocumentPickers {
        DocumentPickers { views }
    }
}

/// The system's side of a dialog's answer: given once, or a failure if dropped unused.
type DialogAnswer<T> = Answer<Result<T, FileDialogError>>;

/// A dialog's answer: what the system gives, and the answer the app awaits, in the picker's own
/// future ([`PickerFuture`]).
fn dialog_answer<T: Send + 'static>() -> (DialogAnswer<T>, Answered<Result<T, FileDialogError>>) {
    answer(Err(FileDialogError::Failed(
        "the dialog closed without an answer".to_owned(),
    )))
}

/// A picker's future: the paths the picker answers, and the picker, dismissed when the app drops
/// the future before the picker has answered.
struct PickerFuture {
    answered: Answered<Result<Vec<PathBuf>, FileDialogError>>,
    /// The picker still presented and the delegate answering it, until either answers.
    showing: Option<(
        Retained<UIDocumentPickerViewController>,
        Retained<PickerDelegate>,
    )>,
}

impl Future for PickerFuture {
    type Output = Result<Vec<PathBuf>, FileDialogError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.answered).poll(cx)
    }
}

impl Drop for PickerFuture {
    fn drop(&mut self) {
        let Some((picker, delegate)) = self.showing.take() else {
            return;
        };
        // UIKit is the main thread's alone, where the app drops its own future. A delegate the
        // picker answered took itself out of the answering ones, and its picker is gone already.
        if MainThreadMarker::new().is_none() {
            return;
        }
        let showing = ANSWERING.with_borrow_mut(|answering| {
            let Some(index) = answering
                .iter()
                .position(|answering| std::ptr::eq(&**answering, &*delegate))
            else {
                return false;
            };
            answering.remove(index);
            true
        });
        // Dismissed by the controller that presented it, which takes with it anything the picker
        // presented in turn; asked of the picker, UIKit would dismiss only that.
        if showing && let Some(presenter) = picker.presentingViewController() {
            presenter.dismissViewControllerAnimated_completion(true, None);
        }
    }
}

impl FileDialogs for DocumentPickers {
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let chosen = self.open(
            parent,
            &content_types(&options.types),
            &options.initial_folder,
            false,
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }

    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        let chosen = self.open(
            parent,
            &content_types(&options.types),
            &options.initial_folder,
            true,
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().map(PickedFile::Path).collect()) })
    }

    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>> {
        // SAFETY: a constant UniformTypeIdentifiers exports for the life of the process.
        let folder = unsafe { UTTypeFolder };
        let chosen = self.open(
            parent,
            &NSArray::from_slice(&[folder]),
            &options.initial_folder,
            false,
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFolder::Path)) })
    }

    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let file = match empty_file(options.suggested_name.as_deref()) {
            Ok(file) => file,
            Err(error) => return failed(error.to_string()),
        };
        let Some(url) = NSURL::from_file_path(&file) else {
            return failed(format!("no URL for {}", file.display()));
        };
        let Some(main_thread) = MainThreadMarker::new() else {
            return failed("a save asked off the main thread".to_owned());
        };
        let picker = UIDocumentPickerViewController::initForExportingURLs_asCopy(
            main_thread.alloc(),
            &NSArray::from_retained_slice(&[url]),
            false,
        );
        if let Some(folder) = options
            .initial_folder
            .as_ref()
            .and_then(NSURL::from_directory_path)
        {
            picker.setDirectoryURL(Some(&folder));
        }
        let chosen = self.present(parent, &picker);
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }
}

impl DocumentPickers {
    fn open(
        &self,
        parent: Option<ViewId>,
        types: &NSArray<UTType>,
        initial_folder: &Option<PathBuf>,
        multiple: bool,
    ) -> FileDialogFuture<Vec<PathBuf>> {
        let Some(main_thread) = MainThreadMarker::new() else {
            return failed("a pick asked off the main thread".to_owned());
        };
        let picker = UIDocumentPickerViewController::initForOpeningContentTypes_asCopy(
            main_thread.alloc(),
            types,
            false,
        );
        picker.setAllowsMultipleSelection(multiple);
        if let Some(folder) = initial_folder.as_ref().and_then(NSURL::from_directory_path) {
            picker.setDirectoryURL(Some(&folder));
        }
        self.present(parent, &picker)
    }

    /// Presents `picker` from `parent`'s view controller, or that of a view in sight, and answers
    /// the paths picked, none if the user cancelled.
    fn present(
        &self,
        parent: Option<ViewId>,
        picker: &UIDocumentPickerViewController,
    ) -> FileDialogFuture<Vec<PathBuf>> {
        let Some(presenter) = self.views.presenter(parent) else {
            return failed("no view controller to present the picker from".to_owned());
        };
        let (answer, answered) = dialog_answer();
        let delegate = PickerDelegate::new(presenter.mtm(), answer);
        picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        ANSWERING.with_borrow_mut(|answering| answering.push(delegate.clone()));
        presenter.presentViewController_animated_completion(picker, true, None);
        Box::pin(PickerFuture {
            answered,
            showing: Some((picker.retain(), delegate)),
        })
    }
}

fn failed<T: 'static>(reason: String) -> FileDialogFuture<T> {
    Box::pin(std::future::ready(Err(FileDialogError::Failed(reason))))
}

/// The uniform types `types` name, by extension and by MIME type; any document when they name
/// none.
fn content_types(types: &[FileType]) -> Retained<NSArray<UTType>> {
    let by_extension = types
        .iter()
        .flat_map(|kind| &kind.extensions)
        .filter_map(|extension| {
            UTType::typeWithFilenameExtension(&NSString::from_str(
                extension.trim_start_matches('.'),
            ))
        });
    let by_mime_type = types
        .iter()
        .flat_map(|kind| &kind.mime_types)
        .filter_map(|mime| UTType::typeWithMIMEType(&NSString::from_str(mime)));
    let mut content_types: Vec<Retained<UTType>> = by_extension.chain(by_mime_type).collect();
    if content_types.is_empty() {
        // SAFETY: a constant UniformTypeIdentifiers exports for the life of the process.
        content_types.push(unsafe { UTTypeItem }.retain());
    }
    NSArray::from_retained_slice(&content_types)
}

/// A file a save has the picker move where the user chooses: empty, under `name`, in a folder of
/// its own in the temporary folder, so two saves of one name never meet.
fn empty_file(name: Option<&str>) -> std::io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let folder = std::env::temp_dir().join(format!(
        "waft-save-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&folder)?;
    let file = folder.join(name.filter(|name| !name.is_empty()).unwrap_or("Untitled"));
    std::fs::write(&file, [])?;
    Ok(file)
}

/// Where a picker's answer goes.
struct Pick {
    answer: RefCell<Option<DialogAnswer<Vec<PathBuf>>>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `PickerDelegate` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftDocumentPickerDelegate"]
    #[ivars = Pick]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn picked(&self, _picker: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            let paths = urls.iter().filter_map(|url| keep_access(&url)).collect();
            self.answer(Ok(paths));
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn cancelled(&self, _picker: &UIDocumentPickerViewController) {
            self.answer(Ok(Vec::new()));
        }
    }
);

impl PickerDelegate {
    fn new(
        main_thread: MainThreadMarker,
        answer: DialogAnswer<Vec<PathBuf>>,
    ) -> Retained<PickerDelegate> {
        let this = main_thread.alloc::<PickerDelegate>().set_ivars(Pick {
            answer: RefCell::new(Some(answer)),
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    /// Answers the picker once, and lets the delegate go.
    fn answer(&self, answer: Result<Vec<PathBuf>, FileDialogError>) {
        if let Some(waiting) = self.ivars().answer.borrow_mut().take() {
            waiting.give(answer);
        }
        let this: *const PickerDelegate = self;
        ANSWERING.with_borrow_mut(|answering| {
            answering.retain(|delegate| !std::ptr::eq(&**delegate, this));
        });
    }
}

/// Starts the access `url` needs, keeps it for the app's life, and answers its path.
fn keep_access(url: &NSURL) -> Option<PathBuf> {
    let path = url.to_file_path()?;
    // SAFETY: a URL the picker answered, whose access it grants until stopped.
    if unsafe { url.startAccessingSecurityScopedResource() } {
        ACCESSED.with_borrow_mut(|accessed| accessed.push(url.retain()));
    }
    Some(path)
}
