//! Notifications: the platform's `NotificationManager`, through Inset's notification receiver
//! (`rs.inset.InsetNotificationReceiver`, which inset-cli builds into the app and declares). The
//! receiver builds a notification from the `Bundle` filled here and shows it under its id as the
//! tag, so a later one under the same id replaces it, on a channel of the app's that sounds or one
//! that does not. It holds a notification for later in an `AlarmManager` alarm's intent and shows it
//! when the alarm goes off, whether or not the app runs. A tap, a button or a reply starts Inset's
//! activity with an intent of the receiver's, which `intents` hands here; one of the notification's
//! own actions answers it, so the notification is taken away then, which a reply needs: before
//! Android 15 the system shows a reply as sending until its notification changes, and from Android
//! 15 it keeps a replied notification, showing the reply, until the user dismisses it.
//!
//! A post or a schedule never asks the user: it answers `Denied` unless the permission is granted,
//! and only the app's `request` asks (`permission`). Under
//! an activity that is not Inset's, or in a package without the receiver, the app has no
//! notifications: nothing would hear a tap.
//!
//! The bundle's keys, the activation's action and extras and the reply's key are the receiver's
//! own: a change to one here is a change to the receiver.

mod permission;

use std::time::{Duration, SystemTime};

use android_activity::AndroidApp;
use inset_embedder::{
    IconImage, Notification, NotificationActivation, NotificationError, NotificationPermission,
    NotificationReplies, NotificationScheduling, Notifications, PostFuture,
};
use jni::objects::{JClass, JObject, JObjectArray, JValue};
use jni::refs::LoaderContext;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use self::permission::Permission;
use crate::activity::with_activity;
use crate::file_dialogs::string;
use crate::log::warn_to_log;

/// The receiver's binary name, as its class loader takes it.
const RECEIVER: &str = "rs.inset.InsetNotificationReceiver";
/// The action of the intent a tap, a button or a reply starts the activity with.
const ACTIVATED: &str = "rs.inset.action.NOTIFICATION_ACTIVATED";
const EXTRA_ID: &str = "rs.inset.notification.ID";
const EXTRA_ACTION: &str = "rs.inset.notification.ACTION";
/// The key of a reply among the reply field's results.
const REPLY: &str = "rs.inset.notification.REPLY";

/// The app's notifications, where the activity and the package are Inset's.
pub(crate) struct AndroidNotifications {
    app: AndroidApp,
    permission: Permission,
}

impl AndroidNotifications {
    pub(crate) fn new(app: AndroidApp) -> Option<AndroidNotifications> {
        let permission = Permission::new(app.clone())?;
        let has_receiver = with_activity(&app, |env, activity| receiver(env, activity).map(drop));
        if has_receiver.is_err() {
            clear_exception(&app);
            return None;
        }
        Some(AndroidNotifications { app, permission })
    }

    /// Hands `notification` to the receiver to show now, or at `at`, in milliseconds since the
    /// epoch, where the user lets the app notify; answers `Denied` without asking where not.
    fn hand_over(&self, notification: &Notification, at: Option<i64>) -> PostFuture {
        let handed = self.permission.allowed().and_then(|()| {
            call_receiver(&self.app, |env, activity, receiver| {
                let bundle = bundle(env, notification)?;
                match at {
                    None => env.call_static_method(
                        receiver,
                        jni_str!("show"),
                        jni_sig!("(Landroid/content/Context;Landroid/os/Bundle;)V"),
                        &[JValue::Object(activity), JValue::Object(&bundle)],
                    ),
                    Some(at) => env.call_static_method(
                        receiver,
                        jni_str!("schedule"),
                        jni_sig!("(Landroid/content/Context;Landroid/os/Bundle;J)V"),
                        &[
                            JValue::Object(activity),
                            JValue::Object(&bundle),
                            JValue::Long(at),
                        ],
                    ),
                }
                .map(drop)
            })
        });
        Box::pin(std::future::ready(handed))
    }
}

impl Notifications for AndroidNotifications {
    fn post(&self, notification: &Notification) -> PostFuture {
        self.hand_over(notification, None)
    }

    fn remove(&self, id: &str) {
        let removed = call_receiver(&self.app, |env, activity, receiver| {
            let id = env.new_string(id)?;
            env.call_static_method(
                receiver,
                jni_str!("remove"),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
                &[JValue::Object(activity), JValue::Object(&id)],
            )
            .map(drop)
        });
        if let Err(error) = removed {
            warn_to_log!("inset: the notification {id} was not removed: {error}");
        }
    }

    /// The user's permission to notify (`permission`).
    fn permission(&self) -> Option<&dyn NotificationPermission> {
        Some(&self.permission)
    }

