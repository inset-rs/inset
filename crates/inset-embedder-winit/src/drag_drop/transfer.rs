//! What is dragged, read through winit's data transfer, where the embedder has no system code of
//! its own for a drag, Windows and Linux among them: one item, as those systems hold one, offered in
//! each form winit lists that has a MIME type, and read through a fetch that winit answers later,
//! with an event of its own.
//!
//! What each system gives through winit: Windows, what winit copies out of the drop's
//! `IDataObject` as the drag enters, its text, its files and a PNG image, and no other form, since
//! winit hands out no `IDataObject`; Wayland, every form of the offer, by the MIME type it is
//! offered as; X11, the forms winit names a type for: text, HTML, RTF, a list of URIs, and images
//! and sounds by their formats.
//!
//! The invariant: winit fetches a drop's data only while the drop is handled. Windows lets the
//! drop's data go as its `Drop` returns, Wayland finishes the offer and X11 the selection once no
//! fetch waits. So a read the app asks for while the drop is reported waits in the item, and the
//! embedder asks winit for it as the report returns, before winit's drop event does
//! ([`TransferForms::fetch_asked`]); a read asked for later is refused before it reaches the item
//! (`inset_embedder::DraggedItems`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use inset_embedder::{ClipboardError, ClipboardReadFuture, DataTransferItem, URI_LIST};
use winit::data_transfer::{DataTransfer, TransferType, TypeHint, TypedData};
use winit::error::TransferError;
use winit::event_loop::AsyncRequestSerial;

use crate::answer::{Answer, answer};

/// What a read of one form answers.
type Content = Result<Vec<u8>, ClipboardError>;

/// The forms a transfer offers, and the reads the app asked of its item, waiting to be fetched.
pub(crate) struct TransferForms {
    forms: Rc<[Form]>,
    asked: Rc<RefCell<Vec<AskedRead>>>,
}

/// One form of a transfer: its MIME type, and what winit is asked for it by.
struct Form {
    mime: String,
    asked_as: Box<dyn TransferType>,
}

/// A read the app asked for, waiting for the embedder to fetch it.
struct AskedRead {
    mime: String,
    answer: Answer<Content>,
}

impl TransferForms {
    /// The forms `transfer` offers, in its order, each MIME type once; none when it offers no form
    /// the framework can name.
    pub(crate) fn offered(transfer: &dyn DataTransfer) -> Option<TransferForms> {
        let mut forms: Vec<Form> = Vec::new();
        for offered in transfer.available_types() {
            let Some(form) = form_of(offered) else {
                continue;
            };
            if !forms.iter().any(|known| known.mime == form.mime) {
                forms.push(form);
            }
        }
        (!forms.is_empty()).then(|| TransferForms {
            forms: forms.into(),
            asked: Rc::default(),
        })
    }

    /// The one item the forms are.
    pub(crate) fn item(&self) -> Box<dyn DataTransferItem> {
        Box::new(TransferItem {
            types: self.forms.iter().map(|form| form.mime.clone()).collect(),
            asked: Rc::clone(&self.asked),
        })
    }

    /// Asks winit, through `fetch`, for each form the app read, the answers to come into
    /// `fetches`; a read winit refuses fails.
    pub(crate) fn fetch_asked(
        &self,
        mut fetch: impl FnMut(&dyn TransferType) -> Result<AsyncRequestSerial, TransferError>,
        fetches: &mut Fetches,
    ) {
        for asked in self.asked.take() {
            let Some(form) = self.forms.iter().find(|form| form.mime == asked.mime) else {
                asked.answer.give(Err(ClipboardError::NotFound));
                continue;
            };
            match fetch(&*form.asked_as) {
                Ok(serial) => fetches.wait(serial, asked.answer),
                Err(error) => asked
                    .answer
                    .give(Err(ClipboardError::Failed(error.to_string()))),
            }
        }
    }
}

/// The reads winit has yet to answer, by the serial of their fetch.
#[derive(Default)]
pub(crate) struct Fetches(HashMap<AsyncRequestSerial, Answer<Content>>);

impl Fetches {
    fn wait(&mut self, serial: AsyncRequestSerial, answer: Answer<Content>) {
        self.0.insert(serial, answer);
    }

    /// winit answered the fetch `serial` with `value`: the read waiting for it completes.
    pub(crate) fn answered(&mut self, serial: AsyncRequestSerial, value: &dyn TypedData) {
        if let Some(answer) = self.0.remove(&serial) {
            answer.give(content_of(value));
        }
    }
}

