//! Android's calls into the app: `ANativeActivity_onCreate`, which `NativeActivity` looks up in
//! the app's library and calls on Android's main thread for each activity it makes, and the
//! callbacks it sets there for that activity (`ANativeActivityCallbacks`).
//!
//! Each activity is a view of the one app. At its creation the activity is given the view it is:
//! the one an activity before it saved, where Android makes it in place of one it destroyed in this
//! process, and a new one otherwise (`saved_view.rs`). The process's first activity starts the
//! app's thread (`app_thread.rs`); from then on each
//! callback hands what Android told it to the app's thread as that view's (`activity_events.rs`),
//! and returns. Flutter's embedding has this shape, an activity's callbacks going from the main
//! thread into the engine (`FlutterActivityAndFragmentDelegate`), though with one activity to an
//! engine.
//!
//! What is ported from android-activity's glue (`native_activity/glue.rs`): the callbacks it sets,
//! the window taken with a reference of the app's own, the configuration read from an asset
//! manager (the one the activity's resources hold now, where the glue reads the one it was
//! handed), the saved state answered in memory `malloc` gave, and the waits where a window or an
//! input queue is taken away. What is dropped: a thread for each activity and its pipe of
//! commands; the waits at start, resume, pause and stop, which Android does not require, as the
//! host takes no lifecycle; the wait at saving the state, since the glue knows the view the
//! activity saves without asking the app; the wait at destruction for that thread to end; and the
//! low memory callback, which the host takes no more than it did.
//!
//! The invariants a later editor will break: a callback never calls the framework, and waits for
//! the app only where `activity_events.rs` says; and nothing here keeps the `ANativeActivity`
//! past the `onDestroy` callback, after which Android deletes it, so the app's thread never
//! touches it, and what an event carries is the app's own (a window acquired, a configuration
//! copied, a global reference to the activity).

// The NDK hands the activity, its window, its input queue and its saved state as raw pointers;
// that is the `unsafe` here.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr::NonNull;

use jni::objects::JObject;
use jni::{Env, jni_sig, jni_str};
use ndk::asset::AssetManager;
use ndk::configuration::Configuration;
use ndk::input_queue::InputQueue;
use ndk::native_window::NativeWindow;
use ndk_sys::{AInputQueue, ANativeActivity, ANativeWindow, ARect};
use waft_embedder::ViewId;

use crate::activity::Activity;
use crate::activity_events::{ActivityEvent, tell, tell_and_wait};
use crate::log::warn_to_log;

/// What the glue keeps of an activity, from its creation to its destruction, as the activity's
/// `instance`.
struct Made {
    /// The view the activity is.
    view: ViewId,
    activity: Activity,
}

/// Android made the activity `native`: it is a view of the app from now on, which the app's
/// thread, started by the process's first activity, is told. `saved_state` is what an activity
/// before it saved, `saved_state_size` bytes, or null.
///
/// # Safety
///
/// `NativeActivity` calls it on Android's main thread with a live activity, as the NDK's
/// `ANativeActivity_createFunc` says.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ANativeActivity_onCreate(
    native: *mut ANativeActivity,
    saved_state: *mut c_void,
    saved_state_size: usize,
) {
    crate::log::forward_standard_streams();
    crate::main_thread::serve_here();
    // SAFETY: `NativeActivity`'s, before the activity's `onDestroy`.
    let activity = unsafe { Activity::of_native(native) }.expect("an activity Android made");
    // SAFETY: the same.
    unsafe { crate::app_thread::start_once(native, &activity) };
    // SAFETY: the same: the saved state is good for the call.
    let saved = unsafe { saved_state_of(saved_state, saved_state_size) };
    let view = crate::saved_view::view_of(saved).unwrap_or_else(crate::views::take_view_id);
    // SAFETY: the same: the callbacks are the activity's to set, and its instance the native
    // code's own, until `onDestroy`.
    unsafe {
        set_callbacks(&mut *(*native).callbacks);
        (*native).instance = Box::into_raw(Box::new(Made {
            view,
            activity: activity.clone(),
        }))
        .cast();
    }
    // SAFETY: the same.
    let configuration = unsafe { configuration_of(&activity, native) };
    tell(
        view,
        ActivityEvent::Created {
            activity,
            configuration,
            restored: !saved.is_empty(),
        },
    );
}

