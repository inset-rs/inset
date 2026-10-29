//! The file dialogs: the Storage Access Framework's document pickers (`ACTION_OPEN_DOCUMENT`,
//! `ACTION_CREATE_DOCUMENT`), which the activity starts for a result. `NativeActivity` hands no
//! result to native code, so the dialogs need Inset's activity, `rs.inset.InsetActivity`, which
//! inset-cli builds into the app and whose `onActivityResult` calls the method registered here;
//! under another activity the dialogs are `None`.
//!
//! A document has no path, so each pick is opened when it is answered: `"rw"` for a writable pick,
//! which a provider answers with a file that seeks, and `"r"` otherwise, which it may answer with a
//! pipe. A pick that seeks is a handle, read and written at any offset in place; a pipe is a
//! stream, read once. The permission a pick grants is taken as persistable, so it outlives the
//! app's run: read only, unless the app asks to write (`FLAG_GRANT_WRITE_URI_PERMISSION`); a save
//! grants writing to the one document it makes.
//!
//! The pickers filter by MIME type alone: a type's extensions become the MIME types Android maps
//! them to. They take no initial folder the app could name by a path, and no confirming label. A
//! folder pick is the document tree `ACTION_OPEN_DOCUMENT_TREE` answers (`document_tree`).
//!
//! The result arrives on the activity's thread, which only hands the documents' URIs on; they are
//! opened on the app's thread, as the app awaits them.

// JNI hands the native method raw arguments, and a document arrives as a raw file descriptor;
// that is the `unsafe` here.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::os::fd::FromRawFd;
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use android_activity::AndroidApp;
use inset_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FileFinishFuture, FileHandle, FileOpenFuture,
    FileReadFuture, FileSizeFuture, FileStream, FileType, FileWriteFuture, FileWriter,
    FolderOptions, OpenOptions, PickedFile, PickedFolder, SaveOptions, ViewId,
};
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};

use crate::activity::with_activity;
use crate::answer::{Answer, Answered, answer};
use crate::document_tree::DocumentFolder;

/// `Activity.RESULT_OK`.
const RESULT_OK: jint = -1;
/// `Intent.FLAG_GRANT_READ_URI_PERMISSION`.
const GRANT_READ: jint = 0x1;
/// `Intent.FLAG_GRANT_WRITE_URI_PERMISSION`.
const GRANT_WRITE: jint = 0x2;
/// `Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION`.
const GRANT_PERSISTABLE: jint = 0x40;

/// The pickers, where the activity is Inset's.
pub(crate) struct DocumentPickers {
    app: AndroidApp,
}

impl DocumentPickers {
    /// The pickers, once the activity's result method is registered; `None` under an activity
    /// that is not Inset's, which hands no result on.
    pub(crate) fn new(app: AndroidApp) -> Option<DocumentPickers> {
        with_activity(&app, |env, activity| {
            let class = env.get_object_class(activity)?;
            let method = [
                // SAFETY: `on_result` takes the arguments `(IILandroid/content/Intent;)V` names,
                // as a static method's native function does.
                unsafe {
                    NativeMethod::from_raw_parts(
                        jni_str!("onResult"),
                        jni_str!("(IILandroid/content/Intent;)V"),
                        on_result as *mut std::ffi::c_void,
                    )
                },
            ];
            // SAFETY: the method's function matches its signature, above.
            let registered = unsafe { env.register_native_methods(&class, &method) };
            if registered.is_err() {
                env.exception_clear();
            }
            Ok(registered.is_ok())
        })
        .unwrap_or(false)
        .then_some(DocumentPickers { app })
    }

