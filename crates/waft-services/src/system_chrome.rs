//! Flutter `services/system_chrome.dart`: the application switcher description, the system
//! overlay style, and what a lifecycle change does to it. The rest of `SystemChrome` waits for
//! the host capabilities it would talk to.
//!
//! The overlay style is a view's, sent to the view's host, since each window carries its own
//! bars, where Flutter's is the app's. The invariant a later editor will break: the pending and the
//! latest style are kept per view, so one view's style never stands for, or hides, another's.

use std::collections::BTreeMap;

use waft_embedder::{
    AppLifecycleState, ApplicationSwitcherDescription, SystemUiOverlayStyle, ViewId,
};
use waft_foundation::{App, Handle, Listener};

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
/// fields of a Dart class and one arena object per [`App`] here, each by view.
#[derive(Default)]
struct SystemChromeStyles {
    pending_style: BTreeMap<ViewId, SystemUiOverlayStyle>,
    latest_style: BTreeMap<ViewId, SystemUiOverlayStyle>,
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
        if let Some(chrome) = app.platform().system_chrome() {
            chrome.set_application_switcher_description(description);
        }
    }

    /// Specifies the style to use for the system overlays (e.g. the status bar on
    /// Android or iOS, the system navigation bar on Android) that are visible (if any) over
    /// `view`.
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
    pub fn set_system_ui_overlay_style(app: &mut App, view: ViewId, style: &SystemUiOverlayStyle) {
        let styles = app.singleton::<SystemChromeStyles>();
        if !app.get(styles).pending_style.is_empty() {
            // The microtask has already been queued; just update the pending value.
            app.get_mut(styles).pending_style.insert(view, *style);
            return;
        }
        if Some(style) == app.get(styles).latest_style.get(&view) {
            // Trivial success: no microtask has been queued and the given style is
            // already in effect, so no need to queue a microtask.
            return;
        }
        app.get_mut(styles).pending_style.insert(view, *style);
        app.schedule_microtask(Listener::new(move |app| {
            Self::send_pending_styles(app, styles)
        }));
    }

    /// The body of the microtask Dart's `setSystemUIOverlayStyle` queues, for each view a style
    /// is pending for.
    fn send_pending_styles(app: &mut App, styles: Handle<SystemChromeStyles>) {
        let pending = std::mem::take(&mut app.get_mut(styles).pending_style);
        debug_assert!(!pending.is_empty());
        for (view, style) in pending {
            if app.get(styles).latest_style.get(&view) == Some(&style) {
                continue;
            }
            if let Some(view) = app.platform().view(view) {
                view.host().set_overlay_style(&style);
            }
            app.get_mut(styles).latest_style.insert(view, style);
        }
    }

    /// Called by the binding during a transition to a new app lifecycle state.
    pub fn handle_app_lifecycle_state_changed(app: &mut App, state: AppLifecycleState) {
        // When the app is detached, clear the record of the style sent to the host
        // so that it will be sent again when the app is reattached.
        if state == AppLifecycleState::Detached {
            let styles = app.singleton::<SystemChromeStyles>();
            app.schedule_microtask(Listener::new(move |app| {
                app.get_mut(styles).latest_style.clear();
            }));
        }
    }

    /// The last style that was set over `view` using
    /// [`set_system_ui_overlay_style`](SystemChrome::set_system_ui_overlay_style).
    pub fn latest_style(app: &mut App, view: ViewId) -> Option<SystemUiOverlayStyle> {
        let styles = app.singleton::<SystemChromeStyles>();
        app.get(styles).latest_style.get(&view).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use waft_foundation::AppCell;

    use waft_embedder::{Brightness, Color, TargetPlatform};
    use waft_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};

    use super::*;

    const VIEW: ViewId = IMPLICIT_VIEW_ID;
    const OTHER_VIEW: ViewId = ViewId(1);

    /// An app with two views, whose hosts record the overlay styles they are handed.
    fn recording_app() -> (Rc<TestPlatform>, Rc<AppCell>) {
        let platform = Rc::new(
            TestPlatform::new()
                .on(TargetPlatform::IOS)
                .with_views(&[VIEW, OTHER_VIEW]),
        );
        (Rc::clone(&platform), AppCell::with_platform(platform))
    }

    /// The styles `view`'s host was handed, oldest first.
    fn styles_of(platform: &TestPlatform, view: ViewId) -> Vec<SystemUiOverlayStyle> {
        let view: Rc<TestView> = platform.test_view(view);
        view.test_host().overlay_styles()
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
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::DARK);
        assert!(
            styles_of(&platform, VIEW).is_empty(),
            "nothing is sent before the microtask runs"
        );
        assert_eq!(SystemChrome::latest_style(&mut app, VIEW), None);

        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).as_slice(),
            [SystemUiOverlayStyle::DARK]
        );
        assert_eq!(
            SystemChrome::latest_style(&mut app, VIEW),
            Some(SystemUiOverlayStyle::DARK)
        );
    }

    #[test]
    fn a_style_already_in_effect_queues_nothing() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();

        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).as_slice(),
            [SystemUiOverlayStyle::LIGHT],
            "an unchanged style does not reach the host again"
        );

        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::DARK);
        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).as_slice(),
            [SystemUiOverlayStyle::LIGHT, SystemUiOverlayStyle::DARK]
        );
    }

    /// `system_chrome_test.dart`: the end of `SystemChrome overlay style test`.
    #[test]
    fn the_style_is_sent_again_after_the_app_was_detached() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(styles_of(&platform, VIEW).len(), 1);

        // The state should be sent again if the app was detached.
        SystemChrome::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Detached);
        app.drain_microtasks();
        SystemChrome::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Resumed);
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(styles_of(&platform, VIEW).len(), 2);
    }

    #[test]
    fn a_style_that_returns_to_the_latest_within_the_turn_sends_nothing() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();

        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::DARK);
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).as_slice(),
            [SystemUiOverlayStyle::LIGHT]
        );
        assert_eq!(
            SystemChrome::latest_style(&mut app, VIEW),
            Some(SystemUiOverlayStyle::LIGHT)
        );
    }

    #[test]
    fn each_view_keeps_its_own_style() {
        let (platform, cell) = recording_app();
        let mut app = cell.borrow_mut();
        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        SystemChrome::set_system_ui_overlay_style(
            &mut app,
            OTHER_VIEW,
            &SystemUiOverlayStyle::DARK,
        );
        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).as_slice(),
            [SystemUiOverlayStyle::LIGHT]
        );
        assert_eq!(
            styles_of(&platform, OTHER_VIEW).as_slice(),
            [SystemUiOverlayStyle::DARK]
        );

        SystemChrome::set_system_ui_overlay_style(&mut app, VIEW, &SystemUiOverlayStyle::LIGHT);
        app.drain_microtasks();
        assert_eq!(
            styles_of(&platform, VIEW).len(),
            1,
            "a style in effect over its view is not sent again for another view's"
        );
    }
}
