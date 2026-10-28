//! The app's start, as Flutter's embedders start an app: the embedder and the app are made before
//! winit's loop runs, with no window yet, and the embedder then handles every one of winit's
//! calls; where the system's window is the screen, the first opens the screen view's
//! (`main_loop/`).

use inset_embedder::{EmbedderClient, PlatformRef};
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::images::CreateImageLoader;
use crate::main_loop::{MainLoop, MainLoopEvent};
use crate::os::{ApplicationLifecycle, NativeMenus, SystemChanges, SystemRequests, ThisOs};
use crate::system_requests::SystemRequest;

/// What starts the app, on the platform it is handed, and answers the client the embedder drives.
pub(crate) type StartApp<C> = Box<dyn FnOnce(PlatformRef) -> C>;

/// Runs the app on winit's loop until it quits.
pub(crate) fn run<C: EmbedderClient + 'static>(
    start: StartApp<C>,
    create_image_loader: Option<CreateImageLoader>,
) {
    let mut builder = EventLoop::<MainLoopEvent>::with_user_event();
    ThisOs::configure_event_loop(&mut builder);
    let event_loop = builder.build().expect("create winit event loop");
    let proxy = event_loop.create_proxy();
    // Before the loop runs: AppKit hands over the files a launch opens before winit's first call.
    forward_system_messages(&proxy);
    let mut main_loop = MainLoop::start(start, create_image_loader, proxy);
    event_loop
        .run_app(&mut main_loop)
        .expect("run winit event loop");
}

/// Sends the loop what the system asks of the application, what it says of it and what it says
/// changed about the machine, from wherever the system makes them.
fn forward_system_messages(proxy: &EventLoopProxy<MainLoopEvent>) {
    if ThisOs::OPENED_BY_ARGUMENTS
        && let Some(launch) = SystemRequest::launch()
    {
        let _ = proxy.send_event(MainLoopEvent::SystemRequest(launch));
    }
    let requests = proxy.clone();
    ThisOs::send_system_requests(Box::new(move |request| {
        let _ = requests.send_event(MainLoopEvent::SystemRequest(request));
    }));
    let notices = proxy.clone();
    ThisOs::send_lifecycle_notices(Box::new(move |notice| {
        let _ = notices.send_event(MainLoopEvent::Lifecycle(notice));
    }));
    let changes = proxy.clone();
    ThisOs::send_system_changes(Box::new(move |change| {
        let _ = changes.send_event(MainLoopEvent::SystemChange(change));
    }));
}
