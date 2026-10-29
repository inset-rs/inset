//! Notifications: the desktop's notification server, `org.freedesktop.Notifications` on the session
//! bus, which GNOME, KDE and the other desktops answer. The server numbers each notification; the
//! app's id is mapped to the number, so posting the id again passes it as `replaces_id`, and
//! removing it closes it. An action is one of the server's action keys, `default` standing for the
//! notification itself; a reply, where the server advertises KDE's `inline-reply`, is the action
//! keyed `inline-reply`, one to a notification. Clicks come as the server's `ActionInvoked` and
//! `NotificationReplied`, heard on a thread of their own while the app runs: the server keeps no
//! click for an app that is not running. No permission is asked, and nothing is scheduled.
//!
//! One connection both calls the server and hears its signals, as a server may send its signals
//! to the connection that posted alone, as KDE's does. The calls are made one at a time on a
//! thread of their own, in the order the app made them, so a post reads the number the post before
//! it was given, and a remove closes a notification whose post was still on its way. The mapping
//! is shared with the thread that hears the signals; a number is forgotten when the app closes its
//! notification or the server says it closed, and a signal about a number the app does not hold is
//! another app's.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use inset_embedder::{
    IconImage, Notification, NotificationActivation, NotificationError, NotificationPermission,
    NotificationReplies, NotificationScheduling, Notifications, PostFuture,
};
use zbus::blocking::{Connection, Proxy};
use zbus::message::Message;
use zbus::zvariant::Value;

use super::Linux;
use super::session_bus;
use crate::answer::{Answer, answer};
use crate::notifications::NotificationActivations;
use crate::os::SystemNotifications;

const SERVER: &str = "org.freedesktop.Notifications";
const SERVER_PATH: &str = "/org/freedesktop/Notifications";
/// The action key of the notification itself.
const DEFAULT_ACTION: &str = "default";
/// KDE's action key of a reply field, and the capability a server advertises it by.
const INLINE_REPLY: &str = "inline-reply";
/// The capability of a server that reads a body's markup, which a plain body is escaped for.
const BODY_MARKUP: &str = "body-markup";

impl SystemNotifications for Linux {
    fn notifications(activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        let server = Server::connect(activations)?;
        Some(Rc::new(server))
    }
}

/// The desktop's notification server, as this app reaches it.
struct Server {
    capabilities: Vec<String>,
    /// The app's desktop file, where the system says which it is.
    desktop_entry: Option<String>,
    /// Where the calls go, to be made in order (`Caller`).
    calls: Sender<Call>,
}

/// A call the app makes of the server.
enum Call {
    Notify(Notify, Answer<Result<(), NotificationError>>),
    Close(String),
}

/// A notification as the server is asked to show it, all but the number it replaces, which is
/// read when the call is made.
struct Notify {
    shown: Shown,
    title: String,
    body: String,
    actions: Vec<String>,
    hints: HashMap<&'static str, Value<'static>>,
}

/// The notifications the server shows for the app, by the app's id and by the server's number.
#[derive(Default)]
struct Posted {
    numbers: HashMap<String, u32>,
    shown: HashMap<u32, Shown>,
}

/// A notification the server shows: the app's id for it, and its reply action's.
struct Shown {
    id: String,
    reply_action: Option<String>,
}

impl Posted {
    fn record(&mut self, number: u32, shown: Shown) {
        if let Some(earlier) = self.numbers.insert(shown.id.clone(), number)
            && earlier != number
        {
            self.shown.remove(&earlier);
        }
        self.shown.insert(number, shown);
    }

    fn forget(&mut self, number: u32) {
        if let Some(shown) = self.shown.remove(&number) {
            self.numbers.remove(&shown.id);
        }
    }

    /// Forgets the notification posted under `id`, answering its number.
    fn forget_id(&mut self, id: &str) -> Option<u32> {
        let number = self.numbers.remove(id)?;
        self.shown.remove(&number);
        Some(number)
    }

    /// What a click on the action `key` of the notification `number` tells the app.
    fn activation(&self, number: u32, key: &str) -> Option<NotificationActivation> {
        let shown = self.shown.get(&number)?;
        Some(NotificationActivation {
            id: shown.id.clone(),
            action: (key != DEFAULT_ACTION).then(|| key.to_owned()),
            reply: None,
        })
    }

    /// What a reply typed into the notification `number` tells the app.
    fn reply(&self, number: u32, text: String) -> Option<NotificationActivation> {
        let shown = self.shown.get(&number)?;
        Some(NotificationActivation {
            id: shown.id.clone(),
            action: shown.reply_action.clone(),
            reply: Some(text),
        })
    }
}

