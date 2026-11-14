//! The threads the image codecs work on, off the app's thread: as many as the device has cores,
//! started when first needed and kept for the process, taking work in the order it was asked, as
//! a concurrent queue of Apple's does for Image I/O. A feed of pictures decodes a few at a time
//! rather than on a thread each.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread;

type Work = Box<dyn FnOnce() + Send>;

/// Runs `work` on one of the threads.
pub(super) fn run(work: impl FnOnce() + Send + 'static) {
    static QUEUE: OnceLock<Mutex<Sender<Work>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| Mutex::new(started()));
    let queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
    // The threads live for the process, so the queue always has a reader.
    let _ = queue.send(Box::new(work));
}

/// The queue the threads take their work from, with the threads started.
fn started() -> Sender<Work> {
    let (queue, work) = channel::<Work>();
    let work = Arc::new(Mutex::new(work));
    let count = thread::available_parallelism().map_or(2, usize::from);
    for index in 0..count {
        let work = Arc::clone(&work);
        thread::Builder::new()
            .name(format!("waft-image-codecs-{index}"))
            .spawn(move || serve(&work))
            .expect("a thread for the image codecs");
    }
    queue
}

fn serve(work: &Mutex<Receiver<Work>>) {
    loop {
        let next = work.lock().unwrap_or_else(PoisonError::into_inner).recv();
        match next {
            Ok(next) => next(),
            Err(_) => return,
        }
    }
}
