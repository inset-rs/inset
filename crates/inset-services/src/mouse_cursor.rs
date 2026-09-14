//! Flutter counterpart: `services/mouse_cursor.dart`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use inset_embedder::{Platform, SystemMouseCursorKind};
use inset_gestures::PointerEvent;

/// A shared reference to an immutable [`MouseCursor`] (a Dart object reference).
pub type MouseCursorRef = Rc<dyn MouseCursor>;

/// Maintains the state of mouse cursors and manages how cursors are searched
/// for.
///
/// This is typically created as a global singleton and owned by `MouseTracker`.
pub struct MouseCursorManager {
    /// The mouse cursor to use if all cursor candidates choose to defer.
    ///
    /// See also:
    ///
    ///  * `MouseCursor.defer`, the mouse cursor object to use to defer.
    pub fallback_mouse_cursor: MouseCursorRef,
    last_session: HashMap<i64, Box<dyn MouseCursorSession>>,
}

impl MouseCursorManager {
    /// Create a [`MouseCursorManager`] by specifying the fallback cursor.
    ///
    /// The `fallback_mouse_cursor` must not be `MouseCursor.defer`.
    pub fn new(fallback_mouse_cursor: MouseCursorRef) -> MouseCursorManager {
        debug_assert!(!fallback_mouse_cursor.is_defer());
        MouseCursorManager {
            fallback_mouse_cursor,
            last_session: HashMap::new(),
        }
    }

    /// Returns the active mouse cursor of a device.
    ///
    /// The return value is the last [`MouseCursor`] activated onto this device,
    /// even if the activation failed.
    ///
    /// This function is only active in debug mode: in release it returns `None`.
    pub fn debug_device_active_cursor(&self, device: i64) -> Option<MouseCursorRef> {
        if !cfg!(debug_assertions) {
            return None;
        }
        self.last_session
            .get(&device)
            .map(|session| session.cursor())
    }

    /// Handles the changes that cause a pointer device to have a new list of
    /// mouse cursor candidates.
    ///
    /// This change can be caused by a pointer event, in which case
    /// `triggering_event` should not be `None`, or by other changes, such as when a
    /// widget has moved under a still mouse, which is detected after the current
    /// frame is complete. In either case, `cursor_candidates` should be the list
    /// of cursors at the location of the mouse in hit-test order.
    pub fn handle_device_cursor_update(
        &mut self,
        platform: &dyn Platform,
        device: i64,
        triggering_event: Option<&PointerEvent>,
        cursor_candidates: impl IntoIterator<Item = MouseCursorRef>,
    ) {
        if let Some(PointerEvent::Removed(_)) = triggering_event {
            self.last_session.remove(&device);
            return;
        }
        let next_cursor = first_non_deferred(cursor_candidates)
            .unwrap_or_else(|| Rc::clone(&self.fallback_mouse_cursor));
        debug_assert!(!next_cursor.is_defer());
        if let Some(last_session) = self.last_session.get(&device)
            && *last_session.cursor() == *next_cursor
        {
            return;
        }
        let mut next_session = next_cursor.create_session(device);
        if let Some(mut last_session) = self.last_session.remove(&device) {
            last_session.dispose();
        }
        next_session.activate(platform);
        self.last_session.insert(device, next_session);
    }
}

/// Manages the duration that a pointing device should display a specific mouse
/// cursor.
///
/// While [`MouseCursor`] classes describe the kind of cursors, [`MouseCursorSession`]
/// classes represent a continuous use of the cursor on a pointing device. The
/// [`MouseCursorSession`] classes can be stateful. For example, a cursor that
/// needs to load resources might want to try a fallback cursor first, and switch
/// to the target cursor when the resources are loaded.
///
/// A [`MouseCursorSession`] has the following lifecycle:
///
///  * When a pointing device should start displaying a cursor, `MouseTracker`
///    creates a session by calling [`MouseCursor::create_session`] on the target
///    cursor, and stores it in a table associated with the device.
///  * `MouseTracker` then immediately calls the session's [`activate`], where
///    the session should fetch resources and make system calls.
///  * When the pointing device should start displaying another cursor,
///    `MouseTracker` calls [`dispose`] on this session. After [`dispose`], this
///    session will no longer be used in the future.
///
/// [`activate`]: MouseCursorSession::activate
/// [`dispose`]: MouseCursorSession::dispose
pub trait MouseCursorSession {
    /// The cursor that created this session.
    fn cursor(&self) -> MouseCursorRef;

