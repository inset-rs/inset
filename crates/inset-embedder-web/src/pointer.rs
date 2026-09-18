//! DOM `buttons` / `button` as Flutter pointer bits.

/// Flutter `kFlutterPointerButtonMousePrimary`.
pub const PRIMARY: i64 = 1 << 0;
/// Flutter `kFlutterPointerButtonMouseSecondary`.
pub const SECONDARY: i64 = 1 << 1;
/// Flutter `kFlutterPointerButtonMouseMiddle`.
pub const MIDDLE: i64 = 1 << 2;
/// Flutter `kFlutterPointerButtonMouseBack`.
pub const BACK: i64 = 1 << 3;
/// Flutter `kFlutterPointerButtonMouseForward`.
pub const FORWARD: i64 = 1 << 4;

/// DOM `MouseEvent.button` (the one that changed) as a Flutter bit.
pub fn flutter_button(button: i16) -> Option<i64> {
    Some(match button {
        0 => PRIMARY,
        1 => MIDDLE,
        2 => SECONDARY,
        3 => BACK,
        4 => FORWARD,
        n if n >= 0 && (n as u32) < i64::BITS => 1_i64 << n,
        _ => return None,
    })
}

/// DOM `MouseEvent.buttons` (the set currently down) as Flutter bits.
///
/// DOM uses 1/2/4 for left/right/middle; Flutter uses the same values for those
/// three. Extra buttons stay in place.
pub fn flutter_buttons(dom_buttons: u16) -> i64 {
    i64::from(dom_buttons)
}

/// DOM `WheelEvent.deltaMode` `DOM_DELTA_LINE`.
#[cfg(any(target_arch = "wasm32", test))]
const DELTA_LINE: u32 = 1;
/// Line-based wheels are not pixels. 40 logical px/line is Chromium's convention.
#[cfg(any(target_arch = "wasm32", test))]
const LOGICAL_PIXELS_PER_LINE: f64 = 40.0;

/// DOM `deltaX` / `deltaY` to dart:ui physical `scrollDelta`.
///
/// Browser wheel deltas are already Flutter's content-forward sign: positive
/// `deltaY` scrolls down. Winit is up-positive and flips; copying that flip
/// here reverses trackpad scrolling on macOS.
/// Only the browser host calls this, so off wasm it is built for its tests alone.
#[cfg(any(target_arch = "wasm32", test))]
pub fn wheel_to_physical(delta_x: f64, delta_y: f64, delta_mode: u32, scale: f64) -> (f64, f64) {
    let (dx, dy) = if delta_mode == DELTA_LINE {
        (
            delta_x * LOGICAL_PIXELS_PER_LINE,
            delta_y * LOGICAL_PIXELS_PER_LINE,
        )
    } else {
        (delta_x, delta_y)
    };
    (dx * scale, dy * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dom_buttons_match_flutter_for_the_three_named_ones() {
        assert_eq!(flutter_button(0), Some(PRIMARY));
        assert_eq!(flutter_button(2), Some(SECONDARY));
        assert_eq!(flutter_button(1), Some(MIDDLE));
        assert_eq!(flutter_buttons(1 | 2), PRIMARY | SECONDARY);
    }

    #[test]
    fn wheel_keeps_dom_sign_and_scales_to_physical() {
        assert_eq!(wheel_to_physical(0.0, 1.0, 1, 2.0), (0.0, 80.0));
        assert_eq!(wheel_to_physical(0.0, -3.0, 1, 1.0), (0.0, -120.0));
        assert_eq!(wheel_to_physical(0.0, -100.0, 0, 2.0), (0.0, -200.0));
    }
}
