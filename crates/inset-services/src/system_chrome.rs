//! Flutter `services/system_chrome.dart`: the application switcher description and the system
//! overlay style. The rest of `SystemChrome` waits for the host capabilities it would talk to.

use inset_embedder::{ApplicationSwitcherDescription, SystemUiOverlayStyle};
use inset_foundation::{App, Handle, Listener};

/// Specifies a set of mutually exclusive system UI overlays.
///
/// Used by [`SystemChrome`] to specify the set of overlays that should be visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemUiOverlay {
    /// The status bar provided by the embedder on the top of the application
    /// surface, if any.
    Top,

    /// The status bar provided by the embedder on the bottom of the application
    /// surface, if any.
    Bottom,
}

/// Dart's `SystemChrome._pendingStyle` and `SystemChrome._latestStyle`, which are static
/// fields of a Dart class and one arena object per [`App`] here.
#[derive(Default)]
struct SystemChromeStyles {
    pending_style: Option<SystemUiOverlayStyle>,
    latest_style: Option<SystemUiOverlayStyle>,
}

/// Controls specific aspects of the operating system's graphical interface and
/// how it interacts with the application.
pub struct SystemChrome;

impl SystemChrome {
    /// Specifies the description of the current state of the application as it
    /// pertains to the application switcher (also known as "recent tasks").
    ///
    /// Any part of the description that is unsupported on the current platform
    /// will be ignored.
    pub fn set_application_switcher_description(
        app: &App,
        description: &ApplicationSwitcherDescription,
    ) {
        app.platform()
            .set_application_switcher_description(description);
    }

    /// Specifies the style to use for the system overlays (e.g. the status bar on
    /// Android or iOS, the system navigation bar on Android) that are visible (if any).
    ///
    /// This method will schedule the embedder update to be run in a microtask.
    /// Any subsequent calls to this method during the current event loop will
    /// overwrite the pending value, such that only the last specified value takes
    /// effect.
    ///
    /// Call this API in code whose lifecycle matches that of the desired
    /// system UI styles. For instance, to change the system UI style on a new
    /// page, consider calling when pushing/popping a new `PageRoute`.
    ///
    /// If a particular style is not supported on the platform, selecting it will
    /// have no effect.
    ///
    /// For more complex control of the system overlay styles, consider using
    /// an `AnnotatedRegion` widget instead of calling this method directly. That widget
    /// places a value directly into the layer tree where it can be hit-tested by the
    /// framework.
    pub fn set_system_ui_overlay_style(app: &mut App, style: &SystemUiOverlayStyle) {
        let styles = app.singleton::<SystemChromeStyles>();
        if app.get(styles).pending_style.is_some() {
            // The microtask has already been queued; just update the pending value.
            app.get_mut(styles).pending_style = Some(*style);
            return;
        }
        if Some(*style) == app.get(styles).latest_style {
            // Trivial success: no microtask has been queued and the given style is
            // already in effect, so no need to queue a microtask.
            return;
        }
        app.get_mut(styles).pending_style = Some(*style);
        app.schedule_microtask(Listener::new(move |app| {
            Self::send_pending_style(app, styles)
        }));
    }

    /// The body of the microtask Dart's `setSystemUIOverlayStyle` queues.
    fn send_pending_style(app: &mut App, styles: Handle<SystemChromeStyles>) {
        let pending = app.get(styles).pending_style;
        debug_assert!(pending.is_some());
        if pending != app.get(styles).latest_style {
            if let Some(style) = pending {
                app.platform().set_system_ui_overlay_style(&style);
            }
            app.get_mut(styles).latest_style = pending;
        }
        app.get_mut(styles).pending_style = None;
    }

