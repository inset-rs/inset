//! The activity as JNI reaches it, and the release of Android the app runs on, which decides how
//! the system is asked.

// The VM and the activity reach this crate as raw pointers, which JNI takes back as its own
// handles, and the release is a C call; that is the `unsafe` here.
#![allow(unsafe_code)]

use android_activity::AndroidApp;
use jni::Env;
use jni::objects::JObject;

/// Runs `work` with this thread attached to the app's VM and the activity in hand.
pub(crate) fn with_activity<T>(
    app: &AndroidApp,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> jni::errors::Result<T> {
    // SAFETY: this process's own VM, as android-activity hands it out.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: an unowned global reference to the activity, borrowed for this frame.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        work(env, &activity)
    })
}

/// The release this app is running on.
pub(crate) fn device_api_level() -> i32 {
    // SAFETY: a plain query of the running system, part of the platform since API 24.
    unsafe { ndk_sys::android_get_device_api_level() }
}
