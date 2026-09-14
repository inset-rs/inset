//! winit's input as the framework takes it: the mouse as Flutter's one pointer, key
//! events as dart:ui [`KeyData`], IME as a text editing value, and the cursor the
//! framework asked for shown on the window the pointer is in.
//!
//! A key the framework declines is offered to the view's text input afterwards — the
//! order Flutter's macOS `FlutterKeyboardManager` keeps between the framework and its
//! text input plugin. The tables the physical and logical keys come from are `keys.rs`,
//! the editing value an IME event makes is `ime.rs`.

use std::collections::HashMap;
use std::time::Duration;

use inset_embedder::{
    EmbedderClient, KeyData, KeyEventDeviceType, KeyEventType, PointerChange,
    SystemMouseCursorKind, View,
};
use winit::event::{ElementState, Ime, KeyEvent, MouseScrollDelta};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, WindowId};

use crate::ime::{ImeOutcome, apply_ime};
use crate::keys;
use crate::os;
use crate::pointer::wheel_to_physical;
use crate::text_input::TextInputKeyEffect;
use crate::window::WinitApp;

/// The keyboard as Flutter's `KeyData`: the modifiers held, and which logical key each
/// held physical key went down with.
pub(crate) struct Keyboard {
    pub(crate) modifiers: ModifiersState,
    /// The logical key each held physical key went down with, keyed by USB HID usage.
    pressing_records: HashMap<u64, u64>,
}

impl Keyboard {
    pub(crate) fn new() -> Keyboard {
        Keyboard {
            modifiers: ModifiersState::default(),
            pressing_records: HashMap::new(),
        }
    }

    /// A winit key event as dart:ui [`KeyData`], or `None` for a key this host
    /// cannot name in Flutter's tables.
    fn key_data(
        &mut self,
        event: &KeyEvent,
        is_synthetic: bool,
        time_stamp: Duration,
    ) -> Option<KeyData> {
        let physical = keys::physical_key_usage(event.physical_key)?;
        let event_type = match (event.state, event.repeat) {
            (ElementState::Pressed, false) => KeyEventType::Down,
            (ElementState::Pressed, true) => KeyEventType::Repeat,
            (ElementState::Released, _) => KeyEventType::Up,
        };
        let logical = keys::logical_key_id(&event.logical_key, event.location);
        // Flutter's embedders remember which logical key a physical key went down
        // with, so its repeats and its up report that one even when the modifiers
        // changed in between (`_pressingRecords` in the engine's `KeyboardConverter`).
        let logical = match event_type {
            KeyEventType::Down => {
                let logical = logical?;
                self.pressing_records.insert(physical, logical);
                logical
            }
            KeyEventType::Repeat => self.pressing_records.get(&physical).copied().or(logical)?,
            KeyEventType::Up => self.pressing_records.remove(&physical).or(logical)?,
        };
        let character = match event_type {
            KeyEventType::Up => None,
            KeyEventType::Down | KeyEventType::Repeat => keys::character_of(event.text.as_deref()),
        };
        Some(KeyData {
            time_stamp,
            event_type,
            device_type: KeyEventDeviceType::Keyboard,
            physical,
            logical,
            character,
            synthesized: is_synthetic,
        })
    }
}

impl<C: EmbedderClient> WinitApp<C> {
    /// Tells the framework the mouse is present, once, before anything else about it.
    pub(crate) fn add_pointer(&mut self, window_id: WindowId) {
        if self.pointer.add() {
            self.send_pointer(window_id, PointerChange::Add);
        }
    }

    /// Sends one change of the pointer in a window, with a scroll's deltas when it is one.
    fn send_pointer_with(
        &mut self,
        window_id: WindowId,
        change: PointerChange,
        scroll: Option<[f64; 2]>,
    ) {
        let Some(view_id) = self.views.get(&window_id).map(|hosted| hosted.view.id()) else {
            return;
        };
        let packet = self
            .pointer
            .packet(view_id, change, scroll, self.platform.elapsed());
        if let Some(client) = &mut self.client {
            client.pointer_data_packet(packet);
        }
    }

