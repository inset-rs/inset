//! The app's thread, and the [`AndroidApp`] its entry is handed.
//!
//! The process's first activity starts the thread (`native_activity.rs`), and it runs for the rest
//! of the process, whichever activities Android makes and destroys meanwhile: it has a looper of
//! its own, which every activity's input queue and the display's signal are attached to, and runs
//! the app's entry, `android_main`, once. Android calls activities on its main thread alone, and
//! android-activity, which Waft's host was built on before, started a thread for each activity
//! and ran the app on it, so the app ended with its activity; here no thread stands for an
//! activity.
//!
//! The invariants a later editor will break: the entry runs once a process, on this thread, and
//! everything of the framework's, the client, the views, the device and the display's signal, is
//! this thread's (`frame_fence.rs` requires every submission on the device's queue from one
//! thread). A panic here ends the process, as one in `main` does, rather than leaving activities
//! whose events nothing takes.

// The app's entry is a symbol the app's library defines, whose signature the compiler cannot check
// across it, and the activity hands its folder as a C string; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::ffi::{CStr, OsStr, c_void};
use std::os::unix::ffi::OsStrExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Once;

use jni::vm::AttachConfig;
use jni::{JavaVM, jni_str};
use ndk::looper::ThreadLooper;

use crate::activity::Activity;
use crate::application::Application;

unsafe extern "Rust" {
    /// The app's entry, which `#[waft::main]` writes: it runs the host
    /// (`AndroidEmbedder::run`) with what it is handed.
    fn android_main(app: AndroidApp);
}

/// The process's app as Android starts it, which its entry is handed: the VM, the application,
/// and the folder Android gives the app's files. There is one a process, made with the first
/// activity, and it outlives every activity.
pub struct AndroidApp {
    application: Application,
    files: Option<PathBuf>,
}

impl AndroidApp {
    /// The app `native`, the activity Android is making, is of.
    ///
    /// # Safety
    ///
    /// `native` is the activity `NativeActivity` handed over, before its `onDestroy` returned.
    unsafe fn of_native(native: *mut ndk_sys::ANativeActivity, activity: &Activity) -> AndroidApp {
        let application = Application::of(activity).expect("an activity hands out its application");
        // SAFETY: the caller's.
        let files = unsafe { (*native).internalDataPath };
        let files = (!files.is_null()).then(|| {
            // SAFETY: a C string the activity holds while it lives.
            let files = unsafe { CStr::from_ptr(files) };
            PathBuf::from(OsStr::from_bytes(files.to_bytes()))
        });
        AndroidApp { application, files }
    }

    /// This process's `JavaVM`, as JNI's raw pointer.
    pub fn vm_as_ptr(&self) -> *mut c_void {
        self.application.vm().get_raw().cast()
    }

    /// The process's `android.app.Application`, the context for what any context answers, as a
    /// JNI global reference the app holds for the process.
    pub fn application_as_ptr(&self) -> *mut c_void {
        self.application.as_raw().cast()
    }

    /// The folder Android gives the app's own files, `Context.getFilesDir`.
    pub fn internal_data_path(&self) -> Option<PathBuf> {
        self.files.clone()
    }

    pub(crate) fn application(&self) -> &Application {
        &self.application
    }
}

/// Starts the app's thread, unless it is started: Android's main thread calls it as it makes an
/// activity, `native`, whose handle is `activity`.
///
/// # Safety
///
/// `native` is the activity `NativeActivity` handed over, before its `onDestroy` returned.
pub(crate) unsafe fn start_once(native: *mut ndk_sys::ANativeActivity, activity: &Activity) {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        // SAFETY: the caller's.
        let app = unsafe { AndroidApp::of_native(native, activity) };
        app.application.hand_to_ndk_context();
        std::thread::Builder::new()
            .name("waft-app".into())
            .spawn(move || run(app))
            .expect("a thread for the app");
    });
}

/// The app's thread: attached to the VM, with its looper, running the app's entry.
fn run(app: AndroidApp) {
    attach_to(&app.application.vm());
    crate::loop_waker::serve(&ThreadLooper::prepare());
    // SAFETY: the app's library defines the entry with this signature: `#[waft::main]` writes it,
    // and an app that writes its own takes `AndroidApp` as Waft's documentation says.
    let ran = catch_unwind(AssertUnwindSafe(|| unsafe { android_main(app) }));
    crate::activity_events::close();
    if ran.is_err() {
        // The panic's message is in the log already, from the standard error (`log.rs`).
        std::process::abort();
    }
}

/// Attaches this thread to `vm` for the rest of its life, named as it is in the system's tools.
fn attach_to(vm: &JavaVM) {
    let attached = vm.attach_current_thread_with_config(
        || AttachConfig::new().thread_name(jni_str!("waft-app")),
        None,
        |_| Ok::<_, jni::errors::Error>(()),
    );
    if let Err(error) = attached {
        crate::log::warn_to_log!("waft: the app's thread is not attached to the VM: {error}");
    }
}
