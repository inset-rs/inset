//! The page's screen, as `window.screen` describes it: the one screen the host reports, as
//! Flutter's web engine reports one display, numbered as it numbers it.
//!
//! The browser names no screen and says nothing of its refresh rate. It gives a screen's place on
//! the desktop only where it has `screen.left` and `screen.top`, which are not standard, so the
//! screen is at the origin elsewhere; what the system's bars leave free is placed by
//! `screen.availLeft` and `screen.availTop`, which are not standard either but every browser has.

use inset_embedder::{Rect, Screen, ScreenId};
use wasm_bindgen::JsValue;
use web_sys::Window;

/// The page's one screen.
pub(crate) const SCREEN: ScreenId = ScreenId(0);

/// The page's screen now, in CSS pixels, at the page's pixel ratio, which the canvas is sized by;
/// `None` where the browser gives no `window.screen`.
pub(crate) fn read_screen(window: &Window) -> Option<Screen> {
    let screen = window.screen().ok()?;
    Some(Screen {
        id: SCREEN,
        name: String::new(),
        frame: Rect::from_ltwh(
            css_pixels(screen.left()),
            css_pixels(screen.top()),
            css_pixels(screen.width()),
            css_pixels(screen.height()),
        ),
        visible_frame: Rect::from_ltwh(
            css_pixels(screen.avail_left()),
            css_pixels(screen.avail_top()),
            css_pixels(screen.avail_width()),
            css_pixels(screen.avail_height()),
        ),
        scale: window.device_pixel_ratio(),
        refresh_rate: None,
    })
}

/// A length `window.screen` answers, none where the browser has none.
fn css_pixels(length: Result<i32, JsValue>) -> f64 {
    f64::from(length.unwrap_or(0))
}