    /// Starts the picker `intent` makes for a result, and answers the URIs of the documents
    /// picked, none if the user cancelled.
    fn pick(
        &self,
        intent: impl for<'local> FnOnce(&mut Env<'local>) -> jni::errors::Result<JObject<'local>>,
    ) -> UrisPicked {
        let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = one_shot();
        locked(&PENDING).insert(request, answer);
        let started = with_activity(&self.app, |env, activity| {
            let intent = intent(env)?;
            env.call_method(
                activity,
                jni_str!("startActivityForResult"),
                jni_sig!("(Landroid/content/Intent;I)V"),
                &[JValue::Object(&intent), JValue::Int(request)],
            )?;
            Ok(())
        });
        if let Err(error) = started {
            locked(&PENDING).remove(&request);
            let _ = with_activity(&self.app, |env, _| {
                env.exception_clear();
                Ok(())
            });
            let (failed, answered) = one_shot();
            failed.give(Err(FileDialogError::Failed(format!(
                "the picker did not start: {error}"
            ))));
            return answered;
        }
        answered
    }

    fn open(&self, options: &OpenOptions, multiple: bool) -> FileDialogFuture<Vec<PickedFile>> {
        let writable = options.writable;
        let types = options.types.clone();
        let picked = self.pick(move |env| {
            let mime_types = mime_types(env, &types)?;
            let intent = intent(env, "android.intent.action.OPEN_DOCUMENT")?;
            set_type(env, &intent, &mime_types)?;
            if multiple {
                put_boolean(env, &intent, "android.intent.extra.ALLOW_MULTIPLE", true)?;
            }
            let flags = GRANT_READ | GRANT_PERSISTABLE | if writable { GRANT_WRITE } else { 0 };
            add_flags(env, &intent, flags)?;
            Ok(intent)
        });
        let app = self.app.clone();
        Box::pin(async move {
            let uris = picked.await?;
            uris.iter()
                .map(|uri| open_document(&app, uri, writable))
                .collect()
        })
    }
}

impl FileDialogs for DocumentPickers {
    fn open_file(
        &self,
        _parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let picked = self.open(&options, false);
        Box::pin(async move { Ok(picked.await?.into_iter().next()) })
    }

    fn open_files(
        &self,
        _parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        self.open(&options, true)
    }

    fn open_folder(
        &self,
        _parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>> {
        let writable = options.writable;
        let picked = self.pick(move |env| {
            let intent = env.new_string("android.intent.action.OPEN_DOCUMENT_TREE")?;
            let intent = env.new_object(
                jni_str!("android/content/Intent"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&intent)],
            )?;
            let flags = GRANT_READ | GRANT_PERSISTABLE | if writable { GRANT_WRITE } else { 0 };
            add_flags(env, &intent, flags)?;
            Ok(intent)
        });
        let app = self.app.clone();
        Box::pin(async move {
            let Some(tree) = picked.await?.into_iter().next() else {
                return Ok(None);
            };
            let folder = DocumentFolder::picked(app, &tree, writable)
                .map_err(|error| FileDialogError::Failed(error.to_string()))?;
            Ok(Some(PickedFolder::Handle(Box::new(folder))))
        })
    }

    fn save_file(
        &self,
        _parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let picked = self.pick(move |env| {
            let mime_types = mime_types(env, &options.types)?;
            let intent = intent(env, "android.intent.action.CREATE_DOCUMENT")?;
            let mime = mime_types
                .first()
                .map_or("application/octet-stream", String::as_str);
            set_type(env, &intent, &[mime.to_owned()])?;
            if let Some(name) = &options.suggested_name {
                put_string(env, &intent, "android.intent.extra.TITLE", name)?;
            }
            add_flags(env, &intent, GRANT_READ | GRANT_WRITE | GRANT_PERSISTABLE)?;
            Ok(intent)
        });
        let app = self.app.clone();
        Box::pin(async move {
            let uris = picked.await?;
            uris.first()
                .map(|uri| open_document(&app, uri, true))
                .transpose()
        })
    }
}

/// An intent of `action` for documents that open, as the pickers ask.
fn intent<'local>(env: &mut Env<'local>, action: &str) -> jni::errors::Result<JObject<'local>> {
    let action = env.new_string(action)?;
    let intent = env.new_object(
        jni_str!("android/content/Intent"),
        jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&action)],
    )?;
    let openable = env.new_string("android.intent.category.OPENABLE")?;
    env.call_method(
        &intent,
        jni_str!("addCategory"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&openable)],
    )?;
    Ok(intent)
}

