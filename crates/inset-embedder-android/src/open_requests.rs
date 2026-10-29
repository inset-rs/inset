//! Being opened: an intent to view (`ACTION_VIEW`) asks to open its data, at launch or while the app
//! runs (`intents`): a `content://` document, opened as a picked one is
//! (`file_dialogs::open_document`), to write where the intent grants writing; a `file://` URI as its
//! path; any other URI as a link. The intent is read where it arrives, which only records its URI;
//! the app's loop opens the documents on the app's thread.

use inset_embedder::{OpenRequest, PickedFile, file_uri_path};
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use crate::file_dialogs::{open_document, string};
use crate::log::warn_to_log;

/// `Intent.ACTION_VIEW`.
const VIEW: &str = "android.intent.action.VIEW";
/// `Intent.FLAG_GRANT_WRITE_URI_PERMISSION`.
const GRANT_WRITE: jint = 0x2;

/// What an intent to view asks to open.
pub(crate) struct Viewed {
    uri: String,
    /// Whether the intent grants writing the document.
    writable: bool,
}

/// What the intents to view that arrived ask the app to open; `None` when nothing.
pub(crate) fn request(arrived: Vec<Viewed>) -> Option<OpenRequest> {
    let mut request = OpenRequest::default();
    for viewed in arrived {
        match viewed.uri.split_once(':').map(|(scheme, _)| scheme) {
            Some("content") => request.files.extend(open(&viewed)),
            Some("file") => match file_uri_path(&viewed.uri) {
                Some(path) => request.files.push(PickedFile::Path(path)),
                None => warn_to_log!("inset: {} names no file", viewed.uri),
            },
            _ => request.links.push(viewed.uri),
        }
    }
    (!request.is_empty()).then_some(request)
}

/// The document an intent asks to open: to write where it grants writing and the provider lets
/// it, to read otherwise.
fn open(viewed: &Viewed) -> Option<PickedFile> {
    let written = viewed
        .writable
        .then(|| open_document(&viewed.uri, true).ok())
        .flatten();
    match written.map_or_else(|| open_document(&viewed.uri, false), Ok) {
        Ok(file) => Some(file),
        Err(error) => {
            warn_to_log!("inset: {} could not be opened: {error}", viewed.uri);
            None
        }
    }
}

/// What `intent` asks to open, if it is an intent to view.
pub(crate) fn viewed(
    env: &mut Env<'_>,
    intent: &JObject<'_>,
) -> jni::errors::Result<Option<Viewed>> {
    let action = env
        .call_method(
            intent,
            jni_str!("getAction"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    if string(env, action)?.as_deref() != Some(VIEW) {
        return Ok(None);
    }
    let data = env
        .call_method(
            intent,
            jni_str!("getDataString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let Some(uri) = string(env, data)? else {
        return Ok(None);
    };
    let flags = env
        .call_method(intent, jni_str!("getFlags"), jni_sig!("()I"), &[])?
        .i()?;
    Ok(Some(Viewed {
        uri,
        writable: flags & GRANT_WRITE != 0,
    }))
}
