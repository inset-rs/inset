//! The positions, ranges and selection rects the field hands UIKit, as Flutter's
//! `FlutterTextPosition`, `FlutterTextRange` and `FlutterTextSelectionRect` are: a position is a
//! UTF-16 offset with the direction it leans, a range a span of them, both immutable, so a copy is
//! the same object.

// UIKit subclasses need `unsafe` to define and initialise; this module writes it for them.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, NSZone};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSCopying, NSRange};
use objc2_ui_kit::{
    NSWritingDirection, UITextPosition, UITextRange, UITextSelectionRect, UITextStorageDirection,
};

pub(crate) struct Position {
    index: usize,
    affinity: UITextStorageDirection,
}

define_class!(
    // SAFETY: UITextPosition has no subclassing requirements, and `WaftTextPosition` has no `Drop`.
    #[unsafe(super(UITextPosition))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftTextPosition"]
    #[ivars = Position]
    pub(crate) struct WaftTextPosition;

    unsafe impl NSObjectProtocol for WaftTextPosition {}
);

impl WaftTextPosition {
    /// The position at `index`, leaning `affinity`'s way.
    pub(crate) fn new(
        main_thread: MainThreadMarker,
        index: usize,
        affinity: UITextStorageDirection,
    ) -> Retained<WaftTextPosition> {
        let this = main_thread.alloc().set_ivars(Position { index, affinity });
        // SAFETY: plain initialisation of an allocated position.
        unsafe { msg_send![super(this), init] }
    }

    pub(crate) fn index(&self) -> usize {
        self.ivars().index
    }

    pub(crate) fn affinity(&self) -> UITextStorageDirection {
        self.ivars().affinity
    }
}

/// `position`'s offset and lean, where it is the field's; UIKit hands some keyboards' positions of
/// its own, which have none.
pub(crate) fn position_of(position: &UITextPosition) -> Option<&WaftTextPosition> {
    position.downcast_ref::<WaftTextPosition>()
}

define_class!(
    // SAFETY: UITextRange has no subclassing requirements, and `WaftTextRange` has no `Drop`.
    #[unsafe(super(UITextRange))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftTextRange"]
    #[ivars = NSRange]
    pub(crate) struct WaftTextRange;

    unsafe impl NSObjectProtocol for WaftTextRange {}

    unsafe impl NSCopying for WaftTextRange {
        #[unsafe(method_id(copyWithZone:))]
        fn copy_with_zone(&self, _zone: *const NSZone) -> Retained<Self> {
            self.copy_with_zone_now(_zone)
        }
    }

    impl WaftTextRange {
        #[unsafe(method_id(start))]
        fn start(&self) -> Retained<UITextPosition> {
            self.start_now()
        }

        #[unsafe(method_id(end))]
        fn end(&self) -> Retained<UITextPosition> {
            self.end_now()
        }

        #[unsafe(method(isEmpty))]
        fn is_empty(&self) -> bool {
            self.ivars().length == 0
        }
    }
);

impl WaftTextRange {
    fn copy_with_zone_now(&self, _zone: *const NSZone) -> Retained<Self> {
        self.retain()
    }

    fn start_now(&self) -> Retained<UITextPosition> {
        let range = *self.ivars();
        Retained::into_super(WaftTextPosition::new(
            self.mtm(),
            range.location,
            UITextStorageDirection::Forward,
        ))
    }

    fn end_now(&self) -> Retained<UITextPosition> {
        let range = *self.ivars();
        Retained::into_super(WaftTextPosition::new(
            self.mtm(),
            range.location + range.length,
            UITextStorageDirection::Backward,
        ))
    }
}

impl WaftTextRange {
    pub(crate) fn new(main_thread: MainThreadMarker, range: NSRange) -> Retained<WaftTextRange> {
        let this = main_thread.alloc().set_ivars(range);
        // SAFETY: plain initialisation of an allocated range.
        unsafe { msg_send![super(this), init] }
    }

    pub(crate) fn range(&self) -> NSRange {
        *self.ivars()
    }
}

/// `range`'s span, where it is the field's.
pub(crate) fn range_of(range: &UITextRange) -> Option<NSRange> {
    Some(range.downcast_ref::<WaftTextRange>()?.range())
}

pub(crate) struct SelectionRect {
    rect: CGRect,
    writing_direction: NSWritingDirection,
    contains_start: bool,
    contains_end: bool,
}

define_class!(
    // SAFETY: UITextSelectionRect has no subclassing requirements, and `WaftTextSelectionRect` has
    // no `Drop`.
    #[unsafe(super(UITextSelectionRect))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftTextSelectionRect"]
    #[ivars = SelectionRect]
    pub(crate) struct WaftTextSelectionRect;

    unsafe impl NSObjectProtocol for WaftTextSelectionRect {}

    impl WaftTextSelectionRect {
        #[unsafe(method(rect))]
        fn rect(&self) -> CGRect {
            self.ivars().rect
        }

        #[unsafe(method(writingDirection))]
        fn writing_direction(&self) -> NSWritingDirection {
            self.ivars().writing_direction
        }

        #[unsafe(method(containsStart))]
        fn contains_start(&self) -> bool {
            self.ivars().contains_start
        }

        #[unsafe(method(containsEnd))]
        fn contains_end(&self) -> bool {
            self.ivars().contains_end
        }

        #[unsafe(method(isVertical))]
        fn is_vertical(&self) -> bool {
            false
        }
    }
);

impl WaftTextSelectionRect {
    pub(crate) fn new(
        main_thread: MainThreadMarker,
        rect: CGRect,
        contains_start: bool,
        contains_end: bool,
    ) -> Retained<WaftTextSelectionRect> {
        let this = main_thread.alloc().set_ivars(SelectionRect {
            rect,
            writing_direction: NSWritingDirection::Natural,
            contains_start,
            contains_end,
        });
        // SAFETY: plain initialisation of an allocated selection rect.
        unsafe { msg_send![super(this), init] }
    }
}
