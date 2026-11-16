//! The field the system's keyboards edit: a `UIView` implementing `UITextInput`, a port of Flutter's
//! `FlutterTextInputView`. It sits, zero-sized, in the view's `UIView` at the editable's origin, and
//! takes the first responder while a text field of the view's has the framework's connection.
//!
//! The field's text, selection and marked range live here, on the main thread, where UIKit asks
//! for them and changes them, and every change the keyboard makes goes to the framework at once,
//! through the host (`host.rs`), as Flutter's view sends `updateEditingState` after each. What the
//! framework sets comes back through [`WaftTextField::set_text_input_state`], which tells the
//! keyboard's input delegate the text and the selection will change and did, as Flutter's does;
//! the framework never hears its own value back. Offsets are UTF-16 code units, which an
//! `NSString` and a `TextEditingValue` both count in.
//!
//! Left out of Flutter's view: Scribble and its focus, the floating cursor, dictation and text
//! placeholders, the edit menu interaction, autofill groups and the restore of the first responder
//! the framework is asked about, the delta model, and accessibility (`PORTING.md`).

// A UIKit subclass, its protocols' methods and the super calls they make need `unsafe`; this
// module writes it for them.
#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};

use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, Message, available, define_class, msg_send,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSArray, NSComparisonResult, NSDictionary, NSInteger, NSMutableString, NSRange, NSString,
};
use objc2_ui_kit::{
    NSWritingDirection, UIInputViewController, UIKeyInput, UIKeyboardAppearance, UIKeyboardType,
    UIResponder, UIReturnKeyType, UITextAutocapitalizationType, UITextAutocorrectionType,
    UITextContentType, UITextInput, UITextInputDelegate, UITextInputStringTokenizer,
    UITextInputTokenizer, UITextInputTraits, UITextLayoutDirection, UITextPosition, UITextRange,
    UITextSelectionRect, UITextSmartDashesType, UITextSmartQuotesType, UITextSpellCheckingType,
    UITextStorageDirection, UIView,
};
use waft_embedder::{
    Matrix4, Offset, Rect, SelectionRect, TextAffinity, TextDirection, TextEditingValue, TextRange,
    TextSelection, ViewId,
};

use super::geometry::{
    GlyphRect, caret_rect, closest_position, first_rect, rect_through, rects_in_range,
};
use super::positions::{
    WaftTextPosition, WaftTextRange, WaftTextSelectionRect, position_of, range_of,
};
use super::traits::{Traits, action_of};
use crate::host::{HostEvent, deliver};

pub(crate) struct Field {
    view: ViewId,
    text: Retained<NSMutableString>,
    selected: Cell<NSRange>,
    marked: Cell<Option<NSRange>>,
    selection_affinity: Cell<TextAffinity>,
    input_delegate: RefCell<Option<Weak<ProtocolObject<dyn UITextInputDelegate>>>>,
    traits: RefCell<Traits>,
    /// Whether a text field of the view's has the framework's connection: Flutter's
    /// `_textInputClient != 0`.
    connected: Cell<bool>,
    resigning_from_framework: Cell<bool>,
    /// Where the marked text is, in the editable's coordinates; `None` before the framework says.
    marked_rect: Cell<Option<Rect>>,
    editable_transform: Cell<[f32; 16]>,
    selection_rects: RefCell<Vec<GlyphRect>>,
    temporarily_deleted_composed_character: RefCell<Option<Retained<NSString>>>,
    tokenizer: RefCell<Option<Retained<UITextInputStringTokenizer>>>,
    input_view_controller: RefCell<Option<Retained<UIInputViewController>>>,
}

