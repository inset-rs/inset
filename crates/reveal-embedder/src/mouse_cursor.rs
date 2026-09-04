//! The system cursors a host can show. Flutter's engine receives these as `kind` strings on
//! `SystemChannels.mouseCursor` (`services/mouse_cursor.dart`); there is no channel here, so
//! the kind is an enum.

/// A system cursor kind (`SystemMouseCursor.kind`). The host maps each to its native cursor and
/// shows the basic arrow for one it lacks; [`None`](Self::None) hides the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemMouseCursorKind {
    None,
    Basic,
    Click,
    Forbidden,
    Wait,
    Progress,
    ContextMenu,
    Help,
    Text,
    VerticalText,
    Cell,
    Precise,
    Move,
    Grab,
    Grabbing,
    NoDrop,
    Alias,
    Copy,
    Disappearing,
    AllScroll,
    ResizeLeftRight,
    ResizeUpDown,
    ResizeUpLeftDownRight,
    ResizeUpRightDownLeft,
    ResizeUp,
    ResizeDown,
    ResizeLeft,
    ResizeRight,
    ResizeUpLeft,
    ResizeUpRight,
    ResizeDownLeft,
    ResizeDownRight,
    ResizeColumn,
    ResizeRow,
    ZoomIn,
    ZoomOut,
}