/// A transfer as one item: a read asked for waits for the embedder to fetch it.
struct TransferItem {
    types: Vec<String>,
    asked: Rc<RefCell<Vec<AskedRead>>>,
}

impl DataTransferItem for TransferItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    /// Waits for the embedder to fetch it: a read never fetched answers that the drag's data
    /// went.
    fn read(&self, mime: &str) -> ClipboardReadFuture {
        if !self.types.iter().any(|offered| offered == mime) {
            return Box::pin(std::future::ready(Err(ClipboardError::NotFound)));
        }
        let (answer, answered) = answer(Err(ClipboardError::Changed));
        self.asked.borrow_mut().push(AskedRead {
            mime: mime.to_owned(),
            answer,
        });
        Box::pin(answered)
    }
}

/// The form `offered` is, by the MIME type it is offered as where the system names one, as
/// Wayland does, or else by winit's type; none for a type with no MIME type.
fn form_of(offered: &dyn TransferType) -> Option<Form> {
    #[cfg(target_os = "linux")]
    if let Some(offered) = offered.cast_ref::<winit::platform::wayland::MimeType>() {
        return offered_mime(offered);
    }
    let hint = offered.hint()?;
    Some(Form {
        mime: mime_of(hint)?,
        asked_as: Box::new(hint),
    })
}

/// A form of a Wayland offer, by its MIME type without parameters: `text/plain;charset=utf-8` is
/// `text/plain`. X11's names that are no MIME type, `UTF8_STRING` among them, are left out.
#[cfg(target_os = "linux")]
fn offered_mime(offered: &winit::platform::wayland::MimeType) -> Option<Form> {
    let named = offered.to_string();
    let essence = named.split(';').next()?.trim();
    essence.contains('/').then(|| Form {
        mime: essence.to_owned(),
        asked_as: Box::new(offered.clone()),
    })
}

/// The MIME type of winit's `hint`; none for media winit names no format of.
fn mime_of(hint: TypeHint) -> Option<String> {
    let mime = match hint {
        TypeHint::Plaintext => "text/plain",
        TypeHint::UriList => URI_LIST,
        TypeHint::Html => "text/html",
        TypeHint::Rtf => "text/rtf",
        TypeHint::Image {
            extension_hint: Some(extension),
        } => return media_type("image", extension),
        TypeHint::Audio {
            extension_hint: Some(extension),
        } => return media_type("audio", extension),
        _ => return None,
    };
    Some(mime.to_owned())
}

/// The MIME type of an image or a sound, `kind`, in the format winit names by its file
/// `extension`; none for a camera's raw format, which has no registered type.
fn media_type(kind: &str, extension: &str) -> Option<String> {
    let subtype = match extension {
        "jpg" => "jpeg",
        "svg" => "svg+xml",
        "ico" => "x-icon",
        "aif" => "aiff",
        "mp3" => "mpeg",
        "raw" => return None,
        other => other,
    };
    Some(format!("{kind}/{subtype}"))
}

/// What winit fetched, as the framework reads the form: a list of URIs one per line (RFC 2483),
/// as Windows gives it only as URIs; text as UTF-8, which winit decodes from what the system
/// gave; anything else as its bytes.
fn content_of(value: &dyn TypedData) -> Content {
    let read = match value.type_().hint() {
        Some(TypeHint::UriList) => value.try_as_uris().map(|uris| uri_list(&uris)),
        Some(TypeHint::Plaintext | TypeHint::Html) => value
            .try_as_string()
            .map(String::into_bytes)
            .or_else(|_| value.try_as_bytes()),
        _ => value.try_as_bytes(),
    };
    read.map_err(|error| ClipboardError::Failed(error.to_string()))
}

