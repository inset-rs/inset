//! A view's text input, as the framework drives it through `TextInputHost`, and Flutter's
//! `FlutterTextInputPlugin` takes its text input channel: the field the keyboards edit
//! (`field.rs`), in the view's `UIView` while its scene is connected; the keyboard's traits
//! (`traits.rs`); and the geometry UIKit asks of the text (`geometry.rs`, `positions.rs`).
//!
//! Each connection gets a field of its own, as Flutter's `setClient` makes a new view: UIKit keeps
//! the keyboard's state (its autocorrection, its suggestions, its password bar) by the field's
//! identity, and a field kept from one connection to the next carries it into the next. A start
//! while the field still has its connection is the framework's `show` or `updateConfig`, which
//! keep it.
//!
//! The view's `UIView` takes the first responder back as the field gives it up, so the hardware
//! keyboard's presses reach the view controller between fields too (`ui_view.rs`).

pub(crate) mod field;
mod geometry;
mod positions;
pub(crate) mod traits;

use std::cell::RefCell;

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
    /// The field of the current connection, or of the last until the next starts.
    field: RefCell<Option<Retained<WaftTextField>>>,
    /// The view's `UIView`, while its scene is connected.
    host: RefCell<Option<Retained<WaftView>>>,
}

impl IosTextInput {
    pub(crate) fn new(view: ViewId) -> IosTextInput {
        IosTextInput {
            view,
            field: RefCell::new(None),
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
        self.host.borrow_mut().take();
    }

    /// The field of the connection starting: the field already showing it, or a new one in the
    /// view's `UIView` in place of the last connection's.
    fn connected_field(&self) -> Option<Retained<WaftTextField>> {
        if let Some(field) = self.field()
            && field.is_connected()
        {
            return Some(field);
        }
        let field = WaftTextField::new(MainThreadMarker::new()?, self.view);
        if let Some(host) = self.host.borrow().as_ref() {
            host.addSubview(&field);
        }
        if let Some(last) = self.field.replace(Some(field.clone())) {
            last.removeFromSuperview();
        }
        Some(field)
    }

    fn field(&self) -> Option<Retained<WaftTextField>> {
        self.field.borrow().clone()
    }
}

impl TextInputHost for IosTextInput {
    /// Flutter's `setClient` with `show`, and `updateConfig`: the keyboard takes the field's
    /// traits, and shows.
    fn start(&self, configuration: &TextInputConfiguration) {
        if let Some(field) = self.connected_field() {
            field.show(Traits::of(configuration));
        }
    }

    /// Flutter's `clearClient` and `hide`: the keyboard goes, and the view's `UIView` takes the
    /// hardware keyboard again.
    fn stop(&self) {
        if let Some(field) = self.field() {
            field.hide();
        }
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
