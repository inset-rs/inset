//! A view's text input, as the framework drives it through `TextInputHost`, and Flutter's
//! `FlutterTextInputPlugin` takes its text input channel: the field the keyboards edit
//! (`field.rs`), in the view's `UIView` while its scene is connected; the keyboard's traits
//! (`traits.rs`); and the geometry UIKit asks of the text (`geometry.rs`, `positions.rs`).
//!
//! Each client gets a field of its own, as Flutter's `setClient` makes a new view: UIKit keeps the
//! keyboard's state (its autocorrection, its suggestions, its password bar) by the field's
//! identity. A cleared client's field is taken out of the view only once it gave the keyboard up,
//! as Flutter defers it to `hide`, so a field taking the connection next keeps the keyboard up
//! rather than flicker it.
//!
//! The view's `UIView` takes the first responder back as the framework hides the keyboard, so the
//! hardware keyboard's presses reach the view controller between fields too (`ui_view.rs`).

pub(crate) mod field;
mod geometry;
mod positions;
pub(crate) mod traits;

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message};
use waft_embedder::{
    Matrix4, Rect, SelectionRect, Size, TextEditingValue, TextInputConfiguration, TextInputHost,
    ViewId,
};

use field::WaftTextField;
use traits::Traits;

use crate::ui_view::WaftView;

/// One view's text input.
pub(crate) struct IosTextInput {
    view: ViewId,
    /// The current client's field, Flutter's `activeView`, kept once the client is cleared until
    /// the next client's replaces it.
    field: RefCell<Option<Retained<WaftTextField>>>,
    /// Earlier clients' fields still in the view, which go once the active field shows or the
    /// keyboard hides.
    leaving: RefCell<Vec<Retained<WaftTextField>>>,
    /// Flutter's `pendingInputViewRemoval`: the cleared field had the keyboard, so `hide` takes it
    /// out of the view once it gave the keyboard up.
    removal_pending: Cell<bool>,
    /// The view's `UIView`, while its scene is connected.
    host: RefCell<Option<Retained<WaftView>>>,
}

impl IosTextInput {
    pub(crate) fn new(view: ViewId) -> IosTextInput {
        IosTextInput {
            view,
            field: RefCell::new(None),
            leaving: RefCell::new(Vec::new()),
            removal_pending: Cell::new(false),
            host: RefCell::new(None),
        }
    }

    /// The view's scene connected, and `host` is its `UIView` from now on: the field, where there
    /// is one, moves into it.
    pub(crate) fn attach(&self, host: &WaftView) {
        if let Some(field) = self.field.borrow().as_ref() {
            host.addSubview(field);
        }
        *self.host.borrow_mut() = Some(host.retain());
    }

    /// The view's scene disconnected, and its `UIView` with it.
    pub(crate) fn detach(&self) {
        if let Some(field) = self.field.borrow().as_ref() {
            field.removeFromSuperview();
        }
        self.remove_leaving_fields();
        self.host.borrow_mut().take();
    }

    /// A new field for a client, in the view's `UIView`, the last client's leaving the view now
    /// or, where it still has the keyboard, once the new one takes it.
    fn new_field(&self) -> Option<Retained<WaftTextField>> {
        let field = WaftTextField::new(MainThreadMarker::new()?, self.view);
        if let Some(host) = self.host.borrow().as_ref() {
            host.addSubview(&field);
        }
        if let Some(last) = self.field.replace(Some(field.clone())) {
            if last.isFirstResponder() {
                self.leaving.borrow_mut().push(last);
            } else {
                last.removeFromSuperview();
            }
        }
        Some(field)
    }

    fn remove_leaving_fields(&self) {
        for field in self.leaving.borrow_mut().drain(..) {
            field.removeFromSuperview();
        }
    }

    fn field(&self) -> Option<Retained<WaftTextField>> {
        self.field.borrow().clone()
    }
}

impl TextInputHost for IosTextInput {
    /// Flutter's `setTextInputClient:withConfiguration:`: a field of the new client's, with its
    /// keyboard's traits.
    fn set_client(&self, configuration: &TextInputConfiguration) {
        self.removal_pending.set(false);
        if let Some(field) = self.new_field() {
            field.configure(Traits::of(configuration));
        }
    }

    /// Flutter's `showTextInput`: the field takes the keyboard, from an earlier client's field
    /// that had it, which then leaves the view.
    fn show(&self) {
        if let Some(field) = self.field() {
            field.become_first_responder_from_framework();
        }
        self.remove_leaving_fields();
    }

    /// Flutter's `updateConfig:`.
    fn update_config(&self, configuration: &TextInputConfiguration) {
        if let Some(field) = self.field() {
            field.update_secure_text_entry(configuration.obscure_text);
        }
    }

    /// Flutter's `clearTextInputClient`: the field is no client's, and leaves the view now, or at
    /// `hide` where it still has the keyboard.
    fn clear_client(&self) {
        let Some(field) = self.field() else {
            return;
        };
        field.disconnect();
        if field.isFirstResponder() {
            self.removal_pending.set(true);
        } else {
            field.removeFromSuperview();
            self.removal_pending.set(false);
        }
    }

    /// Flutter's `hideTextInput`: the keyboard goes, the cleared field leaves the view, and the
    /// view's `UIView` takes the hardware keyboard again.
    fn hide(&self) {
        if let Some(field) = self.field() {
            field.resign_first_responder_from_framework();
            if self.removal_pending.replace(false) {
                field.removeFromSuperview();
            }
        }
        self.remove_leaving_fields();
        if let Some(host) = self.host.borrow().as_ref() {
            host.becomeFirstResponder();
        }
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        if let Some(field) = self.field() {
            field.set_text_input_state(value);
        }
    }

    /// Flutter's `setMarkedTextRect`.
    fn set_composing_rect(&self, rect: Rect) {
        if let Some(field) = self.field() {
            field.set_marked_rect(rect);
        }
    }

    /// Dropped, as Flutter's iOS embedding has no caret rect: UIKit asks the field for the caret
    /// at a position, which the selection rects answer.
    fn set_caret_rect(&self, _rect: Rect) {}

    /// Flutter's `setEditableSizeAndTransform`.
    fn set_client_geometry(&self, _size: Size, transform: &Matrix4) {
        if let Some(field) = self.field() {
            field.set_editable_transform(transform);
        }
    }

    fn set_selection_rects(&self, selection_rects: &[SelectionRect]) {
        if let Some(field) = self.field() {
            field.set_selection_rects(selection_rects);
        }
    }

    /// UIKit turns the editing keys the framework leaves into the field's edits, as AppKit does on
    /// macOS.
    fn handles_editing_keys(&self) -> bool {
        true
    }
}
