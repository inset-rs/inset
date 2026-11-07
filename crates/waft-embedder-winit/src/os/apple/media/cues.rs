//! A player's subtitles as cues: an `AVPlayerItemLegibleOutput` added to its item, set to suppress
//! the player's own rendering of them, so AVPlayer draws no subtitles (`suppressesPlayerRendering`).
//! At its `SourceAndRulesOnly` text styling resolution, which leaves out the user's Media
//! Accessibility settings, as WebKit sets its own legible output, it hands over the attributed
//! strings to show from each moment on with the media's own styling alone, which become the cues
//! the app draws (`cue_markup`); the user's settings are read on their own (`caption_appearance`).
//!
//! The delegate is called on the main queue, where the player is controlled, so the cues shown and
//! a selection's clearing them take turns.

use std::sync::{Mutex, MutexGuard, PoisonError};

use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_av_foundation::{
    AVPlayerItemLegibleOutput, AVPlayerItemLegibleOutputPushDelegate,
    AVPlayerItemLegibleOutputTextStylingResolutionSourceAndRulesOnly, AVPlayerItemOutput,
    AVPlayerItemOutputPushDelegate,
};
use objc2_core_media::CMTime;
use objc2_foundation::{NSArray, NSAttributedString};
use waft_embedder::Cue;

use super::abort_on_panic;
use super::cue_markup::cue_of;
use crate::media::Changes;

/// The cues of a player item's legible output, as they come.
pub(super) struct LegibleCues {
    output: Retained<AVPlayerItemLegibleOutput>,
    /// The delegate the output calls, which it holds weakly.
    delegate: Retained<LegibleDelegate>,
}

impl LegibleCues {
    /// An output whose cues tell `changes` each time they change.
    pub(super) fn new(changes: Changes) -> LegibleCues {
        let delegate = LegibleDelegate::alloc().set_ivars(Shown {
            cues: Mutex::default(),
            changes,
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let delegate: Retained<LegibleDelegate> = unsafe { msg_send![super(delegate), init] };
        // SAFETY: an output of attributed strings in the media's own styling, which the player
        // does not draw, calling the delegate on the main queue; the resolution is a constant
        // AVFoundation exports for the life of the process.
        let output = unsafe {
            let output = AVPlayerItemLegibleOutput::new();
            output.setSuppressesPlayerRendering(true);
            output.setTextStylingResolution(
                AVPlayerItemLegibleOutputTextStylingResolutionSourceAndRulesOnly,
            );
            output.setDelegate_queue(
                Some(ProtocolObject::from_ref(&*delegate)),
                Some(DispatchQueue::main()),
            );
            output
        };
        LegibleCues { output, delegate }
    }

    /// The output the player's item hands its subtitles to.
    pub(super) fn output(&self) -> &AVPlayerItemOutput {
        &self.output
    }

    /// The cues to show now.
    pub(super) fn cues(&self) -> Vec<Cue> {
        self.delegate.ivars().cues().clone()
    }

    /// Shows no cues until the output hands over more, as when the text track changes.
    pub(super) fn clear(&self) {
        self.delegate.ivars().show(Vec::new());
    }
}

/// The cues shown, and whom to tell when they change.
struct Shown {
    cues: Mutex<Vec<Cue>>,
    changes: Changes,
}

impl Shown {
    fn cues(&self) -> MutexGuard<'_, Vec<Cue>> {
        self.cues.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Shows `cues` from now on, telling the changes if they differ from those shown.
    fn show(&self, cues: Vec<Cue>) {
        let changed = {
            let mut shown = self.cues();
            let changed = *shown != cues;
            *shown = cues;
            changed
        };
        if changed {
            self.changes.notify();
        }
    }
}

define_class!(
    /// Keeps the cues a legible output hands over.
    //
    // SAFETY: NSObject has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "WaftLegibleOutputDelegate"]
    #[ivars = Shown]
    struct LegibleDelegate;

    unsafe impl NSObjectProtocol for LegibleDelegate {}

    unsafe impl AVPlayerItemOutputPushDelegate for LegibleDelegate {
        /// A seek or a change of direction: what was shown is no longer, as Media3 clears its cues
        /// when the position resets.
        #[unsafe(method(outputSequenceWasFlushed:))]
        fn flushed(&self, _output: &AVPlayerItemOutput) {
            abort_on_panic(|| self.ivars().show(Vec::new()));
        }
    }

    unsafe impl AVPlayerItemLegibleOutputPushDelegate for LegibleDelegate {
        #[unsafe(method(legibleOutput:didOutputAttributedStrings:nativeSampleBuffers:forItemTime:))]
        fn delivered(
            &self,
            _output: &AVPlayerItemLegibleOutput,
            strings: &NSArray<NSAttributedString>,
            _native_samples: &NSArray,
            _item_time: CMTime,
        ) {
            abort_on_panic(|| {
                let cues = strings.iter().map(|string| cue_of(&string)).collect();
                self.ivars().show(cues);
            });
        }
    }
);