impl Server {
    /// The server, or `None` where the session bus has none, running or one it starts when asked,
    /// which the first call, for its capabilities, tells by failing. Called on the main thread at
    /// startup: each call waits no longer than the bus's patience.
    fn connect(activations: NotificationActivations) -> Option<Server> {
        let connection = session_bus::connect()?;
        let capabilities = capabilities(&connection)?;
        let posted = Arc::new(Mutex::new(Posted::default()));
        hear_clicks(&connection, Arc::clone(&posted), activations)?;
        let calls = Caller::start(connection, posted)?;
        Some(Server {
            capabilities,
            desktop_entry: desktop_entry(),
            calls,
        })
    }

    fn can(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|offered| offered == capability)
    }

    /// `notification` as the server is asked to show it.
    fn notify(&self, notification: &Notification) -> Notify {
        let (actions, reply) = self.actions(notification);
        let body = notification.body.clone().unwrap_or_default();
        Notify {
            hints: self.hints(
                notification,
                reply.as_ref().map(|(_, placeholder)| placeholder),
            ),
            shown: Shown {
                id: notification.id.clone(),
                reply_action: reply.map(|(id, _)| id),
            },
            title: notification.title.clone(),
            body: if self.can(BODY_MARKUP) {
                escaped(&body)
            } else {
                body
            },
            actions,
        }
    }

    /// The server's actions for `notification`: the notification itself first, then each of its
    /// own, and its first reply as KDE's inline reply where the server takes one; and the reply's
    /// action id and placeholder.
    fn actions(&self, notification: &Notification) -> (Vec<String>, Option<(String, String)>) {
        let mut keys = vec![DEFAULT_ACTION.to_owned(), String::new()];
        let mut reply = None;
        for action in &notification.actions {
            match action.reply_placeholder() {
                Some(placeholder) if reply.is_none() && self.can(INLINE_REPLY) => {
                    keys.extend([INLINE_REPLY.to_owned(), action.label.clone()]);
                    reply = Some((action.id.clone(), placeholder.to_owned()));
                }
                _ => keys.extend([action.id.clone(), action.label.clone()]),
            }
        }
        (keys, reply)
    }

    /// The hints for `notification`, with its reply field's placeholder.
    fn hints(
        &self,
        notification: &Notification,
        reply_placeholder: Option<&String>,
    ) -> HashMap<&'static str, Value<'static>> {
        let mut hints = HashMap::new();
        if notification.silent {
            hints.insert("suppress-sound", Value::from(true));
        }
        if let Some(image) = &notification.image {
            hints.insert("image-data", image_data(image));
        }
        if let Some(placeholder) = reply_placeholder {
            hints.insert(
                "x-kde-reply-placeholder-text",
                Value::from(placeholder.clone()),
            );
        }
        if let Some(entry) = &self.desktop_entry {
            hints.insert("desktop-entry", Value::from(entry.clone()));
        }
        hints
    }
}

impl Notifications for Server {
    fn post(&self, notification: &Notification) -> PostFuture {
        let (answer, answered) = answer(Err(NotificationError::Failed(
            "the notification server ended without an answer".to_owned(),
        )));
        // A call that cannot be sent is dropped with its answer, which answers the failure.
        let _ = self
            .calls
            .send(Call::Notify(self.notify(notification), answer));
        Box::pin(answered)
    }

    fn remove(&self, id: &str) {
        let _ = self.calls.send(Call::Close(id.to_owned()));
    }

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        None
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        self.can(INLINE_REPLY)
            .then_some(self as &dyn NotificationReplies)
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        None
    }
}

impl NotificationReplies for Server {}

/// The server's capabilities, or `None` where it cannot be reached.
fn capabilities(connection: &Connection) -> Option<Vec<String>> {
    connection
        .call_method(
            Some(SERVER),
            SERVER_PATH,
            Some(SERVER),
            "GetCapabilities",
            &(),
        )
        .ok()?
        .body()
        .deserialize()
        .ok()
}

/// What makes the app's calls of the server, one at a time, on a thread of its own.
struct Caller {
    connection: Connection,
    posted: Arc<Mutex<Posted>>,
    /// The app's name, as the server shows it.
    app_name: String,
}

