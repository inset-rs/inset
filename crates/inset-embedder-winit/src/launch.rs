//! The app's start, as Flutter's embedders start an app: the embedder and the app are made before
//! winit's loop runs, with the implicit view and no window yet, and the embedder then handles every
//! one of winit's calls, the first of which opens that view's window (`main_loop/`).

use inset_embedder::{EmbedderClient, PlatformRef};
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::WinitEmbedder;
use crate::images::CreateImageLoader;
use crate::main_loop::{MainLoop, MainLoopEvent};
use crate::os::{ApplicationLifecycle, SystemRequests, ThisOs};

/// What starts the app, on the platform it is handed, and answers the client the embedder drives.
pub(crate) type StartApp<C> = Box<dyn FnOnce(PlatformRef) -> C>;

/// Runs the app on winit's loop until it quits.
pub(crate) fn run<C: EmbedderClient + 'static>(
    config: WinitEmbedder,
    start: StartApp<C>,
    create_image_loader: Option<CreateImageLoader>,
) {
    let event_loop = EventLoop::<MainLoopEvent>::with_user_event()
        .build()
        .expect("create winit event loop");
    let proxy = event_loop.create_proxy();
    // Before the loop runs: AppKit hands over the files a launch opens before winit's first call.
    forward_system_messages(&proxy);
    let mut main_loop = MainLoop::start(config, start, create_image_loader, proxy);
    event_loop
        .run_app(&mut main_loop)
        .expect("run winit event loop");
}

/// Sends the loop what the system asks of the application and what it says of it, from wherever
/// the system makes them.
fn forward_system_messages(proxy: &EventLoopProxy<MainLoopEvent>) {
    let requests = proxy.clone();
    ThisOs::send_system_requests(Box::new(move |request| {
        let _ = requests.send_event(MainLoopEvent::SystemRequest(request));
    }));
    let notices = proxy.clone();
    ThisOs::send_lifecycle_notices(Box::new(move |notice| {
        let _ = notices.send_event(MainLoopEvent::Lifecycle(notice));
    }));
}