    /// A reply field, the platform's `RemoteInput`.
    fn replies(&self) -> Option<&dyn NotificationReplies> {
        Some(self)
    }

    /// An alarm's, held by the system until the device restarts.
    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        Some(self)
    }
}

impl NotificationReplies for AndroidNotifications {}

impl NotificationScheduling for AndroidNotifications {
    fn schedule(&self, notification: &Notification, at: SystemTime) -> PostFuture {
        match at.duration_since(SystemTime::now()) {
            Ok(wait) if !wait.is_zero() => self.hand_over(notification, Some(epoch_millis(at))),
            _ => self.post(notification),
        }
    }
}

/// What a tap on one of the app's notifications, one of its actions or a reply tells the app, for an
/// intent of the receiver's; `None` for any other intent. A tap on one of its actions takes the
/// notification away, as the action answers it. Activity's thread.
pub(crate) fn activation(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
    intent: &JObject<'_>,
) -> jni::errors::Result<Option<NotificationActivation>> {
    let action = env
        .call_method(
            intent,
            jni_str!("getAction"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    if string(env, action)?.as_deref() != Some(ACTIVATED) {
        return Ok(None);
    }
    let Some(id) = string_extra(env, intent, EXTRA_ID)? else {
        return Ok(None);
    };
    let action = string_extra(env, intent, EXTRA_ACTION)?;
    let reply = reply(env, intent)?;
    if action.is_some() {
        dismiss(env, activity, &id)?;
    }
    Ok(Some(NotificationActivation { id, action, reply }))
}

/// The notification as the receiver reads it: the keys are the receiver's.
fn bundle<'local>(
    env: &mut Env<'local>,
    notification: &Notification,
) -> jni::errors::Result<JObject<'local>> {
    let bundle = env.new_object(jni_str!("android/os/Bundle"), jni_sig!("()V"), &[])?;
    put_string(env, &bundle, "id", Some(&notification.id))?;
    put_string(env, &bundle, "title", Some(&notification.title))?;
    put_string(env, &bundle, "body", notification.body.as_deref())?;
    let silent = env.new_string("silent")?;
    env.call_method(
        &bundle,
        jni_str!("putBoolean"),
        jni_sig!("(Ljava/lang/String;Z)V"),
        &[JValue::Object(&silent), JValue::Bool(notification.silent)],
    )?;
    if let Some(image) = &notification.image {
        put_image(env, &bundle, image)?;
    }
    let actions = &notification.actions;
    let ids: Vec<Option<&str>> = actions.iter().map(|action| Some(&*action.id)).collect();
    let labels: Vec<Option<&str>> = actions.iter().map(|action| Some(&*action.label)).collect();
    let placeholders: Vec<Option<&str>> = actions
        .iter()
        .map(|action| action.reply_placeholder())
        .collect();
    put_strings(env, &bundle, "actionIds", &ids)?;
    put_strings(env, &bundle, "actionLabels", &labels)?;
    put_strings(env, &bundle, "placeholders", &placeholders)?;
    Ok(bundle)
}

/// The image as its pixels, non-premultiplied ARGB colours row by row, and its size.
fn put_image(
    env: &mut Env<'_>,
    bundle: &JObject<'_>,
    image: &IconImage,
) -> jni::errors::Result<()> {
    let colors = argb_colors(&image.rgba);
    let pixels = jni::objects::JIntArray::new(env, colors.len())?;
    pixels.set_region(env, 0, &colors)?;
    let key = env.new_string("pixels")?;
    env.call_method(
        bundle,
        jni_str!("putIntArray"),
        jni_sig!("(Ljava/lang/String;[I)V"),
        &[JValue::Object(&key), JValue::Object(&pixels)],
    )?;
    put_int(env, bundle, "width", image.width as jint)?;
    put_int(env, bundle, "height", image.height as jint)
}

/// RGBA bytes as the platform's colours, alpha in the top byte.
fn argb_colors(rgba: &[u8]) -> Vec<jint> {
    let (pixels, _) = rgba.as_chunks::<4>();
    pixels
        .iter()
        .map(|&[red, green, blue, alpha]| i32::from_be_bytes([alpha, red, green, blue]))
        .collect()
}

