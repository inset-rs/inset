//! The system's file dialogs as the framework's (`waft_embedder`'s `file_dialogs`): what each
//! system shows is `os/`'s; here is what the systems share: the window a dialog's parent view is
//! in, the answer a dialog gives once the user has chosen, from whatever thread the system answers
//! on, and the conversions the systems' answers and options need, which build and test on every
//! system.
//!
//! A dialog's future is answered once, with what the user chose, or with a failure when the system
//! drops its answer (`answer.rs`); dropped before that, it closes its dialog.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};

use waft_embedder::{FileDialogError, FileDialogFuture, ViewId};
use winit::window::Window;

use crate::answer::{Answer, Answered, answer};
use crate::state::EmbedderState;

/// Where the views' windows are, for a dialog shown on its parent's.
pub(crate) struct ParentWindows(Rc<EmbedderState>);

impl ParentWindows {
    pub(crate) fn new(state: Rc<EmbedderState>) -> ParentWindows {
        ParentWindows(state)
    }

    /// The window `parent` is in; `None` for no parent, or one with no window.
    pub(crate) fn window(&self, parent: Option<ViewId>) -> Option<Arc<dyn Window>> {
        Some(self.0.windows().by_view(parent?)?.window)
    }
}

/// The system's side of a dialog's answer: given once, from any thread, or a failure if dropped
/// unused (`answer.rs`).
pub(crate) type DialogAnswer<T> = Answer<Result<T, FileDialogError>>;

/// A dialog's answer: what the system gives, and the future the app awaits. `close` closes the
/// dialog when the app drops the future before the dialog has answered, so a panel the app has
/// stopped waiting for does not stay on screen; it is never called once the system answered.
pub(crate) fn dialog_answer<T: Send + 'static>(
    close: impl FnOnce() + 'static,
) -> (DialogAnswer<T>, FileDialogFuture<T>) {
    let (answer, answered) = answer(Err(FileDialogError::Failed(
        "the dialog closed without an answer".to_owned(),
    )));
    (
        answer,
        Box::pin(Dialog {
            answered,
            close: Some(Box::new(close)),
        }),
    )
}

/// A dialog's future: the answer the system gives, and the dialog it belongs to, which the drop
/// closes when the app stops waiting before the system answered.
struct Dialog<T> {
    answered: Answered<Result<T, FileDialogError>>,
    close: Option<Box<dyn FnOnce()>>,
}

impl<T> Future for Dialog<T> {
    type Output = Result<T, FileDialogError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.answered).poll(cx)
    }
}

impl<T> Drop for Dialog<T> {
    fn drop(&mut self) {
        if self.answered.settled() {
            return;
        }
        if let Some(close) = self.close.take() {
            close();
        }
    }
}

/// Each type's extensions as the patterns a Windows dialog filters by, `*.txt;*.md`, with its
/// label; a type with no extensions filters nothing there, and is left out.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn extension_patterns(types: &[waft_embedder::FileType]) -> Vec<(String, String)> {
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
    types: &[waft_embedder::FileType],
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
    use std::cell::Cell;

    use waft_embedder::FileType;

    use super::*;

    fn text() -> FileType {
        FileType {
            label: "Text".to_owned(),
            extensions: vec!["txt".to_owned(), ".md".to_owned()],
            mime_types: vec!["text/plain".to_owned()],
        }
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
    fn a_dropped_dialog_closes_and_an_answered_one_does_not() {
        let closed = Rc::new(Cell::new(false));
        {
            let closed = Rc::clone(&closed);
            let (_answer, dialog) = dialog_answer::<()>(move || closed.set(true));
            drop(dialog);
        }
        assert!(
            closed.get(),
            "the dialog closes when its future is dropped before an answer"
        );

        let closed = Rc::new(Cell::new(false));
        let (answer, dialog) = dialog_answer::<()>({
            let closed = Rc::clone(&closed);
            move || closed.set(true)
        });
        answer.give(Ok(()));
        drop(dialog);
        assert!(
            !closed.get(),
            "a dialog the system answered closes nothing when its future is dropped"
        );
    }
}