    /// The last style that was set using
    /// [`set_system_ui_overlay_style`](SystemChrome::set_system_ui_overlay_style).
    pub fn latest_style(app: &mut App) -> Option<SystemUiOverlayStyle> {
        let styles = app.singleton::<SystemChromeStyles>();
        app.get(styles).latest_style
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Instant;

    use inset_embedder::{
        Brightness, Color, Platform, PlatformRef, TargetPlatform, ViewId, ViewRef,
    };

    use super::*;

    #[derive(Default)]
    struct RecordingPlatform {
        styles: RefCell<Vec<SystemUiOverlayStyle>>,
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

        fn set_system_ui_overlay_style(&self, style: &SystemUiOverlayStyle) {
            self.styles.borrow_mut().push(*style);
        }
    }

    fn recording_app() -> (Rc<RecordingPlatform>, Rc<AppCell>) {
        let platform = Rc::new(RecordingPlatform::default());
        let cell = AppCell::with_platform(Rc::clone(&platform) as PlatformRef);
        (platform, cell)
    }

    #[test]
    fn light_and_dark_differ_only_in_the_two_status_bar_brightnesses() {
        assert_eq!(
            SystemUiOverlayStyle::LIGHT.status_bar_icon_brightness,
            Some(Brightness::Light)
        );
        assert_eq!(
            SystemUiOverlayStyle::LIGHT.status_bar_brightness,
            Some(Brightness::Dark)
        );
        assert_eq!(
            SystemUiOverlayStyle::DARK.status_bar_icon_brightness,
            Some(Brightness::Dark)
        );
        assert_eq!(
            SystemUiOverlayStyle::DARK.status_bar_brightness,
            Some(Brightness::Light)
        );
        assert_eq!(
            SystemUiOverlayStyle::LIGHT.system_navigation_bar_color,
            SystemUiOverlayStyle::DARK.system_navigation_bar_color
        );
        assert_ne!(SystemUiOverlayStyle::LIGHT, SystemUiOverlayStyle::DARK);
    }

    #[test]
    fn copy_with_replaces_only_what_it_is_given() {
        let style = SystemUiOverlayStyle::LIGHT
            .copy_with()
            .status_bar_color(Color::new(0xFF112233));
        assert_eq!(style.status_bar_color, Some(Color::new(0xFF112233)));
        assert_eq!(
            style.status_bar_icon_brightness,
            SystemUiOverlayStyle::LIGHT.status_bar_icon_brightness
        );
        assert_eq!(SystemUiOverlayStyle::LIGHT.status_bar_color, None);
    }

    /// `system_chrome_test.dart`: `SystemChrome overlay style test`.
    #[test]
    fn only_the_last_style_of_a_turn_reaches_the_host() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::LIGHT);
        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::DARK);
        assert!(
            platform.styles.borrow().is_empty(),
            "nothing is sent before the microtask runs"
        );
        assert_eq!(SystemChrome::latest_style(&mut app), None);

        app.drain_microtasks();
        assert_eq!(
            platform.styles.borrow().as_slice(),
            [SystemUiOverlayStyle::DARK]
        );
        assert_eq!(
            SystemChrome::latest_style(&mut app),
            Some(SystemUiOverlayStyle::DARK)
        );
    }

    #[test]
    fn a_style_already_in_effect_queues_nothing() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();

        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(
            platform.styles.borrow().as_slice(),
            [SystemUiOverlayStyle::LIGHT],
            "an unchanged style does not reach the host again"
        );

        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::DARK);
        app.drain_microtasks();
        assert_eq!(
            platform.styles.borrow().as_slice(),
            [SystemUiOverlayStyle::LIGHT, SystemUiOverlayStyle::DARK]
        );
    }

    #[test]
    fn a_style_that_returns_to_the_latest_within_the_turn_sends_nothing() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        platform.styles.borrow_mut().clear();

        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::DARK);
        SystemChrome::set_system_ui_overlay_style(&mut app, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert!(platform.styles.borrow().is_empty());
        assert_eq!(
            SystemChrome::latest_style(&mut app),
            Some(SystemUiOverlayStyle::LIGHT)
        );
    }
}
