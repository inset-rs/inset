//! The intents that reach an activity: the one it started with, and each one Waft's activity hands
//! on from `onNewIntent` (`NativeActivity` keeps it to itself). Each is read where it arrives, the
//! one an activity started with on the app's thread and a later one on Android's main thread, into
//! what it asks of the app: files and links to open (`open_requests`), or a tap on one of the app's
//! notifications (`notifications`); other intents ask nothing. Under an activity that is not
//! Waft's, only the intent the activity started with comes. A document activity's intents ask
//! nothing of the app: the one it started with is its view's (`view_requests.rs`), and no other is
//! sent it.
//!
//! What an intent asks is asked of the app, whichever activity it reached: it is set down in one
//! queue and the app's loop woken; the loop hands it on once the client is there, so the intent an
//! activity started with waits for the app to start.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{LazyLock, Mutex, MutexGuard, Once, PoisonError};

use jni::errors::LogErrorAndDefault;
use jni::objects::JObject;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::NotificationActivation;

use crate::activity::{Activity, WAFT_ACTIVITY, WAFT_DOCUMENT_ACTIVITY};
use crate::application::{Application, package_class};
use crate::log::warn_to_log;
use crate::loop_waker::wake_app_loop;
use crate::open_requests::{self, Viewed};

/// What the intents that arrived ask, not yet handed on.
static ARRIVED: LazyLock<Mutex<Arrived>> = LazyLock::new(Mutex::default);

/// What the intents that arrived since the loop last looked ask of the app.
#[derive(Default)]
pub(crate) struct Arrived {
    /// The URIs intents to view asked to open.
    pub(crate) viewed: Vec<Viewed>,
    /// The taps on the app's notifications, in the order they came.
    pub(crate) activations: Vec<NotificationActivation>,
}

/// Hears the intents that reach Waft's activities from now on.
pub(crate) fn hear(application: &Application) {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        if let Err(error) = hear_new_intents(application) {
            warn_to_log!("waft: intents that reach the running app are not heard: {error}");
        }
    });
}

/// Reads the intent `activity` started with.
pub(crate) fn read_launch_intent(activity: &Activity) {
    if let Err(error) = activity.with(record_launch_intent) {
        activity.clear_exception();
        warn_to_log!("waft: the intent the app started with could not be read: {error}");
    }
}

/// What the intents that arrived since the last call ask.
pub(crate) fn take() -> Arrived {
    std::mem::take(&mut *locked(&ARRIVED))
}

/// Registers `WaftActivity.onIntent`, which an activity calls from `onNewIntent`.
fn hear_new_intents(application: &Application) -> jni::errors::Result<()> {
    let method = [
        // SAFETY: `on_intent` takes the arguments `(Landroid/content/Intent;)V` names, as an
        // instance method's native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onIntent"),
                jni_str!("(Landroid/content/Intent;)V"),
                on_intent as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: the method's function matches its signature, above.
    unsafe { application.register_native_methods(WAFT_ACTIVITY, &method) }
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
    record(env, activity, &intent)
}

/// What `WaftActivity.onNewIntent` hands on, on Android's main thread.
extern "system" fn on_intent<'local>(
    mut env: EnvUnowned<'local>,
    activity: JObject<'local>,
    intent: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if is_document(env, &activity) {
            return Ok(());
        }
        record(env, &activity, &intent)?;
        wake_app_loop();
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// Whether `activity` is Waft's document activity; not in a package without it.
fn is_document(env: &mut Env<'_>, activity: &JObject<'_>) -> bool {
    let class = package_class(env, activity, WAFT_DOCUMENT_ACTIVITY);
    let is_document = class.and_then(|class| env.is_instance_of(activity, &class));
    is_document.unwrap_or_else(|_| {
        env.exception_clear();
        false
    })
}

/// Sets down what `intent` asks of the app.
fn record(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
    intent: &JObject<'_>,
) -> jni::errors::Result<()> {
    if intent.is_null() {
        return Ok(());
    }
    if let Some(viewed) = open_requests::viewed(env, intent)? {
        locked(&ARRIVED).viewed.push(viewed);
    } else if let Some(activation) = crate::notifications::activation(env, activity, intent)? {
        locked(&ARRIVED).activations.push(activation);
    }
    Ok(())
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