/// The intent's type: the one MIME type, or any with the list as `EXTRA_MIME_TYPES`.
fn set_type(
    env: &mut Env<'_>,
    intent: &JObject<'_>,
    mime_types: &[String],
) -> jni::errors::Result<()> {
    let single = match mime_types {
        [mime] => mime.as_str(),
        _ => "*/*",
    };
    let single = env.new_string(single)?;
    env.call_method(
        intent,
        jni_str!("setType"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&single)],
    )?;
    if mime_types.len() > 1 {
        let list: JObjectArray<'_, JString<'_>> =
            env.new_object_type_array::<JString>(mime_types.len(), JString::null())?;
        for (index, mime) in mime_types.iter().enumerate() {
            let mime = env.new_string(mime)?;
            list.set_element(env, index, &mime)?;
        }
        let key = env.new_string("android.intent.extra.MIME_TYPES")?;
        env.call_method(
            intent,
            jni_str!("putExtra"),
            jni_sig!("(Ljava/lang/String;[Ljava/lang/String;)Landroid/content/Intent;"),
            &[JValue::Object(&key), JValue::Object(&list)],
        )?;
    }
    Ok(())
}

fn put_boolean(
    env: &mut Env<'_>,
    intent: &JObject<'_>,
    key: &str,
    value: bool,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    env.call_method(
        intent,
        jni_str!("putExtra"),
        jni_sig!("(Ljava/lang/String;Z)Landroid/content/Intent;"),
        &[JValue::Object(&key), JValue::Bool(value)],
    )?;
    Ok(())
}

