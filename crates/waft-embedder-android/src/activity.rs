//! One activity as JNI reaches it: the handle for what belongs to this activity and no other. Its
//! window, which carries the system bars' style and the refresh rate asked of the display; what it
//! shows the user over itself, a picker, a permission question, another app's activity; its task's
//! description; and the Java objects Waft's activity made for it, the text input view and the
//! insets listener. What any context answers goes through the application instead
//! (`application.rs`), which is there while the process is.
//!
//! This is where the host's handles are made from android-activity's `AndroidApp`, which the glue
//! hands out for each activity it runs. A later editor must keep it so: a module that calls Java
//! holds an [`Activity`] or an [`Application`](crate::application::Application), never the glue's
//! own type, so that glue of Waft's own replaces what is here and nothing else. Flutter's embedding
//! has the same two, an engine's application context and the activity attached to it
//! (`FlutterPluginBinding.getApplicationContext`, `ActivityPluginBinding.getActivity`).

// The VM and the activity reach this crate as raw pointers, which JNI takes back as its own
// handles, and the release is a C call; that is the `unsafe` here.
#![allow(unsafe_code)]

use android_activity::AndroidApp;
use jni::objects::JObject;
use jni::{Env, JavaVM};

use crate::application::package_class;
use crate::log::warn_to_log;

/// Waft's activity, which declares the native methods the host registers for an activity's
/// results, intents and permission answers.
pub(crate) const WAFT_ACTIVITY: &str = "rs.waft.WaftActivity";

/// One activity of the app's.
#[derive(Clone)]
pub(crate) struct Activity {
    glue: AndroidApp,
}

impl Activity {
    /// The activity the glue runs `glue` for.
    pub(crate) fn of(glue: &AndroidApp) -> Activity {
        Activity { glue: glue.clone() }
    }

    /// Runs `work` with this thread attached to the app's VM and the activity in hand.
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        self.vm().attach_current_thread(|env| {
            // SAFETY: an unowned global reference to the activity, which the glue keeps while
            // `self.glue` lives, borrowed for this frame.
            let activity = unsafe { JObject::from_raw(env, self.glue.activity_as_ptr().cast()) };
            work(env, &activity)
        })
    }

    /// Runs `work` on Android's main thread with the activity in hand, where a window and its
    /// views may be touched, and returns at once; what the system would not take of `what` is said
    /// in the log.
    pub(crate) fn on_main_thread(
        &self,
        what: &'static str,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
    ) {
        let activity = self.clone();
        self.glue.run_on_java_main_thread(Box::new(move || {
            if let Err(error) = activity.with(work) {
                warn_to_log!("the system would not take {what}: {error:?}");
            }
        }));
    }

    /// Whether the activity is Waft's, `rs.waft.WaftActivity` or a subclass of it, which hands
    /// its results, its intents and its permission answers to the methods the host registers; the
    /// bare `NativeActivity` keeps them to itself.
    pub(crate) fn is_wafts(&self) -> bool {
        let is_wafts = self.with(|env, activity| {
            let class = package_class(env, activity, WAFT_ACTIVITY)?;
            env.is_instance_of(activity, &class)
        });
        is_wafts.unwrap_or_else(|_| {
            self.clear_exception();
            false
        })
    }

    /// Clears the Java exception a failed call left on this thread, so the next call into Java is
    /// not refused.
    pub(crate) fn clear_exception(&self) {
        let _ = self.with(|env, _| {
            env.exception_clear();
            Ok(())
        });
    }

    /// The app's VM.
    pub(crate) fn vm(&self) -> JavaVM {
        // SAFETY: this process's own VM, as the glue hands it out.
        unsafe { JavaVM::from_raw(self.glue.vm_as_ptr().cast()) }
    }
}

/// The release this app is running on.
pub(crate) fn device_api_level() -> i32 {
    // SAFETY: a plain query of the running system, part of the platform since API 24.
    unsafe { ndk_sys::android_get_device_api_level() }
}
