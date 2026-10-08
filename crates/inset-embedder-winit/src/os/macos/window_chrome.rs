//! What a window's configuration asks of AppKit beyond winit's attributes: its shadow, presence
//! on every Space, the glass or blur behind a see-through window, and, for a window that never
//! becomes key, the pointer's moves.

use inset_embedder::{WindowBackground, WindowConfig};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSGlassEffectView, NSGlassEffectViewStyle, NSTrackingArea,
    NSTrackingAreaOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::NSRect;
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Window, WindowAttributes};

use super::{AppKit, ns_view, ns_window};
use crate::os::WindowChrome;

impl WindowChrome for AppKit {
    /// What winit has an AppKit attribute for: the shadow.
    fn extend_attributes(attributes: WindowAttributes, config: &WindowConfig) -> WindowAttributes {
        attributes.with_has_shadow(config.shadow)
    }

    /// Pointer tracking for a window that never activates, presence on every Space, and the
    /// blur or glass behind a see-through window.
    fn configure(window: &Window, config: &WindowConfig) {
        if !config.activating {
            track_pointer_always(window);
        }
        if config.all_desktops {
            join_all_spaces(window);
        }
        if matches!(
            config.background,
            WindowBackground::Blurred | WindowBackground::Glass
        ) {
            put_effect_behind_content(window, config.background);
        }
    }
}

/// Has AppKit report the pointer's moves to the window's view whether or not the window is key.
///
/// winit tracks the mouse with a tracking rect, which reports entry and exit alone, and
/// takes moves from `acceptsMouseMovedEvents`, which AppKit honours for the key window
/// only. A window that never activates is never key, so without this it would hear the
/// pointer arrive and leave but never move: no hover, and clicks at stale positions.
/// Flutter's macOS embedder offers the same as its `always` mouse tracking mode. The
/// view is the owner, so winit's own handlers receive what the area reports.
fn track_pointer_always(window: &Window) {
    let Some(view) = ns_view(window) else {
        return;
    };
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

/// Shows the window on every Space, a full-screen app's included, and leaves it in place when
/// Mission Control spreads the windows out.
fn join_all_spaces(window: &Window) {
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
}

/// Puts the blur or glass `background` behind winit's view, sized with the window.
///
/// The effect view goes behind winit's view, as a sibling in the window's frame view: a
/// subview of winit's view would draw over its Metal layer. Liquid Glass renders as its flat
/// material there rather than as glass, which needs the view inside the content view, where
/// winit's Metal layer did not stay above it.
fn put_effect_behind_content(window: &Window, background: WindowBackground) {
    if let Some(mtm) = MainThreadMarker::new()
        && let Some(ns_window) = ns_window(window)
        && let Some(content) = ns_window.contentView()
        // SAFETY: the view hierarchy of a live window, read on the main thread.
        && let Some(frame_view) = unsafe { content.superview() }
    {
        let effect = background_view(mtm, background, content.frame());
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
