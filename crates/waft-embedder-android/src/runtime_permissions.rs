//! Runtime permissions: what the app holds once the user grants it, such as posting notifications
//! from Android 13 or using the camera, asked for with the system's question over an activity
//! (`Activity.requestPermissions`), the answer coming back through Waft's activity
//! (`onRequestPermissionsResult`, which `NativeActivity` keeps to itself, handed on to
//! `WaftActivity.onPermissionsResult`). Whether the app holds one is the application's to answer;
//! the question, and whether the user refused it once (`shouldShowRequestPermissionRationale`),
//! are an activity's alone.
//!
//! Android shows one question at a time and answers a second asked meanwhile as interrupted, so a
//! question for a permission already being asked joins it, and hears its answer.
//!
//! The invariant: the answer method is registered once for the process, and every question the
//! host asks, whatever its permission, goes through here, matched to its answer by a request code
//! no other question in the process has, whichever activity hands the answer on.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JIntArray, JObject, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};

use crate::activity::{Activity, WAFT_ACTIVITY};
use crate::answer::{Answer, Answered, answer};
use crate::application::Application;

/// `PackageManager.PERMISSION_GRANTED`.
const GRANTED: jint = 0;

/// The questions waiting for the user's answer, by request code: the permission asked, and each
/// asker's answer.
static PENDING: LazyLock<Mutex<HashMap<jint, Question>>> = LazyLock::new(Mutex::default);
/// The request codes, counted from one.
static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1);

/// One question shown, and who waits for its answer.
struct Question {
    permission: String,
    askers: Vec<Answer<Asked>>,
}

/// What the user answered the system's question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    Granted,
    Refused,
    /// The question went before the user answered it, as when its activity went.
    Interrupted,
}

/// Registers `WaftActivity.onPermissionsResult`, which an activity calls from
/// `onRequestPermissionsResult`, once for the process, and answers whether it is registered: not
/// in a package without Waft's activity, which hands no answer on.
pub(crate) fn answers_are_heard(application: &Application) -> bool {
    static HEARD: OnceLock<bool> = OnceLock::new();
    *HEARD.get_or_init(|| {
        let method = [
            // SAFETY: `on_permissions_result` takes the arguments `(I[I)V` names, as a static
            // method's native function does.
            unsafe {
                NativeMethod::from_raw_parts(
                    jni_str!("onPermissionsResult"),
                    jni_str!("(I[I)V"),
                    on_permissions_result as *mut std::ffi::c_void,
                )
            },
        ];
        // SAFETY: the method's function matches its signature, above.
        unsafe { application.register_native_methods(WAFT_ACTIVITY, &method) }.is_ok()
    })
}

/// Shows the system's question for `permission` over `activity`, or joins the one shown for it,
/// and answers the user's answer; an error where the activity would not ask, with no question
/// left waiting.
pub(crate) fn ask(activity: &Activity, permission: &str) -> jni::errors::Result<Answered<Asked>> {
    let (answer, answered) = answer(Asked::Interrupted);
    let request = {
        let mut pending = locked(&PENDING);
        if let Some(shown) = pending
            .values_mut()
            .find(|question| question.permission == permission)
        {
            shown.askers.push(answer);
            return Ok(answered);
        }
        let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let question = Question {
            permission: permission.to_owned(),
            askers: vec![answer],
        };
        pending.insert(request, question);
        request
    };
    let asked = activity.with(|env, activity| {
        let permissions = env.new_object_array(1, jni_str!("java/lang/String"), JObject::null())?;
        let permission = env.new_string(permission)?;
        permissions.set_element(env, 0, &permission)?;
        env.call_method(
            activity,
            jni_str!("requestPermissions"),
            jni_sig!("([Ljava/lang/String;I)V"),
            &[JValue::Object(&permissions), JValue::Int(request)],
        )
        .map(drop)
    });
    if let Err(error) = asked {
        if let Some(unasked) = locked(&PENDING).remove(&request) {
            unasked.answer(Asked::Interrupted);
        }
        return Err(error);
    }
    Ok(answered)
}

/// Whether the app holds `permission`, as the system keeps it for the app.
pub(crate) fn is_granted(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    permission: &str,
) -> jni::errors::Result<bool> {
    let permission = env.new_string(permission)?;
    let granted = env
        .call_method(
            context,
            jni_str!("checkSelfPermission"),
            jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&permission)],
        )?
        .i()?;
    Ok(granted == GRANTED)
}

/// Whether the user refused `permission` once and may be asked again.
pub(crate) fn refused_once(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
    permission: &str,
) -> jni::errors::Result<bool> {
    let permission = env.new_string(permission)?;
    env.call_method(
        activity,
        jni_str!("shouldShowRequestPermissionRationale"),
        jni_sig!("(Ljava/lang/String;)Z"),
        &[JValue::Object(&permission)],
    )?
    .z()
}

/// What `WaftActivity.onRequestPermissionsResult` hands on, on Android's main thread: the answer
/// for the one permission asked, or none where the question was interrupted.
extern "system" fn on_permissions_result<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    request: jint,
    results: JIntArray<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let Some(question) = locked(&PENDING).remove(&request) else {
            return Ok(());
        };
        let mut first = [GRANTED];
        let asked = if results.is_null() || results.len(env)? == 0 {
            Asked::Interrupted
        } else {
            results.get_region(env, 0, &mut first)?;
            if first[0] == GRANTED {
                Asked::Granted
            } else {
                Asked::Refused
            }
        };
        question.answer(asked);
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

impl Question {
    /// Gives every asker `asked`.
    fn answer(self, asked: Asked) {
        for asker in self.askers {
            asker.give(asked);
        }
    }
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
