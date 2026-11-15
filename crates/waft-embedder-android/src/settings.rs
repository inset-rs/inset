//! The user's settings Flutter's Android embedder sends the framework on its settings channel
//! and the activity's configuration does not carry: whether a password field briefly shows the
//! character just typed, the system's "Show passwords" (`Settings.System.TEXT_SHOW_PASSWORD`),
//! which Flutter's `FlutterView.sendUserSettingsToFlutter` reads as `brieflyShowPassword`.
//!
//! Flutter's view reads it again whenever a `ContentObserver` on the setting fires; an observer is
//! Java code, so the host reads it again where it reads the configuration and as a window of the
//! app takes the focus, which it does when the user comes back from the system's settings.

use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::application::Application;
use crate::log::warn_to_log;

/// `Settings.System.TEXT_SHOW_PASSWORD`.
const TEXT_SHOW_PASSWORD: &str = "show_password";

/// Whether the system's "Show passwords" is on: Flutter's `brieflyShowPassword`, on where the
/// system has no value, as Flutter reads it, and where it cannot be read, as dart:ui's default.
pub(crate) fn read_briefly_show_password(application: &Application) -> bool {
    match application.with(text_show_password) {
        Ok(shown) => shown,
        Err(error) => {
            application.clear_exception();
            warn_to_log!("waft: the system's \"Show passwords\" is not read: {error}");
            true
        }
    }
}

/// `Settings.System.getInt(resolver, TEXT_SHOW_PASSWORD, 1) == 1`, as Flutter's view reads it.
fn text_show_password(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<bool> {
    let resolver = env
        .call_method(
            context,
            jni_str!("getContentResolver"),
            jni_sig!("()Landroid/content/ContentResolver;"),
            &[],
        )?
        .l()?;
    let name = env.new_string(TEXT_SHOW_PASSWORD)?;
    let value = env
        .call_static_method(
            jni_str!("android/provider/Settings$System"),
            jni_str!("getInt"),
            jni_sig!("(Landroid/content/ContentResolver;Ljava/lang/String;I)I"),
            &[
                JValue::Object(&resolver),
                JValue::Object(&name),
                JValue::Int(1),
            ],
        )?
        .i()?;
    Ok(value == 1)
}
