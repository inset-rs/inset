//! The host's entry: the process handed to `UIApplicationMain`, and the app started once the launch
//! finishes, by Waft's application delegate (`app_delegate.rs`).
//!
//! UIKit makes the delegate itself, by its class's name, so the start closure waits here for it,
//! and the classes UIKit makes by name, the application delegate and the scene delegate the
//! `Info.plist` names, are registered with the Objective-C runtime before `UIApplicationMain` runs.

use std::cell::RefCell;

use objc2::{ClassType, MainThreadMarker};
use objc2_foundation::NSString;
use objc2_ui_kit::UIApplication;
use waft_embedder::{EmbedderClient, PlatformRef};

use crate::app_delegate::WaftAppDelegate;
use crate::host::{Host, Serve};
use crate::scene_delegate::WaftSceneDelegate;

/// Makes the host as the launch finishes.
type Launch = Box<dyn FnOnce(MainThreadMarker) -> Box<dyn Serve>>;

thread_local! {
    /// What makes the host, from `run` until the launch finishes. Main thread.
    static LAUNCH: RefCell<Option<Launch>> = const { RefCell::new(None) };
}

/// Runs a Waft application on iOS.
#[derive(Default)]
pub struct IosEmbedder {}

impl IosEmbedder {
    pub fn new() -> IosEmbedder {
        IosEmbedder {}
    }

    /// Hands the process to UIKit for the rest of its life. `start` runs once, as the launch
    /// finishes, with the implicit view in `Platform::views` and no window for it yet, and returns
    /// the client the host drives.
    ///
    /// # Panics
    ///
    /// Off the main thread, which `main` is called on.
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) -> ! {
        let main_thread =
            MainThreadMarker::new().expect("IosEmbedder::run on the main thread, which main is on");
        let launch: Launch = Box::new(move |main_thread| {
            Box::new(Host::launch(main_thread, start)) as Box<dyn Serve>
        });
        LAUNCH.set(Some(launch));
        let _ = WaftSceneDelegate::class();
        let delegate = NSString::from_class(WaftAppDelegate::class());
        UIApplication::main(None, Some(&delegate), main_thread)
    }
}

/// The launch finished: the host is made, and serves what UIKit told meanwhile. Once a process.
pub(crate) fn launched(main_thread: MainThreadMarker) {
    let Some(launch) = LAUNCH.take() else {
        return;
    };
    let host = waft_embedder_apple::abort_on_panic(|| launch(main_thread));
    crate::host::install(host);
}
