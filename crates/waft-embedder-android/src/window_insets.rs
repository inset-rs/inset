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
//! Each activity's window has a listener of its own, and the native method is the process's: so a
//! listener is given the id of the host's view its window is ([`hear`]) and hands it back with each
//! telling, as the Java side of Flutter's embedding carries its shell's id into each native call
//! (`FlutterJNI.nativeShellHolderId`). Flutter has no counterpart for the rest, since its engine
//! has one activity at a time: the latest insets are kept for each view, and a telling wakes the
//! app's one loop. A later editor must keep them by view: one slot for the app would hand one
//! window's insets, its keyboard's among them, to another's view.
//!
//! Under an activity that is not Waft's nothing tells the insets, and the host knows only the
//! rectangle `NativeActivity` laid its content out in ([`WindowInsets::around_content`]).

// JNI hands the native method raw arguments, and registering it takes its function as a raw
// pointer; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use jni::objects::{JObject, JValue};
use jni::sys::{jint, jlong};
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::{ViewId, ViewPadding};

use crate::activity::Activity;
use crate::application::Application;
use crate::log::warn_to_log;
use crate::loop_waker::wake_app_loop;

/// Waft's window insets listener, which declares the native method registered here.
const LISTENER: &str = "rs.waft.WaftWindowInsets";

/// The insets Android applied to each view's window since the loop last looked.
static APPLIED: Mutex<BTreeMap<ViewId, WindowInsets>> = Mutex::new(BTreeMap::new());

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

/// Hears the insets Android applies to `activity`'s window from now on as those of `view`, the
/// host's view that window is, and asks for the ones it already applied. Under an activity that
/// is not Waft's there is nothing to hear.
pub(crate) fn hear(application: &Application, activity: &Activity, view: ViewId) {
    // The window of an activity the view had before may have left its own.
    locked(&APPLIED).remove(&view);
    if !insets_are_told(application) {
        warn_to_log!("waft: the window's insets are not heard: the package has no {LISTENER}");
        return;
    }
    let heard = activity.with(|env, activity| {
        let listener = listener(env, activity)?;
        attach(env, &listener, view)
    });
    if let Err(error) = heard {
        activity.clear_exception();
        warn_to_log!("waft: the window's insets are not heard: {error}");
    }
}

/// The insets Android applied since the last call, by the view whose window it applied them to.
pub(crate) fn take() -> BTreeMap<ViewId, WindowInsets> {
    std::mem::take(&mut *locked(&APPLIED))
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

/// Registers the listener's `onWindowInsets`, which a listener calls on the main thread, once for
/// the process, and answers whether it is registered: not in a package without the listener.
fn insets_are_told(application: &Application) -> bool {
    static TOLD: OnceLock<bool> = OnceLock::new();
    *TOLD.get_or_init(|| {
        let method = [
            // SAFETY: `on_window_insets` takes the arguments `(JIIIIIIIIIIII)V` names, as an
            // instance method's native function does.
            unsafe {
                NativeMethod::from_raw_parts(
                    jni_str!("onWindowInsets"),
                    jni_str!("(JIIIIIIIIIIII)V"),
                    on_window_insets as *mut std::ffi::c_void,
                )
            },
        ];
        // SAFETY: the method's function matches its signature, above.
        unsafe { application.register_native_methods(LISTENER, &method) }.is_ok()
    })
}

/// Gives the listener the id of the host's view its window is, which it hands back with each
/// telling, and has Android apply the window's insets once more: the ones it applied before
/// reached nobody.
fn attach(env: &mut Env<'_>, listener: &JObject<'_>, view: ViewId) -> jni::errors::Result<()> {
    env.call_method(
        listener,
        jni_str!("attach"),
        jni_sig!("(J)V"),
        &[JValue::Long(view.0 as jlong)],
    )
    .map(drop)
}

/// What a listener's `onWindowInsets` hands on, on the main thread, for the window the view
/// numbered `view` is: the view padding, the view insets and then the system gesture insets, each
/// side in pixels.
#[allow(clippy::too_many_arguments)]
extern "system" fn on_window_insets<'local>(
    _env: EnvUnowned<'local>,
    _listener: JObject<'local>,
    view: jlong,
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
    locked(&APPLIED).insert(ViewId(view as u64), insets);
    wake_app_loop();
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
