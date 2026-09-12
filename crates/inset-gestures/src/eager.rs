//! Flutter counterpart: `gestures/eager.dart`.
//!
//! Leaf object. Superclass bags and `super` namespaces are in `recognizer.rs`.

use inset_foundation::{App, Handle};

use crate::arena::GestureDisposition;
use crate::events::{PointerDownEvent, PointerEvent};
use crate::recognizer::{
    GestureRecognizerData, OneSequenceData, OneSequenceGestureRecognizer, OneSequenceLeafData,
    RecognizerLeaf, RecognizerLeafData,
};

/// A gesture recognizer that eagerly claims victory in all gesture arenas.
///
/// This is typically passed in `AndroidView.gestureRecognizers` in order to immediately dispatch
/// all touch events inside the view bounds to the embedded Android view.
pub struct EagerGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
}

impl EagerGestureRecognizer {
    pub fn new(app: &mut App) -> Handle<EagerGestureRecognizer> {
        app.create(EagerGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
        })
    }
}

impl RecognizerLeafData for EagerGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for EagerGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }

    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl RecognizerLeaf for EagerGestureRecognizer {
    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, _event: &PointerDownEvent) {
        OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        OneSequenceGestureRecognizer::dispose(self, app);
    }

    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        self.start_tracking_pointer(app, event.pointer, event.transform);
        self.resolve(app, GestureDisposition::Accepted);
        OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, event.pointer);
    }

    fn handle_event(self: Handle<Self>, _app: &mut App, _event: PointerEvent) {}

    fn did_stop_tracking_last_pointer(self: Handle<Self>, _app: &mut App, _pointer: i64) {}

    fn debug_description(self: Handle<Self>) -> &'static str {
        "eager"
    }
}