    /// The device ID of the pointing device.
    fn device(&self) -> i64;

    /// Override this method to do the work of changing the cursor of the device.
    ///
    /// Called right after this session is created.
    ///
    /// This method has full control over the cursor until the [`dispose`] call,
    /// and can make system calls to change the pointer cursor as many times as
    /// necessary (usually through `platform`'s system cursor request).
    ///
    /// [`dispose`]: MouseCursorSession::dispose
    fn activate(&mut self, platform: &dyn Platform);

    /// Called when device stops displaying the cursor.
    ///
    /// After this call, this session instance will no longer be used in the
    /// future.
    ///
    /// When implementing this method in subclasses, you should release resources
    /// and prevent [`activate`] from causing side effects after disposal.
    ///
    /// [`activate`]: MouseCursorSession::activate
    fn dispose(&mut self);
}

/// An interface for mouse cursor definitions.
///
/// A mouse cursor is a graphical image on the screen that echoes the movement
/// of a pointing device, such as a mouse or a stylus. A [`MouseCursor`] object
/// defines a kind of mouse cursor, such as an arrow, a pointing hand, or an
/// I-beam.
///
/// During the painting phase, [`MouseCursor`] objects are assigned to regions on
/// the screen via annotations. Later during a device update (e.g. when a mouse
/// moves), `MouseTracker` finds the _active cursor_, the top-most region
/// associated with the position of each mouse device, and activates the cursor
/// as necessary.
///
/// ## Cursor classes
///
/// A [`SystemMouseCursor`] is a cursor that is natively supported by the
/// platform that the program is running on. All supported system mouse cursors
/// are enumerated in [`SystemMouseCursors`].
///
/// ## Using cursors
///
/// A [`MouseCursor`] object is used by being assigned to a `MouseRegion` or
/// another widget that exposes the `MouseRegion` API, such as
/// `InkResponse.mouseCursor`.
///
/// ## Related classes
///
/// [`MouseCursorSession`] represents the duration when a pointing device
/// displays a cursor, and defines the states and behaviors of the cursor.
///
/// [`MouseCursorManager`] is a mixin that manages the states of mouse cursors,
/// such as `MouseTracker`.
///
/// Values are shared as [`MouseCursorRef`]. Dart's `MouseCursor.defer` and
/// `MouseCursor.uncontrolled` are `<dyn MouseCursor>::defer()` and
/// `<dyn MouseCursor>::uncontrolled()`.
pub trait MouseCursor: fmt::Debug {
    /// Associate a pointing device to this cursor.
    ///
    /// A mouse cursor class usually has a corresponding [`MouseCursorSession`]
    /// class, and instantiates such class in this method.
    ///
    /// This method is called each time a pointing device starts displaying this
    /// cursor. A given cursor can be displayed by multiple devices at the same
    /// time, in which case this method will be called separately for each device.
    fn create_session(&self, device: i64) -> Box<dyn MouseCursorSession>;

    /// A very short description of the mouse cursor.
    ///
    /// The `debug_description` should be a few words that can describe this cursor
    /// to make debug information more readable. It is returned as the `Debug`
    /// output when the diagnostic level is at or above info.
    fn debug_description(&self) -> String;

    /// Downcast support for Dart's `==` between cursor classes.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`. Identity unless a class overrides it, as [`SystemMouseCursor`] does.
    fn eq_cursor(&self, other: &dyn MouseCursor) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn MouseCursor)
    }

    /// Whether this is `MouseCursor.defer`.
    fn is_defer(&self) -> bool {
        self.as_any().is::<DeferringMouseCursor>()
    }
}

