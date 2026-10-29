//! Pinned shortcuts: an icon of the app's own on the home screen that opens one view, through the
//! framework's `ShortcutManager` (API 26). Asking the launcher to pin one has the launcher ask the
//! user (`requestPinShortcut`); the launcher answers through Inset's shortcut receiver
//! (`rs.inset.InsetShortcutReceiver`, which inset-cli builds into the app and declares) once the
//! user pinned it, and says nothing when the user declined. Android lets no app remove a pinned
//! shortcut: the app changes its label or icon, or disables it, greyed out with a message, and
//! the user removes it.
//!
//! A shortcut starts Inset's document activity with the shortcut's initial data (`task`), so it
//! finds the task that has that data, or starts one whose view the platform gives the app,
//! carrying the data.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use inset_embedder::IconImage;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};

use crate::activity::{clear_exception, device_api_level, package_class, with_application};
use crate::answer::{Answer, answer};
use crate::bitmap::fitted_bitmap;
use crate::log::warn_to_log;
use crate::task::document_intent;

/// Inset's shortcut receiver, which hears the launcher's answer.
const RECEIVER: &str = "rs.inset.InsetShortcutReceiver";
/// The extra of the launcher's answer numbering the request: the receiver's own.
const EXTRA_REQUEST: &str = "rs.inset.shortcut.REQUEST";
/// The first release whose launcher pins an app's shortcuts on request.
const PINS_ON_REQUEST: i32 = 26;
/// `PendingIntent.FLAG_IMMUTABLE`.
const IMMUTABLE: jint = 0x0400_0000;
/// `PendingIntent.FLAG_UPDATE_CURRENT`.
const UPDATE_CURRENT: jint = 0x0800_0000;

/// The launcher's answer to one request, on its way to the app.
type PinAnswer = Answer<Result<(), PinError>>;

/// The requests waiting for the launcher's answer, by number.
static PENDING: LazyLock<Mutex<HashMap<jint, PinAnswer>>> = LazyLock::new(Mutex::default);
/// The requests' numbers, counted from one.
static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1);

/// A shortcut on the home screen that opens one view.
#[derive(Clone, Debug, PartialEq)]
pub struct PinnedShortcut {
    /// The app's own name for the shortcut, by which it is updated and disabled.
    pub id: String,
    pub label: String,
    pub icon: IconImage,
    /// What the view it opens carries, as `View::initial_data`.
    pub initial_data: String,
}

/// Why a shortcut was not pinned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinError {
    /// The launcher pins no shortcuts on request.
    Unsupported,
    Failed(String),
}

impl fmt::Display for PinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PinError::Unsupported => f.write_str("the launcher pins no shortcuts on request"),
            PinError::Failed(reason) => write!(f, "the shortcut was not pinned: {reason}"),
        }
    }
}

impl std::error::Error for PinError {}

/// The launcher's answer to a request to pin: `Ok` once the user pinned the shortcut. Android
/// tells no app that the user declined, so then it never resolves.
pub type PinFuture = Pin<Box<dyn Future<Output = Result<(), PinError>>>>;

/// The app's pinned shortcuts, reached as `AndroidPlatform::pinned_shortcuts`.
pub struct PinnedShortcuts {
    /// Whether the receiver hands the launcher's answers on, as in a package inset-cli built.
    answers_heard: bool,
}

impl PinnedShortcuts {
    /// The shortcuts, with the receiver's answer method registered where the package has it.
    pub(crate) fn new() -> PinnedShortcuts {
        let registered = with_application(register_answers);
        if registered.is_err() {
            clear_exception();
        }
        PinnedShortcuts {
            answers_heard: registered.is_ok(),
        }
    }

    /// Asks the launcher to pin `shortcut`; the launcher asks the user.
    pub fn request_pin(&self, shortcut: &PinnedShortcut) -> PinFuture {
        if device_api_level() < PINS_ON_REQUEST {
            return Box::pin(std::future::ready(Err(PinError::Unsupported)));
        }
        if !self.answers_heard {
            return Box::pin(std::future::ready(Err(PinError::Failed(
                "the package has no Inset shortcut receiver to hear the answer; inset-cli declares one"
                    .to_owned(),
            ))));
        }
        let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = answer(Err(PinError::Failed("no answer came".to_owned())));
        locked(&PENDING).insert(request, answer);
        let asked =
            with_application(|env, application| ask_to_pin(env, application, shortcut, request))
                .unwrap_or_else(|error| {
                    clear_exception();
                    Err(PinError::Failed(error.to_string()))
                });
        if let Err(error) = asked
            && let Some(answer) = locked(&PENDING).remove(&request)
        {
            answer.give(Err(error));
        }
        Box::pin(answered)
    }

