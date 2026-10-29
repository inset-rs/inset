//! Being opened: the intent the activity started with, and each one that reaches it while it runs,
//! which Inset's activity hands on from `onNewIntent` (`NativeActivity` keeps it to itself). An
//! intent to view (`ACTION_VIEW`) asks to open its data: a `content://` document, opened as a
//! picked one is (`file_dialogs::open_document`), to write where the intent grants writing; a
//! `file://` URI as its path; any other URI as a link. Under an activity that is not Inset's, only
//! the intent the activity started with comes.
//!
//! An intent arrives on the activity's thread, which only records its URI and wakes the loop; the
//! loop opens the documents on the app's thread and hands the request on once the client is there,
//! so the intent the activity started with waits for the app to start.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use android_activity::{AndroidApp, AndroidAppWaker};
use inset_embedder::{OpenRequest, PickedFile, file_uri_path};
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};

use crate::activity::with_activity;
use crate::file_dialogs::{open_document, string};
use crate::log::warn_to_log;

/// `Intent.ACTION_VIEW`.
const VIEW: &str = "android.intent.action.VIEW";
/// `Intent.FLAG_GRANT_WRITE_URI_PERMISSION`.
const GRANT_WRITE: jint = 0x2;

/// The URIs intents to view brought, not yet handed on.
static ARRIVED: LazyLock<Mutex<Vec<Viewed>>> = LazyLock::new(Mutex::default);
/// Wakes the loop when an intent arrives.
static WAKER: OnceLock<AndroidAppWaker> = OnceLock::new();

/// What an intent to view asks to open.
struct Viewed {
    uri: String,
    /// Whether the intent grants writing the document.
    writable: bool,
}

/// The intents to view the activity receives.
pub(crate) struct OpenRequests {
    app: AndroidApp,
}

impl OpenRequests {
    /// Hears the intents that reach the activity from now on, and records the one it started with.
    pub(crate) fn new(app: AndroidApp) -> OpenRequests {
        let _ = WAKER.set(app.create_waker());
        if let Err(error) = with_activity(&app, hear_new_intents) {
            clear_exception(&app);
            warn_to_log!("inset: intents that reach the running app are not heard: {error}");
        }
        if let Err(error) = with_activity(&app, record_launch_intent) {
            clear_exception(&app);
            warn_to_log!("inset: the intent the app started with could not be read: {error}");
        }
        OpenRequests { app }
    }

    /// What the intents that arrived since the last call ask to open; `None` when nothing.
    pub(crate) fn take(&self) -> Option<OpenRequest> {
        let arrived = std::mem::take(&mut *locked(&ARRIVED));
        let mut request = OpenRequest::default();
        for viewed in arrived {
            match viewed.uri.split_once(':').map(|(scheme, _)| scheme) {
                Some("content") => request.files.extend(self.open(&viewed)),
                Some("file") => match file_uri_path(&viewed.uri) {
                    Some(path) => request.files.push(PickedFile::Path(path)),
                    None => warn_to_log!("inset: {} names no file", viewed.uri),
                },
                _ => request.links.push(viewed.uri),
            }
        }
        (!request.is_empty()).then_some(request)
    }

    /// The document an intent asks to open: to write where it grants writing and the provider
    /// lets it, to read otherwise.
    fn open(&self, viewed: &Viewed) -> Option<PickedFile> {
        let written = viewed
            .writable
            .then(|| open_document(&self.app, &viewed.uri, true).ok())
            .flatten();
        match written.map_or_else(|| open_document(&self.app, &viewed.uri, false), Ok) {
            Ok(file) => Some(file),
            Err(error) => {
                warn_to_log!("inset: {} could not be opened: {error}", viewed.uri);
                None
            }
        }
    }
}

/// Registers `InsetActivity.onIntent`, which the activity calls from `onNewIntent`.
fn hear_new_intents(env: &mut Env<'_>, activity: &JObject<'_>) -> jni::errors::Result<()> {
    let class = env.get_object_class(activity)?;
    let method = [
        // SAFETY: `on_intent` takes the arguments `(Landroid/content/Intent;)V` names, as a static
        // method's native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onIntent"),
                jni_str!("(Landroid/content/Intent;)V"),
                on_intent as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: the method's function matches its signature, above.
    unsafe { env.register_native_methods(&class, &method) }
}

fn record_launch_intent(env: &mut Env<'_>, activity: &JObject<'_>) -> jni::errors::Result<()> {
    let intent = env
        .call_method(
            activity,
            jni_str!("getIntent"),
            jni_sig!("()Landroid/content/Intent;"),
            &[],
        )?
        .l()?;
    record(env, &intent)
}

/// What `InsetActivity.onNewIntent` hands on, on the activity's thread.
extern "system" fn on_intent<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    intent: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        record(env, &intent)?;
        if let Some(waker) = WAKER.get() {
            waker.wake();
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// Records what an intent to view asks to open; other intents ask nothing.
fn record(env: &mut Env<'_>, intent: &JObject<'_>) -> jni::errors::Result<()> {
    if intent.is_null() {
        return Ok(());
    }
    let action = env
        .call_method(
            intent,
            jni_str!("getAction"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    if string(env, action)?.as_deref() != Some(VIEW) {
        return Ok(());
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
        return Ok(());
    };
    let flags = env
        .call_method(intent, jni_str!("getFlags"), jni_sig!("()I"), &[])?
        .i()?;
    locked(&ARRIVED).push(Viewed {
        uri,
        writable: flags & GRANT_WRITE != 0,
    });
    Ok(())
}

fn clear_exception(app: &AndroidApp) {
    let _ = with_activity(app, |env, _| {
        env.exception_clear();
        Ok(())
    });
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
