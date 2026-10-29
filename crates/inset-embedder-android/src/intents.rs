//! The intents that reach the activities: the one each started with, and each one Inset's activity
//! hands on from `onNewIntent` (`NativeActivity` keeps it to itself). Each is read into what it asks
//! of the app: files and links to open (`open_requests`), or a tap on one of the app's
//! notifications (`notifications`); other intents ask nothing. In a package without Inset's
//! activity, only the intent each activity started with comes.
//!
//! What an intent asks is set down and the app's thread woken; its loop hands it on once the client
//! is there, so the intent the first activity started with waits for the app to start.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use android_activity::AndroidApp;
use inset_embedder::NotificationActivation;
use jni::errors::LogErrorAndDefault;
use jni::objects::JObject;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use ndk::looper::ForeignLooper;

use crate::activity::{clear_exception, inset_activity_class, with_activity, with_application};
use crate::log::warn_to_log;
use crate::open_requests::{self, Viewed};

/// What the intents that arrived ask, not yet handed on.
static ARRIVED: LazyLock<Mutex<Arrived>> = LazyLock::new(Mutex::default);
/// The app's looper, woken when an intent arrives.
static LOOPER: OnceLock<ForeignLooper> = OnceLock::new();

/// What the intents that arrived since the loop last looked ask of the app.
#[derive(Default)]
pub(crate) struct Arrived {
    /// The URIs intents to view asked to open.
    pub(crate) viewed: Vec<Viewed>,
    /// The taps on the app's notifications, in the order they came.
    pub(crate) activations: Vec<NotificationActivation>,
}

/// Hears the intents that reach Inset's activities from now on, and wakes `looper`, the app's,
/// when one arrives.
pub(crate) fn hear(looper: ForeignLooper) {
    let _ = LOOPER.set(looper);
    if let Err(error) = with_application(hear_new_intents) {
        clear_exception();
        warn_to_log!("inset: intents that reach the running app are not heard: {error}");
    }
}

/// Reads the intent the activity `app` started with.
pub(crate) fn read_launch_intent(app: &AndroidApp) {
    if let Err(error) = with_activity(app, record_launch_intent) {
        clear_exception();
        warn_to_log!("inset: the intent the activity started with could not be read: {error}");
    }
}

/// What the intents that arrived since the last call ask.
pub(crate) fn take() -> Arrived {
    std::mem::take(&mut *locked(&ARRIVED))
}

/// Registers `InsetActivity.onIntent`, which the activity calls from `onNewIntent`.
fn hear_new_intents(env: &mut Env<'_>, application: &JObject<'_>) -> jni::errors::Result<()> {
    let class = inset_activity_class(env, application)?;
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
    record(env, activity, &intent)
}

/// What `InsetActivity.onNewIntent` hands on, on the Java main thread.
extern "system" fn on_intent<'local>(
    mut env: EnvUnowned<'local>,
    activity: JObject<'local>,
    intent: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        record(env, &activity, &intent)?;
        if let Some(looper) = LOOPER.get() {
            looper.wake();
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
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