impl Caller {
    /// Starts the thread that makes the calls sent to it, in the order they are sent, and answers
    /// where to send them.
    fn start(connection: Connection, posted: Arc<Mutex<Posted>>) -> Option<Sender<Call>> {
        let (calls, made) = mpsc::channel();
        let caller = Caller {
            connection,
            posted,
            app_name: app_name(),
        };
        std::thread::Builder::new()
            .name("inset-notifications".to_owned())
            .spawn(move || caller.make_calls(made))
            .ok()?;
        Some(calls)
    }

    fn make_calls(&self, calls: Receiver<Call>) {
        for call in calls {
            match call {
                Call::Notify(notify, answer) => answer.give(
                    self.notify(notify)
                        .map_err(|error| NotificationError::Failed(error.to_string())),
                ),
                Call::Close(id) => self.close(&id),
            }
        }
    }

    /// Shows `notify` in place of the notification shown under its id, and records its number.
    fn notify(&self, notify: Notify) -> zbus::Result<()> {
        let replaces = locked(&self.posted)
            .numbers
            .get(&notify.shown.id)
            .copied()
            .unwrap_or(0);
        let number: u32 = self
            .connection
            .call_method(
                Some(SERVER),
                SERVER_PATH,
                Some(SERVER),
                "Notify",
                &(
                    self.app_name.as_str(),
                    replaces,
                    "",
                    notify.title.as_str(),
                    notify.body.as_str(),
                    notify.actions,
                    notify.hints,
                    -1i32,
                ),
            )?
            .body()
            .deserialize()?;
        locked(&self.posted).record(number, notify.shown);
        Ok(())
    }

    /// Closes the notification shown under `id`, if there is one; a later click on it is not the
    /// app's.
    fn close(&self, id: &str) {
        let Some(number) = locked(&self.posted).forget_id(id) else {
            return;
        };
        let _ = self.connection.call_method(
            Some(SERVER),
            SERVER_PATH,
            Some(SERVER),
            "CloseNotification",
            &(number,),
        );
    }
}

/// Hears the server's signals for the app's notifications on a thread of its own, sending each
/// click and reply to `activations`.
fn hear_clicks(
    connection: &Connection,
    posted: Arc<Mutex<Posted>>,
    activations: NotificationActivations,
) -> Option<()> {
    let server = Proxy::new(connection, SERVER, SERVER_PATH, SERVER).ok()?;
    let signals = server.receive_all_signals().ok()?;
    std::thread::Builder::new()
        .name("inset-notification-clicks".to_owned())
        .spawn(move || {
            for signal in signals {
                if let Some(activation) = heard(&signal, &posted) {
                    activations.send(activation);
                }
            }
        })
        .ok()?;
    Some(())
}

/// What a signal of the server tells the app, if it is about one of the app's notifications.
fn heard(signal: &Message, posted: &Mutex<Posted>) -> Option<NotificationActivation> {
    let header = signal.header();
    let body = signal.body();
    match header.member()?.as_str() {
        "ActionInvoked" => {
            let (number, key): (u32, String) = body.deserialize().ok()?;
            locked(posted).activation(number, &key)
        }
        "NotificationReplied" => {
            let (number, text): (u32, String) = body.deserialize().ok()?;
            locked(posted).reply(number, text)
        }
        "NotificationClosed" => {
            let (number, _reason): (u32, u32) = body.deserialize().ok()?;
            locked(posted).forget(number);
            None
        }
        _ => None,
    }
}

/// The server's `image-data` hint, `(iiibiiay)`: width, height, the bytes of a row, alpha, bits
/// per sample, channels, and the pixels.
fn image_data(image: &IconImage) -> Value<'static> {
    let width = image.width as i32;
    Value::from((
        width,
        image.height as i32,
        width * 4,
        true,
        8i32,
        4i32,
        image.rgba.to_vec(),
    ))
}

/// `text` as a body the server reads markup in shows it.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The app's name, as the server shows it: its executable's.
fn app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

/// The app's desktop file id, where the system says which it is: a Flatpak's app id, which names
/// its desktop file, or the file GLib's launcher started this very process from.
fn desktop_entry() -> Option<String> {
    let variable = |name| std::env::var(name).ok();
    desktop_entry_from(
        variable("FLATPAK_ID"),
        variable("GIO_LAUNCHED_DESKTOP_FILE"),
        variable("GIO_LAUNCHED_DESKTOP_FILE_PID"),
        std::process::id(),
    )
}

