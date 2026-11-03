//! The browser's clipboard, as far as it goes: the asynchronous clipboard
//! (`navigator.clipboard`) for the app's own reads and writes, and the `copy`, `cut` and `paste`
//! events of the user's own command, which the browser runs for a key the app did not handle.
//!
//! What the browser allows, and so what the framework's clipboard can do here: items in the types
//! the browser carries (`ClipboardItem.supports`; Chrome writes the first item alone), each given
//! or promised, a promise resolved when the browser takes the write rather than when someone
//! pastes; no file, given or promised, since a page has no paths; no options, as the browser
//! has none, so a copy is neither kept out of history nor this device's alone, and one that
//! expires is not cleared, since a page cannot tell whether its copy is still there without
//! reading the clipboard. A read the browser may first ask the user about, or refuse: Firefox
//! and Safari show their own Paste button. A read's items are what the browser read, so a read
//! never lands on another copy. The browser says the clipboard changed with `clipboardchange`
//! where it has it, for the page's own writes too, which it does not tell apart, and the host
//! says it may have changed at each focus where the browser has no such event.
//!
//! In a `paste` event, the pasted text is the event's to read only while the event is
//! dispatched, so it is read then, and the pasted files later, from the files themselves.

use inset_embedder::{
    Clipboard, ClipboardError, ClipboardEvent, ClipboardItem, ClipboardItemsFuture,
    ClipboardReadFuture, ClipboardWriteFuture, Content, ContentFuture, CopyOptions, CopyRequest,
    DataTransferItem,
};
use js_sys::{Array, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{Blob, BlobPropertyBag, DataTransfer, File};

/// The browser's asynchronous clipboard, the app's own reads and writes.
pub(crate) struct BrowserClipboard;

impl Clipboard for BrowserClipboard {
    /// The browser's `read`, which may ask the user first; a read it refuses lists nothing.
    fn items(&self) -> ClipboardItemsFuture {
        let Some(clipboard) = navigator_clipboard() else {
            return Box::pin(std::future::ready(Vec::new()));
        };
        let read = clipboard.read();
        Box::pin(async move {
            let Ok(items) = JsFuture::from(read).await else {
                return Vec::new();
            };
            Array::from(&items)
                .iter()
                .filter_map(|item| item.dyn_into::<web_sys::ClipboardItem>().ok())
                .map(|item| Box::new(ReadItem::new(item)) as Box<dyn DataTransferItem>)
                .collect()
        })
    }

    /// The browser's `write`, started in the call, so it is inside the user's gesture where the
    /// browser asks for one.
    fn write(&self, items: Vec<ClipboardItem>, _options: CopyOptions) -> ClipboardWriteFuture {
        let written = navigator_clipboard()
            .ok_or_else(|| ClipboardError::Failed("the page has no clipboard".to_owned()))
            .and_then(|clipboard| {
                let items = items
                    .into_iter()
                    .map(browser_item)
                    .collect::<Result<Array, _>>()?;
                Ok(clipboard.write(&items))
            });
        Box::pin(async move {
            JsFuture::from(written?)
                .await
                .map(|_| ())
                .map_err(|_| ClipboardError::Denied)
        })
    }
}

/// The `copy` or `cut` the user asked for, written through the browser's `write`, which the
/// event's gesture allows.
pub(crate) fn copy_request() -> CopyRequest {
    CopyRequest::new(|items, options| BrowserClipboard.write(items, options))
}

/// What the user pasted, as `paste`'s data has it now: its text, one item in each type the
/// browser gives as text, and each file as an item of its own.
pub(crate) fn pasted(data: &DataTransfer) -> ClipboardEvent {
    let mut items: Vec<Box<dyn DataTransferItem>> = Vec::new();
    let text = PastedText::read(data);
    if !text.types.is_empty() {
        items.push(Box::new(text));
    }
    if let Some(files) = data.files() {
        for index in 0..files.length() {
            if let Some(file) = files.item(index) {
                items.push(Box::new(PastedFile::new(file)));
            }
        }
    }
    ClipboardEvent::Paste(items)
}

fn navigator_clipboard() -> Option<web_sys::Clipboard> {
    Some(web_sys::window()?.navigator().clipboard())
}

/// Whether the browser says when the clipboard changes, with `clipboardchange`.
pub(crate) fn tells_changes(clipboard: &web_sys::Clipboard) -> bool {
    Reflect::has(clipboard, &JsValue::from_str("onclipboardchange")).unwrap_or(false)
}

/// The browser's item for `item`: each form a blob, or a promise of one.
fn browser_item(item: ClipboardItem) -> Result<JsValue, ClipboardError> {
    let forms = Object::new();
    for (mime, content) in item.representations {
        let form = match content {
            Content::Data(bytes) => JsValue::from(blob(&mime, &bytes)?),
            Content::Promise(produce) => JsValue::from(promised_blob(mime.clone(), produce())),
            Content::File(_) | Content::FilePromise { .. } => {
                return Err(ClipboardError::Unsupported);
            }
        };
        let _ = Reflect::set(&forms, &JsValue::from_str(&mime), &form);
    }
    web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&forms)
        .map(JsValue::from)
        .map_err(|_| ClipboardError::Unsupported)
}

