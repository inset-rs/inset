//! The platform's [`VsyncWaiter`] where the main loop serves it: a wait asked from any thread
//! reaches the loop as an event, and the frame scheduler serves it at the next vsync of the screen
//! it names, from the source the frames of the views on that screen are drawn at
//! (`frame_scheduler.rs`). A system whose display links tick in every mode of its run loop serves
//! the waits there instead (`os`'s `FramePresentation::vsync_waiter`), since the loop hears nothing
//! while the system runs a loop of its own, as a menu does.

use std::sync::Weak;

use inset_embedder::{ScreenId, VsyncObserver, VsyncWaiter};

use crate::main_loop::{MainLoopEvent, MainLoopProxy};

/// The screens' vsyncs as any thread waits on them: each wait is an event through the loop's
/// proxy.
pub(crate) struct LoopVsyncWaiter(pub(crate) MainLoopProxy);

impl VsyncWaiter for LoopVsyncWaiter {
    fn next_vsync(&self, screen: ScreenId, observer: Weak<dyn VsyncObserver>) {
        self.0.send(MainLoopEvent::VsyncWaited(screen, observer));
    }
}
