//! One instance of an app: a later launch hands its command line to the instance already running,
//! and quits. An app's own pattern rather than the platform's, as Electron's
//! `requestSingleInstanceLock` and Tauri's single-instance plugin are, and opt-in.
//!
//! A later launch's arguments arrive as the strings they are, unread, with the folder it was
//! started in: which of them are files is the app's to say, as it says for its own launch's.
//!
//! Each system has its own way: Linux's session bus, where the first instance owns the app's id as
//! its name and a later launch calls it; Windows' named mutex, which tells a later launch it is
//! second, and a named pipe it writes its arguments to; macOS keeps one instance itself, handing a
//! later open to the running app as an open request, so every launch there is the first, as on the
//! phones and the web.
//!
//! An app reads its own launch's command line, hears the later launches' through
//! [`FirstInstance::launches`], and hands the files it decides they name to
//! `App::open_requested`, where the platform's own open requests also arrive
//! (`App::set_open_requested_handler`): those bring the files the system itself hands over, and on
//! macOS every later open. All of them reach the same code:
//!
//! ```ignore
//! use std::ffi::OsString;
//! use std::path::Path;
//! use std::rc::Rc;
//!
//! use inset::ui::{OpenRequest, PickedFile};
//! use inset::{App, run_app};
//! use inset_single_instance::{SingleInstance, single_instance};
//!
//! #[inset::main]
//! fn main(app: &mut App) {
//!     let first = match single_instance("com.example.notes") {
//!         Ok(SingleInstance::First(first)) => Some(first),
//!         // The first instance has this launch's command line.
//!         Ok(SingleInstance::Second) => std::process::exit(0),
//!         Err(_) => None,
//!     };
//!     app.set_open_requested_handler(Some(Rc::new(open)));
//!     let current_dir = std::env::current_dir().unwrap_or_default();
//!     let launch = files_named(std::env::args_os().skip(1), &current_dir);
//!     if !launch.is_empty() {
//!         app.open_requested(launch);
//!     }
//!     if let Some(first) = first {
//!         let mut launches = first.launches();
//!         app.spawn(async move |cx| {
//!             loop {
//!                 let launch = launches.next_launch().await;
//!                 let request = files_named(launch.arguments, &launch.current_dir);
//!                 cx.update(|app| app.open_requested(request));
//!             }
//!         });
//!     }
//!     run_app(app, home());
//! }
//!
//! /// The files this app's command line names: each of its arguments, in the folder it was
//! /// started in.
//! fn files_named(arguments: impl IntoIterator<Item = OsString>, current_dir: &Path) -> OpenRequest {
//!     let files = arguments.into_iter().map(|argument| PickedFile::Path(current_dir.join(argument)));
//!     OpenRequest { files: files.collect(), links: Vec::new() }
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

#[cfg(target_os = "linux")]
mod dbus;
#[cfg(target_os = "windows")]
mod windows;

/// Whether this launch is the app's only instance.
pub enum SingleInstance {
    /// No other instance runs: this one is the app, and hears the later launches.
    First(FirstInstance),
    /// Another instance runs, and has been handed this launch's command line: this one should
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

    /// The later launches, as they come, holding the claim as long as it lives.
    pub fn launches(self) -> LaterLaunches {
        LaterLaunches { first: self }
    }
}

/// The launches after the first instance's, each passed on to it.
pub struct LaterLaunches {
    first: FirstInstance,
}

impl LaterLaunches {
    /// The next launch passed on; it waits until one comes.
    pub fn next_launch(&mut self) -> impl Future<Output = Launch> + '_ {
        NextLaunch {
            arrivals: &self.first.arrivals,
        }
    }
}

/// A later launch: its arguments, without the program's name, as the strings the system gave it,
/// and the folder it was started in, which relative paths are in. A launch with no arguments asks
/// the app to show itself again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub arguments: Vec<OsString>,
    pub current_dir: PathBuf,
}

/// The launches waiting, and the task waiting for one.
#[derive(Default)]
struct Arrivals(Mutex<(VecDeque<Launch>, Option<Waker>)>);

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
            arrivals.0.push_back(Launch {
                arguments,
                current_dir,
            });
            arrivals.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn locked(&self) -> MutexGuard<'_, (VecDeque<Launch>, Option<Waker>)> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct NextLaunch<'a> {
    arrivals: &'a Arrivals,
}

impl Future for NextLaunch<'_> {
    type Output = Launch;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Launch> {
        let mut arrivals = self.arrivals.locked();
        match arrivals.0.pop_front() {
            Some(launch) => Poll::Ready(launch),
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

    pub(crate) fn now<T>(future: impl Future<Output = T>) -> Option<T> {
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
    fn a_later_launchs_arguments_arrive_unread() {
        let arrivals = Arc::new(Arrivals::default());
        let first = FirstInstance::new(Arc::clone(&arrivals), Box::new(()));
        let mut launches = first.launches();
        assert!(now(launches.next_launch()).is_none(), "nothing yet");
        let arguments = ["phone", "--verbose", "notes.txt"]
            .map(OsString::from)
            .to_vec();
        arrivals.arrive(arguments.clone(), PathBuf::from("/home/me"));
        assert_eq!(
            now(launches.next_launch()),
            Some(Launch {
                arguments,
                current_dir: PathBuf::from("/home/me"),
            })
        );
    }
}