/// `uris` as a `text/uri-list`.
fn uri_list(uris: &[String]) -> Vec<u8> {
    let mut list = String::new();
    for uri in uris {
        list.push_str(uri);
        list.push_str("\r\n");
    }
    list.into_bytes()
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use winit::data_transfer::{DataTransferSendBuilder, SendData};

    use super::*;

    /// A transfer offering `hints`, as a drag's source would.
    fn offering(hints: &[TypeHint]) -> Box<dyn DataTransfer> {
        let mut transfer = DataTransferSendBuilder::new(());
        for &hint in hints {
            transfer.add_type(hint, |_, _| None::<SendData>);
        }
        Box::new(transfer)
    }

    /// What winit answers a fetch with: `bytes`, of `hint`'s type.
    #[derive(Debug)]
    struct Fetched {
        hint: TypeHint,
        bytes: Vec<u8>,
    }

    impl TypedData for Fetched {
        fn type_(&self) -> &dyn TransferType {
            &self.hint
        }

        fn try_read(&self) -> Option<Box<dyn io::BufRead>> {
            Some(Box::new(io::Cursor::new(self.bytes.clone())))
        }

        fn try_as_uris(&self) -> io::Result<Vec<String>> {
            Ok(String::from_utf8_lossy(&self.bytes)
                .lines()
                .map(str::to_owned)
                .collect())
        }

        fn try_as_string(&self) -> io::Result<String> {
            Ok(String::from_utf8_lossy(&self.bytes).into_owned())
        }
    }

    fn poll<T>(future: &mut Pin<Box<dyn Future<Output = T>>>) -> Poll<T> {
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    }

    const PNG: TypeHint = TypeHint::Image {
        extension_hint: Some("png"),
    };

    #[test]
    fn a_transfer_is_one_item_in_each_form_with_a_mime_type_once() {
        let transfer = offering(&[
            TypeHint::UriList,
            TypeHint::Plaintext,
            TypeHint::Plaintext,
            PNG,
            TypeHint::Image {
                extension_hint: None,
            },
        ]);
        let forms = TransferForms::offered(&*transfer).unwrap();
        assert_eq!(forms.item().types(), [URI_LIST, "text/plain", "image/png"]);
    }

    #[test]
    fn a_transfer_with_no_form_the_framework_names_carries_nothing() {
        let transfer = offering(&[TypeHint::Audio {
            extension_hint: None,
        }]);
        assert!(TransferForms::offered(&*transfer).is_none());
    }

    #[test]
    fn a_read_waits_for_its_fetch_and_completes_with_what_winit_answers() {
        let forms = TransferForms::offered(&*offering(&[TypeHint::UriList, PNG])).unwrap();
        let item = forms.item();
        let mut read = item.read(URI_LIST);
        assert!(
            poll(&mut read).is_pending(),
            "nothing is fetched until asked"
        );

        let serial = AsyncRequestSerial::get();
        let mut asked_as = Vec::new();
        let mut fetches = Fetches::default();
        forms.fetch_asked(
            |form| {
                asked_as.push(form.hint());
                Ok(serial)
            },
            &mut fetches,
        );
        assert_eq!(asked_as, [Some(TypeHint::UriList)]);
        assert!(poll(&mut read).is_pending());

        let fetched = Fetched {
            hint: TypeHint::UriList,
            bytes: b"file:///a\nfile:///b".to_vec(),
        };
        fetches.answered(serial, &fetched);
        assert_eq!(
            poll(&mut read),
            Poll::Ready(Ok(b"file:///a\r\nfile:///b\r\n".to_vec()))
        );
    }

    #[test]
    fn a_read_winit_refuses_fails_and_one_never_fetched_answers_that_the_data_went() {
        let forms = TransferForms::offered(&*offering(&[PNG])).unwrap();
        let item = forms.item();
        let mut refused = item.read("image/png");
        forms.fetch_asked(|_| Err(TransferError::Failed), &mut Fetches::default());
        assert!(matches!(
            poll(&mut refused),
            Poll::Ready(Err(ClipboardError::Failed(_)))
        ));

        let mut never_fetched = item.read("image/png");
        drop(forms);
        drop(item);
        assert_eq!(
            poll(&mut never_fetched),
            Poll::Ready(Err(ClipboardError::Changed))
        );
    }

    #[test]
    fn a_form_not_offered_is_not_found() {
        let forms = TransferForms::offered(&*offering(&[PNG])).unwrap();
        let mut read = forms.item().read("text/plain");
        assert_eq!(poll(&mut read), Poll::Ready(Err(ClipboardError::NotFound)));
    }

    #[test]
    fn media_are_named_by_their_registered_types() {
        assert_eq!(
            mime_of(TypeHint::Image {
                extension_hint: Some("jpg")
            })
            .as_deref(),
            Some("image/jpeg")
        );
        assert_eq!(
            mime_of(TypeHint::Audio {
                extension_hint: Some("mp3")
            })
            .as_deref(),
            Some("audio/mpeg")
        );
        assert_eq!(
            mime_of(TypeHint::Image {
                extension_hint: Some("raw")
            }),
            None
        );
    }
}
