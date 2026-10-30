//! Opening: intents to view. A link opens through `ACTION_VIEW` on its URI, in the app the system
//! resolves it to, which asks the user when several take it. A file of the app's opens through
//! Inset's file provider (`rs.inset.InsetFileProvider`, which inset-cli builds into the app and
//! declares at `<package>.inset.files`): no other app reads the app's storage by its path, so the
//! intent carries the provider's `content://` URI for the file, whose path is the file's, and grants
//! the app that opens it reading that one URI. Without the provider, opening a file fails. Android
//! has no file manager that shows a file in its folder, so revealing one fails.
//!
//! Whether some app takes a link is `PackageManager.resolveActivity`'s answer, which from Android
//! 11 sees only the apps whose intents the manifest's `<queries>` names. The application's
//! meta-data lists those schemes (`rs.inset.queried_schemes`, which inset-cli writes beside the
//! `<queries>`): for one of them, or before Android 11, finding no app is an answer; for another
//! scheme, `None`, unless an app was found.
//!
//! Each call starts the intent from the activity in front, on the app's thread, and answers at once;
//! what the package declares and which apps take a link the application answers.

use std::path::Path;

use inset_embedder::{OpenError, OpenFuture, Opener};
use jni::objects::{JObject, JValue};
use jni::strings::JNIStr;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use crate::activity::{Activities, clear_exception, device_api_level, with_application};
use crate::file_dialogs::string;

/// Android 11, from which an app sees only the apps its manifest's `<queries>` names.
const PACKAGE_VISIBILITY: i32 = 30;
/// `PackageManager.MATCH_DEFAULT_ONLY`: the activities an implicit intent would start.
const MATCH_DEFAULT_ONLY: jint = 0x10000;
/// `PackageManager.GET_META_DATA`.
const GET_META_DATA: jint = 0x80;
/// `Intent.FLAG_GRANT_READ_URI_PERMISSION`.
const GRANT_READ: jint = 0x1;
/// The meta-data key under which inset-cli lists the schemes `<queries>` names.
const QUERIED_SCHEMES_KEY: &str = "rs.inset.queried_schemes";
/// The type of a file whose extension names none.
const ANY_BYTES: &str = "application/octet-stream";

/// Intents to view, started from the activity in front.
pub(crate) struct IntentOpener {
    activities: Activities,
    /// The authority of Inset's file provider, where the app declares it.
    files: Option<String>,
    /// The schemes the manifest's `<queries>` names.
    queried_schemes: Vec<String>,
}

impl IntentOpener {
    pub(crate) fn new(activities: Activities) -> IntentOpener {
        let declared = with_application(|env, application| {
            Ok((
                file_provider(env, application)?,
                queried_schemes(env, application)?,
            ))
        });
        let (files, queried_schemes) = declared.unwrap_or_else(|_| {
            clear_exception();
            (None, Vec::new())
        });
        IntentOpener {
            activities,
            files,
            queried_schemes,
        }
    }

    /// Starts the intent `intent` makes, from the activity in front.
    fn start(
        &self,
        intent: impl for<'local> FnOnce(&mut Env<'local>) -> jni::errors::Result<JObject<'local>>,
    ) -> Result<(), OpenError> {
        let started = self.activities.with_front(|env, activity| {
            let started = intent(env).and_then(|intent| {
                env.call_method(
                    activity,
                    jni_str!("startActivity"),
                    jni_sig!("(Landroid/content/Intent;)V"),
                    &[JValue::Object(&intent)],
                )
                .map(drop)
            });
            Ok(started.map_err(|error| opening_error(env, &error)))
        });
        started.unwrap_or_else(|error| {
            clear_exception();
            Err(OpenError::Failed(error.to_string()))
        })
    }

    /// Whether some app takes the link, as `resolveActivity` sees it.
    fn resolves(&self, link: &str) -> Option<bool> {
        let resolved = with_application(|env, application| {
            let uri = parse(env, link)?;
            let intent = view_intent(env, &uri)?;
            let manager = package_manager(env, application)?;
            let info = env
                .call_method(
                    &manager,
                    jni_str!("resolveActivity"),
                    jni_sig!("(Landroid/content/Intent;I)Landroid/content/pm/ResolveInfo;"),
                    &[JValue::Object(&intent), JValue::Int(MATCH_DEFAULT_ONLY)],
                )?
                .l()?;
            Ok(!info.is_null())
        });
        match resolved {
            Ok(resolved) => Some(resolved),
            Err(_) => {
                clear_exception();
                None
            }
        }
    }
}

impl Opener for IntentOpener {
    fn open_link(&self, link: &str) -> OpenFuture {
        answered(self.start(|env| {
            let uri = parse(env, link)?;
            view_intent(env, &uri)
        }))
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        if !path.is_file() {
            return failed(format!("{} is not there", path.display()));
        }
        let Some(authority) = &self.files else {
            return failed(
                "the app declares no Inset file provider to share the file through; inset-cli declares one"
                    .to_owned(),
            );
        };
        let Some(file) = path.to_str() else {
            return failed(format!("{} is no Unicode path", path.display()));
        };
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        answered(self.start(|env| shared_file_intent(env, authority, file, extension.as_deref())))
    }

    fn reveal_file(&self, _path: &Path) -> OpenFuture {
        failed("Android has no file manager that shows a file in its folder".to_owned())
    }

