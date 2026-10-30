//! The app's activities as JNI reaches them, the application that outlives them, and the release of
//! Android the app runs on, which decides how the system is asked.
//!
//! Each activity the system creates attaches to the app as a view and leaves it when it is
//! destroyed (`app_thread`). What the app asks of Java goes through one of three:
//!
//! - an activity of its own, for what belongs to that activity: its window, its display, its
//!   intent ([`with_activity`]);
//! - the activity in front, the one the user last turned to, for what shows the user something:
//!   a picker, a question, another app ([`Activities::with_front`]);
//! - the application, for what any context answers: the clipboard, documents, notifications
//!   ([`with_application`]). It is there for the whole process, so this keeps working while the app
//!   has no activity.
//!
//! [`Activities`] belongs to the app's thread; only an [`AndroidApp`] taken out of it travels to
//! another.

// The VM, the activity and the application reach this crate as raw pointers, which JNI takes back
// as its own handles, and the release is a C call; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

use android_activity::AndroidApp;
use inset_embedder::ViewId;
use jni::objects::{JClass, JClassLoader, JObject};
use jni::refs::LoaderContext;
use jni::{Env, jni_sig, jni_str};

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

/// Runs `work` with this thread attached to the app's VM and the application in hand, as a
/// `Context`.
pub(crate) fn with_application<T>(
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> jni::errors::Result<T> {
    // android-activity sets the VM and the application down before the first `android_main`.
    let context = ndk_context::android_context();
    // SAFETY: this process's own VM, as android-activity set it down.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: the global reference to the application android-activity keeps for the
        // process's life, borrowed for this frame.
        let application = unsafe { JObject::from_raw(env, context.context().cast()) };
        work(env, &application)
    })
}

/// The package's class `name`, through `context`'s class loader, which is the package's: a thread
/// the app started finds no class of the package's by its name alone, and the application's own
/// class is the platform's, whose loader knows none of them either.
pub(crate) fn package_class<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    name: &str,
) -> jni::errors::Result<JClass<'local>> {
    let loader = env
        .call_method(
            context,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?;
    let loader = env.cast_local::<JClassLoader>(loader)?;
    LoaderContext::Loader(&loader).load_class(env, jni::strings::JNIString::new(name), false)
}

/// Clears the Java exception a failed call left on this thread, so the next call into Java is not
/// refused.
pub(crate) fn clear_exception() {
    let _ = with_application(|env, _| {
        env.exception_clear();
        Ok(())
    });
}

/// The release this app is running on.
pub(crate) fn device_api_level() -> i32 {
    // SAFETY: a plain query of the running system, part of the platform since API 24.
    unsafe { ndk_sys::android_get_device_api_level() }
}

/// The activities attached to the app, each by the view it is, the one in front last.
#[derive(Clone, Default)]
pub(crate) struct Activities {
    attached: Rc<RefCell<Vec<Attached>>>,
}

struct Attached {
    view: ViewId,
    app: AndroidApp,
}

impl Activities {
    /// An activity the system created, in front until another is.
    pub(crate) fn attach(&self, view: ViewId, app: AndroidApp) {
        self.attached.borrow_mut().push(Attached { view, app });
    }

    /// The activity `view` is, destroyed.
    pub(crate) fn detach(&self, view: ViewId) {
        self.attached
            .borrow_mut()
            .retain(|attached| attached.view != view);
    }

    /// The activity `view` is took the focus: the user turned to it.
    pub(crate) fn bring_to_front(&self, view: ViewId) {
        let mut attached = self.attached.borrow_mut();
        if let Some(index) = attached.iter().position(|attached| attached.view == view) {
            let turned_to = attached.remove(index);
            attached.push(turned_to);
        }
    }

    /// The activity `view` is.
    pub(crate) fn of(&self, view: ViewId) -> Option<AndroidApp> {
        self.attached
            .borrow()
            .iter()
            .find(|attached| attached.view == view)
            .map(|attached| attached.app.clone())
    }

    /// The activity the user last turned to; `None` while the app has none.
    pub(crate) fn front(&self) -> Option<AndroidApp> {
        self.attached
            .borrow()
            .last()
            .map(|attached| attached.app.clone())
    }

    /// Runs `work` with this thread attached to the app's VM and the activity in front in hand; an
    /// error while the app has no activity.
    pub(crate) fn with_front<T>(
        &self,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        let front = self
            .front()
            .ok_or(jni::errors::Error::NullPtr("the activity in front"))?;
        with_activity(&front, work)
    }
}