    /// Changes a pinned shortcut's label, icon or what its view carries.
    pub fn update(&self, shortcut: &PinnedShortcut) {
        if device_api_level() < PINS_ON_REQUEST {
            return;
        }
        let updated = with_application(|env, application| {
            let manager = shortcut_manager(env, application)?;
            let info = shortcut_info(env, application, &manager, shortcut)?;
            let shortcuts = one_element_list(env, &info)?;
            env.call_method(
                &manager,
                jni_str!("updateShortcuts"),
                jni_sig!("(Ljava/util/List;)Z"),
                &[JValue::Object(&shortcuts)],
            )?
            .z()
        });
        match updated {
            Ok(true) => {}
            Ok(false) => warn_to_log!("inset: the shortcut {} was not updated now", shortcut.id),
            Err(error) => {
                clear_exception();
                warn_to_log!(
                    "inset: the shortcut {} was not updated: {error}",
                    shortcut.id
                );
            }
        }
    }

    /// Greys out the shortcut `id`, which says `message` when tapped: Android lets no app remove a
    /// pinned shortcut.
    pub fn disable(&self, id: &str, message: &str) {
        if device_api_level() < PINS_ON_REQUEST {
            return;
        }
        let disabled = with_application(|env, application| {
            let manager = shortcut_manager(env, application)?;
            let id = env.new_string(id)?;
            let ids = one_element_list(env, &id)?;
            let message = env.new_string(message)?;
            env.call_method(
                &manager,
                jni_str!("disableShortcuts"),
                jni_sig!("(Ljava/util/List;Ljava/lang/CharSequence;)V"),
                &[JValue::Object(&ids), JValue::Object(&message)],
            )
            .map(drop)
        });
        if let Err(error) = disabled {
            clear_exception();
            warn_to_log!("inset: the shortcut {id} was not disabled: {error}");
        }
    }
}

/// Asks the launcher to pin `shortcut`, its answer to come to the receiver numbered `request`;
/// an error where the launcher pins nothing or was not asked.
fn ask_to_pin(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    shortcut: &PinnedShortcut,
    request: jint,
) -> jni::errors::Result<Result<(), PinError>> {
    let manager = shortcut_manager(env, context)?;
    let supported = env
        .call_method(
            &manager,
            jni_str!("isRequestPinShortcutSupported"),
            jni_sig!("()Z"),
            &[],
        )?
        .z()?;
    if !supported {
        return Ok(Err(PinError::Unsupported));
    }
    let info = shortcut_info(env, context, &manager, shortcut)?;
    let answer_to = answer_sender(env, context, request)?;
    let asked = env
        .call_method(
            &manager,
            jni_str!("requestPinShortcut"),
            jni_sig!("(Landroid/content/pm/ShortcutInfo;Landroid/content/IntentSender;)Z"),
            &[JValue::Object(&info), JValue::Object(&answer_to)],
        )?
        .z()?;
    Ok(if asked {
        Ok(())
    } else {
        Err(PinError::Failed("the launcher was not asked".to_owned()))
    })
}

