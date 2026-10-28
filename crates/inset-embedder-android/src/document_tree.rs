//! A picked folder: the document tree `ACTION_OPEN_DOCUMENT_TREE` answers, reached through
//! `DocumentsContract`. Its permission is taken as persistable when it is picked, read only unless
//! the app asked to write, and covers every document in it. A folder is a document of the tree,
//! named by its id: listed by querying its children, a child file opened as a picked document is
//! (`file_dialogs`), a child folder as another of these, and, when writable, files and folders
//! made by `DocumentsContract.createDocument`.
//!
//! Every call reaches the provider through JNI on the app's thread, as the app awaits it.

use android_activity::AndroidApp;
use inset_embedder::{
    EntryKind, FileOpenFuture, FolderEntriesFuture, FolderEntry, FolderHandle, PickedFile,
    PickedFolder,
};
use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::activity::with_activity;
use crate::file_dialogs::{keep_permission, open_document, string};

/// `DocumentsContract.Document.MIME_TYPE_DIR`.
const FOLDER_MIME: &str = "vnd.android.document/directory";
/// The MIME type of a file whose name says none.
const ANY_BYTES: &str = "application/octet-stream";

/// A folder of a picked document tree.
pub(crate) struct DocumentFolder {
    app: AndroidApp,
    /// The tree the user picked, whose permission covers this folder.
    tree: String,
    /// This folder's document id in the tree.
    document: String,
    name: String,
    writable: bool,
}

/// One child of a folder, as its provider lists it.
struct Child {
    document: String,
    name: String,
    folder: bool,
}

impl DocumentFolder {
    /// The folder at the root of the tree `tree` names, its permission kept.
    pub(crate) fn picked(app: AndroidApp, tree: &str, writable: bool) -> std::io::Result<Self> {
        let (document, name) = reach(&app, |env, activity| {
            let resolver = resolver(env, activity)?;
            let tree_uri = parse(env, tree)?;
            keep_permission(env, &resolver, &tree_uri, writable);
            let document = env
                .call_static_method(
                    jni_str!("android/provider/DocumentsContract"),
                    jni_str!("getTreeDocumentId"),
                    jni_sig!("(Landroid/net/Uri;)Ljava/lang/String;"),
                    &[JValue::Object(&tree_uri)],
                )?
                .l()?;
            let document = string(env, document)?.unwrap_or_default();
            let uri = document_uri(env, &tree_uri, &document)?;
            let name = query_children_or_self(env, &resolver, &uri)?
                .into_iter()
                .next()
                .map(|child| child.name)
                .unwrap_or_default();
            Ok((document, name))
        })
        .map_err(java_error)?;
        Ok(DocumentFolder {
            app,
            tree: tree.to_owned(),
            document,
            name,
            writable,
        })
    }

    /// The folder's children, as its provider lists them.
    fn children(&self) -> std::io::Result<Vec<Child>> {
        reach(&self.app, |env, activity| {
            let resolver = resolver(env, activity)?;
            let tree = parse(env, &self.tree)?;
            let document = env.new_string(&self.document)?;
            let children = env
                .call_static_method(
                    jni_str!("android/provider/DocumentsContract"),
                    jni_str!("buildChildDocumentsUriUsingTree"),
                    jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;"),
                    &[JValue::Object(&tree), JValue::Object(&document)],
                )?
                .l()?;
            query_children_or_self(env, &resolver, &children)
        })
        .map_err(java_error)
    }