    fn can_open_link(&self, link: &str) -> Option<bool> {
        let (scheme, _) = link.split_once(':')?;
        let scheme = scheme.to_ascii_lowercase();
        let resolved = self.resolves(link)?;
        let sees_every_app =
            device_api_level() < PACKAGE_VISIBILITY || self.queried_schemes.contains(&scheme);
        (resolved || sees_every_app).then_some(resolved)
    }
}

/// `ACTION_VIEW` on `uri`.
fn view_intent<'local>(
    env: &mut Env<'local>,
    uri: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    let action = env.new_string("android.intent.action.VIEW")?;
    env.new_object(
        jni_str!("android/content/Intent"),
        jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
        &[JValue::Object(&action), JValue::Object(uri)],
    )
}

/// `ACTION_VIEW` on the provider's URI for `file`, of the type its extension names, granting the
/// app that opens it reading that URI.
fn shared_file_intent<'local>(
    env: &mut Env<'local>,
    authority: &str,
    file: &str,
    extension: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    let builder = env.new_object(jni_str!("android/net/Uri$Builder"), jni_sig!("()V"), &[])?;
    build_uri(env, &builder, jni_str!("scheme"), "content")?;
    build_uri(env, &builder, jni_str!("authority"), authority)?;
    build_uri(env, &builder, jni_str!("path"), file)?;
    let uri = env
        .call_method(
            &builder,
            jni_str!("build"),
            jni_sig!("()Landroid/net/Uri;"),
            &[],
        )?
        .l()?;
    let action = env.new_string("android.intent.action.VIEW")?;
    let intent = env.new_object(
        jni_str!("android/content/Intent"),
        jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&action)],
    )?;
    let mime = match extension {
        Some(extension) => mime_type_of(env, extension)?,
        None => None,
    };
    let mime = env.new_string(mime.as_deref().unwrap_or(ANY_BYTES))?;
    env.call_method(
        &intent,
        jni_str!("setDataAndType"),
        jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&uri), JValue::Object(&mime)],
    )?;
    env.call_method(
        &intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(GRANT_READ)],
    )?;
    Ok(intent)
}

/// Sets one part of the URI `builder` builds.
fn build_uri(
    env: &mut Env<'_>,
    builder: &JObject<'_>,
    part: &JNIStr,
    value: &str,
) -> jni::errors::Result<()> {
    let value = env.new_string(value)?;
    env.call_method(
        builder,
        part,
        jni_sig!("(Ljava/lang/String;)Landroid/net/Uri$Builder;"),
        &[JValue::Object(&value)],
    )?;
    Ok(())
}

/// The MIME type Android maps `extension` to.
fn mime_type_of(env: &mut Env<'_>, extension: &str) -> jni::errors::Result<Option<String>> {
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
}

/// The authority of Inset's file provider, where the manifest declares it.
fn file_provider(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<Option<String>> {
    let package = env
        .call_method(
            context,
            jni_str!("getPackageName"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let Some(package) = string(env, package)? else {
        return Ok(None);
    };
    let authority = format!("{package}.inset.files");
    let manager = package_manager(env, context)?;
    let name = env.new_string(&authority)?;
    let provider = env
        .call_method(
            &manager,
            jni_str!("resolveContentProvider"),
            jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/ProviderInfo;"),
            &[JValue::Object(&name), JValue::Int(0)],
        )?
        .l()?;
    Ok((!provider.is_null()).then_some(authority))
}

/// The schemes the application's meta-data says the manifest's `<queries>` names.
fn queried_schemes(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<Vec<String>> {
    let manager = package_manager(env, context)?;
    let package = env
        .call_method(
            context,
            jni_str!("getPackageName"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let info = env
        .call_method(
            &manager,
            jni_str!("getApplicationInfo"),
            jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;"),
            &[JValue::Object(&package), JValue::Int(GET_META_DATA)],
        )?
        .l()?;
    let meta_data = env
        .get_field(&info, jni_str!("metaData"), jni_sig!("Landroid/os/Bundle;"))?
        .l()?;
    if meta_data.is_null() {
        return Ok(Vec::new());
    }
    let key = env.new_string(QUERIED_SCHEMES_KEY)?;
    let schemes = env
        .call_method(
            &meta_data,
            jni_str!("getString"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
            &[JValue::Object(&key)],
        )?
        .l()?;
    Ok(string(env, schemes)?
        .map(|schemes| schemes.split(',').map(str::to_owned).collect())
        .unwrap_or_default())
}

fn package_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        context,
        jni_str!("getPackageManager"),
        jni_sig!("()Landroid/content/pm/PackageManager;"),
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

/// What a failed call into Java means for opening: no app where no activity takes the intent. The
/// exception it left is cleared.
fn opening_error(env: &mut Env<'_>, error: &jni::errors::Error) -> OpenError {
    let Some(exception) = env.exception_occurred() else {
        return OpenError::Failed(error.to_string());
    };
    env.exception_clear();
    let no_app = env
        .is_instance_of(
            &exception,
            jni_str!("android/content/ActivityNotFoundException"),
        )
        .unwrap_or(false);
    if no_app {
        return OpenError::NoApp;
    }
    let described = env
        .call_method(
            &exception,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|text| text.l())
        .and_then(|text| string(env, text));
    match described {
        Ok(Some(text)) => OpenError::Failed(text),
        _ => {
            env.exception_clear();
            OpenError::Failed(error.to_string())
        }
    }
}

fn failed(reason: String) -> OpenFuture {
    answered(Err(OpenError::Failed(reason)))
}

fn answered(result: Result<(), OpenError>) -> OpenFuture {
    Box::pin(std::future::ready(result))
}
