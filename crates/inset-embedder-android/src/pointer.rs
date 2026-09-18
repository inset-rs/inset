//! Android's pointers as Flutter's, following the engine's `AndroidTouchProcessor`.
//!
//! The rules here take plain numbers rather than the platform's own types, so the tables
//! are checked on any machine; `input` walks a real `MotionEvent` through them.

use inset_embedder::PointerDeviceKind;

/// `MotionEvent.TOOL_TYPE_*`. `UNKNOWN` is the zero the platform gives a tool it cannot
/// name, and every other number falls to the same answer, so nothing names it.
pub mod tool {
    pub const FINGER: u32 = 1;
    pub const STYLUS: u32 = 2;
    pub const MOUSE: u32 = 3;
    pub const ERASER: u32 = 4;
}

/// Flutter's `kFlutterPointerButtonMouse*`, which are also Android's `BUTTON_*` for the
/// five a mouse has, so a mouse's state needs no translation beyond the mask.
const MOUSE_BUTTONS: i64 = 0x1F;
/// `MotionEvent.BUTTON_STYLUS_PRIMARY`, the first bit above the mouse's.
const STYLUS_BUTTON_SHIFT: u32 = 4;
/// The two bits a stylus has, once shifted down.
const STYLUS_BUTTONS: i64 = 0xF;

/// Pixels a wheel notch scrolls where the platform names no measure of its own, which is
/// the engine's `DEFAULT_*_SCROLL_FACTOR`.
const SCROLL_PIXELS_PER_NOTCH: f64 = 48.0;

/// How many bits of a device number the tool type takes, so that one finger and one stylus
/// reporting the same pointer id are two devices. The engine does the same.
const TOOL_TYPE_BITS: u32 = 3;
const TOOL_TYPE_MASK: i64 = (1 << TOOL_TYPE_BITS) - 1;

/// The device a pointer is, told apart by what is touching as well as by its id.
pub fn device_id(pointer_id: i32, tool_type: u32) -> i64 {
    (i64::from(pointer_id) << TOOL_TYPE_BITS) | (i64::from(tool_type) & TOOL_TYPE_MASK)
}

/// What is touching the screen, as dart:ui names it.
pub fn kind_of_tool(tool_type: u32) -> PointerDeviceKind {
    match tool_type {
        tool::FINGER => PointerDeviceKind::Touch,
        tool::MOUSE => PointerDeviceKind::Mouse,
        tool::STYLUS => PointerDeviceKind::Stylus,
        tool::ERASER => PointerDeviceKind::InvertedStylus,
        _ => PointerDeviceKind::Unknown,
    }
}

/// The buttons held, as Flutter numbers them. A mouse's five are the same bits Android
/// reports; a stylus's two sit above them and come down to Flutter's second and third.
/// Nothing else has buttons.
pub fn buttons_of(button_state: i64, kind: PointerDeviceKind) -> i64 {
    match kind {
        PointerDeviceKind::Mouse => button_state & MOUSE_BUTTONS,
        PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus => {
            (button_state >> STYLUS_BUTTON_SHIFT) & STYLUS_BUTTONS
        }
        _ => 0,
    }
}

/// A wheel's notches as physical pixels, in dart:ui's content-forward sign: Android reports
/// a scroll away from the user as positive, where Flutter moves the content the other way.
pub fn scroll_delta(horizontal: f32, vertical: f32) -> [f64; 2] {
    [
        -f64::from(horizontal) * SCROLL_PIXELS_PER_NOTCH,
        -f64::from(vertical) * SCROLL_PIXELS_PER_NOTCH,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_names_the_device_kind() {
        assert_eq!(kind_of_tool(tool::FINGER), PointerDeviceKind::Touch);
        assert_eq!(kind_of_tool(tool::MOUSE), PointerDeviceKind::Mouse);
        assert_eq!(kind_of_tool(tool::STYLUS), PointerDeviceKind::Stylus);
        assert_eq!(
            kind_of_tool(tool::ERASER),
            PointerDeviceKind::InvertedStylus,
            "the other end of a stylus"
        );
        assert_eq!(
            kind_of_tool(0),
            PointerDeviceKind::Unknown,
            "TOOL_TYPE_UNKNOWN, and anything else the platform reports"
        );
    }

    #[test]
    fn one_id_with_two_tools_is_two_devices() {
        assert_ne!(
            device_id(0, tool::FINGER),
            device_id(0, tool::STYLUS),
            "a finger and a stylus sharing an id are told apart"
        );
        assert_eq!(device_id(1, tool::FINGER), (1 << 3) | 1);
    }

    #[test]
    fn a_mouse_keeps_androids_own_button_bits() {
        // Android's BUTTON_PRIMARY, SECONDARY, TERTIARY, BACK and FORWARD.
        for state in [0x01, 0x02, 0x04, 0x08, 0x10] {
            assert_eq!(buttons_of(state, PointerDeviceKind::Mouse), state);
        }
        assert_eq!(
            buttons_of(0x20, PointerDeviceKind::Mouse),
            0,
            "a stylus's bits are not a mouse's"
        );
    }

    #[test]
    fn a_styluss_buttons_come_down_to_flutters_second_and_third() {
        // BUTTON_STYLUS_PRIMARY and BUTTON_STYLUS_SECONDARY.
        assert_eq!(buttons_of(0x20, PointerDeviceKind::Stylus), 0x2);
        assert_eq!(buttons_of(0x40, PointerDeviceKind::Stylus), 0x4);
        assert_eq!(buttons_of(0x60, PointerDeviceKind::InvertedStylus), 0x6);
    }

    #[test]
    fn a_finger_has_no_buttons_of_its_own() {
        assert_eq!(buttons_of(0x1F, PointerDeviceKind::Touch), 0);
    }

    #[test]
    fn a_wheel_scrolls_the_content_the_way_flutter_expects() {
        assert_eq!(scroll_delta(0.0, 1.0), [0.0, -48.0]);
        assert_eq!(scroll_delta(-1.0, 0.0), [48.0, 0.0]);
    }
}
