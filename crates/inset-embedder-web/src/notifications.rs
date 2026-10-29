//! Notifications: the browser's `Notification`, which a page shows while it runs. A notification is
//! shown with the app's id as its `tag`, so the browser puts a later one under the same id in its
//! place, alerting again unless it is silent; the page keeps the `Notification` it made for each id,
//! so `remove` closes the one showing, and forgets it once the browser says it closed. A click is
//! heard by the notification's `tag`, and the browser's own response to it, bringing the page
//! forward, is left as it is.
//!
//! These are the page's own notifications, which have no buttons: buttons, typed replies and the
//! clicks that come while no page runs are a service worker's (`showNotification`,
//! `notificationclick`), and Inset gives the app none. So a notification shows without its
//! actions, a click on it names none, a click once the page has closed reaches nothing, and nothing
//! is scheduled. Where the browser refuses a page's own notifications, as Chrome on Android does,
//! the post fails with what it threw.
//!
//! The permission is the browser's for the page's origin. Only the app's `request` asks, and it
//! asks at once, before its future is awaited, as browsers show their question only while the page
//! handles the user's click or key; a post never asks, and answers `Denied` unless the user allowed
//! the page.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use inset_embedder::{
    IconImage, Notification, NotificationActivation, NotificationError, NotificationPermission,
    NotificationReplies, NotificationScheduling, Notifications, PermissionFuture, PermissionState,
    PostFuture,
};
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{Clamped, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement, ImageData, NotificationOptions};

use crate::file_dialogs::describe;

/// The browser's notifications for the page.
pub(crate) struct BrowserNotifications(Rc<Showing>);

/// Where the host hears a click on a notification.
type ClickListener = Rc<dyn Fn(NotificationActivation)>;

/// The notifications the page shows, by the app's id, and where a click on one goes.
struct Showing {
    by_id: RefCell<HashMap<String, web_sys::Notification>>,
    clicked: RefCell<Option<ClickListener>>,
    /// Every notification's `click` and `close` handlers, which live as long as the page: a
    /// notification may be clicked or closed at any time after it shows.
    on_click: Function,
    on_close: Function,
}

impl BrowserNotifications {
    /// The browser's notifications, or `None` where the page has none: in a browser without them,
    /// or outside a secure context, where the browser leaves `Notification` out.
    pub(crate) fn new() -> Option<BrowserNotifications> {
        let window = web_sys::window()?;
        let api = Reflect::get(&window, &JsValue::from_str("Notification")).ok()?;
        if api.is_undefined() {
            return None;
        }
        Some(BrowserNotifications(Rc::new_cyclic(|showing| Showing {
            by_id: RefCell::default(),
            clicked: RefCell::default(),
            on_click: handler(showing.clone(), Showing::was_clicked),
            on_close: handler(showing.clone(), Showing::was_closed),
        })))
    }

    /// Hands each click on a notification to `clicked`. The host sets it in the task that made the
    /// client, before any click can come.
    pub(crate) fn hear_clicks(&self, clicked: impl Fn(NotificationActivation) + 'static) {
        *self.0.clicked.borrow_mut() = Some(Rc::new(clicked));
    }
}

impl Notifications for BrowserNotifications {
    /// Shows `notification` where the user lets the page notify, and answers `Denied` without
    /// asking where not, as the browser's own `Notification` shows nothing then.
    fn post(&self, notification: &Notification) -> PostFuture {
        let posted = match permission() {
            PermissionState::Granted => self.0.show(notification),
            PermissionState::Denied | PermissionState::NotAsked => Err(NotificationError::Denied),
        };
        Box::pin(std::future::ready(posted))
    }

    /// Closes the notification this page showed under `id`; one an earlier visit showed is out of
    /// the page's reach.
    fn remove(&self, id: &str) {
        let shown = self.0.by_id.borrow_mut().remove(id);
        if let Some(shown) = shown {
            shown.close();
        }
    }

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        Some(self)
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        None
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        None
    }
}

impl NotificationPermission for BrowserNotifications {
    fn state(&self) -> PermissionFuture {
        Box::pin(std::future::ready(permission()))
    }

