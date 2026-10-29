//! Being opened: the files an installed web app is launched with, which the browser queues on
//! `window.launchQueue` (Chrome and Edge) when the user opens a file with the app, at launch and,
//! for an app whose manifest launches into the window it has, while it runs. Each launch's files are
//! the browser's `FileSystemFileHandle`s, read-only, as the browser grants a launch no write
//! without asking the user in a gesture; a folder handle is left out. Where the browser has no
//! launch queue, nothing comes.
//!
//! Links stay the page's: a scheme the manifest registers reaches the app inside the page's own
//! address, in a shape the manifest chose, which the app reads itself.

use std::rc::Rc;

use inset_embedder::{OpenRequest, PickedFile};
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::file_dialogs::SystemFile;

/// Hands each launch's files to `opened`, once each handle's file is taken, for as long as the
/// page lives; nothing where the browser has no launch queue.
pub(crate) fn consume(window: &web_sys::Window, opened: impl Fn(OpenRequest) + 'static) {
    let Some(set_consumer) = set_consumer(window) else {
        return;
    };
    let opened = Rc::new(opened);
    let consumer = Closure::<dyn FnMut(JsValue)>::new(move |parameters: JsValue| {
        let opened = Rc::clone(&opened);
        wasm_bindgen_futures::spawn_local(async move {
            let request = OpenRequest {
                files: launched_files(&parameters).await,
                links: Vec::new(),
            };
            if !request.is_empty() {
                opened(request);
            }
        });
    });
    let _ = set_consumer.0.call1(&set_consumer.1, consumer.as_ref());
    consumer.forget();
}

/// `launchQueue.setConsumer` and the queue it is called on.
fn set_consumer(window: &web_sys::Window) -> Option<(Function, JsValue)> {
    let queue = Reflect::get(window, &JsValue::from_str("launchQueue")).ok()?;
    if queue.is_undefined() || queue.is_null() {
        return None;
    }
    let method = Reflect::get(&queue, &JsValue::from_str("setConsumer")).ok()?;
    Some((method.dyn_into().ok()?, queue))
}

/// The file handles of a launch's `files`, each as a read-only pick; one whose file the browser
/// does not give is left out.
async fn launched_files(parameters: &JsValue) -> Vec<PickedFile> {
    let files = Reflect::get(parameters, &JsValue::from_str("files")).unwrap_or_default();
    if !Array::is_array(&files) {
        return Vec::new();
    }
    let mut launched = Vec::new();
    for handle in files
        .unchecked_into::<Array>()
        .iter()
        .filter(is_file_handle)
    {
        if let Ok(file) = SystemFile::open(handle, false).await {
            launched.push(PickedFile::Handle(Box::new(file)));
        }
    }
    launched
}

fn is_file_handle(handle: &JsValue) -> bool {
    Reflect::get(handle, &JsValue::from_str("kind"))
        .ok()
        .and_then(|kind| kind.as_string())
        .is_some_and(|kind| kind == "file")
}