define_class!(
    // SAFETY: UIView has no subclassing requirements, and `WaftTextField` has no `Drop`.
    #[unsafe(super(UIView, UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftTextField"]
    #[ivars = Field]
    pub(crate) struct WaftTextField;

    unsafe impl NSObjectProtocol for WaftTextField {}

    impl WaftTextField {
        #[unsafe(method(canBecomeFirstResponder))]
        fn can_become_first_responder(&self) -> bool {
            self.ivars().connected.get()
        }

        /// Flutter's `resignFirstResponder`: where the framework did not ask for it, the
        /// framework hears its client's connection closed, which a field no client has any more,
        /// as another field takes the first responder from it, says of none.
        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            // SAFETY: the super call of the method overridden here.
            let resigned: bool = unsafe { msg_send![super(self), resignFirstResponder] };
            let ivars = self.ivars();
            if resigned && !ivars.resigning_from_framework.get() && ivars.connected.get() {
                deliver(HostEvent::TextInputClosed(ivars.view));
            }
            resigned
        }

        #[unsafe(method_id(inputViewController))]
        fn input_view_controller(&self) -> Option<Retained<UIInputViewController>> {
            self.input_view_controller_now()
        }
    }

    unsafe impl UITextInputTraits for WaftTextField {
        #[unsafe(method(keyboardType))]
        fn keyboard_type(&self) -> UIKeyboardType {
            self.ivars().traits.borrow().keyboard_type
        }

        #[unsafe(method(returnKeyType))]
        fn return_key_type(&self) -> UIReturnKeyType {
            self.ivars().traits.borrow().return_key_type
        }

        #[unsafe(method(autocapitalizationType))]
        fn autocapitalization_type(&self) -> UITextAutocapitalizationType {
            self.ivars().traits.borrow().autocapitalization_type
        }

        #[unsafe(method(autocorrectionType))]
        fn autocorrection_type(&self) -> UITextAutocorrectionType {
            self.ivars().traits.borrow().autocorrection_type
        }

        #[unsafe(method(spellCheckingType))]
        fn spell_checking_type(&self) -> UITextSpellCheckingType {
            self.ivars().traits.borrow().spell_checking_type
        }

        #[unsafe(method(smartDashesType))]
        fn smart_dashes_type(&self) -> UITextSmartDashesType {
            self.ivars().traits.borrow().smart_dashes_type
        }

        #[unsafe(method(smartQuotesType))]
        fn smart_quotes_type(&self) -> UITextSmartQuotesType {
            self.ivars().traits.borrow().smart_quotes_type
        }

        #[unsafe(method(keyboardAppearance))]
        fn keyboard_appearance(&self) -> UIKeyboardAppearance {
            self.ivars().traits.borrow().keyboard_appearance
        }

        #[unsafe(method(isSecureTextEntry))]
        fn is_secure_text_entry(&self) -> bool {
            self.ivars().traits.borrow().secure_text_entry
        }

        #[unsafe(method_id(textContentType))]
        fn text_content_type(&self) -> Retained<UITextContentType> {
            self.text_content_type_now()
        }
    }

    unsafe impl UIKeyInput for WaftTextField {
        #[unsafe(method(hasText))]
        fn has_text(&self) -> bool {
            self.ivars().text.length() > 0
        }

        #[unsafe(method(insertText:))]
        fn insert_text(&self, text: &NSString) {
            self.insert(text);
        }

        #[unsafe(method(deleteBackward))]
        fn delete_backward(&self) {
            self.delete_before_caret();
        }
    }

    unsafe impl UITextInput for WaftTextField {
        #[unsafe(method_id(textInRange:))]
        fn text_in_range(&self, range: &UITextRange) -> Option<Retained<NSString>> {
            self.text_in_range_now(range)
        }

        #[unsafe(method(replaceRange:withText:))]
        fn replace_range_with_text(&self, range: &UITextRange, text: &NSString) {
            if let Some(range) = range_of(range) {
                self.replace(range, text);
            }
        }

        #[unsafe(method_id(selectedTextRange))]
        fn selected_text_range(&self) -> Option<Retained<UITextRange>> {
            self.selected_text_range_now()
        }

        /// Flutter's `setSelectedTextRange:`, ignored where the field takes no selection.
        #[unsafe(method(setSelectedTextRange:))]
        fn set_selected_text_range(&self, range: Option<&UITextRange>) {
            if !self.ivars().traits.borrow().enable_interactive_selection {
                return;
            }
            if let Some(range) = range.and_then(range_of) {
                self.set_selected_local(range);
            }
            self.update_editing_state();
        }

        #[unsafe(method_id(markedTextRange))]
        fn marked_text_range(&self) -> Option<Retained<UITextRange>> {
            self.marked_text_range_now()
        }

        #[unsafe(method_id(markedTextStyle))]
        fn marked_text_style(&self) -> Option<Retained<NSDictionary<NSString, AnyObject>>> {
            self.marked_text_style_now()
        }

        #[unsafe(method(setMarkedTextStyle:))]
        fn set_marked_text_style(&self, _style: Option<&NSDictionary<NSString, AnyObject>>) {}

        #[unsafe(method(setMarkedText:selectedRange:))]
        fn set_marked_text_selected_range(
            &self,
            marked_text: Option<&NSString>,
            selected_range: NSRange,
        ) {
            self.mark(marked_text, selected_range);
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            if self.ivars().marked.take().is_some() {
                self.update_editing_state();
            }
        }

        #[unsafe(method_id(beginningOfDocument))]
        fn beginning_of_document(&self) -> Retained<UITextPosition> {
            self.beginning_of_document_now()
        }

        #[unsafe(method_id(endOfDocument))]
        fn end_of_document(&self) -> Retained<UITextPosition> {
            self.end_of_document_now()
        }

        #[unsafe(method_id(textRangeFromPosition:toPosition:))]
        fn text_range_from_position_to_position(
            &self,
            from: &UITextPosition,
            to: &UITextPosition,
        ) -> Option<Retained<UITextRange>> {
            self.text_range_from_position_to_position_now(from, to)
        }

        #[unsafe(method_id(positionFromPosition:offset:))]
        fn position_from_position_offset(
            &self,
            position: &UITextPosition,
            offset: NSInteger,
        ) -> Option<Retained<UITextPosition>> {
            self.position_from_position_offset_now(position, offset)
        }

        /// Flutter's, with its right and down taking one step whatever the offset.
        #[unsafe(method_id(positionFromPosition:inDirection:offset:))]
        fn position_from_position_in_direction_offset(
            &self,
            position: &UITextPosition,
            direction: UITextLayoutDirection,
            offset: NSInteger,
        ) -> Option<Retained<UITextPosition>> {
            self.position_from_position_in_direction_offset_now(position, direction, offset)
        }

        #[unsafe(method(comparePosition:toPosition:))]
        fn compare_position_to_position(
            &self,
            position: &UITextPosition,
            other: &UITextPosition,
        ) -> NSComparisonResult {
            let (Some(position), Some(other)) = (position_of(position), position_of(other)) else {
                return NSComparisonResult::Same;
            };
            match position.index().cmp(&other.index()) {
                std::cmp::Ordering::Less => NSComparisonResult::Ascending,
                std::cmp::Ordering::Greater => NSComparisonResult::Descending,
                std::cmp::Ordering::Equal if position.affinity() == other.affinity() => {
                    NSComparisonResult::Same
                }
                std::cmp::Ordering::Equal
                    if position.affinity() == UITextStorageDirection::Backward =>
                {
                    NSComparisonResult::Ascending
                }
                std::cmp::Ordering::Equal => NSComparisonResult::Descending,
            }
        }

        #[unsafe(method(offsetFromPosition:toPosition:))]
        fn offset_from_position_to_position(
            &self,
            from: &UITextPosition,
            to: &UITextPosition,
        ) -> NSInteger {
            let (Some(from), Some(to)) = (position_of(from), position_of(to)) else {
                return 0;
            };
            to.index() as NSInteger - from.index() as NSInteger
        }

        #[unsafe(method_id(inputDelegate))]
        fn input_delegate(&self) -> Option<Retained<ProtocolObject<dyn UITextInputDelegate>>> {
            self.input_delegate_now()
        }

        #[unsafe(method(setInputDelegate:))]
        fn set_input_delegate(
            &self,
            delegate: Option<&ProtocolObject<dyn UITextInputDelegate>>,
        ) {
            *self.ivars().input_delegate.borrow_mut() = delegate.map(Weak::from);
        }

        #[unsafe(method_id(tokenizer))]
        fn tokenizer(&self) -> Retained<ProtocolObject<dyn UITextInputTokenizer>> {
            self.tokenizer_now()
        }

        #[unsafe(method_id(positionWithinRange:farthestInDirection:))]
        fn position_within_range_farthest_in_direction(
            &self,
            range: &UITextRange,
            direction: UITextLayoutDirection,
        ) -> Option<Retained<UITextPosition>> {
            self.position_within_range_farthest_in_direction_now(range, direction)
        }

        #[unsafe(method_id(characterRangeByExtendingPosition:inDirection:))]
        fn character_range_by_extending_position_in_direction(
            &self,
            position: &UITextPosition,
            direction: UITextLayoutDirection,
        ) -> Option<Retained<UITextRange>> {
            self.character_range_by_extending_position_in_direction_now(position, direction)
        }

        #[unsafe(method(baseWritingDirectionForPosition:inDirection:))]
        fn base_writing_direction_for_position_in_direction(
            &self,
            _position: &UITextPosition,
            _direction: UITextStorageDirection,
        ) -> NSWritingDirection {
            NSWritingDirection::Natural
        }

        #[unsafe(method(setBaseWritingDirection:forRange:))]
        fn set_base_writing_direction_for_range(
            &self,
            _direction: NSWritingDirection,
            _range: &UITextRange,
        ) {
        }

        #[unsafe(method(firstRectForRange:))]
        fn first_rect_for_range(&self, range: &UITextRange) -> CGRect {
            let Some(range) = range_of(range) else {
                return CGRect::ZERO;
            };
            self.first_rect(range)
        }

        #[unsafe(method(caretRectForPosition:))]
        fn caret_rect_for_position(&self, position: &UITextPosition) -> CGRect {
            let Some(position) = position_of(position) else {
                return CGRect::ZERO;
            };
            self.caret_rect(position.index(), position.affinity())
        }

        #[unsafe(method_id(selectionRectsForRange:))]
        fn selection_rects_for_range(
            &self,
            range: &UITextRange,
        ) -> Retained<NSArray<UITextSelectionRect>> {
            self.selection_rects_for_range_now(range)
        }

        #[unsafe(method_id(closestPositionToPoint:))]
        fn closest_position_to_point(&self, point: CGPoint) -> Option<Retained<UITextPosition>> {
            self.closest_position_to_point_now(point)
        }

        #[unsafe(method_id(closestPositionToPoint:withinRange:))]
        fn closest_position_to_point_within_range(
            &self,
            point: CGPoint,
            range: &UITextRange,
        ) -> Option<Retained<UITextPosition>> {
            self.closest_position_to_point_within_range_now(point, range)
        }

        #[unsafe(method_id(characterRangeAtPoint:))]
        fn character_range_at_point(&self, _point: CGPoint) -> Option<Retained<UITextRange>> {
            self.character_range_at_point_now(_point)
        }

        #[unsafe(method(shouldChangeTextInRange:replacementText:))]
        fn should_change_text_in_range_replacement_text(
            &self,
            _range: &UITextRange,
            text: &NSString,
        ) -> bool {
            self.should_change_to(text)
        }
    }
);

