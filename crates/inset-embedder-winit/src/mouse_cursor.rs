//! The framework's [`MouseCursor`]: the cursor it asks for goes to the event loop, which shows
//! it on the window the pointer is in.

use inset_embedder::{MouseCursor, SystemMouseCursorKind, ViewId};
use winit::window::{CursorIcon, Window};

use crate::WinitPlatform;
use crate::app_requests::AppRequest;

impl MouseCursor for WinitPlatform {
    /// One mouse: the window the pointer was last seen in shows the cursor, and the loop
    /// already knows that window, so neither the view nor the device is needed to pick it.
    fn activate_system_cursor(&self, _view: ViewId, _device: i64, kind: SystemMouseCursorKind) {
        self.send(AppRequest::ShowCursor(kind));
    }
}

/// Shows `kind` on `window`, the one the pointer was last seen in.
pub(crate) fn show(window: &Window, kind: SystemMouseCursorKind) {
    match cursor_icon_of(kind) {
        Some(icon) => {
            window.set_cursor(icon);
            window.set_cursor_visible(true);
        }
        None => window.set_cursor_visible(false),
    }
}

/// The winit icon for a system cursor kind; `None` hides the cursor
/// ([`SystemMouseCursorKind::None`]). Kinds winit lacks fall back to the default arrow, as
/// the Flutter engine falls back to `basic`.
fn cursor_icon_of(kind: SystemMouseCursorKind) -> Option<CursorIcon> {
    let icon = match kind {
        SystemMouseCursorKind::None => return None,
        SystemMouseCursorKind::Basic | SystemMouseCursorKind::Disappearing => CursorIcon::Default,
        SystemMouseCursorKind::Click => CursorIcon::Pointer,
        SystemMouseCursorKind::Forbidden => CursorIcon::NotAllowed,
        SystemMouseCursorKind::Wait => CursorIcon::Wait,
        SystemMouseCursorKind::Progress => CursorIcon::Progress,
        SystemMouseCursorKind::ContextMenu => CursorIcon::ContextMenu,
        SystemMouseCursorKind::Help => CursorIcon::Help,
        SystemMouseCursorKind::Text => CursorIcon::Text,
        SystemMouseCursorKind::VerticalText => CursorIcon::VerticalText,
        SystemMouseCursorKind::Cell => CursorIcon::Cell,
        SystemMouseCursorKind::Precise => CursorIcon::Crosshair,
        SystemMouseCursorKind::Move => CursorIcon::Move,
        SystemMouseCursorKind::Grab => CursorIcon::Grab,
        SystemMouseCursorKind::Grabbing => CursorIcon::Grabbing,
        SystemMouseCursorKind::NoDrop => CursorIcon::NoDrop,
        SystemMouseCursorKind::Alias => CursorIcon::Alias,
        SystemMouseCursorKind::Copy => CursorIcon::Copy,
        SystemMouseCursorKind::AllScroll => CursorIcon::AllScroll,
        SystemMouseCursorKind::ResizeLeftRight => CursorIcon::EwResize,
        SystemMouseCursorKind::ResizeUpDown => CursorIcon::NsResize,
        SystemMouseCursorKind::ResizeUpLeftDownRight => CursorIcon::NwseResize,
        SystemMouseCursorKind::ResizeUpRightDownLeft => CursorIcon::NeswResize,
        SystemMouseCursorKind::ResizeUp => CursorIcon::NResize,
        SystemMouseCursorKind::ResizeDown => CursorIcon::SResize,
        SystemMouseCursorKind::ResizeLeft => CursorIcon::WResize,
        SystemMouseCursorKind::ResizeRight => CursorIcon::EResize,
        SystemMouseCursorKind::ResizeUpLeft => CursorIcon::NwResize,
        SystemMouseCursorKind::ResizeUpRight => CursorIcon::NeResize,
        SystemMouseCursorKind::ResizeDownLeft => CursorIcon::SwResize,
        SystemMouseCursorKind::ResizeDownRight => CursorIcon::SeResize,
        SystemMouseCursorKind::ResizeColumn => CursorIcon::ColResize,
        SystemMouseCursorKind::ResizeRow => CursorIcon::RowResize,
        SystemMouseCursorKind::ZoomIn => CursorIcon::ZoomIn,
        SystemMouseCursorKind::ZoomOut => CursorIcon::ZoomOut,
    };
    Some(icon)
}

#[cfg(test)]
mod tests {
    use inset_embedder::SystemMouseCursorKind;
    use winit::window::CursorIcon;

    use super::cursor_icon_of;

    #[test]
    fn cursor_kinds_map_to_winit_icons() {
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Click),
            Some(CursorIcon::Pointer)
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Basic),
            Some(CursorIcon::Default)
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::None),
            None,
            "none hides the cursor"
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Disappearing),
            Some(CursorIcon::Default),
            "a kind winit lacks falls back to the arrow"
        );
    }
}
