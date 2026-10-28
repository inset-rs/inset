//! The system's file dialogs as the framework's (`inset_embedder`'s `file_dialogs`): what each
//! system shows is `os/`'s; here is what the systems share: the window a dialog's parent view is
//! in, the answer a dialog gives once the user has chosen, from whatever thread the system answers
//! on, and the conversions the systems' answers and options need, which build and test on every
//! system.
//!
//! The invariant: a dialog's future is answered exactly once, with what the user chose, or with a
//! failure when the system drops its answer, so the app never waits on a dialog for ever.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use inset_embedder::{FileDialogError, FileDialogFuture, ViewId};
use winit::window::Window;

use crate::state::EmbedderState;

/// Where the views' windows are, for a dialog shown on its parent's.
pub(crate) struct ParentWindows(Rc<EmbedderState>);

impl ParentWindows {
    pub(crate) fn new(state: Rc<EmbedderState>) -> ParentWindows {
        ParentWindows(state)
    }

    /// The window `parent` is in; `None` for no parent, or one with no window.
    pub(crate) fn window(&self, parent: Option<ViewId>) -> Option<Arc<Window>> {
        Some(self.0.views().by_view(parent?)?.window)
    }
}

/// What a dialog answers, on its way from the system to the app.
struct Answer<T> {
    answered: Option<Result<T, FileDialogError>>,
    waiter: Option<Waker>,
}

/// The system's side of a dialog's answer: given once, from any thread, or a failure if dropped
/// unused.
pub(crate) struct DialogAnswer<T>(Arc<Mutex<Answer<T>>>);

/// A dialog's answer: what the system gives, and the future the app awaits.
pub(crate) fn dialog_answer<T: Send + 'static>() -> (DialogAnswer<T>, FileDialogFuture<T>) {
    let shared = Arc::new(Mutex::new(Answer {
        answered: None,
        waiter: None,
    }));
    (
        DialogAnswer(Arc::clone(&shared)),
        Box::pin(Answered(shared)),
    )
}

impl<T> DialogAnswer<T> {
    pub(crate) fn give(self, answer: Result<T, FileDialogError>) {
        give(&self.0, answer);
    }
}

impl<T> Drop for DialogAnswer<T> {
    fn drop(&mut self) {
        give(
            &self.0,
            Err(FileDialogError::Failed(
                "the dialog closed without an answer".to_owned(),
            )),
        );
    }
}

/// Answers `answer` unless an answer was given already, and wakes the app's task.
fn give<T>(shared: &Mutex<Answer<T>>, answer: Result<T, FileDialogError>) {
    let waiter = {
        let mut shared = locked(shared);
        if shared.answered.is_some() {
            return;
        }
        shared.answered = Some(answer);
        shared.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

struct Answered<T>(Arc<Mutex<Answer<T>>>);

impl<T> Future for Answered<T> {
    type Output = Result<T, FileDialogError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = locked(&self.0);
        match shared.answered.take() {
            Some(answer) => Poll::Ready(answer),
            None => {
                shared.waiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The path a `file://` URI names, as the Linux desktop portal answers a pick; `None` for any
/// other URI, or one on another host.
#[cfg(any(target_os = "linux", all(test, unix)))]
pub(crate) fn file_uri_path(uri: &str) -> Option<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt;

    let rest = uri.strip_prefix("file://")?;
    let path = match rest.find('/')? {
        0 => rest,
        host_end if &rest[..host_end] == "localhost" => &rest[host_end..],
        _ => return None,
    };
    let mut bytes = Vec::with_capacity(path.len());
    let mut encoded = path.bytes();
    while let Some(byte) = encoded.next() {
        if byte == b'%' {
            let high = encoded.next().and_then(hex_digit)?;
            let low = encoded.next().and_then(hex_digit)?;
            bytes.push(high << 4 | low);
        } else {
            bytes.push(byte);
        }
    }
    Some(std::ffi::OsString::from_vec(bytes).into())
}

#[cfg(any(target_os = "linux", all(test, unix)))]
fn hex_digit(digit: u8) -> Option<u8> {
    char::from(digit).to_digit(16).map(|value| value as u8)
}

/// Each type's extensions as the patterns a Windows dialog filters by, `*.txt;*.md`, with its
/// label; a type with no extensions filters nothing there, and is left out.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn extension_patterns(types: &[inset_embedder::FileType]) -> Vec<(String, String)> {
    types
        .iter()
        .filter(|kind| !kind.extensions.is_empty())
        .map(|kind| {
            let patterns: Vec<String> = kind
                .extensions
                .iter()
                .map(|extension| format!("*.{}", extension.trim_start_matches('.')))
                .collect();
            (kind.label.clone(), patterns.join(";"))
        })
        .collect()
}

/// Each type as the desktop portal's filter: its label, and a glob per extension (kind 0) and
/// each MIME type (kind 1).
#[cfg(any(target_os = "linux", test))]
pub(crate) fn portal_filters(
    types: &[inset_embedder::FileType],
) -> Vec<(String, Vec<(u32, String)>)> {
    types
        .iter()
        .map(|kind| {
            let globs = kind
                .extensions
                .iter()
                .map(|extension| (0, format!("*.{}", extension.trim_start_matches('.'))));
            let mime_types = kind.mime_types.iter().map(|mime| (1, mime.clone()));
            (kind.label.clone(), globs.chain(mime_types).collect())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use inset_embedder::FileType;

    use super::*;

    fn text() -> FileType {
        FileType {
            label: "Text".to_owned(),
            extensions: vec!["txt".to_owned(), ".md".to_owned()],
            mime_types: vec!["text/plain".to_owned()],
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_file_uri_names_its_path_decoded() {
        use std::path::PathBuf;

        assert_eq!(
            file_uri_path("file:///home/me/My%20Notes/caf%C3%A9.txt"),
            Some(PathBuf::from("/home/me/My Notes/café.txt"))
        );
        assert_eq!(
            file_uri_path("file://localhost/tmp/a"),
            Some(PathBuf::from("/tmp/a"))
        );
        assert_eq!(file_uri_path("file://elsewhere/tmp/a"), None);
        assert_eq!(file_uri_path("https://example.org/a"), None);
        assert_eq!(file_uri_path("file:///bad%2"), None);
    }

    #[test]
    fn types_become_each_systems_filters() {
        let images = FileType {
            label: "Images".to_owned(),
            extensions: Vec::new(),
            mime_types: vec!["image/png".to_owned()],
        };
        assert_eq!(
            extension_patterns(&[text(), images.clone()]),
            [("Text".to_owned(), "*.txt;*.md".to_owned())]
        );
        assert_eq!(
            portal_filters(&[text(), images]),
            [
                (
                    "Text".to_owned(),
                    vec![
                        (0, "*.txt".to_owned()),
                        (0, "*.md".to_owned()),
                        (1, "text/plain".to_owned())
                    ]
                ),
                ("Images".to_owned(), vec![(1, "image/png".to_owned())]),
            ]
        );
    }

    #[test]
    fn an_answer_is_given_once_and_a_dropped_one_fails() {
        let (answer, mut answered) = dialog_answer::<u32>();
        answer.give(Ok(7));
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(
            answered.as_mut().poll(&mut context),
            Poll::Ready(Ok(7))
        ));
        let (dropped, mut answered) = dialog_answer::<u32>();
        drop(dropped);
        assert!(matches!(
            answered.as_mut().poll(&mut context),
            Poll::Ready(Err(FileDialogError::Failed(_)))
        ));
    }
}