/// The answers to UIKit's questions that answer an object.
impl WaftTextField {
    fn text_range_from_position_to_position_now(
        &self,
        from: &UITextPosition,
        to: &UITextPosition,
    ) -> Option<Retained<UITextRange>> {
        let from = position_of(from)?.index();
        let to = position_of(to)?.index();
        // A tokenizer that does not handle CJK well may answer `to` before `from`: swapped, as
        // a native text view does.
        let range = NSRange::new(from.min(to), from.abs_diff(to));
        Some(Retained::into_super(WaftTextRange::new(self.mtm(), range)))
    }

    fn position_from_position_offset_now(
        &self,
        position: &UITextPosition,
        offset: NSInteger,
    ) -> Option<Retained<UITextPosition>> {
        let index = self.moved_by_graphemes(position_of(position)?.index(), offset)?;
        Some(self.position(index, UITextStorageDirection::Forward))
    }

    fn position_from_position_in_direction_offset_now(
        &self,
        position: &UITextPosition,
        direction: UITextLayoutDirection,
        offset: NSInteger,
    ) -> Option<Retained<UITextPosition>> {
        let offset = match direction {
            UITextLayoutDirection::Left | UITextLayoutDirection::Up => -offset,
            _ => 1,
        };
        let index = self.moved_by_graphemes(position_of(position)?.index(), offset)?;
        Some(self.position(index, UITextStorageDirection::Forward))
    }

