//! The user's permission to notify. From Android 13, for an app that targets it, posting is the
//! runtime permission `POST_NOTIFICATIONS`, which the activity asks for with the system's question
//! (`requestPermissions`), the answer coming back through Waft's activity
//! (`onRequestPermissionsResult`, which `NativeActivity` keeps to itself). Before that there is no
//! question: the user turns the app's notifications off in its settings, which
//! `areNotificationsEnabled` tells, and that is the whole state.
//!
//! The system says whether the permission is granted, and whether the user refused it once and
//! may be asked again (`shouldShowRequestPermissionRationale`), but not whether they were never
//! asked or refused it for good: both answer `NotAsked`. Asking then shows the question, or for one
//! refused for good answers at once that it is denied.
//!
//! Only the app's `request` asks. A post never does, as `notify` does not: it answers `Denied`
//! unless the permission is granted, where the system would show nothing.
//!
//! Whether the permission is granted is the application's to answer. The question, and whether the
//! user refused it once, are an activity's alone (`Activity.requestPermissions`,
//! `shouldShowRequestPermissionRationale`): the one the permission was made with, the app's one,
//! over which the question shows. An answer is matched to its question by a request code no other
//! question in the process has, whichever activity hands it on.

// JNI hands the native method raw arguments; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JIntArray, JObject, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::{NotificationError, NotificationPermission, PermissionFuture, PermissionState};

use crate::activity::{AttachedActivity, WAFT_ACTIVITY, device_api_level};
use crate::answer::{Answer, answer};
use crate::application::Application;
use crate::log::warn_to_log;

/// `Manifest.permission.POST_NOTIFICATIONS`.
const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";
/// Android 13, from which posting is a permission the user is asked for.
const ASKED_FROM: i32 = 33;
/// `PackageManager.PERMISSION_GRANTED`.
const GRANTED: jint = 0;

/// The questions waiting for the user's answer, by request code.
static PENDING: LazyLock<Mutex<HashMap<jint, Answer<PermissionState>>>> =
    LazyLock::new(Mutex::default);
/// The request codes, counted from one.
static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1);

/// The user's permission to notify, as the system keeps it for the app.
#[derive(Clone)]
pub(super) struct Permission {
    application: Application,
    /// The activity the question is asked over, which gets its answer: the one attached to the
    /// app.
    activity: AttachedActivity,
    /// Whether the system asks the user: Android 13 and later, for an app that targets it.
    asks: bool,
}

impl Permission {
    /// The permission, asked for over `activity`, once the answer method of Waft's activity is
    /// registered; `None` under an activity that is not Waft's, which hands no answer on.
    pub(super) fn new(application: Application, activity: AttachedActivity) -> Option<Permission> {
        let is_wafts = activity.get().is_some_and(|activity| activity.is_wafts());
        if !(answers_are_heard(&application) && is_wafts) {
            return None;
        }
        let asks = application.with(|env, context| {
            Ok(device_api_level() >= ASKED_FROM && target_api_level(env, context)? >= ASKED_FROM)
        });
        match asks {
            Ok(asks) => Some(Permission {
                application,
                activity,
                asks,
            }),
            Err(_) => {
                application.clear_exception();
                None
            }
        }
    }

    /// Whether the app may notify now, without asking.
    pub(super) fn allowed(&self) -> Result<(), NotificationError> {
        match self.current() {
            PermissionState::Granted => Ok(()),
            PermissionState::Denied | PermissionState::NotAsked => Err(NotificationError::Denied),
        }
    }

    /// The permission as the system tells it now.
    fn current(&self) -> PermissionState {
        let current = self.where_held().and_then(|held| match held {
            Some(held) => Ok(held),
            None => self.where_not_held(),
        });
        current.unwrap_or_else(|error| {
            warn_to_log!("waft: the permission to notify could not be read: {error}");
            PermissionState::Denied
        })
    }

