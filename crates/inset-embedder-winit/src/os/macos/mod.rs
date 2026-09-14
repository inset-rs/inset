//! What AppKit adds to a winit window: presence on every Space, the glass or blur behind
//! a see-through window, a frame that animates, the popup menu, and, for a window that
//! never becomes key, the pointer's moves.

// AppKit's selectors and view hierarchy need `unsafe`; this module and its children are
// the one place in the host that writes it.
#![allow(unsafe_code)]

mod menu;
mod vsync;

use std::time::Duration;

use inset_embedder::{Rect, WindowBackground, WindowConfig};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAnimationContext, NSAutoresizingMaskOptions, NSEvent,
    NSGlassEffectView, NSGlassEffectViewStyle, NSScreen, NSTrackingArea, NSTrackingAreaOptions,
    NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CAMediaTimingFunction, kCAMediaTimingFunctionEaseInEaseOut};
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes};

pub(crate) use menu::popup_menu;
pub(crate) use vsync::Vsync;

/// AppKit commits window geometry before winit's next redraw, so a resize presents the
/// new layout before it returns, through the window's own transaction so the frame and
/// the geometry it was laid out for are committed together.
pub(crate) const FRAME_ON_RESIZE: bool = true;

fn ns_view(window: &Window) -> Option<Retained<NSView>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: winit's AppKit handle is the NSView of a live window, read on the main thread.
    unsafe { Retained::retain(appkit.ns_view.cast::<NSView>().as_ptr()) }
}

fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    ns_view(window)?.window()
}

/// What winit has an AppKit attribute for: the shadow.
pub(crate) fn extend_attributes(
    attributes: WindowAttributes,
    config: &WindowConfig,
) -> WindowAttributes {
    attributes.with_has_shadow(config.shadow)
}

/// Applies what the configuration asks for and winit has no attribute for.
pub(crate) fn configure(window: &Window, config: &WindowConfig) {
    if !config.activating
        && let Some(view) = ns_view(window)
    {
        track_pointer_always(&view);
    }
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    if config.all_desktops {
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
    // The effect view goes behind winit's view, as a sibling in the window's frame view:
    // a subview of winit's view would draw over its Metal layer. Liquid Glass renders as
    // its flat material there rather than as glass, which needs the view inside the
    // content view, where winit's Metal layer did not stay above it.
    if matches!(
        config.background,
        WindowBackground::Blurred | WindowBackground::Glass
    ) && let Some(mtm) = MainThreadMarker::new()
        && let Some(content) = ns_window.contentView()
        // SAFETY: the view hierarchy of a live window, read on the main thread.
        && let Some(frame_view) = unsafe { content.superview() }
    {
        let effect = background_view(mtm, config.background, content.frame());
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        frame_view.addSubview_positioned_relativeTo(
            &effect,
            NSWindowOrderingMode::Below,
            Some(&content),
        );
    }
}

/// The buttons the mouse holds down now, as Flutter numbers them: the same bits AppKit
/// reports. A native menu swallows the release of the button that opened it, so the
/// host reconciles its own record with this once the menu is gone.
pub(crate) fn pressed_buttons() -> Option<i64> {
    Some(NSEvent::pressedMouseButtons() as i64)
}

/// Has AppKit report the pointer's moves to the view whether or not its window is key.
///
/// winit tracks the mouse with a tracking rect, which reports entry and exit alone, and
/// takes moves from `acceptsMouseMovedEvents`, which AppKit honours for the key window
/// only. A window that never activates is never key, so without this it would hear the
/// pointer arrive and leave but never move: no hover, and clicks at stale positions.
/// Flutter's macOS embedder offers the same as its `always` mouse tracking mode. The
/// view is the owner, so winit's own handlers receive what the area reports.
fn track_pointer_always(view: &NSView) {
    let owner: &AnyObject = view.as_ref();
    // SAFETY: the options name no user info, and the view outlives the area it owns.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            view.bounds(),
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(owner),
            None,
        )
    };
    view.addTrackingArea(&area);
}

/// Where the pointer is now, in the window's physical pixels from its top-left corner.
/// winit's entry event carries no position; AppKit's answer is current.
pub(crate) fn pointer_position(window: &Window) -> Option<[f64; 2]> {
    let view = ns_view(window)?;
    let ns_window = view.window()?;
    let in_window = ns_window.convertPointFromScreen(NSEvent::mouseLocation());
    let in_view = view.convertPoint_fromView(in_window, None);
    let y = if view.isFlipped() {
        in_view.y
    } else {
        view.bounds().size.height - in_view.y
    };
    let scale = ns_window.backingScaleFactor();
    Some([in_view.x * scale, y * scale])
}

/// Liquid Glass where the system has it (macOS 26), else the vibrancy blur.
fn background_view(
    mtm: MainThreadMarker,
    background: WindowBackground,
    frame: NSRect,
) -> Retained<NSView> {
    if background == WindowBackground::Glass && AnyClass::get(c"NSGlassEffectView").is_some() {
        let glass = NSGlassEffectView::initWithFrame(mtm.alloc(), frame);
        glass.setStyle(NSGlassEffectViewStyle::Regular);
        return Retained::into_super(glass);
    }
    let blur = NSVisualEffectView::initWithFrame(mtm.alloc(), frame);
    blur.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    blur.setState(NSVisualEffectState::Active);
    blur.setMaterial(NSVisualEffectMaterial::Popover);
    Retained::into_super(blur)
}

/// Moves and sizes the window over `duration` with AppKit's own animation, easing in and
/// out, as a window a person drags settles. `false` when the window cannot be reached, so
/// the caller sets the frame at once instead.
/// Puts `window` at `frame` in one step, position and size together.
pub(crate) fn set_frame(window: &Window, frame: Rect) -> bool {
    let Some(ns_window) = ns_window(window) else {
        return false;
    };
    let Some(rect) = appkit_rect(frame) else {
        return false;
    };
    ns_window.setFrame_display(rect, true);
    true
}

pub(crate) fn animate_frame(window: &Window, frame: Rect, duration: Duration) -> bool {
    let Some(ns_window) = ns_window(window) else {
        return false;
    };
    let Some(rect) = appkit_rect(frame) else {
        return false;
    };
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

/// `frame`, given in winit's top-left points, as AppKit measures it: from the bottom left
/// of the primary screen.
fn appkit_rect(frame: Rect) -> Option<NSRect> {
    let mtm = MainThreadMarker::new()?;
    let primary = NSScreen::screens(mtm).firstObject()?;
    let screen_height = primary.frame().size.height;
    Some(NSRect::new(
        NSPoint::new(frame.left, screen_height - frame.top - frame.height()),
        NSSize::new(frame.width(), frame.height()),
    ))
}