    fn position_within_range_farthest_in_direction_now(
        &self,
        range: &UITextRange,
        direction: UITextLayoutDirection,
    ) -> Option<Retained<UITextPosition>> {
        let range = range_of(range)?;
        Some(match direction {
            UITextLayoutDirection::Left | UITextLayoutDirection::Up => {
                self.position(range.location, UITextStorageDirection::Forward)
            }
            _ => self.position(
                range.location + range.length,
                UITextStorageDirection::Backward,
            ),
        })
    }

    fn character_range_by_extending_position_in_direction_now(
        &self,
        position: &UITextPosition,
        direction: UITextLayoutDirection,
    ) -> Option<Retained<UITextRange>> {
        let index = position_of(position)?.index();
        let (start, end) = match direction {
            UITextLayoutDirection::Left | UITextLayoutDirection::Up => {
                (self.decrement(index), index)
            }
            _ => (index, self.increment(index)),
        };
        Some(Retained::into_super(WaftTextRange::new(
            self.mtm(),
            NSRange::new(start, end - start),
        )))
    }

    fn selection_rects_for_range_now(
        &self,
        range: &UITextRange,
    ) -> Retained<NSArray<UITextSelectionRect>> {
        // Some keyboards, the Japanese one in the simulator among them, ask with a range of
        // positions not the field's.
        let Some(range) = range_of(range) else {
            return NSArray::new();
        };
        let rects: Vec<Retained<UITextSelectionRect>> = self
            .rects_in(range)
            .into_iter()
            .map(|rect| {
                Retained::into_super(WaftTextSelectionRect::new(
                    self.mtm(),
                    cg_rect(rect.rect),
                    rect.contains_start,
                    rect.contains_end,
                ))
            })
            .collect();
        NSArray::from_retained_slice(&rects)
    }

    fn closest_position_to_point_within_range_now(
        &self,
        point: CGPoint,
        range: &UITextRange,
    ) -> Option<Retained<UITextPosition>> {
        self.closest_within(point, range_of(range)?)
    }

    fn input_view_controller_now(&self) -> Option<Retained<UIInputViewController>> {
        if self.ivars().traits.borrow().is_system_keyboard_enabled {
            return None;
        }
        let mut controller = self.ivars().input_view_controller.borrow_mut();
        Some(
            controller
                .get_or_insert_with(|| UIInputViewController::new(self.mtm()))
                .clone(),
        )
    }

    fn text_content_type_now(&self) -> Retained<UITextContentType> {
        self.ivars().traits.borrow().text_content_type.clone()
    }

    fn text_in_range_now(&self, range: &UITextRange) -> Option<Retained<NSString>> {
        let range = range_of(range)?;
        let text = &self.ivars().text;
        let location = range.location.min(text.length());
        let length = (text.length() - location).min(range.length);
        Some(text.substringWithRange(NSRange::new(location, length)))
    }

    fn selected_text_range_now(&self) -> Option<Retained<UITextRange>> {
        Some(Retained::into_super(WaftTextRange::new(
            self.mtm(),
            self.ivars().selected.get(),
        )))
    }

    fn marked_text_range_now(&self) -> Option<Retained<UITextRange>> {
        let marked = self.ivars().marked.get()?;
        Some(Retained::into_super(WaftTextRange::new(self.mtm(), marked)))
    }

    fn marked_text_style_now(&self) -> Option<Retained<NSDictionary<NSString, AnyObject>>> {
        None
    }

    fn beginning_of_document_now(&self) -> Retained<UITextPosition> {
        self.position(0, UITextStorageDirection::Forward)
    }

    fn end_of_document_now(&self) -> Retained<UITextPosition> {
        self.position(self.ivars().text.length(), UITextStorageDirection::Backward)
    }