/// The desktop file id the environment names: the Flatpak's app id, or the launched desktop file's
/// name without `.desktop` where it was launched for `pid`, and not for a process that started
/// this one.
fn desktop_entry_from(
    flatpak_id: Option<String>,
    launched_file: Option<String>,
    launched_pid: Option<String>,
    pid: u32,
) -> Option<String> {
    if let Some(id) = flatpak_id.filter(|id| !id.is_empty()) {
        return Some(id);
    }
    if launched_pid?.parse::<u32>().ok()? != pid {
        return None;
    }
    let launched_file = launched_file?;
    let name = std::path::Path::new(&launched_file).file_name()?.to_str()?;
    name.strip_suffix(".desktop").map(str::to_owned)
}

fn locked(posted: &Mutex<Posted>) -> MutexGuard<'_, Posted> {
    posted.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_names_its_notification_until_it_closes() {
        let mut posted = Posted::default();
        posted.record(
            7,
            Shown {
                id: "message".to_owned(),
                reply_action: Some("answer".to_owned()),
            },
        );
        assert_eq!(posted.numbers.get("message"), Some(&7));
        assert_eq!(
            posted.activation(7, DEFAULT_ACTION),
            Some(NotificationActivation {
                id: "message".to_owned(),
                action: None,
                reply: None,
            })
        );
        assert_eq!(
            posted.reply(7, "on my way".to_owned()),
            Some(NotificationActivation {
                id: "message".to_owned(),
                action: Some("answer".to_owned()),
                reply: Some("on my way".to_owned()),
            })
        );
        posted.forget(7);
        assert_eq!(posted.activation(7, "later"), None);
        assert!(posted.numbers.is_empty());
    }

    #[test]
    fn a_closed_id_forgets_its_number() {
        let mut posted = Posted::default();
        let shown = |id: &str| Shown {
            id: id.to_owned(),
            reply_action: None,
        };
        posted.record(7, shown("message"));
        posted.record(9, shown("message"));
        assert_eq!(posted.activation(7, DEFAULT_ACTION), None, "replaced");
        assert_eq!(posted.forget_id("message"), Some(9));
        assert_eq!(posted.activation(9, DEFAULT_ACTION), None);
        assert_eq!(posted.forget_id("message"), None);
    }

    #[test]
    fn the_image_hint_has_the_specifications_signature() {
        let image = IconImage {
            width: 2,
            height: 1,
            rgba: Arc::from([0u8; 8].as_slice()),
            template: false,
        };
        let hint = image_data(&image);
        assert_eq!(hint.value_signature().to_string(), "(iiibiiay)");
    }

    #[test]
    fn the_desktop_entry_is_the_one_the_environment_names_for_this_process() {
        let some = |text: &str| Some(text.to_owned());
        assert_eq!(
            desktop_entry_from(some("org.example.Notes"), None, None, 7).as_deref(),
            Some("org.example.Notes")
        );
        let launched = some("/usr/share/applications/org.example.Notes.desktop");
        assert_eq!(
            desktop_entry_from(None, launched.clone(), some("7"), 7).as_deref(),
            Some("org.example.Notes")
        );
        assert_eq!(
            desktop_entry_from(None, launched, some("6"), 7),
            None,
            "the launcher started this process's parent"
        );
        assert_eq!(desktop_entry_from(None, None, None, 7), None);
    }

    /// A notification server of the test's own, which numbers what it is asked to show and keeps
    /// each call.
    struct FakeServer {
        notified: Arc<Mutex<Vec<Notified>>>,
        closed: Arc<Mutex<Vec<u32>>>,
    }

    /// One `Notify` call, as the fake server heard it.
    #[derive(Clone, Debug)]
    struct Notified {
        replaces: u32,
        body: String,
        actions: Vec<String>,
        placeholder: Option<String>,
        image_signature: Option<String>,
        suppress_sound: Option<bool>,
    }

    #[zbus::interface(name = "org.freedesktop.Notifications")]
    impl FakeServer {
        fn get_capabilities(&self) -> Vec<String> {
            ["actions", "body", BODY_MARKUP, INLINE_REPLY]
                .map(str::to_owned)
                .to_vec()
        }

        #[expect(clippy::too_many_arguments, reason = "the specification's signature")]
        fn notify(
            &self,
            _app_name: String,
            replaces_id: u32,
            _app_icon: String,
            _summary: String,
            body: String,
            actions: Vec<String>,
            hints: HashMap<String, zbus::zvariant::OwnedValue>,
            _expire_timeout: i32,
        ) -> u32 {
            let placeholder = hints
                .get("x-kde-reply-placeholder-text")
                .and_then(|value| String::try_from(value.clone()).ok());
            let image_signature = hints
                .get("image-data")
                .map(|value| value.value_signature().to_string());
            let suppress_sound = hints
                .get("suppress-sound")
                .and_then(|value| bool::try_from(value.clone()).ok());
            let mut notified = self.notified.lock().unwrap();
            notified.push(Notified {
                replaces: replaces_id,
                body,
                actions,
                placeholder,
                image_signature,
                suppress_sound,
            });
            if replaces_id == 0 {
                40 + notified.len() as u32
            } else {
                replaces_id
            }
        }

        fn close_notification(&self, id: u32) {
            self.closed.lock().unwrap().push(id);
        }
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        struct Unpark(std::thread::Thread);
        impl std::task::Wake for Unpark {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let mut future = std::pin::pin!(future);
        let waker = std::task::Waker::from(Arc::new(Unpark(std::thread::current())));
        let mut context = std::task::Context::from_waker(&waker);
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
            std::thread::park_timeout(std::time::Duration::from_millis(100));
        }
    }

    #[test]
    #[ignore = "needs a session bus of its own: dbus-run-session -- cargo test -- --ignored"]
    fn a_notification_is_posted_replaced_clicked_and_replied_to_through_the_server() {
        let notified = Arc::new(Mutex::new(Vec::new()));
        let closed = Arc::new(Mutex::new(Vec::new()));
        let fake = zbus::blocking::connection::Builder::session()
            .and_then(|builder| builder.name(SERVER))
            .and_then(|builder| {
                builder.serve_at(
                    SERVER_PATH,
                    FakeServer {
                        notified: Arc::clone(&notified),
                        closed: Arc::clone(&closed),
                    },
                )
            })
            .and_then(|builder| builder.build())
            .expect("the fake server on the session bus");
        let (clicked, clicks) = std::sync::mpsc::channel();
        let clicked = Mutex::new(clicked);
        let server = Server::connect(NotificationActivations::to(move |activation| {
            let _ = clicked.lock().unwrap().send(activation);
        }))
        .expect("the server");
        let mut message = Notification::new("message", "New message");
        message.body = Some("a < b".to_owned());
        message.silent = true;
        message.image = Some(IconImage {
            width: 1,
            height: 1,
            rgba: Arc::from([255u8, 0, 0, 255].as_slice()),
            template: false,
        });
        message.actions = vec![
            inset_embedder::NotificationAction::button("later", "Later"),
            server
                .replies()
                .expect("the server takes replies")
                .reply_action("answer", "Reply", "Your answer"),
        ];
        let first = server.post(&message);
        let second = server.post(&message);
        block_on(first).expect("posted");
        block_on(second).expect("posted again, before the first post was answered");
        let calls = notified.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].replaces, 0);
        assert_eq!(calls[1].replaces, 41, "the second post replaces the first");
        assert_eq!(calls[0].body, "a &lt; b");
        assert_eq!(
            calls[0].actions,
            ["default", "", "later", "Later", INLINE_REPLY, "Reply"]
        );
        assert_eq!(calls[0].placeholder.as_deref(), Some("Your answer"));
        assert_eq!(calls[0].image_signature.as_deref(), Some("(iiibiiay)"));
        assert_eq!(calls[0].suppress_sound, Some(true));

        let emit = |member: &str, body: &(u32, &str)| {
            fake.emit_signal(None::<&str>, SERVER_PATH, SERVER, member, body)
                .expect("a signal");
        };
        let next = || {
            clicks
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("a click")
        };
        emit("ActionInvoked", &(41, "later"));
        assert_eq!(next().action.as_deref(), Some("later"));
        emit("NotificationReplied", &(41, "on my way"));
        let reply = next();
        assert_eq!(
            (reply.action.as_deref(), reply.reply.as_deref()),
            (Some("answer"), Some("on my way"))
        );
        emit("ActionInvoked", &(41, DEFAULT_ACTION));
        assert_eq!(next().action, None);
        emit("ActionInvoked", &(99, "later"));
        assert!(
            clicks
                .recv_timeout(std::time::Duration::from_millis(500))
                .is_err(),
            "another app's notification is not the app's"
        );

        let reminder = Notification::new("reminder", "Stand up");
        let posted = server.post(&reminder);
        server.remove("reminder");
        block_on(posted).expect("posted");
        let after = server.post(&Notification::new("after", "After"));
        block_on(after).expect("posted after the remove");
        assert_eq!(
            *closed.lock().unwrap(),
            [43],
            "a remove sent while its post was on its way closes what the post showed"
        );
    }
}