    fn request(&self) -> PermissionFuture {
        match permission() {
            PermissionState::NotAsked => Box::pin(requested()),
            state => Box::pin(std::future::ready(state)),
        }
    }
}

impl Showing {
    /// Shows `notification` under its id, in place of the one showing under it.
    fn show(&self, notification: &Notification) -> Result<(), NotificationError> {
        let options = options_of(notification)?;
        let shown = web_sys::Notification::new_with_options(&notification.title, &options)
            .map_err(|error| NotificationError::Failed(describe(&error)))?;
        shown.set_onclick(Some(&self.on_click));
        shown.set_onclose(Some(&self.on_close));
        self.by_id
            .borrow_mut()
            .insert(notification.id.clone(), shown);
        Ok(())
    }

    fn was_clicked(&self, clicked: &web_sys::Notification) {
        let Some(id) = clicked.tag() else {
            return;
        };
        let Some(hear) = self.clicked.borrow().clone() else {
            return;
        };
        hear(NotificationActivation {
            id,
            action: None,
            reply: None,
        });
    }

    /// Forgets `closed`, unless a later notification under its id has taken its place.
    fn was_closed(&self, closed: &web_sys::Notification) {
        let Some(id) = closed.tag() else {
            return;
        };
        let mut by_id = self.by_id.borrow_mut();
        if by_id.get(&id) == Some(closed) {
            by_id.remove(&id);
        }
    }
}

/// A handler for an event of a notification, calling `hear` with it for as long as the page keeps
/// its notifications.
fn handler(showing: Weak<Showing>, hear: fn(&Showing, &web_sys::Notification)) -> Function {
    Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let notification = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Notification>().ok());
        if let (Some(showing), Some(notification)) = (showing.upgrade(), notification) {
            hear(&showing, &notification);
        }
    })
    .into_js_value()
    .unchecked_into()
}

/// The browser's options for `notification`: its id as the tag the browser replaces by, and no
/// actions, which only a service worker's notification has.
fn options_of(notification: &Notification) -> Result<NotificationOptions, NotificationError> {
    let options = NotificationOptions::new();
    options.set_tag(&notification.id);
    if let Some(body) = &notification.body {
        options.set_body(body);
    }
    if let Some(image) = &notification.image {
        options.set_icon(&png_data_url(image)?);
    }
    options.set_silent(Some(notification.silent));
    // The browser refuses to alert again for a silent notification, or one without a tag.
    options.set_renotify(!notification.silent && !notification.id.is_empty());
    Ok(options)
}

/// `image` as a PNG `data:` URL, encoded by a canvas of the browser's own.
fn png_data_url(image: &IconImage) -> Result<String, NotificationError> {
    drawn(image)
        .and_then(|canvas| canvas.to_data_url())
        .map_err(|error| NotificationError::Failed(describe(&error)))
}

/// A canvas the size of `image`, with its pixels.
fn drawn(image: &IconImage) -> Result<HtmlCanvasElement, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("the page has no document"))?;
    let canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;
    canvas.set_width(image.width);
    canvas.set_height(image.height);
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("the canvas has no 2D context"))?
        .dyn_into()?;
    let pixels = ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(&image.rgba),
        image.width,
        image.height,
    )?;
    context.put_image_data(&pixels, 0.0, 0.0)?;
    Ok(canvas)
}

/// The page's permission, as the browser keeps it for the page's origin.
fn permission() -> PermissionState {
    state_of(web_sys::Notification::permission())
}

/// Asks the user with the browser's question, from now; answers the permission after.
fn requested() -> impl Future<Output = PermissionState> {
    let asked = web_sys::Notification::request_permission()
        .ok()
        .and_then(|answer| JsValue::from(answer).dyn_into::<Promise>().ok());
    async move {
        let Some(asked) = asked else {
            return permission();
        };
        JsFuture::from(asked)
            .await
            .ok()
            .and_then(|answer| web_sys::NotificationPermission::from_js_value(&answer))
            .map_or_else(permission, state_of)
    }
}

fn state_of(permission: web_sys::NotificationPermission) -> PermissionState {
    match permission {
        web_sys::NotificationPermission::Granted => PermissionState::Granted,
        web_sys::NotificationPermission::Denied => PermissionState::Denied,
        _ => PermissionState::NotAsked,
    }
}
