//! Flutter counterpart: `widgets/magnifier.dart` (`MagnifierBuilder`,
//! `MagnifierInfo`, `TextMagnifierConfiguration`, `MagnifierController`).
//!
//! `RawMagnifier`, `MagnifierDecoration`, and the decorative magnifier widgets wait.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_animation::AnimationController;
use reveal_embedder::{Offset, Rect};
use reveal_foundation::{App, Handle, Listener, ValueNotifier};
use reveal_scheduler::TickerFuture;

use crate::framework::{BuildContext, WidgetRef};
use crate::widgets::basic::WidgetBuilder;
use crate::widgets::overlay::{Overlay, OverlayEntry};

/// Signature for a builder that builds a [`WidgetRef`] with a [`MagnifierController`].
///
/// The builder is called exactly once per magnifier.
///
/// If the `controller` parameter's [`MagnifierController::animation_controller`]
/// field is set (by the builder) to an [`AnimationController`], the
/// [`MagnifierController`] will drive the animation during entry and exit.
///
/// The `magnifier_info` parameter is updated with new [`MagnifierInfo`] instances
/// during the lifetime of the built magnifier, e.g. as the user moves their
/// finger around the text field.
pub type MagnifierBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        Handle<MagnifierController>,
        Handle<ValueNotifier<MagnifierInfo>>,
    ) -> Option<WidgetRef>,
>;

/// A data class that contains the geometry information of text layouts
/// and selection gestures, used to position magnifiers.
#[derive(Clone, Copy, PartialEq)]
pub struct MagnifierInfo {
    /// The offset of the gesture position that the magnifier should be shown at.
    pub global_gesture_position: Offset,
    /// The rect of the current line the magnifier should be shown at, without
    /// taking into account any padding of the field; only the position of the
    /// first and last character.
    pub current_line_boundaries: Rect,
    /// The rect of the handle that the magnifier should follow.
    pub caret_rect: Rect,
    /// The bounds of the entire text field that the magnifier is bound to.
    pub field_bounds: Rect,
}

impl MagnifierInfo {
    /// Constructs a [`MagnifierInfo`] from provided geometry values.
    pub const fn new(
        global_gesture_position: Offset,
        caret_rect: Rect,
        field_bounds: Rect,
        current_line_boundaries: Rect,
    ) -> MagnifierInfo {
        MagnifierInfo {
            global_gesture_position,
            caret_rect,
            field_bounds,
            current_line_boundaries,
        }
    }

    /// Const [`MagnifierInfo`] with all values set to 0.
    pub const EMPTY: MagnifierInfo = MagnifierInfo {
        global_gesture_position: Offset::ZERO,
        caret_rect: Rect::ZERO,
        current_line_boundaries: Rect::ZERO,
        field_bounds: Rect::ZERO,
    };
}

impl Debug for MagnifierInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MagnifierInfo(position: {:?}, line: {:?}, caret: {:?}, field: {:?})",
            self.global_gesture_position,
            self.current_line_boundaries,
            self.caret_rect,
            self.field_bounds
        )
    }
}

/// A configuration object for a magnifier (e.g. in a text field).
///
/// In general, most features of the magnifier can be configured by controlling
/// the widgets built by the [`magnifier_builder`](Self::magnifier_builder).
#[derive(Clone)]
pub struct TextMagnifierConfiguration {
    magnifier_builder: Option<MagnifierBuilder>,
    /// Whether a magnifier should show the text editing handles or not.
    ///
    /// This flag is used by `SelectionOverlay.showMagnifier` to control the order
    /// of layers in the rendering; specifically, whether to place the layer
    /// containing the handles above or below the layer containing the magnifier
    /// in the [`Overlay`].
    pub should_display_handles_in_magnifier: bool,
}

impl TextMagnifierConfiguration {
    /// Constructs a [`TextMagnifierConfiguration`] from parts.
    ///
    /// If [`magnifier_builder`](Self::magnifier_builder) is not set, a default
    /// [`MagnifierBuilder`] will be used that does not build a magnifier.
    pub const fn new() -> TextMagnifierConfiguration {
        TextMagnifierConfiguration {
            magnifier_builder: None,
            should_display_handles_in_magnifier: true,
        }
    }

    /// The builder callback that creates the widget that renders the magnifier.
    pub fn magnifier_builder(&self) -> MagnifierBuilder {
        self.magnifier_builder
            .clone()
            .unwrap_or_else(|| Rc::new(none))
    }

    /// Dart `TextMagnifierConfiguration(magnifierBuilder:)`.
    pub fn with_magnifier_builder(
        mut self,
        magnifier_builder: MagnifierBuilder,
    ) -> TextMagnifierConfiguration {
        self.magnifier_builder = Some(magnifier_builder);
        self
    }