    pub(crate) fn send_pointer(&mut self, window_id: WindowId, change: PointerChange) {
        self.send_pointer_with(window_id, change, None);
    }

    /// Sends the presses and releases the host missed while a native menu ran its own
    /// event loop, by what the system says is held down now.
    pub(crate) fn reconcile_buttons(&mut self, window_id: WindowId) {
        let Some(actual) = os::pressed_buttons() else {
            return;
        };
        for change in self.pointer.reconcile(actual) {
            self.send_pointer(window_id, change);
        }
    }

    pub(crate) fn send_scroll(&mut self, window_id: WindowId, delta: MouseScrollDelta) {
        let scale = self
            .views
            .get(&window_id)
            .map(|hosted| hosted.view.metrics().device_pixel_ratio)
            .unwrap_or(1.0);
        let scroll = wheel_to_physical(delta, scale);
        self.send_pointer_with(window_id, PointerChange::Hover, Some(scroll));
    }

    pub(crate) fn send_ime(&mut self, window_id: WindowId, ime: Ime) {
        let Some(hosted) = self.views.get(&window_id) else {
            return;
        };
        let view_id = hosted.view.id();
        let outcome = apply_ime(&hosted.view.editing_state.borrow(), &ime);
        match outcome {
            ImeOutcome::None => {}
            ImeOutcome::Closed => {
                if let Some(client) = &mut self.client {
                    client.text_input_closed(view_id);
                }
            }
            ImeOutcome::Value(value) => {
                *hosted.view.editing_state.borrow_mut() = value.clone();
                if let Some(client) = &mut self.client {
                    client.text_input_editing_value(view_id, value);
                }
            }
        }
    }

    pub(crate) fn send_key(&mut self, window_id: WindowId, event: &KeyEvent, is_synthetic: bool) {
        let Some(data) = self
            .keyboard
            .key_data(event, is_synthetic, self.platform.elapsed())
        else {
            return;
        };
        let Some(client) = &mut self.client else {
            return;
        };
        // Flutter's macOS `FlutterKeyboardManager`: the text input plugin sees a key only
        // after the framework declined it.
        if !client.key_data(data) && event.state == ElementState::Pressed {
            self.type_into_text_input(window_id, event);
        }
    }

    /// `FlutterTextInputPlugin.handleKeyEvent` for the view's active text input, if any.
    fn type_into_text_input(&mut self, window_id: WindowId, event: &KeyEvent) {
        let Some(hosted) = self.views.get(&window_id) else {
            return;
        };
        let Some(text_input) = hosted.view.text_input.get() else {
            return;
        };
        let view_id = hosted.view.id();
        match text_input.key_effect(
            &event.logical_key,
            event.text.as_deref(),
            self.keyboard.modifiers,
        ) {
            TextInputKeyEffect::None => {}
            TextInputKeyEffect::Insert(text) => self.send_ime(window_id, Ime::Commit(text)),
            TextInputKeyEffect::Enter { insert, action } => {
                if let Some(text) = insert {
                    self.send_ime(window_id, Ime::Commit(text));
                }
                if let Some(client) = &mut self.client {
                    client.text_input_action(view_id, action);
                }
            }
        }
    }

    pub(crate) fn apply_cursor_request(&mut self) {
        let Some(kind) = self.platform.cursor_request.take() else {
            return;
        };
        let Some(window) = self
            .pointer
            .window
            .and_then(|id| self.views.get(&id).map(|view| &view.window))
        else {
            return;
        };
        match cursor_icon_of(kind) {
            Some(icon) => {
                window.set_cursor(icon);
                window.set_cursor_visible(true);
            }
            None => window.set_cursor_visible(false),
        }
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