impl dyn MouseCursor {
    /// A special class that indicates that the region with this cursor defers the
    /// choice of cursor to the next region behind it.
    ///
    /// When an event occurs, `MouseTracker` will update each pointer's cursor by
    /// finding the list of regions that contain the pointer's location, from front
    /// to back in hit-test order. The pointer's cursor will be the first cursor
    /// that is not deferred.
    pub fn defer() -> MouseCursorRef {
        Rc::new(DeferringMouseCursor)
    }

    /// A special value that doesn't change cursor by itself, but make a region
    /// that blocks other regions behind it from changing the cursor.
    ///
    /// When a pointer enters a region with a cursor of `uncontrolled`, the pointer
    /// retains its previous cursor and keeps so until it moves out of the region.
    /// Technically, this region absorb the mouse cursor hit test without changing
    /// the pointer's cursor.
    ///
    /// This is useful in a region that displays a platform view, which let the
    /// operating system handle pointer events and change cursors accordingly. To
    /// achieve this, the region's cursor must not be any Flutter cursor, since
    /// that might overwrite the system request upon pointer entering; the cursor
    /// must not be `None` either, since that allows the widgets behind the region to
    /// change cursors.
    pub fn uncontrolled() -> MouseCursorRef {
        Rc::new(NoopMouseCursor)
    }
}

impl PartialEq for dyn MouseCursor {
    fn eq(&self, other: &dyn MouseCursor) -> bool {
        self.eq_cursor(other)
    }
}

/// The cursor behind `MouseCursor.defer`. Every instance is equal.
#[derive(Debug)]
pub struct DeferringMouseCursor;

impl MouseCursor for DeferringMouseCursor {
    fn create_session(&self, _device: i64) -> Box<dyn MouseCursorSession> {
        panic!("DeferringMouseCursor can not create a session");
    }

    fn debug_description(&self) -> String {
        "defer".to_owned()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_cursor(&self, other: &dyn MouseCursor) -> bool {
        other.is_defer()
    }
}

/// Returns the first cursor from the given cursors that is not a
/// [`DeferringMouseCursor`], or `None` if none is found.
fn first_non_deferred(cursors: impl IntoIterator<Item = MouseCursorRef>) -> Option<MouseCursorRef> {
    cursors.into_iter().find(|cursor| !cursor.is_defer())
}

struct NoopMouseCursorSession {
    device: i64,
}

impl MouseCursorSession for NoopMouseCursorSession {
    fn cursor(&self) -> MouseCursorRef {
        <dyn MouseCursor>::uncontrolled()
    }

    fn device(&self) -> i64 {
        self.device
    }

    fn activate(&mut self, _platform: &dyn Platform) {
        // Nothing.
    }

    fn dispose(&mut self) {
        // Nothing.
    }
}

/// The cursor behind `MouseCursor.uncontrolled`. Every instance is equal.
#[derive(Debug)]
pub struct NoopMouseCursor;

impl MouseCursor for NoopMouseCursor {
    fn create_session(&self, device: i64) -> Box<dyn MouseCursorSession> {
        Box::new(NoopMouseCursorSession { device })
    }

    fn debug_description(&self) -> String {
        "uncontrolled".to_owned()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_cursor(&self, other: &dyn MouseCursor) -> bool {
        other.as_any().is::<NoopMouseCursor>()
    }
}

struct SystemMouseCursorSession {
    cursor: SystemMouseCursor,
    device: i64,
}

impl MouseCursorSession for SystemMouseCursorSession {
    fn cursor(&self) -> MouseCursorRef {
        Rc::new(self.cursor)
    }

    fn device(&self) -> i64 {
        self.device
    }

    fn activate(&mut self, platform: &dyn Platform) {
        if let Some(cursor) = platform.mouse_cursor() {
            cursor.activate_system_cursor(self.device, self.cursor.kind());
        }
    }

