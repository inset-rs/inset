//! What the embedder knows that both of its sides need: the main loop, which changes it as the
//! system's events and the app's requests come in, and the platform, which answers the app from it
//! at once (`main_loop/`, `platform.rs`). The shape of winit's own `AppState` on macOS: one owner,
//! which both sides reach through its parts' named operations, never through fields.

use std::rc::Rc;

use winit::event_loop::EventLoopProxy;

use crate::app_requests::AppRequests;
use crate::appearance::Appearance;
use crate::images::{CreateImageLoader, Images};
use crate::locales::Locales;
use crate::main_loop::MainLoopEvent;
use crate::system_requests::SystemRequestHandlers;
use crate::view::WinitView;
use crate::windows::{OpenWindow, Views};

/// What the main loop and the platform both know.
pub(crate) struct EmbedderState {
    /// The app's views and their windows.
    views: Views,
    /// Light or dark, as the app was last told.
    appearance: Appearance,
    /// The user's languages, as the app was last told.
    locales: Locales,
    /// The image loader and the renderer's image store, once the first view has a device.
    images: Images,
    /// What the app asked of the main loop and it has not served yet. Shared with the window
    /// maker and the app's handles on its windows, which send to it.
    requests: Rc<AppRequests>,
    /// The app's handlers for what the system asks of it as a whole.
    handlers: SystemRequestHandlers,
}

impl EmbedderState {
    /// The state of an app about to start: no view yet, and the user's languages as the system
    /// lists them now. Requests reach the main loop through `proxy`.
    pub(crate) fn new(
        proxy: EventLoopProxy<MainLoopEvent>,
        create_image_loader: Option<CreateImageLoader>,
    ) -> EmbedderState {
        EmbedderState {
            views: Views::default(),
            appearance: Appearance::default(),
            locales: Locales::read(),
            images: Images::new(create_image_loader),
            requests: Rc::new(AppRequests::new(move |event| {
                let _ = proxy.send_event(event);
            })),
            handlers: SystemRequestHandlers::default(),
        }
    }

    pub(crate) fn views(&self) -> &Views {
        &self.views
    }

    pub(crate) fn appearance(&self) -> &Appearance {
        &self.appearance
    }

    pub(crate) fn locales(&self) -> &Locales {
        &self.locales
    }

    pub(crate) fn images(&self) -> &Images {
        &self.images
    }

    pub(crate) fn requests(&self) -> &Rc<AppRequests> {
        &self.requests
    }

    pub(crate) fn handlers(&self) -> &SystemRequestHandlers {
        &self.handlers
    }

    /// The screen view, made as the app starts: among the views from now on, for the app's whole
    /// life, and the image loader starts on its device.
    pub(crate) fn add_screen_view(&self, view: Rc<WinitView>) {
        self.images.start_once(view.image_context());
        self.views.add_screen_view(view);
    }

    /// A view the app asked for, with its window: among the views from now on, and the image
    /// loader starts on its device if it is the first.
    pub(crate) fn add_view_with_window(&self, open: OpenWindow) {
        self.images.start_once(open.view.image_context());
        self.views.add_with_window(open);
    }
}