/// The state an activity before saved, `size` bytes at `saved`, or none.
///
/// # Safety
///
/// `saved` is null, or the start of `size` bytes good for `'a`.
unsafe fn saved_state_of<'a>(saved: *const c_void, size: usize) -> &'a [u8] {
    if saved.is_null() {
        return &[];
    }
    // SAFETY: the caller's.
    unsafe { std::slice::from_raw_parts(saved.cast(), size) }
}

/// Has Android call the host for what it tells `callbacks`' activity. The lifecycle's own steps,
/// start, resume, pause and stop, and low memory are left unset: the host takes none of them.
fn set_callbacks(callbacks: &mut ndk_sys::ANativeActivityCallbacks) {
    callbacks.onDestroy = Some(on_destroy);
    callbacks.onSaveInstanceState = Some(on_save_instance_state);
    callbacks.onConfigurationChanged = Some(on_configuration_changed);
    callbacks.onWindowFocusChanged = Some(on_window_focus_changed);
    callbacks.onNativeWindowCreated = Some(on_native_window_created);
    callbacks.onNativeWindowResized = Some(on_native_window_resized);
    callbacks.onNativeWindowRedrawNeeded = Some(on_native_window_redraw_needed);
    callbacks.onNativeWindowDestroyed = Some(on_native_window_destroyed);
    callbacks.onInputQueueCreated = Some(on_input_queue_created);
    callbacks.onInputQueueDestroyed = Some(on_input_queue_destroyed);
    callbacks.onContentRectChanged = Some(on_content_rect_changed);
}

/// What the glue keeps of `native`.
///
/// # Safety
///
/// `native` is an activity `ANativeActivity_onCreate` was called with, before its `onDestroy`
/// returned.
unsafe fn made<'a>(native: *mut ANativeActivity) -> &'a Made {
    // SAFETY: the caller's: the instance is the `Made` the activity's creation set.
    unsafe { &*(*native).instance.cast::<Made>() }
}

/// The view `native` is.
///
/// # Safety
///
/// As [`made`].
unsafe fn view_of(native: *mut ANativeActivity) -> ViewId {
    // SAFETY: the caller's.
    unsafe { made(native) }.view
}

/// The configuration of `activity`'s resources now, read from the asset manager its resources
/// hold now (`getResources().getAssets()`). The one `NativeActivity` handed the native code at
/// its creation (`ANativeActivity.assetManager`) is not the one Android updates at a change of
/// configuration: on the emulator's API 37 it kept the mode the activity started in through a
/// switch to dark mode and back. Where Java cannot be asked, that one is read, `native`'s.
///
/// # Safety
///
/// As [`made`]: the activity's asset manager lives while it does.
unsafe fn configuration_of(activity: &Activity, native: *mut ANativeActivity) -> Configuration {
    match activity.with(current_configuration) {
        Ok(configuration) => configuration,
        Err(error) => {
            activity.clear_exception();
            warn_to_log!("waft: the resources' configuration is not read: {error}");
            // SAFETY: the caller's; the manager is borrowed for the call, not owned.
            let assets =
                unsafe { AssetManager::from_ptr(NonNull::new_unchecked((*native).assetManager)) };
            Configuration::from_asset_manager(&assets)
        }
    }
}

/// The configuration of the asset manager `activity`'s resources hold.
fn current_configuration(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
) -> jni::errors::Result<Configuration> {
    let resources = env
        .call_method(
            activity,
            jni_str!("getResources"),
            jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )?
        .l()?;
    let assets = env
        .call_method(
            &resources,
            jni_str!("getAssets"),
            jni_sig!("()Landroid/content/res/AssetManager;"),
            &[],
        )?
        .l()?;
    // SAFETY: `assets` is a live `android.content.res.AssetManager` for the call, which the native
    // manager answered belongs to, and the configuration is copied out of it before it returns.
    let manager = unsafe { ndk_sys::AAssetManager_fromJava(env.get_raw().cast(), assets.as_raw()) };
    let manager = NonNull::new(manager).ok_or(jni::errors::Error::NullPtr("the asset manager"))?;
    // SAFETY: the same.
    let assets = unsafe { AssetManager::from_ptr(manager) };
    Ok(Configuration::from_asset_manager(&assets))
}

/// Android destroys the activity: gone for good where it is finishing, and otherwise to be made
/// again. What the glue kept of it goes, as Android deletes `native` once this returns.
unsafe extern "C" fn on_destroy(native: *mut ANativeActivity) {
    // SAFETY: Android's, which calls this last; the instance is taken back once, and cleared.
    let made = unsafe { Box::from_raw((*native).instance.cast::<Made>()) };
    unsafe { (*native).instance = std::ptr::null_mut() };
    let finishing = made.activity.is_finishing().unwrap_or_else(|error| {
        made.activity.clear_exception();
        warn_to_log!("waft: whether the activity is finishing is not known: {error}");
        true
    });
    tell(made.view, ActivityEvent::Destroyed { finishing });
}

