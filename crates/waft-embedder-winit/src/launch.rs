//! The app's start, as Flutter's embedders start an app: the embedder and the app are made before
//! winit's loop runs, with no window yet, and the embedder then handles every one of winit's
//! calls; where the system's window is the screen, the first opens the screen view's
//! (`main_loop/`).

use waft_embedder::{EmbedderClient, PlatformRef};
use winit::event_loop::EventLoop;

use crate::LinuxSettings;
use crate::gpu::GpuRequest;
use crate::images::CreateImageLoader;
use crate::main_loop::{MainLoop, MainLoopEvent, MainLoopProxy};
use crate::notifications::NotificationActivations;
use crate::os::System;
use crate::os::capabilities::{
    ApplicationLifecycle, EventLoopSetup, NativeComposition, SystemChanges, SystemNotifications,
    SystemRequests,
};

/// What starts the app, on the platform it is handed, and answers the client the embedder drives.
pub(crate) type StartApp<C> = Box<dyn FnOnce(PlatformRef) -> C>;

/// Runs the app on winit's loop until it quits, with what it tells Linux's desktops, its device
/// on the system's software adapter where `force_fallback_adapter` asks.
pub(crate) fn run<C: EmbedderClient + 'static>(
    start: StartApp<C>,
    create_image_loader: Option<CreateImageLoader>,
    linux: LinuxSettings,
    force_fallback_adapter: bool,
) {
    let mut builder = EventLoop::builder();
    System::configure_event_loop(&mut builder);
    let event_loop = builder.build().expect("create winit event loop");
    System::event_loop_built(&event_loop);
    let (proxy, events) = MainLoopProxy::new(event_loop.create_proxy());
    // Before the loop runs: AppKit hands over the files a launch opens before winit's first call.
    forward_system_messages(&proxy);
    let gpu_request = GpuRequest {
        display: Some(event_loop.owned_display_handle()),
        force_fallback_adapter,
    };
    let main_loop = MainLoop::start(
        start,
        create_image_loader,
        linux,
        gpu_request,
        proxy,
        events,
    );
    event_loop.run_app(main_loop).expect("run winit event loop");
}

/// Sends the loop what the system asks of the application, what it says of it and what it says
/// changed about the machine, from wherever the system makes them.
fn forward_system_messages(proxy: &MainLoopProxy) {
    let requests = proxy.clone();
    System::send_system_requests(Box::new(move |request| {
        requests.send(MainLoopEvent::SystemRequest(request));
    }));
    let notices = proxy.clone();
    System::send_lifecycle_notices(Box::new(move |notice| {
        notices.send(MainLoopEvent::Lifecycle(notice));
    }));
    let changes = proxy.clone();
    System::send_system_changes(Box::new(move |change| {
        changes.send(MainLoopEvent::SystemChange(change));
    }));
    System::hear_notification_activations(NotificationActivations::new(proxy.clone()));
    let waker = proxy.waker();
    System::wake_on_layer_presents(Box::new(move || waker.wake_up()));
}
