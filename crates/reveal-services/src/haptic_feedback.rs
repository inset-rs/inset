//! Flutter `services/haptic_feedback.dart`.

use reveal_embedder::HapticFeedbackType;
use reveal_foundation::App;

/// Allows access to the haptic feedback interface on the device.
///
/// This API is intentionally terse since it calls default platform behavior. It
/// is not suitable for precise control of the system's haptic feedback module.
///
/// See also:
///
/// * [Human Interface Haptics Guidelines](https://developer.apple.com/design/human-interface-guidelines/playing-haptics)
pub struct HapticFeedback;

impl HapticFeedback {
    /// Provides vibration haptic feedback to the user for a short duration.
    ///
    /// On iOS devices that support haptic feedback, this uses the default system
    /// vibration value (`kSystemSoundID_Vibrate`).
    ///
    /// On Android, this uses the platform haptic feedback API to simulate a
    /// response to a long press (`HapticFeedbackConstants.LONG_PRESS`).
    pub fn vibrate(app: &App) {
        app.platform().haptic_feedback(HapticFeedbackType::Vibrate);
    }

    /// Provides a haptic feedback corresponding a collision impact with a light mass.
    ///
    /// On iOS versions 10 and above, this uses a `UIImpactFeedbackGenerator` with
    /// `UIImpactFeedbackStyleLight`. This call has no effects on iOS versions
    /// below 10.
    ///
    /// On Android, this uses `HapticFeedbackConstants.VIRTUAL_KEY`.
    ///
    /// See also:
    ///
    /// * [Human Interface Selection Playing Impact Haptic](https://developer.apple.com/design/human-interface-guidelines/playing-haptics#Impact)
    pub fn light_impact(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::LightImpact);
    }

    /// Provides a haptic feedback corresponding a collision impact with a medium mass.
    ///
    /// On iOS versions 10 and above, this uses a `UIImpactFeedbackGenerator` with
    /// `UIImpactFeedbackStyleMedium`. This call has no effects on iOS versions
    /// below 10.
    ///
    /// On Android, this uses `HapticFeedbackConstants.KEYBOARD_TAP`.
    pub fn medium_impact(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::MediumImpact);
    }

    /// Provides a haptic feedback corresponding a collision impact with a heavy mass.
    ///
    /// On iOS versions 10 and above, this uses a `UIImpactFeedbackGenerator` with
    /// `UIImpactFeedbackStyleHeavy`. This call has no effects on iOS versions
    /// below 10.
    ///
    /// On Android, this uses `HapticFeedbackConstants.CONTEXT_CLICK` on API levels
    /// 23 and above. This call has no effects on Android API levels below 23.
    pub fn heavy_impact(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::HeavyImpact);
    }

    /// Provides a haptic feedback indication selection changing through discrete values.
    ///
    /// On iOS versions 10 and above, this uses a `UISelectionFeedbackGenerator`.
    /// This call has no effects on iOS versions below 10.
    ///
    /// On Android, this uses `HapticFeedbackConstants.CLOCK_TICK`.
    ///
    /// See also:
    ///
    /// * [Human Interface Selection Playing Selection Haptics](https://developer.apple.com/design/human-interface-guidelines/playing-haptics#Selection)
    pub fn selection_click(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::SelectionClick);
    }

    /// Provides a haptic feedback indicating that a task or action has completed
    /// successfully.
    ///
    /// On iOS, this uses a `UINotificationFeedbackGenerator` with
    /// `UINotificationFeedbackTypeSuccess`.
    ///
    /// On Android, this uses `HapticFeedbackConstants.CONFIRM` on API levels 30
    /// and above. This call has no effects on Android API levels below 30.
    ///
    /// See also:
    ///
    ///  * [Human Interface Guidelines Playing Haptics](https://developer.apple.com/design/human-interface-guidelines/playing-haptics#Notification)
    pub fn success_notification(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::SuccessNotification);
    }

    /// Provides a haptic feedback indicating that a task or action has produced
    /// a warning.
    ///
    /// On iOS, this uses a `UINotificationFeedbackGenerator` with
    /// `UINotificationFeedbackTypeWarning`.
    ///
    /// On Android, this uses `HapticFeedbackConstants.KEYBOARD_TAP` on API
    /// levels 30 and above. This call has no effects on Android API levels below
    /// 30.
    pub fn warning_notification(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::WarningNotification);
    }

    /// Provides a haptic feedback indicating that a task or action has failed.
    ///
    /// On iOS, this uses a `UINotificationFeedbackGenerator` with
    /// `UINotificationFeedbackTypeError`.
    ///
    /// On Android, this uses `HapticFeedbackConstants.REJECT` on API levels 30
    /// and above. This call has no effects on Android API levels below 30.
    pub fn error_notification(app: &App) {
        app.platform()
            .haptic_feedback(HapticFeedbackType::ErrorNotification);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Instant;

    use reveal_embedder::{Platform, PlatformRef, TargetPlatform, ViewId, ViewRef};

    use super::*;

    #[derive(Default)]
    struct RecordingPlatform {
        kinds: RefCell<Vec<HapticFeedbackType>>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::IOS
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }

        fn haptic_feedback(&self, kind: HapticFeedbackType) {
            self.kinds.borrow_mut().push(kind);
        }
    }

    #[test]
    fn each_call_sends_its_own_feedback_kind_to_the_platform() {
        let platform = Rc::new(RecordingPlatform::default());
        let app = App::with_platform(Rc::clone(&platform) as PlatformRef);

        HapticFeedback::vibrate(&app);
        HapticFeedback::light_impact(&app);
        HapticFeedback::medium_impact(&app);
        HapticFeedback::heavy_impact(&app);
        HapticFeedback::selection_click(&app);
        HapticFeedback::success_notification(&app);
        HapticFeedback::warning_notification(&app);
        HapticFeedback::error_notification(&app);

        assert_eq!(
            platform.kinds.borrow().as_slice(),
            [
                HapticFeedbackType::Vibrate,
                HapticFeedbackType::LightImpact,
                HapticFeedbackType::MediumImpact,
                HapticFeedbackType::HeavyImpact,
                HapticFeedbackType::SelectionClick,
                HapticFeedbackType::SuccessNotification,
                HapticFeedbackType::WarningNotification,
                HapticFeedbackType::ErrorNotification,
            ]
        );
    }
}