    /// Dart `TextMagnifierConfiguration(shouldDisplayHandlesInMagnifier:)`.
    pub fn should_display_handles_in_magnifier(
        mut self,
        should_display_handles_in_magnifier: bool,
    ) -> TextMagnifierConfiguration {
        self.should_display_handles_in_magnifier = should_display_handles_in_magnifier;
        self
    }

    /// A constant for a [`TextMagnifierConfiguration`] that is disabled, meaning it
    /// never builds anything, regardless of platform.
    pub const DISABLED: TextMagnifierConfiguration = TextMagnifierConfiguration::new();
}

impl Default for TextMagnifierConfiguration {
    fn default() -> TextMagnifierConfiguration {
        TextMagnifierConfiguration::new()
    }
}

fn none(
    _app: &mut App,
    _context: BuildContext,
    _controller: Handle<MagnifierController>,
    _magnifier_info: Handle<ValueNotifier<MagnifierInfo>>,
) -> Option<WidgetRef> {
    None
}

/// A controller for a magnifier.
///
/// [`MagnifierController`]'s main benefit over holding a raw [`OverlayEntry`] is that
/// [`MagnifierController`] will handle logic around waiting for a magnifier to animate in or out.
///
/// If a magnifier chooses to have an entry / exit animation, it should provide the animation
/// controller to [`MagnifierController::animation_controller`]. [`MagnifierController`] will then drive
/// the [`AnimationController`] and wait for it to be complete before removing it from the
/// [`Overlay`].
///
/// To check the status of the magnifier, see [`MagnifierController::shown`].
pub struct MagnifierController {
    /// The controller that will be driven in / out when show / hide is triggered,
    /// respectively.
    pub animation_controller: Option<Handle<AnimationController>>,
    overlay_entry: Option<Handle<OverlayEntry>>,
}

impl MagnifierController {
    /// If there is no in / out animation for the magnifier, [`animation_controller`](Self::animation_controller)
    /// should be left unset.
    pub fn new(app: &mut App) -> Handle<MagnifierController> {
        app.create(MagnifierController {
            animation_controller: None,
            overlay_entry: None,
        })
    }

    /// Dart `MagnifierController(animationController:)`.
    pub fn animation_controller(
        self: Handle<Self>,
        app: &mut App,
        animation_controller: Handle<AnimationController>,
    ) -> Handle<Self> {
        animation_controller.set_value(app, 0.0);
        app.get_mut(self).animation_controller = Some(animation_controller);
        self
    }

    /// The magnifier's [`OverlayEntry`], if currently in the overlay.
    ///
    /// This is exposed so that other overlay entries can be positioned above or
    /// below this [`overlay_entry`](Self::overlay_entry). Anything in the paint order after the
    /// `RawMagnifier` in this [`OverlayEntry`] will not be displayed in the
    /// magnifier; if it is desired for an overlay entry to be displayed in the
    /// magnifier, it *must* be positioned below the magnifier.
    ///
    /// To check if a magnifier is in the overlay, use [`shown`](Self::shown). The
    /// [`overlay_entry`](Self::overlay_entry) field may be non-null even when the magnifier is
    /// not visible.
    pub fn overlay_entry(self: Handle<Self>, app: &App) -> Option<Handle<OverlayEntry>> {
        app.get(self).overlay_entry
    }

    /// Whether the magnifier is currently being shown.
    ///
    /// This is false when nothing is in the overlay, when the
    /// [`animation_controller`](Self::animation_controller) is in the
    /// [`reveal_animation::AnimationStatus::Dismissed`] state, or when
    /// the [`animation_controller`](Self::animation_controller) is animating out (i.e. in the
    /// [`reveal_animation::AnimationStatus::Reverse`] state).
    ///
    /// It is true in the opposite cases, i.e. when the overlay is not empty, and
    /// either the [`animation_controller`](Self::animation_controller) is `None`, in the
    /// [`reveal_animation::AnimationStatus::Completed`] state, or in the
    /// [`reveal_animation::AnimationStatus::Forward`]
    /// state.
    pub fn shown(self: Handle<Self>, app: &App) -> bool {
        app.get(self).overlay_entry.is_some()
            && app
                .get(self)
                .animation_controller
                .is_none_or(|controller| controller.status(app).is_forward_or_completed())
    }

