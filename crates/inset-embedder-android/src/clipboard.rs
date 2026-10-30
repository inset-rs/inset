//! The system's clipboard: Android's `ClipboardManager`, reached through JNI from the app's thread
//! through the application, as far as an app without Java code of its own reaches it.
//!
//! A clip lists its MIME types in its description, which an app reads without reading the clip,
//! so the clip is listed as one item in those types, and a read takes the clip's first item that
//! holds the type, reading the clip then: its text as `text/plain`, its HTML as `text/html`, a
//! URI as `text/uri-list`, and the content a URI names, through the content resolver, in the
//! clip's other types. Android 12 and later tell the user when an app reads the clip, not its
//! description. An app writes text, with HTML beside it where it has some, and nothing else:
//! every other form, a file and a form produced when pasted among them, is on Android a content
//! provider's, which needs Java code of the app's own, so those are refused.
//!
//! A sensitive copy carries `ClipDescription.EXTRA_IS_SENSITIVE`, which keeps it out of the
//! system's preview and the keyboards' suggestions; a copy for this device only is as any other,
//! since Android has no such notion; one that expires is cleared at its time by the host's loop,
//! which wakes for it as it wakes for the app's timers, if the clip is still the same, as Android
//! has no expiry an app can set: the time is kept only while the app runs. Android tells changes
//! only to a listener Java code provides, so the host looks as an activity takes the focus,
//! comparing the clip's time, with the app's own writes counted as seen, and after it cleared an
//! expired copy.

use std::cell::{Cell, RefCell};
use std::time::Instant;

use inset_embedder::{
    Clipboard, ClipboardError, ClipboardItem, ClipboardItemsFuture, ClipboardReadFuture,
    ClipboardWriteFuture, Content, CopyOptions, PastedItem, URI_LIST,
};
use jni::objects::{JByteArray, JObject, JString, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::activity::{device_api_level, with_application};

const TEXT: &str = "text/plain";
const HTML: &str = "text/html";

/// The first release whose clipboard can be cleared.
const CLEAR_CLIP: i32 = 28;
/// `ClipDescription.EXTRA_IS_SENSITIVE`.
const IS_SENSITIVE: &str = "android.content.extra.IS_SENSITIVE";

/// A copy of the app's that expires: when, and the time of the clip the app set.
struct Expiry {
    at: Instant,
    set_at: i64,
}

/// The system's clipboard.
pub(crate) struct AndroidClipboard {
    /// The time of the clip when the clipboard was last asked whether it changed, or the app last
    /// wrote it, whichever came last.
    seen: Cell<Option<i64>>,
    /// The app's last copy, while it waits to expire; a newer copy replaces it.
    expiring: RefCell<Option<Expiry>>,
}

impl AndroidClipboard {
    pub(crate) fn new() -> AndroidClipboard {
        AndroidClipboard {
            seen: Cell::new(None),
            expiring: RefCell::default(),
        }
    }

    /// Whether another writer changed the clip since this was last asked: its time is another.
    pub(crate) fn has_changed(&self) -> bool {
        let now = with_application(clip_time).ok().flatten();
        self.seen.replace(now) != now
    }

    /// When the host's loop is to clear a copy of the app's that expires; none while no copy
    /// waits to.
    pub(crate) fn expiry(&self) -> Option<Instant> {
        self.expiring.borrow().as_ref().map(|expiry| expiry.at)
    }

    /// Clears the app's copy whose time has come by `now`, if the clip is still the one it set;
    /// answers whether it cleared one, which the app hears as a change.
    pub(crate) fn clear_expired(&self, now: Instant) -> bool {
        if self.expiry().is_none_or(|at| at > now) {
            return false;
        }
        let Some(expired) = self.expiring.take() else {
            return false;
        };
        let cleared = with_application(|env, context| {
            if clip_time(env, context)? != Some(expired.set_at) {
                return Ok(false);
            }
            let manager = clipboard_manager(env, context)?;
            env.call_method(&manager, jni_str!("clearPrimaryClip"), jni_sig!("()V"), &[])?;
            Ok(true)
        });
        self.seen.set(with_application(clip_time).ok().flatten());
        matches!(cleared, Ok(true))
    }

    fn write_now(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let texts = items
            .iter()
            .map(text_and_html)
            .collect::<Result<Vec<_>, _>>()?;
        with_application(|env, context| set_clip(env, context, &texts, options))
            .map_err(|error| ClipboardError::Failed(format!("{error:?}")))?;
        // The app made this change.
        let set_at = with_application(clip_time).ok().flatten();
        self.seen.set(set_at);
        let expiry = options
            .expires_after
            .filter(|_| device_api_level() >= CLEAR_CLIP)
            .zip(set_at)
            .map(|(after, set_at)| Expiry {
                at: Instant::now() + after,
                set_at,
            });
        self.expiring.replace(expiry);
        Ok(())
    }
}

impl Clipboard for AndroidClipboard {
    /// The clip's description, read in the call, as one item in its types; nothing where the
    /// system would not say, as for an app without the focus.
    fn items(&self) -> ClipboardItemsFuture {
        let listed = with_application(listed_clip)
            .ok()
            .flatten()
            .map(|clip| Box::new(ListedClip { clip }) as Box<dyn PastedItem>);
        Box::pin(std::future::ready(listed.into_iter().collect()))
    }

    /// Set in the call, since the system takes the clip at once.
    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        Box::pin(std::future::ready(self.write_now(items, options)))
    }
}