fn blob(mime: &str, bytes: &[u8]) -> Result<Blob, ClipboardError> {
    let parts = Array::of1(&Uint8Array::from(bytes));
    let properties = BlobPropertyBag::new();
    properties.set_type(mime);
    Blob::new_with_u8_array_sequence_and_options(&parts, &properties)
        .map_err(|_| ClipboardError::Failed("the browser made no blob".to_owned()))
}

/// A promise of the blob `produced` makes, which the browser waits for before it writes.
fn promised_blob(mime: String, produced: ContentFuture) -> Promise {
    future_to_promise(async move {
        let bytes = produced
            .await
            .ok_or_else(|| JsValue::from_str("the promised form was not produced"))?;
        blob(&mime, &bytes)
            .map(JsValue::from)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    })
}

/// The bytes of `blob`, once the browser has read them.
async fn bytes_of(blob: Blob) -> Result<Vec<u8>, ClipboardError> {
    let buffer = JsFuture::from(blob.array_buffer())
        .await
        .map_err(|_| ClipboardError::Failed("the browser did not read the blob".to_owned()))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

/// An item the browser's `read` gave.
struct ReadItem {
    item: web_sys::ClipboardItem,
    types: Vec<String>,
}

impl ReadItem {
    fn new(item: web_sys::ClipboardItem) -> ReadItem {
        let types = item
            .types()
            .iter()
            .filter_map(|mime| mime.as_string())
            .collect();
        ReadItem { item, types }
    }
}

impl DataTransferItem for ReadItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        if !self.types.iter().any(|offered| offered == mime) {
            return Box::pin(std::future::ready(Err(ClipboardError::NotFound)));
        }
        let form = self.item.get_type(mime);
        Box::pin(async move {
            let blob = JsFuture::from(form)
                .await
                .map_err(|_| ClipboardError::Denied)?
                .dyn_into::<Blob>()
                .map_err(|_| ClipboardError::Failed("the browser gave no blob".to_owned()))?;
            bytes_of(blob).await
        })
    }
}

/// The text a `paste` carried, in each type the browser gave as text.
struct PastedText {
    types: Vec<String>,
    texts: Vec<String>,
}

impl PastedText {
    /// What `data` holds as text now, while its event is dispatched.
    fn read(data: &DataTransfer) -> PastedText {
        let mut pasted = PastedText {
            types: Vec::new(),
            texts: Vec::new(),
        };
        for mime in data.types().iter().filter_map(|mime| mime.as_string()) {
            // "Files" names the pasted files, which are items of their own.
            if mime == "Files" {
                continue;
            }
            if let Ok(text) = data.get_data(&mime) {
                pasted.types.push(mime);
                pasted.texts.push(text);
            }
        }
        pasted
    }
}

impl DataTransferItem for PastedText {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        let text = self.types.iter().position(|offered| offered == mime);
        let read = text
            .map(|position| self.texts[position].clone().into_bytes())
            .ok_or(ClipboardError::NotFound);
        Box::pin(std::future::ready(read))
    }
}

/// A file a `paste` carried, read from the file itself.
struct PastedFile {
    file: File,
    types: [String; 1],
}

impl PastedFile {
    fn new(file: File) -> PastedFile {
        let mime = match file.type_() {
            mime if mime.is_empty() => "application/octet-stream".to_owned(),
            mime => mime,
        };
        PastedFile {
            file,
            types: [mime],
        }
    }
}

impl DataTransferItem for PastedFile {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        if mime != self.types[0] {
            return Box::pin(std::future::ready(Err(ClipboardError::NotFound)));
        }
        Box::pin(bytes_of(Blob::from(self.file.clone())))
    }
}