    /// The child `name`, if the folder has one.
    fn child(&self, name: &str) -> std::io::Result<Child> {
        self.children()?
            .into_iter()
            .find(|child| child.name == name)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("no {name} here"))
            })
    }

    /// The URI of the document `document` of this folder's tree.
    fn uri_of(&self, document: &str) -> std::io::Result<String> {
        reach(&self.app, |env, _| {
            let tree = parse(env, &self.tree)?;
            let uri = document_uri(env, &tree, document)?;
            Ok(uri_text(env, &uri)?.unwrap_or_default())
        })
        .map_err(java_error)
    }

    fn file(&self, document: &str) -> std::io::Result<PickedFile> {
        let uri = self.uri_of(document)?;
        open_document(&self.app, &uri, self.writable)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }

    fn folder(&self, document: String, name: String) -> PickedFolder {
        PickedFolder::Handle(Box::new(DocumentFolder {
            app: self.app.clone(),
            tree: self.tree.clone(),
            document,
            name,
            writable: self.writable,
        }))
    }

    /// Makes a document `name` of `mime` in this folder; answers its id.
    fn create(&self, mime: &str, name: &str) -> std::io::Result<String> {
        reach(&self.app, |env, activity| {
            let resolver = resolver(env, activity)?;
            let tree = parse(env, &self.tree)?;
            let parent = document_uri(env, &tree, &self.document)?;
            let mime = env.new_string(mime)?;
            let name = env.new_string(name)?;
            let made = env
                .call_static_method(
                    jni_str!("android/provider/DocumentsContract"),
                    jni_str!("createDocument"),
                    jni_sig!("(Landroid/content/ContentResolver;Landroid/net/Uri;Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;"),
                    &[
                        JValue::Object(&resolver),
                        JValue::Object(&parent),
                        JValue::Object(&mime),
                        JValue::Object(&name),
                    ],
                )?
                .l()?;
            if made.is_null() {
                return Err(jni::errors::Error::NullPtr("the provider made nothing"));
            }
            let document = env
                .call_static_method(
                    jni_str!("android/provider/DocumentsContract"),
                    jni_str!("getDocumentId"),
                    jni_sig!("(Landroid/net/Uri;)Ljava/lang/String;"),
                    &[JValue::Object(&made)],
                )?
                .l()?;
            Ok(string(env, document)?.unwrap_or_default())
        })
        .map_err(java_error)
    }

    /// The MIME type a file named `name` is made with: the one Android maps its extension to.
    fn mime_of(&self, name: &str) -> String {
        let Some((_, extension)) = name.rsplit_once('.') else {
            return ANY_BYTES.to_owned();
        };
        reach(&self.app, |env, _| {
            let map = env
                .call_static_method(
                    jni_str!("android/webkit/MimeTypeMap"),
                    jni_str!("getSingleton"),
                    jni_sig!("()Landroid/webkit/MimeTypeMap;"),
                    &[],
                )?
                .l()?;
            let extension = env.new_string(extension)?;
            let mime = env
                .call_method(
                    &map,
                    jni_str!("getMimeTypeFromExtension"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &[JValue::Object(&extension)],
                )?
                .l()?;
            string(env, mime)
        })
        .ok()
        .flatten()
        .unwrap_or_else(|| ANY_BYTES.to_owned())
    }
}

fn ready<T: 'static>(value: std::io::Result<T>) -> FileOpenFuture<T> {
    Box::pin(std::future::ready(value))
}

impl FolderHandle for DocumentFolder {
    fn name(&self) -> &str {
        &self.name
    }

    fn entries(&self) -> FolderEntriesFuture {
        let entries = self.children().map(|children| {
            children
                .into_iter()
                .map(|child| FolderEntry {
                    name: child.name,
                    kind: if child.folder {
                        EntryKind::Folder
                    } else {
                        EntryKind::File
                    },
                })
                .collect()
        });
        Box::pin(std::future::ready(entries))
    }

    fn open_file(&self, name: &str) -> FileOpenFuture<PickedFile> {
        ready(
            self.child(name)
                .and_then(|child| self.file(&child.document)),
        )
    }

    fn open_folder(&self, name: &str) -> FileOpenFuture<PickedFolder> {
        let child = self.child(name).and_then(|child| {
            if child.folder {
                Ok(self.folder(child.document, child.name))
            } else {
                Err(std::io::Error::other(format!("{name} is no folder")))
            }
        });
        ready(child)
    }

    fn create_file(&self, name: &str) -> Option<FileOpenFuture<PickedFile>> {
        if !self.writable {
            return None;
        }
        let mime = self.mime_of(name);
        Some(ready(
            self.create(&mime, name)
                .and_then(|document| self.file(&document)),
        ))
    }