/// Android asks what the activity would have back if it is made again: the token of its view
/// (`saved_view.rs`), in memory `malloc` gave, which `NativeActivity` copies and frees.
unsafe extern "C" fn on_save_instance_state(
    native: *mut ANativeActivity,
    saved_size: *mut usize,
) -> *mut c_void {
    // SAFETY: Android's, between the activity's creation and destruction.
    let token = crate::saved_view::token(unsafe { view_of(native) });
    // SAFETY: plain allocation, written in full below before it is handed over.
    let saved = unsafe { libc::malloc(token.len()) };
    if saved.is_null() {
        // SAFETY: Android's, which reads the size it handed over.
        unsafe { *saved_size = 0 };
        return saved;
    }
    // SAFETY: `saved` holds `token.len()` bytes, and `saved_size` is Android's to read.
    unsafe {
        std::ptr::copy_nonoverlapping(token.as_ptr(), saved.cast::<u8>(), token.len());
        *saved_size = token.len();
    }
    saved
}

unsafe extern "C" fn on_configuration_changed(native: *mut ANativeActivity) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let made = unsafe { made(native) };
    // SAFETY: the same.
    let configuration = unsafe { configuration_of(&made.activity, native) };
    tell(
        made.view,
        ActivityEvent::ConfigurationChanged(configuration),
    );
}

unsafe extern "C" fn on_window_focus_changed(native: *mut ANativeActivity, focused: i32) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let view = unsafe { view_of(native) };
    tell(view, ActivityEvent::FocusChanged(focused != 0));
}

/// The activity's window, which the app takes a reference of its own to.
unsafe extern "C" fn on_native_window_created(
    native: *mut ANativeActivity,
    window: *mut ANativeWindow,
) {
    // SAFETY: Android's: a live window, acquired here and released when the app drops it.
    let (view, window) = unsafe {
        (
            view_of(native),
            NativeWindow::clone_from_ptr(NonNull::new_unchecked(window)),
        )
    };
    tell(view, ActivityEvent::WindowCreated(window));
}

unsafe extern "C" fn on_native_window_resized(
    native: *mut ANativeActivity,
    _window: *mut ANativeWindow,
) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let view = unsafe { view_of(native) };
    tell(view, ActivityEvent::WindowResized);
}

unsafe extern "C" fn on_native_window_redraw_needed(
    native: *mut ANativeActivity,
    _window: *mut ANativeWindow,
) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let view = unsafe { view_of(native) };
    tell(view, ActivityEvent::WindowRedrawNeeded);
}

/// The window is taken away, and destroyed once this returns: it waits until the app has let go.
unsafe extern "C" fn on_native_window_destroyed(
    native: *mut ANativeActivity,
    _window: *mut ANativeWindow,
) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let view = unsafe { view_of(native) };
    tell_and_wait(view, ActivityEvent::WindowDestroyed);
}

unsafe extern "C" fn on_input_queue_created(native: *mut ANativeActivity, queue: *mut AInputQueue) {
    // SAFETY: Android's: a live queue, until the callback that takes it away.
    let (view, queue) = unsafe {
        (
            view_of(native),
            InputQueue::from_ptr(NonNull::new_unchecked(queue)),
        )
    };
    tell(view, ActivityEvent::InputQueueCreated(queue));
}

/// The input queue is taken away, and destroyed once this returns: it waits until the app has
/// detached it from its looper.
unsafe extern "C" fn on_input_queue_destroyed(
    native: *mut ANativeActivity,
    _queue: *mut AInputQueue,
) {
    // SAFETY: Android's, between the activity's creation and destruction.
    let view = unsafe { view_of(native) };
    tell_and_wait(view, ActivityEvent::InputQueueDestroyed);
}

unsafe extern "C" fn on_content_rect_changed(native: *mut ANativeActivity, rect: *const ARect) {
    // SAFETY: Android's: the rectangle is good for the call.
    let (view, rect) = unsafe { (view_of(native), *rect) };
    tell(
        view,
        ActivityEvent::ContentRectChanged([rect.left, rect.top, rect.right, rect.bottom]),
    );
}