/// The text of `item`, and its HTML where it has some: the only forms an app sets a clip in.
fn text_and_html(item: &ClipboardItem) -> Result<(String, Option<String>), ClipboardError> {
    let mut text = None;
    let mut html = None;
    for (mime, content) in &item.representations {
        let Content::Data(bytes) = content else {
            return Err(ClipboardError::Unsupported);
        };
        let value = String::from_utf8_lossy(bytes).into_owned();
        match mime.as_str() {
            TEXT => text = Some(value),
            HTML => html = Some(value),
            _ => return Err(ClipboardError::Unsupported),
        }
    }
    Ok((text.ok_or(ClipboardError::Unsupported)?, html))
}

/// The clip as it was listed: its types, and its time, which a read checks.
struct Listing {
    types: Vec<String>,
    time: Option<i64>,
}

/// The clip on the clipboard as it was listed.
struct ListedClip {
    clip: Listing,
}

impl PastedItem for ListedClip {
    fn types(&self) -> &[String] {
        &self.clip.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        let read = if !self.clip.types.iter().any(|offered| offered == mime) {
            Err(ClipboardError::NotFound)
        } else {
            with_application(|env, context| read_clip(env, context, &self.clip, mime))
                .unwrap_or_else(|error| Err(ClipboardError::Failed(format!("{error:?}"))))
        };
        Box::pin(std::future::ready(read))
    }
}

fn clipboard_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    let name = env.new_string("clipboard")?;
    env.call_method(
        context,
        jni_str!("getSystemService"),
        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
        &[JValue::Object(&name)],
    )?
    .l()
}

/// The time the clip on the clipboard was set; none without a clip.
fn clip_time(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<Option<i64>> {
    let Some(description) = clip_description(env, context)? else {
        return Ok(None);
    };
    let time = env
        .call_method(&description, jni_str!("getTimestamp"), jni_sig!("()J"), &[])?
        .j()?;
    Ok(Some(time))
}

fn clip_description<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> jni::errors::Result<Option<JObject<'local>>> {
    let manager = clipboard_manager(env, context)?;
    let description = env
        .call_method(
            &manager,
            jni_str!("getPrimaryClipDescription"),
            jni_sig!("()Landroid/content/ClipDescription;"),
            &[],
        )?
        .l()?;
    Ok((!description.is_null()).then_some(description))
}