fn put_string(
    env: &mut Env<'_>,
    bundle: &JObject<'_>,
    key: &str,
    value: Option<&str>,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    let value = match value {
        Some(value) => JObject::from(env.new_string(value)?),
        None => JObject::null(),
    };
    env.call_method(
        bundle,
        jni_str!("putString"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
        &[JValue::Object(&key), JValue::Object(&value)],
    )
    .map(drop)
}

fn put_int(
    env: &mut Env<'_>,
    bundle: &JObject<'_>,
    key: &str,
    value: jint,
) -> jni::errors::Result<()> {
    let key = env.new_string(key)?;
    env.call_method(
        bundle,
        jni_str!("putInt"),
        jni_sig!("(Ljava/lang/String;I)V"),
        &[JValue::Object(&key), JValue::Int(value)],
    )
    .map(drop)
}

/// `values` as a string array, a `None` as a null entry.
fn put_strings(
    env: &mut Env<'_>,
    bundle: &JObject<'_>,
    key: &str,
    values: &[Option<&str>],
) -> jni::errors::Result<()> {
    let string_class = env.find_class(jni_str!("java/lang/String"))?;
    let array: JObjectArray<'_> =
        env.new_object_array(values.len() as i32, &string_class, JObject::null())?;
    for (index, value) in values.iter().enumerate() {
        if let Some(value) = value {
            let value = env.new_string(value)?;
            array.set_element(env, index, &value)?;
        }
    }
    let key = env.new_string(key)?;
    env.call_method(
        bundle,
        jni_str!("putStringArray"),
        jni_sig!("(Ljava/lang/String;[Ljava/lang/String;)V"),
        &[JValue::Object(&key), JValue::Object(&array)],
    )
    .map(drop)
}

fn string_extra(
    env: &mut Env<'_>,
    intent: &JObject<'_>,
    key: &str,
) -> jni::errors::Result<Option<String>> {
    let key = env.new_string(key)?;
    let value = env
        .call_method(
            intent,
            jni_str!("getStringExtra"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
            &[JValue::Object(&key)],
        )?
        .l()?;
    string(env, value)
}

/// What the user typed into the notification's reply field, which the system put in `intent`.
fn reply(env: &mut Env<'_>, intent: &JObject<'_>) -> jni::errors::Result<Option<String>> {
    let results = env
        .call_static_method(
            jni_str!("android/app/RemoteInput"),
            jni_str!("getResultsFromIntent"),
            jni_sig!("(Landroid/content/Intent;)Landroid/os/Bundle;"),
            &[JValue::Object(intent)],
        )?
        .l()?;
    if results.is_null() {
        return Ok(None);
    }
    let key = env.new_string(REPLY)?;
    let text = env
        .call_method(
            &results,
            jni_str!("getCharSequence"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/CharSequence;"),
            &[JValue::Object(&key)],
        )?
        .l()?;
    if text.is_null() {
        return Ok(None);
    }
    let text = env
        .call_method(
            &text,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    string(env, text)
}

/// Takes away the notification shown under `id`.
fn dismiss(env: &mut Env<'_>, activity: &JObject<'_>, id: &str) -> jni::errors::Result<()> {
    let receiver = receiver(env, activity)?;
    let id = env.new_string(id)?;
    env.call_static_method(
        &receiver,
        jni_str!("dismiss"),
        jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
        &[JValue::Object(activity), JValue::Object(&id)],
    )
    .map(drop)
}

/// Inset's notification receiver, through the activity's class loader: a thread the app started
/// finds no class of the package's by its name alone.
fn receiver<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JClass<'local>> {
    let name = jni::strings::JNIString::new(RECEIVER);
    LoaderContext::FromObject(activity).load_class(env, name, false)
}

/// Calls the receiver with the activity as its context; a Java exception is the failure it
/// describes.
fn call_receiver(
    app: &AndroidApp,
    call: impl for<'local> FnOnce(
        &mut Env<'local>,
        &JObject<'_>,
        &JClass<'local>,
    ) -> jni::errors::Result<()>,
) -> Result<(), NotificationError> {
    with_activity(app, |env, activity| {
        let called = receiver(env, activity).and_then(|receiver| call(env, activity, &receiver));
        Ok(called.map_err(|error| failure(env, &error)))
    })
    .unwrap_or_else(|error| Err(NotificationError::Failed(error.to_string())))
}

/// What a failed call into Java tells: the exception it left, which is cleared, or the call's
/// error.
fn failure(env: &mut Env<'_>, error: &jni::errors::Error) -> NotificationError {
    let Some(exception) = env.exception_occurred() else {
        return NotificationError::Failed(error.to_string());
    };
    env.exception_clear();
    let described = env
        .call_method(
            &exception,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|text| text.l())
        .and_then(|text| string(env, text));
    env.exception_clear();
    NotificationError::Failed(match described {
        Ok(Some(text)) => text,
        _ => error.to_string(),
    })
}

/// `at` in milliseconds since the epoch, as `AlarmManager` takes a time on the clock.
fn epoch_millis(at: SystemTime) -> i64 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn clear_exception(app: &AndroidApp) {
    let _ = with_activity(app, |env, _| {
        env.exception_clear();
        Ok(())
    });
}
