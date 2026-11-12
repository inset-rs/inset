//! The window's insets: what the system covers of the activity's window, as Android applies them
//! to it. Waft's window insets listener (`rs.waft.WaftWindowInsets`, which waft-cli builds into the
//! app and `WaftActivity` has hear the window's decor view) reads the system's bars and the
//! display's cutout apart from the keyboard, and the system's gesture zones, as Flutter's
//! `FlutterView.onApplyWindowInsets` does, and hands them on each time they are applied.
//!
//! They are applied on Android's main thread, which never calls the framework: the latest are set
//! down and the loop woken, and the loop takes them ([`take`]) and has the view's metrics follow.
//! Only the latest matter, so a later one replaces one the loop has not taken yet.
//!
//! Under an activity that is not Waft's nothing tells the insets, and the host knows only the
//! rectangle `NativeActivity` laid its content out in ([`WindowInsets::around_content`]).

// JNI hands the native method raw arguments, and registering it takes its function as a raw
// pointer; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{Mutex, MutexGuard, PoisonError};

use android_activity::{AndroidApp, AndroidAppWaker};
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::ViewPadding;

use crate::activity::with_activity;
use crate::log::warn_to_log;

/// The insets Android applied since the loop last looked.
static APPLIED: Mutex<Option<WindowInsets>> = Mutex::new(None);
/// Wakes the loop when Android applies the window's insets: the loop of the activity that hears
/// them now, which is a new one for each activity the process starts.
static WAKER: Mutex<Option<AndroidAppWaker>> = Mutex::new(None);

/// What the system covers of the window, each side in pixels from the window's edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowInsets {
    /// The system's bars and the display's cutout: Flutter's `viewPadding`.
    pub(crate) view_padding: ViewPadding,
    /// The keyboard: Flutter's `viewInsets`.
    pub(crate) view_insets: ViewPadding,
    /// Where the system takes a swipe for its own navigation, `WindowInsets.Type.systemGestures()`:
    /// Flutter's `systemGestureInsets`.
    pub(crate) system_gesture_insets: ViewPadding,
}

impl WindowInsets {
    /// What `content`, the rectangle `NativeActivity` laid its content out in, leaves of a window
    /// of `size` on each side, taken for the system's bars: all that is known under an activity
    /// that is not Waft's, which has no keyboard to tell from them and no gesture zones. An empty
    /// rectangle, as before the activity's first layout, is no bars at all.
    pub(crate) fn around_content(size: [u32; 2], content: [i32; 4]) -> WindowInsets {
        let [width, height] = size.map(f64::from);
        let [left, top, right, bottom] = content.map(f64::from);
        let view_padding = if right <= left || bottom <= top {
            ViewPadding::ZERO
        } else {
            ViewPadding {
                left: left.max(0.0),
                top: top.max(0.0),
                right: (width - right).max(0.0),
                bottom: (height - bottom).max(0.0),
            }
        };
        WindowInsets {
            view_padding,
            view_insets: ViewPadding::ZERO,
            system_gesture_insets: ViewPadding::ZERO,
        }
    }
}

/// Hears the insets Android applies to the activity's window from now on, and asks for the ones
/// it already applied. Under an activity that is not Waft's there is nothing to hear.
pub(crate) fn hear(app: &AndroidApp) {
    *locked(&WAKER) = Some(app.create_waker());
    // An activity before this one, in the same process, may have left its window's.
    locked(&APPLIED).take();
    let heard = with_activity(app, |env, activity| {
        let listener = listener(env, activity)?;
        register_native_method(env, &listener)?;
        tell_again(env, &listener)
    });
    if let Err(error) = heard {
        clear_exception(app);
        warn_to_log!("waft: the window's insets are not heard: {error}");
    }
}

/// The insets Android applied since the last call, if it applied any.
pub(crate) fn take() -> Option<WindowInsets> {
    locked(&APPLIED).take()
}

/// Waft's window insets listener, from the activity.
fn listener<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("windowInsets"),
        jni_sig!("()Lrs/waft/WaftWindowInsets;"),
        &[],
    )?
    .l()
}

/// Registers the listener's `onWindowInsets`, which it calls on the main thread.
fn register_native_method(env: &mut Env<'_>, listener: &JObject<'_>) -> jni::errors::Result<()> {
    let class = env.get_object_class(listener)?;
    let method = [
        // SAFETY: `on_window_insets` takes the arguments `(IIIIIIIIIIII)V` names, as an instance
        // method's native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onWindowInsets"),
                jni_str!("(IIIIIIIIIIII)V"),
                on_window_insets as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: the method's function matches its signature, above.
    unsafe { env.register_native_methods(&class, &method) }
}

/// Has Android apply the window's insets once more: the ones it applied before the method was
/// registered reached nobody.
fn tell_again(env: &mut Env<'_>, listener: &JObject<'_>) -> jni::errors::Result<()> {
    env.call_method(listener, jni_str!("tellAgain"), jni_sig!("()V"), &[])
        .map(drop)
}

/// What the listener's `onWindowInsets` hands on, on the main thread: the view padding, the view
/// insets and then the system gesture insets, each side in pixels.
#[allow(clippy::too_many_arguments)]
extern "system" fn on_window_insets<'local>(
    _env: EnvUnowned<'local>,
    _listener: JObject<'local>,
    view_padding_left: jint,
    view_padding_top: jint,
    view_padding_right: jint,
    view_padding_bottom: jint,
    view_inset_left: jint,
    view_inset_top: jint,
    view_inset_right: jint,
    view_inset_bottom: jint,
    system_gesture_inset_left: jint,
    system_gesture_inset_top: jint,
    system_gesture_inset_right: jint,
    system_gesture_inset_bottom: jint,
) {
    let insets = WindowInsets {
        view_padding: ViewPadding {
            left: f64::from(view_padding_left),
            top: f64::from(view_padding_top),
            right: f64::from(view_padding_right),
            bottom: f64::from(view_padding_bottom),
        },
        view_insets: ViewPadding {
            left: f64::from(view_inset_left),
            top: f64::from(view_inset_top),
            right: f64::from(view_inset_right),
            bottom: f64::from(view_inset_bottom),
        },
        system_gesture_insets: ViewPadding {
            left: f64::from(system_gesture_inset_left),
            top: f64::from(system_gesture_inset_top),
            right: f64::from(system_gesture_inset_right),
            bottom: f64::from(system_gesture_inset_bottom),
        },
    };
    *locked(&APPLIED) = Some(insets);
    if let Some(waker) = locked(&WAKER).as_ref() {
        waker.wake();
    }
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