    fn tokenizer_now(&self) -> Retained<ProtocolObject<dyn UITextInputTokenizer>> {
        let mut tokenizer = self.ivars().tokenizer.borrow_mut();
        let tokenizer = tokenizer.get_or_insert_with(|| {
            let responder: &UIResponder = self;
            // SAFETY: the field implements `UITextInput`, and the tokenizer holds it weakly.
            unsafe {
                UITextInputStringTokenizer::initWithTextInput(
                    UITextInputStringTokenizer::alloc(self.mtm()),
                    responder,
                )
            }
        });
        ProtocolObject::from_retained(tokenizer.clone())
    }

    fn closest_position_to_point_now(&self, point: CGPoint) -> Option<Retained<UITextPosition>> {
        if self.ivars().selection_rects.borrow().is_empty() {
            let selected = self.ivars().selected.get();
            return Some(self.position(selected.location, UITextStorageDirection::Forward));
        }
        let length = self.ivars().text.length();
        self.closest_within(point, NSRange::new(0, length))
    }

    fn character_range_at_point_now(&self, _point: CGPoint) -> Option<Retained<UITextRange>> {
        let index = self.ivars().selected.get().location;
        let range = self.character_range_at(index);
        Some(Retained::into_super(WaftTextRange::new(self.mtm(), range)))
    }
}

impl WaftTextField {
    /// The field of the view numbered `view`, unconfigured, with no text.
    pub(crate) fn new(main_thread: MainThreadMarker, view: ViewId) -> Retained<WaftTextField> {
        let this = main_thread.alloc().set_ivars(Field {
            view,
            text: NSMutableString::new(),
            selected: Cell::new(NSRange::new(0, 0)),
            marked: Cell::new(None),
            selection_affinity: Cell::new(TextAffinity::Downstream),
            input_delegate: RefCell::new(None),
            traits: RefCell::new(Traits::unconfigured()),
            connected: Cell::new(false),
            resigning_from_framework: Cell::new(false),
            marked_rect: Cell::new(None),
            editable_transform: Cell::new(Matrix4::IDENTITY.to_flutter_array()),
            selection_rects: RefCell::new(Vec::new()),
            temporarily_deleted_composed_character: RefCell::new(None),
            tokenizer: RefCell::new(None),
            input_view_controller: RefCell::new(None),
        });
        // SAFETY: `initWithFrame:` of an allocated view, zero-sized as Flutter's is.
        unsafe { msg_send![super(this), initWithFrame: CGRect::ZERO] }
    }

    /// Flutter's `configureWithDictionary:` and `setTextInputClient:`: the field is the client's,
    /// with its keyboard's `traits`.
    pub(crate) fn configure(&self, traits: Traits) {
        *self.ivars().traits.borrow_mut() = traits;
        self.ivars().connected.set(true);
    }

    /// Flutter's `updateConfig:`, of which iOS takes only whether the text is secure: the space
    /// bar's cursor depends on it, so the keyboard reloads where it changed.
    pub(crate) fn update_secure_text_entry(&self, secure: bool) {
        let changed = {
            let mut traits = self.ivars().traits.borrow_mut();
            let changed = traits.secure_text_entry != secure;
            traits.secure_text_entry = secure;
            changed
        };
        if changed {
            self.reloadInputViews();
        }
    }

    /// Flutter's `setTextInputClient:0` with its frame emptied, as `clearTextInputClient` does: no
    /// client has the field any more.
    pub(crate) fn disconnect(&self) {
        self.ivars().connected.set(false);
        self.setFrame(CGRect::ZERO);
    }

    /// Flutter's `becomeFirstResponderFromFramework`: the keyboard shows for the field.
    pub(crate) fn become_first_responder_from_framework(&self) {
        self.becomeFirstResponder();
    }

    /// Flutter's `resignFirstResponderFromFramework`: the keyboard goes, and the framework, which
    /// asked, does not hear of it.
    pub(crate) fn resign_first_responder_from_framework(&self) {
        self.ivars().resigning_from_framework.set(true);
        self.resignFirstResponder();
        self.ivars().resigning_from_framework.set(false);
    }

    /// Flutter's `setTextInputState:`: the framework's value, the keyboard's input delegate told
    /// the text and the selection will change and did.
    pub(crate) fn set_text_input_state(&self, value: &TextEditingValue) {
        let ivars = self.ivars();
        let delegate = self.input_delegate_now();
        let new_text = NSString::from_str(&value.text);
        let text_changed = !ivars.text.isEqualToString(&new_text);
        if text_changed {
            if let Some(delegate) = &delegate {
                delegate.textWillChange(Some(self.as_text_input()));
            }
            ivars.text.setString(&new_text);
        }
        let length = ivars.text.length();
        let composing = clamp(span(value.composing.start, value.composing.end), length);
        ivars.marked.set(composing.filter(|range| range.length > 0));
        let selection = value.selection;
        let selected = clamp(span(selection.base_offset, selection.extent_offset), length)
            .unwrap_or(NSRange::new(0, 0));
        if selected != ivars.selected.get() {
            if let Some(delegate) = &delegate {
                delegate.selectionWillChange(Some(self.as_text_input()));
            }
            self.set_selected_local(selected);
            ivars.selection_affinity.set(selection.affinity);
            if let Some(delegate) = &delegate {
                delegate.selectionDidChange(Some(self.as_text_input()));
            }
        }
        if text_changed && let Some(delegate) = &delegate {
            delegate.textDidChange(Some(self.as_text_input()));
        }
    }

