//! Pinned shortcuts: an icon of the app's own on the home screen that opens one view, through the
//! framework's `ShortcutManager` (API 26). Asking the launcher to pin one hands it the request
//! (`requestPinShortcut`), which may take seconds, so the call runs on a thread of its own; the
//! launcher then asks the user, whose answer Android tells no app. Android lets no app remove a
//! pinned shortcut: the app changes its label or icon, or disables it, greyed out with a message,
//! and the user removes it.
//!
//! A shortcut starts Inset's document activity with the shortcut's initial data (`task`), so it
//! finds the task that has that data, or starts one whose view the platform gives the app,
//! carrying the data.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use inset_embedder::IconImage;
use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::activity::{clear_exception, device_api_level, with_application};
use crate::answer::answer;
use crate::bitmap::fitted_bitmap;
use crate::log::warn_to_log;
use crate::task::document_intent;

/// The first release whose launcher pins an app's shortcuts on request.
const PINS_ON_REQUEST: i32 = 26;

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

/// Whether the launcher has a request to pin: `Ok` once it has it, before the user answers, which
/// Android tells no app.
pub type PinFuture = Pin<Box<dyn Future<Output = Result<(), PinError>>>>;

/// The app's pinned shortcuts, reached as `AndroidPlatform::pinned_shortcuts`: the system's
/// `ShortcutManager`, which holds them.
pub struct PinnedShortcuts(());

impl PinnedShortcuts {
    pub(crate) fn new() -> PinnedShortcuts {
        PinnedShortcuts(())
    }

    /// Asks the launcher to pin `shortcut`; the launcher asks the user. The future answers once the
    /// launcher has the request, not when the user has answered it.
    pub fn request_pin(&self, shortcut: &PinnedShortcut) -> PinFuture {
        if device_api_level() < PINS_ON_REQUEST {
            return Box::pin(std::future::ready(Err(PinError::Unsupported)));
        }
        // Answered on the thread, or, dropped with it where no thread could be made, answered
        // that the request was not handed over.
        let (handed, handed_over) = answer(Err(PinError::Failed(
            "the request was not handed to the launcher".to_owned(),
        )));
        let shortcut = shortcut.clone();
        let _ = std::thread::Builder::new()
            .name("inset-shortcut-pin".into())
            .spawn(move || handed.give(hand_over(&shortcut)));
        Box::pin(handed_over)
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

/// Hands the launcher the request to pin `shortcut`: the launcher's word that it has it.
fn hand_over(shortcut: &PinnedShortcut) -> Result<(), PinError> {
    with_application(|env, application| ask_to_pin(env, application, shortcut)).unwrap_or_else(
        |error| {
            clear_exception();
            Err(PinError::Failed(error.to_string()))
        },
    )
}

/// Asks the launcher to pin `shortcut`; an error where the launcher pins nothing or would not
/// take the request.
fn ask_to_pin(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    shortcut: &PinnedShortcut,
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
    let no_answer_wanted = JObject::null();
    let taken = env
        .call_method(
            &manager,
            jni_str!("requestPinShortcut"),
            jni_sig!("(Landroid/content/pm/ShortcutInfo;Landroid/content/IntentSender;)Z"),
            &[JValue::Object(&info), JValue::Object(&no_answer_wanted)],
        )?
        .z()?;
    Ok(if taken {
        Ok(())
    } else {
        Err(PinError::Failed(
            "the launcher would not take the request".to_owned(),
        ))
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