    fn dispose(&mut self) {
        // Nothing.
    }
}

/// A mouse cursor that is natively supported on the platform that the
/// application is running on.
///
/// System cursors can be used without external resources, and their appearances
/// match the experience of native apps. Examples of system cursors are a
/// pointing arrow, a pointing hand, a double arrow for resizing, or a text
/// I-beam, etc.
///
/// An instance of [`SystemMouseCursor`] refers to one cursor from each platform
/// that represents the same concept, such as being text, being clickable, or
/// being a forbidden operation. Since the set of system cursors supported by each
/// platform varies, multiple instances can correspond to the same system cursor.
///
/// Each cursor is noted with its corresponding native cursors on each platform:
///
///  * Android: API name in Java
///  * Web: CSS cursor
///  * Windows: Win32 API
///  * Linux: GDK, `gdk_cursor_new_from_name`
///  * macOS: API name in ObjectiveC
///
/// If the platform that the application is running on is not listed for a
/// cursor, using this cursor falls back to [`SystemMouseCursors::BASIC`].
///
/// [`SystemMouseCursors`] enumerates the complete set of system cursors supported
/// by Flutter, which are hard-coded in the engine. Therefore, manually
/// instantiating this class is not supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SystemMouseCursor {
    kind: SystemMouseCursorKind,
}

impl SystemMouseCursor {
    const fn new(kind: SystemMouseCursorKind) -> SystemMouseCursor {
        SystemMouseCursor { kind }
    }

    /// The kind of the cursor.
    ///
    /// The interpretation of `kind` is platform-dependent.
    pub fn kind(&self) -> SystemMouseCursorKind {
        self.kind
    }
}

impl MouseCursor for SystemMouseCursor {
    fn create_session(&self, device: i64) -> Box<dyn MouseCursorSession> {
        Box::new(SystemMouseCursorSession {
            cursor: *self,
            device,
        })
    }

    fn debug_description(&self) -> String {
        format!("SystemMouseCursor({:?})", self.kind)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_cursor(&self, other: &dyn MouseCursor) -> bool {
        other
            .as_any()
            .downcast_ref::<SystemMouseCursor>()
            .is_some_and(|other| other.kind == self.kind)
    }
}

impl From<SystemMouseCursor> for MouseCursorRef {
    fn from(cursor: SystemMouseCursor) -> MouseCursorRef {
        Rc::new(cursor)
    }
}

/// A collection of system [`MouseCursor`]s.
///
/// System cursors are standard mouse cursors that are provided by the current
/// platform. They don't require external resources.
///
/// [`SystemMouseCursors`] is a superset of the system cursors of every platform
/// that Flutter supports, therefore some of these objects might map to the same
/// result, or fallback to the [`BASIC`](Self::BASIC) arrow. This mapping is defined
/// by the Flutter engine.
///
/// The cursors should be named based on the cursors' use cases instead of their
/// appearance, because different platforms might (although not commonly) use
/// different shapes for the same use case.
pub struct SystemMouseCursors;

impl SystemMouseCursors {
    /// Hide the cursor.
    ///
    /// Any cursor other than [`NONE`](Self::NONE) or `MouseCursor.uncontrolled`
    /// unhides the cursor.
    pub const NONE: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::None);

    /// The platform-dependent basic cursor.
    ///
    /// Typically the shape of an arrow.
    pub const BASIC: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Basic);