    /// Displays the magnifier.
    ///
    /// Returns a future that completes when the magnifier is fully shown, i.e. done
    /// with its entry animation.
    ///
    /// To control what overlays are shown in the magnifier, use `below`. See
    /// [`overlay_entry`](Self::overlay_entry) for more details on how to utilize `below`.
    ///
    /// If the magnifier already exists (i.e. [`overlay_entry`](Self::overlay_entry) is `Some`),
    /// then [`show`](Self::show) will replace the old overlay without playing an exit
    /// animation. Consider awaiting [`hide`](Self::hide) first, to animate from the old
    /// magnifier to the new one.
    pub fn show(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        builder: WidgetBuilder,
        below: Option<Handle<OverlayEntry>>,
    ) -> TickerFuture {
        if let Some(entry) = app.get(self).overlay_entry {
            entry.remove(app);
            entry.dispose(app);
        }

        let overlay_state = Overlay::of(app, context, true);
        let overlay_entry = OverlayEntry::new(app, builder, false, false, false);
        app.get_mut(self).overlay_entry = Some(overlay_entry);
        overlay_state.insert(app, overlay_entry, below, None);

        if let Some(controller) = app.get(self).animation_controller {
            controller.forward(app, None)
        } else {
            TickerFuture::complete()
        }
    }

    /// Schedules a hide of the magnifier.
    ///
    /// If this [`MagnifierController`] has an [`animation_controller`](Self::animation_controller),
    /// then [`hide`](Self::hide) reverses the animation controller and waits
    /// for the animation to complete. Then, if `remove_from_overlay`
    /// is true, remove the magnifier from the overlay.
    ///
    /// In general, `remove_from_overlay` should be true, unless
    /// the magnifier needs to preserve states between shows / hides.
    ///
    /// See also:
    ///
    ///  * [`remove_from_overlay`](Self::remove_from_overlay) which removes the [`OverlayEntry`] from the [`Overlay`]
    ///    synchronously.
    pub fn hide(self: Handle<Self>, app: &mut App, remove_from_overlay: bool) -> TickerFuture {
        if app.get(self).overlay_entry.is_none() {
            return TickerFuture::complete();
        }

        if let Some(controller) = app.get(self).animation_controller {
            let future = controller.reverse(app, None);
            if remove_from_overlay {
                future.when_complete(
                    app,
                    Listener::handle_method(self, MagnifierController::remove_from_overlay),
                );
            }
            return future;
        }

        if remove_from_overlay {
            self.remove_from_overlay(app);
        }
        TickerFuture::complete()
    }

    /// Remove the [`OverlayEntry`] from the [`Overlay`].
    ///
    /// This method removes the [`OverlayEntry`] synchronously,
    /// regardless of exit animation: this leads to abrupt removals
    /// of [`OverlayEntry`]s with animations.
    ///
    /// To allow the [`OverlayEntry`] to play its exit animation, consider calling
    /// [`hide`](Self::hide) instead, with `remove_from_overlay` set to true, and optionally await
    /// the returned future.
    pub fn remove_from_overlay(self: Handle<Self>, app: &mut App) {
        if let Some(entry) = app.get(self).overlay_entry {
            entry.remove(app);
            entry.dispose(app);
        }
        app.get_mut(self).overlay_entry = None;
    }

    /// A utility for calculating a new [`Rect`] from `rect` such that
    /// `rect` is fully constrained within `bounds`.
    ///
    /// Any point in the output rect is guaranteed to also be a point contained in `bounds`.
    ///
    /// It is a runtime error for `rect`.width to be greater than `bounds`.width,
    /// and it is also an error for `rect`.height to be greater than `bounds`.height.
    ///
    /// This algorithm translates `rect` the shortest distance such that it is entirely within
    /// `bounds`.
    ///
    /// If `rect` is already within `bounds`, no shift will be applied to `rect` and
    /// `rect` will be returned as-is.
    ///
    /// It is perfectly valid for the output rect to have a point along the edge of the
    /// `bounds`. If the desired output rect requires that no edges are parallel to edges
    /// of `bounds`, see [`Rect::deflate`] by 1 on `bounds` to achieve this effect.
    pub fn shift_within_bounds(rect: Rect, bounds: Rect) -> Rect {
        debug_assert!(
            rect.width() <= bounds.width(),
            "attempted to shift {rect:?} within {bounds:?}, but the rect has a greater width.",
        );
        debug_assert!(
            rect.height() <= bounds.height(),
            "attempted to shift {rect:?} within {bounds:?}, but the rect has a greater height.",
        );

        let mut rect_shift = Offset::ZERO;
        if rect.left < bounds.left {
            rect_shift = rect_shift + Offset::new(bounds.left - rect.left, 0.0);
        } else if rect.right > bounds.right {
            rect_shift = rect_shift + Offset::new(bounds.right - rect.right, 0.0);
        }

        if rect.top < bounds.top {
            rect_shift = rect_shift + Offset::new(0.0, bounds.top - rect.top);
        } else if rect.bottom > bounds.bottom {
            rect_shift = rect_shift + Offset::new(0.0, bounds.bottom - rect.bottom);
        }

        rect.shift(rect_shift)
    }
}
