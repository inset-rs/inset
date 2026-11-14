//! The thread the cameras are opened, configured, closed and opened again on, one job at a time,
//! as Apple's documentation has a capture session started on a serial queue of its own: opening a
//! camera and configuring its session block on the camera service, which the app's thread does
//! not wait for, and what Android tells of a camera, on threads of its own, is acted on here in the
//! order it came, so a camera's state changes one step at a time.

use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock, PoisonError};

type Job = Box<dyn FnOnce() + Send>;

/// Runs `job` on the cameras' thread, after every job queued before it; dropped where the thread
/// could not start.
pub(super) fn queue(job: impl FnOnce() + Send + 'static) {
    static QUEUE: OnceLock<Option<Mutex<Sender<Job>>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (queue, jobs) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("waft cameras".to_owned())
            .spawn(move || jobs.into_iter().for_each(|job| job()))
            .ok()?;
        Some(Mutex::new(queue))
    });
    match queue {
        Some(queue) => {
            let _ = queue
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(Box::new(job));
        }
        None => {
            crate::log::warn_to_log!("waft: no thread for the cameras; a camera job is dropped")
        }
    }
}
