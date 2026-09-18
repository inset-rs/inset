//! Asking the system to run the display at the rate the app can draw.
//!
//! A panel that can go faster than 60 does not do so for an app that never says it wants
//! it: the system holds an app with no preference at the rate it judges enough, and the
//! app's own drawing is not taken as the ask. The ask is
//! `WindowManager.LayoutParams.preferredRefreshRate`, set on the activity's window.
//!
//! A Flutter application sets that same field from its own activity, because the engine
//! does not set it for them; an engine that never asks is why a Flutter app runs at 60 on
//! a 120 panel.

// The VM and the activity reach this crate as raw pointers, which JNI takes back as its
// own handles; that is the `unsafe` here.
#![allow(unsafe_code)]

use android_activity::AndroidApp;
use jni::objects::{JObject, JObjectArray, JValue};
use jni::{jni_sig, jni_str};

/// Asks the activity's window to run as fast as its display can, which is the fastest of
/// the modes the display reports. Window attributes belong to the Java main thread, so the
/// work is posted there and this returns at once.
pub(crate) fn follow_the_display(app: &AndroidApp) {
    let app = app.clone();
    let posted = app.clone();
    posted.run_on_java_main_thread(Box::new(move || {
        if let Err(error) = prefer_the_fastest_mode(&app) {
            crate::log::warn_to_log!("the window would not take a refresh rate: {error:?}");
        }
    }));
}

fn prefer_the_fastest_mode(app: &AndroidApp) -> jni::errors::Result<()> {
    // SAFETY: this process's own VM, as android-activity hands it out.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: an unowned global reference to the activity, borrowed for this frame.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let window = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;
        let Some(rate) = fastest_mode(env, &window)? else {
            return Ok(());
        };
        let attributes = env
            .call_method(
                &window,
                jni_str!("getAttributes"),
                jni_sig!("()Landroid/view/WindowManager$LayoutParams;"),
                &[],
            )?
            .l()?;
        env.set_field(
            &attributes,
            jni_str!("preferredRefreshRate"),
            jni_sig!("F"),
            JValue::Float(rate),
        )?;
        // Setting the field alone changes nothing: the window takes its attributes again.
        env.call_method(
            &window,
            jni_str!("setAttributes"),
            jni_sig!("(Landroid/view/WindowManager$LayoutParams;)V"),
            &[JValue::Object(&attributes)],
        )?;
        Ok(())
    })
}

/// The fastest of the modes this window's display reports, or `None` where it reports
/// none. `getDefaultDisplay` is deprecated in favour of the activity's own `getDisplay`,
/// which arrived in API 30; this library runs on 24, so it asks the window manager.
fn fastest_mode(env: &mut jni::Env<'_>, window: &JObject<'_>) -> jni::errors::Result<Option<f32>> {
    let manager = env
        .call_method(
            window,
            jni_str!("getWindowManager"),
            jni_sig!("()Landroid/view/WindowManager;"),
            &[],
        )?
        .l()?;
    let display = env
        .call_method(
            &manager,
            jni_str!("getDefaultDisplay"),
            jni_sig!("()Landroid/view/Display;"),
            &[],
        )?
        .l()?;
    let modes = env
        .call_method(
            &display,
            jni_str!("getSupportedModes"),
            jni_sig!("()[Landroid/view/Display$Mode;"),
            &[],
        )?
        .l()?;
    let modes: JObjectArray = env.cast_local::<JObjectArray>(modes)?;
    let count = modes.len(env)?;
    let mut fastest: Option<f32> = None;
    for index in 0..count {
        let mode = modes.get_element(env, index)?;
        let rate = env
            .call_method(&mode, jni_str!("getRefreshRate"), jni_sig!("()F"), &[])?
            .f()?;
        if fastest.is_none_or(|known| rate > known) {
            fastest = Some(rate);
        }
    }
    Ok(fastest)
}