    /// Flutter's `setMarkedRect:`: `rect`, in the editable's coordinates; a rect of negative size
    /// is none.
    pub(crate) fn set_marked_rect(&self, rect: Rect) {
        let none = rect.width() < 0.0 && rect.height() < 0.0;
        self.ivars().marked_rect.set((!none).then_some(rect));
    }

    /// Flutter's `setEditableTransform:`, and, on iOS 17 and later, the field placed at the
    /// editable's origin, so the system's autocorrection highlight lies over the text.
    pub(crate) fn set_editable_transform(&self, transform: &Matrix4) {
        let transform = transform.to_flutter_array();
        self.ivars().editable_transform.set(transform);
        if available!(ios = 17.0) {
            let origin = CGPoint::new(f64::from(transform[12]), f64::from(transform[13]));
            self.setFrame(CGRect::new(origin, CGSize::ZERO));
        }
    }

    /// Flutter's `setSelectionRects:`.
    pub(crate) fn set_selection_rects(&self, rects: &[SelectionRect]) {
        *self.ivars().selection_rects.borrow_mut() = rects
            .iter()
            .map(|rect| GlyphRect {
                position: usize::try_from(rect.position).unwrap_or(0),
                rect: rect.bounds,
                rtl: rect.direction == TextDirection::Rtl,
            })
            .collect();
    }

    fn input_delegate_now(&self) -> Option<Retained<ProtocolObject<dyn UITextInputDelegate>>> {
        self.ivars().input_delegate.borrow().as_ref()?.load()
    }

    fn as_text_input(&self) -> &ProtocolObject<dyn UITextInput> {
        ProtocolObject::from_ref(self)
    }

    fn position(&self, index: usize, affinity: UITextStorageDirection) -> Retained<UITextPosition> {
        Retained::into_super(WaftTextPosition::new(self.mtm(), index, affinity))
    }

    /// Flutter's `setSelectedTextRangeLocal:`: `range`, widened to whole graphemes where the field
    /// has text, without telling the framework.
    fn set_selected_local(&self, range: NSRange) {
        let text = &self.ivars().text;
        let range = if text.length() > 0 {
            text.rangeOfComposedCharacterSequencesForRange(clamp_range(range, text.length()))
        } else {
            range
        };
        self.ivars().selected.set(range);
    }

    /// Flutter's `replaceRange:withText:` and `replaceRangeLocal:withText:`: `text` in place of
    /// `range`, the caret after it and nothing marked, which the framework hears.
    fn replace(&self, range: NSRange, text: &NSString) {
        let ivars = self.ivars();
        let length = ivars.text.length();
        ivars
            .text
            .replaceCharactersInRange_withString(clamp_range(range, length), text);
        let caret = clamp_range(
            NSRange::new(range.location + text.length(), 0),
            ivars.text.length(),
        );
        self.set_selected_local(caret);
        ivars.marked.set(None);
        self.update_editing_state();
    }

    /// Flutter's `setMarkedText:selectedRange:`: `marked_text` in place of what was marked, or of
    /// the selection, marked itself, with `selected_range` of it selected.
    fn mark(&self, marked_text: Option<&NSString>, selected_range: NSRange) {
        let ivars = self.ivars();
        let empty = NSString::new();
        let marked_text = marked_text.unwrap_or(&empty);
        let replaced = ivars
            .marked
            .get()
            .filter(|marked| marked.length > 0)
            .unwrap_or(ivars.selected.get());
        ivars
            .text
            .replaceCharactersInRange_withString(replaced, marked_text);
        let marked = NSRange::new(replaced.location, marked_text.length());
        ivars.marked.set((marked.length > 0).then_some(marked));
        let selected = clamp_range(
            NSRange::new(
                selected_range.location + marked.location,
                selected_range.length,
            ),
            ivars.text.length(),
        );
        self.set_selected_local(selected);
        self.update_editing_state();
    }

    /// Flutter's `insertText:`: `text` in place of what is marked, or of the selection, the
    /// selection rects after it moved along. iOS hands the second half of an emoji deleted and
    /// typed again as a lone surrogate, which the emoji deleted last stands in for.
    fn insert(&self, text: &NSString) {
        let ivars = self.ivars();
        let deleted = ivars
            .temporarily_deleted_composed_character
            .borrow()
            .clone();
        let text = match deleted {
            Some(deleted)
                if deleted.length() > 0
                    && text.length() == 1
                    && is_lone_surrogate(text)
                    && text.characterAtIndex(0) == deleted.characterAtIndex(0) =>
            {
                ivars.temporarily_deleted_composed_character.replace(None);
                deleted
            }
            _ => text.retain(),
        };
        let replaced = ivars.marked.get().unwrap_or(ivars.selected.get());
        self.move_selection_rects_for_insert(replaced.location, text.length());
        ivars.selection_affinity.set(TextAffinity::Downstream);
        self.replace(replaced, &text);
    }

