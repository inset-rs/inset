//! The view's frames the GPU may still be working on, at most two, as Impeller's hardware-buffer
//! swapchain keeps (`kMaxPendingPresents`): before a frame is drawn, the loop waits for the last
//! submission of the frame two before it. This is the view's one back-pressure: the loop runs at
//! most two frames ahead of the GPU, and so takes at most a few of the view's buffers at a time.
//!
//! The invariant: a frame's last submission is held here from its submit until two later frames
//! wait for their turn, and its semaphore with it (`frame_fence`), so no semaphore is destroyed
//! while its signal is pending.

use std::collections::VecDeque;

use crate::frame_fence::FrameSubmission;

/// How many frames may be pending on the GPU while the loop draws another.
const MAX_PENDING: usize = 2;

/// The last submissions of the frames pending on the GPU, oldest first.
pub(super) struct FramesInFlight {
    pending: VecDeque<FrameSubmission>,
}

impl FramesInFlight {
    pub(super) fn new() -> FramesInFlight {
        FramesInFlight {
            pending: VecDeque::with_capacity(MAX_PENDING),
        }
    }

    /// Waits until fewer than two frames are pending, so another may be drawn.
    pub(super) fn wait_for_turn(&mut self) {
        while self.pending.len() >= MAX_PENDING {
            // Dropping a submission waits for it (`FrameSubmission`).
            drop(self.pending.pop_front());
        }
    }

    /// A frame was submitted, its last submission `submission`.
    pub(super) fn submitted(&mut self, submission: FrameSubmission) {
        self.pending.push_back(submission);
    }
}
