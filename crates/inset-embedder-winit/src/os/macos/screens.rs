//! Where AppKit puts a window: its frame, set at once or animated, a window that slides in, the
//! screens it can be on and the pointer's place on them, and AppKit's points, measured from the
//! bottom left of the primary screen, as winit's, measured from its top left.

use std::time::Duration;

use inset_embedder::{Offset, Rect, Screen};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAnimatablePropertyContainer, NSAnimationContext, NSEvent, NSScreen};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CAMediaTimingFunction, kCAMediaTimingFunctionEaseInEaseOut};
use winit::window::Window;

use super::{AppKit, ns_window};
use crate::os::{Screens, WindowMotion};

impl WindowMotion for AppKit {
    fn set_frame(window: &Window, frame: Rect) -> bool {
        let Some(ns_window) = ns_window(window) else {
            return false;
        };
        let Some(primary_height) = primary_screen_height() else {
            return false;
        };
        ns_window.setFrame_display(appkit_rect(frame, primary_height), true);
        true
    }

    /// With AppKit's own animation, easing in and out, as a window a person drags settles.
    fn animate_frame(window: &Window, frame: Rect, duration: Duration) -> bool {
        let Some(ns_window) = ns_window(window) else {
            return false;
        };
        let Some(primary_height) = primary_screen_height() else {
            return false;
        };
        let rect = appkit_rect(frame, primary_height);
        NSAnimationContext::beginGrouping();
        let context = NSAnimationContext::currentContext();
        context.setDuration(duration.as_secs_f64());
        // SAFETY: a constant name Core Animation exports for the life of the process.
        let ease =
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseInEaseOut });
        context.setTimingFunction(Some(&ease));
        ns_window.animator().setFrame_display(rect, true);
        NSAnimationContext::endGrouping();
        true
    }

    /// With AppKit's own animation of the window's place and opacity. The size stays, so the
    /// window's surface is not configured again while it moves.
    fn show_sliding_in(window: &Window, from: Offset, duration: Duration) -> bool {
        let Some(ns_window) = ns_window(window) else {
            return false;
        };
        let placed = ns_window.frame();
        ns_window.setAlphaValue(0.0);
        ns_window.setFrame_display(moved_by(placed, from), false);
        window.set_visible(true);
        NSAnimationContext::beginGrouping();
        NSAnimationContext::currentContext().setDuration(duration.as_secs_f64());
        let animator = ns_window.animator();
        animator.setFrame_display(placed, true);
        animator.setAlphaValue(1.0);
        NSAnimationContext::endGrouping();
        true
    }
}

impl Screens for AppKit {
    fn screens() -> Option<&'static dyn inset_embedder::Screens> {
        Some(&AppKitScreens)
    }
}

/// The screens as AppKit knows them.
struct AppKitScreens;

impl inset_embedder::Screens for AppKitScreens {
    /// Every screen AppKit knows, the one with the menu bar first. A visible frame leaves the
    /// Dock's band out only while the Dock keeps its space, and not while it hides itself.
    fn screens(&self) -> Vec<Screen> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Vec::new();
        };
        let Some(primary_height) = primary_screen_height() else {
            return Vec::new();
        };
        NSScreen::screens(mtm)
            .iter()
            .map(|screen| Screen {
                frame: window_rect(screen.frame(), primary_height),
                visible_frame: window_rect(screen.visibleFrame(), primary_height),
            })
            .collect()
    }

    fn pointer_location(&self) -> Option<Offset> {
        pointer_location()
    }
}

/// Where the pointer is now on the screens: AppKit's answer, which is where the pointer was at
/// the event AppKit is delivering.
pub(super) fn pointer_location() -> Option<Offset> {
    Some(window_point(
        NSEvent::mouseLocation(),
        primary_screen_height()?,
    ))
}

/// The height of the primary screen, the one with the menu bar, which AppKit measures from its
/// bottom left and winit from its top left.
fn primary_screen_height() -> Option<f64> {
    let mtm = MainThreadMarker::new()?;
    let primary = NSScreen::screens(mtm).firstObject()?;
    Some(primary.frame().size.height)
}

/// `frame`, given in winit's top-left points, as AppKit measures it: from the bottom left of
/// the primary screen, `primary_height` tall.
fn appkit_rect(frame: Rect, primary_height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(frame.left, primary_height - frame.top - frame.height()),
        NSSize::new(frame.width(), frame.height()),
    )
}

/// `rect`, as AppKit measures it, in winit's top-left points: `appkit_rect` the other way.
fn window_rect(rect: NSRect, primary_height: f64) -> Rect {
    Rect::from_ltwh(
        rect.origin.x,
        primary_height - rect.origin.y - rect.size.height,
        rect.size.width,
        rect.size.height,
    )
}

/// `point`, as AppKit measures it, in winit's top-left points.
fn window_point(point: NSPoint, primary_height: f64) -> Offset {
    Offset::new(point.x, primary_height - point.y)
}

/// `frame`, as AppKit measures it, moved by `offset` in winit's top-left points, whose y grows
/// down where AppKit's grows up.
fn moved_by(mut frame: NSRect, offset: Offset) -> NSRect {
    frame.origin.x += offset.dx();
    frame.origin.y -= offset.dy();
    frame
}