    /// The selection rects moved along for `inserted` units typed at `at`, as Flutter's
    /// `insertText:` copies them, until the framework sends new ones.
    fn move_selection_rects_for_insert(&self, at: usize, inserted: usize) {
        let mut rects = self.ivars().selection_rects.borrow_mut();
        let mut moved = Vec::with_capacity(rects.len());
        for rect in rects.iter() {
            if rect.position == at {
                for step in 0..=inserted {
                    moved.push(GlyphRect {
                        position: rect.position + step,
                        ..*rect
                    });
                }
            } else {
                let position = if rect.position > at {
                    rect.position + inserted
                } else {
                    rect.position
                };
                moved.push(GlyphRect { position, ..*rect });
            }
        }
        *rects = moved;
    }

    /// Flutter's `deleteBackward`: the selection, or the unit before the caret, the whole of an
    /// emoji, but one unit of another grapheme, as Thai input wants a vowel deleted alone.
    fn delete_before_caret(&self) {
        let ivars = self.ivars();
        ivars.selection_affinity.set(TextAffinity::Downstream);
        let mut selected = ivars.selected.get();
        if selected.length == 0 && ivars.text.length() > 0 && selected.location > 0 {
            let mut range = NSRange::new(selected.location - 1, 1);
            let character = ivars
                .text
                .rangeOfComposedCharacterSequenceAtIndex(selected.location - 1);
            if is_emoji(&ivars.text, character) {
                range = NSRange::new(character.location, selected.location - character.location);
            }
            selected = range;
            ivars.selected.set(range);
        }
        if selected.length == 0 {
            return;
        }
        if is_emoji(&ivars.text, selected) {
            let deleted = ivars.text.substringWithRange(selected);
            let first = deleted.rangeOfComposedCharacterSequenceAtIndex(0);
            ivars
                .temporarily_deleted_composed_character
                .replace(Some(deleted.substringWithRange(first)));
        }
        self.replace(selected, &NSString::new());
    }

    /// Flutter's `shouldChangeTextInRange:replacementText:`: a new line in a field whose return key
    /// is the default one is a new line the framework hears of as an action, and in any other
    /// field it is the return key's action alone.
    fn should_change_to(&self, text: &NSString) -> bool {
        self.ivars()
            .temporarily_deleted_composed_character
            .replace(None);
        if !text.isEqualToString(&NSString::from_str("\n")) {
            return true;
        }
        let return_key_type = self.ivars().traits.borrow().return_key_type;
        let view = self.ivars().view;
        if return_key_type == UIReturnKeyType::Default {
            deliver(HostEvent::TextInputAction(
                view,
                waft_embedder::TextInputAction::Newline,
            ));
            return true;
        }
        deliver(HostEvent::TextInputAction(view, action_of(return_key_type)));
        false
    }

    /// Flutter's `updateEditingState`: the field's value, as the framework is to hear it.
    fn update_editing_state(&self) {
        let ivars = self.ivars();
        let selected = ivars.selected.get();
        let composing = match ivars.marked.get() {
            Some(marked) => TextRange::new(
                marked.location as i32,
                (marked.location + marked.length) as i32,
            ),
            None => TextRange::EMPTY,
        };
        let value = TextEditingValue::new()
            .text(ivars.text.to_string())
            .selection(
                TextSelection::new(
                    selected.location as i32,
                    (selected.location + selected.length) as i32,
                )
                .affinity(ivars.selection_affinity.get()),
            )
            .composing(composing);
        deliver(HostEvent::TextEdited(ivars.view, value));
    }

    /// Flutter's `positionFromPosition:offset:`: `offset` graphemes on from `index`; `None` past
    /// either end of the text.
    fn moved_by_graphemes(&self, index: usize, offset: NSInteger) -> Option<usize> {
        let length = self.ivars().text.length();
        let target = index as NSInteger + offset;
        if target < 0 || target > length as NSInteger {
            return None;
        }
        let mut position = index;
        if offset >= 0 {
            for _ in 0..offset {
                if position >= length {
                    break;
                }
                position = self.increment(position);
            }
        } else {
            for _ in 0..offset.unsigned_abs() {
                if position == 0 {
                    break;
                }
                position = self.decrement(position);
            }
        }
        Some(position)
    }

    /// Flutter's `decrementOffsetPosition:`: the start of the grapheme before `position`.
    fn decrement(&self, position: usize) -> usize {
        self.character_range_at(position.saturating_sub(1)).location
    }

    /// Flutter's `incrementOffsetPosition:`: the end of the grapheme at `position`.
    fn increment(&self, position: usize) -> usize {
        let character = self.character_range_at(position);
        (position + character.length).min(self.ivars().text.length())
    }

    /// `fml::RangeForCharacterAtIndex`: the grapheme at `index`, or an empty range past the text.
    fn character_range_at(&self, index: usize) -> NSRange {
        let text = &self.ivars().text;
        if index >= text.length() {
            return NSRange::new(index, 0);
        }
        text.rangeOfComposedCharacterSequenceAtIndex(index)
    }

