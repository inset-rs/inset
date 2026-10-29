//! One instance of an app: a later launch hands the files and links it was started with to the
//! instance already running, and quits. An app's own pattern rather than the platform's, as
//! Electron's `requestSingleInstanceLock` and Tauri's single-instance plugin are, and opt-in.
//!
//! Each system has its own way: Linux's D-Bus activation, where the first instance owns the app's
//! name on the session bus and a later launch calls its `org.freedesktop.Application.Open`;
//! Windows' named mutex, which tells a later launch it is second, and a named pipe it writes its
//! arguments to; macOS keeps one instance itself, handing a later open to the running app as an
//! open request, so every launch there is the first, as on the phones and the web.
//!
//! An app combines it with the platform's own open requests (`App::set_open_requested_handler`):
//! those bring the first launch's own files and links, and on macOS every later open; the stream
//! of [`FirstInstance::opened`] brings what later launches pass on. Both reach the same code:
//!
//! ```ignore
//! use std::rc::Rc;
//!
//! use inset::ui::OpenRequest;
//! use inset::{App, run_app};
//! use inset_single_instance::{SingleInstance, single_instance};
//!
//! #[inset::main]
//! fn main(app: &mut App) {
//!     let first = match single_instance("com.example.notes") {
//!         Ok(SingleInstance::First(first)) => Some(first),
//!         // The first instance has this launch's files.
//!         Ok(SingleInstance::Second) => std::process::exit(0),
//!         Err(_) => None,
//!     };
//!     app.set_open_requested_handler(Some(Rc::new(open)));
//!     if let Some(first) = first {
//!         let mut opened = first.opened();
//!         app.spawn(async move |cx| {
//!             loop {
//!                 let request = opened.next_request().await;
//!                 cx.update(|app| open(app, request));
//!             }
//!         });
//!     }
//!     run_app(app, home());
//! }
//!
//! /// Shows the request's files and links; an empty request asks the app to show itself.
//! fn open(app: &mut App, request: OpenRequest) {
//!     // ...
//! }
//! ```
//!
//! The invariant: the first instance keeps its claim as long as its stream lives, so a launch
//! while it runs is always second.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use inset_embedder::OpenRequest;

#[cfg(target_os = "linux")]
mod dbus;
#[cfg(target_os = "windows")]
mod windows;

/// Whether this launch is the app's only instance.
pub enum SingleInstance {
    /// No other instance runs: this one is the app, and hears what later launches ask to open.
    First(FirstInstance),
    /// Another instance runs, and has been handed this launch's files and links: this one should
    /// quit.
    Second,
}

/// Claims to be the one instance of the app `id`, a reverse-DNS name such as
/// `com.example.notes`, or, where another instance holds the claim, hands it this launch's
/// arguments. An error leaves the app to run as though alone.
pub fn single_instance(id: &str) -> io::Result<SingleInstance> {
    #[cfg(target_os = "linux")]
    return dbus::claim(id);
    #[cfg(target_os = "windows")]
    return windows::claim(id);
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = id;
        Ok(SingleInstance::First(FirstInstance::new(
            Arc::default(),
            Box::new(()),
        )))
    }
}

/// The app's only instance, holding its claim.
pub struct FirstInstance {
    arrivals: Arc<Arrivals>,
    /// What holds the claim: the mutex and the pipe's thread, or the name on the bus.
    _claim: Box<dyn Send>,
}

impl FirstInstance {
    /// The first instance, hearing later launches through `arrivals` while `claim` holds.
    fn new(arrivals: Arc<Arrivals>, claim: Box<dyn Send>) -> FirstInstance {
        FirstInstance {
            arrivals,
            _claim: claim,
        }
    }

    /// What later launches ask to open, as they ask, holding the claim as long as it lives. A
    /// launch with nothing to open asks for an empty request: the app shows itself again.
    pub fn opened(self) -> Opened {
        Opened { first: self }
    }
}

/// The requests later launches pass on to the first instance.
pub struct Opened {
    first: FirstInstance,
}

impl Opened {
    /// The next request a later launch passes on; it waits until one comes.
    pub fn next_request(&mut self) -> impl Future<Output = OpenRequest> + '_ {
        NextArrival {
            arrivals: &self.first.arrivals,
        }
    }
}

/// A later launch's arguments, and the folder it was started in, which relative paths are in.
struct Arrival {
    arguments: Vec<OsString>,
    current_dir: PathBuf,
}

/// The arrivals waiting, and the task waiting for one.
#[derive(Default)]
struct Arrivals(Mutex<(VecDeque<Arrival>, Option<Waker>)>);

impl Arrivals {
    /// A later launch passed `arguments` on, from `current_dir`.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "windows")),
        allow(
            dead_code,
            reason = "no later launch reaches this system's first instance"
        )
    )]
    fn arrive(&self, arguments: Vec<OsString>, current_dir: PathBuf) {
        let waker = {
            let mut arrivals = self.locked();
            arrivals.0.push_back(Arrival {
                arguments,
                current_dir,
            });
            arrivals.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn locked(&self) -> MutexGuard<'_, (VecDeque<Arrival>, Option<Waker>)> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct NextArrival<'a> {
    arrivals: &'a Arrivals,
}

impl Future for NextArrival<'_> {
    type Output = OpenRequest;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<OpenRequest> {
        let mut arrivals = self.arrivals.locked();
        match arrivals.0.pop_front() {
            Some(arrival) => Poll::Ready(OpenRequest::from_arguments(
                arrival.arguments,
                &arrival.current_dir,
            )),
            None => {
                arrivals.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// This launch's arguments, and the folder it was started in.
#[cfg_attr(
    not(any(target_os = "linux", target_os = "windows")),
    allow(
        dead_code,
        reason = "no later launch reaches this system's first instance"
    )
)]
fn this_launch() -> (Vec<OsString>, PathBuf) {
    (
        std::env::args_os().skip(1).collect(),
        std::env::current_dir().unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> Option<T> {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => Some(output),
            Poll::Pending => None,
        }
    }

    #[test]
    fn a_later_launchs_arguments_arrive_as_a_request() {
        let arrivals = Arc::new(Arrivals::default());
        let first = FirstInstance::new(Arc::clone(&arrivals), Box::new(()));
        let mut opened = first.opened();
        assert!(now(opened.next_request()).is_none(), "nothing yet");
        arrivals.arrive(
            vec![
                OsString::from("notes.txt"),
                OsString::from("https://example.org"),
            ],
            PathBuf::from("/home/me"),
        );
        let request = now(opened.next_request()).expect("the launch's request");
        assert_eq!(request.links, ["https://example.org"]);
        assert_eq!(request.files.len(), 1);
    }
}