/// The clip's types and time, from its description; none without a clip.
fn listed_clip(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<Option<Listing>> {
    let Some(description) = clip_description(env, context)? else {
        return Ok(None);
    };
    let count = env
        .call_method(
            &description,
            jni_str!("getMimeTypeCount"),
            jni_sig!("()I"),
            &[],
        )?
        .i()?;
    let mut types: Vec<String> = Vec::new();
    for index in 0..count {
        let mime = env
            .call_method(
                &description,
                jni_str!("getMimeType"),
                jni_sig!("(I)Ljava/lang/String;"),
                &[JValue::Int(index)],
            )?
            .l()?;
        let mime = env.cast_local::<JString>(mime)?.try_to_string(env)?;
        for offered in offered_as(&mime) {
            if !types.contains(&offered) {
                types.push(offered);
            }
        }
    }
    Ok(Some(Listing {
        types,
        time: clip_time(env, context)?,
    }))
}

/// The types a clip's MIME type `mime` is read in: text and HTML as they are, Android's intents
/// not at all, and anything else as the content of a URI, which is also read as its URL.
fn offered_as(mime: &str) -> Vec<String> {
    match mime {
        TEXT | HTML | URI_LIST => vec![mime.to_owned()],
        "text/vnd.android.intent" => Vec::new(),
        _ => vec![mime.to_owned(), URI_LIST.to_owned()],
    }
}

/// The clip's first item holding `mime`, read, if the clip is still the one `listed`.
fn read_clip(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    listed: &Listing,
    mime: &str,
) -> jni::errors::Result<Result<Vec<u8>, ClipboardError>> {
    if clip_time(env, context)? != listed.time {
        return Ok(Err(ClipboardError::Changed));
    }
    let manager = clipboard_manager(env, context)?;
    let clip = env
        .call_method(
            &manager,
            jni_str!("getPrimaryClip"),
            jni_sig!("()Landroid/content/ClipData;"),
            &[],
        )?
        .l()?;
    if clip.is_null() {
        return Ok(Err(ClipboardError::Changed));
    }
    let count = env
        .call_method(&clip, jni_str!("getItemCount"), jni_sig!("()I"), &[])?
        .i()?;
    for index in 0..count {
        let item = env
            .call_method(
                &clip,
                jni_str!("getItemAt"),
                jni_sig!("(I)Landroid/content/ClipData$Item;"),
                &[JValue::Int(index)],
            )?
            .l()?;
        if let Some(bytes) = read_item(env, context, &item, mime)? {
            return Ok(Ok(bytes));
        }
    }
    Ok(Err(ClipboardError::NotFound))
}

/// One item of a clip in `mime`; none when it does not hold it.
fn read_item(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    item: &JObject<'_>,
    mime: &str,
) -> jni::errors::Result<Option<Vec<u8>>> {
    match mime {
        TEXT => {
            let text = env
                .call_method(
                    item,
                    jni_str!("getText"),
                    jni_sig!("()Ljava/lang/CharSequence;"),
                    &[],
                )?
                .l()?;
            string_of(env, &text).map(|text| text.map(String::into_bytes))
        }
        HTML => {
            let html = env
                .call_method(
                    item,
                    jni_str!("getHtmlText"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            string_of(env, &html).map(|html| html.map(String::into_bytes))
        }
        URI_LIST => {
            let uri = uri_of(env, item)?;
            let url = string_of(env, &uri)?;
            Ok(url.map(|url| format!("{url}\r\n").into_bytes()))
        }
        _ => {
            let uri = uri_of(env, item)?;
            if uri.is_null() {
                return Ok(None);
            }
            content_of(env, context, &uri).map(Some)
        }
    }
}

fn uri_of<'local>(
    env: &mut Env<'local>,
    item: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        item,
        jni_str!("getUri"),
        jni_sig!("()Landroid/net/Uri;"),
        &[],
    )?
    .l()
}

/// `object`'s `toString`; none for null.
fn string_of(env: &mut Env<'_>, object: &JObject<'_>) -> jni::errors::Result<Option<String>> {
    if object.is_null() {
        return Ok(None);
    }
    let string = env
        .call_method(
            object,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    env.cast_local::<JString>(string)?
        .try_to_string(env)
        .map(Some)
}

/// What `uri` names, read whole through the activity's content resolver.
fn content_of(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    uri: &JObject<'_>,
) -> jni::errors::Result<Vec<u8>> {
    const CHUNK: usize = 64 * 1024;
    let resolver = env
        .call_method(
            context,
            jni_str!("getContentResolver"),
            jni_sig!("()Landroid/content/ContentResolver;"),
            &[],
        )?
        .l()?;
    let stream = env
        .call_method(
            &resolver,
            jni_str!("openInputStream"),
            jni_sig!("(Landroid/net/Uri;)Ljava/io/InputStream;"),
            &[JValue::Object(uri)],
        )?
        .l()?;
    let buffer: JByteArray<'_> = env.new_byte_array(CHUNK)?;
    let mut chunk = vec![0i8; CHUNK];
    let mut content = Vec::new();
    loop {
        let read = env
            .call_method(
                &stream,
                jni_str!("read"),
                jni_sig!("([B)I"),
                &[JValue::Object(&buffer)],
            )?
            .i()?;
        let Ok(read) = usize::try_from(read) else {
            break;
        };
        buffer.get_region(env, 0, &mut chunk[..read])?;
        content.extend(chunk[..read].iter().map(|&byte| byte as u8));
    }
    env.call_method(&stream, jni_str!("close"), jni_sig!("()V"), &[])?;
    Ok(content)
}

/// Sets the clip to one item for each of `texts`, each text with its HTML where it has some.
fn set_clip(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    texts: &[(String, Option<String>)],
    options: CopyOptions,
) -> jni::errors::Result<()> {
    let manager = clipboard_manager(env, context)?;
    let description = clip_description_for(env, texts, options)?;
    let mut items = Vec::new();
    for (text, html) in texts {
        items.push(clip_item(env, text, html.as_deref())?);
    }
    let Some((first, rest)) = items.split_first() else {
        if device_api_level() >= CLEAR_CLIP {
            env.call_method(&manager, jni_str!("clearPrimaryClip"), jni_sig!("()V"), &[])?;
        }
        return Ok(());
    };
    let clip = env.new_object(
        jni_str!("android/content/ClipData"),
        jni_sig!("(Landroid/content/ClipDescription;Landroid/content/ClipData$Item;)V"),
        &[JValue::Object(&description), JValue::Object(first)],
    )?;
    for item in rest {
        env.call_method(
            &clip,
            jni_str!("addItem"),
            jni_sig!("(Landroid/content/ClipData$Item;)V"),
            &[JValue::Object(item)],
        )?;
    }
    env.call_method(
        &manager,
        jni_str!("setPrimaryClip"),
        jni_sig!("(Landroid/content/ClipData;)V"),
        &[JValue::Object(&clip)],
    )?;
    Ok(())
}

/// The description of a clip of `texts`: text, and HTML where any item has some, marked
/// sensitive as `options` says.
fn clip_description_for<'local>(
    env: &mut Env<'local>,
    texts: &[(String, Option<String>)],
    options: CopyOptions,
) -> jni::errors::Result<JObject<'local>> {
    let with_html = texts.iter().any(|(_, html)| html.is_some());
    let mimes: &[&str] = if with_html { &[TEXT, HTML] } else { &[TEXT] };
    let string_class = env.find_class(jni_str!("java/lang/String"))?;
    let array = env.new_object_array(mimes.len() as i32, &string_class, JObject::null())?;
    for (index, mime) in mimes.iter().enumerate() {
        let mime = env.new_string(mime)?;
        array.set_element(env, index, &mime)?;
    }
    let label = env.new_string("")?;
    let description = env.new_object(
        jni_str!("android/content/ClipDescription"),
        jni_sig!("(Ljava/lang/CharSequence;[Ljava/lang/String;)V"),
        &[JValue::Object(&label), JValue::Object(&array)],
    )?;
    if options.sensitive {
        let extras = env.new_object(
            jni_str!("android/os/PersistableBundle"),
            jni_sig!("()V"),
            &[],
        )?;
        let key = env.new_string(IS_SENSITIVE)?;
        env.call_method(
            &extras,
            jni_str!("putBoolean"),
            jni_sig!("(Ljava/lang/String;Z)V"),
            &[JValue::Object(&key), JValue::Bool(true)],
        )?;
        env.call_method(
            &description,
            jni_str!("setExtras"),
            jni_sig!("(Landroid/os/PersistableBundle;)V"),
            &[JValue::Object(&extras)],
        )?;
    }
    Ok(description)
}

/// A clip's item of `text`, with `html` beside it where there is some.
fn clip_item<'local>(
    env: &mut Env<'local>,
    text: &str,
    html: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    let text = env.new_string(text)?;
    match html {
        Some(html) => {
            let html = env.new_string(html)?;
            env.new_object(
                jni_str!("android/content/ClipData$Item"),
                jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/String;)V"),
                &[JValue::Object(&text), JValue::Object(&html)],
            )
        }
        None => env.new_object(
            jni_str!("android/content/ClipData$Item"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(&text)],
        ),
    }
}
