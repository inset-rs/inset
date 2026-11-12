//! The process's `Application` as JNI reaches it: the context for what any context answers, the
//! clipboard, documents and their permissions, the package's manifest and the apps that take a
//! link, notifications, and the package's own classes. It is there while the process is, whichever
//! activities come and go, so what holds it keeps working with no activity at all. What needs one
//! activity and no other goes through that activity (`activity.rs`).
//!
//! The package's classes are loaded by name through a context's class loader, which is the
//! package's ([`package_class`]): a thread the app started finds no class of the package's by its
//! name alone, and the application's own class is the platform's, whose loader knows none of them.
//! The native methods a class declares are registered on the class loaded so, once for the process
//! ([`Application::register_native_methods`]), whichever of its objects call them later. A later
//! editor must keep both so: a class taken from one activity's object, or a method registered as
//! each activity starts, ties the process's code to one activity again.

// Registering native methods takes their functions as raw pointers, whose signatures the caller
// answers for, and `ndk-context` takes the VM and the application as raw pointers; that is the
// `unsafe` here.
#![allow(unsafe_code)]

use std::sync::Arc;

use jni::objects::{Global, JClass, JClassLoader, JObject};
use jni::refs::LoaderContext;
use jni::strings::JNIString;
use jni::{Env, JavaVM, NativeMethod, jni_sig, jni_str};

use crate::activity::Activity;

/// The process's application.
#[derive(Clone)]
pub(crate) struct Application {
    vm: JavaVM,
    context: Arc<Global<JObject<'static>>>,
}

impl Application {
    /// The application `activity` is of.
    pub(crate) fn of(activity: &Activity) -> jni::errors::Result<Application> {
        let vm = activity.vm();
        activity.with(|env, activity| {
            let application = env
                .call_method(
                    activity,
                    jni_str!("getApplication"),
                    jni_sig!("()Landroid/app/Application;"),
                    &[],
                )?
                .l()?;
            Ok(Application {
                vm,
                context: Arc::new(env.new_global_ref(application)?),
            })
        })
    }

    /// The app's VM.
    pub(crate) fn vm(&self) -> JavaVM {
        self.vm.clone()
    }

    /// The application as JNI's raw global reference, which this handle keeps.
    pub(crate) fn as_raw(&self) -> jni::sys::jobject {
        self.context.as_raw()
    }

    /// Hands the VM and the application to `ndk-context`, through which crates written for
    /// Android find them, such as cpal's audio, as android-activity did; once for the process.
    pub(crate) fn hand_to_ndk_context(&self) {
        let kept = self.with(|env, application| env.new_global_ref(application));
        match kept {
            Ok(kept) => {
                // SAFETY: the VM is this process's and the reference a global one, which is never
                // deleted: `ndk-context` hands both out for the rest of the process.
                unsafe {
                    ndk_context::initialize_android_context(
                        self.vm.get_raw().cast(),
                        kept.into_raw().cast(),
                    );
                }
            }
            Err(error) => crate::log::warn_to_log!("waft: ndk-context has no application: {error}"),
        }
    }

    /// Runs `work` with this thread attached to the app's VM and the application in hand, as a
    /// `Context`.
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        self.vm
            .attach_current_thread(|env| work(env, self.context.as_obj()))
    }

    /// Registers `methods` on the package's class `class`, which declares them. An error, the
    /// exception cleared, in a package without the class, where they have nowhere to be
    /// registered.
    ///
    /// # Safety
    ///
    /// Each method's function must take the arguments its signature names.
    pub(crate) unsafe fn register_native_methods(
        &self,
        class: &str,
        methods: &[NativeMethod],
    ) -> jni::errors::Result<()> {
        let registered = self.with(|env, application| {
            let class = package_class(env, application, class)?;
            // SAFETY: the caller's.
            unsafe { env.register_native_methods(&class, methods) }
        });
        if registered.is_err() {
            self.clear_exception();
        }
        registered
    }

    /// Clears the Java exception a failed call left on this thread, so the next call into Java is
    /// not refused.
    pub(crate) fn clear_exception(&self) {
        let _ = self.with(|env, _| {
            env.exception_clear();
            Ok(())
        });
    }
}

/// The package's class of the binary name `name`, through the class loader of `context`, the
/// application or an activity at hand.
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
    LoaderContext::Loader(&loader).load_class(env, JNIString::new(name), false)
}