    /// The state where the app holds the permission, or the system asks for none: whether the
    /// user has the app's notifications on. `None` where the app does not hold it. The
    /// application answers.
    fn where_held(&self) -> jni::errors::Result<Option<PermissionState>> {
        self.application.with(|env, context| {
            let held = !self.asks || has_permission(env, context)?;
            if !held {
                return Ok(None);
            }
            Ok(Some(granted_if(notifications_enabled(env, context)?)))
        })
    }

    /// The state where the app does not hold the permission: refused, where the user refused it
    /// once, and otherwise not asked for. Only an activity answers.
    fn where_not_held(&self) -> jni::errors::Result<PermissionState> {
        self.activity.with(|env, activity| {
            Ok(if refused_once(env, activity)? {
                PermissionState::Denied
            } else {
                PermissionState::NotAsked
            })
        })
    }

    /// Shows the system's question, and answers the user's answer.
    fn ask(&self) -> PermissionFuture {
        let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = answer(PermissionState::NotAsked);
        locked(&PENDING).insert(request, answer);
        let asked = self.activity.with(|env, activity| {
            let permissions =
                env.new_object_array(1, jni_str!("java/lang/String"), JObject::null())?;
            let permission = env.new_string(POST_NOTIFICATIONS)?;
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
            warn_to_log!("waft: the permission to notify could not be asked for: {error}");
            self.activity.clear_exception();
            if let Some(answer) = locked(&PENDING).remove(&request) {
                answer.give(self.current());
            }
        }
        Box::pin(answered)
    }
}

impl NotificationPermission for Permission {
    fn state(&self) -> PermissionFuture {
        Box::pin(std::future::ready(self.current()))
    }

    fn request(&self) -> PermissionFuture {
        match self.current() {
            PermissionState::NotAsked => self.ask(),
            answered => Box::pin(std::future::ready(answered)),
        }
    }
}

fn granted_if(enabled: bool) -> PermissionState {
    if enabled {
        PermissionState::Granted
    } else {
        PermissionState::Denied
    }
}

/// Registers `WaftActivity.onPermissionsResult`, which an activity calls from
/// `onRequestPermissionsResult`, once for the process, and answers whether it is registered: not
/// in a package without Waft's activity.
fn answers_are_heard(application: &Application) -> bool {
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

/// What `WaftActivity.onRequestPermissionsResult` hands on, on Android's main thread: the answer
/// for the one permission asked, or none where the question was interrupted.
extern "system" fn on_permissions_result<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    request: jint,
    results: JIntArray<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let Some(answer) = locked(&PENDING).remove(&request) else {
            return Ok(());
        };
        let mut first = [GRANTED];
        let answered = if results.is_null() || results.len(env)? == 0 {
            PermissionState::NotAsked
        } else {
            results.get_region(env, 0, &mut first)?;
            granted_if(first[0] == GRANTED)
        };
        answer.give(answered);
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

fn notifications_enabled(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<bool> {
    let service = env.new_string("notification")?;
    let manager = env
        .call_method(
            context,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service)],
        )?
        .l()?;
    env.call_method(
        &manager,
        jni_str!("areNotificationsEnabled"),
        jni_sig!("()Z"),
        &[],
    )?
    .z()
}

fn has_permission(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<bool> {
    let permission = env.new_string(POST_NOTIFICATIONS)?;
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

/// Whether the user refused the permission once and may be asked again.
fn refused_once(env: &mut Env<'_>, activity: &JObject<'_>) -> jni::errors::Result<bool> {
    let permission = env.new_string(POST_NOTIFICATIONS)?;
    env.call_method(
        activity,
        jni_str!("shouldShowRequestPermissionRationale"),
        jni_sig!("(Ljava/lang/String;)Z"),
        &[JValue::Object(&permission)],
    )?
    .z()
}

/// The release the app's package targets, which decides whether the system asks for it.
fn target_api_level(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<i32> {
    let info = env
        .call_method(
            context,
            jni_str!("getApplicationInfo"),
            jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )?
        .l()?;
    env.get_field(&info, jni_str!("targetSdkVersion"), jni_sig!("I"))?
        .i()
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