fn put_string(
    env: &mut Env<'_>,
    intent: &JObject<'_>,
    key: &str,
    value: &str,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    let value = env.new_string(value)?;
    env.call_method(
        intent,
        jni_str!("putExtra"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    Ok(())
}

fn add_flags(env: &mut Env<'_>, intent: &JObject<'_>, flags: jint) -> jni::errors::Result<()> {
    env.call_method(
        intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(flags)],
    )?;
    Ok(())
}

/// The MIME types `types` name: their own, and those Android maps their extensions to.
fn mime_types(env: &mut Env<'_>, types: &[FileType]) -> jni::errors::Result<Vec<String>> {
    let mut mime_types: Vec<String> = types
        .iter()
        .flat_map(|kind| kind.mime_types.iter().cloned())
        .collect();
    let extensions: Vec<&String> = types.iter().flat_map(|kind| &kind.extensions).collect();
    if !extensions.is_empty() {
        let map = env
            .call_static_method(
                jni_str!("android/webkit/MimeTypeMap"),
                jni_str!("getSingleton"),
                jni_sig!("()Landroid/webkit/MimeTypeMap;"),
                &[],
            )?
            .l()?;
        for extension in extensions {
            let extension = env.new_string(extension.trim_start_matches('.'))?;
            let mime = env
                .call_method(
                    &map,
                    jni_str!("getMimeTypeFromExtension"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &[JValue::Object(&extension)],
                )?
                .l()?;
            if let Some(mime) = string(env, mime)?
                && !mime_types.contains(&mime)
            {
                mime_types.push(mime);
            }
        }
    }
    Ok(mime_types)
}

pub(crate) fn string(
    env: &mut Env<'_>,
    object: JObject<'_>,
) -> jni::errors::Result<Option<String>> {
    if object.is_null() {
        return Ok(None);
    }
    env.cast_local::<JString>(object)?
        .try_to_string(env)
        .map(Some)
}

/// The picks waiting for their results, by request code.
static PENDING: LazyLock<Mutex<HashMap<jint, Answer<Picked>>>> = LazyLock::new(Mutex::default);
/// The request codes, counted from one.
static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1);

/// What `InsetActivity.onActivityResult` hands on, on the activity's thread: the URIs of the
/// documents picked, as strings, which the app's thread opens.
extern "system" fn on_result<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    request: jint,
    result: jint,
    data: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let Some(answer) = locked(&PENDING).remove(&request) else {
            return Ok(());
        };
        if result != RESULT_OK || data.is_null() {
            answer.give(Ok(Vec::new()));
            return Ok(());
        }
        answer.give(Ok(picked_uris(env, &data)?));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// The URIs a picker's result carries: its clip's, for several, or its data's.
fn picked_uris(env: &mut Env<'_>, data: &JObject<'_>) -> jni::errors::Result<Vec<String>> {
    let clip = env
        .call_method(
            data,
            jni_str!("getClipData"),
            jni_sig!("()Landroid/content/ClipData;"),
            &[],
        )?
        .l()?;
    if !clip.is_null() {
        let count = env
            .call_method(&clip, jni_str!("getItemCount"), jni_sig!("()I"), &[])?
            .i()?;
        let mut uris = Vec::new();
        for index in 0..count {
            let item = env
                .call_method(
                    &clip,
                    jni_str!("getItemAt"),
                    jni_sig!("(I)Landroid/content/ClipData$Item;"),
                    &[JValue::Int(index)],
                )?
                .l()?;
            let uri = env
                .call_method(
                    &item,
                    jni_str!("getUri"),
                    jni_sig!("()Landroid/net/Uri;"),
                    &[],
                )?
                .l()?;
            uris.extend(uri_string(env, uri)?);
        }
        return Ok(uris);
    }
    let uri = env
        .call_method(
            data,
            jni_str!("getData"),
            jni_sig!("()Landroid/net/Uri;"),
            &[],
        )?
        .l()?;
    Ok(uri_string(env, uri)?.into_iter().collect())
}

fn uri_string(env: &mut Env<'_>, uri: JObject<'_>) -> jni::errors::Result<Option<String>> {
    if uri.is_null() {
        return Ok(None);
    }
    let text = env
        .call_method(
            &uri,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    string(env, text)
}

/// Opens the document `uri` names, keeping its permission: a handle where it seeks, a stream
/// where it is a pipe.
pub(crate) fn open_document(
    app: &AndroidApp,
    uri: &str,
    writable: bool,
) -> Result<PickedFile, FileDialogError> {
    let (fd, name) = with_activity(app, |env, activity| {
        let resolver = env
            .call_method(
                activity,
                jni_str!("getContentResolver"),
                jni_sig!("()Landroid/content/ContentResolver;"),
                &[],
            )?
            .l()?;
        let text = env.new_string(uri)?;
        let uri = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("parse"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[JValue::Object(&text)],
            )?
            .l()?;
        keep_permission(env, &resolver, &uri, writable);
        let name = display_name(env, &resolver, &uri).unwrap_or_else(|_| {
            env.exception_clear();
            None
        });
        let mode = env.new_string(if writable { "rw" } else { "r" })?;
        let descriptor = env
            .call_method(
                &resolver,
                jni_str!("openFileDescriptor"),
                jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;"),
                &[JValue::Object(&uri), JValue::Object(&mode)],
            )?
            .l()?;
        let fd = env
            .call_method(&descriptor, jni_str!("detachFd"), jni_sig!("()I"), &[])?
            .i()?;
        Ok((fd, name))
    })
    .map_err(|error| {
        let _ = with_activity(app, |env, _| {
            env.exception_clear();
            Ok(())
        });
        FileDialogError::Failed(format!("the document could not be opened: {error}"))
    })?;
    // SAFETY: `detachFd` handed this descriptor over; the file owns it from here.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let name = name.unwrap_or_default();
    let uri = uri.to_owned();
    Ok(if file.stream_position().is_ok() {
        PickedFile::Handle(Box::new(Document {
            file,
            name,
            uri,
            writable,
        }))
    } else {
        PickedFile::Stream(Box::new(DocumentStream { file, name, uri }))
    })
}

/// Takes the pick's permission as persistable, which a provider may decline.
pub(crate) fn keep_permission(
    env: &mut Env<'_>,
    resolver: &JObject<'_>,
    uri: &JObject<'_>,
    writable: bool,
) {
    let flags = GRANT_READ | if writable { GRANT_WRITE } else { 0 };
    let taken = env.call_method(
        resolver,
        jni_str!("takePersistableUriPermission"),
        jni_sig!("(Landroid/net/Uri;I)V"),
        &[JValue::Object(uri), JValue::Int(flags)],
    );
    if taken.is_err() {
        env.exception_clear();
    }
}

/// The document's name, as its provider shows it.
fn display_name(
    env: &mut Env<'_>,
    resolver: &JObject<'_>,
    uri: &JObject<'_>,
) -> jni::errors::Result<Option<String>> {
    let none = JObject::null();
    let cursor = env
        .call_method(
            resolver,
            jni_str!("query"),
            jni_sig!("(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;"),
            &[
                JValue::Object(uri),
                JValue::Object(&none),
                JValue::Object(&none),
                JValue::Object(&none),
                JValue::Object(&none),
            ],
        )?
        .l()?;
    if cursor.is_null() {
        return Ok(None);
    }
    let column = env.new_string("_display_name")?;
    let index = env
        .call_method(
            &cursor,
            jni_str!("getColumnIndex"),
            jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&column)],
        )?
        .i()?;
    let first = env
        .call_method(&cursor, jni_str!("moveToFirst"), jni_sig!("()Z"), &[])?
        .z()?;
    let name = if first && index >= 0 {
        let name = env
            .call_method(
                &cursor,
                jni_str!("getString"),
                jni_sig!("(I)Ljava/lang/String;"),
                &[JValue::Int(index)],
            )?
            .l()?;
        string(env, name)?
    } else {
        None
    };
    env.call_method(&cursor, jni_str!("close"), jni_sig!("()V"), &[])?;
    Ok(name)
}