    /// A cursor that emphasizes an element being clickable, such as a hyperlink.
    ///
    /// Typically the shape of a pointing hand.
    pub const CLICK: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Click);

    /// A cursor indicating an operation that will not be carried out.
    ///
    /// Typically the shape of a circle with a diagonal line. May fall back to
    /// [`NO_DROP`](Self::NO_DROP).
    pub const FORBIDDEN: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::Forbidden);

    /// A cursor indicating the status that the program is busy and therefore
    /// can not be interacted with.
    ///
    /// Typically the shape of an hourglass or a watch.
    ///
    /// This cursor is not available as a system cursor on macOS. Although macOS
    /// displays a "spinning ball" cursor when busy, it's handled by the OS and not
    /// exposed for applications to choose.
    pub const WAIT: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Wait);

    /// A cursor indicating the status that the program is busy but can still be
    /// interacted with.
    ///
    /// Typically the shape of an arrow with an hourglass or a watch at the
    /// corner. Does *not* fall back to [`WAIT`](Self::WAIT) if unavailable.
    pub const PROGRESS: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Progress);

    /// A cursor indicating somewhere the user can trigger a context menu.
    ///
    /// Typically the shape of an arrow with a small menu at the corner.
    pub const CONTEXT_MENU: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ContextMenu);

    /// A cursor indicating help information.
    ///
    /// Typically the shape of a question mark, or an arrow therewith.
    pub const HELP: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Help);

    /// A cursor indicating selectable text.
    ///
    /// Typically the shape of a capital I.
    pub const TEXT: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Text);

    /// A cursor indicating selectable vertical text.
    ///
    /// Typically the shape of a capital I rotated to be horizontal. May fall
    /// back to [`TEXT`](Self::TEXT).
    pub const VERTICAL_TEXT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::VerticalText);

    /// A cursor indicating selectable table cells.
    ///
    /// Typically the shape of a hollow plus sign.
    pub const CELL: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Cell);

    /// A cursor indicating precise selection, such as selecting a pixel in a
    /// bitmap.
    ///
    /// Typically the shape of a crosshair.
    pub const PRECISE: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Precise);

    /// A cursor indicating moving something.
    ///
    /// Typically the shape of four-way arrow. May fall back to
    /// [`ALL_SCROLL`](Self::ALL_SCROLL).
    pub const MOVE: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Move);

    /// A cursor indicating something that can be dragged.
    ///
    /// Typically the shape of an open hand.
    pub const GRAB: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Grab);

    /// A cursor indicating something that is being dragged.
    ///
    /// Typically the shape of a closed hand.
    pub const GRABBING: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Grabbing);

    /// A cursor indicating somewhere that the current item may not be dropped.
    ///
    /// Typically the shape of a hand with a [`FORBIDDEN`](Self::FORBIDDEN) sign at
    /// the corner. May fall back to [`FORBIDDEN`](Self::FORBIDDEN).
    pub const NO_DROP: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::NoDrop);

    /// A cursor indicating that the current operation will create an alias of, or
    /// a shortcut of the item.
    ///
    /// Typically the shape of an arrow with a shortcut icon at the corner.
    pub const ALIAS: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Alias);

    /// A cursor indicating that the current operation will copy the item.
    ///
    /// Typically the shape of an arrow with a boxed plus sign at the corner.
    pub const COPY: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::Copy);

    /// A cursor indicating that the current operation will result in the
    /// disappearance of the item.
    ///
    /// Typically the shape of an arrow with a cloud of smoke at the corner.
    pub const DISAPPEARING: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::Disappearing);

    /// A cursor indicating scrolling in any direction.
    ///
    /// Typically the shape of a dot surrounded by 4 arrows.
    pub const ALL_SCROLL: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::AllScroll);

    /// A cursor indicating resizing an object bidirectionally from its left or
    /// right edge.
    ///
    /// Typically the shape of a bidirectional arrow pointing left and right.
    pub const RESIZE_LEFT_RIGHT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeLeftRight);

    /// A cursor indicating resizing an object bidirectionally from its top or
    /// bottom edge.
    ///
    /// Typically the shape of a bidirectional arrow pointing up and down.
    pub const RESIZE_UP_DOWN: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUpDown);

    /// A cursor indicating resizing an object bidirectionally from its top left or
    /// bottom right corner.
    ///
    /// Typically the shape of a bidirectional arrow pointing upper left and lower
    /// right.
    pub const RESIZE_UP_LEFT_DOWN_RIGHT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUpLeftDownRight);

    /// A cursor indicating resizing an object bidirectionally from its top right or
    /// bottom left corner.
    ///
    /// Typically the shape of a bidirectional arrow pointing upper right and lower
    /// left.
    pub const RESIZE_UP_RIGHT_DOWN_LEFT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUpRightDownLeft);

    /// A cursor indicating resizing an object from its top edge.
    ///
    /// Typically the shape of an arrow pointing up. May fallback to
    /// [`RESIZE_UP_DOWN`](Self::RESIZE_UP_DOWN).
    pub const RESIZE_UP: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUp);

    /// A cursor indicating resizing an object from its bottom edge.
    ///
    /// Typically the shape of an arrow pointing down. May fallback to
    /// [`RESIZE_UP_DOWN`](Self::RESIZE_UP_DOWN).
    pub const RESIZE_DOWN: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeDown);

    /// A cursor indicating resizing an object from its left edge.
    ///
    /// Typically the shape of an arrow pointing left. May fallback to
    /// [`RESIZE_LEFT_RIGHT`](Self::RESIZE_LEFT_RIGHT).
    pub const RESIZE_LEFT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeLeft);

    /// A cursor indicating resizing an object from its right edge.
    ///
    /// Typically the shape of an arrow pointing right. May fallback to
    /// [`RESIZE_LEFT_RIGHT`](Self::RESIZE_LEFT_RIGHT).
    pub const RESIZE_RIGHT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeRight);

    /// A cursor indicating resizing an object from its top-left corner.
    ///
    /// Typically the shape of an arrow pointing upper left. May fallback to
    /// [`RESIZE_UP_LEFT_DOWN_RIGHT`](Self::RESIZE_UP_LEFT_DOWN_RIGHT).
    pub const RESIZE_UP_LEFT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUpLeft);

    /// A cursor indicating resizing an object from its top-right corner.
    ///
    /// Typically the shape of an arrow pointing upper right. May fallback to
    /// [`RESIZE_UP_RIGHT_DOWN_LEFT`](Self::RESIZE_UP_RIGHT_DOWN_LEFT).
    pub const RESIZE_UP_RIGHT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeUpRight);

    /// A cursor indicating resizing an object from its bottom-left corner.
    ///
    /// Typically the shape of an arrow pointing lower left. May fallback to
    /// [`RESIZE_UP_RIGHT_DOWN_LEFT`](Self::RESIZE_UP_RIGHT_DOWN_LEFT).
    pub const RESIZE_DOWN_LEFT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeDownLeft);

    /// A cursor indicating resizing an object from its bottom-right corner.
    ///
    /// Typically the shape of an arrow pointing lower right. May fallback to
    /// [`RESIZE_UP_LEFT_DOWN_RIGHT`](Self::RESIZE_UP_LEFT_DOWN_RIGHT).
    pub const RESIZE_DOWN_RIGHT: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeDownRight);

    /// A cursor indicating resizing a column, or an item horizontally.
    ///
    /// Typically the shape of arrows pointing left and right with a vertical bar
    /// separating them. May fallback to [`RESIZE_LEFT_RIGHT`](Self::RESIZE_LEFT_RIGHT).
    pub const RESIZE_COLUMN: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeColumn);

    /// A cursor indicating resizing a row, or an item vertically.
    ///
    /// Typically the shape of arrows pointing up and down with a horizontal bar
    /// separating them. May fallback to [`RESIZE_UP_DOWN`](Self::RESIZE_UP_DOWN).
    pub const RESIZE_ROW: SystemMouseCursor =
        SystemMouseCursor::new(SystemMouseCursorKind::ResizeRow);

    /// A cursor indicating zooming in.
    ///
    /// Typically a magnifying glass with a plus sign.
    pub const ZOOM_IN: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::ZoomIn);

    /// A cursor indicating zooming out.
    ///
    /// Typically a magnifying glass with a minus sign.
    pub const ZOOM_OUT: SystemMouseCursor = SystemMouseCursor::new(SystemMouseCursorKind::ZoomOut);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use inset_embedder::{InertPlatform, MouseCursor as MouseCursorHost};
    use inset_gestures::{PointerHoverEvent, PointerRemovedEvent};

    use super::*;

    /// A host that records the cursor requests it receives.
    struct RecordingPlatform {
        activated: RefCell<Vec<(i64, SystemMouseCursorKind)>>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> inset_embedder::TargetPlatform {
            InertPlatform.target_platform()
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            InertPlatform.now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<inset_embedder::ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: inset_embedder::ViewId) -> Option<inset_embedder::ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<inset_embedder::ViewRef> {
            None
        }

        fn mouse_cursor(&self) -> Option<&dyn MouseCursorHost> {
            Some(self)
        }
    }

    impl MouseCursorHost for RecordingPlatform {
        fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
            self.activated.borrow_mut().push((device, kind));
        }
    }

    fn recording_platform() -> RecordingPlatform {
        RecordingPlatform {
            activated: RefCell::new(Vec::new()),
        }
    }

    fn hover() -> PointerEvent {
        PointerEvent::Hover(PointerHoverEvent::default())
    }

    #[test]
    fn system_cursors_compare_by_kind_and_special_cursors_by_class() {
        let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
        let click_again: MouseCursorRef = SystemMouseCursors::CLICK.into();
        let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
        assert!(*click == *click_again);
        assert!(*click != *basic);
        assert!(*<dyn MouseCursor>::defer() == *<dyn MouseCursor>::defer());
        assert!(*<dyn MouseCursor>::uncontrolled() == *<dyn MouseCursor>::uncontrolled());
        assert!(*<dyn MouseCursor>::defer() != *<dyn MouseCursor>::uncontrolled());
        assert!(*<dyn MouseCursor>::defer() != *click);
        assert_eq!(click.debug_description(), "SystemMouseCursor(Click)");
        assert_eq!(<dyn MouseCursor>::defer().debug_description(), "defer");
    }

    #[test]
    fn manager_activates_the_first_non_deferred_candidate() {
        let platform = recording_platform();
        let mut manager = MouseCursorManager::new(SystemMouseCursors::BASIC.into());
        manager.handle_device_cursor_update(
            &platform,
            1,
            Some(&hover()),
            [<dyn MouseCursor>::defer(), SystemMouseCursors::TEXT.into()],
        );
        assert_eq!(
            *platform.activated.borrow(),
            [(1, SystemMouseCursorKind::Text)],
            "the deferring region lets the text region behind it decide"
        );
        let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
        assert!(*manager.debug_device_active_cursor(1).unwrap() == *text);
    }

    #[test]
    fn manager_falls_back_when_every_candidate_defers() {
        let platform = recording_platform();
        let mut manager = MouseCursorManager::new(SystemMouseCursors::BASIC.into());
        manager.handle_device_cursor_update(
            &platform,
            1,
            Some(&hover()),
            [<dyn MouseCursor>::defer()],
        );
        assert_eq!(
            *platform.activated.borrow(),
            [(1, SystemMouseCursorKind::Basic)]
        );
    }

    #[test]
    fn manager_does_not_reactivate_the_same_cursor() {
        let platform = recording_platform();
        let mut manager = MouseCursorManager::new(SystemMouseCursors::BASIC.into());
        manager.handle_device_cursor_update(
            &platform,
            1,
            Some(&hover()),
            [SystemMouseCursors::CLICK.into()],
        );
        manager.handle_device_cursor_update(&platform, 1, None, [SystemMouseCursors::CLICK.into()]);
        assert_eq!(platform.activated.borrow().len(), 1);

        manager.handle_device_cursor_update(&platform, 1, None, [SystemMouseCursors::GRAB.into()]);
        assert_eq!(platform.activated.borrow().len(), 2);
    }

    #[test]
    fn an_uncontrolled_region_keeps_the_previous_system_request() {
        let platform = recording_platform();
        let mut manager = MouseCursorManager::new(SystemMouseCursors::BASIC.into());
        manager.handle_device_cursor_update(
            &platform,
            1,
            Some(&hover()),
            [SystemMouseCursors::CLICK.into()],
        );
        manager.handle_device_cursor_update(
            &platform,
            1,
            None,
            [<dyn MouseCursor>::uncontrolled()],
        );
        assert_eq!(
            *platform.activated.borrow(),
            [(1, SystemMouseCursorKind::Click)],
            "the no-op session makes no request; the host keeps whatever it shows"
        );
        assert!(
            *manager.debug_device_active_cursor(1).unwrap() == *<dyn MouseCursor>::uncontrolled()
        );
    }

    #[test]
    fn a_removed_device_forgets_its_session() {
        let platform = recording_platform();
        let mut manager = MouseCursorManager::new(SystemMouseCursors::BASIC.into());
        manager.handle_device_cursor_update(
            &platform,
            1,
            Some(&hover()),
            [SystemMouseCursors::CLICK.into()],
        );
        let removed = PointerEvent::Removed(PointerRemovedEvent::default());
        manager.handle_device_cursor_update(&platform, 1, Some(&removed), []);
        assert!(manager.debug_device_active_cursor(1).is_none());
    }
}