    /// Flutter's `firstRectForRange:`: the marked text's rect through the editable's transform
    /// while text is marked; else, on iOS 17 and later, the range's first line from the selection
    /// rects, and nothing before, as Flutter hides the iOS 16 highlight without Scribble.
    fn first_rect(&self, range: NSRange) -> CGRect {
        let ivars = self.ivars();
        if ivars.marked.get().is_some() {
            let Some(marked_rect) = ivars.marked_rect.get() else {
                return CGRect::ZERO;
            };
            // A caret rect the framework sent in place of the marked text's is widened, so the
            // candidates show.
            let rect = if marked_rect.width() <= 0.0 || marked_rect.height() <= 0.0 {
                Rect::from_ltrb(
                    marked_rect.left - 0.1,
                    marked_rect.top,
                    marked_rect.right + 0.1,
                    marked_rect.bottom,
                )
            } else {
                marked_rect
            };
            let Some(in_view) = rect_through(&ivars.editable_transform.get(), rect) else {
                return CGRect::ZERO;
            };
            let Some(host) = self.superview() else {
                return cg_rect(in_view);
            };
            return host.convertRect_toView(cg_rect(in_view), Some(self));
        }
        if !available!(ios = 17.0) {
            return CGRect::ZERO;
        }
        let rects = ivars.selection_rects.borrow();
        first_rect(
            &rects,
            range.location,
            range.location + range.length,
            ivars.text.length(),
        )
        .map_or(CGRect::ZERO, cg_rect)
    }

    /// Flutter's `caretRectForPosition:`, from the rects of the graphemes around `index`.
    fn caret_rect(&self, index: usize, affinity: UITextStorageDirection) -> CGRect {
        let length = self.ivars().text.length();
        let around = NSRange::new(index.saturating_sub(1), if index >= length { 1 } else { 2 });
        let around = self
            .ivars()
            .text
            .rangeOfComposedCharacterSequencesForRange(clamp_range(around, length));
        let rects: Vec<(Rect, bool)> = {
            let glyphs = self.ivars().selection_rects.borrow();
            self.rects_in(around)
                .into_iter()
                .map(|rect| {
                    let rtl = glyphs
                        .iter()
                        .find(|glyph| glyph.position == rect.position)
                        .is_some_and(|glyph| glyph.rtl);
                    (rect.rect, rtl)
                })
                .collect()
        };
        caret_rect(&rects, index, affinity == UITextStorageDirection::Forward)
            .map_or(CGRect::ZERO, cg_rect)
    }

    fn rects_in(&self, range: NSRange) -> Vec<super::geometry::RangeRect> {
        rects_in_range(
            &self.ivars().selection_rects.borrow(),
            range.location,
            range.location + range.length,
            self.ivars().text.length(),
        )
    }

    fn closest_within(&self, point: CGPoint, range: NSRange) -> Option<Retained<UITextPosition>> {
        let (index, upstream) = closest_position(
            &self.ivars().selection_rects.borrow(),
            Offset::new(point.x, point.y),
            range.location,
            range.location + range.length,
            1.0,
        )?;
        let affinity = if upstream {
            UITextStorageDirection::Backward
        } else {
            UITextStorageDirection::Forward
        };
        Some(self.position(index, affinity))
    }
}

/// The span between two offsets the framework sent, either way round; `None` for one of none.
fn span(base: i32, extent: i32) -> Option<NSRange> {
    let start = base.min(extent);
    if start < 0 {
        return None;
    }
    Some(NSRange::new(start as usize, base.abs_diff(extent) as usize))
}

/// Flutter's `clampSelection:forText:`, of a span that may be none.
fn clamp(range: Option<NSRange>, length: usize) -> Option<NSRange> {
    Some(clamp_range(range?, length))
}

/// Flutter's `clampSelection:forText:`: `range` within a text `length` units long.
fn clamp_range(range: NSRange, length: usize) -> NSRange {
    let start = range.location.min(length);
    NSRange::new(start, range.length.min(length - start))
}

fn cg_rect(rect: Rect) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.left, rect.top),
        CGSize::new(rect.width(), rect.height()),
    )
}

/// Whether `text` is one unit that is half of a surrogate pair, which has no UTF-8 of its own:
/// Flutter's `!text.UTF8String`.
fn is_lone_surrogate(text: &NSString) -> bool {
    (0xd800..=0xdfff).contains(&text.characterAtIndex(0))
}

/// Flutter's `IsEmoji`: whether the first code point of `range` of `text` has Unicode's Emoji
/// property, as ICU says, which iOS ships.
fn is_emoji(text: &NSString, range: NSRange) -> bool {
    if range.length == 0 || range.location >= text.length() {
        return false;
    }
    let lead = u32::from(text.characterAtIndex(range.location));
    let code_point = if (0xd800..=0xdbff).contains(&lead) && range.length > 1 {
        let trail = u32::from(text.characterAtIndex(range.location + 1));
        0x10000 + ((lead - 0xd800) << 10) + (trail.wrapping_sub(0xdc00) & 0x3ff)
    } else {
        lead
    };
    // SAFETY: ICU's property query, which reads only the integers it is handed.
    unsafe { u_hasBinaryProperty(code_point as i32, UCHAR_EMOJI) != 0 }
}

/// ICU's `UCHAR_EMOJI`.
const UCHAR_EMOJI: i32 = 57;

#[link(name = "icucore")]
unsafe extern "C" {
    fn u_hasBinaryProperty(c: i32, which: i32) -> i8;
}
