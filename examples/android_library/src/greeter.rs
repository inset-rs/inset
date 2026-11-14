//! The AAR's class, called through JNI: `Greeter.greet(Context)`, with the application as the
//! context.
//!
//! The class is loaded through the application's class loader, the package's: a thread the app
//! started finds no class of the package's by its name alone.

// Taking the VM and the application from `ndk-context` as raw pointers is the `unsafe` here.
#![allow(unsafe_code)]

use jni::objects::{JClassLoader, JObject, JString, JValue};
use jni::refs::LoaderContext;
use jni::strings::JNIString;
use jni::{JavaVM, jni_sig, jni_str};

/// The library's class, by its binary name.
const GREETER: &str = "com.example.greeter.Greeter";

/// What the greeter answers.
pub(crate) fn greet() -> jni::errors::Result<String> {
    let context = ndk_context::android_context();
    // SAFETY: the process's VM, which Waft's Android host handed to `ndk-context`.
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: the global reference to the application that `ndk-context` keeps for the
        // process, borrowed for this frame.
        let application = unsafe { JObject::from_raw(env, context.context().cast()) };
        let loader = env
            .call_method(
                &application,
                jni_str!("getClassLoader"),
                jni_sig!("()Ljava/lang/ClassLoader;"),
                &[],
            )?
            .l()?;
        let loader = env.cast_local::<JClassLoader>(loader)?;
        let greeter =
            LoaderContext::Loader(&loader).load_class(env, JNIString::new(GREETER), false)?;
        let answer = env
            .call_static_method(
                &greeter,
                jni_str!("greet"),
                jni_sig!("(Landroid/content/Context;)Ljava/lang/String;"),
                &[JValue::Object(&application)],
            )?
            .l()?;
        env.cast_local::<JString>(answer)?.try_to_string(env)
    })
}