/// A picked document that seeks: read and, when writable, written in place at any offset.
struct Document {
    file: File,
    name: String,
    /// The `content:` URI the document was picked as, which any app it is granted to reads.
    uri: String,
    writable: bool,
}

impl FileHandle for Document {
    fn name(&self) -> &str {
        &self.name
    }

    fn url(&self) -> String {
        self.uri.clone()
    }

    fn size(&self) -> FileSizeFuture {
        let size = self.file.metadata().map(|metadata| metadata.len());
        Box::pin(std::future::ready(size))
    }

    fn read_at<'a>(&'a self, offset: u64, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        Box::pin(std::future::ready(self.file.read_at(buf, offset)))
    }

    fn open_write(&self) -> Option<FileOpenFuture<Box<dyn FileWriter>>> {
        if !self.writable {
            return None;
        }
        let writer = self
            .file
            .try_clone()
            .map(|file| Box::new(DocumentWriter { file }) as Box<dyn FileWriter>);
        Some(Box::pin(std::future::ready(writer)))
    }
}

/// Writing a picked document, which lands in place as each write is made.
struct DocumentWriter {
    file: File,
}

impl FileWriter for DocumentWriter {
    fn write_at<'a>(&'a mut self, offset: u64, bytes: &'a [u8]) -> FileWriteFuture<'a> {
        Box::pin(std::future::ready(self.file.write_all_at(bytes, offset)))
    }

    fn set_len(&mut self, len: u64) -> FileWriteFuture<'_> {
        Box::pin(std::future::ready(self.file.set_len(len)))
    }

    /// The writes are in place already; finishing has the provider's file hold them.
    fn finish(self: Box<Self>) -> FileFinishFuture {
        Box::pin(std::future::ready(self.file.sync_all()))
    }
}

/// A picked document that is a pipe: read once, start to end.
struct DocumentStream {
    file: File,
    name: String,
    /// The `content:` URI the document was picked as.
    uri: String,
}

impl FileStream for DocumentStream {
    fn name(&self) -> &str {
        &self.name
    }

    fn url(&self) -> String {
        self.uri.clone()
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> FileReadFuture<'a> {
        Box::pin(std::future::ready(self.file.read(buf)))
    }
}

/// The URIs a picker answers, on their way from the activity's thread to the app's.
type Picked = Result<Vec<String>, FileDialogError>;

/// The app's side of a pick: the URIs, once the result came.
type UrisPicked = Answered<Picked>;

/// A pick's answer, a failure if the picker ended without one.
fn one_shot() -> (Answer<Picked>, UrisPicked) {
    answer(Err(FileDialogError::Failed(
        "the picker ended without an answer".to_owned(),
    )))
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
