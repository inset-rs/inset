//! Flutter counterpart: `services/live_text.dart`.

use reveal_foundation::App;

/// Utility methods for interacting with the system's Live Text.
///
/// For example, the Live Text input feature of iOS turns the keyboard into a camera view for
/// directly inserting text obtained through OCR into the active field.
///
/// See also:
///  * <https://developer.apple.com/documentation/uikit/uiresponder/3778577-capturetextfromcamera>
///  * <https://support.apple.com/guide/iphone/use-live-text-iphcf0b71b0e/ios>
pub struct LiveText;

impl LiveText {
    /// Returns true if the Live Text input feature is available on the current device.
    pub fn is_live_text_input_available(_app: &App) -> bool {
        false
    }

    /// Start Live Text input.
    ///
    /// If any `TextInputConnection` is currently active, calling this method will tell the text field
    /// to start Live Text input. If the current device doesn't support Live Text input,
    /// nothing will happen.
    pub fn start_live_text_input(_app: &App) {}
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;

    use super::*;

    #[test]
    fn unsupported_defaults() {
        let cell = AppCell::new();
        let app = cell.borrow_mut();
        assert!(!LiveText::is_live_text_input_available(&app));
        LiveText::start_live_text_input(&app);
    }
}