/// `shortcut` as Android's `ShortcutInfo`: its icon fitted to the most a launcher shows, and an
/// intent that starts Inset's document activity with its initial data. A shortcut's intent needs
/// an action, which here is `ACTION_MAIN`: it opens a view as the launcher's icon opens the app.
fn shortcut_info<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    manager: &JObject<'_>,
    shortcut: &PinnedShortcut,
) -> jni::errors::Result<JObject<'local>> {
    let id = env.new_string(&shortcut.id)?;
    let builder = env.new_object(
        jni_str!("android/content/pm/ShortcutInfo$Builder"),
        jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
        &[JValue::Object(context), JValue::Object(&id)],
    )?;
    let label = env.new_string(&shortcut.label)?;
    for setter in [jni_str!("setShortLabel"), jni_str!("setLongLabel")] {
        env.call_method(
            &builder,
            setter,
            jni_sig!("(Ljava/lang/CharSequence;)Landroid/content/pm/ShortcutInfo$Builder;"),
            &[JValue::Object(&label)],
        )?;
    }
    let largest = env
        .call_method(manager, jni_str!("getIconMaxWidth"), jni_sig!("()I"), &[])?
        .i()?;
    let bitmap = fitted_bitmap(env, &shortcut.icon, largest)?;
    let icon = env
        .call_static_method(
            jni_str!("android/graphics/drawable/Icon"),
            jni_str!("createWithBitmap"),
            jni_sig!("(Landroid/graphics/Bitmap;)Landroid/graphics/drawable/Icon;"),
            &[JValue::Object(&bitmap)],
        )?
        .l()?;
    env.call_method(
        &builder,
        jni_str!("setIcon"),
        jni_sig!("(Landroid/graphics/drawable/Icon;)Landroid/content/pm/ShortcutInfo$Builder;"),
        &[JValue::Object(&icon)],
    )?;
    let intent = document_intent(env, context, Some(&shortcut.initial_data))?;
    let main = env.new_string("android.intent.action.MAIN")?;
    env.call_method(
        &intent,
        jni_str!("setAction"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&main)],
    )?;
    env.call_method(
        &builder,
        jni_str!("setIntent"),
        jni_sig!("(Landroid/content/Intent;)Landroid/content/pm/ShortcutInfo$Builder;"),
        &[JValue::Object(&intent)],
    )?;
    env.call_method(
        &builder,
        jni_str!("build"),
        jni_sig!("()Landroid/content/pm/ShortcutInfo;"),
        &[],
    )?
    .l()
}

/// Where the launcher sends its answer to request `request`: a broadcast to Inset's shortcut
/// receiver, its number as an extra and as the pending intent's own, so that no two requests share
/// one.
fn answer_sender<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    request: jint,
) -> jni::errors::Result<JObject<'local>> {
    let intent = env.new_object(jni_str!("android/content/Intent"), jni_sig!("()V"), &[])?;
    let receiver = env.new_string(RECEIVER)?;
    env.call_method(
        &intent,
        jni_str!("setClassName"),
        jni_sig!("(Landroid/content/Context;Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(context), JValue::Object(&receiver)],
    )?;
    let key = env.new_string(EXTRA_REQUEST)?;
    env.call_method(
        &intent,
        jni_str!("putExtra"),
        jni_sig!("(Ljava/lang/String;I)Landroid/content/Intent;"),
        &[JValue::Object(&key), JValue::Int(request)],
    )?;
    let pending = env
        .call_static_method(
            jni_str!("android/app/PendingIntent"),
            jni_str!("getBroadcast"),
            jni_sig!(
                "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;"
            ),
            &[
                JValue::Object(context),
                JValue::Int(request),
                JValue::Object(&intent),
                JValue::Int(IMMUTABLE | UPDATE_CURRENT),
            ],
        )?
        .l()?;
    env.call_method(
        &pending,
        jni_str!("getIntentSender"),
        jni_sig!("()Landroid/content/IntentSender;"),
        &[],
    )?
    .l()
}

fn shortcut_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    let service = env.new_string("shortcut")?;
    env.call_method(
        context,
        jni_str!("getSystemService"),
        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
        &[JValue::Object(&service)],
    )?
    .l()
}

/// A Java list holding `element` alone.
fn one_element_list<'local>(
    env: &mut Env<'local>,
    element: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_static_method(
        jni_str!("java/util/Collections"),
        jni_str!("singletonList"),
        jni_sig!("(Ljava/lang/Object;)Ljava/util/List;"),
        &[JValue::Object(element)],
    )?
    .l()
}

/// Registers `InsetShortcutReceiver.onPinned`, which the receiver calls with the launcher's
/// answer.
fn register_answers(env: &mut Env<'_>, application: &JObject<'_>) -> jni::errors::Result<()> {
    let class = package_class(env, application, RECEIVER)?;
    let method = [
        // SAFETY: `on_pinned` takes the arguments `(I)V` names, as a static method's native
        // function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onPinned"),
                jni_str!("(I)V"),
                on_pinned as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: the method's function matches its signature, above.
    unsafe { env.register_native_methods(&class, &method) }
}

/// What `InsetShortcutReceiver.onReceive` hands on, on the Java main thread: the user pinned the
/// shortcut asked for with `request`.
extern "system" fn on_pinned<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    request: jint,
) {
    env.with_env(|_env| -> jni::errors::Result<()> {
        if let Some(answer) = locked(&PENDING).remove(&request) {
            answer.give(Ok(()));
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
