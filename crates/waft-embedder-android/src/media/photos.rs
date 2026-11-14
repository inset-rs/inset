//! A running camera's photos: each asked with a still capture into the camera's JPEG stream,
//! which Android encodes and, with the orientation the capture asked for, writes the EXIF of; the
//! photos the stream delivers answer the captures in the order they were asked, and a capture
//! Android says failed answers its failure.
//!
//! The invariant: each photo asked is answered once, a failure where its capture failed, the camera
//! stopped, or the stream went without delivering it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use waft_embedder::{MediaError, Photo};

use super::camera_images::CameraImage;
use crate::answer::{Answer, Answered, answer};

/// The photos asked of a camera and not yet answered, in the order asked.
#[derive(Default)]
pub(super) struct Photos {
    pending: Mutex<VecDeque<Asked>>,
    next: AtomicU64,
}

/// A photo asked, by its number, and the answer it waits for.
type Asked = (u64, Answer<Result<Photo, MediaError>>);

/// What a photo asked answers.
pub(super) type PhotoAnswered = Answered<Result<Photo, MediaError>>;

impl Photos {
    /// A photo asked, by its number, and its answer.
    pub(super) fn expect(&self) -> (u64, PhotoAnswered) {
        let number = self.next.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = answer(Err(MediaError::Failed(
            "the camera stopped before it took the photo".to_owned(),
        )));
        self.pending().push_back((number, answer));
        (number, answered)
    }

    /// The JPEG stream delivered `image`: the photo asked first and not yet answered. On the
    /// stream's thread.
    pub(super) fn delivered(&self, image: CameraImage) {
        let Some((_, answer)) = self.pending().pop_front() else {
            return;
        };
        let photo = image.encoded().map_err(|error| {
            MediaError::Failed(format!("the camera's photo could not be read: {error}"))
        });
        answer.give(photo.map(|bytes| Photo {
            bytes,
            mime_type: "image/jpeg".to_owned(),
        }));
    }

    /// The photo `number` is not taken, for `error`.
    pub(super) fn failed(&self, number: u64, error: MediaError) {
        let mut pending = self.pending();
        let Some(index) = pending.iter().position(|(asked, _)| *asked == number) else {
            return;
        };
        if let Some((_, answer)) = pending.remove(index) {
            drop(pending);
            answer.give(Err(error));
        }
    }

    fn pending(&self) -> MutexGuard<'_, VecDeque<Asked>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
