//! One activity as JNI reaches it: the handle for what belongs to this activity and no other. Its
//! window, which carries the system bars' style and the refresh rate asked of the display; what it
//! shows the user over itself, a picker, a permission question, another app's activity; its task's
//! description; and the Java objects Waft's activity made for it, the text input view and the
//! insets listener. What any context answers goes through the application instead
//! (`application.rs`), which is there while the process is.
//!
//! An activity comes and goes while the app runs on: Android destroys one and makes another, and
//! the app outlives both. So what outlives activities, a view or the app, holds the one attached to
//! it now ([`AttachedActivity`]), as Flutter's engine holds the activity attached to it
//! (`ActivityControlSurface.attachToActivity`, `ActivityPluginBinding.getActivity`), and never an
//! [`Activity`] of its own: one kept would be a destroyed activity once Android made the next.
//!
//! The handle is made where Android hands the activity over (`native_activity.rs`), as a reference
//! of the host's own: the one `NativeActivity` holds is good only until the activity's `onDestroy`
//! returns, and the app's thread may still call into an activity after that, harmlessly.

// The VM and the activity reach this crate as raw pointers, which JNI takes back as its own
// handles; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use jni::objects::{Global, JObject};
use jni::{Env, JavaVM, jni_sig, jni_str};

use crate::application::package_class;
use crate::log::warn_to_log;

/// Waft's activity, which declares the native methods the host registers for an activity's
/// results, intents and permission answers.
pub(crate) const WAFT_ACTIVITY: &str = "rs.waft.WaftActivity";

/// One activity of the app's.
#[derive(Clone)]
pub(crate) struct Activity {
    vm: JavaVM,
    object: Arc<Global<JObject<'static>>>,
}

impl Activity {
    /// The activity `native` is, as `NativeActivity` hands it to its native code, on Android's
    /// main thread.
    ///
    /// # Safety
    ///
    /// `native` is the activity `NativeActivity` handed over, before its `onDestroy` returned.
    pub(crate) unsafe fn of_native(
        native: *mut ndk_sys::ANativeActivity,
    ) -> jni::errors::Result<Activity> {
        // SAFETY: the caller's: the activity carries this process's VM and a global reference to
        // its Java object, both good while the activity is.
        let (vm, object) = unsafe {
            (
                JavaVM::from_raw((*native).vm.cast()),
                (*native).clazz.cast(),
            )
        };
        let object = vm.attach_current_thread(|env| {
            // SAFETY: the reference `native` holds, borrowed for this frame.
            let object = unsafe { JObject::from_raw(env, object) };
            env.new_global_ref(object)
        })?;
        Ok(Activity {
            vm,
            object: Arc::new(object),
        })
    }

    /// Runs `work` with this thread attached to the app's VM and the activity in hand.
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        self.vm
            .attach_current_thread(|env| work(env, self.object.as_obj()))
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
        crate::main_thread::run_on_main_thread(move || {
            if let Err(error) = activity.with(work) {
                warn_to_log!("the system would not take {what}: {error:?}");
            }
        });
    }

    /// Whether this is the same activity as `other`.
    pub(crate) fn is(&self, other: &Activity) -> bool {
        Arc::ptr_eq(&self.object, &other.object)
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

    /// Whether the activity is gone for good, `Activity.isFinishing`: finished by the app or the
    /// user, rather than destroyed by Android to be made again.
    pub(crate) fn is_finishing(&self) -> jni::errors::Result<bool> {
        self.with(|env, activity| {
            env.call_method(activity, jni_str!("isFinishing"), jni_sig!("()Z"), &[])?
                .z()
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
        self.vm.clone()
    }
}

/// The activity attached to what outlives activities, a view or the app, while one is: the one
/// Android made last for it, until Android destroys it. The app's thread's alone.
#[derive(Clone, Default)]
pub(crate) struct AttachedActivity {
    attached: Rc<RefCell<Option<Activity>>>,
}

impl AttachedActivity {
    /// `activity` is the one attached from now on.
    pub(crate) fn attach(&self, activity: &Activity) {
        *self.attached.borrow_mut() = Some(activity.clone());
    }

    /// `activity` was destroyed: nothing is attached, unless another was since.
    pub(crate) fn detach(&self, activity: &Activity) {
        let mut attached = self.attached.borrow_mut();
        if attached.as_ref().is_some_and(|known| known.is(activity)) {
            *attached = None;
        }
    }

    /// The activity attached now.
    pub(crate) fn get(&self) -> Option<Activity> {
        self.attached.borrow().clone()
    }

    /// Runs `work` with the attached activity in hand, as [`Activity::with`]; an error while none
    /// is attached.
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        match self.get() {
            Some(activity) => activity.with(work),
            None => Err(jni::errors::Error::NullPtr(
                "the attached activity, while none is",
            )),
        }
    }

    /// Runs `work` on Android's main thread with the attached activity in hand, as
    /// [`Activity::on_main_thread`]; nothing while none is attached.
    pub(crate) fn on_main_thread(
        &self,
        what: &'static str,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
    ) {
        match self.get() {
            Some(activity) => activity.on_main_thread(what, work),
            None => warn_to_log!("waft: no activity to take {what}"),
        }
    }

    /// Clears the Java exception a failed call left on this thread.
    pub(crate) fn clear_exception(&self) {
        if let Some(activity) = self.get() {
            activity.clear_exception();
        }
    }
}

/// The release this app is running on.
pub(crate) fn device_api_level() -> i32 {
    // SAFETY: a plain query of the running system, part of the platform since API 24.
    unsafe { ndk_sys::android_get_device_api_level() }
}