    fn create_folder(&self, name: &str) -> Option<FileOpenFuture<PickedFolder>> {
        if !self.writable {
            return None;
        }
        Some(ready(
            self.create(FOLDER_MIME, name)
                .map(|document| self.folder(document, name.to_owned())),
        ))
    }
}

/// The documents a query of `uri` answers: a folder's children, or the one document itself.
fn query_children_or_self(
    env: &mut Env<'_>,
    resolver: &JObject<'_>,
    uri: &JObject<'_>,
) -> jni::errors::Result<Vec<Child>> {
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
        return Ok(Vec::new());
    }
    let id_column = column(env, &cursor, "document_id")?;
    let name_column = column(env, &cursor, "_display_name")?;
    let mime_column = column(env, &cursor, "mime_type")?;
    let mut children = Vec::new();
    while env
        .call_method(&cursor, jni_str!("moveToNext"), jni_sig!("()Z"), &[])?
        .z()?
    {
        let document = cell(env, &cursor, id_column)?.unwrap_or_default();
        let name = cell(env, &cursor, name_column)?.unwrap_or_default();
        let mime = cell(env, &cursor, mime_column)?.unwrap_or_default();
        children.push(Child {
            document,
            name,
            folder: mime == FOLDER_MIME,
        });
    }
    env.call_method(&cursor, jni_str!("close"), jni_sig!("()V"), &[])?;
    Ok(children)
}

fn column(env: &mut Env<'_>, cursor: &JObject<'_>, name: &str) -> jni::errors::Result<i32> {
    let name = env.new_string(name)?;
    env.call_method(
        cursor,
        jni_str!("getColumnIndex"),
        jni_sig!("(Ljava/lang/String;)I"),
        &[JValue::Object(&name)],
    )?
    .i()
}

/// The text of `column` in the cursor's current row; none for a column it does not have.
fn cell(
    env: &mut Env<'_>,
    cursor: &JObject<'_>,
    column: i32,
) -> jni::errors::Result<Option<String>> {
    if column < 0 {
        return Ok(None);
    }
    let text = env
        .call_method(
            cursor,
            jni_str!("getString"),
            jni_sig!("(I)Ljava/lang/String;"),
            &[JValue::Int(column)],
        )?
        .l()?;
    string(env, text)
}

fn resolver<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("getContentResolver"),
        jni_sig!("()Landroid/content/ContentResolver;"),
        &[],
    )?
    .l()
}

fn parse<'local>(env: &mut Env<'local>, uri: &str) -> jni::errors::Result<JObject<'local>> {
    let text = env.new_string(uri)?;
    env.call_static_method(
        jni_str!("android/net/Uri"),
        jni_str!("parse"),
        jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
        &[JValue::Object(&text)],
    )?
    .l()
}

/// The URI of the document `document` of the tree `tree`.
fn document_uri<'local>(
    env: &mut Env<'local>,
    tree: &JObject<'_>,
    document: &str,
) -> jni::errors::Result<JObject<'local>> {
    let document = env.new_string(document)?;
    env.call_static_method(
        jni_str!("android/provider/DocumentsContract"),
        jni_str!("buildDocumentUriUsingTree"),
        jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;"),
        &[JValue::Object(tree), JValue::Object(&document)],
    )?
    .l()
}

fn uri_text(env: &mut Env<'_>, uri: &JObject<'_>) -> jni::errors::Result<Option<String>> {
    let text = env
        .call_method(
            uri,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    string(env, text)
}

/// Runs `work` with the activity, as `with_activity` does, clearing the Java exception a failure
/// leaves, so the next call into Java is not refused.
fn reach<T>(
    app: &AndroidApp,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> jni::errors::Result<T> {
    let reached = with_activity(app, work);
    if reached.is_err() {
        let _ = with_activity(app, |env, _| {
            env.exception_clear();
            Ok(())
        });
    }
    reached
}

/// A JNI failure as an I/O error.
fn java_error(error: jni::errors::Error) -> std::io::Error {
    std::io::Error::other(format!("the provider failed: {error}"))
}
