//! Flutter counterpart: `widgets/feedback.dart`.
//!
//! Theme-less widgets version: haptic feedback only. Semantics events and
//! [`SystemSound`] wait; see PORTING.md.

use inset_embedder::TargetPlatform;
use inset_foundation::App;
use inset_gestures::{GestureLongPressCallback, GestureTapCallback};
use inset_services::HapticFeedback;

use crate::framework::BuildContext;

/// Provides platform-specific acoustic and/or haptic feedback for certain
/// actions.
///
/// For example, to play the Android-typically click sound when a button is
/// tapped, call [`for_tap`](Self::for_tap). For the Android-specific vibration when long pressing
/// an element, call [`for_long_press`](Self::for_long_press). Alternatively, you can also wrap your
/// `GestureDetector.on_tap` or `GestureDetector.on_long_press` callback in
/// [`wrap_for_tap`](Self::wrap_for_tap) or [`wrap_for_long_press`](Self::wrap_for_long_press) to achieve the same.
///
/// All methods in this class are usually called from within a
/// `StatelessWidget::build` method or from a `State`'s methods as you have to
/// provide a [`BuildContext`].
pub struct Feedback;

impl Feedback {
    /// Provides platform-specific feedback for a tap.
    ///
    /// On Android the click system sound is played. On iOS this is a no-op.
    ///
    /// See also:
    ///
    ///  * [`wrap_for_tap`](Self::wrap_for_tap) to trigger platform-specific feedback before executing a
    ///    [`GestureTapCallback`].
    pub fn for_tap(app: &App, _context: BuildContext) {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::Fuchsia => {}
            TargetPlatform::IOS
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }

    /// Wraps a [`GestureTapCallback`] to provide platform specific feedback for a
    /// tap before the provided callback is executed.
    ///
    /// On Android the platform-typical click system sound is played. On iOS this
    /// is a no-op as that platform usually doesn't provide feedback for a tap.
    ///
    /// See also:
    ///
    ///  * [`for_tap`](Self::for_tap) to just trigger the platform-specific feedback without wrapping
    ///    a [`GestureTapCallback`].
    pub fn wrap_for_tap(
        callback: Option<GestureTapCallback>,
        context: BuildContext,
    ) -> Option<GestureTapCallback> {
        let callback = callback?;
        Some(inset_foundation::Listener::new(move |app| {
            Feedback::for_tap(app, context);
            callback.call(app);
        }))
    }

    /// Provides platform-specific feedback for a long press.
    ///
    /// On Android the platform-typical vibration is triggered. On iOS a
    /// heavy-impact haptic feedback is triggered alongside the click system
    /// sound, which was observed to be the default behavior on a physical iPhone
    /// 15 Pro running iOS version 17.5.
    ///
    /// See also:
    ///
    ///  * [`wrap_for_long_press`](Self::wrap_for_long_press) to trigger platform-specific feedback before
    ///    executing a [`GestureLongPressCallback`].
    pub fn for_long_press(app: &App, _context: BuildContext) {
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::Fuchsia => {
                HapticFeedback::vibrate(app);
            }
            TargetPlatform::IOS => {
                HapticFeedback::heavy_impact(app);
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {}
        }
    }

    /// Wraps a [`GestureLongPressCallback`] to provide platform specific feedback
    /// for a long press before the provided callback is executed.
    ///
    /// On Android the platform-typical vibration is triggered. On iOS a
    /// heavy-impact haptic feedback is triggered alongside the click system
    /// sound, which was observed to be the default behavior on a physical iPhone
    /// 15 Pro running iOS version 17.5.
    ///
    /// See also:
    ///
    ///  * [`for_long_press`](Self::for_long_press) to just trigger the platform-specific feedback without
    ///    wrapping a [`GestureLongPressCallback`].
    pub fn wrap_for_long_press(
        callback: Option<GestureLongPressCallback>,
        context: BuildContext,
    ) -> Option<GestureLongPressCallback> {
        let callback = callback?;
        Some(inset_foundation::Listener::new(move |app| {
            Feedback::for_long_press(app, context);
            callback.call(app);
        }))
    }
}
